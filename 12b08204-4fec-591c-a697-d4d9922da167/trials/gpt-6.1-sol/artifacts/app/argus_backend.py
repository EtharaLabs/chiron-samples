"""Argus ASGI application factory."""
import hashlib
import json
from fastapi import FastAPI, Depends, Request
from fastapi.exceptions import RequestValidationError
from starlette.concurrency import run_in_threadpool
from starlette.datastructures import FormData
from starlette.responses import HTMLResponse
from starlette.staticfiles import StaticFiles
from itsdangerous import URLSafeTimedSerializer, BadSignature
from jinja2 import Environment, FileSystemLoader, select_autoescape
from argus.backend.http import EarlyResponse, respond, is_json, BadRequest
from argus.backend.util.config import Config


async def prepare_request(request: Request):
    from argus.backend.service.user import load_logged_in_user
    request.state.body = await request.body()
    media = request.headers.get('content-type', '').split(';', 1)[0].strip().lower()
    request.state.form = await request.form() if media in ('application/x-www-form-urlencoded', 'multipart/form-data') else FormData()
    request.state.payload = None
    request.state.json_error = False
    if is_json(request):
        try:
            request.state.payload = json.loads(request.state.body)
        except (ValueError, UnicodeDecodeError):
            request.state.json_error = True
    await run_in_threadpool(load_logged_in_user, request=request)


