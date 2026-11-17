from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from argus.backend.error_handlers import handle_api_exception
from argus.backend.service.email_service import EmailService
from argus.backend.service.testrun import TestRunService
from argus.backend.service.client_service import ClientService
from argus.backend.util.common import get_payload
from argus.backend.plugins.loader import AVAILABLE_PLUGINS
from argus.backend.controller.ssh_api import bp as ssh_bp
from argus.backend.controller.replay_api import bp as replay_bp
bp = APIRouter(prefix='/client')
bp.include_router(ssh_bp)
bp.include_router(replay_bp)
for plugin in AVAILABLE_PLUGINS.values():
    if plugin.controller:
        bp.include_router(plugin.controller)

@bp.api_route('/testrun/{run_id}/info', methods=['GET', 'HEAD', 'OPTIONS'], name='client_api.get_run_info', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_run_info(request: Request, run_id: str):
    result = ClientService().get_run_info(run_id=run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/submit', methods=['POST', 'OPTIONS'], name='client_api.submit_run', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def submit_run(request: Request, run_type: str):
    payload = get_payload(request)
    result = ClientService().submit_run(run_type=run_type, request_data=payload, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/{run_id}/get', methods=['GET', 'HEAD', 'OPTIONS'], name='client_api.get_run', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_run(request: Request, run_type: str, run_id: str):
    result = ClientService().get_run(run_type=run_type, run_id=run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/{run_id}/heartbeat', methods=['POST', 'OPTIONS'], name='client_api.run_heartbeat', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def run_heartbeat(request: Request, run_type: str, run_id: str):
    result = ClientService().heartbeat(run_type=run_type, run_id=run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/{run_id}/get_status', methods=['GET', 'HEAD', 'OPTIONS'], name='client_api.run_get_status', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def run_get_status(request: Request, run_type: str, run_id: str):
    result = ClientService().get_run_status(run_type=run_type, run_id=run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/{run_id}/set_status', methods=['POST', 'OPTIONS'], name='client_api.run_set_status', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def run_set_status(request: Request, run_type: str, run_id: str):
    payload = get_payload(request)
    result = ClientService().update_run_status(run_type=run_type, run_id=run_id, new_status=payload['new_status'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/{run_id}/update_product_version', methods=['POST', 'OPTIONS'], name='client_api.run_update_product_version', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def run_update_product_version(request: Request, run_type: str, run_id: str):
    payload = get_payload(request)
    result = ClientService().submit_product_version(run_type=run_type, run_id=run_id, version=payload['product_version'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/{run_id}/logs/submit', methods=['POST', 'OPTIONS'], name='client_api.run_submit_logs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def run_submit_logs(request: Request, run_type: str, run_id: str):
    payload = get_payload(request)
    result = ClientService().submit_logs(run_type=run_type, run_id=run_id, logs=payload['logs'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/config/submit', methods=['POST', 'OPTIONS'], name='client_api.submit_run_config', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def submit_run_config(request: Request, run_id: str):
    payload = get_payload(request)
    result = ClientService().submit_config(run_id, config_name=payload['name'], config_content=payload['content'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/config/all', methods=['GET', 'HEAD', 'OPTIONS'], name='client_api.get_all_run_configs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_all_run_configs(request: Request, run_id: str):
    result = ClientService().get_all_configs(run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/{run_id}/finalize', methods=['POST', 'OPTIONS'], name='client_api.run_finalize', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def run_finalize(request: Request, run_type: str, run_id: str):
    try:
        payload = get_payload(request)
    except Exception:
        payload = None
    result = ClientService().finish_run(run_type=run_type, run_id=run_id, payload=payload, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/{run_type}/{run_id}/submit_results', methods=['POST', 'OPTIONS'], name='client_api.submit_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def submit_results(request: Request, run_type: str, run_id: str):
    payload = get_payload(request)
    return respond(ClientService().submit_results(run_type=run_type, run_id=run_id, results=payload, request=request))

@bp.api_route('/testrun/pytest/result/submit', methods=['POST', 'OPTIONS'], name='client_api.submit_pytest_result', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def submit_pytest_result(request: Request):
    payload = get_payload(request)
    result = ClientService().submit_pytest_result(request_data=payload, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/pytest/{test_name}/stats/{field_name}/{aggr_function}', methods=['GET', 'HEAD', 'OPTIONS'], name='client_api.get_pytest_test_field_stats', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_pytest_test_field_stats(request: Request, test_name: str, field_name: str, aggr_function: str):
    """
        Method: GET
        Params:
            test_name: name of a pytest unit, for example "sample.py::TestSample::test_sampe"
            field_name: a field inside PytestResultTable that supports aggregation, e.g. duration
            aggr_function: Supported: avg, count, min, max - which function to use for the aggregate
    """
    result = TestRunService().get_pytest_test_field_stats(test_name=test_name, field_name=field_name, aggr_function=aggr_function, query=dict(request.query_params), request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/report/email', methods=['POST', 'OPTIONS'], name='client_api.send_email_report', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def send_email_report(request: Request):
    payload = get_payload(request)
    result = EmailService().send_report(request_data=payload, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/testrun/report', methods=['POST', 'OPTIONS'], name='client_api.render_email_report', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def render_email_report(request: Request):
    payload = get_payload(request)
    result = EmailService().display_report(request_data=payload, request=request)
    return respond(result)
