from argus.backend.util.config import Config
'Controller-level integration tests for user endpoints in\n``argus/backend/controller/admin_api.py``.\n\nURL prefix: ``/admin/api/v1`` (admin_api blueprint nested under admin).\n\nScope (iteration 10 of the controller coverage matrix):\n\n- ``GET  /admin/api/v1/users``\n- ``POST /admin/api/v1/user/<id>/email/set``\n- ``POST /admin/api/v1/user/<id>/delete``\n- ``POST /admin/api/v1/user/<id>/password/set``\n- ``POST /admin/api/v1/user/<id>/admin/toggle``\n'
import json
import uuid
import pytest
from argus.backend.models.web import User, UserRoles
from argus.backend.tests.util import client_post
ADMIN_PREFIX = '/admin/api/v1'

@pytest.fixture
def saved_g_user(*, test_request):
    """Persist ``g.user`` so admin self-protection paths can compare ids."""
    test_request.state.user.password = 'test_password'
    test_request.state.user.roles = [r.value if hasattr(r, 'value') else r for r in test_request.state.user.roles]
    test_request.state.user.save()
    return test_request.state.user

@pytest.fixture
def regular_user():
    user = User(id=uuid.uuid4(), username=f'adm10_user_{uuid.uuid4().hex[:8]}', full_name='Admin10 Test User', email=f'adm10_{uuid.uuid4().hex[:8]}@scylladb.com', password='hash:placeholder', roles=[UserRoles.User.value])
    user.save()
    return user

def test_admin_users_returns_dict_without_password(flask_client, regular_user):
    resp = flask_client.get(f'{ADMIN_PREFIX}/users')
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert isinstance(body['response'], dict)
    assert str(regular_user.id) in body['response']
    entry = body['response'][str(regular_user.id)]
    assert 'password' not in entry
    assert 'api_token' not in entry
    assert entry['username'] == regular_user.username
    assert entry['email'] == regular_user.email

def test_admin_user_change_email(flask_client, regular_user):
    new_email = f'new_{uuid.uuid4().hex[:8]}@scylladb.com'
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/email/set', {'newEmail': new_email})
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response'] is True
    resp = flask_client.get(f'{ADMIN_PREFIX}/users')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'ok'
    assert resp.json()['response'][str(regular_user.id)]['email'] == new_email

def test_admin_user_change_email_invalid_format_errors(flask_client, regular_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/email/set', {'newEmail': 'not-a-valid-email'})
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_user_change_email_duplicate_errors(flask_client, regular_user, saved_g_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/email/set', {'newEmail': saved_g_user.email})
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_user_change_password(flask_client, regular_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/password/set', {'newPassword': 'newSecret123'})
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response'] is True

def test_admin_user_change_password_too_short_errors(flask_client, regular_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/password/set', {'newPassword': 'abc'})
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_user_change_password_empty_errors(flask_client, regular_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/password/set', {'newPassword': ''})
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_toggle_admin_grants_then_revokes(flask_client, regular_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/admin/toggle')
    assert resp.status_code == 200, resp.content
    assert resp.json()['status'] == 'ok'
    resp = flask_client.get(f'{ADMIN_PREFIX}/users')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'ok'
    assert UserRoles.Admin.value in resp.json()['response'][str(regular_user.id)]['roles']
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/admin/toggle')
    assert resp.status_code == 200, resp.content
    assert resp.json()['status'] == 'ok'
    resp = flask_client.get(f'{ADMIN_PREFIX}/users')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'ok'
    assert UserRoles.Admin.value not in resp.json()['response'][str(regular_user.id)]['roles']

def test_admin_toggle_admin_self_forbidden(flask_client, saved_g_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{saved_g_user.id}/admin/toggle')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_delete_user(flask_client, regular_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/delete')
    assert resp.status_code == 200, resp.content
    body = resp.json()
    assert body['status'] == 'ok'
    assert body['response'] is True
    resp = flask_client.get(f'{ADMIN_PREFIX}/users')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'ok'
    assert str(regular_user.id) not in resp.json()['response']

def test_admin_delete_user_self_forbidden(flask_client, saved_g_user):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{saved_g_user.id}/delete')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_delete_admin_user_forbidden(flask_client, regular_user):
    client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/admin/toggle')
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{regular_user.id}/delete')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'

def test_admin_delete_user_unknown_id_errors(flask_client):
    resp = client_post(flask_client, f'{ADMIN_PREFIX}/user/{uuid.uuid4()}/delete')
    assert resp.status_code == 200
    assert resp.json()['status'] == 'error'
