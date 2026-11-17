from argus.backend.http import json_body
from argus.backend.http import path_uuid
from fastapi import APIRouter, Request, Depends
from starlette.responses import Response
from argus.backend.http import respond, jsonify, redirect, url_for, require_json, authorize
from argus.backend.util.config import Config
import logging
from datetime import datetime, timezone
from uuid import UUID
import requests
from argus.backend.error_handlers import APIException, handle_api_exception
from argus.backend.controller.notification_api import bp as notifications_bp
from argus.backend.controller.client_api import bp as client_bp
from argus.backend.controller.testrun_api import bp as testrun_bp
from argus.backend.controller.team import bp as team_bp
from argus.backend.controller.view_api import bp as view_bp
from argus.backend.controller.planner_api import bp as planner_bp
from argus.backend.service.argus_service import ArgusService, ScheduleUpdateRequest
from argus.backend.service.results_service import ResultsService
from argus.backend.service.testrun import TestRunService
from argus.backend.service.user import UserService
from argus.backend.service.stats import ReleaseStatsCollector
from argus.backend.models.web import ArgusRelease, ArgusGroup, ArgusTest, User, UserOauthToken
from argus.backend.util.common import get_payload
bp = APIRouter(prefix='/api/v1')
bp.include_router(notifications_bp)
bp.include_router(client_bp)
bp.include_router(testrun_bp)
bp.include_router(team_bp)
bp.include_router(view_bp)
bp.include_router(planner_bp)
LOGGER = logging.getLogger(__name__)

