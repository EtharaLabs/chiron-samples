"""Probes run with the submission's own interpreter, from the rebuilt tree, as the unprivileged user.

    probe.py routes <rules.json>   S5 and S10: the factory's application type and its route coverage
    probe.py server-config         K1 to K4: the configuration gunicorn loads for the graded launch

Each prints one JSON object on stdout.
"""

from __future__ import annotations

import json
import os
import re
import sys
import tempfile
from pathlib import Path

SAMPLES = {
    "uuid": "00000000-0000-4000-8000-000000000000",
    "int": "1",
    "float": "1.5",
    "path": "a/b",
    "string": "x",
    "default": "x",
}
CONVERTER = re.compile(r"<(?:(?P<conv>[a-z_]+)(?:\([^)]*\))?:)?(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)>")
GRADED_ARGV = ["--worker-class", "uvicorn.workers.UvicornWorker", "--workers", "1",
               "--bind", "127.0.0.1:8000", "argus_backend:create_app()"]


def sample(rule: str) -> str:
    return CONVERTER.sub(lambda m: SAMPLES.get(m["conv"] or "default", "x"), rule)


def walk(routes, prefix=""):
    from fastapi import FastAPI
    from fastapi.routing import APIRoute
    from starlette.applications import Starlette
    from starlette.routing import Mount, Router
    from starlette.staticfiles import StaticFiles

    try:  # FastAPI keeps an included router as one lazy route; expand it to the routes it serves
        from fastapi.routing import iter_route_contexts
        routes = [context.original_route for context in iter_route_contexts(list(routes))]
    except ImportError:
        routes = list(routes)
    for route in routes:
        if isinstance(route, APIRoute):
            yield ("api", prefix, route)
        elif isinstance(route, Mount):
            inner = route.app
            if isinstance(inner, StaticFiles):
                yield ("static", prefix, route)
            elif isinstance(inner, FastAPI | Starlette | Router):
                yield ("mount", prefix, route)
            else:
                yield ("foreign", prefix, route)
        else:
            yield ("other", prefix, route)


def routes(rules_path: str) -> dict:
    sys.path.insert(0, os.getcwd())
    from fastapi import FastAPI
    from starlette.routing import Match, Mount

    import argus_backend

    app = argus_backend.create_app()
    out: dict = {"type": f"{type(app).__module__}.{type(app).__qualname__}",
                 "fastapi": isinstance(app, FastAPI), "foreign_mounts": [], "unmatched": []}
    for kind, _, route in walk(app.routes):
        if kind == "foreign":
            out["foreign_mounts"].append(f"{route.path} -> {type(route.app).__module__}."
                                         f"{type(route.app).__qualname__}")
    rules = json.loads(Path(rules_path).read_text())

    def contexts(router_routes):
        """(route, effective path, methods) for every route, included routers expanded."""
        try:
            from fastapi.routing import iter_route_contexts
            return [(c.original_route, c.path, c.methods)
                    for c in iter_route_contexts(list(router_routes))]
        except ImportError:
            return [(r, getattr(r, "path", None), getattr(r, "methods", None)) for r in router_routes]

    def resolve(router_routes, path, method, depth=0):
        from fastapi.routing import APIRoute
        from starlette.routing import compile_path

        scope = {"type": "http", "path": path, "method": method, "root_path": "",
                 "path_params": {}, "query_string": b"", "headers": []}
        for route, full_path, methods in contexts(router_routes):
            if isinstance(route, Mount):
                match, child = route.matches(scope)
                if match == Match.NONE:
                    continue
                kind = next(k for k, _, r in walk([route]))
                if kind == "static":
                    return "static"
                if kind == "mount" and depth < 8:
                    rest = child.get("path", path)
                    root = child.get("root_path", "")
                    inner_path = path[len(root):] if root and path.startswith(root) else rest
                    found = resolve(getattr(route.app, "routes", []), inner_path or "/", method,
                                    depth + 1)
                    if found:
                        return found
                continue
            if not full_path:
                match, _ = route.matches(scope)
                if match == Match.FULL:
                    return "other"
                continue
            regex, _, _ = compile_path(full_path)
            if not regex.match(path):
                continue
            allowed = set(methods or ())
            if "GET" in allowed:
                allowed.add("HEAD")
            if allowed and method not in allowed:
                continue
            return "api" if isinstance(route, APIRoute) else "other"
        return ""

    for rule in rules:
        path = sample(rule["rule"])
        for method in rule["methods"]:
            how = resolve(app.routes, path, method)
            if how not in {"api", "static"}:
                out["unmatched"].append(f"{method} {rule['rule']} ({how or 'no route'})")
    return out


def server_config() -> dict:
    os.environ.pop("GUNICORN_CMD_ARGS", None)
    sys.path.insert(0, os.getcwd())
    from gunicorn.app.wsgiapp import WSGIApplication
    from gunicorn.glogging import Logger

    sys.argv = ["gunicorn", *GRADED_ARGV]
    application = WSGIApplication("%(prog)s [OPTIONS] [APP_MODULE]", prog="gunicorn")
    cfg = application.cfg
    out = {
        "max_requests": cfg.max_requests,
        "graceful_timeout": cfg.graceful_timeout,
        "worker_class": cfg.worker_class_str,
    }
    sys.argv = ["gunicorn", GRADED_ARGV[-1]]
    out["configured_worker_class"] = WSGIApplication(
        "%(prog)s [OPTIONS] [APP_MODULE]", prog="gunicorn"
    ).cfg.worker_class_str
    sys.argv = ["gunicorn", *GRADED_ARGV]
    out["exit_hooks"] = {}
    # The stand-ins' log has exactly the public methods of gunicorn's own Logger, as no-ops: a hook
    # that calls a method gunicorn's logger lacks fails here as it would crash gunicorn.
    gunicorn_log = type("Log", (), {name: staticmethod(lambda *args, **kwargs: None) for name in dir(Logger)
                                    if not name.startswith("_") and callable(getattr(Logger, name))})()
    for hook in ("child_exit", "worker_exit"):
        with tempfile.TemporaryDirectory() as directory:
            names = [f"gauge_livesum_{pid}.db" for pid in (4101, 4102)]
            names += [f"gauge_liveall_{pid}.db" for pid in (4101, 4102)]
            names += [f"counter_{pid}.db" for pid in (4101, 4102)]
            for name in names:
                Path(directory, name).write_bytes(b"")
            previous = os.environ.get("PROMETHEUS_MULTIPROC_DIR")
            os.environ["PROMETHEUS_MULTIPROC_DIR"] = directory

            class Worker:
                pid = 4101
                log = gunicorn_log

            class Server:
                cfg = application.cfg
                log = gunicorn_log

            error = ""
            try:
                getattr(cfg, hook)(Server(), Worker())
            except Exception as exc:  # a hook that raises cleans nothing
                error = f"{type(exc).__name__}: {exc}"
            if previous is None:
                os.environ.pop("PROMETHEUS_MULTIPROC_DIR", None)
            else:
                os.environ["PROMETHEUS_MULTIPROC_DIR"] = previous
            out["exit_hooks"][hook] = {"left": sorted(os.listdir(directory)), "error": error}
    return out


def main() -> None:
    verb = sys.argv[1]
    if verb == "routes":
        result = routes(sys.argv[2])
    elif verb == "server-config":
        result = server_config()
    else:
        raise SystemExit(f"unknown probe {verb}")
    sys.stdout.write("\nPROBE-RESULT " + json.dumps(result, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
