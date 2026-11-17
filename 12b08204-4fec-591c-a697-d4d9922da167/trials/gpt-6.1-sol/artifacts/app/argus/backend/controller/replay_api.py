from __future__ import annotations
from fastapi import APIRouter, Request, Depends
from argus.backend.http import respond, authorize
'Replay-log ingest endpoint.\n\nPOST /api/v1/client/replay/ingest\n\nAccepts an archive of JSONL replay logs and re-applies the recorded\nrequests against the running FastAPI app (via an in-process proxy through\n``TestClient(app)``). See ``docs/plans/request_replay.md`` for\nthe full design.\n\nSupported archive formats: ``tar.zst`` (the canonical CLI output),\n``tar.gz``/``tgz``, plain ``tar``, and ``zip``. Format is detected from\nmagic bytes in the request body; the ``Content-Type`` header is only\nused to reject obvious mismatches at the door.\n'
from argus.backend.error_handlers import APIException, handle_api_exception
from argus.backend.service.replay_service import ReplayService
bp = APIRouter(prefix='/replay')
ACCEPTED_CONTENT_TYPES = frozenset({'application/x-tar-zstd', 'application/zstd', 'application/gzip', 'application/x-gzip', 'application/x-tar-gzip', 'application/x-tgz', 'application/x-tar', 'application/tar', 'application/zip', 'application/x-zip-compressed', 'application/x-zip', 'application/octet-stream'})

class UnsupportedMediaType(APIException):
    """Request body is not one of :data:`ACCEPTED_CONTENT_TYPES`."""

class EmptyRequest(APIException):
    """Request body is empty -- nothing to ingest."""

@bp.api_route('/ingest', methods=['POST', 'OPTIONS'], name='replay_api.replay_ingest', response_model=None, dependencies=[Depends(authorize(True, True, None, False))])
def replay_ingest(request: Request):
    content_type = (request.headers.get('content-type') or '').split(';', 1)[0].strip().lower()
    if content_type and content_type not in ACCEPTED_CONTENT_TYPES:
        raise UnsupportedMediaType(f'expected one of {sorted(ACCEPTED_CONTENT_TYPES)}, got {content_type!r}')
    archive = request.state.body
    if not archive:
        raise EmptyRequest('request body is empty')
    dry_run = request.query_params.get('dry_run', 'false').lower() in {'1', 'true', 'yes'}
    create_missing_tests = request.query_params.get('create_missing_tests', 'false').lower() in {'1', 'true', 'yes'}
    backfill_logs = request.query_params.get('backfill_logs', 'true').lower() in {'1', 'true', 'yes'}
    auth_header = request.headers.get('Authorization')
    summary = ReplayService(app=request.app, auth_header=auth_header, create_missing_tests=create_missing_tests, backfill_logs=backfill_logs).ingest(archive, dry_run=dry_run)
    return respond({'status': 'ok', 'response': summary.as_dict()})
