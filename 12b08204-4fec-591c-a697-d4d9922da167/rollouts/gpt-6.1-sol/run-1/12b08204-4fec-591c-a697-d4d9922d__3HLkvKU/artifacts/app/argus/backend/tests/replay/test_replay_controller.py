from __future__ import annotations
from argus.backend.util.config import Config
"Controller-level tests for :mod:`argus.backend.controller.replay_api`.\n\nStands up a minimal FastAPI app with only the replay blueprint registered so we\ndon't need the heavyweight Docker/ScyllaDB fixture. The ReplayService is\npatched out -- the dispatch logic itself is covered by\n``test_replay_service``.\n"
import io
import json
import tarfile
from unittest.mock import patch
import pytest
import zstandard as zstd
from fastapi import FastAPI, Request

@pytest.fixture
def app(monkeypatch):
    from unittest.mock import patch
    from argus_backend import create_app, prepare_request
    from fastapi import Request
    from starlette.datastructures import FormData
    monkeypatch.setattr(Config, 'CONFIG', {'SECRET_KEY': 'replay-tests', 'APP_LOG_LEVEL': 'INFO'})
    with patch('argus.backend.db.ScyllaCluster.get'):
        application = create_app()

    async def prepared(request: Request):
        request.state.body = await request.body()
        request.state.form = FormData()
        request.state.payload = None
        request.state.user = None
    application.dependency_overrides[prepare_request] = prepared
    for route in application.routes:
        if getattr(route, 'path', '') == '/api/v1/client/replay/ingest':
            for dependency in route.dependencies:
                if dependency.dependency is not prepare_request:
                    application.dependency_overrides[dependency.dependency] = lambda: None
    return application

@pytest.fixture
def client(app):
    from fastapi.testclient import TestClient
    return TestClient(app, raise_server_exceptions=False)

def _make_archive(records: list[dict]) -> bytes:
    tar_buf = io.BytesIO()
    with tarfile.open(fileobj=tar_buf, mode='w') as tar:
        payload = '\n'.join((json.dumps(r) for r in records)).encode('utf-8')
        info = tarfile.TarInfo(name='argus_replay_log_r_1.jsonl')
        info.size = len(payload)
        tar.addfile(info, io.BytesIO(payload))
    return zstd.ZstdCompressor().compress(tar_buf.getvalue())

def test_replay_ingest_rejects_unsupported_content_type(client):
    response = client.post('/api/v1/client/replay/ingest', content=b'...', headers={'Content-Type': 'text/plain'})
    assert response.status_code == 200
    assert response.json()['status'] == 'error'
    assert response.json()['response']['exception'] == 'UnsupportedMediaType'

def test_replay_ingest_rejects_empty_body(client):
    response = client.post('/api/v1/client/replay/ingest', content=b'', headers={'Content-Type': 'application/x-tar-zstd'})
    assert response.status_code == 200
    assert response.json()['status'] == 'error'
    assert response.json()['response']['exception'] == 'EmptyRequest'

def test_replay_ingest_returns_summary_for_valid_archive(client):
    archive = _make_archive([{'ts': 1, 'method': 'POST', 'endpoint': '/testrun/$type/submit', 'location_params': {'type': 'generic'}, 'params': None, 'body': {'run_id': 'x'}, 'test_type': 'generic', 'success': True}])
    with patch('argus.backend.controller.replay_api.ReplayService') as mock_service_cls:
        instance = mock_service_cls.return_value
        instance.ingest.return_value.as_dict.return_value = {'total': 1, 'processed': 1, 'succeeded': 1, 'failed': 0, 'skipped_no_replay': 0, 'errors': []}
        response = client.post('/api/v1/client/replay/ingest', content=archive, headers={'Content-Type': 'application/x-tar-zstd'})
    assert response.status_code == 200
    assert response.json()['status'] == 'ok'
    assert response.json()['response']['total'] == 1
    instance.ingest.assert_called_once()
    _, kwargs = instance.ingest.call_args
    assert kwargs['dry_run'] is False

def test_replay_ingest_dry_run_flag_forwarded(client):
    archive = _make_archive([])
    with patch('argus.backend.controller.replay_api.ReplayService') as mock_service_cls:
        instance = mock_service_cls.return_value
        instance.ingest.return_value.as_dict.return_value = {'total': 0, 'processed': 0, 'succeeded': 0, 'failed': 0, 'skipped_no_replay': 0, 'errors': []}
        client.post('/api/v1/client/replay/ingest?dry_run=true', content=archive, headers={'Content-Type': 'application/x-tar-zstd'})
        _, kwargs = instance.ingest.call_args
        assert kwargs['dry_run'] is True

def test_replay_ingest_forwards_service_error_through_handler(client):
    from argus.backend.service.replay_service import ReplayServiceError
    with patch('argus.backend.controller.replay_api.ReplayService') as mock_service_cls:
        mock_service_cls.return_value.ingest.side_effect = ReplayServiceError('Failed to decode archive: bad zstd')
        response = client.post('/api/v1/client/replay/ingest', content=b'garbage', headers={'Content-Type': 'application/x-tar-zstd'})
    assert response.status_code == 200
    assert response.json()['status'] == 'error'
    assert response.json()['response']['exception'] == 'ReplayServiceError'
    assert 'Failed to decode archive' in response.json()['response']['message']