@bp.api_route('/version', methods=['GET', 'HEAD', 'OPTIONS'], name='api.app_version', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def app_version(request: Request):
    service = ArgusService()
    argus_version = service.get_version()
    return respond(jsonify({'status': 'ok', 'response': {'commit_id': argus_version}}))

@bp.api_route('/test/{build_id:path}/{build_number:int}', methods=['GET', 'HEAD', 'OPTIONS'], name='api.get_run_by_build', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_run_by_build(request: Request, build_id: str, build_number: int):
    run = TestRunService().get_run_by_build_number(build_id, build_number)
    if not run:
        raise Exception(f'Run not found for {build_id} #{build_number}')
    return respond(jsonify({'status': 'ok', 'response': {'run_id': str(run.id), 'plugin_name': run._plugin_name, 'url': url_for(request, 'main.get_run_by_plugin', plugin_name=run._plugin_name, run_id=run.id, _external=True)}}))

@bp.api_route('/releases', methods=['GET', 'HEAD', 'OPTIONS'], name='api.releases', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def releases(request: Request):
    service = ArgusService()
    force_all = request.query_params.get('all', False)
    all_releases = service.get_releases()
    response = jsonify({'status': 'ok', 'response': [d.model_dump() for d in all_releases if d.enabled or force_all]})
    return respond(response)

@bp.api_route('/release/activity', methods=['GET', 'HEAD', 'OPTIONS'], name='api.release_activity', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_activity(request: Request):
    release_name = request.query_params.get('releaseName')
    if not release_name:
        raise Exception('Release name not specified in the request')
    service = ArgusService()
    activity_data = service.fetch_release_activity(release_name)
    return respond(jsonify({'status': 'ok', 'response': activity_data}))

@bp.api_route('/release/planner/data', methods=['GET', 'HEAD', 'OPTIONS'], name='api.release_planner_data', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_planner_data(request: Request):
    release_id = request.query_params.get('releaseId')
    if not release_id:
        raise Exception('Release Id not specified')
    service = ArgusService()
    planner_data = service.get_planner_data(release_id)
    return respond(jsonify({'status': 'ok', 'response': planner_data}))

@bp.api_route('/release/{release_id}/versions', methods=['GET', 'HEAD', 'OPTIONS'], name='api.release_versions', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_versions(request: Request, release_id: str):
    release_id = path_uuid(release_id)
    service = ArgusService()
    distinct_versions = service.get_distinct_release_versions(release_id=release_id)
    return respond(jsonify({'status': 'ok', 'response': distinct_versions}))

@bp.api_route('/release/{release_id}/pytest/results', methods=['GET', 'HEAD', 'OPTIONS'], name='api.release_pytest_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_pytest_results(request: Request, release_id: str):
    release_id = path_uuid(release_id)
    service = TestRunService()
    res = service.get_pytest_release_results(release_id=release_id)
    return respond(jsonify({'status': 'ok', 'response': res}))

@bp.api_route('/release/{release_id}/images', methods=['GET', 'HEAD', 'OPTIONS'], name='api.release_images', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_images(request: Request, release_id: str):
    release_id = path_uuid(release_id)
    service = ArgusService()
    distinct_images = service.get_distinct_release_images(release_id=release_id)
    return respond(jsonify({'status': 'ok', 'response': distinct_images}))

@bp.api_route('/release/planner/comment/get/test', methods=['GET', 'HEAD', 'OPTIONS'], name='api.get_planner_comment_by_test', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def get_planner_comment_by_test(request: Request):
    test_id = request.query_params.get('id')
    if not test_id:
        raise Exception('TestId was not specified')
    service = ArgusService()
    planner_comments_by_test = service.get_planner_comment_by_test(UUID(test_id), request=request)
    response = jsonify({'status': 'ok', 'response': planner_comments_by_test})
    return respond(response)

@bp.api_route('/release/schedules/comment/update', methods=['POST', 'OPTIONS'], name='api.release_schedules_comment_update', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_schedules_comment_update(request: Request):
    require_json(request)
    request_payload = json_body(request)
    service = ArgusService()
    comment_update_result = service.update_schedule_comment(request_payload)
    return respond(jsonify({'status': 'ok', 'response': comment_update_result}))

@bp.api_route('/release/schedules', methods=['GET', 'HEAD', 'OPTIONS'], name='api.release_schedules', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_schedules(request: Request):
    release = request.query_params.get('releaseId')
    if not release:
        raise Exception('No releaseId provided')
    service = ArgusService()
    release_schedules_data = service.get_schedules_for_release(release)
    return respond(jsonify({'status': 'ok', 'response': release_schedules_data}))

@bp.api_route('/release/schedules/assignee/update', methods=['POST', 'OPTIONS'], name='api.release_schedules_assignee_update', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_schedules_assignee_update(request: Request):
    require_json(request)
    request_payload = json_body(request)
    service = ArgusService()
    assignee_update_status = service.update_schedule_assignees(request_payload)
    return respond(jsonify({'status': 'ok', 'response': assignee_update_status}))

@bp.api_route('/release/assignees/groups', methods=['GET', 'HEAD', 'OPTIONS'], name='api.group_assignees', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def group_assignees(request: Request):
    release_id = request.query_params.get('releaseId')
    version = request.query_params.get('version')
    plan_id = request.query_params.get('planId')
    if not release_id:
        raise Exception('Missing releaseId')
    service = ArgusService()
    group_assignees_list = service.get_groups_assignees(release_id, version, plan_id)
    return respond(jsonify({'status': 'ok', 'response': group_assignees_list}))

@bp.api_route('/release/assignees/tests', methods=['GET', 'HEAD', 'OPTIONS'], name='api.tests_assignees', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def tests_assignees(request: Request):
    group_id = request.query_params.get('groupId')
    version = request.query_params.get('version')
    plan_id = request.query_params.get('planId')
    if not group_id:
        raise Exception('Missing groupId')
    service = ArgusService()
    tests_assignees_list = service.get_tests_assignees(group_id, version, plan_id)
    return respond(jsonify({'status': 'ok', 'response': tests_assignees_list}))

@bp.api_route('/release/schedules/submit', methods=['POST', 'OPTIONS'], name='api.release_schedules_submit', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_schedules_submit(request: Request):
    require_json(request)
    payload = json_body(request)
    service = ArgusService()
    schedule_submit_result = service.submit_new_schedule(release=payload['releaseId'], start_time=payload['start'], end_time=payload['end'], tests=payload['tests'], groups=payload['groups'], assignees=payload['assignees'], tag=payload['tag'], comments=payload.get('comments'), group_ids=payload.get('groupIds'))
    return respond(jsonify({'status': 'ok', 'response': schedule_submit_result}))

@bp.api_route('/release/schedules/delete', methods=['POST', 'OPTIONS'], name='api.release_schedules_delete', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_schedules_delete(request: Request):
    require_json(request)
    request_payload = json_body(request)
    service = ArgusService()
    schedule_delete_result = service.delete_schedule(request_payload)
    return respond(jsonify({'status': 'ok', 'response': schedule_delete_result}))

@bp.api_route('/release/schedules/update', methods=['POST', 'OPTIONS'], name='api.release_schedule_update', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_schedule_update(request: Request):
    payload = get_payload(request)
    req = ScheduleUpdateRequest(**payload)
    service = ArgusService()
    update_result = service.update_schedule(release_id=req.release_id, schedule_id=req.schedule_id, old_tests=req.old_tests, new_tests=req.new_tests, comments=req.comments, assignee=req.assignee)
    return respond(jsonify({'status': 'ok', 'response': update_result}))

@bp.api_route('/groups', methods=['GET', 'HEAD', 'OPTIONS'], name='api.argus_groups', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def argus_groups(request: Request):
    release_id = request.query_params.get('releaseId')
    if not release_id:
        raise Exception('No releaseId provided')
    force_all = request.query_params.get('all', False)
    service = ArgusService()
    groups = service.get_groups(UUID(release_id))
    result_groups = [g.model_dump() for g in groups if g.enabled or force_all]
    response = jsonify({'status': 'ok', 'response': result_groups})
    return respond(response)

@bp.api_route('/tests', methods=['GET', 'HEAD', 'OPTIONS'], name='api.argus_tests', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def argus_tests(request: Request):
    group_id = request.query_params.get('groupId')
    if not group_id:
        raise Exception('No groupId provided')
    force_all = request.query_params.get('all', False)
    service = ArgusService()
    tests = service.get_tests(group_id=UUID(group_id))
    result_tests = [t.model_dump() for t in tests if t.enabled or force_all]
    response = jsonify({'status': 'ok', 'response': result_tests})
    return respond(response)

@bp.api_route('/release/{release_id}/details', methods=['GET', 'HEAD', 'OPTIONS'], name='api.get_release_details', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_release_details(request: Request, release_id: str):
    release = ArgusRelease.get(id=path_uuid(release_id))
    response = jsonify({'status': 'ok', 'response': release})
    return respond(response)

@bp.api_route('/group/{group_id}/details', methods=['GET', 'HEAD', 'OPTIONS'], name='api.get_group_details', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_group_details(request: Request, group_id: str):
    group = ArgusGroup.get(id=path_uuid(group_id))
    response = jsonify({'status': 'ok', 'response': group})
    return respond(response)

@bp.api_route('/test/{test_id}/details', methods=['GET', 'HEAD', 'OPTIONS'], name='api.get_test_details', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_test_details(request: Request, test_id: str):
    test = ArgusTest.get(id=path_uuid(test_id))
    response = jsonify({'status': 'ok', 'response': test})
    return respond(response)

@bp.api_route('/test/{test_id}/set_plugin', methods=['POST', 'OPTIONS'], name='api.set_test_plugin', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def set_test_plugin(request: Request, test_id: str):
    payload = get_payload(request)
    current_user: User = request.state.user
    test: ArgusTest = ArgusTest.get(id=path_uuid(test_id))
    test.plugin_name = payload['plugin_name']
    test.save()
    return respond({'status': 'ok', 'response': test})

@bp.api_route('/test-info', methods=['GET', 'HEAD', 'OPTIONS'], name='api.test_info', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def test_info(request: Request):
    test_id = request.query_params.get('testId')
    if not test_id:
        raise Exception('No testId provided')
    service = ArgusService()
    info = service.get_test_info(test_id=UUID(test_id))
    return respond({'status': 'ok', 'response': info})

@bp.api_route('/test-results', methods=['GET', 'HEAD', 'OPTIONS'], name='api.test_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def test_results(request: Request):
    test_id = request.query_params.get('testId')
    start_date_str = request.query_params.get('startDate')
    end_date_str = request.query_params.get('endDate')
    table_names = request.query_params.getlist('tableNames[]')
    if not test_id:
        raise Exception('No testId provided')
    start_date = datetime.fromisoformat(start_date_str).astimezone(timezone.utc) if start_date_str else None
    end_date = datetime.fromisoformat(end_date_str).astimezone(timezone.utc) if end_date_str else None
    service = ResultsService()
    if request.method == 'HEAD':
        exists = service.is_results_exist(test_id=UUID(test_id))
        return respond(Response(status_code=200 if exists else 404, media_type='text/html'))
    graphs, ticks, releases_filters = service.get_test_graphs(test_id=UUID(test_id), start_date=start_date, end_date=end_date, table_names=table_names)
    graph_views = service.get_argus_graph_views(test_id=UUID(test_id))
    return respond({'status': 'ok', 'response': {'graphs': graphs, 'ticks': ticks, 'releases_filters': releases_filters, 'graph_views': graph_views}})

@bp.api_route('/create-graph-view', methods=['POST', 'OPTIONS'], name='api.create_graph_view', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def create_graph_view(request: Request):
    payload = get_payload(request)
    service = ResultsService()
    test_id = payload['testId']
    name = payload['name']
    description = payload['description']
    graph_view = service.create_argus_graph_view(test_id=UUID(test_id), name=name, description=description)
    return respond({'status': 'ok', 'response': graph_view})

@bp.api_route('/update-graph-view', methods=['POST', 'OPTIONS'], name='api.update_graph_view', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def update_graph_view(request: Request):
    payload = get_payload(request)
    service = ResultsService()
    test_id = payload['testId']
    id = payload['id']
    name = payload['name']
    description = payload['description']
    graphs = payload['graphs']
    graph_view = service.update_argus_graph_view(test_id=UUID(test_id), view_id=UUID(id), name=name, description=description, graphs=graphs)
    return respond({'status': 'ok', 'response': graph_view})

@bp.api_route('/test_run/comment/get', methods=['GET', 'HEAD', 'OPTIONS'], name='api.get_test_run_comment', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_test_run_comment(request: Request):
    comment_id = request.query_params.get('commentId')
    if not comment_id:
        raise Exception("commentId wasn't specified in the request")
    service = ArgusService()
    comment = service.get_comment(comment_id=UUID(comment_id))
    return respond(jsonify({'status': 'ok', 'response': comment if comment else False}))

@bp.api_route('/users', methods=['GET', 'HEAD', 'OPTIONS'], name='api.user_info', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def user_info(request: Request):
    result = UserService().get_users()
    return respond(jsonify({'status': 'ok', 'response': result}))

@bp.api_route('/release/stats/v2', methods=['GET', 'HEAD', 'OPTIONS'], name='api.release_stats_v2', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_stats_v2(request: Request):
    request.scope['query_string'].decode(encoding='UTF-8')
    release = request.query_params.get('release')
    limited = bool(int(request.query_params.get('limited')))
    version = request.query_params.get('productVersion', None)
    image_id = request.query_params.get('imageId', None)
    include_no_version = bool(int(request.query_params.get('includeNoVersion', True)))
    force = bool(int(request.query_params.get('force')))
    stats = ReleaseStatsCollector(release_name=release, release_version=version).collect(limited=limited, force=force, include_no_version=include_no_version, image_id=image_id, request=request)
    res = jsonify({'status': 'ok', 'response': stats})
    return respond(res)

@bp.api_route('/test_runs/poll', methods=['GET', 'HEAD', 'OPTIONS'], name='api.test_runs_poll', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def test_runs_poll(request: Request):
    raise APIException('This endpoint has been removed')

@bp.api_route('/test_run/poll', methods=['GET', 'HEAD', 'OPTIONS'], name='api.test_run_poll_single', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def test_run_poll_single(request: Request):
    raise APIException('This endpoint has been removed')

@bp.api_route('/release/create', methods=['POST', 'OPTIONS'], name='api.release_create', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def release_create(request: Request):
    require_json(request)
    request_payload = json_body(request)
    service = ArgusService()
    result = service.create_release(request_payload, request=request)
    return respond(jsonify({'status': 'ok', 'response': result}))

@bp.api_route('/artifact/resolveSize', methods=['GET', 'HEAD', 'OPTIONS'], name='api.resolve_artifact_size', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def resolve_artifact_size(request: Request):
    link = request.query_params.get('l')
    if not link:
        raise Exception('No link provided')
    length = TestRunService().resolve_artifact_size(link, request=request)
    return respond({'status': 'ok', 'response': {'artifactSize': length}})

@bp.api_route('/s3/{bucket_name}/{bucket_path:path}', methods=['GET', 'HEAD', 'OPTIONS'], name='api.s3_generic_proxy', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def s3_generic_proxy(request: Request, bucket_name: str, bucket_path: str):
    service = TestRunService()
    result = service.proxy_s3_file(bucket_name=bucket_name, bucket_path=bucket_path)
    return respond(redirect(result, code=302))

@bp.api_route('/user/token', methods=['GET', 'HEAD', 'OPTIONS'], name='api.user_token', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def user_token(request: Request):
    token = UserService().get_or_generate_token(user=request.state.user)
    return respond({'status': 'ok', 'response': {'token': token}})

@bp.api_route('/user/jobs', methods=['GET', 'HEAD', 'OPTIONS'], name='api.user_jobs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def user_jobs(request: Request):
    service = ArgusService()
    result = list(service.get_jobs_for_user(user=request.state.user))
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/user/planned_jobs', methods=['GET', 'HEAD', 'OPTIONS'], name='api.user_planned_jobs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def user_planned_jobs(request: Request):
    service = ArgusService()
    result = list(service.get_planned_jobs_for_user(user=request.state.user))
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/zeus/{endpoint:path}', methods=['GET', 'POST', 'HEAD', 'PUT', 'DELETE', 'OPTIONS'], name='api.zeus_proxy', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def zeus_proxy(request: Request, endpoint: str):
    zeus_host = Config.load_yaml_config().get('ZEUS_HOST')
    zeus_schema = Config.load_yaml_config().get('ZEUS_SCHEMA', 'http')
    zeus_token = Config.load_yaml_config().get('ZEUS_TOKEN')
    if not zeus_host:
        raise Exception('ZEUS_HOST is not configured, proxying is impossible.')
    if not zeus_token:
        raise Exception('Missing authorization token for Zeus [ZEUS_TOKEN]')
    query_str = request.scope['query_string'].decode('utf-8')
    method = request.method.lower()
    headers = dict(request.headers)
    headers['X-Forwarded-For'] = request.client.host if request.client else None
    headers['X-Argus-Proxy'] = '1'
    headers['Authorization'] = f'Bearer {zeus_token}'
    body = request.state.body
    zeus_host = Config.load_yaml_config().get('ZEUS_HOST')
    zeus_schema = Config.load_yaml_config().get('ZEUS_SCHEMA')
    session = requests.Session()
    proxy_request = requests.Request(method=method, url=f'{zeus_schema}://{zeus_host}/{endpoint}?{query_str}', headers=headers, data=body)
    prepared = proxy_request.prepare()
    response = session.send(prepared)
    return respond((response.content, response.status_code, response.headers.items()))
