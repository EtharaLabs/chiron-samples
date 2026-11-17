from argus.backend.http import form_field
from fastapi import APIRouter, Request, Depends
from starlette.responses import Response
from argus.backend.http import respond, redirect, flash, render_template, url_for, authorize
import datetime
import json
import logging
from uuid import UUID
from argus.backend.controller.notifications import bp as notifications_bp
from argus.backend.controller.team_ui import bp as teams_bp
from argus.backend.error_handlers import handle_profile_exception, handle_view_not_found
from argus.backend.service.argus_service import ArgusService
from argus.backend.models.web import ArgusRelease, WebFileStorage
from argus.backend.service.testrun import TestRunService
from argus.backend.service.planner_service import PlanningService
from argus.backend.service.user import UserService, UserServiceException
from argus.backend.service.views import UserViewException, UserViewService
LOGGER = logging.getLogger(__name__)
bp = APIRouter(prefix='')
bp.include_router(notifications_bp)
bp.include_router(teams_bp)

@bp.api_route('/test_runs', methods=['GET', 'HEAD', 'OPTIONS'], name='main.test_runs', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def test_runs(request: Request):
    return respond(render_template(request, 'test_runs.html.j2'))

@bp.api_route('/test_run/{run_id}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.test_run', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def test_run(request: Request, run_id: str):
    return respond(render_template(request, 'test_run.html.j2', id=run_id))

@bp.api_route('/test/{test_id}/runs', methods=['GET', 'HEAD', 'OPTIONS'], name='main.runs', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def runs(request: Request, test_id: str):
    additional_runs = request.query_params.getlist('additionalRuns[]')
    return respond(render_template(request, 'standalone_test_with_runs.html.j2', test_id=test_id, additional_runs=additional_runs))

@bp.api_route('/tests/{plugin_name}/{run_id}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.get_run_by_plugin', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
@bp.api_route('/tests/{plugin_name}/{run_id}/{tab}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.get_run_by_plugin', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def get_run_by_plugin(request: Request, plugin_name: str, run_id: str, tab: str='details'):
    try:
        run_id = UUID(run_id)
    except ValueError:
        flash(request, message=f'Invalid UUID: {run_id}', category='error')
        return respond(redirect(url_for(request, 'main.error', type=404)))
    run = TestRunService().get_run(plugin_name, run_id)
    if not run:
        flash(request, f'Run {plugin_name}/{run_id} not found.', 'error')
        return respond(redirect(url_for(request, 'main.error', type=404)))
    return respond(render_template(request, 'run_view_by_plugin.html.j2', run=run, tab=tab))

@bp.api_route('/test/{build_id:path}/{build_number:int}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.get_run_by_build', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
@bp.api_route('/test/{build_id:path}/{build_number:int}/{tab}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.get_run_by_build', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def get_run_by_build(request: Request, build_id: str, build_number: int, tab: str='details'):
    run = TestRunService().get_run_by_build_number(build_id, build_number)
    if not run:
        flash(request, f'Run {build_id} #{build_number} not found.', 'error')
        return respond(redirect(url_for(request, 'main.error', type=404)))
    return respond(render_template(request, 'run_view_by_plugin.html.j2', run=run, tab=tab))

@bp.api_route('/', methods=['GET', 'HEAD', 'OPTIONS'], name='main.home', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def home(request: Request):
    return respond(redirect(url_for(request, 'main.run_dashboard')))

@bp.api_route('/run_dashboard', methods=['GET', 'HEAD', 'OPTIONS'], name='main.run_dashboard', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
@bp.api_route('/workspace', methods=['GET', 'HEAD', 'OPTIONS'], name='main.run_dashboard', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def run_dashboard(request: Request):
    return respond(render_template(request, 'dashboard.html.j2'))

@bp.api_route('/releases', methods=['GET', 'HEAD', 'OPTIONS'], name='main.releases', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def releases(request: Request):
    service = ArgusService()
    all_releases = service.get_releases()
    return respond(render_template(request, 'releases.html.j2', releases=all_releases))

@bp.api_route('/views', methods=['GET', 'HEAD', 'OPTIONS'], name='main.views', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def views(request: Request):
    service = UserViewService()
    all_views = service.get_all_views(request=request)
    return respond(render_template(request, 'views.html.j2', views=sorted(all_views, key=lambda view: view.created or datetime.datetime.fromtimestamp(0), reverse=True)))

@bp.api_route('/view/{view_name}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.view_dashboard', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def view_dashboard(request: Request, view_name: str):
    service = UserViewService()
    view = service.get_view_by_name(view_name=view_name)
    data_json = view
    view.widget_settings = json.loads(view.widget_settings)
    return respond(render_template(request, 'view_dashboard.html.j2', data=data_json))

@bp.api_route('/plan/{plan_id}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.plan_dashboard', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def plan_dashboard(request: Request, plan_id: str):
    service = PlanningService()
    plan = service.get_plan(plan_id=plan_id)
    data_json = plan
    return respond(render_template(request, 'plan_dashboard.html.j2', data=data_json))

@bp.api_route('/alert_debug', methods=['GET', 'HEAD', 'OPTIONS'], name='main.alert_debug', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def alert_debug(request: Request):
    alert_type = request.query_params.get('type', 'success')
    message = request.query_params.get('message', 'No message provided')
    flash(request, message=message, category=alert_type)
    return respond(render_template(request, 'flash_debug.html.j2'))

@bp.api_route('/dashboard/{release_name:path}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.release_dashboard', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def release_dashboard(request: Request, release_name: str):
    service = ArgusService()
    release, release_groups, release_tests = service.get_data_for_release_dashboard(release_name=release_name)
    data_json = {'release': release.model_dump(), 'groups': [group.model_dump() for group in release_groups], 'tests': [test.model_dump() for test in release_tests]}
    return respond(render_template(request, 'release_dashboard.html.j2', release_name=release_name, data=data_json))

@bp.api_route('/release/{name}/scheduler', methods=['GET', 'HEAD', 'OPTIONS'], name='main.release_scheduler', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def release_scheduler(request: Request, name: str):
    service = ArgusService()
    release, release_groups, release_tests = service.get_data_for_release_dashboard(release_name=name)
    data_json = {'release': release.model_dump(), 'groups': [group.model_dump() for group in release_groups], 'tests': [test.model_dump() for test in release_tests]}
    return respond(render_template(request, 'release_schedule.html.j2', release_name=name, data=data_json))

@bp.api_route('/release/by-id/{id}/planner', methods=['GET', 'HEAD', 'OPTIONS'], name='main.release_planner_by_id', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def release_planner_by_id(request: Request, id: str):
    release = ArgusRelease.get(id=UUID(id))
    return respond(redirect(url_for(request, 'main.release_planner', name=release.name)))

@bp.api_route('/release/{name}/planner', methods=['GET', 'HEAD', 'OPTIONS'], name='main.release_planner', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def release_planner(request: Request, name: str):
    service = PlanningService()
    planner_data = service.release_planner(name, request=request)
    return respond(render_template(request, 'release_planner.html.j2', release_name=planner_data['release'].name, planner_data=planner_data))

@bp.api_route('/release/{name}/duty', methods=['GET', 'HEAD', 'OPTIONS'], name='main.duty_planner', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def duty_planner(request: Request, name: str):
    service = ArgusService()
    release, release_groups, release_tests = service.get_data_for_release_dashboard(release_name=name)
    data_json = {'release': release.model_dump(), 'groups': [group.model_dump() for group in release_groups], 'tests': [test.model_dump() for test in release_tests]}
    return respond(render_template(request, 'duty_planner.html.j2', release_name=name, data=data_json))

@bp.api_route('/error/', methods=['GET', 'HEAD', 'OPTIONS'], name='main.error', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def error(request: Request):
    return respond(render_template(request, 'error.html.j2', type=request.query_params.get('type', 400)))

@bp.api_route('/profile/', methods=['GET', 'HEAD', 'OPTIONS'], name='main.profile', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def profile(request: Request):
    first_run = request.session.pop('first_run_info', None)
    token_generated = request.session.pop('token_generated', None)
    return respond(render_template(request, 'profile.html.j2', first_run=first_run, token_generated=token_generated))

@bp.api_route('/profile/create', methods=['GET', 'POST', 'HEAD', 'OPTIONS'], name='main.profile_user_create', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def profile_user_create(request: Request):
    if not request.session.get('registration_allowed', False):
        raise UserServiceException('Registration is not allowed at the moment.')
    result = {}
    if request.method == 'POST':
        if request.session.get('lock_user_email') and request.state.form.get('email') != request.session.get('oauth_email'):
            raise UserServiceException('Email changed while being locked to oauth one.')
        result = UserService().create_user(username=form_field(request, 'username'), email=form_field(request, 'email'), full_name=form_field(request, 'full_name'), request=request)
        if result['created']:
            request.session.clear()
            request.session['user_id'] = str(result['user'].id)
            request.session['first_run_info'] = {'password': result['temp_password'], 'first_login': True}
            return respond(redirect(url_for(request, 'main.profile')))
    return respond(render_template(request, 'create_user.html.j2', feedback=result.get('form_feedback', {})))

@bp.api_route('/profile/oauth/github', methods=['GET', 'HEAD', 'OPTIONS'], name='main.profile_oauth_github_callback', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def profile_oauth_github_callback(request: Request):
    req_state = request.query_params.get('state', '')
    if req_state != request.session['csrf_token']:
        return respond(redirect(url_for(request, 'main.error', type=403)))
    req_code = request.query_params.get('code', 'WTF')
    service = UserService()
    try:
        first_run_info = service.github_callback(req_code, request=request)
    except Exception as exc:
        LOGGER.error('An error occured in callback', exc_info=True)
        flash(request, message=exc.args[0], category='error')
        return respond(redirect(url_for(request, 'main.error', type=403)))
    if first_run_info:
        request.session['first_run_info'] = first_run_info
    if (path := request.session.pop('redirect_target')):
        return respond(redirect(path))
    return respond(redirect(url_for(request, 'main.profile')))

@bp.api_route('/storage/picture/{picture_id}', methods=['GET', 'HEAD', 'OPTIONS'], name='main.get_picture', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def get_picture(request: Request, picture_id: str):
    try:
        picture = WebFileStorage.get(id=UUID(picture_id))
        with open(picture.filepath, 'rb') as file:
            return Response(file.read(), media_type='image/*')
    except FileNotFoundError:
        return Response('404 NOT FOUND', status_code=404, media_type='text/plain')

@bp.api_route('/profile/update/picture', methods=['POST', 'OPTIONS'], name='main.upload_file', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def upload_file(request: Request):
    req_file = request.state.form.get('filedata')
    picture_data = req_file.file.read()
    picture_name = req_file.filename
    if not req_file.content_type.startswith('image/'):
        flash(request, message=f'Expected image/*, got {req_file.content_type}', category='error')
        return respond(redirect(url_for(request, 'main.profile')))
    if not picture_data:
        flash(request, message='No picture provided', category='error')
        return respond(redirect(url_for(request, 'main.profile')))
    service = UserService()
    filename, filepath = service.save_profile_picture_to_disk(picture_name, picture_data, request.state.user.username)
    service.update_profile_picture(filename, filepath, request=request)
    return respond(redirect(url_for(request, 'main.profile')))

@bp.api_route('/profile/update/name', methods=['POST', 'OPTIONS'], name='main.update_full_name', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def update_full_name(request: Request):
    new_name = request.state.form.get('new_name')
    if not new_name:
        flash(request, message='Incorrect new name', category='error')
    else:
        service = UserService()
        service.update_name(request.state.user, new_name)
        flash(request, 'Successfully changed name!', category='success')
    return respond(redirect(url_for(request, 'main.profile')))

@bp.api_route('/profile/update/username', methods=['POST', 'OPTIONS'], name='main.update_user_name', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def update_user_name(request: Request):
    new_username = request.state.form.get('new_username')
    if not new_username:
        flash(request, message='Missing username in payload', category='error')
    else:
        service = UserService()
        service.change_username(request.state.user, new_username)
        flash(request, 'Successfully changed username!', category='success')
    return respond(redirect(url_for(request, 'main.profile')))

@bp.api_route('/profile/update/email', methods=['POST', 'OPTIONS'], name='main.update_email', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def update_email(request: Request):
    new_email = request.state.form.get('new_email')
    if not request.session.get('original_user') and (not request.state.user.is_admin()):
        flash(request, 'Not authorized to change email.')
        return respond(redirect(url_for(request, 'main.profile')))
    if not new_email:
        flash(request, 'Incorrect new email', category='error')
    else:
        service = UserService()
        service.update_email(request.state.user, new_email, request=request)
        flash(request, 'Successfully changed email!', category='success')
    return respond(redirect(url_for(request, 'main.profile')))

@bp.api_route('/profile/update/password', methods=['POST', 'OPTIONS'], name='main.update_password', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def update_password(request: Request):
    old_password = request.state.form.get('old_password')
    new_password = request.state.form.get('new_password')
    new_password_confirm = request.state.form.get('new_password_confirm')
    if not old_password:
        flash(request, "Old password wasn't provided", category='error')
        return respond(redirect(url_for(request, 'main.profile')))
    if not new_password:
        flash(request, "New password wasn't provided", category='error')
        return respond(redirect(url_for(request, 'main.profile')))
    if not new_password == new_password_confirm:
        flash(request, "New password doesn't match confirmation!", category='error')
        return respond(redirect(url_for(request, 'main.profile')))
    service = UserService()
    try:
        service.update_password(request.state.user, old_password=old_password, new_password=new_password, request=request)
    except Exception:
        flash(request, 'Old password is incorrect', category='error')
        return respond(redirect(url_for(request, 'main.profile')))
    flash(request, 'Successfully changed password!')
    return respond(redirect(url_for(request, 'main.profile')))

@bp.api_route('/profile/jobs', methods=['GET', 'HEAD', 'OPTIONS'], name='main.profile_jobs', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def profile_jobs(request: Request):
    return respond(render_template(request, 'profile_jobs.html.j2'))

@bp.api_route('/profile/schedules', methods=['GET', 'HEAD', 'OPTIONS'], name='main.profile_schedules', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def profile_schedules(request: Request):
    service = ArgusService()
    schedules = service.get_schedules_for_user(request.state.user)
    return respond(render_template(request, 'profile_schedules.html.j2', schedules=schedules))
