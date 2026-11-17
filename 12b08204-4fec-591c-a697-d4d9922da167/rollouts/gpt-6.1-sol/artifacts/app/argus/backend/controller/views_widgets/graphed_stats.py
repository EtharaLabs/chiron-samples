from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from uuid import UUID
from argus.backend.util.common import get_payload
from argus.backend.models.web import ArgusUserView
from argus.backend.service.views_widgets.graphed_stats import GraphedStatsService
bp = APIRouter(prefix='/widgets')

@bp.api_route('/graphed_stats', methods=['GET', 'HEAD', 'OPTIONS'], name='graphed_stats.get_graphed_stats', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_graphed_stats(request: Request):
    view_id = UUID(request.query_params.get('view_id'))
    view: ArgusUserView = ArgusUserView.get(id=view_id)
    service = GraphedStatsService()
    response_data = {'test_runs': [], 'nemesis_data': []}
    filters = request.query_params.get('filters')
    for test_id in view.tests:
        data = service.get_graphed_stats(test_id, filters, request=request)
        response_data['test_runs'].extend(data['test_runs'])
        response_data['nemesis_data'].extend(data['nemesis_data'])
    return respond({'status': 'ok', 'response': response_data})

@bp.api_route('/runs_details', methods=['POST', 'OPTIONS'], name='graphed_stats.get_runs_details', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_runs_details(request: Request):
    """Get detailed information for provided test runs including assignee and attached issues."""
    data = get_payload(request)
    if not data or 'run_ids' not in data:
        raise ValueError('Missing run_ids parameter')
    run_ids = data['run_ids']
    if not isinstance(run_ids, list):
        raise ValueError('run_ids must be a list')
    service = GraphedStatsService()
    result = service.get_runs_details(run_ids, request=request)
    return respond({'status': 'ok', 'response': result})
