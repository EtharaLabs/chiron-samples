from argus.backend.http import json_body
from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from datetime import UTC, datetime
from argus.backend.error_handlers import handle_api_exception
from argus.backend.plugins.sct.testrun import SCTEventSeverity
from argus.backend.plugins.sct.service import SCTService, SCTServiceException
from argus.backend.util.common import get_payload
bp = APIRouter(prefix='/sct')

@bp.api_route('/{run_id}/packages/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_submit_packages', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_submit_packages(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.submit_packages(run_id=run_id, packages=payload['packages'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/screenshots/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_submit_screenshots', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_submit_screenshots(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.submit_screenshots(run_id=run_id, screenshot_links=payload['screenshot_links'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/sct_runner/set', methods=['POST', 'OPTIONS'], name='sct_api.sct_set_runner', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_set_runner(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.set_sct_runner(run_id=run_id, public_ip=payload['public_ip'], private_ip=payload['private_ip'], region=payload['region'], backend=payload['backend'], name=payload.get('name'), request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/resource/all', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_resource_all', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_resource_all(request: Request, run_id: str):
    result = SCTService.get_resources(run_id=run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/resource/{name}/get', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_resource_get', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_resource_get(request: Request, run_id: str, name: str):
    result = SCTService.get_resource(run_id=run_id, name=name)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/resource/create', methods=['POST', 'OPTIONS'], name='sct_api.sct_resource_create', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_resource_create(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.create_resource(run_id=run_id, resource_details=payload['resource'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/resource/{resource_name}/terminate', methods=['POST', 'OPTIONS'], name='sct_api.sct_resource_terminate', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_resource_terminate(request: Request, run_id: str, resource_name: str):
    payload = get_payload(request)
    result = SCTService.terminate_resource(run_id=run_id, resource_name=resource_name, reason=payload['reason'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/resource/{resource_name}/shards', methods=['POST', 'OPTIONS'], name='sct_api.sct_resource_update_shards', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_resource_update_shards(request: Request, run_id: str, resource_name: str):
    payload = get_payload(request)
    result = SCTService.update_resource_shards(run_id=run_id, resource_name=resource_name, new_shards=payload['shards'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/resource/{resource_name}/update', methods=['POST', 'OPTIONS'], name='sct_api.sct_resource_update', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_resource_update(request: Request, run_id: str, resource_name: str):
    payload = get_payload(request)
    result = SCTService.update_resource(run_id=run_id, resource_name=resource_name, update_data=payload['update_data'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/nemesis/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_nemesis_submit', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_nemesis_submit(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.submit_nemesis(run_id=run_id, nemesis_details=payload['nemesis'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/nemesis/get', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_nemesis_get', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_nemesis_get(request: Request, run_id: str):
    result = SCTService.get_nemesis(run_id=run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/nemesis/finalize', methods=['POST', 'OPTIONS'], name='sct_api.sct_nemesis_finalize', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_nemesis_finalize(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.finalize_nemesis(run_id=run_id, nemesis_details=payload['nemesis'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/events/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_events_submit', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_events_submit(request: Request, run_id: str):
    """
        Legacy endpoint. Deprecated
        Submit a structure of EventsBySeverity that will be saved
        onto SCTTestRun
    """
    payload = get_payload(request)
    result = SCTService.submit_events(run_id=run_id, events=payload['events'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/events/get', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_events_get', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_events_get(request: Request, run_id: str):
    limit = int(request.query_params.get('limit', 100))
    before = request.query_params.get('before')
    after = request.query_params.get('after')
    severities = request.query_params.getlist('severity')
    result = SCTService.get_events(run_id=run_id, limit=limit, before=before, after=after, severities=severities)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/events/{severity}/get', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_events_get_by_severity', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_events_get_by_severity(request: Request, run_id: str, severity: str):
    limit = int(request.query_params.get('limit', 100))
    before = request.query_params.get('before')
    after = request.query_params.get('after')
    result = SCTService.get_events(run_id=run_id, limit=limit, before=before, after=after, severities=[SCTEventSeverity(severity)])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/events/{severity}/count', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_events_count_by_severity', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_events_count_by_severity(request: Request, run_id: str, severity: str):
    result = SCTService.count_events_by_severity(run_id=run_id, severity=SCTEventSeverity(severity))
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/event/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_event_submit', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_event_submit(request: Request, run_id: str):
    """
        Submit an event or a collection of events
    """
    payload = get_payload(request)
    event_data = payload['data']
    if isinstance(event_data, list):
        result = all([SCTService.submit_event(run_id=run_id, raw_event=e) for e in event_data])
    else:
        result = SCTService.submit_event(run_id=run_id, raw_event=event_data)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/gemini/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_gemini_results_submit', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_gemini_results_submit(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.submit_gemini_results(run_id=run_id, gemini_data=payload['gemini_data'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/performance/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_performance_results_submit', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_performance_results_submit(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.submit_performance_results(run_id=run_id, performance_results=payload['performance_results'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/performance/history', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_get_performance_history', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_get_performance_history(request: Request, run_id: str):
    result = SCTService.get_performance_history_for_test(run_id=run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/release/{release_name:path}/kernels', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_get_kernel_report', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_get_kernel_report(request: Request, release_name: str):
    result = SCTService.get_scylla_version_kernels_report(release_name=release_name)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/junit/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_submit_junit_report', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_submit_junit_report(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.junit_submit(run_id, payload['file_name'], payload['content'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/stress_cmd/submit', methods=['POST', 'OPTIONS'], name='sct_api.sct_add_stress_cmd', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_add_stress_cmd(request: Request, run_id: str):
    payload = get_payload(request)
    result = SCTService.add_stress_command(run_id, cmd=payload['cmd'], ts=payload.get('ts', datetime.now(UTC).timestamp()), loader_name=payload['loader_name'], log_name=payload['log_name'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/stress_cmd/get', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_get_all_stress_cmds', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_get_all_stress_cmds(request: Request, run_id: str):
    result = SCTService.get_stress_commands(run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/similar_events', methods=['GET', 'HEAD', 'OPTIONS'], name='sct_api.sct_get_similar_events', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_get_similar_events(request: Request, run_id: str):
    result = SCTService.get_similar_events(run_id=run_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{run_id}/event/similar', methods=['POST', 'OPTIONS'], name='sct_api.sct_get_similar_events_realtime', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_get_similar_events_realtime(request: Request, run_id: str):
    """Get similar events for a specific event using real-time vector search"""
    payload = get_payload(request)
    severity = payload.get('severity')
    ts = payload.get('ts')
    limit = int(payload.get('limit', 100))
    err_message = ''
    if not severity and (not ts):
        err_message = 'Missing required parameters: severity and ts'
    elif not severity:
        err_message = 'Missing required parameter: severity'
    elif not ts:
        err_message = 'Missing required parameter: ts'
    if err_message:
        raise SCTServiceException(err_message)
    result = SCTService.get_similar_events_realtime(run_id=run_id, severity=severity, ts=ts, limit=limit)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/similar_runs_info', methods=['POST', 'OPTIONS'], name='sct_api.sct_get_similar_runs_info', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_get_similar_runs_info(request: Request):
    """Get build IDs and issues for a list of run IDs"""
    data = json_body(request)
    if not data or 'run_ids' not in data:
        return respond(({'status': 'error', 'response': 'Missing run_ids parameter'}, 400))
    run_ids = data['run_ids']
    if not isinstance(run_ids, list):
        return respond(({'status': 'error', 'response': 'run_ids must be a list'}, 400))
    result = SCTService.get_similar_runs_info(run_ids=run_ids)
    return respond({'status': 'ok', 'response': result})
