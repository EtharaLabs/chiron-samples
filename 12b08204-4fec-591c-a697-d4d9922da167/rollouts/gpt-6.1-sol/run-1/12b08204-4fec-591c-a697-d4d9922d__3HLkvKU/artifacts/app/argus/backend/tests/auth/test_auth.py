from argus.backend.tests._helpers.sessions import cookie_session
from argus.backend.util.config import Config
import uuid
import pytest
from werkzeug.security import generate_password_hash
from argus.backend.models.web import User, UserRoles

@pytest.fixture(autouse=True)
def secret_key(argus_app):
    """auth endpoints rely on signed sessions which require a secret key."""
    original = argus_app.state.config['SECRET_KEY']
    argus_app.state.config['SECRET_KEY'] = 'test-secret-key'
    yield
    argus_app.state.config['SECRET_KEY'] = original

@pytest.fixture
def saved_g_user(*, test_request):
    """Persist the authenticated user so endpoints can save it."""
    test_request.state.user.password = 'test_password_hash'
    test_request.state.user.roles = [role.value if hasattr(role, 'value') else role for role in test_request.state.user.roles]
    test_request.state.user.save()
    return test_request.state.user

def test_register_redirects_to_login(flask_client):
    res = flask_client.get('/auth/register', follow_redirects=False)
    assert res.status_code == 302
    assert '/auth/login' in res.headers['Location']

def test_login_get_authenticated_redirects_to_home(flask_client):
    res = flask_client.get('/auth/login', follow_redirects=False)
    assert res.status_code == 302
    assert res.headers['Location'].endswith('/')

def test_login_get_authenticated_uses_redirect_target(flask_client):
    with cookie_session(flask_client) as sess:
        sess['redirect_target'] = '/some/path'
    res = flask_client.get('/auth/login', follow_redirects=False)
    assert res.status_code == 302
    assert res.headers['Location'].endswith('/some/path')

def test_logout_clears_session_and_sets_manual_logout(flask_client):
    with cookie_session(flask_client) as sess:
        sess['user_id'] = str(uuid.uuid4())
    res = flask_client.post('/auth/logout', follow_redirects=False)
    assert res.status_code == 302
    assert '/auth/login' in res.headers['Location']
    with cookie_session(flask_client) as sess:
        assert 'user_id' not in sess
        assert sess.get('manual_logout') is True

def test_generate_api_token_persists_and_redirects_to_profile(flask_client, saved_g_user):
    res = flask_client.post('/auth/profile/api/token/generate', follow_redirects=False)
    assert res.status_code == 302
    assert '/profile' in res.headers['Location']
    refreshed = User.get(id=saved_g_user.id)
    assert refreshed.api_token
    with cookie_session(flask_client) as sess:
        assert sess.get('token_generated') == refreshed.api_token

def test_cf_login_without_jwt_redirects_to_login_with_manual_logout(flask_client):
    res = flask_client.post('/auth/login/cf', follow_redirects=False)
    assert res.status_code == 302
    assert '/auth/login' in res.headers['Location']
    with cookie_session(flask_client) as sess:
        assert sess.get('manual_logout') is True

def test_impersonate_get_renders_user_switch(flask_client):
    res = flask_client.get('/auth/admin/impersonate')
    assert res.status_code == 200
    assert b'user' in res.content.lower()

def test_impersonate_post_missing_user_id_flashes_and_redirects(flask_client):
    res = flask_client.post('/auth/admin/impersonate', data={}, follow_redirects=False)
    assert res.status_code == 302
    assert '/profile' in res.headers['Location']

def test_stop_impersonation_without_active_session_errors(flask_client):
    res = flask_client.post('/auth/admin/impersonate/stop', follow_redirects=False)
    assert res.status_code in (302, 200)

