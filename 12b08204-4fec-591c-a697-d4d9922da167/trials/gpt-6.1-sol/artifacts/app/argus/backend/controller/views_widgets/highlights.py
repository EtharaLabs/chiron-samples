from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
from dataclasses import asdict
from uuid import UUID
from argus.backend.service.views_widgets.highlights import HighlightCreate, HighlightsService, HighlightArchive, HighlightUpdate, HighlightSetAssignee, HighlightSetCompleted, CommentUpdate, CommentDelete, CommentCreate, HighlightGroupCreate
from argus.backend.util.common import get_payload
bp = APIRouter(prefix='/widgets')

@bp.api_route('/highlights/create', methods=['POST', 'OPTIONS'], name='view_widgets.create_highlight', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def create_highlight(request: Request):
    creator_id = request.state.user.id
    payload = HighlightCreate(**get_payload(request))
    service = HighlightsService()
    highlight = service.create(creator_id, payload, request=request)
    return respond({'status': 'ok', 'response': asdict(highlight)})

@bp.api_route('/highlights/create_group', methods=['POST', 'OPTIONS'], name='view_widgets.create_highlight_group', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def create_highlight_group(request: Request):
    creator_id = request.state.user.id
    payload = HighlightGroupCreate(**get_payload(request))
    service = HighlightsService()
    action_items = service.create_group(creator_id, payload, request=request)
    return respond({'status': 'ok', 'response': [asdict(action) for action in action_items]})

@bp.api_route('/highlights', methods=['GET', 'HEAD', 'OPTIONS'], name='view_widgets.get_highlights', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_highlights(request: Request):
    view_id = UUID(request.query_params.get('view_id'))
    index = int(request.query_params.get('index'))
    service = HighlightsService()
    highlights, action_items = service.get_highlights(view_id, index, request=request)
    return respond({'status': 'ok', 'response': {'highlights': [asdict(h) for h in highlights], 'action_items': [asdict(a) for a in action_items]}})

@bp.api_route('/highlights/archive', methods=['POST', 'OPTIONS'], name='view_widgets.archive_highlight', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def archive_highlight(request: Request):
    payload = HighlightArchive(**get_payload(request))
    service = HighlightsService()
    service.archive_highlight(payload, request=request)
    return respond({'status': 'ok'})

@bp.api_route('/highlights/unarchive', methods=['POST', 'OPTIONS'], name='view_widgets.unarchive_highlight', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def unarchive_highlight(request: Request):
    payload = HighlightArchive(**get_payload(request))
    service = HighlightsService()
    service.unarchive_highlight(payload, request=request)
    return respond({'status': 'ok'})

@bp.api_route('/highlights/update', methods=['POST', 'OPTIONS'], name='view_widgets.update_highlight', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def update_highlight(request: Request):
    payload = HighlightUpdate(**get_payload(request))
    service = HighlightsService()
    updated_highlight = service.update_highlight(request.state.user.id, payload, request=request)
    return respond({'status': 'ok', 'response': asdict(updated_highlight)})

@bp.api_route('/highlights/set_assignee', methods=['POST', 'OPTIONS'], name='view_widgets.set_assignee', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def set_assignee(request: Request):
    payload = HighlightSetAssignee(**get_payload(request))
    service = HighlightsService()
    updated_action_item = service.set_assignee(payload, request=request)
    if payload.assignee_id:
        service.send_action_notification(sender_id=request.state.user.id, username=request.state.user.username, view_id=payload.view_id, assignee_id=payload.assignee_id, action=updated_action_item.content, request=request)
    return respond({'status': 'ok', 'response': asdict(updated_action_item)})

@bp.api_route('/highlights/set_completed', methods=['POST', 'OPTIONS'], name='view_widgets.set_completed', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def set_completed(request: Request):
    payload = HighlightSetCompleted(**get_payload(request))
    service = HighlightsService()
    updated_action_item = service.set_completed(payload, request=request)
    return respond({'status': 'ok', 'response': asdict(updated_action_item)})

@bp.api_route('/highlights/comments/create', methods=['POST', 'OPTIONS'], name='view_widgets.create_comment', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def create_comment(request: Request):
    creator_id = request.state.user.id
    payload = CommentCreate(**get_payload(request))
    service = HighlightsService()
    comment = service.create_comment(creator_id, payload, request=request)
    return respond({'status': 'ok', 'response': asdict(comment)})

@bp.api_route('/highlights/comments/update', methods=['POST', 'OPTIONS'], name='view_widgets.update_comment', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def update_comment(request: Request):
    user_id = request.state.user.id
    payload = CommentUpdate(**get_payload(request))
    service = HighlightsService()
    updated_comment = service.update_comment(user_id, payload, request=request)
    return respond({'status': 'ok', 'response': asdict(updated_comment)})

@bp.api_route('/highlights/comments/delete', methods=['POST', 'OPTIONS'], name='view_widgets.delete_comment', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def delete_comment(request: Request):
    user_id = request.state.user.id
    payload = CommentDelete(**get_payload(request))
    service = HighlightsService()
    service.delete_comment(user_id, payload, request=request)
    return respond({'status': 'ok'})

@bp.api_route('/highlights/comments', methods=['GET', 'HEAD', 'OPTIONS'], name='view_widgets.get_comments', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_comments(request: Request):
    view_id = UUID(request.query_params.get('view_id'))
    index = int(request.query_params.get('index'))
    highlight_created_at = float(request.query_params.get('created_at'))
    service = HighlightsService()
    comments = service.get_comments(view_id, index, highlight_created_at, request=request)
    return respond({'status': 'ok', 'response': [asdict(c) for c in comments]})
