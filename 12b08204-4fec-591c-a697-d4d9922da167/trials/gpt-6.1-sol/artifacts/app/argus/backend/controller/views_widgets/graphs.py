from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from uuid import UUID
from datetime import datetime, timezone
from argus.backend.models.web import ArgusUserView, ArgusTest
from argus.backend.service.results_service import ResultsService
bp = APIRouter(prefix='/widgets')

@bp.api_route('/graphs/graph_views', methods=['GET', 'HEAD', 'OPTIONS'], name='graphs.get_graph_views', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_graph_views(request: Request):
    view_id = UUID(request.query_params.get('view_id'))
    view: ArgusUserView = ArgusUserView.get(id=view_id)
    start_date = request.query_params.get('start_date')
    end_date = request.query_params.get('end_date')
    service = ResultsService()
    response = {}
    tests_details = {}
    for test_id in view.tests:
        test_uuid = test_id
        graph_views = service.get_argus_graph_views(test_uuid)
        if graph_views:
            test_name = ArgusTest.get(id=test_uuid).name
            tests_details[str(test_id)] = {'name': test_name}
        view_data = []
        for graph_view in graph_views:
            table_names = set()
            for graph_name in graph_view.graphs.keys():
                table_name = graph_name.rsplit(' - ', 1)[0]
                table_names.add(table_name)
            start_dt = datetime.fromisoformat(start_date).astimezone(timezone.utc) if start_date else None
            end_dt = datetime.fromisoformat(end_date).astimezone(timezone.utc) if end_date else None
            graphs, ticks, releases_filters = service.get_test_graphs(test_id=test_uuid, start_date=start_dt, end_date=end_dt, table_names=list(table_names))
            graphs = [graph for graph in graphs if graph['options']['plugins']['title']['text'] in graph_view.graphs.keys()]
            if graphs:
                view_data.append({'id': str(graph_view.id), 'name': graph_view.name, 'description': graph_view.description, 'graphs': graphs, 'ticks': ticks, 'releases_filters': releases_filters})
        response[str(test_id)] = view_data
    return respond({'status': 'ok', 'response': response, 'tests_details': tests_details})