def create_app(config=None):
    from argus.backend.controller import auth, main, api, admin
    from argus.backend.db import ScyllaCluster
    from argus.backend.template_filters import export_filters
    from argus.backend.util.encoders import ArgusJSONProvider
    from argus.backend.error_handlers import handle_api_exception
    from argus.backend.metrics import RequestMetrics
    settings = dict(Config.load_yaml_config())
    if config:
        settings.update(config)
    Config.CONFIG = settings
    from argus.backend.util.logsetup import setup_application_logging
    setup_application_logging(log_level=settings.get('APP_LOG_LEVEL', 'INFO'))
    ScyllaCluster.get(settings)
    if 'cf' in settings.get('LOGIN_METHODS', []) and settings.get('CLOUDFLARE_ACCESS_TEAM_DOMAIN'):
        from jwt import PyJWKClient
        settings['CLOUDFLARE_ACCESS_JWK_CLIENT'] = PyJWKClient(
            f"https://{settings['CLOUDFLARE_ACCESS_TEAM_DOMAIN']}/cdn-cgi/access/certs", cache_keys=True, lifespan=3600, timeout=5)
    app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None,
                  redirect_slashes=False, dependencies=[Depends(prepare_request)])
    app.state.config = settings
    app.state.extensions = {}
    env = Environment(loader=FileSystemLoader('templates'),
                      autoescape=select_autoescape(('html', 'htm', 'xml', 'xhtml', 'svg')))
    env.policies['json.dumps_kwargs']['default'] = ArgusJSONProvider.default
    for func in export_filters():
        env.filters[func.filter_name] = func
    app.state.templates = env

    def install(router, prefix=''):
        for route in router.routes:
            if hasattr(route, 'original_router'):
                install(route.original_router, prefix + route.include_context.prefix)
                continue
            path = prefix + route.path
            name = route.name
            if path.startswith('/admin/api/'):
                name = 'admin.' + name
            elif path.startswith('/api/v1/') and not name.startswith('api.'):
                parent = 'api.'
                if path.startswith('/api/v1/client/') and not name.startswith('client_api.'):
                    parent += 'client_api.'
                if path.startswith('/api/v1/views/') and not name.startswith('view_api.'):
                    parent += 'view_api.'
                name = parent + name
            elif path.startswith(('/teams/', '/notifications/')):
                name = 'main.' + name
            app.add_api_route(path, route.endpoint, methods=route.methods, name=name,
                              response_model=None, dependencies=route.dependencies)
    for router in (auth.bp, main.bp, api.bp, admin.bp):
        install(router)
    app.router.routes.sort(key=lambda r: tuple(2 if ':path}' in x else 1 if '{' in x else 0 for x in r.path.split('/')))
    app.mount('/s', StaticFiles(directory='public'), name='static')
    from argus.backend.service.user import SSH_TUNNEL_SERVER_ALLOWED_ENDPOINTS_KEY
    app.state.extensions[SSH_TUNNEL_SERVER_ALLOWED_ENDPOINTS_KEY] = {
        (r.name, m) for r in app.routes for m in getattr(r, 'methods', []) if r.path == '/api/v1/client/ssh/keys'}

    @app.exception_handler(EarlyResponse)
    async def early(request, exc):
        return exc.response

    async def error(request, exc):
        if request.scope.get('route') is not None and request.url.path.startswith(('/api/', '/admin/api/')):
            if isinstance(exc, ValueError) and str(exc) == 'badly formed hexadecimal UUID string':
                trace = exc.__traceback__
                while trace and trace.tb_next:
                    trace = trace.tb_next
                bad_value = trace.tb_frame.f_locals.get('hex') if trace else None
                if any(str(v).replace('urn:', '').replace('uuid:', '').strip('{}').replace('-', '') == bad_value
                       for v in request.path_params.values()):
                    exc = RequestValidationError([{'loc': ('path',), 'msg': 'Invalid UUID', 'type': 'uuid_parsing'}])
            return respond(await run_in_threadpool(handle_api_exception, exc, request=request))
        from argus.backend.service.user import UserServiceException
        from argus.backend.service.views import UserViewException
        from argus.backend.error_handlers import handle_profile_exception, handle_view_not_found
        if isinstance(exc, UserServiceException):
            return await run_in_threadpool(handle_profile_exception, exc, request=request)
        if isinstance(exc, UserViewException):
            return await run_in_threadpool(handle_view_not_found, exc, request=request)
        return HTMLResponse('<!doctype html>\n<html lang=en>\n<title>500 Internal Server Error</title>\n<h1>Internal Server Error</h1>\n<p>The server encountered an internal error and was unable to complete your request. Either the server is overloaded or there is an error in the application.</p>\n', status_code=500)
    from starlette.exceptions import HTTPException
    from argus.backend.http import http_error, BadRequestKeyError

    @app.exception_handler(HTTPException)
    async def http_exception(request, exc):
        return http_error(exc.status_code)

    @app.exception_handler(BadRequestKeyError)
    async def form_error(request, exc):
        if request.url.path.startswith(('/api/', '/admin/api/')):
            return await error(request, exc)
        return http_error(400)
    import cassandra.cluster
    from argus.backend.error_handlers import DBErrorHandler

    async def database_error(request, exc):
        return respond(await run_in_threadpool(DBErrorHandler.handle_db_errors, exc))
    app.add_exception_handler(cassandra.cluster.NoHostAvailable, database_error)
    app.add_exception_handler(cassandra.cluster.NoConnectionsAvailable, database_error)
    app.add_exception_handler(Exception, error)
    app.add_exception_handler(RequestValidationError, error)
    app.add_exception_handler(BadRequest, error)
    metrics = RequestMetrics(app)
    @app.middleware('http')
    async def session_and_metrics(request, call_next):
        from time import perf_counter
        if request.client is None and request.headers.get('X-Forwarded-For'):
            # Nginx supplies the peer address over the private Unix socket.
            request.scope['client'] = (request.headers['X-Forwarded-For'].split(',')[0].strip(), 0)
        signer = URLSafeTimedSerializer(settings.get('SECRET_KEY', ''), salt='cookie-session',
                                        signer_kwargs={'key_derivation': 'hmac', 'digest_method': hashlib.sha1})
        cookie = request.cookies.get('session')
        try:
            session = signer.loads(cookie, max_age=2678400) if cookie else {}
        except BadSignature:
            session = {}
        before = json.dumps(session, sort_keys=True)
        request.scope['session'] = session
        start = perf_counter()
        unhandled_exception = False
        matches = []
        try:
            from argus.backend.http import http_error, redirect
            from starlette.routing import Match, Mount
            matches = [r for r in app.routes if r.matches(request.scope)[0] == Match.FULL]
            if matches:
                request.scope['route'] = matches[0]
            if not matches or request.method == 'OPTIONS' or isinstance(matches[0], Mount):
                from argus.backend.service.user import load_logged_in_user
                request.state.body = await request.body()
                request.state.payload = None
                if is_json(request):
                    try:
                        request.state.payload = json.loads(request.state.body)
                    except (ValueError, UnicodeDecodeError):
                        pass
                await run_in_threadpool(load_logged_in_user, request=request)
            if request.method == 'OPTIONS' and matches:
                request.scope['route'] = matches[0]
                response = HTMLResponse('')
            elif not matches and any(getattr(r, 'path', None) == request.url.path+'/' for r in app.routes):
                response = redirect(str(request.url.replace(path=request.url.path+'/')), 308)
            elif request.url.path in ('/s', '/s/'):
                request.scope.pop('route', None)
                response = http_error(404)
            else:
                response = await call_next(request)
        except Exception as exc:
            response = await error(request, exc)
            unhandled_exception = response.status_code == 500
        if not matches:
            request.scope.pop('route', None)
        if session:
            response.headers.append('Vary', 'Cookie')
            if json.dumps(session, sort_keys=True) != before:
                response.set_cookie('session', signer.dumps(session), httponly=True, samesite=None)
        elif cookie and before != '{}':
            response.delete_cookie('session', httponly=True, samesite=None)
        metrics.observe(request, response, perf_counter()-start, unhandled_exception)
        if request.method == 'HEAD':
            response.body_iterator = iter_empty()
        return response
    return app


async def iter_empty():
    if False:
        yield b''


start_server = create_app
