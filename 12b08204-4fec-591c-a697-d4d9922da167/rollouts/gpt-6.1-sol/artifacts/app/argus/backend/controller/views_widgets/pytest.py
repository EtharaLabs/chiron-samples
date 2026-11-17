from argus.backend.http import path_uuid
from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from uuid import UUID
from argus.backend.service.views_widgets.pytest import PytestViewService
bp = APIRouter(prefix='/widgets')

@bp.api_route('/pytest/view', methods=['GET', 'HEAD', 'OPTIONS'], name='pytest.get_versioned_runs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_versioned_runs(request: Request):
    service = PytestViewService()
    return respond({'status': 'ok', 'response': 0})

@bp.api_route('/pytest/release/{release_id}/results', methods=['GET', 'HEAD', 'OPTIONS'], name='pytest.get_release_pytest_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_release_pytest_results(request: Request, release_id: str):
    release_id = path_uuid(release_id)
    service = PytestViewService()
    res = service.release_results(release_id, request=request)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/pytest/view/{view_id}/results', methods=['GET', 'HEAD', 'OPTIONS'], name='pytest.get_view_pytest_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_view_pytest_results(request: Request, view_id: str):
    service = PytestViewService()
    res = service.view_results(view_id, request=request)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/pytest/results', methods=['GET', 'HEAD', 'OPTIONS'], name='pytest.get_pytest_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_pytest_results(request: Request):
    service = PytestViewService()
    res = service.result_filter(request=request)
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/pytest/{test_name:path}/{id}/fields', methods=['GET', 'HEAD', 'OPTIONS'], name='pytest.get_user_fields_for_test', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_user_fields_for_test(request: Request, test_name: str, id: str):
    service = PytestViewService()
    res = service.get_user_fields_for_result(test_name, id)
    return respond({'status': 'ok', 'response': res})
