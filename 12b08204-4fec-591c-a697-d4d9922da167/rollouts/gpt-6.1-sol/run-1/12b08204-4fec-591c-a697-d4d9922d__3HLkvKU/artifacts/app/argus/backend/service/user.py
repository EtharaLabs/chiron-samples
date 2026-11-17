from fastapi import Request
from argus.backend.util.config import Config
from datetime import UTC, datetime
import functools
import hashlib
import mimetypes
import os
import base64
import logging
import re
from uuid import UUID
from time import time
from hashlib import sha384
from coodie.exceptions import DocumentNotFound
import magic
import requests
import jwt
from werkzeug.security import generate_password_hash, check_password_hash
from argus.backend.db import ScyllaCluster
from argus.backend.error_handlers import APIException
from argus.backend.models.web import User, UserOauthToken, UserRoles, WebFileStorage
from argus.backend.util.common import HTTPView, gen_pass
LOGGER = logging.getLogger(__name__)
SSH_TUNNEL_SERVER_ALLOWED_ENDPOINTS_KEY = 'ssh_tunnel_server_allowed_endpoints'

class UserServiceException(Exception):
    pass

class GithubOrganizationMissingError(Exception):
    pass

class UserService:
    EMAIL_RE = re.compile("^[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$", re.IGNORECASE)

    def __init__(self) -> None:
        self.cluster = ScyllaCluster.get()
        self.session = self.cluster.session

    @staticmethod
    def check_roles(roles: list[UserRoles] | UserRoles, user: User) -> bool:
        if not user:
            return False
        if isinstance(roles, str):
            return roles in user.roles
        elif isinstance(roles, list):
            for role in roles:
                if role in user.roles:
                    return True
        return False

    def github_callback(self, req_code: str, *, request: Request=None) -> dict | None:
        if 'gh' not in Config.load_yaml_config().get('LOGIN_METHODS', []):
            raise UserServiceException('Github Login is disabled')
        oauth_response = requests.post('https://github.com/login/oauth/access_token', headers={'Accept': 'application/json'}, params={'code': req_code, 'client_id': Config.load_yaml_config().get('GITHUB_CLIENT_ID'), 'client_secret': Config.load_yaml_config().get('GITHUB_CLIENT_SECRET')})
        oauth_data = oauth_response.json()
        user_info = requests.get('https://api.github.com/user', headers={'Accept': 'application/json', 'Authorization': f"token {oauth_data.get('access_token')}"}).json()
        email_info = requests.get('https://api.github.com/user/emails', headers={'Accept': 'application/json', 'Authorization': f"token {oauth_data.get('access_token')}"}).json()
        organizations = requests.get('https://api.github.com/user/orgs', headers={'Accept': 'application/json', 'Authorization': f"token {oauth_data.get('access_token')}"}).json()
        temp_password = None
        required_organizations = Config.load_yaml_config().get('GITHUB_REQUIRED_ORGANIZATIONS')
        if required_organizations:
            logins = set([org['login'] for org in organizations])
            required_organizations = set(required_organizations)
            if len(logins.intersection(required_organizations)) == 0:
                raise GithubOrganizationMissingError('Not a member of a required organization or missing organization scope')
        try:
            user = User.get(username=user_info.get('login'))
        except DocumentNotFound:
            user = User()
            user.username = user_info.get('login')
            scylla_email = next(iter([email.get('email') for email in email_info if email.get('email').endswith('@scylladb.com')]), None)
            primary_email = next(iter([email.get('email') for email in email_info if email.get('primary') and email.get('verified')]), None)
            user.email = scylla_email or primary_email
            user.full_name = user_info.get('name', user_info.get('login'))
            user.registration_date = datetime.utcnow()
            user.roles = ['ROLE_USER']
            temp_password = gen_pass()
            user.password = generate_password_hash(temp_password)
            avatar_url: str = user_info.get('avatar_url')
            avatar = requests.get(avatar_url).content
            avatar_name = avatar_url.split('/')[-1]
            filename, filepath = self.save_profile_picture_to_disk(avatar_name, avatar, user.username)
            web_file = WebFileStorage()
            web_file.filename = filename
            web_file.filepath = filepath
            web_file.save()
            user.picture_id = web_file.id
            user.save()
        try:
            tokens = UserOauthToken.find(user_id=user.id).all()
            github_token = [token for token in tokens if token.kind == 'github'][0]
            github_token.token = oauth_data.get('access_token')
            github_token.save()
        except IndexError:
            github_token = UserOauthToken()
            github_token.kind = 'github'
            github_token.user_id = user.id
            github_token.token = oauth_data.get('access_token')
            github_token.save()
        redirect_target = request.session.get('redirect_target')
        request.session.clear()
        request.session['user_id'] = str(user.id)
        request.session['redirect_target'] = redirect_target
        if temp_password:
            return {'password': temp_password, 'first_login': True}
        return None

    def cf_login_or_register(self, *, request: Request=None):
        cf_access_jwt = request.headers.get('Cf-Access-Jwt-Assertion')
        if cf_access_jwt and 'cf' in Config.load_yaml_config().get('LOGIN_METHODS', []):
            try:
                res = _get_user_from_cf_access(cf_access_jwt)
            except UserServiceException as exc:
                request.session['manual_logout'] = True
                raise exc
            if res['exists']:
                request.session['user_id'] = str(res['user'].id)
                request.session['auth_via_cf'] = True
                return {'redirect_to': 'main.profile', 'redirect_optional': True, 'found': True}
            else:
                email = res['payload'].get('email')
                suggested_username: str = email.split('@')[0]
                suggested_name = ' '.join((part.capitalize() for part in suggested_username.split('.')))
                request.session['registration_allowed'] = True
                request.session['lock_user_email'] = True
                request.session['auth_via_cf'] = True
                request.session['oauth_email'] = email
                request.session['oauth_username'] = suggested_username
                request.session['oauth_name'] = suggested_name
                return {'redirect_to': 'main.profile_user_create', 'redirect_optional': False, 'found': False}
        request.session['manual_logout'] = True
        return {'redirect_to': 'auth.login', 'redirect_optional': False, 'found': False}

    def get_users(self) -> dict:
        users = User.find().all()
        return {str(user.id): user.to_json() for user in users}

    def get_users_privileged(self, service_only: bool=False) -> dict:
        users: list[User] = User.find().all()
        if service_only:
            users = [u for u in users if u.is_service_user()]
        users = {str(user.id): user.model_dump() for user in sorted(users, key=lambda u: u.username)}
        for user in users.values():
            user.pop('password')
            user.pop('api_token')
        return users

    def set_user_impersonation(self, user_id: str, *, request: Request=None):
        if request.session.get('original_user'):
            raise UserServiceException('Cannot impersonate while already impersonating a user.')
        user = User.get(id=UUID(user_id))
        request.session['original_user'] = str(request.state.user.id)
        request.state.user = user
        request.session['user_id'] = str(user.id)

    def stop_user_impersonation(self, *, request: Request=None):
        if not request.session.get('original_user'):
            raise UserServiceException('No impersonation in progress.')
        user_id = request.session.pop('original_user')
        request.state.user = User.get(id=UUID(user_id))
        request.session['user_id'] = str(request.state.user.id)

    def generate_token(self, user: User):
        token_digest = f"{user.username}-{int(time())}-{base64.encodebytes(os.urandom(128)).decode(encoding='utf-8')}"
        new_token = base64.encodebytes(sha384(token_digest.encode(encoding='utf-8')).digest()).decode(encoding='utf-8').strip()
        user.api_token = new_token
        user.save()
        return new_token

    def get_or_generate_token(self, user: User) -> str:
        if user.api_token:
            return user.api_token
        return self.generate_token(user)

    def update_email(self, user: User, new_email: str, *, request: Request=None):
        if (existing := User.exists_by_email(new_email)) and existing.id != user.id:
            raise UserServiceException('This email is already taken.')
        if not self.EMAIL_RE.match(new_email):
            raise UserServiceException('Invalid email.')
        user.email = new_email
        user.save()
        return True

    def toggle_admin(self, user_id: str, *, request: Request=None):
        user: User = User.get(id=UUID(user_id))
        if user.id == request.state.user.id:
            raise UserServiceException('Cannot toggle admin role from yourself.')
        is_admin = UserService.check_roles(UserRoles.Admin, user)
        if is_admin:
            user.roles.remove(UserRoles.Admin)
        else:
            user.set_as_admin()
        user.save()
        return True

    def create_user(self, username: str, email: str, full_name: str, *, request: Request=None) -> dict:
        result = {'created': False, 'user': None, 'temp_password': None, 'first_run': False, 'form_feedback': {'username': ['is-valid', ''], 'email': ['is-valid', ''], 'full_name': ['is-valid', '']}}
        errors = False
        if User.exists_by_name(username):
            result['form_feedback']['username'] = ['is-invalid', 'This username is already taken.']
            errors = True
        if '@' in username:
            result['form_feedback']['username'] = ['is-invalid', "Cannot use '@' in the username."]
            errors = True
        if User.exists_by_email(email):
            result['form_feedback']['email'] = ['is-invalid', 'This email is already taken.']
            errors = True
        if not self.EMAIL_RE.match(email):
            result['form_feedback']['email'] = ['is-invalid', 'Invalid email.']
            errors = True
        if len(full_name) == 0:
            result['form_feedback']['full_name'] = ['is-invalid', 'No display name provided.']
            errors = True
        if errors:
            return result
        user = User()
        user.username = username
        user.email = email
        user.full_name = full_name
        user.registration_date = datetime.now(tz=UTC)
        user.roles = ['ROLE_USER']
        if (avatar := request.state.form.get('avatar')):
            content = avatar.file.read()
            avatar_mime = magic.from_buffer(content, mime=True)
            if not re.match('^image/.*', avatar_mime, re.IGNORECASE):
                raise UserServiceException(f'Expected image/*, got {avatar_mime} for user avatar.')
            avatar_ext = mimetypes.guess_extension(avatar_mime)
            filename = f'{username}_{datetime.now(tz=UTC).timestamp()}{avatar_ext}'
            filename, filepath = self.save_profile_picture_to_disk(filename, content, user.username)
            web_file = WebFileStorage()
            web_file.filename = filename
            web_file.filepath = filepath
            web_file.save()
            user.picture_id = web_file.id
        temp_password = gen_pass()
        user.password = generate_password_hash(temp_password)
        user.save()
        result['user'] = user
        result['created'] = True
        result['temp_password'] = temp_password
        result['first_run'] = True
        return result

    def delete_user(self, user_id: str, *, request: Request=None):
        user: User = User.get(id=UUID(user_id))
        if user.id == request.state.user.id:
            raise UserServiceException('Cannot delete user that you are logged in as.')
        if user.is_admin():
            raise UserServiceException('Cannot delete admin users. Unset admin flag before deleting')
        user.delete()
        return True

    def update_password(self, user: User, old_password: str, new_password: str, force=False, *, request: Request=None):
        if not check_password_hash(user.password, old_password) and (not force):
            raise UserServiceException('Incorrect old password')
        if not new_password:
            raise UserServiceException('Empty new password')
        if len(new_password) < 5:
            raise UserServiceException('New password is too short')
        user.password = generate_password_hash(new_password)
        user.save()
        return True

    def change_username(self, user: User, new_username: str):
        if (existing := User.exists_by_name(new_username)) and existing.id != user.id:
            raise UserServiceException('This username is already taken.')
        if '@' in new_username:
            raise UserServiceException("Cannot use '@' in the username")
        user.username = new_username
        user.save()

    def update_name(self, user: User, new_name: str):
        user.full_name = new_name
        user.save()

    def save_profile_picture_to_disk(self, original_filename: str, filedata: bytes, suffix: str):
        filename_fragment = hashlib.sha256(os.urandom(64)).hexdigest()[:10]
        filename = f'profile_{suffix}_{filename_fragment}'
        filepath = f'storage/profile_pictures/{filename}'
        with open(filepath, 'wb') as file:
            file.write(filedata)
        return (original_filename, filepath)

    def update_profile_picture(self, filename: str, filepath: str, *, request: Request=None):
        web_file = WebFileStorage()
        web_file.filename = filename
        web_file.filepath = filepath
        web_file.save()
        try:
            if (old_picture_id := request.state.user.picture_id):
                old_file = WebFileStorage.get(id=old_picture_id)
                os.unlink(old_file.filepath)
                old_file.delete()
        except Exception as exc:
            print(exc)
        request.state.user.picture_id = web_file.id
        request.state.user.save()

