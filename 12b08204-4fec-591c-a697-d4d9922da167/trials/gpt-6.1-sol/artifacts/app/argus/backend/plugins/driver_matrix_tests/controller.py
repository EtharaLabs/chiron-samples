from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from argus.backend.error_handlers import handle_api_exception
from argus.backend.plugins.driver_matrix_tests.raw_types import DriverMatrixSubmitEnvRequest, DriverMatrixSubmitFailureRequest, DriverMatrixSubmitResultRequest
from argus.backend.plugins.driver_matrix_tests.service import DriverMatrixService
from argus.backend.util.common import get_payload
bp = APIRouter(prefix='/driver_matrix')

@bp.api_route('/test_report', methods=['GET', 'HEAD', 'OPTIONS'], name='driver_matrix_api.driver_matrix_test_report', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def driver_matrix_test_report(request: Request):
    build_id = request.query_params.get('buildId')
    if not build_id:
        raise Exception('No build id provided')
    result = DriverMatrixService().tested_versions_report(build_id=build_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/result/submit', methods=['POST', 'OPTIONS'], name='driver_matrix_api.submit_result', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def submit_result(request: Request):
    payload = get_payload(request)
    request_data = DriverMatrixSubmitResultRequest(**payload)
    result = DriverMatrixService().submit_driver_result(driver_name=request_data.driver_name, driver_type=request_data.driver_type, run_id=request_data.run_id, raw_xml=request_data.raw_xml)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/result/fail', methods=['POST', 'OPTIONS'], name='driver_matrix_api.submit_failure', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def submit_failure(request: Request):
    payload = get_payload(request)
    request_data = DriverMatrixSubmitFailureRequest(**payload)
    result = DriverMatrixService().submit_driver_failure(driver_name=request_data.driver_name, driver_type=request_data.driver_type, run_id=request_data.run_id, failure_reason=request_data.failure_reason)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/env/submit', methods=['POST', 'OPTIONS'], name='driver_matrix_api.submit_env', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def submit_env(request: Request):
    payload = get_payload(request)
    request_data = DriverMatrixSubmitEnvRequest(**payload)
    result = DriverMatrixService().submit_env_info(run_id=request_data.run_id, raw_env=request_data.raw_env)
    return respond({'status': 'ok', 'response': result})
