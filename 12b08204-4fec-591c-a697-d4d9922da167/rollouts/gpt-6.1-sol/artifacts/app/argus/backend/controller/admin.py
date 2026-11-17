from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, render_template, authorize
import logging
from argus.backend.controller.admin_api import bp as admin_api_bp
from argus.backend.models.web import UserRoles
bp = APIRouter(prefix='/admin')
bp.include_router(admin_api_bp)
LOGGER = logging.getLogger(__name__)

@bp.api_route('/', methods=['GET', 'HEAD', 'OPTIONS'], name='admin.index', response_model=None, dependencies=[Depends(authorize(True, False, UserRoles.Admin, False))])
@bp.api_route('/{path}', methods=['GET', 'HEAD', 'OPTIONS'], name='admin.index', response_model=None, dependencies=[Depends(authorize(True, False, UserRoles.Admin, False))])
def index(request: Request, path: str='index'):
    return respond(render_template(request, 'admin/index.html.j2', current_route=path))
