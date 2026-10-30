"""Primitives the graded pipeline reads its evidence from.

Nothing here decides a rubric item. Each function turns one artefact (a source tree, a
build ledger, an ELF file, an install prefix, a cargo manifest) into a plain record that
`pipeline` compares against the frozen State A record of the same shape. Keeping the
comparison out of here is what lets every relation be stated once, against a manifest the
image froze from State A rather than against a constant written by hand.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import time
import tomllib
from pathlib import Path
from typing import Any

# Every extension a C/C++/Objective-C/assembly compiler treats as a translation unit; the
# recorder refuses exactly these, so the source tree is scanned for exactly these.
SOURCE_SUFFIXES = frozenset(
    {
        ".c",
        ".i",
        ".ii",
        ".cc",
        ".cp",
        ".cxx",
        ".cpp",
        ".c++",
        ".C",
        ".CPP",
        ".m",
        ".mi",
        ".mm",
        ".M",
        ".mii",
        ".s",
        ".S",
        ".sx",
        ".asm",
    }
)
# Anything already compiled: shipping one of these and linking it is the prohibited act,
# whatever it is named.
ARTIFACT_SUFFIXES = frozenset(
    {".o", ".obj", ".a", ".lo", ".la", ".so", ".dylib", ".dll", ".lib", ".rlib", ".rmeta"}
)
SHARED_OBJECT = re.compile(r"\.so(\.\d+)*$")
# The calls a JSON library has no business reaching, under every spelling the dynamic
# linker would resolve.
FORBIDDEN_IMPORTS = frozenset(
    {
        "dlopen",
        "dlmopen",
        "dlsym",
        "fork",
        "vfork",
        "execve",
        "posix_spawn",
        "system",
        "popen",
    }
)
# A cargo config table that points the crate graph at bytes the submission did not write.
SOURCE_REDIRECTS = frozenset({"replace-with", "directory", "registry", "git", "local-registry"})

CLEAN_PATH = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"


class Run:
    """One finished subprocess: its status and its captured streams."""

    def __init__(self, code: int, out: str, err: str) -> None:
        self.code = code
        self.out = out
        self.err = err

    @property
    def ok(self) -> bool:
        return self.code == 0

    def tail(self, lines: int = 12) -> str:
        text = (self.out + self.err).strip().splitlines()
        return "\n".join(text[-lines:])


def _end_group(group: int) -> None:
    """SIGKILL every process in `group`, ignoring a group that has already gone.

    `group` is always the pid of a child this module started with `start_new_session=True`,
    so the group holds that child and its descendants and can hold nothing else: the group
    did not exist before the child created it, and a pid is not reused while the child is
    still a member. Killing it therefore cannot reach this process, its parents, or PID 1.
    The two guards below are belt and braces for exactly that: a group id of 0 would mean
    this process's own group, and so would this process's own group id.
    """
    if group <= 1:
        return
    try:
        if group == os.getpgrp():
            return
        os.killpg(group, signal.SIGKILL)
    except OSError:
        return


def run(
    argv: list[str],
    *,
    cwd: str | Path | None = None,
    env: dict[str, str] | None = None,
    timeout: int = 1800,
) -> Run:
    """`argv` to completion, never raising: a failure is a recorded status, not an exception.

    Every graded command in this verifier goes through here, and every one of them runs in a
    session of its own. The submission owns the build steps this launches, so a build step can
    double-fork a process meant to outlive the build and rewrite the verifier's own evidence
    once the stager has written it. `start_new_session=True` makes the child a session and
    process-group leader of a group that did not exist a moment ago, and whatever the child's
    status, that whole group is SIGKILLed before this returns. A double fork does not change
    a process group, so an orphan the build left behind is still a member and dies here; a
    process that calls `setsid` for itself escapes the group and is caught instead by the
    `/proc` sweep in `stage_note`, which runs before the stager writes anything.
    """
    try:
        child = subprocess.Popen(  # noqa: S603 - argv is built here, never from the submission
            argv,
            cwd=str(cwd) if cwd else None,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            errors="replace",
            start_new_session=True,
        )
    except (OSError, ValueError) as error:
        return Run(127, "", f"{type(error).__name__}: {error}")
    group = child.pid
    try:
        try:
            out, err = child.communicate(timeout=timeout)
            code = child.returncode
        except subprocess.TimeoutExpired:
            # The group must die before the pipes are drained again: a surviving grandchild
            # holds the write end open and `communicate` would never return.
            _end_group(group)
            try:
                out, err = child.communicate(timeout=60)
            except (OSError, subprocess.SubprocessError):
                out, err = "", ""
            code = child.returncode if child.returncode is not None else 124
            err = (err or "") + f"\nTimeoutExpired after {timeout}s"
        except (OSError, subprocess.SubprocessError) as error:
            _end_group(group)
            try:
                child.wait(timeout=60)
            except (OSError, subprocess.SubprocessError):
                pass
            return Run(127, "", f"{type(error).__name__}: {error}")
    finally:
        _end_group(group)
        # The child is reaped by `communicate`/`wait` above; anything below it in the group is
        # not this process's child to reap, and a zombie writes nothing and is skipped by the
        # sweep. A short pause lets the kernel deliver the signal before the next stage runs.
        time.sleep(0.05)
    return Run(code, out or "", err or "")


def base_env(**overrides: str) -> dict[str, str]:
    """A fixed, minimal environment: a record may not depend on the ambient one."""
    env = {
        "PATH": CLEAN_PATH,
        "HOME": overrides.pop("HOME", "/tmp"),
        "LC_ALL": "C",
        "LANG": "C",
        "TZ": "UTC",
        "SOURCE_DATE_EPOCH": "1788891432",
    }
    env.update(overrides)
    return env


def sha256_file(path: Path) -> str | None:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for block in iter(lambda: handle.read(1 << 20), b""):
                digest.update(block)
    except OSError:
        return None
    return digest.hexdigest()


def real_tool(name: str) -> str | None:
    """The genuine tool, found on a clean PATH so a shim can never be picked up."""
    return shutil.which(name, path=CLEAN_PATH)


# --------------------------------------------------------------------------- source tree


def snapshot(root: Path) -> dict[str, Any]:
    """The submitted tree as it stood before the graded build: files, header, units, artefacts.

    `.git` is skipped: it holds the task's own starting commit as packed blobs, which are
    neither translation units a build can reach nor artefacts a link can name.
    """
    files: list[str] = []
    units: list[str] = []
    artifacts: dict[str, str] = {}
    for directory, subdirectories, names in os.walk(root):
        subdirectories[:] = sorted(name for name in subdirectories if name != ".git")
        for name in sorted(names):
            whole = Path(directory) / name
            relative = str(whole.relative_to(root))
            files.append(relative)
            suffix = whole.suffix
            if suffix in SOURCE_SUFFIXES:
                units.append(relative)
            if suffix in ARTIFACT_SUFFIXES or SHARED_OBJECT.search(name):
                try:
                    artifacts[str(whole.resolve())] = relative
                except OSError:
                    artifacts[str(whole)] = relative
    return {
        "files": files,
        "header_sha256": sha256_file(root / "src" / "yyjson.h"),
        "translation_units": units,
        "artifacts": artifacts,
    }


# A span long enough that no two independent implementations write it by accident, sampled
# densely enough that a copy cannot hide between samples. The verdict is the share of the
# original implementation that the whole tree carries, not a count in any one file, so a copy
# split across several files still answers for all of it. The share is what keeps the relation
# fair: quoting a function in a design note is honest work and reaches a fraction of a percent,
# while shipping the implementation under any name reaches all of it.
IMPLEMENTATION_SPAN = 256
IMPLEMENTATION_STRIDE = 64
IMPLEMENTATION_SHARE = 0.25
_RUN_OF_SPACE = re.compile(r"\s+")


def _folded(text: str) -> str:
    return _RUN_OF_SPACE.sub(" ", text)


def implementation_spans(source: Path, header: Path) -> frozenset[str]:
    """Whitespace-folded spans that identify the original implementation and nothing else.

    A span is kept only when it is absent from the public header, which the task requires the
    submission to keep verbatim, and when it carries both `;` and `)`, so declarations and
    prose shared between the two files cannot stand in for implementation code.
    """
    try:
        body = _folded(source.read_text(encoding="utf-8", errors="replace"))
        public = _folded(header.read_text(encoding="utf-8", errors="replace"))
    except OSError:
        return frozenset()
    spans = {
        span
        for index in range(0, max(len(body) - IMPLEMENTATION_SPAN, 0), IMPLEMENTATION_STRIDE)
        for span in (body[index : index + IMPLEMENTATION_SPAN],)
        if ";" in span and ")" in span and span not in public
    }
    return frozenset(spans)


def embedded_implementation(root: Path, spans: frozenset[str]) -> dict[str, Any]:
    """How much of the original implementation the submitted tree still carries, and where.

    `src/yyjson.h` is skipped because the task requires it to stay as it is, and `.git` is
    skipped because it holds the starting commit, which is the original tree by construction.
    """
    if not spans:
        return {"share": 0.0, "carried": 0, "total": 0, "files": [], "over": False}
    seen: set[str] = set()
    files: list[tuple[str, int]] = []
    for directory, subdirectories, names in os.walk(root):
        subdirectories[:] = sorted(name for name in subdirectories if name != ".git")
        for name in sorted(names):
            whole = Path(directory) / name
            relative = str(whole.relative_to(root))
            if relative == str(Path("src") / "yyjson.h"):
                continue
            try:
                if whole.stat().st_size > (64 << 20):
                    continue
                text = _folded(whole.read_text(encoding="utf-8", errors="replace"))
            except OSError:
                continue
            here = {span for span in spans if span in text}
            if here:
                files.append((relative, len(here)))
                seen |= here
    share = len(seen) / len(spans)
    return {
        "share": share,
        "carried": len(seen),
        "total": len(spans),
        "files": files,
        "over": share >= IMPLEMENTATION_SHARE,
    }


# -------------------------------------------------------------------------- build ledger


def ledger_entries(path: Path) -> list[dict[str, Any]]:
    """Every invocation the recorder wrote, in order. A malformed line is kept out."""
    entries: list[dict[str, Any]] = []
    try:
        text = path.read_text(encoding="ascii", errors="replace")
    except OSError:
        return entries
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            entry = json.loads(line)
        except ValueError:
            continue
        if isinstance(entry, dict):
            entries.append(entry)
    return entries


def _link_candidates(entry: dict[str, Any]) -> list[str]:
    """Every path a link invocation names, including the ones `-L` and `-l` spell together."""
    argv = [str(item) for item in entry.get("argv") or []]
    cwd = str(entry.get("cwd") or ".")
    candidates = [str(item) for item in entry.get("inputs") or []]

    directories: list[str] = []
    libraries: list[str] = []
    index = 0
    while index < len(argv):
        token = argv[index]
        index += 1
        for flag, sink in (("-L", directories), ("-l", libraries)):
            if token == flag and index < len(argv):
                sink.append(argv[index])
                index += 1
            elif token.startswith(flag) and len(token) > 2:
                sink.append(token[2:])
    for directory in directories or ["."]:
        for name in libraries:
            if name.startswith(":"):
                candidates.append(os.path.join(directory, name[1:]))
                continue
            candidates.extend(
                os.path.join(directory, f"lib{name}{suffix}") for suffix in (".so", ".a")
            )

    resolved: list[str] = []
    for candidate in candidates:
        whole = candidate if os.path.isabs(candidate) else os.path.join(cwd, candidate)
        try:
            resolved.append(str(Path(whole).resolve()))
        except OSError:
            resolved.append(whole)
    return resolved


def shipped_inputs(entries: list[dict[str, Any]], artifacts: dict[str, str]) -> list[str]:
    """The already-compiled files the build linked that the submission shipped rather than built."""
    found: list[str] = []
    for entry in entries:
        if entry.get("kind") != "link":
            continue
        for path in _link_candidates(entry):
            if path in artifacts and artifacts[path] not in found:
                found.append(artifacts[path])
    return found


# -------------------------------------------------------------------------- crate sources


def _config_redirects(root: Path) -> list[str]:
    """Every cargo `[source.*]` table under the tree that points the crate graph elsewhere."""
    findings: list[str] = []
    for name in ("config.toml", "config"):
        for path in sorted(root.rglob(f".cargo/{name}")):
            try:
                config = tomllib.loads(path.read_text(encoding="utf-8", errors="replace"))
            except (OSError, ValueError):
                findings.append(f"unreadable {path.relative_to(root)}")
                continue
            sources = config.get("source")
            if not isinstance(sources, dict):
                continue
            for source, table in sources.items():
                keys = set(table) if isinstance(table, dict) else set()
                if keys & SOURCE_REDIRECTS:
                    findings.append(f"{path.relative_to(root)}: source.{source}")
    return findings


def _vendor_directories(root: Path) -> list[str]:
    """Any checked-in crate tree: a vendored copy is a third-party crate by another route."""
    findings: list[str] = []
    for manifest in sorted(root.rglob("Cargo.toml")):
        relative = manifest.relative_to(root)
        if "vendor" in relative.parts[:-1] or ".cargo" in relative.parts[:-1]:
            findings.append(str(relative))
    return findings


def crate_findings(
    root: Path, manifests: list[Path], cargo: str | None, cargo_home: Path
) -> list[str]:
    """Every crate the build would compile that did not come from the submitted source.

    A package cargo resolved from a registry or a git source names its origin in
    `metadata.source`; a path dependency has none, and is acceptable only while its manifest
    stays inside the submitted tree. An unreadable graph is itself a finding: the absence of
    a foreign crate is credited only when it can be shown.
    """
    findings = _config_redirects(root) + _vendor_directories(root)
    if cargo is None:
        return findings + ["cargo is not installed"]
    for manifest in manifests:
        if not manifest.is_file():
            findings.append(f"missing manifest {manifest}")
            continue
        result = run(
            [cargo, "metadata", "--offline", "--format-version", "1", "--manifest-path", str(manifest)],
            cwd=root,
            env=base_env(CARGO_NET_OFFLINE="true", CARGO_HOME=str(cargo_home)),
        )
        if not result.ok:
            findings.append(f"cargo metadata failed for {manifest}")
            continue
        try:
            metadata = json.loads(result.out)
        except ValueError:
            findings.append(f"cargo metadata is not JSON for {manifest}")
            continue
        for package in metadata.get("packages") or []:
            name = str(package.get("name"))
            source = package.get("source")
            if source:
                findings.append(f"{name} from {source}")
                continue
            where = package.get("manifest_path") or ""
            try:
                inside = Path(where).resolve().is_relative_to(root.resolve())
            except (OSError, ValueError):
                inside = False
            if not inside:
                findings.append(f"{name} outside the tree at {where}")
    return findings


def cargo_manifests(root: Path, entries: list[dict[str, Any]]) -> list[Path]:
    """Every manifest the build asked cargo about, plus the tree's own."""
    manifests: list[Path] = []
    for candidate in [root / "Cargo.toml"]:
        if candidate.is_file():
            manifests.append(candidate)
    for entry in entries:
        if entry.get("kind") != "cargo":
            continue
        argv = [str(item) for item in entry.get("argv") or []]
        for index, token in enumerate(argv):
            named: str | None = None
            if token == "--manifest-path" and index + 1 < len(argv):
                named = argv[index + 1]
            elif token.startswith("--manifest-path="):
                named = token.split("=", 1)[1]
            if named is None:
                continue
            whole = Path(named)
            if not whole.is_absolute():
                whole = Path(str(entry.get("cwd") or ".")) / named
            try:
                whole = whole.resolve()
            except OSError:
                continue
            if whole.is_file() and whole not in manifests:
                manifests.append(whole)
    return manifests


