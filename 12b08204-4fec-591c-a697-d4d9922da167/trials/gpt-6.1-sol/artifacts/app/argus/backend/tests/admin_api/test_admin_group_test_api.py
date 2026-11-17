from argus.backend.util.config import Config
'Controller-level integration tests for group + test endpoints in\n``argus/backend/controller/admin_api.py``.\n\nURL prefix: ``/admin/api/v1`` (admin_api blueprint nested under admin).\n\nScope (iteration 9 of the controller coverage matrix):\n\n- ``POST /admin/api/v1/group/create``\n- ``POST /admin/api/v1/group/update``\n- ``POST /admin/api/v1/group/delete``\n- ``POST /admin/api/v1/test/create``\n- ``POST /admin/api/v1/test/update``\n- ``POST /admin/api/v1/test/delete``\n- ``POST /admin/api/v1/test/batch_move``\n- ``GET  /admin/api/v1/groups/get``\n- ``GET  /admin/api/v1/tests/get``\n- ``POST /admin/api/v1/release/group/state/toggle``\n- ``POST /admin/api/v1/release/test/state/toggle``\n'
import time
import uuid
import pytest
from argus.backend.tests.util import client_post
ADMIN_PREFIX = '/admin/api/v1'
API_PREFIX = '/api/v1'

@pytest.fixture
def admin_release(release_manager_service, *, test_request):
    name = f'adm9_rel_{time.time_ns()}'
    return release_manager_service.create_release(name, name, False, request=test_request)

@pytest.fixture
def admin_group_via_api(flask_client, admin_release):
    name = f'adm9_grp_{time.time_ns()}'
    payload = {'group_name': name, 'pretty_name': name, 'build_system_id': f'bsid_{time.time_ns()}', 'release_id': str(admin_release.id)}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/group/create', payload)
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    return body['response']['new_group']

@pytest.fixture
def admin_test_via_api(flask_client, admin_release, admin_group_via_api):
    name = f'adm9_test_{time.time_ns()}'
    payload = {'test_name': name, 'pretty_name': name, 'build_id': f'buildid_{time.time_ns()}', 'build_url': 'http://example.com/job/1', 'group_id': str(admin_group_via_api['id']), 'release_id': str(admin_release.id), 'plugin_name': 'scylla-cluster-tests'}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/test/create', payload)
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    return body['response']['new_test']

def test_admin_create_group_round_trip(flask_client, admin_release, admin_group_via_api):
    resp = flask_client.get(f"{API_PREFIX}/group/{admin_group_via_api['id']}/details")
    assert resp.status_code == 200
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['name'] == admin_group_via_api['name']
    assert str(body['response']['release_id']) == str(admin_release.id)

def test_admin_get_groups_includes_created(flask_client, admin_release, admin_group_via_api):
    resp = flask_client.get(f'{ADMIN_PREFIX}/groups/get', params={'releaseId': str(admin_release.id)})
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert any((g['id'] == admin_group_via_api['id'] and g['name'] == admin_group_via_api['name'] for g in body['response']))

def test_admin_get_groups_missing_release_id(flask_client):
    resp = flask_client.get(f'{ADMIN_PREFIX}/groups/get')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_update_group(flask_client, admin_group_via_api):
    payload = {'group_id': admin_group_via_api['id'], 'name': admin_group_via_api['name'] + '_upd', 'pretty_name': 'updated-pretty', 'enabled': False, 'build_system_id': 'updated-bsid'}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/group/update', payload)
    assert resp.status_code == 200, resp.content
    assert resp.json()['status'] == 'ok'
    resp = flask_client.get(f"{API_PREFIX}/group/{admin_group_via_api['id']}/details")
    assert resp.status_code == 200
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['pretty_name'] == 'updated-pretty'
    assert body['response']['enabled'] is False
    assert body['response']['build_system_id'] == 'updated-bsid'

def test_admin_toggle_group_enabled(flask_client, admin_group_via_api):
    payload = {'entityId': admin_group_via_api['id'], 'state': False}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/release/group/state/toggle', payload)
    assert resp.status_code == 200, resp.content
    assert resp.json()['status'] == 'ok'
    resp = flask_client.get(f"{API_PREFIX}/group/{admin_group_via_api['id']}/details")
    assert resp.status_code == 200
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['enabled'] is False

def test_admin_create_test_round_trip(flask_client, admin_group_via_api, admin_test_via_api):
    resp = flask_client.get(f"{API_PREFIX}/test/{admin_test_via_api['id']}/details")
    assert resp.status_code == 200
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['name'] == admin_test_via_api['name']
    assert str(body['response']['group_id']) == str(admin_group_via_api['id'])
    assert body['response']['plugin_name'] == 'scylla-cluster-tests'

