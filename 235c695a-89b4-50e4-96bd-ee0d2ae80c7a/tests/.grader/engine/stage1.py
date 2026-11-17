"""Stage I static checks: the migration happened and is not faked.

Each check returns (passed, evidence). Evidence names a path and what fired, so a reviewer can
confirm a failure without rerunning anything.
"""

from __future__ import annotations

import ast
import base64
import bz2
import gzip
import io
import lzma
import re
import tarfile
import zipfile
from pathlib import Path

from .util import read_text, sha256_bytes

# Directories whose Makefiles are statement-named harness or third-party files, never the build.
MAKE_EXEMPT_PREFIXES = (
    "abc/", "tests/", "docs/", "examples/", "libs/bigint/", "libs/ezsat/", "libs/subcircuit/",
    "libs/cxxopts/",
)
SKIP_SCAN_PREFIXES = ("abc/", "libs/cxxopts/")

MAKE_CMD = re.compile(
    r"(?:^|[\s;&|(`'\"=])(?:sudo\s+|time\s+|nice\s+|exec\s+|env\s+(?:[A-Za-z_][A-Za-z0-9_]*=\S*\s+)*)*"
    r"(?:[A-Za-z_][A-Za-z0-9_]*=\S*\s+)*(?:/usr/bin/|/bin/)?(g?make|\$\(MAKE\)|\$\{MAKE\}|\$MAKE)"
    r"(?=$|\s|;|\)|`|'|\")"
)
HARNESS_TARGET = re.compile(r"(?:-C|--directory(?:=|\s+))\s*(?:\./)?(?:tests|docs)\b")


def make_commands(text: str) -> list[str]:
    """Shell command lines in text that invoke make outside the tests/ and docs/ harnesses."""
    hits = []
    for raw in text.split("\n"):
        line = raw.split(" #", 1)[0].strip()
        if not line or line.startswith("#"):
            continue
        for match in MAKE_CMD.finditer(line):
            tail = line[match.end():]
            if HARNESS_TARGET.search(tail.split("&&")[0].split(";")[0]):
                continue
            hits.append(line)
            break
    return hits


WRAPPERS = {"sudo", "env", "time", "nice", "exec", "command", "nohup"}
WORKSPACE = ("${{ github.workspace }}", "${GITHUB_WORKSPACE}", "$GITHUB_WORKSPACE")
HERE = ("$(pwd)", "$PWD", "${PWD}", "`pwd`")


def _shell_tokens(text: str) -> list[tuple[str, str]]:
    """Words and control operators of a shell script, without comments.

    Quotes are removed; `$(...)`, `${...}` and backquotes stay inside the word they belong to.
    """
    tokens: list[tuple[str, str]] = []
    word: list[str] = []
    have = False
    i, n = 0, len(text)

    def flush() -> None:
        nonlocal word, have
        if have:
            tokens.append(("w", "".join(word)))
        word, have = [], False

    while i < n:
        c = text[i]
        if c == "\\" and i + 1 < n:
            if text[i + 1] != "\n":
                word.append(text[i + 1])
                have = True
            i += 2
            continue
        if c == "'":
            j = text.find("'", i + 1)
            j = n if j < 0 else j
            word.append(text[i + 1:j])
            have, i = True, j + 1
            continue
        if c == '"':
            j, buf = i + 1, []
            while j < n and text[j] != '"':
                if text[j] == "\\" and j + 1 < n:
                    buf.append(text[j + 1])
                    j += 2
                    continue
                buf.append(text[j])
                j += 1
            word.append("".join(buf))
            have, i = True, j + 1
            continue
        if c == "$" and text.startswith("$(", i):
            depth, j = 0, i + 1
            while j < n:
                if text[j] == "(":
                    depth += 1
                elif text[j] == ")":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            word.append(text[i:j + 1])
            have, i = True, j + 1
            continue
        if c == "$" and text.startswith("${", i):
            j = text.find("}", i)
            j = n - 1 if j < 0 else j
            word.append(text[i:j + 1])
            have, i = True, j + 1
            continue
        if c == "`":
            j = text.find("`", i + 1)
            j = n if j < 0 else j
            word.append(text[i:j + 1])
            have, i = True, j + 1
            continue
        if c == "#" and not have:
            j = text.find("\n", i)
            i = n if j < 0 else j
            continue
        if c in " \t\r":
            flush()
            i += 1
            continue
        if text.startswith(("&&", "||"), i):
            flush()
            tokens.append(("o", text[i:i + 2]))
            i += 2
            continue
        if c in "\n;|&()":
            flush()
            tokens.append(("o", c))
            i += 1
            continue
        word.append(c)
        have = True
        i += 1
    flush()
    return tokens