def test_password_login_success_sets_user_id_in_session(flask_client, argus_db, *, test_request):
    """Posting valid credentials with password login enabled stores user_id in session."""
    raw_password = 's3cret-pw'
    user = User()
    user.id = uuid.uuid4()
    user.username = f'pw-user-{uuid.uuid4().hex[:8]}'
    user.email = f'{user.username}@scylladb.com'
    user.password = generate_password_hash(raw_password)
    user.roles = ['ROLE_USER']
    user.save()
    original_methods = Config.load_yaml_config().get('LOGIN_METHODS', [])
    Config.load_yaml_config()['LOGIN_METHODS'] = ['password']
    original_g_user = test_request.state.user
    test_request.state.user = None
    try:
        res = flask_client.post('/auth/login', data={'username': user.username, 'password': raw_password}, follow_redirects=False)
        assert res.status_code == 302
        assert res.headers['Location'].endswith('/')
        with cookie_session(flask_client) as sess:
            assert sess.get('user_id') == str(user.id)
    finally:
        Config.load_yaml_config()['LOGIN_METHODS'] = original_methods
        test_request.state.user = original_g_user

def test_password_login_disabled_flashes_error(flask_client, *, test_request):
    """Password login posts redirect to /auth/login when password method disabled."""
    original_methods = Config.load_yaml_config().get('LOGIN_METHODS', [])
    Config.load_yaml_config()['LOGIN_METHODS'] = ['cf']
    original_g_user = test_request.state.user
    test_request.state.user = None
    try:
        with cookie_session(flask_client) as sess:
            sess.pop('user_id', None)
        res = flask_client.post('/auth/login', data={'username': 'anyone', 'password': 'x'}, follow_redirects=False)
        assert res.status_code == 302
        with cookie_session(flask_client) as sess:
            assert 'user_id' not in sess
    finally:
        Config.load_yaml_config()['LOGIN_METHODS'] = original_methods
        test_request.state.user = original_g_user

def test_cf_login_with_valid_jwt_logs_in_existing_user(flask_client, mock_cf_access_payload):
    """CF JWT happy path: /auth/login/cf logs in matching @scylladb.com user."""
    user = User()
    user.id = uuid.uuid4()
    user.username = f'cf-user-{uuid.uuid4().hex[:8]}'
    user.email = f'{user.username}@scylladb.com'
    user.roles = ['ROLE_USER']
    user.save()
    mock_cf_access_payload.return_value = {'email': user.email}
    original_methods = Config.load_yaml_config().get('LOGIN_METHODS', [])
    Config.load_yaml_config()['LOGIN_METHODS'] = ['cf']
    try:
        res = flask_client.post('/auth/login/cf', headers={'Cf-Access-Jwt-Assertion': 'fake-jwt'}, follow_redirects=False)
        assert res.status_code == 302
        assert '/profile' in res.headers['Location']
        with cookie_session(flask_client) as sess:
            assert sess.get('user_id') == str(user.id)
            assert sess.get('auth_via_cf') is True
    finally:
        Config.load_yaml_config()['LOGIN_METHODS'] = original_methods

def test_full_impersonation_flow(flask_client, *, test_request):
    """Admin impersonates another user, then stops impersonation."""
    target = User()
    target.id = uuid.uuid4()
    target.username = f'imp-{uuid.uuid4().hex[:8]}'
    target.email = f'{target.username}@scylladb.com'
    target.roles = ['ROLE_USER']
    target.save()
    test_request.state.user.password = 'irrelevant'
    test_request.state.user.save()
    admin_id = test_request.state.user.id
    with cookie_session(flask_client) as sess:
        sess.pop('original_user', None)
    res = flask_client.post('/auth/admin/impersonate', data={'user_id': str(target.id)}, follow_redirects=False)
    assert res.status_code == 302
    assert '/profile' in res.headers['Location']
    with cookie_session(flask_client) as sess:
        assert sess.get('original_user') == str(admin_id)
        assert sess.get('user_id') == str(target.id)
    res = flask_client.post('/auth/admin/impersonate/stop', follow_redirects=False)
    assert res.status_code == 302
    with cookie_session(flask_client) as sess:
        assert 'original_user' not in sess
        assert sess.get('user_id') == str(admin_id)
    test_request.state.user = User.get(id=admin_id)