def test_admin_get_tests_includes_created(flask_client, admin_group_via_api, admin_test_via_api):
    resp = flask_client.get(f'{ADMIN_PREFIX}/tests/get', params={'groupId': admin_group_via_api['id']})
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert any((t['id'] == admin_test_via_api['id'] and t['name'] == admin_test_via_api['name'] for t in body['response']))

def test_admin_get_tests_missing_group_id(flask_client):
    resp = flask_client.get(f'{ADMIN_PREFIX}/tests/get')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_update_test(flask_client, admin_group_via_api, admin_test_via_api):
    payload = {'test_id': admin_test_via_api['id'], 'name': admin_test_via_api['name'] + '_upd', 'pretty_name': 'updated-test-pretty', 'plugin_name': 'driver-matrix-tests', 'enabled': False, 'build_system_id': admin_test_via_api['build_system_id'], 'build_system_url': 'http://example.com/job/2', 'group_id': admin_group_via_api['id']}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/test/update', payload)
    assert resp.status_code == 200, resp.content
    assert resp.json()['status'] == 'ok'
    resp = flask_client.get(f"{API_PREFIX}/test/{admin_test_via_api['id']}/details")
    assert resp.status_code == 200
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['pretty_name'] == 'updated-test-pretty'
    assert body['response']['plugin_name'] == 'driver-matrix-tests'
    assert body['response']['enabled'] is False
    assert body['response']['build_system_url'] == 'http://example.com/job/2'

def test_admin_toggle_test_enabled(flask_client, admin_test_via_api):
    payload = {'entityId': admin_test_via_api['id'], 'state': False}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/release/test/state/toggle', payload)
    assert resp.status_code == 200, resp.content
    assert resp.json()['status'] == 'ok'
    resp = flask_client.get(f"{API_PREFIX}/test/{admin_test_via_api['id']}/details")
    assert resp.status_code == 200
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['enabled'] is False

def test_admin_batch_move_tests(flask_client, admin_release, admin_group_via_api, admin_test_via_api):
    second_group_name = f'adm9_grp2_{time.time_ns()}'
    create_resp = client_post(flask_client, f'{ADMIN_PREFIX}/group/create', {'group_name': second_group_name, 'pretty_name': second_group_name, 'build_system_id': f'bsid2_{time.time_ns()}', 'release_id': str(admin_release.id)})
    assert create_resp.status_code == 200, create_resp.content
    second_group = create_resp.json()['response']['new_group']
    payload = {'new_group_id': second_group['id'], 'tests': [admin_test_via_api['id']]}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/test/batch_move', payload)
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['moved'] is True
    resp = flask_client.get(f"{API_PREFIX}/test/{admin_test_via_api['id']}/details")
    assert resp.status_code == 200
    body = resp.json()
    assert body['status'] == 'ok'
    assert str(body['response']['group_id']) == str(second_group['id'])

def test_admin_delete_test(flask_client, admin_test_via_api):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/test/delete', {'test_id': admin_test_via_api['id']})
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['deleted'] is True
    miss = flask_client.get(f"{API_PREFIX}/test/{admin_test_via_api['id']}/details")
    assert miss.status_code == 200
    assert miss.json()['status'] == 'error'

def test_admin_delete_group_cascades_tests(flask_client, admin_group_via_api, admin_test_via_api):
    payload = {'group_id': admin_group_via_api['id'], 'delete_tests': True}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/group/delete', payload)
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response']['deleted'] is True
    miss_group = flask_client.get(f"{API_PREFIX}/group/{admin_group_via_api['id']}/details")
    assert miss_group.json()['status'] == 'error'
    miss_test = flask_client.get(f"{API_PREFIX}/test/{admin_test_via_api['id']}/details")
    assert miss_test.json()['status'] == 'error'

def test_admin_delete_group_relocates_tests(flask_client, admin_release, admin_group_via_api, admin_test_via_api):
    target_name = f'adm9_target_{time.time_ns()}'
    create_resp = client_post(flask_client, f'{ADMIN_PREFIX}/group/create', {'group_name': target_name, 'pretty_name': target_name, 'build_system_id': f'target_bsid_{time.time_ns()}', 'release_id': str(admin_release.id)})
    target = create_resp.json()['response']['new_group']
    payload = {'group_id': admin_group_via_api['id'], 'delete_tests': False, 'new_group_id': target['id']}
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/group/delete', payload)
    assert resp.status_code == 200, resp.content
    assert resp.json()['status'] == 'ok'
    resp = flask_client.get(f"{API_PREFIX}/test/{admin_test_via_api['id']}/details")
    assert resp.status_code == 200
    body = resp.json()
    assert body['status'] == 'ok'
    assert str(body['response']['group_id']) == str(target['id'])

def test_admin_delete_test_unknown_id_errors(flask_client):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/test/delete', {'test_id': str(uuid.uuid4())})
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_create_test_requires_json(flask_client):
    resp = flask_client.post(f'{ADMIN_PREFIX}/test/create', content='not-json', headers={'Content-Type': 'text/plain'})
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'
