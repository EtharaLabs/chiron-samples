from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, jsonify, authorize
import logging
from uuid import UUID
from argus.backend.controller.views_widgets.highlights import bp as highlights_bp
from argus.backend.controller.views_widgets.summary import bp as summary_bp
from argus.backend.controller.views_widgets.graphs import bp as graphs_bp
from argus.backend.controller.views_widgets.nemesis_stats import bp as nemesis_stats_bp
from argus.backend.controller.views_widgets.graphed_stats import bp as graphed_stats_bp
from argus.backend.controller.views_widgets.pytest import bp as pytest_bp
from argus.backend.error_handlers import APIException, handle_api_exception
from argus.backend.models.web import User
from argus.backend.service.stats import ViewStatsCollector
from argus.backend.service.views import UserViewService
from argus.backend.util.common import get_payload
bp = APIRouter(prefix='/views')
LOGGER = logging.getLogger(__name__)
bp.include_router(highlights_bp)
bp.include_router(summary_bp)
bp.include_router(graphs_bp)
bp.include_router(graphed_stats_bp)
bp.include_router(nemesis_stats_bp)
bp.include_router(pytest_bp)

class ViewApiException(APIException):
    pass

@bp.api_route('/', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.index', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def index(request: Request):
    return respond({'status': 'ok', 'response': {'version': 'v1'}})

@bp.api_route('/create', methods=['POST', 'OPTIONS'], name='view_api.create_view', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def create_view(request: Request):
    payload = get_payload(request)
    service = UserViewService()
    view = service.create_view(name=payload['name'], items=payload['items'], widget_settings=payload['settings'], description=payload.get('description'), display_name=payload.get('displayName'), request=request)
    return respond({'status': 'ok', 'response': view})

@bp.api_route('/get', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.get_view', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_view(request: Request):
    view_id = request.query_params.get('viewId')
    if not view_id:
        raise ViewApiException('No viewId provided.')
    service = UserViewService()
    view = service.get_view(UUID(view_id), request=request)
    return respond({'status': 'ok', 'response': view})

@bp.api_route('/all', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.get_all_views', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_all_views(request: Request):
    user_id = request.query_params.get('userId')
    if user_id:
        user = User.get(id=UUID(user_id))
    else:
        user = None
    service = UserViewService()
    views = service.get_all_views(user, request=request)
    return respond({'status': 'ok', 'response': views})

@bp.api_route('/update', methods=['POST', 'OPTIONS'], name='view_api.update_view', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def update_view(request: Request):
    payload = get_payload(request)
    service = UserViewService()
    res = service.update_view(view_id=payload['viewId'], update_data=payload['updateData'], request=request)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/delete', methods=['POST', 'OPTIONS'], name='view_api.delete_view', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def delete_view(request: Request):
    payload = get_payload(request)
    service = UserViewService()
    res = service.delete_view(payload['viewId'], request=request)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/search', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.search_tests', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def search_tests(request: Request):
    query = request.query_params.get('query')
    service = UserViewService()
    if query:
        res = service.test_lookup(query)
    else:
        res = []
    return respond({'status': 'ok', 'response': {'hits': res, 'total': len(res)}})

@bp.api_route('/stats', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.view_stats', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def view_stats(request: Request):
    view_id = request.query_params.get('viewId')
    if not view_id:
        raise ViewApiException('No view id provided.')
    limited = bool(int(request.query_params.get('limited', 0)))
    version = request.query_params.get('productVersion', None)
    include_no_version = bool(int(request.query_params.get('includeNoVersion', True)))
    image_id = request.query_params.get('imageId', None)
    force = bool(int(request.query_params.get('force', 0)))
    widget_id = request.query_params.get('widgetId', None)
    if widget_id:
        widget_id = int(widget_id)
    collector = ViewStatsCollector(view_id=view_id, filter=version)
    stats = collector.collect(limited=limited, force=force, include_no_version=include_no_version, widget_id=widget_id, image_id=image_id, request=request)
    res = jsonify({'status': 'ok', 'response': stats})
    return respond(res)

@bp.api_route('/{view_id}/versions', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.view_versions', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def view_versions(request: Request, view_id: str):
    service = UserViewService()
    res = service.get_versions_for_view(view_id)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/{view_id}/images', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.view_images', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def view_images(request: Request, view_id: str):
    service = UserViewService()
    res = service.get_images_for_view(view_id)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/{view_id}/resolve', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.view_resolve', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def view_resolve(request: Request, view_id: str):
    service = UserViewService()
    res = service.resolve_view_for_edit(view_id)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/{view_id}/resolve/tests', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.view_resolve_tests', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def view_resolve_tests(request: Request, view_id: str):
    service = UserViewService()
    res = service.resolve_view_tests(view_id)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/{view_id}/pytest/results', methods=['GET', 'HEAD', 'OPTIONS'], name='view_api.view_get_pytest_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def view_get_pytest_results(request: Request, view_id: str):
    service = UserViewService()
    res = service.get_pytest_view_results(view_id)
    return respond({'status': 'ok', 'response': res})
