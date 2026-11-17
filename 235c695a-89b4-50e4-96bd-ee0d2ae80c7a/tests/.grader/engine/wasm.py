"""Facts about a WebAssembly module that its build options decide: imports, exports, memory,
the initial stack pointer and which custom sections survived the link."""

from __future__ import annotations

from pathlib import Path

KINDS = {0: "func", 1: "table", 2: "memory", 3: "global", 4: "tag"}


class Reader:
    def __init__(self, data: bytes, pos: int = 0) -> None:
        self.data = data
        self.pos = pos

    def byte(self) -> int:
        value = self.data[self.pos]
        self.pos += 1
        return value

    def uleb(self) -> int:
        result = shift = 0
        while True:
            b = self.byte()
            result |= (b & 0x7F) << shift
            shift += 7
            if b < 0x80:
                return result

    def sleb(self) -> int:
        result = shift = 0
        while True:
            b = self.byte()
            result |= (b & 0x7F) << shift
            shift += 7
            if b < 0x80:
                if b & 0x40:
                    result -= 1 << shift
                return result

    def name(self) -> str:
        size = self.uleb()
        text = self.data[self.pos:self.pos + size].decode("utf-8", "replace")
        self.pos += size
        return text

    def limits(self) -> str:
        flag = self.byte()
        low = self.uleb()
        return f"{low}..{self.uleb()}" if flag & 1 else f"{low}.."


def facts(path: Path) -> dict[str, str]:
    data = path.read_bytes()
    if data[:4] != b"\0asm":
        return {"magic": "not-wasm"}
    pos = 8
    imports, exports, custom, memory, globals_ = [], [], [], [], []
    while pos < len(data):
        sid = data[pos]
        r = Reader(data, pos + 1)
        size = r.uleb()
        start, end = r.pos, r.pos + size
        body = Reader(data, start)
        try:
            if sid == 0:
                custom.append(body.name())
            elif sid == 2:
                for _ in range(body.uleb()):
                    module, field, kind = body.name(), body.name(), body.byte()
                    imports.append(f"{module}.{field}:{KINDS.get(kind, kind)}")
                    if kind == 0:
                        body.uleb()
                    elif kind == 1:
                        body.byte()
                        body.limits()
                    elif kind == 2:
                        body.limits()
                    elif kind == 3:
                        body.byte()
                        body.byte()
                    elif kind == 4:
                        body.byte()
                        body.uleb()
            elif sid == 5:
                for _ in range(body.uleb()):
                    memory.append(body.limits())
            elif sid == 6:
                for _ in range(body.uleb()):
                    vtype, mut = body.byte(), body.byte()
                    op = body.byte()
                    value = body.sleb() if op in (0x41, 0x42) else f"op{op:#x}"
                    while body.byte() != 0x0B:
                        pass
                    globals_.append(f"{vtype:#x}/{mut}={value}")
            elif sid == 7:
                for _ in range(body.uleb()):
                    field, kind = body.name(), body.byte()
                    body.uleb()
                    exports.append(f"{field}:{KINDS.get(kind, kind)}")
        except IndexError:
            return {"magic": "truncated"}
        pos = end
    return {
        "imports": "\n".join(sorted(imports)),
        "exports": "\n".join(sorted(exports)),
        "memory": " ".join(memory),
        "stack-pointer": globals_[0] if globals_ else "none",
        "custom-sections": " ".join(sorted(set(custom))),
    }