def allow_ssh_tunnel_server_scope(view: HTTPView):
    setattr(view, 'allow_ssh_tunnel_server_scope', True)
    return view

def load_logged_in_user(*, request: Request=None):
    auth_header = request.headers.get('Authorization')
    if auth_header:
        try:
            auth_schema, *auth_data = auth_header.split()
            if auth_schema == 'token':
                token = auth_data[0]
                request.state.user = User.get(api_token=token)
                return
        except IndexError as exception:
            raise APIException('Malformed authorization header') from exception
        except DocumentNotFound as exception:
            raise APIException('User not found for supplied token') from exception
    if (user_id := request.session.get('user_id')):
        try:
            request.state.user = User.get(id=UUID(user_id))
            return
        except DocumentNotFound:
            request.session.clear()
    request.state.user = None

def is_ssh_tunnel_server_user(user: User | None) -> bool:
    if not user:
        return False
    return UserService.check_roles(UserRoles.SSHTunnelServer, user)

def is_ssh_tunnel_server_request_allowed(*, request: Request=None) -> bool:
    endpoint = request.scope.get('route').name
    if not endpoint:
        return False
    allowed_endpoint_methods = request.app.state.extensions.get(SSH_TUNNEL_SERVER_ALLOWED_ENDPOINTS_KEY, set())
    return (endpoint, request.method) in allowed_endpoint_methods

