"""Replay one scenario (a Stage II group or a Stage III family) against a running application.

A scenario is JSON: {"id": ..., "steps": [step, ...]}. Step kinds:
  {"case": "<case-id>", "http": {"method", "path", "headers"?, "auth"?, "json"? | "body"?},
   "compare"?: "full" | "status" | "status+headers", "capture"?: {"var": "json:a.0.b" | "header:ETag"}}
  {"stub": {"method", "path", "json"?}}          grader-owned stub call, never recorded
  {"sql": "<statement>"}                           grader-owned database seed, never recorded
  {"restart": true} | {"restart": {"env": {...}}}  stop and start the application on the same
                                                   database, optionally with environment overrides
  {"reset": {"env": {...}}}                        stop the application, recreate the database and the
                                                   stub, and start it with the bound environment plus
                                                   these overrides (overrides of earlier steps dropped)
  {"sleep": seconds}                               wait for asynchronous work State A starts
${var} in a path, header value or JSON string is replaced by a value captured earlier in the
same scenario. Recording writes one transcript; comparison returns per-case agreement.
"""

from __future__ import annotations

import base64
import datetime
import json
import math
import re
import subprocess
import time
import urllib.error
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any

ADMIN = ("root", "shroot")
READ_ONLY = ("root2", "shroot2")
# Headers that belong to the recorded contract when State A sends them. Content-Type is
# compared by media type only; ETag by presence (its value is State A's per-process hashCode,
# so a case that needs it captures it and sends it back as If-None-Match).
CONTRACT_HEADERS = (
    "content-type", "location", "cache-control", "etag", "x-total-count",
    "access-control-allow-origin", "access-control-allow-methods",
    "access-control-allow-headers", "access-control-allow-credentials",
    "access-control-expose-headers", "access-control-max-age", "www-authenticate",
)
VAR = re.compile(r"\$\{([A-Za-z0-9_]+)\}")
# Contract replay_comparison.cache_control: whole seconds of slack on a clock-derived max-age.
CACHE_SLACK_SECONDS = 1  # bound by contract revision 4 (replay_comparison.cache_control.slack_seconds)


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: ANN001
        return None


OPENER = urllib.request.build_opener(_NoRedirect)


@dataclass
class Response:
    status: int
    headers: dict[str, str]
    body: str


def _subst(value: Any, env: dict[str, str]) -> Any:
    if isinstance(value, str):
        return VAR.sub(lambda m: env[m.group(1)], value)
    if isinstance(value, list):
        return [_subst(v, env) for v in value]
    if isinstance(value, dict):
        return {k: _subst(v, env) for k, v in value.items()}
    return value


def _auth_header(auth: Any) -> str | None:
    if auth is None:
        return None
    if auth == "admin":
        user, password = ADMIN
    elif auth == "readonly":
        user, password = READ_ONLY
    elif isinstance(auth, dict) and "raw" in auth:
        return auth["raw"]
    else:
        user, password = auth["user"], auth["password"]
    return "Basic " + base64.b64encode(f"{user}:{password}".encode()).decode()


def send(base: str, spec: dict[str, Any], timeout: float = 60.0) -> Response:
    url = base + spec["path"]
    headers = dict(spec.get("headers", {}))
    data = None
    if "json" in spec:
        data = json.dumps(spec["json"]).encode()
        headers.setdefault("Content-Type", "application/json")
    elif "body" in spec:
        data = spec["body"].encode()
    auth = _auth_header(spec.get("auth"))
    if auth:
        headers["Authorization"] = auth
    request = urllib.request.Request(url, data=data, method=spec["method"], headers=headers)
    try:
        with OPENER.open(request, timeout=timeout) as reply:
            status, raw, hdrs = reply.status, reply.read(), reply.headers
    except urllib.error.HTTPError as error:
        status, raw, hdrs = error.code, error.read(), error.headers
    folded: dict[str, str] = {}
    for name, value in hdrs.items():
        key = name.lower()
        folded[key] = f"{folded[key]}, {value}" if key in folded else value
    return Response(status, folded, raw.decode("utf-8", errors="replace"))


def _pick(resp: Response, selector: str) -> str:
    kind, _, path = selector.partition(":")
    if kind == "header":
        return resp.headers.get(path.lower(), "")
    node: Any = json.loads(resp.body)
    for part in [p for p in path.split(".") if p]:
        node = node[int(part)] if isinstance(node, list) else node[part]
    return node if isinstance(node, str) else json.dumps(node)


def _media(value: str) -> str:
    return value.split(";", 1)[0].strip().lower()


def _parse(body: str) -> Any:
    try:
        return ("json", json.loads(body))
    except ValueError:
        return ("text", body)


def _directives(value: str) -> dict[str, str | None]:
    out: dict[str, str | None] = {}
    for part in value.split(","):
        name, eq, arg = part.strip().partition("=")
        if name.strip():
            out[name.strip().lower()] = arg.strip().strip('"') if eq else None
    return out


def until_next_month(instant: float) -> int:
    """State A's DateTimeUtils.untilNextMonthUtc(SECONDS) evaluated at application-clock ``instant``."""
    now = datetime.datetime.fromtimestamp(instant, tz=datetime.timezone.utc)
    year, month = (now.year + 1, 1) if now.month == 12 else (now.year, now.month + 1)
    nxt = datetime.datetime(year, month, 1, tzinfo=datetime.timezone.utc).timestamp()
    return math.floor(nxt - instant)