def _join(base: str | None, rel: str) -> str | None:
    if base is None:
        return None
    parts = [part for part in base.split("/") if part]
    for part in rel.split("/"):
        if part in ("", "."):
            continue
        if part == "..":
            if not parts:
                return None
            parts.pop()
        else:
            parts.append(part)
    return "/".join(parts)


def _resolve(cwd: str | None, target: str | None) -> str | None:
    """Directory relative to the repository root, or None when unknown or outside it."""
    if cwd is None or target is None:
        return None
    for root in WORKSPACE:
        if target == root:
            return ""
        if target.startswith(root + "/"):
            return _join("", target[len(root) + 1:])
    for here in HERE:
        if target == here:
            return cwd
        if target.startswith(here + "/"):
            return _join(cwd, target[len(here) + 1:])
    if "$" in target or "`" in target or target.startswith(("/", "~", "-")):
        return None
    return _join(cwd, target)


def make_runs(text: str, start: str | None = "") -> list[tuple[str | None, list[str], str]]:
    """(directory, makefiles, command) for each make invocation in a shell script.

    The directory follows `cd` and `-C` relative to the repository root (None once it leaves the
    repository or depends on a variable); subshells restore it.
    """
    runs: list[tuple[str | None, list[str], str]] = []
    cwd: str | None = start
    scopes: list[str | None] = []
    argv: list[str] = []

    def finish() -> None:
        nonlocal cwd, argv
        words = list(argv)
        argv = []
        while words and (re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", words[0]) or words[0] in WRAPPERS):
            words.pop(0)
        if not words:
            return
        head = words[0]
        if head in ("cd", "pushd"):
            cwd = _resolve(cwd, words[1] if len(words) > 1 else None)
            return
        if head.rsplit("/", 1)[-1] not in ("make", "gmake") and head not in ("$(MAKE)", "${MAKE}", "$MAKE"):
            return
        where, files, k = cwd, [], 1
        while k < len(words):
            word = words[k]
            if word in ("-C", "--directory") and k + 1 < len(words):
                where, k = _resolve(where, words[k + 1]), k + 2
                continue
            if word.startswith("--directory="):
                where = _resolve(where, word.split("=", 1)[1])
            elif word.startswith("-C") and len(word) > 2:
                where = _resolve(where, word[2:])
            elif word in ("-f", "--file", "--makefile") and k + 1 < len(words):
                files.append(words[k + 1])
                k += 2
                continue
            elif word.startswith(("--file=", "--makefile=")):
                files.append(word.split("=", 1)[1])
            k += 1
        runs.append((where, files, " ".join(words)))

    for kind, value in _shell_tokens(text):
        if kind == "w":
            argv.append(value)
            continue
        finish()
        if value == "(":
            scopes.append(cwd)
        elif value == ")" and scopes:
            cwd = scopes.pop()
    finish()
    return runs


def runs_old_build(where: str | None, files: list[str]) -> bool:
    """A make run that drives the repository's own top-level or ABC makefiles."""
    if files:
        for name in files:
            path = _resolve(where, name)
            if path is not None and ("/" not in path or path.startswith("abc/")):
                return True
        return False
    return where in ("", "abc")


def workflow_runs(text: str) -> list[tuple[str, str]]:
    """(working-directory, command text) for every `run:` value in a workflow or action file."""
    lines = text.split("\n")
    steps: list[tuple[str, str]] = []
    i = 0
    while i < len(lines):
        match = re.match(r"^(\s*)(-\s+)?run:\s*(.*)$", lines[i])
        if not match:
            i += 1
            continue
        indent = len(match.group(1)) + (len(match.group(2)) if match.group(2) else 0)
        value = match.group(3).strip()
        body = []
        if value[:1] in ("|", ">"):
            j = i + 1
            while j < len(lines) and (not lines[j].strip() or len(lines[j]) - len(lines[j].lstrip()) > indent):
                body.append(lines[j].strip())
                j += 1
            i = j
        else:
            body.append(value.strip("'\""))
            i += 1
        workdir = ""
        lo = max(0, i - 40)
        window = "\n".join(lines[lo:min(len(lines), i + 10)])
        wd = re.findall(r"working-directory:\s*(\S+)", window)
        if wd:
            workdir = wd[-1].strip("'\"")
        steps.append((workdir, "\n".join(body)))
    return steps


def make_build_absent(tree: Path, files: list[str]) -> tuple[bool, list[str]]:
    evidence = []
    for rel in files:
        name = rel.rsplit("/", 1)[-1]
        if rel in ("Makefile", "makefile", "GNUmakefile", "Makefile.conf"):
            evidence.append(f"{rel}: the top-level GNU Make build file is still present")
            continue
        if rel.startswith(SKIP_SCAN_PREFIXES):
            continue
        if name in ("Makefile.inc", "Makefile.conf"):
            evidence.append(f"{rel}: a GNU Make build fragment is still present")
        elif (name in ("Makefile", "makefile", "GNUmakefile") or name.endswith(".mk")) and not rel.startswith(MAKE_EXEMPT_PREFIXES):
            evidence.append(f"{rel}: a Makefile outside the tests/, docs/ and examples/ harnesses")
    for rel in files:
        if not (rel.startswith(".github/workflows/") or (rel.startswith(".github/actions/") and rel.endswith((".yml", ".yaml")))):
            continue
        text = read_text(tree / rel) or ""
        for workdir, command in workflow_runs(text):
            start = _resolve("", workdir) if workdir else ""
            for where, makefiles, line in make_runs(command, start):
                if runs_old_build(where, makefiles):
                    evidence.append(f"{rel}: workflow step runs make on the repository's own makefiles: {line[:160]}")
    packaging = _packaging_scripts(tree, files)
    for rel in files:
        if rel == "flake.nix" or (rel.startswith("nix/") and rel.endswith(".nix")) or rel == "Brewfile" or rel in packaging:
            text = read_text(tree / rel) or ""
            for line in make_commands(text):
                evidence.append(f"{rel}: build recipe still runs make: {line[:160]}")
            if rel in packaging and PY_MAKE.search(re.sub(r"(?m)^\s*#.*$", "", text)) and not make_commands(text):
                evidence.append(f"{rel}: the packaging route still runs make")
    return (not evidence, evidence)


def _make_lines(statea: Path, make_files: list[str]) -> set[str]:
    lines: set[str] = set()
    for rel in make_files:
        text = read_text(statea / rel) or ""
        for raw in text.split("\n"):
            line = " ".join(raw.split())
            if len(line) < 12 or line.startswith("#"):
                continue
            if "$(" in line or ":=" in line or "+=" in line or re.match(r"^[\w./$()%-]+\s*:", line):
                lines.add(line)
    return lines


MAGIC = [
    (b"\x7fELF", "an ELF object"),
    (b"!<arch>\n", "an ar archive of objects"),
    (b"\0asm", "a WebAssembly module"),
    (b"\xcf\xfa\xed\xfe", "a Mach-O object"),
    (b"\xce\xfa\xed\xfe", "a Mach-O object"),
    (b"\xca\xfe\xba\xbe", "a fat Mach-O or class file"),
]
GENERATED = [
    (re.compile(r"A Bison parser, made by GNU Bison"), "a Bison-generated parser"),
    (re.compile(r"A lexical scanner generated by flex"), "a flex-generated scanner"),
    (re.compile(r"yosys_version_str\s*=\s*\"Yosys\s+\d"), "a generated version source"),
]


def _unpack(data: bytes, depth: int = 0) -> list[tuple[str, bytes]]:
    """Members of compressed or archived data, recursively, bounded in size and depth."""
    if depth > 3 or len(data) > 64 << 20:
        return []
    out: list[tuple[str, bytes]] = []
    try:
        if data[:2] == b"\x1f\x8b":
            out.append(("gunzip", gzip.decompress(data)))
        elif data[:3] == b"BZh":
            out.append(("bunzip2", bz2.decompress(data)))
        elif data[:6] == b"\xfd7zXZ\x00":
            out.append(("unxz", lzma.decompress(data)))
        elif data[:4] == b"PK\x03\x04":
            with zipfile.ZipFile(io.BytesIO(data)) as archive:
                for info in archive.infolist()[:5000]:
                    if not info.is_dir():
                        out.append((info.filename, archive.read(info)))
        elif len(data) > 512 and data[257:262] == b"ustar":
            with tarfile.open(fileobj=io.BytesIO(data)) as archive:
                for member in archive.getmembers()[:5000]:
                    handle = archive.extractfile(member) if member.isfile() else None
                    if handle:
                        out.append((member.name, handle.read()))
    except (OSError, EOFError, lzma.LZMAError, zipfile.BadZipFile, tarfile.TarError, ValueError):
        return []
    nested = []
    for name, member in out:
        nested.append((name, member))
        for inner, blob in _unpack(member, depth + 1):
            nested.append((f"{name}/{inner}", blob))
    return nested


B64 = re.compile(rb"[A-Za-z0-9+/=\n\r]{2048,}")


def no_embedded_old_build(
    tree: Path, changed: list[str], statea: Path, make_files: list[str], generated: dict[str, str]
) -> tuple[bool, list[str]]:
    lines = _make_lines(statea, make_files)
    gen_names = {rel.rsplit("/", 1)[-1] for rel in generated}
    gen_digests = set(generated.values())
    evidence = []

    def scan(label: str, data: bytes) -> None:
        for magic, what in MAGIC:
            if data.startswith(magic):
                evidence.append(f"{label}: {what} that State A does not carry")
                return
        if b"\0" in data[:8192]:
            return
        text = data.decode("utf-8", "surrogateescape")
        found = {line for line in (" ".join(raw.split()) for raw in text.split("\n")) if line in lines}
        if len(found) >= 25:
            evidence.append(f"{label}: carries {len(found)} lines of the State A GNU Make build")
        for rule, what in GENERATED:
            if rule.search(text):
                evidence.append(f"{label}: {what}")
                break

    for rel in changed:
        path = tree / rel
        if path.is_symlink() or not path.is_file():
            continue
        try:
            data = path.read_bytes()
        except OSError:
            continue
        name = rel.rsplit("/", 1)[-1]
        if re.fullmatch(r"version_[0-9a-f]+\.cc", name) or (name in gen_names and not (statea / rel).exists()):
            evidence.append(f"{rel}: a file the State A build generates is committed as source")
        if sha256_bytes(data) in gen_digests:
            evidence.append(f"{rel}: byte-identical to a file the State A build generates")
        scan(rel, data)
        for inner, blob in _unpack(data):
            scan(f"{rel}!{inner}", blob)
        for match in B64.finditer(data):
            try:
                blob = base64.b64decode(re.sub(rb"\s", b"", match.group(0)), validate=False)
            except ValueError:
                continue
            scan(f"{rel}!base64@{match.start()}", blob)
            for inner, member in _unpack(blob):
                scan(f"{rel}!base64@{match.start()}!{inner}", member)
    return (not evidence, sorted(set(evidence)))


CMAKE_FILE = re.compile(r"(^|/)(CMakeLists\.txt|[^/]+\.cmake|CMakePresets\.json|CMakeUserPresets\.json)$")
SCRIPT_FILE = re.compile(r"\.(py|sh|bash|pl|rb|js|nix|toml|cfg)$")


def _cmake_blocks(text: str) -> list[str]:
    """Each top-level command call `name(...)` in CMake text, with balanced parentheses."""
    blocks = []
    clean = re.sub(r"#\[\[.*?\]\]|#[^\n]*", "", text, flags=re.S)
    for match in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(", clean):
        depth, j = 0, match.end() - 1
        while j < len(clean):
            if clean[j] == "(":
                depth += 1
            elif clean[j] == ")":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        blocks.append(clean[match.start():j + 1])
    return blocks


def _build_scripts(tree: Path, files: list[str]) -> set[str]:
    """Scripts the CMake build runs: those a CMake file names."""
    cmake_text = "\n".join(read_text(tree / rel) or "" for rel in files if CMAKE_FILE.search(rel))
    return {rel for rel in files if SCRIPT_FILE.search(rel) and rel.rsplit("/", 1)[-1] in cmake_text}


def _packaging_scripts(tree: Path, files: list[str]) -> set[str]:
    """The Python packaging route: setup.py and the pyproject build backend module."""
    scripts = {"setup.py"} & set(files)
    project = read_text(tree / "pyproject.toml") or ""
    backend = re.search(r"""^\s*build-backend\s*=\s*["']([A-Za-z0-9_.]+)""", project, re.M)
    paths = re.search(r"""^\s*backend-path\s*=\s*\[([^\]]*)\]""", project, re.M)
    if backend:
        module = backend.group(1).split(".")[0]
        for base in re.findall(r"""["']([^"']+)["']""", paths.group(1)) if paths else [""]:
            for rel in (f"{base}/{module}.py".lstrip("/"), f"{base}/{module}/__init__.py".lstrip("/")):
                if rel in files:
                    scripts.add(rel)
    return scripts


PY_MAKE = re.compile(r"""["'](?:g?make)["']\s*[,\]]""")
MAKEFILE_FLAGS = ("-f", "--file", "--makefile")


def _harness_dir(path: str) -> bool:
    """A make directory that is the tests/ or docs/ harness: its last component, never through `..`."""
    parts = [p for p in path.strip("\"'").split("/") if p not in ("", ".")]
    return bool(parts) and parts[-1] in ("tests", "docs") and ".." not in parts


def _strings(node: ast.AST | None, assigned: dict[str, ast.AST], depth: int = 3) -> list[str]:
    """String literals of an expression in source order, following simple name assignments."""
    if isinstance(node, ast.Constant):
        return [node.value] if isinstance(node.value, str) else []
    if isinstance(node, ast.Name):
        value = assigned.get(node.id)
        return _strings(value, assigned, depth - 1) if value is not None and depth > 0 else []
    return [s for child in ast.iter_child_nodes(node) for s in _strings(child, assigned, depth)] if node else []


def _python_harness_only(text: str) -> bool:
    """True when every make argv a Python build script spells runs the tests/ or docs/ harness.

    A script that does not parse, or names make anywhere but as the first word of an argv list,
    is never cleared.
    """
    try:
        module = ast.parse(text)
    except (SyntaxError, ValueError):
        return False
    nodes = list(ast.walk(module))
    assigned = {n.targets[0].id: n.value for n in nodes
                if isinstance(n, ast.Assign) and len(n.targets) == 1 and isinstance(n.targets[0], ast.Name)}
    cwd = {id(n.args[0]): kw.value for n in nodes if isinstance(n, ast.Call) and n.args
           for kw in n.keywords if kw.arg == "cwd"}
    argvs = [n for n in nodes if isinstance(n, (ast.List, ast.Tuple)) and n.elts
             and isinstance(n.elts[0], ast.Constant) and n.elts[0].value in ("make", "gmake")]
    if len(argvs) != len(PY_MAKE.findall(re.sub(r"(?m)^\s*#.*$", "", text))):
        return False
    for argv in argvs:
        rest = argv.elts[1:]
        words = [e.value if isinstance(e, ast.Constant) and isinstance(e.value, str) else None for e in rest]
        if any(w is not None and w.startswith(MAKEFILE_FLAGS) for w in words):
            return False
        dirs = ["/".join(_strings(rest[k + 1], assigned)) for k, w in enumerate(words[:-1]) if w in ("-C", "--directory")]
        dirs += [w[2:] for w in words if w is not None and w.startswith("-C") and len(w) > 2]
        dirs += [w.split("=", 1)[1] for w in words if w is not None and w.startswith("--directory=")]
        if id(argv) in cwd:
            dirs.append("/".join(_strings(cwd[id(argv)], assigned)))
        if not dirs or not all(_harness_dir(d) for d in dirs):
            return False
    return True


def forwarding_static(tree: Path, files: list[str]) -> list[str]:
    evidence = []
    for rel in files:
        if rel.startswith(SKIP_SCAN_PREFIXES) or rel.startswith(("tests/", "docs/", "examples/")):
            continue
        if not CMAKE_FILE.search(rel):
            continue
        text = read_text(tree / rel) or ""
        for block in _cmake_blocks(text):
            head = block.split("(", 1)[0].strip().lower()
            if head in ("externalproject_add", "fetchcontent_declare", "fetchcontent_populate"):
                evidence.append(f"{rel}: {head} pulls in a sub-build or download")
                continue
            if head not in ("execute_process", "add_custom_command", "add_custom_target"):
                continue
            if re.search(r"WORKING_DIRECTORY\s+\"?\$\{(?:CMAKE_SOURCE_DIR|CMAKE_CURRENT_SOURCE_DIR|PROJECT_SOURCE_DIR)\}/(?:tests|docs)\b", block):
                continue
            for cmd in re.findall(r"COMMAND\s+([^\s)]+)", block):
                word = cmd.strip("\"'").rsplit("/", 1)[-1]
                if word in ("make", "gmake", "${CMAKE_MAKE_PROGRAM}", "$(MAKE)"):
                    evidence.append(f"{rel}: {head} runs {word}")
    for rel in sorted(_build_scripts(tree, files)):
        if rel.startswith(SKIP_SCAN_PREFIXES) or rel.startswith(("tests/", "docs/", "examples/")):
            continue
        text = read_text(tree / rel) or ""
        if rel.endswith(".py"):
            if PY_MAKE.search(re.sub(r"(?m)^\s*#.*$", "", text)) and not _python_harness_only(text):
                evidence.append(f"{rel}: a script the build runs invokes make")
        elif rel.endswith((".sh", ".bash")):
            for where, _files, line in make_runs(text):
                if where is None or not where.startswith(("tests", "docs")):
                    evidence.append(f"{rel}: a script the build runs invokes make: {line[:160]}")
    return evidence


NATIVE_LINK = re.compile(r"^(C|CXX)_(EXECUTABLE_LINKER|SHARED_LIBRARY_LINKER|STATIC_LIBRARY_LINKER|MODULE_LIBRARY_LINKER)__")
NATIVE_COMPILE = re.compile(r"^(C|CXX)_COMPILER__")


def ninja_products(build_ninja: Path) -> dict[str, tuple[str, list[str]]]:
    """output -> (rule, explicit and implicit inputs) for every build statement in a Ninja file.

    CMake lists the libraries an executable links as implicit inputs, so they are kept;
    order-only inputs are not.
    """
    text = read_text(build_ninja, limit=400_000_000) or ""
    text = text.replace("$\n", " ")
    graph: dict[str, tuple[str, list[str]]] = {}
    for line in text.split("\n"):
        if not line.startswith("build "):
            continue
        head, _, rest = line[6:].partition(": ")
        if not rest:
            continue
        outputs = [o for o in head.split(" | ")[0].split() if o]
        explicit, _, implicit = rest.split(" || ")[0].partition(" | ")
        parts = explicit.split()
        if not parts:
            continue
        rule, inputs = parts[0], parts[1:] + implicit.split()
        for out in outputs:
            graph[out.replace("$:", ":")] = (rule, [i.replace("$:", ":") for i in inputs])
    return graph


def native_objects(graph: dict[str, tuple[str, list[str]]], target: str) -> int:
    """Objects compiled by native rules that reach target through native link or archive rules."""
    seen, objects, stack = set(), set(), [target]
    while stack:
        node = stack.pop()
        if node in seen or node not in graph:
            continue
        seen.add(node)
        rule, inputs = graph[node]
        if NATIVE_COMPILE.match(rule):
            objects.add(node)
            continue
        if node == target or NATIVE_LINK.match(rule):
            stack.extend(inputs)
    return len(objects)


def find_target(graph: dict[str, tuple[str, list[str]]], name: str) -> str | None:
    for out in graph:
        if out == name or out.endswith("/" + name):
            return out
    return None


TOKENS = [
    re.compile(r"/logs/verifier\b"), re.compile(r"/opt/grader\b"), re.compile(r"/var/grader\b"),
    re.compile(r"(?<![\w}\]).$/-])/tests(?=[/\s\"')]|$)", re.M), re.compile(r"\breward(?:-details)?\.json\b"),
    re.compile(r"\brewardkit\b"), re.compile(r"\bctrf\b", re.I), re.compile(r"\b(?:GATE_VERDICT|VETO_REPORT)\b"),
    re.compile(r"\bHARBOR_[A-Z_]+"),
]


def no_verifier_awareness(tree: Path, changed: list[str], statea: Path) -> tuple[bool, list[str]]:
    evidence = []
    for rel in changed:
        if rel.startswith(SKIP_SCAN_PREFIXES):
            continue
        text = read_text(tree / rel)
        if text is None:
            continue
        before = read_text(statea / rel) or ""
        for rule in TOKENS:
            match = rule.search(text)
            if match and not rule.search(before):
                line = text[: match.start()].count("\n") + 1
                evidence.append(f"{rel}:{line}: names a verifier-private identifier {match.group(0)!r}")
    return (not evidence, evidence)
