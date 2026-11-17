"""Serve State A as the behaviour oracle.

Seeds one admin user whose API key is `oracle-admin-key` and one normal user
whose API key is `oracle-user-key` (send either in the DATALAB_API_KEY header),
then serves the app on 127.0.0.1. The configuration mirrors State A's own
server test configuration.
"""
import hashlib
import os
from pathlib import Path

import pymongo
from bson import ObjectId

port = int(os.environ.get("ORACLE_PORT", "5001"))
mongo_port = int(os.environ.get("ORACLE_MONGO_PORT", "27018"))
database = "datalab-oracle"
uri = f"mongodb://127.0.0.1:{mongo_port}/{database}"
files = Path(os.environ.get("ORACLE_FILES", "/tmp/oracle-files"))
files.mkdir(parents=True, exist_ok=True)

client = pymongo.MongoClient(uri)
if os.environ.get("ORACLE_KEEP_DATA") != "1":
    client.drop_database(database)
db = client.get_database(database)
for oid, key, role, name in (
    ("8" * 24, "oracle-admin-key", "admin", "Oracle Admin"),
    ("1" * 24, "oracle-user-key", "user", "Oracle User"),
):
    if db.users.find_one({"_id": ObjectId(oid)}) is None:
        db.users.insert_one(
            {"_id": ObjectId(oid), "contact_email": "oracle@example.org", "display_name": name, "account_status": "active"}
        )
        db.api_keys.insert_one(
            {"_id": ObjectId(), "user": ObjectId(oid), "name": "oracle key", "hash": hashlib.sha512(key.encode()).hexdigest(), "type": "api_key"}
        )
        db.roles.insert_one({"_id": ObjectId(oid), "role": role})

os.environ.setdefault("PYDATALAB_SECRET_KEY", hashlib.sha512(b"oracle").hexdigest())
from pydatalab.main import create_app  # noqa: E402

app = create_app(
    {
        "MONGO_URI": uri,
        "FILE_DIRECTORY": str(files),
        "TESTING": False,
        "ROOT_PATH": "/",
        "SECRET_KEY": os.environ["PYDATALAB_SECRET_KEY"],
        "REMOTE_FILESYSTEMS": [{"name": "example_data", "hostname": None, "path": "/opt/oracle/example_data"}],
        "MAX_CONTENT_LENGTH": 10 * 1000**2,
    },
    env_file=False,
)
app.run(host="127.0.0.1", port=port)
