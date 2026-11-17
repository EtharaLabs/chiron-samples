from argus.backend.http import json_body
from argus.backend.http import path_uuid
from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, require_json, authorize
from dataclasses import asdict
import logging
from typing import TypedDict
from uuid import UUID
from argus.backend.error_handlers import handle_api_exception
from argus.backend.service.release_manager import ReleaseEditPayload, ReleaseManagerService
from argus.backend.service.tunnel_service import ProxyTunnelConfigPayload, TunnelService
from argus.backend.service.user import UserService
from argus.backend.models.web import User, UserRoles
bp = APIRouter(prefix='/api/v1')
LOGGER = logging.getLogger(__name__)
ACTIVE_ONLY_MAP = {'true': True, 'false': False}

class ProxyTunnelActivePayload(TypedDict):
    is_active: bool

class ProxyTunnelConfigCreatePayload(ProxyTunnelConfigPayload):
    pass

def parse_active_only(value: str | None) -> bool | None:
    if value is None:
        return None
    lowered = value.strip().lower()
    if lowered not in ACTIVE_ONLY_MAP:
        raise Exception('active_only must be one of: true,false')
    return ACTIVE_ONLY_MAP[lowered]

def get_payload(client_request: Request):
    require_json(client_request)
    request_payload = json_body(client_request)
    return request_payload

