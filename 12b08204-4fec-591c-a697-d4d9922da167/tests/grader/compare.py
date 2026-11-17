"""The Stage II and Stage III comparison profile: one HTTP exchange reduced to what is compared.

An exchange is compared on its status code; its Location as host-relative path and query; its
Content-Type media type and charset; the names and attributes of the cookies it sets (Path, HttpOnly,
SameSite, Secure, and whether Expires or Max-Age is present), never their values; and its body,
as canonical JSON (key order ignored, values exact), as Prometheus exposition (series names and
label sets, values dropped), or as normalised HTML or text (whitespace runs collapsed, CSRF tokens
masked). Identifiers and timestamps the case itself generates are mapped by first appearance, so two
runs agree when they generate the same structure. Server, Date, Content-Length, Connection,
Transfer-Encoding, Cache-Control, every other header and header order are ignored.

For a case recorded under the stated validation change only the status code, the envelope's status
and response.exception are compared.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from datetime import UTC, datetime
from email.utils import parsedate_to_datetime
from urllib.parse import urlsplit

UUID = re.compile(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b", re.I)
ISO = re.compile(
    r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:?\d{2})?(?![\d.])"
)
RFC = re.compile(
    r"\b(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun), \d{2} (?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)"
    r" \d{4} \d{2}:\d{2}:\d{2} (?:GMT|[+-]\d{4})"
)
EPOCH = re.compile(r"(?<![\w.])1[6-9]\d{8}(?:\d{3})?(?:\.\d+)?(?![\w.])")
TRACE = re.compile(r"(?<![A-Za-z0-9+/])[A-Za-z0-9+/]{43}=(?![A-Za-z0-9+/=])")
HEX_TOKEN = re.compile(r"(?<![0-9a-fA-F])[0-9a-f]{32,128}(?![0-9a-fA-F])")
B64_TOKEN = re.compile(r"(?<![A-Za-z0-9+/=])[A-Za-z0-9+][A-Za-z0-9+/]{38,}={0,2}(?![A-Za-z0-9+/=])")
CSRF = [
    re.compile(r'(name="csrf_token"[^>]*value=")[^"]*(")'),
    re.compile(r'(value=")[^"]*("[^>]*name="csrf_token")'),
    re.compile(r'(<meta name="csrf-token" content=")[^"]*(")'),
]
SPACE = re.compile(r"\s+")
SERIES = re.compile(r"^(?P<name>[a-zA-Z_:][a-zA-Z0-9_:]*)(?:\{(?P<labels>.*)\})?\s+\S+(?:\s+\S+)?$")
LABEL = re.compile(r'([a-zA-Z_][a-zA-Z0-9_]*)="((?:[^"\\]|\\.)*)"')
COOKIE_FLAGS = ("path", "httponly", "samesite", "secure")


@dataclass
class Window:
    """Timestamps inside [start, end] (epoch seconds) were generated while the case ran."""

    start: float
    end: float

    def generated(self, seconds: float) -> bool:
        return self.start <= seconds <= self.end


@dataclass
class Mapper:
    """First-appearance mapping of generated identifiers within one case."""

    known: frozenset[str]
    window: Window
    ids: dict[str, str] = field(default_factory=dict)

    def uuid(self, match: re.Match[str]) -> str:
        value = match.group(0).lower()
        if value in self.known:
            return value
        return self.ids.setdefault(value, f"<id{len(self.ids) + 1}>")

    def stamp(self, match: re.Match[str]) -> str:
        seconds = _seconds(match.group(0))
        if seconds is not None and self.window.generated(seconds):
            return "<now>"
        return match.group(0)

    def token(self, match: re.Match[str]) -> str:
        value = match.group(0)
        if value in self.known:
            return value
        if B64_TOKEN.fullmatch(value) and not HEX_TOKEN.fullmatch(value) and not (
            re.search(r"[0-9]", value) and re.search(r"[A-Z]", value) and re.search(r"[a-z]", value)
        ):
            return value
        return self.ids.setdefault(value, f"<tok{len(self.ids) + 1}>")

    def text(self, value: str) -> str:
        value = UUID.sub(self.uuid, value)
        value = TRACE.sub("<trace>", value)
        value = HEX_TOKEN.sub(self.token, value)
        value = B64_TOKEN.sub(self.token, value)
        value = ISO.sub(self.stamp, value)
        value = RFC.sub(self.stamp, value)
        value = EPOCH.sub(self.stamp, value)
        return TRACE.sub("<trace>", value)

    def number(self, value: float) -> object:
        for seconds in (value, value / 1000):
            if 1.6e9 <= seconds <= 2.2e9 and self.window.generated(seconds):
                return "<now>"
        return value


def _seconds(text: str) -> float | None:
    try:
        if text[:1].isdigit() and "-" not in text:
            number = float(text)
            return number / 1000 if number > 1e11 else number
        if "," in text:
            return parsedate_to_datetime(text).timestamp()
        moment = datetime.fromisoformat(text.replace("Z", "+00:00").replace(" ", "T"))
        if moment.tzinfo is None:
            moment = moment.replace(tzinfo=UTC)
        return moment.timestamp()
    except (ValueError, TypeError, OverflowError):
        return None


def _json(value: object, mapper: Mapper) -> object:
    if isinstance(value, dict):
        out: dict[str, object] = {}
        for key, item in value.items():
            mapped_key = mapper.text(key)
            out[mapped_key] = "<trace>" if key == "trace_id" else _json(item, mapper)
        return out
    if isinstance(value, list):
        return [_json(item, mapper) for item in value]
    if isinstance(value, str):
        return mapper.text(value)
    if isinstance(value, bool) or value is None:
        return value
    if isinstance(value, int | float):
        return mapper.number(value)
    return value


def _metrics(text: str) -> list[str]:
    rows: set[str] = set()
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        if line.startswith("#"):
            parts = line.split(None, 3)
            if len(parts) >= 3 and parts[1] in {"TYPE", "HELP"}:
                rows.add(" ".join(parts[:3]) + (" " + parts[3] if parts[1] == "TYPE" else ""))
            continue
        match = SERIES.match(line)
        if match is None:
            rows.add(f"unparsed {line}")
            continue
        labels = sorted(LABEL.findall(match["labels"] or ""))
        rows.add(match["name"] + "{" + ",".join(f"{k}={v}" for k, v in labels) + "}")
    return sorted(rows)


def _html(text: str, mapper: Mapper) -> str:
    for pattern in CSRF:
        text = pattern.sub(r"\1<csrf>\2", text)
    return SPACE.sub(" ", mapper.text(text)).strip()


def media(content_type: str) -> tuple[str, str]:
    parts = [part.strip() for part in content_type.split(";")]
    charset = ""
    for part in parts[1:]:
        if part.lower().startswith("charset="):
            charset = part.split("=", 1)[1].strip().strip('"').lower()
    return parts[0].lower(), charset


def cookies(set_cookie: list[str]) -> list[list[object]]:
    out = []
    for header in set_cookie:
        pieces = [piece.strip() for piece in header.split(";")]
        name = pieces[0].split("=", 1)[0]
        attrs: dict[str, object] = {}
        lifetime = False
        for piece in pieces[1:]:
            key, _, value = piece.partition("=")
            key = key.strip().lower()
            if key in {"expires", "max-age"}:
                lifetime = True
            elif key in COOKIE_FLAGS:
                attrs[key] = value.strip().lower() if key in {"path", "samesite"} else True
        attrs["lifetime"] = lifetime
        out.append([name, attrs])
    return sorted(out, key=lambda row: json.dumps(row, sort_keys=True))


def location(value: str | None, mapper: Mapper) -> str | None:
    if not value:
        return None
    parts = urlsplit(value)
    return mapper.text(parts.path + (f"?{parts.query}" if parts.query else ""))


def reduce(exchange: dict, mapper: Mapper, profile: str = "main") -> dict:
    """The compared form of one recorded exchange (status, headers, raw body as text)."""
    headers = {key.lower(): value for key, value in exchange["headers"]}
    content_type = headers.get("content-type", "")
    kind, charset = media(content_type)
    body = exchange["body"]
    if profile == "validation":
        try:
            parsed = json.loads(body)
        except ValueError:
            parsed = None
        envelope = parsed if isinstance(parsed, dict) else {}
        response = envelope.get("response")
        return {
            "status": exchange["status"],
            "envelope_status": envelope.get("status"),
            "exception": response.get("exception") if isinstance(response, dict) else None,
        }
    reduced: dict[str, object] = {
        "status": exchange["status"],
        "location": location(headers.get("location"), mapper),
        "media": kind,
        "charset": charset,
        "cookies": cookies([value for key, value in exchange["headers"] if key.lower() == "set-cookie"]),
    }
    if kind == "application/json" or (body[:1] in "{[" and kind in {"", "text/plain"}):
        try:
            reduced["json"] = _json(json.loads(body), mapper)
            return reduced
        except ValueError:
            pass
    if kind == "text/plain" and "# TYPE" in body:
        reduced["metrics"] = _metrics(body)
    elif kind.startswith("text/") or kind in {"application/xhtml+xml", ""}:
        reduced["text"] = _html(body, mapper)
    else:
        reduced["bytes_sha256"] = exchange.get("body_sha256")
    return reduced


def diff(expected: dict, actual: dict) -> str:
    """A short human reason naming the first compared field that differs, or empty."""
    for key in sorted(set(expected) | set(actual)):
        if expected.get(key) != actual.get(key):
            want = json.dumps(expected.get(key), sort_keys=True)[:300]
            got = json.dumps(actual.get(key), sort_keys=True)[:300]
            return f"{key}: expected {want} got {got}"
    return ""