# ------------------------------------------------------------------------------- ELF files


def _nm(path: Path, flag: str) -> list[str] | None:
    tool = real_tool("nm")
    if tool is None:
        return None
    result = run([tool, "-D", flag, str(path)], env=base_env())
    if not result.ok:
        return None
    names: list[str] = []
    for line in result.out.splitlines():
        fields = line.split()
        if not fields:
            continue
        names.append(fields[-1].split("@", 1)[0])
    return sorted(set(names))


def exported_symbols(path: Path) -> list[str] | None:
    """Every name the shared object defines in its dynamic symbol table."""
    return _nm(path, "--defined-only")


def imported_symbols(path: Path) -> list[str] | None:
    """Every name the shared object expects the dynamic linker to resolve for it."""
    return _nm(path, "--undefined-only")


def dynamic_section(path: Path) -> dict[str, Any] | None:
    """The shared object's `DT_SONAME` and its `DT_NEEDED` set."""
    tool = real_tool("readelf")
    if tool is None:
        return None
    result = run([tool, "-d", str(path)], env=base_env())
    if not result.ok:
        return None
    soname: str | None = None
    needed: list[str] = []
    for line in result.out.splitlines():
        match = re.search(r"\(SONAME\).*\[([^\]]+)\]", line)
        if match:
            soname = match.group(1)
        match = re.search(r"\(NEEDED\).*\[([^\]]+)\]", line)
        if match:
            needed.append(match.group(1))
    return {"soname": soname, "needed": sorted(set(needed))}


