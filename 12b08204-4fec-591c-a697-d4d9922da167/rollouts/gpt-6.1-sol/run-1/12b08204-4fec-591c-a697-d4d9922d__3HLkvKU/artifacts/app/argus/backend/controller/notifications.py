from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, render_template, authorize
bp = APIRouter(prefix='/notifications')

@bp.api_route('/', methods=['GET', 'HEAD', 'OPTIONS'], name='notifications.index', response_model=None, dependencies=[Depends(authorize(True, False, None, False))])
def index(request: Request):
    return respond(render_template(request, 'profile_notifications.html.j2'))
