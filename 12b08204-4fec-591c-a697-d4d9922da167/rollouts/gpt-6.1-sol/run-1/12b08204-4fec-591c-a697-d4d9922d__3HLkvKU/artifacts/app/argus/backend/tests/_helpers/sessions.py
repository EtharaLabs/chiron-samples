"""Set up signed browser session cookies using the native client's cookie jar."""
from contextlib import contextmanager
from itsdangerous import URLSafeTimedSerializer, BadSignature
import hashlib

@contextmanager
def cookie_session(client):
    signer = URLSafeTimedSerializer(client.app.state.config.get('SECRET_KEY', ''), salt='cookie-session', signer_kwargs={'key_derivation': 'hmac', 'digest_method': hashlib.sha1})
    cookie = client.cookies.get('session')
    try:
        values = signer.loads(cookie) if cookie else {}
    except BadSignature:
        values = {}
    yield values
    client.cookies.clear()
    client.cookies.set('session', signer.dumps(values), domain='testserver.local', path='/')
