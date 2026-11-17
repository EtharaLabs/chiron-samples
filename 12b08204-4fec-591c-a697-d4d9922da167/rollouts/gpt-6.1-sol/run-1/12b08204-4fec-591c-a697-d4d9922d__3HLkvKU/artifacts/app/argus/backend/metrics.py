"""One instrumentation point for all HTTP requests, including error responses."""
import os
from http import HTTPStatus
from prometheus_client import CollectorRegistry, Counter, Histogram, Gauge, generate_latest, multiprocess
from starlette.responses import Response
from fastapi import Request, Depends
from argus.backend.http import authorize
from argus.backend import metrics_labels as labels


class RequestMetrics:
    def __init__(self, app):
        self.registry = CollectorRegistry()
        if not os.environ.get('PROMETHEUS_MULTIPROC_DIR'):
            from prometheus_client import GCCollector, ProcessCollector, PlatformCollector
            GCCollector(registry=self.registry)
            ProcessCollector(registry=self.registry)
            PlatformCollector(registry=self.registry)
        kw = {'registry': self.registry}
        self.duration = Histogram('http_request_duration_seconds', 'HTTP request duration, in seconds', [
                                  'method', 'endpoint', 'status'], **kw)
        self.total = Counter('http_request_total', 'Total number of HTTP requests', ['method', 'status'], **kw)
        self.exceptions = Counter('http_request_exceptions_total',
                                  'Total number of HTTP requests which resulted in an exception',
                                  ['method', 'status'], **kw)
        Gauge('exporter_info', 'Information about the exporter', ['version'],
              multiprocess_mode='max', **kw).labels('0.23.2').set(1)
        specs = {
            'by_endpoint': ('endpoint', 'method', 'status'),
            'by_ip': ('ip', 'endpoint'),
            'ssh_tunnel': ('ssh_tunnel', 'tunnel_established', 'endpoint'),
            'job_tunnel': ('job_name', 'branch', 'client_version', 'ssh_tunnel'),
            'by_user_agent': ('user_agent_category', 'client_version', 'endpoint'),
            'tunnel_build': ('build_id', 'build_url', 'ssh_tunnel'),
        }
        self.counters = {k: Counter('http_request_'+k+'_total', 'Total Requests made', v, **kw)
                         for k, v in specs.items()}

        @app.api_route('/metrics', methods=['GET', 'HEAD', 'OPTIONS'], name='prometheus_metrics', response_model=None, dependencies=[Depends(authorize(True, True))] if os.environ.get('PROMETHEUS_MULTIPROC_DIR') else [])
        def metrics(request: Request):
            registry = self.registry
            if os.environ.get('PROMETHEUS_MULTIPROC_DIR'):
                registry = CollectorRegistry()
                multiprocess.MultiProcessCollector(registry)
            return Response(generate_latest(registry), headers={'Content-Type': 'text/plain; version=0.0.4; charset=utf-8'})

    def observe(self, request, response, elapsed, unhandled_exception=False):
        endpoint = getattr(request.scope.get('route'), 'name', None) or 'None'
        status = str(response.status_code)
        user_loaded = hasattr(request.state, 'user')
        if endpoint != 'prometheus_metrics' or response.status_code != 200:
            if user_loaded:
                self.duration.labels(request.method, endpoint, status).observe(elapsed)
            self.total.labels(request.method, status).inc()
            if unhandled_exception:
                self.exceptions.labels(request.method, status).inc()
        if not user_loaded or endpoint == 'None' or request.method == 'OPTIONS':
            return
        try:
            phrase = HTTPStatus(response.status_code).phrase.upper()
        except ValueError:
            phrase = 'UNKNOWN'
        vals = {
            'endpoint': endpoint, 'method': request.method, 'status': status+' '+phrase,
            'ip': request.client.host if request.client else None, 'ssh_tunnel': labels.ssh_tunnel(request=request),
            'tunnel_established': 'yes' if request.headers.get('X-Tunnel-Established-At') else 'no',
            'job_name': labels.job_name(request=request), 'branch': labels.branch(request=request),
            'client_version': labels.client_version(request=request),
            'user_agent_category': labels.categorize_user_agent(request.headers.get('User-Agent', '')),
            'build_id': labels.build_id(request=request), 'build_url': request.headers.get('X-Argus-Build-Url') or '',
        }
        for counter in self.counters.values():
            counter.labels(**{k: vals[k] for k in counter._labelnames}).inc()
