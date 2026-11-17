"""Static Stage I scans over the graded tree and the built jar (grader-owned, never shipped to the agent)."""

from __future__ import annotations

import fnmatch
import hashlib
import io
import re
import zipfile
from pathlib import Path
from typing import Any, Iterator

SKIP_DIRS = {".git", "target"}
TEXT_LIMIT = 4 * 1024 * 1024
SOURCE_SUFFIXES = {".java", ".kt", ".kts", ".groovy", ".scala"}
BUILD_NAMES = {"pom.xml", "build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts",
               "gradle.properties", "build.xml", "ivy.xml"}
LOCK_PATTERNS = ("*.lock", "*.lockfile", "gradle.lockfile", "*-lock.json", "dependency-reduced-pom.xml")
CONTAINER_PATTERNS = ("Dockerfile", "Dockerfile.*", "*.dockerfile", "Containerfile", "docker-compose*.yml",
                      "docker-compose*.yaml", "compose*.yml", "compose*.yaml")
CONTAINER_WORDS = re.compile(r"\b(wildfly|jboss|resteasy|weld|glassfish|payara|tomee|javaee|jakartaee|jersey)\b", re.I)
COMMENT = re.compile(r"//[^\n]*|/\*.*?\*/", re.S)
XML_COMMENT = re.compile(r"<!--.*?-->", re.S)
COORD = re.compile(r"<groupId>\s*([^<\s]+)\s*</groupId>\s*<artifactId>\s*([^<\s]+)\s*</artifactId>", re.S)
GRADLE_COORD = re.compile(r"""['"]([A-Za-z0-9_.\-]+):([A-Za-z0-9_.\-]+)(?::[^'"]*)?['"]""")


def walk(root: Path) -> Iterator[Path]:
    for path in sorted(root.rglob("*")):
        rel = path.relative_to(root)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        if path.is_file() and not path.is_symlink():
            yield path


def read_text(path: Path) -> str | None:
    try:
        if path.stat().st_size > TEXT_LIMIT:
            return None
        data = path.read_bytes()
    except OSError:
        return None
    if b"\0" in data[:8192]:
        return None
    return data.decode("utf-8", "replace")


def coordinate_retired(group: str, artifact: str, patterns: list[str]) -> bool:
    for pattern in patterns:
        g, a = pattern.split(":", 1)
        if fnmatch.fnmatchcase(group, g) and fnmatch.fnmatchcase(artifact, a):
            return True
    return False


def package_pattern(packages: list[str]) -> re.Pattern[str]:
    return re.compile(r"(?<![\w.])(" + "|".join(re.escape(p) for p in packages) + r")(?=[.;*\s])")


def retired_stack(tree: Path, deps: list[tuple[str, str]], spec: dict[str, Any]) -> list[str]:
    """Findings for s1_retired_stack_absent: graph, manifests, lockfiles, container files, sources, descriptors."""
    findings: list[str] = []
    coords = spec["retired_coordinates"]
    for group, artifact in deps:
        if coordinate_retired(group, artifact, coords):
            findings.append(f"resolved module graph: {group}:{artifact}")
    pkg = package_pattern(spec["retired_packages"])
    descriptors = set(spec["retired_descriptors"])
    for path in walk(tree):
        rel = path.relative_to(tree).as_posix()
        name = path.name
        if name in descriptors:
            findings.append(f"retired deployment descriptor: {rel}")
            continue
        is_build = name in BUILD_NAMES or name.endswith(".gradle") or name.endswith(".gradle.kts")
        is_lock = any(fnmatch.fnmatchcase(name, p) for p in LOCK_PATTERNS)
        is_container = any(fnmatch.fnmatchcase(name, p) for p in CONTAINER_PATTERNS)
        is_source = path.suffix in SOURCE_SUFFIXES
        if not (is_build or is_lock or is_container or is_source):
            continue
        text = read_text(path)
        if text is None:
            continue
        if is_build or is_lock:
            body = XML_COMMENT.sub(" ", text)
            pairs = COORD.findall(body) + GRADLE_COORD.findall(body)
            for group, artifact in pairs:
                if coordinate_retired(group, artifact, coords):
                    findings.append(f"{rel}: names retired coordinate {group}:{artifact}")
        if is_container:
            for lineno, line in enumerate(text.splitlines(), 1):
                stripped = line.strip()
                if stripped.startswith("#"):
                    continue
                hit = CONTAINER_WORDS.search(line)
                if hit:
                    findings.append(f"{rel}:{lineno}: container file names the retired stack ({hit.group(1)})")
                    break
        if is_source:
            code = COMMENT.sub(" ", text)
            hit = pkg.search(code)
            if hit:
                lineno = code.count("\n", 0, hit.start()) + 1
                findings.append(f"{rel}:{lineno}: uses retired package {hit.group(1)}")
    return findings


