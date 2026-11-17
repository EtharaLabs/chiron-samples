"""Assemble the judged packets from a tree (q measurement, build revision 3).

app:   argus_backend.py, every non-test .py file importing the new stack, every non-test .py
       file defining a module-level name from the Flask surface list, and every non-test .py
       file importing such a name or such a module (the J0 selection).
tests: every test file of the tree, rendered as a diff against the State A test tree, with
       the submission's own line numbers on every kept and added line.

Each packet is split into parts under 900 kB so Reward Kit never skips one for size.
"""

from __future__ import annotations

import ast
import difflib
import re
import sys
from pathlib import Path

STACK = re.compile(
    r"^\s*(from|import)\s+(fastapi|starlette|uvicorn|gunicorn|jinja2|itsdangerous|werkzeug)\b", re.M
)
SURFACE = {
    "request", "g", "session", "current_app", "Blueprint", "jsonify", "make_response", "abort",
    "render_template", "render_template_string", "get_flashed_messages", "flash", "url_for",
    "redirect", "send_file", "send_from_directory", "stream_with_context", "after_this_request",
    "has_request_context", "has_app_context", "copy_current_request_context", "LocalProxy",
    "LocalStack",
}
SKIP = {".git", ".venv", "node_modules", "frontend", "__pycache__", "dev-db", "build", "dist"}
LIMIT = 900_000


def is_test(rel: Path) -> bool:
    return "tests" in rel.parts or rel.name.startswith("test_") or rel.name == "conftest.py"


def sources(root: Path):
    for p in sorted(root.rglob("*.py")):
        rel = p.relative_to(root)
        if any(part in SKIP or part.startswith(".") for part in rel.parts[:-1]):
            continue
        yield rel, p


def defines_surface(text: str) -> bool:
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return False
    for node in tree.body:
        names: list[str] = []
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            names = [node.name]
        elif isinstance(node, ast.Assign):
            names = [t.id for t in node.targets if isinstance(t, ast.Name)]
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            names = [node.target.id]
        if SURFACE.intersection(names):
            return True
    return False


def numbered(rel: Path, text: str) -> str:
    lines = text.splitlines()
    body = "\n".join(f"{i:5d}| {line}" for i, line in enumerate(lines, 1))
    return f"### FILE {rel.as_posix()} ({len(lines)} lines)\n{body}\n"


def module_name(rel: Path) -> str:
    parts = list(rel.with_suffix("").parts)
    if parts[-1] == "__init__":
        parts.pop()
    return ".".join(parts)


def imports_surface(rel: Path, text: str, surface_modules: set[str]) -> bool:
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return False
    package = module_name(rel).split(".")
    if rel.name != "__init__.py":
        package = package[:-1]
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom):
            base = package[: len(package) - node.level + 1] if node.level else []
            module = ".".join(base + ([node.module] if node.module else []))
            names = {alias.name for alias in node.names}
            if SURFACE & names or module in surface_modules:
                return True
            if any(f"{module}.{name}" in surface_modules for name in names):
                return True
        elif isinstance(node, ast.Import):
            if any(alias.name in surface_modules for alias in node.names):
                return True
    return False


def app_packet(root: Path) -> list[str]:
    files = [(rel, p.read_text(errors="replace")) for rel, p in sources(root) if not is_test(rel)]
    surface_modules = {module_name(rel) for rel, text in files if defines_surface(text)}
    chunks = []
    for rel, text in files:
        if (rel.as_posix() == "argus_backend.py" or STACK.search(text) or defines_surface(text)
                or imports_surface(rel, text, surface_modules)):
            chunks.append(numbered(rel, text))
    return chunks


def test_files(root: Path) -> dict[str, str]:
    return {rel.as_posix(): p.read_text(errors="replace") for rel, p in sources(root) if is_test(rel)}


def tests_packet(state_a: Path, root: Path) -> list[str]:
    old, new = test_files(state_a), test_files(root)
    chunks = []
    for name in sorted(set(old) | set(new)):
        a = old.get(name, "").splitlines()
        b = new.get(name, "").splitlines()
        if a == b:
            continue
        status = "added" if name not in old else "removed" if name not in new else "changed"
        out = [f"### TEST FILE {name} ({status}; '+NNNN' and ' NNNN' carry this tree's line numbers, '-A NNNN' carries State A's)"]
        for group in difflib.SequenceMatcher(None, a, b, autojunk=False).get_grouped_opcodes(3):
            first, last = group[0], group[-1]
            out.append(f"@@ State A {first[1] + 1}-{last[2]}, this tree {first[3] + 1}-{last[4]} @@")
            for op, x1, x2, y1, y2 in group:
                if op == "equal":
                    out.extend(f" {y1 + k + 1:5d}| {b[y1 + k]}" for k in range(y2 - y1))
                    continue
                if op in ("delete", "replace"):
                    out.extend(f"-A {x1 + k + 1:5d}| {a[x1 + k]}" for k in range(x2 - x1))
                if op in ("insert", "replace"):
                    out.extend(f"+{y1 + k + 1:5d}| {b[y1 + k]}" for k in range(y2 - y1))
        chunks.append("\n".join(out) + "\n")
    return chunks


def write(chunks: list[str], out: Path, stem: str) -> None:
    out.mkdir(parents=True, exist_ok=True)
    parts, cur = [], ""
    for c in chunks:
        if cur and len((cur + c).encode()) > LIMIT:
            parts.append(cur)
            cur = ""
        cur += c
    parts.append(cur)
    for n, text in enumerate(parts, 1):
        (out / f"{stem}-{n}.txt").write_text(text)
    print(stem, len(parts), "parts", sum(len(p.encode()) for p in parts), "bytes")


if __name__ == "__main__":
    kind, *args = sys.argv[1:]
    if kind == "app":
        write(app_packet(Path(args[0])), Path(args[1]), "app")
    else:
        write(tests_packet(Path(args[0]), Path(args[1])), Path(args[2]), "tests")
