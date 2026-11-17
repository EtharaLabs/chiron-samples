from argus.backend.http import json_body
from fastapi import APIRouter, Request, Depends
from starlette.responses import Response
from argus.backend.http import respond, authorize
import logging
from dataclasses import asdict
from typing import TypedDict
from argus.backend.error_handlers import handle_api_exception
from argus.backend.models.web import UserRoles
from argus.backend.service.tunnel_service import TunnelService, TunnelServiceException
from argus.backend.service.user import allow_ssh_tunnel_server_scope

class RegisterTunnelPayload(TypedDict, total=False):
    public_key: str
    ttl_seconds: int
bp = APIRouter(prefix='/ssh')
LOGGER = logging.getLogger(__name__)

@bp.api_route('/tunnel', methods=['POST', 'OPTIONS'], name='ssh_api.register_tunnel', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def register_tunnel(request: Request):
    """
    Register a client SSH public key and obtain proxy tunnel connection details.

    Request JSON body
    -----------------
    public_key  : str  — OpenSSH-format ed25519 (or other) public key (required)
    ttl_seconds : int  — optional key lifetime in seconds.
                         Must be within [3600, 2592000] (1h..30d).
                         Default is 86400 (24h).

    Response
    --------
    {
        "status": "ok",
        "response": {
            "key_id":               "<uuid>",
            "proxy_host":           "proxy.example.com",
            "proxy_port":           22,
            "proxy_user":           "argus-proxy",
            "target_host":          "10.0.0.5",
            "target_port":          8080,
            "host_key_fingerprint": "SHA256:...",
            "expires_at":           "2026-04-16T12:00:00Z"
        }
    }
    """
    payload: RegisterTunnelPayload = json_body(request) or {}
    public_key = payload.get('public_key')
    ttl_seconds = payload.get('ttl_seconds')
    result = TunnelService().register_tunnel(user=request.state.user, public_key=public_key, ttl_seconds=ttl_seconds, request=request)
    return respond({'status': 'ok', 'response': asdict(result)})

@bp.api_route('/tunnel', methods=['GET', 'HEAD', 'OPTIONS'], name='ssh_api.get_tunnel_connection', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_tunnel_connection(request: Request):
    result = TunnelService().get_tunnel_connection(user_id=request.state.user.id, proxy_host=request.query_params.get('proxy_host'), request=request)
    return respond({'status': 'ok', 'response': asdict(result)})

@bp.api_route('/tunnel/keys', methods=['GET', 'HEAD', 'OPTIONS'], name='ssh_api.get_user_keys', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_user_keys(request: Request):
    """
    Return SSH keys owned by the authenticated user.

    Optional query params:
    - tunnel_id: UUID of a specific tunnel to scope keys
    """
    tunnel_id = request.query_params.get('tunnel_id')
    result = TunnelService().list_keys(tunnel_id=tunnel_id, user_id=request.state.user.id)
    return respond({'status': 'ok', 'response': [asdict(row) for row in result]})

@bp.api_route('/keys', methods=['GET', 'HEAD', 'OPTIONS'], name='ssh_api.get_authorized_keys', response_model=None, dependencies=[Depends(authorize(True, True, [UserRoles.SSHTunnelServer, UserRoles.Admin], False))])
def get_authorized_keys(request: Request):
    """
    Return non-expired SSH public keys in OpenSSH ``authorized_keys`` format
    (plain text, one key per line).

    This endpoint is called by the proxy host's ``AuthorizedKeysCommand``
    (via ``argus ssh keys list``) on every SSH connection attempt.

    Optional query params:
    - fingerprint: ``SHA256:...`` of the key the client offered, taken from the
      sshd ``%f`` token. Scopes the response to that one key. Proxy hosts that
      still run the old wrapper omit it and get the full list.
    """
    try:
        keys_text = TunnelService().get_authorized_keys(fingerprint=request.query_params.get('fingerprint'), request=request)
    except TunnelServiceException as exc:
        LOGGER.warning('Rejected authorized_keys request: %s', exc)
        return respond(Response('', media_type='text/plain', status_code=400))
    except Exception:
        LOGGER.exception('authorized_keys lookup failed')
        return respond(Response('', media_type='text/plain', status_code=500))
    return respond(Response(keys_text, media_type='text/plain'))
