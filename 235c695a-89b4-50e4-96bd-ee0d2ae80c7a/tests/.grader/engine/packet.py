"""Line-numbered projections of the migrated tree for the judges.

Each packet is a directory of text parts. Every part lists files from one tree, each opened by a
`==> <path> <==` header and printed with its line numbers, so a judge can cite a path, a line and
a quote. The packets carry the files' own bytes and nothing the grader wrote about them.
"""

from __future__ import annotations

import shutil
from pathlib import Path

from .stage1 import _build_scripts, _packaging_scripts
from .util import read_text, walk_files

PART_LIMIT = 900_000
SKIP = ("abc/", "libs/cxxopts/")


def _render(root: Path, rels: list[str]) -> list[str]:
    parts, current = [], []
    size = 0
    for rel in rels:
        text = read_text(root / rel)
        if text is None:
            continue
        lines = text.split("\n")
        if lines and lines[-1] == "":
            lines.pop()
        block = f"==> {rel} <==\n" + "".join(f"{n:5d}| {line}\n" for n, line in enumerate(lines, 1)) + "\n"
        if current and size + len(block) > PART_LIMIT:
            parts.append("".join(current))
            current, size = [], 0
        current.append(block)
        size += len(block)
    if current:
        parts.append("".join(current))
    return parts


def _write(out: Path, parts: list[str]) -> None:
    out.mkdir(parents=True, exist_ok=True)
    for index, part in enumerate(parts or ["(no files)\n"], 1):
        (out / f"part-{index:02d}.txt").write_text(part[:PART_LIMIT + 200_000], encoding="utf-8")


def _changed(tree: Path, statea: Path, rels: list[str]) -> list[str]:
    return [rel for rel in rels if (statea / rel).is_file() is False or (statea / rel).read_bytes() != (tree / rel).read_bytes()]


def build_files(tree: Path, files: list[str], statea: Path) -> list[str]:
    """Every file the migrated build reads to configure or drive itself."""
    picked = set()
    for rel in files:
        name = rel.rsplit("/", 1)[-1]
        if rel.startswith(SKIP):
            continue
        if name == "CMakeLists.txt" or name.endswith(".cmake") or name in ("CMakePresets.json", "CMakeUserPresets.json"):
            picked.add(rel)
    picked |= {rel for rel in _build_scripts(tree, files) if not rel.startswith(SKIP)}
    picked |= set(_packaging_scripts(tree, files))
    for rel in ("setup.py", "pyproject.toml", "flake.nix", "Brewfile"):
        if (tree / rel).is_file():
            picked.add(rel)
    picked |= {rel for rel in files if rel.startswith("nix/") and rel.endswith(".nix")}
    # Changes inside the submodules are part of the build the submission drives.
    picked |= set(_changed(tree, statea, [rel for rel in files if rel.startswith(SKIP)]))
    return sorted(picked)


def ci_files(root: Path) -> list[str]:
    return sorted(rel for rel in walk_files(root) if rel.startswith((".github/workflows/", ".github/actions/")))


DATA_SUFFIXES = (".v", ".sv", ".il", ".ys", ".vhd", ".lib", ".blif", ".aig", ".aag", ".json", ".gz", ".png", ".svg", ".vcd", ".fst")


def harness_files(tree: Path, files: list[str], statea: Path) -> tuple[list[str], list[str], list[str]]:
    """Test and docs harness files the submission changed, their originals, and removed test files."""
    rels = [rel for rel in files if rel.startswith(("tests/", "docs/")) and not rel.endswith(DATA_SUFFIXES)]
    changed = _changed(tree, statea, rels)
    original = [rel for rel in changed if (statea / rel).is_file()]
    removed = sorted(f"tests/{rel}" for rel in walk_files(statea / "tests") if not (tree / "tests" / rel).exists())
    return changed, original, removed


def write(tree: Path, files: list[str], statea: Path, out: Path) -> None:
    shutil.rmtree(out, ignore_errors=True)
    _write(out / "build", _render(tree, build_files(tree, files, statea)))
    _write(out / "ci-migrated", _render(tree, ci_files(tree)))
    _write(out / "ci-original", _render(statea, ci_files(statea)))
    changed, original, removed = harness_files(tree, files, statea)
    _write(out / "harness-migrated", _render(tree, changed))
    _write(out / "harness-original", _render(statea, original))
    (out / "harness-migrated" / "removed.txt").write_text(
        "Files under tests/ before the migration that the migrated tree no longer has:\n"
        + "".join(f"{rel}\n" for rel in removed),
        encoding="utf-8",
    )
    packaging = ["setup.py", "pyproject.toml", "wheels/cibw_before_all.sh", "wheels/cibw_before_build.sh",
                 "wheels/_run_cibw_linux.py", ".github/workflows/wheels.yml"]
    _write(out / "packaging-migrated", _render(tree, [rel for rel in packaging if (tree / rel).is_file()]))
    _write(out / "packaging-original", _render(statea, [rel for rel in packaging if (statea / rel).is_file()]))