def verifier_names(tree: Path, spec: dict[str, Any]) -> list[str]:
    """Findings for the static part of s1_no_verifier_detection."""
    words = [re.escape(w) for w in spec["grader_names"]]
    paths = [r"(?<![\w.-])" + re.escape(p) + r"(?=[/\"'\s:;,)]|$)" for p in spec["grader_paths"]]
    pattern = re.compile("|".join(paths + words))
    findings: list[str] = []
    for path in walk(tree):
        text = read_text(path)
        if text is None:
            continue
        hit = pattern.search(text)
        if hit:
            lineno = text.count("\n", 0, hit.start()) + 1
            findings.append(f"{path.relative_to(tree).as_posix()}:{lineno}: names {hit.group(0)!r}")
    return findings


def _zip_entries(data: bytes, depth: int = 0) -> Iterator[tuple[str, bytes]]:
    try:
        archive = zipfile.ZipFile(io.BytesIO(data))
    except (zipfile.BadZipFile, ValueError, OSError):
        return
    with archive:
        for info in archive.infolist():
            if info.is_dir() or info.file_size > 256 * 1024 * 1024:
                continue
            try:
                body = archive.read(info)
            except (zipfile.BadZipFile, ValueError, OSError, RuntimeError):
                continue
            yield info.filename, body
            if depth < 3 and body[:4] == b"PK\x03\x04":
                for name, inner in _zip_entries(body, depth + 1):
                    yield f"{info.filename}!/{name}", inner


def state_a_classes(tree: Path, jar: Path | None, digests: set[str]) -> list[str]:
    """Any State A war class present byte-identical in the graded tree (any name) or in the built jar."""
    findings: list[str] = []
    for path in walk(tree):
        try:
            data = path.read_bytes()
        except OSError:
            continue
        rel = path.relative_to(tree).as_posix()
        if hashlib.sha256(data).hexdigest() in digests:
            findings.append(f"{rel}: byte-identical State A class")
        elif data[:4] == b"PK\x03\x04":
            for name, body in _zip_entries(data):
                if hashlib.sha256(body).hexdigest() in digests:
                    findings.append(f"{rel}!/{name}: byte-identical State A class")
                    break
        if len(findings) > 20:
            break
    if jar is not None and jar.is_file():
        for name, body in _zip_entries(jar.read_bytes()):
            if hashlib.sha256(body).hexdigest() in digests:
                findings.append(f"built jar {name}: byte-identical State A class")
                break
    return findings


def jar_retired(jar: Path, spec: dict[str, Any]) -> list[str]:
    """Retired coordinates bundled in the jar's BOOT-INF/lib (pom.properties) or retired packages in its classes."""
    findings: list[str] = []
    prefixes = tuple(p.replace(".", "/") + "/" for p in spec["retired_packages"])
    seen: set[str] = set()
    for name, body in _zip_entries(jar.read_bytes()):
        if name.endswith("pom.properties"):
            props = dict(line.split("=", 1) for line in body.decode("utf-8", "replace").splitlines()
                         if "=" in line and not line.startswith("#"))
            group, artifact = props.get("groupId", ""), props.get("artifactId", "")
            if coordinate_retired(group, artifact, spec["retired_coordinates"]) and artifact not in seen:
                seen.add(artifact)
                findings.append(f"built jar bundles retired coordinate {group}:{artifact} ({name})")
        elif name.endswith(".class"):
            inner = name.rsplit("!/", 1)[-1]
            inner = inner.removeprefix("BOOT-INF/classes/")
            if inner.startswith(prefixes) and inner not in seen:
                seen.add(inner)
                findings.append(f"built jar carries retired-package class {name}")
        if len(findings) > 20:
            break
    return findings
