from argus.backend.http import path_uuid
from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
import logging
from uuid import UUID
from argus.backend.error_handlers import handle_api_exception
from argus.backend.service.planner_service import CopyPlanPayload, PlanningService, TempPlanPayload
from argus.backend.service.test_lookup import TestLookup
from argus.backend.util.common import get_payload
bp = APIRouter(prefix='/planning')
LOGGER = logging.getLogger(__name__)

@bp.api_route('/', methods=['GET', 'HEAD', 'OPTIONS'], name='planning_api.version', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def version(request: Request):
    result = PlanningService().version()
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/plan/{plan_id}/copy/check', methods=['GET', 'HEAD', 'OPTIONS'], name='planning_api.is_plan_eligible_for_copy', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def is_plan_eligible_for_copy(request: Request, plan_id: str):
    release_id = request.query_params.get('releaseId')
    if not release_id:
        raise Exception('Missing release id.')
    result = PlanningService().check_plan_copy_eligibility(plan_id=path_uuid(plan_id), target_release_id=UUID(release_id))
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/release/{release_id}/gridview', methods=['GET', 'HEAD', 'OPTIONS'], name='planning_api.grid_view_for_release', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def grid_view_for_release(request: Request, release_id: str):
    result = PlanningService().get_gridview_for_release(release_id=path_uuid(release_id))
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/search', methods=['GET', 'HEAD', 'OPTIONS'], name='planning_api.search_tests', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def search_tests(request: Request):
    query = request.query_params.get('query')
    release_id = request.query_params.get('releaseId')
    service = TestLookup
    if query:
        res = service.test_lookup(query, release_id=release_id)
    else:
        res = []
    return respond({'status': 'ok', 'response': {'hits': res, 'total': len(res)}})

@bp.api_route('/group/{group_id}/explode', methods=['GET', 'HEAD', 'OPTIONS'], name='planning_api.explode_group', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def explode_group(request: Request, group_id: str):
    service = TestLookup
    res = service.explode_group(group_id=group_id)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/plan/{plan_id}/get', methods=['GET', 'HEAD', 'OPTIONS'], name='planning_api.get_plan', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_plan(request: Request, plan_id: str):
    result = PlanningService().get_plan(plan_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/release/{release_id}/all', methods=['GET', 'HEAD', 'OPTIONS'], name='planning_api.get_plans_for_release', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_plans_for_release(request: Request, release_id: str):
    result = PlanningService().get_plans_for_release(release_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/plan/create', methods=['POST', 'OPTIONS'], name='planning_api.create_plan', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def create_plan(request: Request):
    payload = get_payload(request)
    result = PlanningService().create_plan(payload, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/plan/update', methods=['POST', 'OPTIONS'], name='planning_api.update_plan', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def update_plan(request: Request):
    payload = get_payload(request)
    result = PlanningService().update_plan(payload, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/plan/copy', methods=['POST', 'OPTIONS'], name='planning_api.copy_plan', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def copy_plan(request: Request):
    payload = get_payload(request)
    payload['plan'] = TempPlanPayload(**payload['plan'])
    result = PlanningService().copy_plan(CopyPlanPayload(**payload), request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/plan/{plan_id}/delete', methods=['DELETE', 'OPTIONS'], name='planning_api.delete_plan', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def delete_plan(request: Request, plan_id: str):
    delete_view = bool(int(request.query_params.get('deleteView', '0')))
    result = PlanningService().delete_plan(plan_id, delete_view=delete_view, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/plan/{plan_id}/owner/set', methods=['POST', 'OPTIONS'], name='planning_api.change_plan_owner', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def change_plan_owner(request: Request, plan_id: str):
    payload = get_payload(request)
    result = PlanningService().change_plan_owner(plan_id=plan_id, new_owner=payload['newOwner'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/plan/{plan_id}/resolve_entities', methods=['GET', 'HEAD', 'OPTIONS'], name='planning_api.resolve_plan_entities', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def resolve_plan_entities(request: Request, plan_id: str):
    service = PlanningService()
    result = service.resolve_plan(plan_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/plan/trigger', methods=['POST', 'OPTIONS'], name='planning_api.trigger_jobs_for_plans', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def trigger_jobs_for_plans(request: Request):
    payload = get_payload(request)
    service = PlanningService()
    result = service.trigger_jobs(payload, request=request)
    return respond({'status': 'ok', 'response': result})
