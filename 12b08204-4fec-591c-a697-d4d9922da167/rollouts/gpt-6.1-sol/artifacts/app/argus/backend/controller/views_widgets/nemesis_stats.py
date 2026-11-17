from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from uuid import UUID
from argus.backend.models.web import ArgusUserView
from argus.backend.service.views_widgets.nemesis_stats import NemesisStatsService
bp = APIRouter(prefix='/widgets')

@bp.api_route('/nemesis_data', methods=['GET', 'HEAD', 'OPTIONS'], name='nemesis_stats.get_nemesis_data', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_nemesis_data(request: Request):
    view_id = UUID(request.query_params.get('view_id'))
    view: ArgusUserView = ArgusUserView.get(id=view_id)
    service = NemesisStatsService()
    nemesis_data = []
    for test_id in view.tests:
        data = service.get_nemesis_data(test_id, request=request)
        nemesis_data.extend(data)
    return respond({'status': 'ok', 'response': {'nemesis_data': nemesis_data}})
