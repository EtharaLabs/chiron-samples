from fastapi import Request
'Label callbacks for the Prometheus counters registered in ``argus_backend``.\n\nThey live here rather than next to ``start_server()`` so a test can import them\nwithout opening a Scylla connection. Every function reads the live request and\nreturns a label value, so each one must return a bounded set of strings: an\nunbounded label value is a new time series per request.\n'
import re
MAX_BUILD_ID_LEN = 256
_BUILD_NUMBER_SUFFIX = re.compile('#\\d+$')
_BRANCH_SEGMENT = re.compile('^[\\w.\\-]{1,64}$')
_CLIENT_VERSION = re.compile('^\\d{1,4}(\\.\\d{1,5}){0,3}([.\\-+][\\w.\\-+]{1,32})?$')

def categorize_user_agent(ua: str) -> str:
    if not ua:
        return 'unknown'
    if 'argus-client-ssh-tunnel' in ua:
        return 'argus-client-tunnel'
    if ua.startswith(('python-requests', 'python-urllib')):
        return 'argus-client'
    if ua.startswith('Go-http-client'):
        return 'argus-cli-go'
    if 'Mozilla' in ua:
        return 'browser'
    if 'curl' in ua:
        return 'curl'
    return 'other'

def ssh_tunnel(*, request: Request=None) -> str:
    return 'yes' if request.headers.get('X-SSH-Tunnel-Origin') else 'no'

def build_id(*, request: Request=None) -> str:
    value = request.headers.get('X-Argus-Build-Id', '').strip()
    if not value or len(value) > MAX_BUILD_ID_LEN:
        return 'unknown'
    return value

def job_name(*, request: Request=None) -> str:
    """The Jenkins job path with the build number removed.

    ``http_request_tunnel_build_total`` mints a series per build and grows
    without bound. This label is bounded by the number of jobs, which is what
    makes per-job tunnel adoption cheap enough to keep on a dashboard.
    """
    return _BUILD_NUMBER_SUFFIX.sub('', build_id(request=request))

def branch(*, request: Request=None) -> str:
    """The release line a job belongs to: the first segment of the job path.

    SCT and dtest pin their own copy of this client per release line, so the
    branch is what decides whether a job can tunnel at all. Grouping by it turns
    "adoption is low" into "these release lines still run a client without
    tunnel support".
    """
    segment = job_name(request=request).split('/', 1)[0]
    if not _BRANCH_SEGMENT.match(segment):
        return 'unknown'
    return segment

def client_version(*, request: Request=None) -> str:
    """The argus-alm version behind the request.

    Tunnel support starts at 0.16.0, so a job that reports an older version, or
    that reports none at all, cannot tunnel no matter how the proxy behaves.
    Telling those two cases apart is the difference between "backport the
    client" and "debug the tunnel".
    """
    value = request.headers.get('X-Argus-Client-Version', '').strip()
    if not _CLIENT_VERSION.match(value):
        return 'unknown'
    return value