def cache_control_problems(recorded: str | None, actual: str | None, rule: str | None,
                           window: tuple[float, float] | None) -> list[str]:
    if recorded is None and actual is None:
        return []
    if recorded is None or actual is None:
        return [f"cache-control {actual!r} != recorded {recorded!r}"]
    want, got = _directives(recorded), _directives(actual)
    if set(want) != set(got):
        return [f"cache-control directives {sorted(got)} != recorded {sorted(want)}"]
    problems = [f"cache-control {name}={got[name]!r} != recorded {want[name]!r}"
                for name in want if name != "max-age" and want[name] != got[name]]
    if "max-age" in want:
        if rule == "clock" and window is not None:
            low = until_next_month(window[1]) - CACHE_SLACK_SECONDS
            high = until_next_month(window[0]) + CACHE_SLACK_SECONDS
            try:
                ok = low <= int(got["max-age"] or "") <= high
            except ValueError:
                ok = False
            if not ok:
                problems.append(f"cache-control max-age {got['max-age']!r} outside State A's rule [{low}, {high}]")
        elif want["max-age"] != got["max-age"]:
            problems.append(f"cache-control max-age {got['max-age']!r} != recorded {want['max-age']!r}")
    return problems


def _compare(case: dict[str, Any], got: Response, window: tuple[float, float] | None = None) -> list[str]:
    want = case["response"]
    mode = case.get("compare", "full")
    problems: list[str] = []
    if got.status != want["status"]:
        problems.append(f"status {got.status} != recorded {want['status']}")
    for name in CONTRACT_HEADERS:
        recorded = want["headers"].get(name)
        actual = got.headers.get(name)
        if recorded is None and actual is None:
            continue
        if name == "etag":
            if (recorded is None) != (actual is None):
                problems.append(f"header etag presence differs (recorded {recorded is not None})")
            continue
        if name == "cache-control":
            problems += cache_control_problems(recorded, actual, case.get("cache_rule"), window)
            continue
        if name == "content-type":
            if recorded is None or actual is None or _media(recorded) != _media(actual):
                problems.append(f"content-type {actual!r} != recorded {recorded!r}")
            continue
        if recorded != actual:
            problems.append(f"header {name} {actual!r} != recorded {recorded!r}")
    if mode in ("status", "status+headers"):
        return problems
    if _parse(got.body) != _parse(want["body"]):
        problems.append(f"body differs: got {got.body[:300]!r} recorded {want['body'][:300]!r}")
    return problems


# Creating a user starts an asynchronous stats parse in State A (and in a faithful port). The replay
# waits this long after every successful user creation, at recording and at grading alike, so no read
# races that parse.
USER_CREATE_SETTLE_SECONDS = 2.0
USER_CREATE = re.compile(r"^/users(/batch)?/?$")


def _creates_users(spec: dict[str, Any], got: Response) -> bool:
    method, path = spec["method"].upper(), str(spec["path"]).split("?", 1)[0]
    return method == "POST" and bool(USER_CREATE.match(path)) and 200 <= got.status < 300


def run(
    scenario: dict[str, Any],
    *,
    base: str,
    stub: str,
    psql: Callable[[str], None],
    restart: Callable[[dict[str, str] | None], None],
    reset: Callable[[dict[str, str] | None], None] | None = None,
    clock_offset: Callable[[], float] = lambda: 0.0,
    recorded: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Replay ``scenario``; with ``recorded`` compare, else return a fresh transcript."""
    env: dict[str, str] = {}
    cases: list[dict[str, Any]] = []
    want = {c["case"]: c for c in (recorded or {}).get("cases", [])}
    for step in scenario["steps"]:
        if "stub" in step:
            send(stub, _subst(step["stub"], env))
            continue
        if "sql" in step:
            psql(_subst(step["sql"], env))
            continue
        if "reset" in step:
            if reset is None:
                raise RuntimeError("scenario resets but no reset is wired")
            reset((step["reset"] or {}).get("env"))
            continue
        if step.get("restart"):
            restart(step["restart"].get("env") if isinstance(step["restart"], dict) else None)
            continue
        if "sleep" in step:
            time.sleep(float(step["sleep"]))
            continue
        spec = _subst(step["http"], env)
        t_send = time.time()
        try:
            got = send(base, spec)
        except OSError as error:  # connection refused, reset or timed out
            got = Response(0, {}, f"transport error: {error}")
        offset = clock_offset()
        window = (t_send + offset, time.time() + offset)
        for var, selector in step.get("capture", {}).items():
            try:
                env[var] = _pick(got, selector)
            except (KeyError, IndexError, ValueError, TypeError):
                env[var] = ""
        row: dict[str, Any] = {
            "case": step["case"],
            "request": spec,
            "compare": step.get("compare", "full"),
            "response": {"status": got.status, "headers": got.headers, "body": got.body},
            "clock_window": list(window),
        }
        if recorded is not None:
            target = want.get(step["case"])
            row["problems"] = ["no recording for this case"] if target is None else _compare(target, got, window)
            row["agrees"] = not row["problems"]
        cases.append(row)
        if _creates_users(spec, got):
            time.sleep(USER_CREATE_SETTLE_SECONDS)
    result: dict[str, Any] = {"id": scenario["id"], "cases": cases}
    if recorded is not None:
        result["agrees"] = bool(cases) and all(c["agrees"] for c in cases) and len(cases) == len(want)
    return result


def psql_runner(dsn: str) -> Callable[[str], None]:
    def _run(statement: str) -> None:
        subprocess.run(["psql", dsn, "-v", "ON_ERROR_STOP=1", "-q", "-c", statement],
                       check=True, capture_output=True)
    return _run
