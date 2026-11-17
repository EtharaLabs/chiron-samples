from argus.backend.http import path_uuid
from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
import logging
from uuid import UUID
from argus.backend.error_handlers import handle_api_exception
from argus.backend.models.web import User
from argus.backend.service.argus_service import ArgusService
from argus.backend.util.common import get_payload
from argus.backend.service.team_manager_service import TeamManagerService
bp = APIRouter(prefix='/team')
LOGGER = logging.getLogger(__name__)

@bp.api_route('/create', methods=['POST', 'OPTIONS'], name='team_api.team_create', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def team_create(request: Request):
    payload = get_payload(request)
    result = TeamManagerService().create_team(name=payload['name'], leader=UUID(payload['leader']), members=[UUID(m) for m in payload['members']])
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{team_id}/get', methods=['GET', 'HEAD', 'OPTIONS'], name='team_api.team_get', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def team_get(request: Request, team_id: str):
    result = TeamManagerService().get_team_by_id(path_uuid(team_id))
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/{team_id}/delete', methods=['DELETE', 'OPTIONS'], name='team_api.team_delete', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def team_delete(request: Request, team_id: str):
    TeamManagerService().delete_team(path_uuid(team_id), request=request)
    return respond({'status': 'ok', 'response': {'team_id': team_id, 'status': 'deleted'}})

@bp.api_route('/{team_id}/edit', methods=['POST', 'OPTIONS'], name='team_api.team_edit', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def team_edit(request: Request, team_id: str):
    payload = get_payload(request)
    team = TeamManagerService().edit_team(team_id=UUID(payload['id']), name=payload['name'], members=[UUID(m) for m in payload['members']], request=request)
    return respond({'status': 'ok', 'response': {'team_id': team_id, 'status': 'updated', 'team': team}})

@bp.api_route('/{team_id}/motd/edit', methods=['POST', 'OPTIONS'], name='team_api.team_edit_motd', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def team_edit_motd(request: Request, team_id: str):
    payload = get_payload(request)
    TeamManagerService().edit_team_motd(team_id=UUID(payload['id']), message=payload['motd'], request=request)
    return respond({'status': 'ok', 'response': {'team_id': team_id, 'status': 'updated'}})

@bp.api_route('/user/{user_id}/teams', methods=['GET', 'HEAD', 'OPTIONS'], name='team_api.user_teams', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def user_teams(request: Request, user_id: str):
    result = TeamManagerService().get_users_teams(user_id=path_uuid(user_id))
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/user/{user_id}/jobs', methods=['GET', 'HEAD', 'OPTIONS'], name='team_api.user_jobs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def user_jobs(request: Request, user_id: str):
    user = User.get(id=path_uuid(user_id))
    result = list(ArgusService().get_jobs_for_user(user))
    return respond({'status': 'ok', 'response': result})

@bp.api_route('/leader/{user_id}/teams', methods=['GET', 'HEAD', 'OPTIONS'], name='team_api.leader_teams', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def leader_teams(request: Request, user_id: str):
    result = TeamManagerService().get_teams_for_user(user_id=path_uuid(user_id))
    return respond({'status': 'ok', 'response': result})
