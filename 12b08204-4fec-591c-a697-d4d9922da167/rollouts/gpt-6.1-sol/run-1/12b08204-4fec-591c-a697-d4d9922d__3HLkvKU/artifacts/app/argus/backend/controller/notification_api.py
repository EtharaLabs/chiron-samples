from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
import logging
from argus.backend.error_handlers import handle_api_exception
from argus.backend.service.notification_manager import NotificationManagerService
from argus.backend.util.common import get_payload
bp = APIRouter(prefix='/notifications')
LOGGER = logging.getLogger(__name__)

@bp.api_route('/get', methods=['GET', 'HEAD', 'OPTIONS'], name='notifications.get_notification', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_notification(request: Request):
    notification_id = request.query_params.get('id')
    if not notification_id:
        raise Exception('No notification id provided')
    service = NotificationManagerService()
    notification = service.get_notificaton(receiver=request.state.user.id, notification_id=notification_id)
    return respond({'status': 'ok', 'response': notification.to_dict()})

@bp.api_route('/get_unread', methods=['GET', 'HEAD', 'OPTIONS'], name='notifications.get_unread_count', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_unread_count(request: Request):
    service = NotificationManagerService()
    unread_count = service.get_unread_count(receiver=request.state.user.id, request=request)
    return respond({'status': 'ok', 'response': unread_count})

@bp.api_route('/summary', methods=['GET', 'HEAD', 'OPTIONS'], name='notifications.get_summary', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def get_summary(request: Request):
    after = request.query_params.get('afterId')
    limit = request.query_params.get('limit')
    limit = int(limit) if limit else 20
    service = NotificationManagerService()
    notifications = service.get_notifications(receiver=request.state.user.id, limit=limit, after=after)
    return respond({'status': 'ok', 'response': [n.to_dict_short_summary() for n in notifications]})

@bp.api_route('/read', methods=['POST', 'OPTIONS'], name='notifications.read_notification', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def read_notification(request: Request):
    payload = get_payload(request)
    service = NotificationManagerService()
    status = service.read_notification(receiver=request.state.user.id, notification_id=payload['id'], request=request)
    return respond({'status': 'ok', 'response': status})
