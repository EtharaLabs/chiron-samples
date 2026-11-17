"""Drive one case against a served application and record every exchange it makes.

A case is a list of steps. Each step names a client, so one case can hold an anonymous visitor and
a logged-in user side by side; each client keeps its own cookie jar for the case and starts empty.
Redirects are never followed: every hop is a step of its own, so a Location is compared as written.
The jar ignores cookie lifetimes except deletion, because the served clock is pinned and the
grader's is not.
"""

from __future__ import annotations

import hashlib
import http.client
import json
import re
import time
from email.utils import parsedate_to_datetime
from urllib.parse import urlencode

FIXED_HOST = "argus.test"


class Jar:
    def __init__(self) -> None:
        self.values: dict[str, str] = {}

    def header(self) -> str | None:
        if not self.values:
            return None
        return "; ".join(f"{name}={value}" for name, value in sorted(self.values.items()))

    def take(self, set_cookie: str, server_date: str | None) -> None:
        pieces = [piece.strip() for piece in set_cookie.split(";")]
        name, _, value = pieces[0].partition("=")
        deleted = False
        for piece in pieces[1:]:
            key, _, raw = piece.partition("=")
            key = key.strip().lower()
            if key == "max-age":
                try:
                    deleted = deleted or int(raw.strip()) <= 0
                except ValueError:
                    pass
            elif key == "expires":
                try:
                    expires = parsedate_to_datetime(raw.strip()).timestamp()
                    now = (
                        parsedate_to_datetime(server_date).timestamp()
                        if server_date
                        else time.time()
                    )
                    deleted = deleted or expires < now - 1
                except (TypeError, ValueError):
                    pass
        if deleted or value.strip('"') == "":
            self.values.pop(name, None)
        else:
            self.values[name] = value


def _body(step: dict) -> tuple[bytes | None, dict[str, str]]:
    headers: dict[str, str] = {}
    if "json" in step:
        headers["Content-Type"] = "application/json"
        return json.dumps(step["json"]).encode(), headers
    if "form" in step:
        headers["Content-Type"] = "application/x-www-form-urlencoded"
        return urlencode(step["form"], doseq=True).encode(), headers
    if "raw" in step:
        if step.get("content_type"):
            headers["Content-Type"] = step["content_type"]
        return step["raw"].encode(), headers
    return None, headers


def target(step: dict) -> str:
    path = step["path"]
    query = step.get("query")
    if query:
        if isinstance(query, list):  # repeated keys, kept in order: a list of [name, value] pairs
            query = [tuple(pair) for pair in query]
        path += ("&" if "?" in path else "?") + urlencode(query, doseq=True)
    return path


def send(port: int, step: dict, jar: Jar, timeout: float = 60.0) -> dict:
    body, headers = _body(step)
    headers["Host"] = FIXED_HOST
    headers.update(step.get("headers", {}))
    cookie = jar.header()
    if cookie and not step.get("no_cookies"):
        headers["Cookie"] = cookie
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
    started = time.time()
    try:
        connection.request(step.get("method", "GET"), target(step), body=body, headers=headers)
        response = connection.getresponse()
        raw = response.read()
        pairs = response.getheaders()
    finally:
        connection.close()
    date = response.getheader("Date")
    for key, value in pairs:
        if key.lower() == "set-cookie":
            jar.take(value, date)
    return {
        "status": response.status,
        "headers": [[key, value] for key, value in pairs],
        "body": raw.decode("utf-8", errors="replace"),
        "body_sha256": hashlib.sha256(raw).hexdigest(),
        "seconds": round(time.time() - started, 3),
    }


PLACEHOLDER = re.compile(r"\{\{([a-z0-9_]+)\}\}")


def _fill(value: object, captured: dict[str, str]) -> object:
    if isinstance(value, str):
        return PLACEHOLDER.sub(lambda m: captured.get(m.group(1), m.group(0)), value)
    if isinstance(value, list):
        return [_fill(item, captured) for item in value]
    if isinstance(value, dict):
        return {key: _fill(item, captured) for key, item in value.items()}
    return value


def _capture(exchange: dict, rules: dict[str, str], captured: dict[str, str]) -> None:
    """rules map a name to a dotted path into the JSON body, or to header:<name>."""
    try:
        body = json.loads(exchange["body"])
    except ValueError:
        body = None
    for name, path in rules.items():
        if path.startswith("header:"):
            wanted = path.split(":", 1)[1].lower()
            for key, value in exchange["headers"]:
                if key.lower() == wanted:
                    captured[name] = value
            continue
        node: object = body
        for part in path.split("."):
            if isinstance(node, dict):
                node = node.get(part)
            elif isinstance(node, list) and part.isdigit() and int(part) < len(node):
                node = node[int(part)]
            else:
                node = None
        if node is not None:
            captured[name] = str(node)


def run_case(port: int, case: dict) -> list[dict]:
    """Every exchange of one case, in order, each with the client and step index it came from."""
    jars: dict[str, Jar] = {}
    captured: dict[str, str] = {}
    out = []
    for index, raw_step in enumerate(case["steps"]):
        step = _fill({k: v for k, v in raw_step.items() if k != "capture"}, captured)
        jar = jars.setdefault(step.get("client", "anon"), Jar())
        try:
            exchange = send(port, step, jar)
        except (OSError, http.client.HTTPException) as exc:
            exchange = {"status": 0, "headers": [], "body": f"transport error: {exc}", "body_sha256": ""}
        exchange["step"] = index
        if raw_step.get("capture"):
            _capture(exchange, raw_step["capture"], captured)
        out.append(exchange)
    return out
