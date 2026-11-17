from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, render_template, authorize
import logging
from uuid import UUID
from argus.backend.controller.notifications import bp as notifications_bp
from argus.backend.models.web import WebFileStorage
from argus.backend.service.user import UserService
LOGGER = logging.getLogger(__name__)
bp = APIRouter(prefix='/teams')

@bp.api_route('/', methods=['GET', 'HEAD', 'OPTIONS'], name='teams.index', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def index(request: Request):
    return respond(render_template(request, 'teams.html.j2'))
