"""Replay recorded cases against one installed build of the application.

usage: python drive.py <cases.json> <out.json> --files <project file list> [--work DIR]

Runs inside the interpreter of the build under test (State A on its retired
set when recording, the graded build when grading). It talks to the
application only through its HTTP interface (the Flask test client), its
settings object and its MongoDB database, so the same cases replay on any
migration. Besides the observations it reports three runtime probes that the
Stage I items read: imports of pydantic.v1 attributed to project files,
pydantic deprecation warnings attributed to project files, and the model
classes the project defines.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import importlib.abc
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
import traceback
import warnings
from pathlib import Path

ARGS = argparse.ArgumentParser()
ARGS.add_argument("cases")
ARGS.add_argument("out")
ARGS.add_argument("--files", required=True, help="file listing the project's installed files, one path per line")
ARGS.add_argument("--work", default=None)
ARGS.add_argument("--settings-case", default=None, help=argparse.SUPPRESS)
ARGS.add_argument("--mongo", default="mongodb://127.0.0.1:27017")
OPTS = ARGS.parse_args()

PROJECT_FILES = {
    os.path.realpath(line.strip()) for line in Path(OPTS.files).read_text().splitlines() if line.strip()
}
SELF = os.path.realpath(__file__)


def project_frame(skip_self: bool = True) -> tuple[str, int] | None:
    """The innermost caller frame that lies in a project file."""
    frame = sys._getframe(1)
    while frame is not None:
        name = os.path.realpath(frame.f_code.co_filename)
        if name in PROJECT_FILES:
            return name, frame.f_lineno
        frame = frame.f_back
    return None


V1_IMPORTS: set[tuple[str, int, str]] = set()


class V1Watch(importlib.abc.MetaPathFinder):
    def find_spec(self, fullname, path=None, target=None):
        if fullname == "pydantic.v1" or fullname.startswith("pydantic.v1."):
            # Attribute to the nearest frame outside the import machinery.
            frame = sys._getframe(1)
            while frame is not None:
                name = frame.f_code.co_filename
                if not name.startswith("<frozen importlib") and "importlib" not in Path(name).parts[-2:-1]:
                    real = os.path.realpath(name)
                    if real != SELF:
                        if real in PROJECT_FILES:
                            V1_IMPORTS.add((real, frame.f_lineno, fullname))
                        break
                frame = frame.f_back
        return None


sys.meta_path.insert(0, V1Watch())

DEPRECATIONS: set[tuple[str, int, str]] = set()
_show = warnings.showwarning


def _record(message, category, filename, lineno, file=None, line=None):
    real = os.path.realpath(filename)
    if "PydanticDeprecat" in category.__name__ and real in PROJECT_FILES:
        DEPRECATIONS.add((real, lineno, category.__name__))


warnings.showwarning = _record
warnings.simplefilter("always")

_warn = warnings.warn


def _warn_recorded(message, category=None, stacklevel=1, source=None, **kwargs):
    """Record a pydantic deprecation before any filter the application sets can drop it."""
    kind = category if category is not None else (type(message) if isinstance(message, Warning) else UserWarning)
    if isinstance(kind, type) and "PydanticDeprecat" in kind.__name__:
        frame = sys._getframe(1)
        for _ in range(max(int(stacklevel), 1) - 1):
            if frame.f_back is None:
                break
            frame = frame.f_back
        real = os.path.realpath(frame.f_code.co_filename)
        if real in PROJECT_FILES:
            DEPRECATIONS.add((real, frame.f_lineno, kind.__name__))
    return _warn(message, category, stacklevel + 1, source, **kwargs)


warnings.warn = _warn_recorded

# ---------------------------------------------------------------- encoding


def encode(value, depth: int = 0):
    """A version-neutral JSON view of a Python value read from the application."""
    import datetime
    import enum

    if depth > 12:
        return "<deep>"
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, enum.Enum):
        return {"enum": encode(value.value, depth + 1)}
    if isinstance(value, (datetime.datetime, datetime.date, datetime.time)):
        return value.isoformat()
    if isinstance(value, Path):
        return str(value)
    if hasattr(value, "unicode_string") and type(value).__module__.startswith(("pydantic", "pydantic_core")):
        # pydantic 2 URL objects; pydantic 1 URLs are str subclasses and take the str branch above.
        return str(value)
    if isinstance(value, bytes):
        return {"bytes_sha256": hashlib.sha256(value).hexdigest()}
    if isinstance(value, dict):
        return {str(k): encode(v, depth + 1) for k, v in value.items()}
    if isinstance(value, (list, tuple, set, frozenset)):
        items = [encode(v, depth + 1) for v in value]
        if isinstance(value, (set, frozenset)):
            items = sorted(items, key=lambda v: json.dumps(v, sort_keys=True))
        return items
    if isinstance(value, type):
        return {"class": value.__qualname__}
    fields = getattr(type(value), "model_fields", None)
    if fields is None and hasattr(type(value), "__fields__") and hasattr(value, "__dict__"):
        fields = getattr(type(value), "__fields__")
    if isinstance(fields, dict):
        data = dict(getattr(value, "__dict__", {}))
        data.update(getattr(value, "__pydantic_extra__", None) or {})
        return {"model": {str(k): encode(v, depth + 1) for k, v in data.items()}}
    return {"str": str(value)}


# ------------------------------------------------------------ normalization

OID = re.compile(r"(?<![0-9a-f])[0-9a-f]{24}(?![0-9a-f])")
STAMP = re.compile(
    r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?"
)


class Normalizer:
    """ObjectIds and generated refcode suffixes become placeholders keyed by first appearance;
    wall-clock timestamps become one placeholder carrying only their shape; nothing else changes."""

    def __init__(self, start: float, prefix: str):
        import datetime

        self.lo = datetime.datetime.fromtimestamp(start - 172800, datetime.timezone.utc)
        self.hi = datetime.datetime.fromtimestamp(start + 172800 + 86400, datetime.timezone.utc)
        self.keys: dict[tuple[str, str], str] = {}
        self.refcode = re.compile(re.escape(prefix) + r":([A-Z]{6})\b")

    def _key(self, kind: str, raw: str) -> str:
        key = (kind, raw)
        if key not in self.keys:
            self.keys[key] = f"<{kind}{sum(1 for k in self.keys if k[0] == kind) + 1}>"
        return self.keys[key]

    def _stamp(self, match: re.Match) -> str:
        import datetime

        raw = match.group(0)
        text = raw.replace(" ", "T").replace("Z", "+00:00")
        try:
            moment = datetime.datetime.fromisoformat(text)
        except ValueError:
            return raw
        if moment.tzinfo is None:
            moment = moment.replace(tzinfo=datetime.timezone.utc)
        if not (self.lo <= moment <= self.hi):
            return raw
        # The shape keeps the date layout and offset but not the fraction: a reading whose
        # microseconds land on a whole millisecond serializes with no fraction at all.
        # No number: whether two separate clock readings land in the same millisecond is not
        # behaviour a port can reproduce, so only the shape is compared.
        shape = re.sub(r"\.d+", "", re.sub(r"\d", "d", raw))
        return "<ts|" + shape + ">"

    def text(self, value: str) -> str:
        value = OID.sub(lambda m: self._key("oid", m.group(0)), value)
        value = self.refcode.sub(lambda m: m.group(0)[: -6] + self._key("ref", m.group(1)), value)
        return STAMP.sub(self._stamp, value)

    def __call__(self, value):
        if isinstance(value, str):
            return self.text(value)
        if isinstance(value, dict):
            return {self.text(str(k)): self(v) for k, v in sorted(value.items(), key=lambda kv: str(kv[0]))}
        if isinstance(value, list):
            return [self(v) for v in value]
        return value


UUID = re.compile(r"(?<![0-9a-f])[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}(?![0-9a-f])")
BOKEH_ID = re.compile(r"p?[0-9]+")


def generated_ids(observations: list) -> list:
    """Second pass over one case's normalized observations, in step order.

    uuid4 values (export and block task ids) become <taskN> keyed by first appearance within
    the case, as ObjectIds are. Bokeh document ids (string values of id and root_id, and members
    of root_ids, inside a bokeh_plot_data value) become <bkN> keyed by first appearance within
    the observation: bokeh numbers its models from a process-wide counter.
    """
    tasks: dict[str, str] = {}

    def task(match: re.Match) -> str:
        return tasks.setdefault(match.group(0), f"<task{len(tasks) + 1}>")

    def bokeh(raw, ids: dict):
        if isinstance(raw, str) and BOKEH_ID.fullmatch(raw):
            return ids.setdefault(raw, f"<bk{len(ids) + 1}>")
        return walk(raw, ids, True)

    def walk(value, ids: dict, inside: bool):
        if isinstance(value, str):
            return UUID.sub(task, value)
        if isinstance(value, dict):
            out = {}
            for key, item in value.items():
                if inside and key in ("id", "root_id"):
                    out[key] = bokeh(item, ids)
                elif inside and key == "root_ids" and isinstance(item, list):
                    out[key] = [bokeh(x, ids) for x in item]
                else:
                    out[key] = walk(item, ids, inside or key == "bokeh_plot_data")
            return out
        if isinstance(value, list):
            return [walk(item, ids, inside) for item in value]
        return value

    return [[index, walk(obs, {}, False)] for index, obs in observations]


# ------------------------------------------------------------------- users

USERS = {
    "user": ("111111111111111111111111", "a1b2c3d4e5f6a1b2c3d4e5f6", "user", "Test User", "active", True),
    "another": ("777777777777777777777777", "0f0e0d0c0b0a0f0e0d0c0b0a", "user", "Another User", "active", True),
    "admin": ("888888888888888888888888", "abcdefabcdef012345678901", "admin", "Test Admin", "active", False),
    "deactivated": ("333333333333333333333333", "3333aaaa3333aaaa3333aaaa", "user", "Deactivated User", "deactivated", False),
    "unverified": ("222222222222222222222222", "2222bbbb2222bbbb2222bbbb", "user", "Unverified User", "unverified", False),
}
GROUP_ID = "222222222222222222222220"


def seed_users(db):
    from bson import ObjectId

    db.groups.insert_one({"_id": ObjectId(GROUP_ID), "display_name": "Demo Group", "group_id": "demo"})
    for oid, key, role, name, status, grouped in USERS.values():
        doc = {"_id": ObjectId(oid), "contact_email": "test@example.org", "display_name": name, "account_status": status}
        if grouped:
            doc["groups"] = [{"immutable_id": ObjectId(GROUP_ID)}]
        db.users.insert_one(doc)
        db.api_keys.insert_one(
            {"_id": ObjectId(), "user": ObjectId(oid), "name": "testing API key",
             "hash": hashlib.sha512(key.encode()).hexdigest(), "type": "api_key"}
        )
        db.roles.insert_one({"_id": ObjectId(oid), "role": role})


# -------------------------------------------------------------- substitution

VAR = re.compile(r"\$\{([A-Za-z0-9_]+)\}")


def subst(value, env: dict):
    if isinstance(value, str):
        whole = VAR.fullmatch(value)
        if whole and whole.group(1) in env and not isinstance(env[whole.group(1)], str):
            return env[whole.group(1)]
        return VAR.sub(lambda m: str(env.get(m.group(1), m.group(0))), value)
    if isinstance(value, dict):
        return {subst(k, env): subst(v, env) for k, v in value.items()}
    if isinstance(value, list):
        return [subst(v, env) for v in value]
    return value


def pointer(doc, path: str):
    if path in ("", "/"):
        return doc
    for part in path.lstrip("/").split("/"):
        part = part.replace("~1", "/").replace("~0", "~")
        if isinstance(doc, list):
            doc = doc[int(part)]
        elif isinstance(doc, dict):
            doc = doc[part]
        else:
            raise KeyError(path)
    return doc


# --------------------------------------------------------------- http cases


class Runner:
    def __init__(self, work: Path):
        from pydatalab.main import create_app
        import pymongo

        self.work = work
        self.files = work / "files"
        self.files.mkdir(parents=True, exist_ok=True)
        self.example = Path(os.environ["GRADER_EXAMPLE_DATA"])
        self.dbname = "datalab-grading"
        uri = f"{OPTS.mongo}/{self.dbname}"
        self.client = pymongo.MongoClient(uri)
        self.client.drop_database(self.dbname)
        self.app = create_app(
            {
                "MONGO_URI": uri,
                "REMOTE_FILESYSTEMS": [
                    {"name": "example_data", "hostname": None, "path": str(self.example)},
                    {"name": "example/data", "hostname": None, "path": str(self.example)},
                ],
                "FILE_DIRECTORY": str(self.files),
                "TESTING": False,
                "ROOT_PATH": "/",
                "SECRET_KEY": os.environ["PYDATALAB_SECRET_KEY"],
                "AUTO_ACTIVATE_ACCOUNTS": False,
                "EMAIL_AUTO_ACTIVATE_ACCOUNTS": False,
                "EMAIL_AUTH_SMTP_SETTINGS": {
                    "MAIL_SERVER": "smtp.example.com",
                    "MAIL_PORT": 587,
                    "MAIL_USERNAME": "test",
                    "MAIL_USE_TLS": True,
                    "MAIL_DEFAULT_SENDER": "test@example.org",
                },
                "EMAIL_DOMAIN_ALLOW_LIST": ["example.org", "ml-evs.science"],
                "MAIL_DEBUG": True,
                "MAIL_SUPPRESS_SEND": True,
                "MAIL_PASSWORD": "test",
                "MAX_CONTENT_LENGTH": 10 * 1000**2,
            },
            env_file=False,
        )
        from pydatalab.config import CONFIG

        self.prefix = CONFIG.IDENTIFIER_PREFIX

    def reset(self):
        self.client.drop_database(self.dbname)
        shutil.rmtree(self.files, ignore_errors=True)
        self.files.mkdir(parents=True)
        seed_users(self.client.get_database(self.dbname))

    def client_for(self, who: str):
        from flask.testing import FlaskClient

        self.app.test_client_class = FlaskClient
        cli = self.app.test_client()
        if who in USERS:
            cli.environ_base["HTTP_DATALAB_API_KEY"] = USERS[who][1]
        elif who.startswith("session:"):
            with cli.session_transaction() as session:
                session["_user_id"] = USERS[who.split(":", 1)[1]][0]
        return cli

    def request(self, step: dict, env: dict):
        cli = self.client_for(step.get("as", "user"))
        kwargs = {}
        if "json" in step:
            kwargs["json"] = subst(step["json"], env)
        if "query" in step:
            kwargs["query_string"] = subst(step["query"], env)
        if "form" in step or "files" in step:
            data: dict = dict(subst(step.get("form", {}), env))
            import io

            for field, specs in step.get("files", {}).items():
                uploads = []
                for spec in specs if isinstance(specs, list) else [specs]:
                    if "example" in spec:
                        blob = (self.example / spec["example"]).read_bytes()
                        name = spec.get("name", Path(spec["example"]).name)
                    else:
                        blob = base64.b64decode(spec["b64"]) if "b64" in spec else spec["text"].encode()
                        name = spec["name"]
                    uploads.append((io.BytesIO(blob), name))
                data[field] = uploads if isinstance(specs, list) else uploads[0]
            kwargs["data"] = data
            kwargs["content_type"] = "multipart/form-data"
        if "raw" in step:
            kwargs["data"] = step["raw"]
            kwargs["content_type"] = step.get("content_type", "application/json")
        response = cli.open(subst(step["path"], env), method=step.get("method", "GET"), **kwargs)
        body = response.get_json(silent=True)
        return response, body

    def run_case(self, case: dict) -> list:
        import time

        from bson import json_util
        from bson.json_util import DatetimeRepresentation, JSONMode, JSONOptions

        # Stored datetimes as ISO 8601 text, so the wall-clock rule applies to them as to responses.
        DB_JSON = JSONOptions(json_mode=JSONMode.RELAXED, datetime_representation=DatetimeRepresentation.ISO8601, tz_aware=True)

        self.reset()
        random.seed(case["id"])
        norm = Normalizer(time.time(), self.prefix)
        env: dict = {}
        out = []
        db = self.client.get_database(self.dbname)
        for index, step in enumerate(case["steps"]):
            if "mark" in step:
                continue
            if "db_insert" in step:
                spec = step["db_insert"]
                try:
                    db[spec["collection"]].insert_many(json_util.loads(json.dumps(subst(spec["docs"], env))))
                except Exception:  # an unresolved capture or a rejected document
                    out.append([index, {"db_op": "error"}])
                continue
            if "db_op" in step:
                spec = step["db_op"]
                try:
                    args = json_util.loads(json.dumps(subst(spec["args"], env)))
                    getattr(db[spec["collection"]], spec["op"])(*args)
                except Exception:
                    out.append([index, {"db_op": "error"}])
                continue
            if "db_find" in step:
                spec = step["db_find"]
                try:
                    found = list(
                        db[spec["collection"]].find(
                            json_util.loads(json.dumps(subst(spec.get("filter", {}), env))),
                            spec.get("projection"),
                        ).sort(spec.get("sort", [["_id", 1]]))
                    )
                except Exception:  # an unresolved capture in the filter
                    out.append([index, {"db_op": "error"}])
                    continue
                out.append([index, {"db": norm(json.loads(json_util.dumps(found, json_options=DB_JSON)))}])
                continue
            try:
                response, body = self.request(step, env)
            except Exception:  # the request crashed inside the application
                out.append([index, {"crash": "exception"}])
                continue
            for name, path in step.get("capture", {}).items():
                try:
                    env[name] = pointer(body, path)
                except (KeyError, IndexError, TypeError, ValueError):
                    env[name] = None
            observe = step.get("observe", "full")
            if observe == "none":
                continue
            row: dict = {"status": response.status_code}
            if observe == "status":
                out.append([index, row])
                continue
            if body is not None:
                if isinstance(observe, list):
                    view = {}
                    for path in observe:
                        try:
                            view[path] = pointer(body, path)
                        except (KeyError, IndexError, TypeError, ValueError):
                            view[path] = "<absent>"
                    row["json"] = norm(view)
                else:
                    row["json"] = norm(body)
            else:
                data = response.get_data()
                row["mimetype"] = response.mimetype
                row["bytes"] = len(data)
                row["sha256"] = hashlib.sha256(data).hexdigest()
            out.append([index, row])
        return generated_ids(out)


# ----------------------------------------------------------- settings cases


def settings_case(case: dict) -> dict:
    """Run one settings case in a fresh interpreter so module-level settings reload."""
    # A fixed directory per case, so paths the settings derive from it are the same on every run.
    work = Path(OPTS.work) / ("settings-" + re.sub(r"[^A-Za-z0-9]+", "-", case["id"]).strip("-"))
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    for name, text in case.get("files", {}).items():
        (work / name).write_text(text)
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith("PYDATALAB_")}
    env["PYDATALAB_CONFIG_FILE"] = str(work / case.get("config_file", "config.json"))
    for k, v in case.get("env", {}).items():
        env[k] = v.replace("${WORK}", str(work))
    spec = work / "case.json"
    spec.write_text(json.dumps(case))
    proc = subprocess.run(
        [sys.executable, SELF, str(spec), str(work / "out.json"), "--files", OPTS.files, "--settings-case", "1"],
        cwd=work, env=env, capture_output=True, text=True, timeout=300,
    )
    if proc.returncode != 0:
        return {"error": "driver", "stderr_tail": proc.stderr[-400:]}
    return json.loads((work / "out.json").read_text())


def settings_child(case: dict) -> dict:
    import time

    started = time.time()
    try:
        from pydatalab.config import CONFIG
    except Exception:
        return {"observations": [{"load": "error"}]}
    norm = Normalizer(started, getattr(CONFIG, "IDENTIFIER_PREFIX", None) or "test")
    rows = []
    for name in case["observe"]:
        try:
            rows.append({name: norm(encode(getattr(CONFIG, name)))})
        except AttributeError:
            rows.append({name: "<absent>"})
    for update in case.get("updates", []):
        try:
            CONFIG.update(update)
            rows.append({"update": "ok", **{k.upper(): norm(encode(getattr(CONFIG, k.upper(), "<absent>"))) for k in update}})
        except Exception:
            rows.append({"update": "error"})
    return {"observations": rows}


# --------------------------------------------------------------------- probes


def model_classes() -> list[dict]:
    """Every class defined in a project module that carries model fields."""
    import inspect

    rows = []
    for module in list(sys.modules.values()):
        filename = getattr(module, "__file__", None)
        if not filename or os.path.realpath(filename) not in PROJECT_FILES:
            continue
        for name, obj in list(vars(module).items()):
            if not inspect.isclass(obj) or obj.__module__ != module.__name__:
                continue
            if not (hasattr(obj, "__fields__") or hasattr(obj, "model_fields")):
                continue
            rows.append({"class": f"{obj.__module__}.{obj.__qualname__}", "mro": [f"{c.__module__}.{c.__qualname__}" for c in obj.__mro__]})
    return rows


def default_path_probe(classes: list[dict]) -> dict:
    import importlib

    try:
        pydantic = importlib.import_module("pydantic")
        core = importlib.import_module("pydantic_core")
    except Exception as exc:
        return {"error": repr(exc)[:200]}
    bad = []
    for row in classes:
        module_name, _, qual = row["class"].rpartition(".")
        obj = sys.modules.get(module_name)
        for part in qual.split("."):
            obj = getattr(obj, part, None)
        if obj is None:
            continue
        ok = (
            isinstance(obj, type)
            and issubclass(obj, pydantic.BaseModel)
            and isinstance(getattr(obj, "__pydantic_validator__", None), core.SchemaValidator)
            and getattr(obj, "__pydantic_core_schema__", None) is not None
        )
        if not ok and not getattr(obj, "__pydantic_generic_metadata__", None):
            bad.append(row["class"])
    return {"version": getattr(pydantic, "VERSION", "?"), "not_v2": sorted(set(bad))}


def main():
    work = Path(OPTS.work or tempfile.mkdtemp(prefix="drive-"))
    work.mkdir(parents=True, exist_ok=True)
    OPTS.work = str(work)
    cases = json.loads(Path(OPTS.cases).read_text())
    if OPTS.settings_case:
        Path(OPTS.out).write_text(json.dumps(settings_child(cases)))
        return
    os.environ.setdefault("PYDATALAB_SECRET_KEY", hashlib.sha512(b"test").hexdigest())
    os.environ["PYDATALAB_CONFIG_FILE"] = str(work / "absent-config.json")
    os.chdir(work)
    result: dict = {"cases": {}, "start": None}
    try:
        runner = Runner(work)
        result["start"] = "ok"
    except Exception:
        result["start"] = "error"
        result["start_trace"] = traceback.format_exc()[-2000:]
        runner = None
    for case in cases:
        if case.get("kind") == "settings":
            result["cases"][case["id"]] = [[0, settings_case(case).get("observations")]]
            continue
        if runner is None:
            result["cases"][case["id"]] = None
            continue
        try:
            result["cases"][case["id"]] = runner.run_case(case)
        except Exception:
            result["cases"][case["id"]] = {"driver_error": traceback.format_exc()[-1200:]}
    classes = model_classes()
    result["probes"] = {
        "v1_imports": sorted(V1_IMPORTS),
        "deprecations": sorted(DEPRECATIONS),
        "model_classes": [row["class"] for row in classes],
        "default_path": default_path_probe(classes),
    }
    Path(OPTS.out).write_text(json.dumps(result, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
