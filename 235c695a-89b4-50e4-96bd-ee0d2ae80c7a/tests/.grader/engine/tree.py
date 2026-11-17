"""Scratch copy of a source tree: build debris left out, owner and modes normalized.

The submitted tree arrives with whatever owner and modes the agent's environment gave it. The
grader never trusts them: directories become 0755, a file becomes 0755 when State A had an
executable file at the same path or the file starts with `#!`, and every other file becomes 0644.
"""

from __future__ import annotations

import fnmatch
import os
import shutil
from pathlib import Path

from .util import chown_tree, is_executable, sha256_file

# Paths the grader drops before copying, matched against the path relative to the tree root.
ROOT_DEBRIS = {
    "build", "share", "yosys", "yosys-abc", "yosys-config", "yosys-filterlib", "yosys-smtbmc",
    "yosys-witness", "libyosys.so", "libyosys.a", "dist", "venv", ".venv", ".cache", "Testing",
    "compile_commands.json", "build.tar",
}
ANY_DEBRIS_DIRS = {".git", "CMakeFiles", "__pycache__", ".pytest_cache", ".mypy_cache", "_skbuild"}
ANY_DEBRIS_FILES = [
    ".git", "*.o", "*.d", "*.a", "*.so", "*.so.*", "*.pyc", "*.pyo", "CMakeCache.txt", "*.ninja",
    ".ninja_deps", ".ninja_log", "cmake_install.cmake", "CTestTestfile.cmake", "*.whl", "*.egg-info",
]
TESTS_DEBRIS = ["*.result", "*.log", "*.err", "*.out", "run-test.mk"]


def debris(rel: str, is_dir: bool, generated_makefile: bool) -> bool:
    parts = rel.split("/")
    name = parts[-1]
    if len(parts) == 1 and name in ROOT_DEBRIS:
        return True
    if is_dir:
        return name in ANY_DEBRIS_DIRS or name.endswith(".egg-info")
    if any(fnmatch.fnmatchcase(name, pattern) for pattern in ANY_DEBRIS_FILES):
        return True
    if parts[0] == "tests" and len(parts) > 2:
        if any(fnmatch.fnmatchcase(name, pattern) for pattern in TESTS_DEBRIS):
            return True
        if generated_makefile:
            return True
    return False


def prepare(src: Path, dst: Path, statea: Path) -> list[str]:
    """Copy src into dst without debris; return the relative paths kept."""
    if dst.exists():
        shutil.rmtree(dst)
    kept: list[str] = []
    for base, dirs, files in os.walk(src):
        rel_base = os.path.relpath(base, src)
        rel_base = "" if rel_base == "." else rel_base.replace(os.sep, "/")
        out_base = dst / rel_base if rel_base else dst
        out_base.mkdir(parents=True, exist_ok=True)
        os.chmod(out_base, 0o755)
        has_generator = "generate_mk.py" in files
        keep_dirs = []
        for name in sorted(dirs):
            rel = f"{rel_base}/{name}" if rel_base else name
            full = os.path.join(base, name)
            if os.path.islink(full):
                if not debris(rel, False, False):
                    os.symlink(os.readlink(full), out_base / name)
                    kept.append(rel)
                continue
            if not debris(rel, True, False):
                keep_dirs.append(name)
        dirs[:] = keep_dirs
        for name in sorted(files):
            rel = f"{rel_base}/{name}" if rel_base else name
            generated = has_generator and name == "Makefile" and rel_base.startswith("tests/")
            if debris(rel, False, generated):
                continue
            full = Path(base) / name
            target = out_base / name
            if full.is_symlink():
                os.symlink(os.readlink(full), target)
            elif full.is_file():
                shutil.copyfile(full, target, follow_symlinks=False)
                os.chmod(target, 0o755 if wants_exec(rel, full, statea) else 0o644)
            else:
                continue
            kept.append(rel)
    chown_tree(dst)
    return sorted(kept)


def wants_exec(rel: str, path: Path, statea: Path) -> bool:
    if is_executable(statea / rel) and (statea / rel).is_file():
        return True
    try:
        with open(path, "rb") as handle:
            return handle.read(2) == b"#!"
    except OSError:
        return False


def changed(rel_paths: list[str], tree: Path, statea: Path) -> list[str]:
    """Paths whose content is new or differs from State A at the same path."""
    out = []
    for rel in rel_paths:
        mine = tree / rel
        theirs = statea / rel
        if mine.is_symlink() or not mine.is_file():
            out.append(rel)
            continue
        if not theirs.is_file() or theirs.is_symlink():
            out.append(rel)
            continue
        if mine.stat().st_size != theirs.stat().st_size or sha256_file(mine) != sha256_file(theirs):
            out.append(rel)
    return out