# -------------------------------------------------------------------------- install trees


def install_manifest(prefix: Path) -> list[list[str]]:
    """Every installed path with its kind and, for a symlink, its target: the install tree's shape."""
    rows: list[list[str]] = []
    for directory, subdirectories, names in os.walk(prefix):
        subdirectories.sort()
        for name in sorted(names):
            whole = Path(directory) / name
            relative = str(whole.relative_to(prefix))
            if whole.is_symlink():
                rows.append([relative, "l", os.readlink(whole)])
            elif whole.is_file():
                rows.append([relative, "f", ""])
            else:
                rows.append([relative, "?", ""])
    return sorted(rows)


def shipped_library(prefix: Path, kind: str) -> Path | None:
    """The regular file the install tree's library name resolves to."""
    entry = prefix / "lib" / ("libyyjson.so" if kind == "shared" else "libyyjson.a")
    try:
        resolved = entry.resolve()
    except OSError:
        return None
    return resolved if resolved.is_file() else None


def tree_bytes(prefix: Path) -> dict[str, bytes]:
    """Every installed regular file's contents, keyed by its path inside the prefix."""
    contents: dict[str, bytes] = {}
    for row in install_manifest(prefix):
        if row[1] != "f":
            continue
        try:
            contents[row[0]] = (prefix / row[0]).read_bytes()
        except OSError:
            contents[row[0]] = b""
    return contents
