from argus.backend.http import form_field
from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, redirect, flash, render_template, url_for, authorize
from argus.backend.util.config import Config
import os
import hashlib
from werkzeug.security import check_password_hash
from coodie.exceptions import DocumentNotFound
from argus.backend.error_handlers import handle_profile_exception
from argus.backend.models.web import User, UserRoles
from argus.backend.service.user import UserService, UserServiceException, load_logged_in_user
bp = APIRouter(prefix='/auth')

@bp.api_route('/register', methods=['GET', 'POST', 'HEAD', 'OPTIONS'], name='auth.register', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def register(request: Request):
    return respond(redirect(url_for(request, 'auth.login')))

@bp.api_route('/login', methods=['GET', 'POST', 'HEAD', 'OPTIONS'], name='auth.login', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def login(request: Request):
    if request.state.user:
        if (redirect_target := request.session.pop('redirect_target', None)):
            return respond(redirect(redirect_target))
        return respond(redirect(url_for(request, 'main.home')))
    token = hashlib.sha256(os.urandom(64)).hexdigest()
    request.session['csrf_token'] = token
    if request.method == 'POST':
        try:
            if 'password' not in Config.load_yaml_config().get('LOGIN_METHODS', []):
                raise UserServiceException('Password Login is disabled')
            username = form_field(request, 'username')
            password = form_field(request, 'password')
            try:
                user: User = User.get(username=username)
            except DocumentNotFound:
                raise UserServiceException('User not found')
            if not check_password_hash(user.password, password):
                raise UserServiceException('Incorrect Password')
            request.session.clear()
            request.session['user_id'] = str(user.id)
            request.session['csrf_token'] = token
        except UserServiceException as exc:
            flash(request, next(iter(exc.args), 'No message'), category='error')
        return respond(redirect(url_for(request, 'main.home')))
    return respond(render_template(request, 'auth/login.html.j2', csrf_token=token, github_cid=Config.load_yaml_config().get('GITHUB_CLIENT_ID', 'NO_CLIENT_ID'), github_scopes=Config.load_yaml_config().get('GITHUB_SCOPES', 'user:email read:user read:org')))

@bp.api_route('/login/cf', methods=['POST', 'OPTIONS'], name='auth.cf_login', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def cf_login(request: Request):
    res = UserService().cf_login_or_register(request=request)
    if not res['redirect_optional']:
        return respond(redirect(url_for(request, res['redirect_to'])))
    if (redirect_target := request.session.pop('redirect_target', None)):
        return respond(redirect(redirect_target))
    return respond(redirect(url_for(request, res['redirect_to'])))

@bp.api_route('/profile/api/token/generate', methods=['POST', 'OPTIONS'], name='auth.generate_api_token', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def generate_api_token(request: Request):
    new_token = UserService().generate_token(request.state.user)
    request.session['token_generated'] = new_token
    return respond(redirect(url_for(request, 'main.profile')))

@bp.api_route('/admin/impersonate', methods=['POST', 'GET', 'HEAD', 'OPTIONS'], name='auth.switch_user', response_model=None, dependencies=[Depends(authorize(True, False, UserRoles.Admin, False))])
def switch_user(request: Request):
    if request.method == 'POST':
        try:
            user_id = form_field(request, 'user_id')
        except KeyError:
            flash(request, 'No user id', category='error')
            return respond(redirect(url_for(request, 'main.profile')))
        UserService().set_user_impersonation(user_id, request=request)
        return respond(redirect(url_for(request, 'main.profile')))
    users = UserService().get_users_privileged(service_only=True)
    return respond(render_template(request, 'auth/user_switch.html.j2', users=users))

@bp.api_route('/admin/impersonate/stop', methods=['POST', 'OPTIONS'], name='auth.stop_impersonation', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def stop_impersonation(request: Request):
    UserService().stop_user_impersonation(request=request)
    return respond(redirect(url_for(request, 'main.profile')))

@bp.api_route('/logout', methods=['POST', 'OPTIONS'], name='auth.logout', response_model=None, dependencies=[Depends(authorize(False, False, None, False))])
def logout(request: Request):
    request.session.clear()
    request.session['manual_logout'] = True
    return respond(redirect(url_for(request, 'auth.login')))
