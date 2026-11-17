from argus.backend.http import path_uuid
from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, redirect, authorize, NotFound
from pathlib import Path
from starlette.convertors import PathConvertor, register_url_convertor
from starlette.responses import FileResponse
import logging
from uuid import UUID
import magic
from coodie.exceptions import DocumentNotFound
from argus.backend.error_handlers import handle_api_exception
from argus.backend.models.web import ArgusTest
from argus.backend.service.github_service import GithubService
from argus.backend.service.issue_service import IssueService
from argus.backend.service.jenkins_service import JenkinsService
from argus.backend.service.jira_service import JiraService
from argus.backend.service.results_service import ResultsService
from argus.backend.service.testrun import TestRunService
from argus.backend.util.common import get_payload
from argus.common.enums import TestInvestigationStatus, TestStatus
bp = APIRouter(prefix='')
LOGGER = logging.getLogger(__name__)


class NonEmptyPathConvertor(PathConvertor):
    regex = '[^/].*'


register_url_convertor('nonempty_path', NonEmptyPathConvertor())


@bp.api_route('/testrun/{filename:nonempty_path}', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.static', response_model=None)
def static_file(request: Request, filename: str):
    # Retain the test-run router's public asset URL and its API error envelope.
    relative_path = Path(filename)
    if relative_path.is_absolute() or '..' in relative_path.parts:
        raise NotFound()
    file_path = Path(__file__).parent / 'testrun' / relative_path
    if not file_path.is_file():
        raise NotFound()
    return FileResponse(file_path)


@bp.api_route('/test/{test_id}/runs', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_runs_for_test', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_runs_for_test(request: Request, test_id: str):
    """
        Fetch test runs for a test using test id.
        This endpoint accepts the following query parameters
        limit (int) - How many runs from a partition to fetch
        before (float) - Timestamp to filter for runs submitted before it
        after (float) - Timestamp to filter for runs submitted after it
        full (str) - Any value here (for API usage purposes, '1') returns
                    the complete object instead of just meta fields
    """
    limit = request.query_params.get('limit')
    before = request.query_params.get('before')
    after = request.query_params.get('after')
    full = request.query_params.get('full')
    if not limit:
        limit = 10
    else:
        limit = int(limit)
    additional_runs = [UUID(run) for run in request.query_params.getlist('additionalRuns[]')]
    service = TestRunService()
    runs = service.get_runs_by_test_id(test_id=path_uuid(test_id), additional_runs=additional_runs, limit=limit, full=bool(full), before=before, after=after)
    return respond({'status': 'ok', 'response': runs})

@bp.api_route('/run/{run_id}/type', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_type_for_run', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_type_for_run(request: Request, run_id: str):
    service = TestRunService()
    run_type = service.get_test_type_for_run(run_id)
    return respond({'status': 'ok', 'response': {'run_type': run_type}})

@bp.api_route('/run/{run_type}/{run_id}', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_testrun', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_testrun(request: Request, run_type: str, run_id: str):
    run_id = path_uuid(run_id)
    service = TestRunService()
    test_run = service.get_run_response(run_type=run_type, run_id=run_id)
    return respond({'status': 'ok', 'response': test_run})

@bp.api_route('/run/{run_id}/activity', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.test_run_activity', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def test_run_activity(request: Request, run_id: str):
    run_id = path_uuid(run_id)
    service = TestRunService()
    activity = service.get_run_events(run_id=run_id)
    return respond({'status': 'ok', 'response': activity})

@bp.api_route('/run/{test_id}/{run_id}/fetch_results', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.fetch_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def fetch_results(request: Request, test_id: str, run_id: str):
    tables = ResultsService().get_run_results(test_id=path_uuid(test_id), run_id=path_uuid(run_id))
    return respond({'status': 'ok', 'tables': tables})

@bp.api_route('/test/{test_id}/run/{run_id}/status/set', methods=['POST', 'OPTIONS'], name='testrun_api.set_testrun_status', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def set_testrun_status(request: Request, test_id: str, run_id: str):
    payload = get_payload(client_request=request)
    new_status = payload.get('status')
    if not new_status:
        raise Exception('Status not specified in the request')
    service = TestRunService()
    result = service.change_run_status(test_id=path_uuid(test_id), run_id=path_uuid(run_id), new_status=TestStatus(new_status), request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/tests/{plugin_name}/{run_id}/log/{log_name}/download', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.download_log', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def download_log(request: Request, plugin_name: str, run_id: str, log_name: str):
    service = TestRunService()
    result = service.get_log(plugin_name=plugin_name, run_id=path_uuid(run_id), log_name=log_name)
    return respond(redirect(result, code=302))

@bp.api_route('/tests/{plugin_name}/{run_id}/screenshot/{image_name}', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.proxy_screenshot', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def proxy_screenshot(request: Request, plugin_name: str, run_id: str, image_name: str):
    service = TestRunService()
    result = service.proxy_stored_s3_image(plugin_name=plugin_name, run_id=path_uuid(run_id), image_name=image_name)
    return respond(redirect(result, code=302))

@bp.api_route('/test/{test_id}/run/{run_id}/investigation_status/set', methods=['POST', 'OPTIONS'], name='testrun_api.set_testrun_investigation_status', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def set_testrun_investigation_status(request: Request, test_id: str, run_id: str):
    payload = get_payload(client_request=request)
    new_status = payload.get('investigation_status')
    if not new_status:
        raise Exception('Status not specified in the request')
    service = TestRunService()
    result = service.change_run_investigation_status(test_id=path_uuid(test_id), run_id=path_uuid(run_id), new_status=TestInvestigationStatus(new_status), request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/test/{test_id}/run/{run_id}/assignee/set', methods=['POST', 'OPTIONS'], name='testrun_api.set_testrun_assignee', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def set_testrun_assignee(request: Request, test_id: str, run_id: str):
    payload = get_payload(client_request=request)
    assignee = payload.get('assignee')
    if not assignee:
        raise Exception('Assignee not specified in the request')
    service = TestRunService()
    result = service.change_run_assignee(test_id=path_uuid(test_id), run_id=path_uuid(run_id), new_assignee=UUID(assignee) if assignee != TestRunService.ASSIGNEE_PLACEHOLDER else None, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/test/{test_id}/run/{run_id}/issues/submit', methods=['POST', 'OPTIONS'], name='testrun_api.issues_submit', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def issues_submit(request: Request, test_id: str, run_id: str):
    payload = get_payload(client_request=request)
    service = IssueService()
    submit_result = service.submit(issue_url=payload['issue_url'], test_id=path_uuid(test_id), run_id=path_uuid(run_id), request=request)
    return respond({'status': 'ok', 'response': submit_result})

@bp.api_route('/test/{test_id}/run/{run_id}/issues/event/{event_id}/submit', methods=['POST', 'OPTIONS'], name='testrun_api.issues_submit_for_event', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def issues_submit_for_event(request: Request, test_id: str, run_id: str, event_id: str):
    payload = get_payload(client_request=request)
    service = IssueService()
    submit_result = service.submit_for_sct_event(issue_url=payload['issue_url'], test_id=path_uuid(test_id), event_id=path_uuid(event_id), run_id=path_uuid(run_id), request=request)
    return respond({'status': 'ok', 'response': submit_result})

@bp.api_route('/issues/get', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.issues_get', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def issues_get(request: Request):
    filter_key = request.query_params.get('filterKey')
    if not filter_key:
        raise Exception('Filter key not provided in the request')
    key_value = request.query_params.get('id')
    if not key_value:
        raise Exception("Id wasn't provided in the request")
    key_value = UUID(key_value)
    aggregate_by_issue = request.query_params.get('aggregateByIssue')
    aggregate_by_issue = bool(int(aggregate_by_issue)) if aggregate_by_issue else False
    product_version = request.query_params.get('productVersion') or None
    include_no_version = bool(int(request.query_params.get('includeNoVersion', 0)))
    service = IssueService()
    issues = service.get(filter_key=filter_key, filter_id=key_value, aggregate_by_issue=aggregate_by_issue, product_version=product_version, include_no_version=include_no_version)
    return respond({'status': 'ok', 'response': issues})

@bp.api_route('/issues/delete', methods=['POST', 'OPTIONS'], name='testrun_api.issues_delete', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def issues_delete(request: Request):
    payload = get_payload(client_request=request)
    service = IssueService()
    result = service.delete(issue_id=payload['issue_id'], run_id=payload['run_id'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/run/{run_id}/comments', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_testrun_comments', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_testrun_comments(request: Request, run_id: str):
    service = TestRunService()
    comments = service.get_run_comments(run_id=path_uuid(run_id))
    return respond({'status': 'ok', 'response': comments})

@bp.api_route('/run/{run_id}/pytest/results', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_testrun_pytest_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_testrun_pytest_results(request: Request, run_id: str):
    service = TestRunService()
    res = service.get_pytest_run_results(run_id=path_uuid(run_id))
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/comment/{comment_id}/get', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_single_comment', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_single_comment(request: Request, comment_id: str):
    service = TestRunService()
    comment = service.get_run_comment(comment_id=path_uuid(comment_id))
    return respond({'status': 'ok', 'response': comment})

@bp.api_route('/test/{test_id}/run/{run_id}/comments/submit', methods=['POST', 'OPTIONS'], name='testrun_api.submit_testrun_comment', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def submit_testrun_comment(request: Request, test_id: str, run_id: str):
    payload = get_payload(request)
    service = TestRunService()
    result = service.post_run_comment(test_id=path_uuid(test_id), run_id=path_uuid(run_id), message=payload['message'], reactions=payload['reactions'], mentions=payload['mentions'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/test/{test_id}/run/{run_id}/comment/{comment_id}/update', methods=['POST', 'OPTIONS'], name='testrun_api.test_run_update_comment', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def test_run_update_comment(request: Request, test_id: str, run_id: str, comment_id: str):
    payload = get_payload(request)
    service = TestRunService()
    result = service.update_run_comment(test_id=path_uuid(test_id), run_id=path_uuid(run_id), comment_id=path_uuid(comment_id), message=payload['message'], reactions=payload['reactions'], mentions=payload['mentions'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/test/{test_id}/run/{run_id}/comment/{comment_id}/delete', methods=['POST', 'OPTIONS'], name='testrun_api.test_run_delete_comment', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def test_run_delete_comment(request: Request, test_id: str, run_id: str, comment_id: str):
    service = TestRunService()
    result = service.delete_run_comment(test_id=path_uuid(test_id), run_id=path_uuid(run_id), comment_id=path_uuid(comment_id), request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/terminate_stuck_runs', methods=['POST', 'OPTIONS'], name='testrun_api.sct_terminate_stuck_runs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def sct_terminate_stuck_runs(request: Request):
    result = TestRunService().terminate_stuck_runs(request=request)
    return respond({'status': 'ok', 'response': {'total': result}})

@bp.api_route('/ignore_jobs', methods=['POST', 'OPTIONS'], name='testrun_api.ignore_jobs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def ignore_jobs(request: Request):
    payload = get_payload(request)
    service = TestRunService()
    result = service.ignore_jobs(test_id=UUID(payload['testId']), reason=payload['reason'], request=request)
    return respond({'status': 'ok', 'response': {'affectedJobs': result}})

@bp.api_route('/get_runs_by_test_id_run_id', methods=['POST', 'OPTIONS'], name='testrun_api.get_runs_by_test_id_run_id', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_runs_by_test_id_run_id(request: Request):
    payload: list[tuple[UUID, UUID]] = get_payload(request)
    service = TestRunService()
    result = service.resolve_run_build_id_and_number_multiple(payload)
    return respond({'status': 'ok', 'response': {'runs': result}})

@bp.api_route('/jenkins/params', methods=['POST', 'OPTIONS'], name='testrun_api.get_jenkins_job_params', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_jenkins_job_params(request: Request):
    payload = get_payload(request)
    service = JenkinsService()
    result = service.retrieve_job_parameters(build_id=payload['buildId'], build_number=payload['buildNumber'], from_defaults=payload.get('fromDefaults', False))
    return respond({'status': 'ok', 'response': {'parameters': result}})

@bp.api_route('/jenkins/build', methods=['POST', 'OPTIONS'], name='testrun_api.build_jenkins_job', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def build_jenkins_job(request: Request):
    payload = get_payload(request)
    service = JenkinsService()
    try:
        test = ArgusTest.get(build_system_id=payload['buildId'])
    except DocumentNotFound:
        test = None
    if test and test.plugin_name == 'scylla-cluster-tests':
        JenkinsService.validate_sct_version_source(payload['parameters'])
    result = service.build_job(build_id=payload['buildId'], params=payload['parameters'], request=request)
    response = {'queueItem': result}
    if payload.get('includeBuildNumber'):
        next_build_number = service.next_build_number(build_id=payload['buildId'])
        if next_build_number > 0:
            response['nextBuildNumber'] = next_build_number
    return respond({'status': 'ok', 'response': response})

@bp.api_route('/jenkins/queue_info', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_queue_info', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_queue_info(request: Request):
    queue_item = request.query_params.get('queueItem')
    if not queue_item:
        raise Exception('No queueItem provided')
    service = JenkinsService()
    result = service.get_queue_info(int(queue_item), request=request)
    return respond({'status': 'ok', 'response': {'queueItem': result}})

@bp.api_route('/jenkins/clone/targets', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_clone_targets', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_clone_targets(request: Request):
    test_id = request.query_params.get('testId')
    if not test_id:
        raise Exception('No testId provided')
    service = JenkinsService()
    result = service.get_releases_for_clone(test_id)
    return respond({'status': 'ok', 'response': {'targets': result}})

@bp.api_route('/jenkins/clone/groups', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_groups_for_target', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_groups_for_target(request: Request):
    target_id = request.query_params.get('targetId')
    if not target_id:
        raise Exception('No targetId provided')
    service = JenkinsService()
    result = service.get_groups_for_release(target_id, request=request)
    return respond({'status': 'ok', 'response': {'groups': result}})

@bp.api_route('/jenkins/clone/create', methods=['POST', 'OPTIONS'], name='testrun_api.clone_jenkins_job', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def clone_jenkins_job(request: Request):
    payload = get_payload(request)
    service = JenkinsService()
    result = service.clone_job(current_test_id=payload['currentTestId'], new_name=payload['newName'], target=payload['target'], group=payload['group'], advanced_settings=payload['advancedSettings'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/jenkins/clone/build', methods=['POST', 'OPTIONS'], name='testrun_api.clone_build_jenkins_job', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def clone_build_jenkins_job(request: Request):
    payload = get_payload(request)
    service = JenkinsService()
    result = service.clone_build_job(build_id=payload['buildId'], params=payload['parameters'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/jenkins/clone/settings', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_clone_job_advanced_settings', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_clone_job_advanced_settings(request: Request):
    build_id = request.query_params.get('buildId')
    if not build_id:
        raise Exception('No testId provided')
    service = JenkinsService()
    result = service.get_advanced_settings(build_id)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/jenkins/clone/settings/change', methods=['POST', 'OPTIONS'], name='testrun_api.set_job_settings', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def set_job_settings(request: Request):
    payload = get_payload(request)
    service = JenkinsService()
    test = ArgusTest.get(build_system_id=payload['buildId'])
    result = service.adjust_job_settings(build_id=test.build_system_id, plugin_name=test.plugin_name, settings=payload['settings'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/jenkins/clone/settings/validate', methods=['POST', 'OPTIONS'], name='testrun_api.clone_validate_new_settings', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def clone_validate_new_settings(request: Request):
    payload = get_payload(request)
    service = JenkinsService()
    result = service.verify_job_settings(build_id=payload['buildId'], new_settings=payload['newSettings'])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/pytest/{test_name}/results', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_pytest_test_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_pytest_test_results(request: Request, test_name: str):
    before = request.query_params.get('before')
    after = request.query_params.get('after')
    result = TestRunService().get_pytest_test_results(test_name=test_name, before=before, after=after, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/pytest/{test_name}/stats/{field_name}/{aggr_function}', methods=['GET', 'HEAD', 'OPTIONS'], name='testrun_api.get_pytest_test_field_stats', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
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
