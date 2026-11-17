"""HTTP responses, templates and native FastAPI request dependencies."""
import json
from functools import partial
from uuid import UUID
from urllib.parse import urlencode
from html import escape
from fastapi import Request
from fastapi.exceptions import RequestValidationError
from starlette.responses import Response, HTMLResponse


def jsonify(value):
    from argus.backend.util.encoders import ArgusJSONProvider
    return Response(json.dumps(value, default=ArgusJSONProvider.default, sort_keys=True), media_type='application/json')


def respond(value):
    if isinstance(value, Response):
        return value
    status, headers = 200, None
    if isinstance(value, tuple):
        value, status, *rest = value
        headers = rest[0] if rest else None
    if isinstance(value, (dict, list)):
        result = jsonify(value)
        result.status_code = status
    else:
        result = HTMLResponse(value or '', status_code=status)
    if headers:
        result.headers.update(dict(headers))
    return result


def redirect(location, code=302):
    body = '<!doctype html>\n<html lang=en>\n<title>Redirecting...</title>\n<h1>Redirecting...</h1>\n<p>You should be redirected automatically to the target URL: <a href="' + \
        escape(str(location), quote=True) + '">' + escape(str(location)) + '</a>. If not, click the link.\n'
    return HTMLResponse(body, status_code=code, headers={'Location': str(location)})


def url_for(request, endpoint, **values):
    external = values.pop('_external', False)
    # Route names retain their public endpoint identifiers.
    for route in request.app.routes:
        if getattr(route, 'name', '') != endpoint:
            continue
        keys = set(getattr(route, 'param_convertors', {}))
        if not keys.issubset(values):
            continue
        path = route.url_path_for(endpoint, **{k: str(values[k]) for k in keys})
        query = {k: v for k, v in values.items() if k not in keys}
        if query:
            path = str(path) + '?' + urlencode(query)
        return str(request.base_url).rstrip('/') + str(path) if external else str(path)
    raise RuntimeError(f'Unknown route: {endpoint}')


def flash(request, message, category='message'):
    messages = request.session.get('_flashes', [])
    messages.append((category, message))
    request.session['_flashes'] = messages


def render_template(request, template, **values):
    from argus.backend.util.config import Config

    def messages(with_categories=False):
        cached = getattr(request.state, 'flashes', None)
        if cached is None:
            cached = request.session.pop('_flashes', [])
            request.state.flashes = cached
        return cached if with_categories else [v for _, v in cached]
    values.update(request=request, session=request.session, user=getattr(request.state, 'user', None),
                  config=Config.load_yaml_config(), url_for=partial(url_for, request), get_flashed_messages=messages)
    return request.app.state.templates.get_template(template).render(**values)


def is_json(request):
    media = request.headers.get('content-type', '').split(';', 1)[0].strip().lower()
    return media == 'application/json' or (media.startswith('application/') and media.endswith('+json'))


def require_json(request):
    if not is_json(request):
        raise RequestValidationError([{'loc': ('body',), 'msg': 'JSON Content-Type required', 'type': 'json_type'}])
    if getattr(request.state, 'json_error', False):
        raise BadRequest()


class BadRequest(Exception):
    code = 400

    def __str__(self):
        return '400 Bad Request: The browser (or proxy) sent a request that this server could not understand.'


class EarlyResponse(Exception):
    def __init__(self, response):
        self.response = response


def authorize(required=False, api=False, roles=None, roles_api=False):
    def dependency(request: Request):
        from argus.backend.service.user import UserService, is_scoped_ssh_tunnel_server_blocked
        user = request.state.user
        if required and user is None:
            if api:
                raise EarlyResponse(respond(({'status': 'error', 'message': 'Authorization required'}, 403)))
            flash(request, 'Unauthorized, please login', 'error')
            request.session['redirect_target'] = request.url.path+'?'+request.url.query
            raise EarlyResponse(redirect(url_for(request, 'auth.login')))
        blocked = required and user and is_scoped_ssh_tunnel_server_blocked(user, request=request)
        if blocked or (roles and not UserService.check_roles(roles, user)):
            if (api if blocked else roles_api):
                raise EarlyResponse(
                    respond(({'status': 'error', 'message': 'Authorization required' if blocked else 'Forbidden'}, 403)))
            flash(request, 'Not authorized to access this area', 'error')
            raise EarlyResponse(redirect(url_for(request, 'main.home')))
    return dependency


def validate_ids(names):
    def dependency(request: Request):
        for name in names:
            if name in request.path_params:
                try:
                    UUID(str(request.path_params[name]))
                except ValueError:
                    raise RequestValidationError(
                        [{'loc': ('path', name), 'msg': 'Invalid UUID', 'type': 'uuid_parsing'}])
    return dependency


def path_uuid(value):
    try:
        return UUID(value)
    except ValueError:
        raise RequestValidationError([{'loc': ('path',), 'msg': 'Invalid UUID', 'type': 'uuid_parsing'}])


class UnsupportedMediaType(Exception):
    def __str__(self):
        return "415 Unsupported Media Type: Did not attempt to load JSON data because the request Content-Type was not 'application/json'."


class NotFound(Exception):
    def __str__(self):
        return '404 Not Found: The requested URL was not found on the server. If you entered the URL manually please check your spelling and try again.'


def json_body(request, silent=False):
    if not is_json(request):
        if silent:
            return None
        raise UnsupportedMediaType()
    if request.state.json_error:
        if silent:
            return None
        raise BadRequest()
    return request.state.payload


def http_error(status):
    from http import HTTPStatus
    title = HTTPStatus(status).phrase
    descriptions = {400: 'The browser (or proxy) sent a request that this server could not understand.',
                    404: 'The requested URL was not found on the server. If you entered the URL manually please check your spelling and try again.', 405: 'The method is not allowed for the requested URL.'}
    return HTMLResponse(f'<!doctype html>\n<html lang=en>\n<title>{status} {title}</title>\n<h1>{title}</h1>\n<p>{descriptions.get(status, title)}</p>\n', status_code=status)


class BadRequestKeyError(KeyError):
    def __str__(self):
        return str(BadRequest())


def form_field(request, key):
    try:
        return request.state.form[key]
    except KeyError:
        raise BadRequestKeyError(key)