def is_scoped_ssh_tunnel_server_blocked(user: User | None, *, request: Request=None) -> bool:
    return is_ssh_tunnel_server_user(user) and (not is_ssh_tunnel_server_request_allowed(request=request))

def _get_cf_access_payload(token: str) -> dict | None:
    cf_domain = Config.load_yaml_config().get('CLOUDFLARE_ACCESS_TEAM_DOMAIN')
    cf_aud = Config.load_yaml_config().get('CLOUDFLARE_ACCESS_AUD')
    if not cf_domain or not cf_aud:
        LOGGER.warning('Cloudflare Access config missing: domain or audience')
        raise UserServiceException('Cloudflare Access config missing: domain or audience')
    jwk_client = Config.load_yaml_config().get('CLOUDFLARE_ACCESS_JWK_CLIENT')
    if not jwk_client:
        LOGGER.warning('Cloudflare Access JWK client is not configured')
        raise UserServiceException('Cloudflare Access JWK client is not configured')
    issuer = f'https://{cf_domain}'
    try:
        signing_key = jwk_client.get_signing_key_from_jwt(token)
        payload = jwt.decode(token, signing_key.key, algorithms=['RS256'], audience=cf_aud, issuer=issuer)
    except jwt.PyJWTError:
        LOGGER.warning('Invalid Cloudflare Access JWT', exc_info=True)
        raise UserServiceException('Invalid Cloudflare Access JWT')
    if not isinstance(payload, dict):
        raise UserServiceException('Invalid Cloudflare Access Payload')
    return payload

def _get_user_from_cf_access(token: str) -> dict:
    payload = _get_cf_access_payload(token)
    if not payload:
        raise UserServiceException('Invalid Cloudflare Access Payload')
    email = payload.get('email')
    if not email:
        raise UserServiceException('No email in the payload.')
    if not email.lower().endswith('@scylladb.com'):
        raise UserServiceException('Email is external to scylladb.com')
    try:
        user = User.get(email=email)
        return {'user': user, 'payload': payload, 'exists': True}
    except DocumentNotFound:
        return {'user': None, 'payload': payload, 'exists': False}