@bp.api_route('/', methods=['GET', 'HEAD', 'OPTIONS'], name='admin_api.index', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def index(request: Request):
    return respond({'version': 'v1'})

@bp.api_route('/release/create', methods=['POST', 'OPTIONS'], name='admin_api.create_release', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def create_release(request: Request):
    payload = get_payload(request)
    release = ReleaseManagerService().create_release(**payload, request=request)
    return respond({'status': 'ok', 'response': {'new_release': release}})

@bp.api_route('/release/set_perpetual', methods=['POST', 'OPTIONS'], name='admin_api.set_release_perpetual', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def set_release_perpetual(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().set_release_perpetuality(**payload)
    return respond({'status': 'ok', 'response': {'updated': result}})

@bp.api_route('/release/set_state', methods=['POST', 'OPTIONS'], name='admin_api.set_release_state', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def set_release_state(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().set_release_state(**payload, request=request)
    return respond({'status': 'ok', 'response': {'updated': result}})

@bp.api_route('/release/set_dormant', methods=['POST', 'OPTIONS'], name='admin_api.set_release_dormancy', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def set_release_dormancy(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().set_release_dormancy(**payload, request=request)
    return respond({'status': 'ok', 'response': {'updated': result}})

@bp.api_route('/release/edit', methods=['POST', 'OPTIONS'], name='admin_api.edit_release', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def edit_release(request: Request):
    payload: ReleaseEditPayload = get_payload(request)
    result = ReleaseManagerService().edit_release(payload, request=request)
    return respond({'status': 'ok', 'response': {'updated': result}})

@bp.api_route('/release/delete', methods=['POST', 'OPTIONS'], name='admin_api.delete_release', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def delete_release(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().delete_release(release_id=payload['releaseId'], request=request)
    return respond({'status': 'ok', 'response': {'deleted': result}})

@bp.api_route('/group/create', methods=['POST', 'OPTIONS'], name='admin_api.create_group', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def create_group(request: Request):
    payload = get_payload(request)
    group = ReleaseManagerService().create_group(**payload, request=request)
    return respond({'status': 'ok', 'response': {'new_group': group}})

@bp.api_route('/group/update', methods=['POST', 'OPTIONS'], name='admin_api.update_group', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def update_group(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().update_group(**payload, request=request)
    return respond({'status': 'ok', 'response': {'updated': result}})

@bp.api_route('/group/delete', methods=['POST', 'OPTIONS'], name='admin_api.delete_group', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def delete_group(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().delete_group(**payload, request=request)
    return respond({'status': 'ok', 'response': {'deleted': result}})

@bp.api_route('/test/create', methods=['POST', 'OPTIONS'], name='admin_api.create_test', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def create_test(request: Request):
    payload = get_payload(request)
    test = ReleaseManagerService().create_test(**payload, request=request)
    return respond({'status': 'ok', 'response': {'new_test': test}})

@bp.api_route('/test/update', methods=['POST', 'OPTIONS'], name='admin_api.update_test', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def update_test(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().update_test(**payload, request=request)
    return respond({'status': 'ok', 'response': {'updated': result}})

@bp.api_route('/test/batch_move', methods=['POST', 'OPTIONS'], name='admin_api.batch_move_tests', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def batch_move_tests(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().batch_move_tests(**payload, request=request)
    return respond({'status': 'ok', 'response': {'moved': result}})

@bp.api_route('/test/delete', methods=['POST', 'OPTIONS'], name='admin_api.delete_test', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def delete_test(request: Request):
    payload = get_payload(request)
    result = ReleaseManagerService().delete_test(**payload, request=request)
    return respond({'status': 'ok', 'response': {'deleted': result}})

@bp.api_route('/releases/get', methods=['GET', 'HEAD', 'OPTIONS'], name='admin_api.get_releases', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def get_releases(request: Request):
    releases = ReleaseManagerService().get_releases()
    return respond({'status': 'ok', 'response': releases})

@bp.api_route('/groups/get', methods=['GET', 'HEAD', 'OPTIONS'], name='admin_api.get_groups_for_release', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def get_groups_for_release(request: Request):
    release_id = request.query_params.get('releaseId')
    if not release_id:
        raise Exception('ReleaseId not provided in the request')
    groups = ReleaseManagerService().get_groups(release_id=UUID(release_id))
    return respond({'status': 'ok', 'response': groups})

@bp.api_route('/tests/get', methods=['GET', 'HEAD', 'OPTIONS'], name='admin_api.get_tests_for_group', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def get_tests_for_group(request: Request):
    group_id = request.query_params.get('groupId')
    if not group_id:
        raise Exception('GroupId not provided in the request')
    tests = ReleaseManagerService().get_tests(group_id=UUID(group_id))
    return respond({'status': 'ok', 'response': tests})

@bp.api_route('/release/test/state/toggle', methods=['POST', 'OPTIONS'], name='admin_api.quick_toggle_test_enabled', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def quick_toggle_test_enabled(request: Request):
    payload = get_payload(request)
    res = ReleaseManagerService().toggle_test_enabled(test_id=payload['entityId'], new_state=payload['state'])
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/release/group/state/toggle', methods=['POST', 'OPTIONS'], name='admin_api.quick_toggle_group_enabled', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def quick_toggle_group_enabled(request: Request):
    payload = get_payload(request)
    res = ReleaseManagerService().toggle_group_enabled(group_id=payload['entityId'], new_state=payload['state'])
    return respond({'status': 'ok', 'response': res})

@bp.api_route('/users', methods=['GET', 'HEAD', 'OPTIONS'], name='admin_api.user_info', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def user_info(request: Request):
    result = UserService().get_users_privileged()
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/user/{user_id}/email/set', methods=['POST', 'OPTIONS'], name='admin_api.user_change_email', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def user_change_email(request: Request, user_id: str):
    payload = get_payload(request)
    user = User.get(id=path_uuid(user_id))
    result = UserService().update_email(user=user, new_email=payload['newEmail'], request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/user/{user_id}/delete', methods=['POST', 'OPTIONS'], name='admin_api.user_delete', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def user_delete(request: Request, user_id: str):
    result = UserService().delete_user(user_id=user_id, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/user/{user_id}/password/set', methods=['POST', 'OPTIONS'], name='admin_api.user_change_password', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def user_change_password(request: Request, user_id: str):
    payload = get_payload(request)
    user = User.get(id=path_uuid(user_id))
    result = UserService().update_password(user=user, old_password='', new_password=payload['newPassword'], force=True, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/user/{user_id}/admin/toggle', methods=['POST', 'OPTIONS'], name='admin_api.user_toggle_admin', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def user_toggle_admin(request: Request, user_id: str):
    result = UserService().toggle_admin(user_id=user_id, request=request)
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/proxy-tunnel/config', methods=['GET', 'HEAD', 'OPTIONS'], name='admin_api.get_proxy_tunnel_config', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def get_proxy_tunnel_config(request: Request):
    tunnel_id = request.query_params.get('tunnel_id')
    config = TunnelService().get_proxy_tunnel_config(tunnel_id=tunnel_id, request=request)
    return respond({'status': 'ok', 'response': asdict(config) if config else None})

@bp.api_route('/proxy-tunnel/configs', methods=['GET', 'HEAD', 'OPTIONS'], name='admin_api.list_proxy_tunnel_configs', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def list_proxy_tunnel_configs(request: Request):
    active_only = parse_active_only(request.query_params.get('active_only'))
    configs = TunnelService().list_proxy_tunnel_configs(active_only=active_only, request=request)
    return respond({'status': 'ok', 'response': [asdict(row) for row in configs]})

@bp.api_route('/proxy-tunnel/config', methods=['POST', 'OPTIONS'], name='admin_api.save_proxy_tunnel_config', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def save_proxy_tunnel_config(request: Request):
    payload: ProxyTunnelConfigCreatePayload = get_payload(request)
    config = TunnelService().save_proxy_tunnel_config(payload, request=request)
    return respond({'status': 'ok', 'response': asdict(config)})

@bp.api_route('/proxy-tunnel/config/{tunnel_id}', methods=['DELETE', 'OPTIONS'], name='admin_api.delete_proxy_tunnel_config', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def delete_proxy_tunnel_config(request: Request, tunnel_id: str):
    payload = json_body(request, silent=True) or {}
    delete_user = bool(payload.get('delete_user', False))
    TunnelService().delete_proxy_tunnel_config(path_uuid(tunnel_id), delete_user=delete_user, request=request)
    return respond({'status': 'ok', 'response': {'deleted': True, 'user_deleted': delete_user}})

@bp.api_route('/proxy-tunnel/config/{tunnel_id}/active', methods=['POST', 'OPTIONS'], name='admin_api.set_proxy_tunnel_config_active', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def set_proxy_tunnel_config_active(request: Request, tunnel_id: str):
    payload: ProxyTunnelActivePayload = get_payload(request)
    config = TunnelService().set_proxy_tunnel_config_active(path_uuid(tunnel_id), payload['is_active'], request=request)
    return respond({'status': 'ok', 'response': asdict(config)})

@bp.api_route('/ssh/keys', methods=['GET', 'HEAD', 'OPTIONS'], name='admin_api.list_ssh_keys', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def list_ssh_keys(request: Request):
    keys = TunnelService().list_keys()
    return respond({'status': 'ok', 'response': [asdict(row) for row in keys]})

@bp.api_route('/ssh/keys/{key_id}', methods=['DELETE', 'OPTIONS'], name='admin_api.delete_ssh_key', response_model=None, dependencies=[Depends(authorize(True, True, UserRoles.Admin, True))])
def delete_ssh_key(request: Request, key_id: str):
    TunnelService().delete_key(path_uuid(key_id))
    return respond({'status': 'ok', 'response': {'deleted': True}})
