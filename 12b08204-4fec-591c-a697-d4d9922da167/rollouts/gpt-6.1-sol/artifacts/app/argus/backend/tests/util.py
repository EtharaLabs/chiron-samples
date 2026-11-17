from argus.backend.util.config import Config
import json

def client_post(flask_client, path: str, payload: dict | None=None):
    return flask_client.post(path, content=json.dumps(payload or {}), headers={'Content-Type': 'application/json'})
