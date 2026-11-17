"""Grader-owned stand-ins for the LARS GPU API and the Folding@Home stats API.

Behaviour follows State A's stubbed-folding-endpoints-war (StubbedLarsEndpoint,
StubbedPointsEndpoint, StubbedUnitsEndpoint) at 03b26502. The /__stub/ control surface is
grader-only: it switches a failure mode on or off and is never reachable from the agent.
"""

from __future__ import annotations

import json
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, unquote, urlsplit

LARS_TEMPLATE = (
    "<tr>\n"
    '<td class="rank-num">${rank}</td>\n'
    "<td>\n"
    '<a title="${displayName} folding PPD information: ${modelInfo}" '
    'href="/gpu_ppd/brands/${manufacturer}/folding_profile/${modelInfo}">\n'
    '<span class="text-${manufacturer} model-name">${displayName}</span> <i class="uil-link mr-1"></i>\n'
    "</a>\n"
    "<br />\n"
    '<span class="model-info">${modelInfo}</span>\n'
    "</td>\n"
    "<td>${averagePpd}</td>\n"
    "<td>linuxAveragePpd_ignored</td>\n"
    "<td>windowsAveragePpd_ignored</td>\n"
    "<td>\n"
    '<a href="/gpu_ppd/brands/${manufacturer}">\n'
    '<span class="text-${manufacturer}">${manufacturer}</span> <i class="uil-link mr-1"></i>\n'
    "</a>\n"
    "</td>\n"
    '<td class="text-uppercase">model_ignored</td>\n'
    "</tr>"
)

LOCK = threading.Lock()
STATE: dict[str, object] = {"points": {}, "units": {}, "gpus": {}, "fail": ""}


def _key(user: str, passkey: str | None) -> str:
    # Java string concatenation of a null passkey yields "null".
    return user + ("null" if passkey is None else passkey)


def _java_long(text: str | None) -> int:
    if text is None:
        return 0
    return int(text)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt: str, *args: object) -> None:  # quiet
        return

    def _send(self, status: int, body: bytes = b"", ctype: str | None = None) -> None:
        self.send_response(status)
        if ctype:
            self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if body and self.command != "HEAD":
            self.wfile.write(body)

    def _body(self) -> bytes:
        length = int(self.headers.get("Content-Length") or 0)
        return self.rfile.read(length) if length else b""

    def _route(self) -> None:
        parts = urlsplit(self.path)
        path = unquote(parts.path)
        query = {k: v[0] for k, v in parse_qs(parts.query, keep_blank_values=True).items()}
        method = self.command
        body = self._body()

        if path.startswith("/__stub/"):
            with LOCK:
                if path == "/__stub/fail":
                    STATE["fail"] = query.get("mode", "")
                elif path == "/__stub/reset":
                    STATE.update(points={}, units={}, gpus={}, fail="")
            self._send(200)
            return

        fail = STATE["fail"]
        if fail == "500":
            self._send(500, b"stub failure", "text/plain")
            return
        if fail == "garbage":
            self._send(200, b"<<not json>>", "application/json")
            return

        segments = [s for s in path.split("/") if s]
        with LOCK:
            points: dict[str, int] = STATE["points"]  # type: ignore[assignment]
            units: dict[str, int] = STATE["units"]  # type: ignore[assignment]
            gpus: dict[str, dict] = STATE["gpus"]  # type: ignore[assignment]

            if segments[:1] == ["user"] and len(segments) == 3 and segments[2] == "stats":
                key = _key(segments[1], query.get("passkey"))
                if method == "GET":
                    out = json.dumps({"earned": points.get(key, 0)}, separators=(",", ":"))
                    self._send(200, out.encode(), "application/json")
                    return
                if method == "POST":
                    value = _java_long(query.get("points"))
                    points[key] = value if value == 0 else points.get(key, 0) + value
                    self._send(200)
                    return
            if segments == ["user"] and method == "DELETE":
                points.clear()
                self._send(200)
                return
            if segments == ["bonus"]:
                if method == "GET":
                    key = _key(query.get("user", "null"), query.get("passkey"))
                    rows = [{"finished": units[key]}] if key in units else []
                    self._send(200, json.dumps(rows, separators=(",", ":")).encode(), "application/json")
                    return
                if method == "POST":
                    key = _key(query.get("user", "null"), query.get("passkey"))
                    value = int(query.get("units") or 0)
                    units[key] = value if value == 0 else units.get(key, 0) + value
                    self._send(200)
                    return
                if method == "DELETE":
                    units.clear()
                    self._send(200)
                    return
            if segments == ["gpu_ppd", "overall_ranks"]:
                if method == "POST":
                    for gpu in json.loads(body or b"[]"):
                        gpus[gpu["modelInfo"]] = gpu
                    self._send(201)
                    return
                if method == "DELETE":
                    gpus.clear()
                    self._send(200)
                    return
                if method == "GET":
                    page = ['<html><table id="primary-datatable"<tbody>']
                    for gpu in gpus.values():
                        row = LARS_TEMPLATE
                        for name in ("displayName", "manufacturer", "modelInfo", "rank", "averagePpd"):
                            row = row.replace("${" + name + "}", str(gpu.get(name, "")))
                        page.append(row)
                    page.append("</tbody></table></html>")
                    self._send(200, "".join(page).encode(), "text/plain")
                    return
        self._send(404)

    do_GET = do_POST = do_DELETE = do_PUT = do_PATCH = do_HEAD = _route


def main() -> None:
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8082
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    server.serve_forever()


if __name__ == "__main__":
    main()
