from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from uuid import UUID
from argus.backend.models.web import ArgusUserView
from argus.backend.service.results_service import ResultsService
from argus.backend.util.common import get_payload
bp = APIRouter(prefix='/widgets')

@bp.api_route('/summary/versioned_runs', methods=['GET', 'HEAD', 'OPTIONS'], name='summary.get_versioned_runs', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_versioned_runs(request: Request):
    view_id = UUID(request.query_params.get('view_id'))
    view: ArgusUserView = ArgusUserView.get(id=view_id)
    service = ResultsService()
    versioned_runs = service.get_tests_by_version('scylla-server', view.tests)
    return respond({'status': 'ok', 'response': versioned_runs})

@bp.api_route('/summary/runs_results', methods=['POST', 'OPTIONS'], name='summary.get_runs_results', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_runs_results(request: Request):
    versioned_runs = get_payload(request)
    service = ResultsService()
    response = {}
    for test_id, test_methods in versioned_runs.items():
        response[test_id] = {}
        for method, run in test_methods.items():
            response[test_id][method] = {}
            run_id = run['run_id']
            response[test_id][method][run_id] = service.get_run_results(UUID(test_id), UUID(run_id), key_metrics=['P99 read', 'P99 write', 'duration', 'Throughput write', 'Throughput read', 'allocs_per_op', 'cpu_cycles_per_op', 'instructions_per_op', 'logallocs_per_op'])
    return respond({'status': 'ok', 'response': response})
