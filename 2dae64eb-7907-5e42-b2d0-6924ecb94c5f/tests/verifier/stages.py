"""The stages the graded pipeline runs, each producing a record and nothing else.

The graded build is the only stage that runs under the recorder: it is the submission's own
build, and it is the one that may not compile a translation unit. Every later stage is the
verifier's instrument and uses the real toolchain on a clean PATH, so a submission cannot
starve a measurement by removing a compiler from its build.

The stage-2 and stage-3 harnesses are linked from objects the image compiled once, against
State A's unchanged header, at image build time. Linking the same objects against the
submission's library is what makes a record difference attributable to the library rather
than to the instrument, exactly as the harness's own `build.sh` intends.
"""

from __future__ import annotations

import json
import lzma
import os
import shutil
import urllib.parse
from pathlib import Path
from typing import Any

import probe
from probe import Run, base_env, run

ROOT = Path(os.environ.get("VERIFIER_ROOT", "/opt/verifier"))
SHIM = ROOT / "shim"
CORPUS = ROOT / "corpus"
RECORDS = ROOT / "records"
OBJECTS = ROOT / "obj"
CONSUMER = ROOT / "consumer"
STAGE3_CORPUS = ROOT / "stage3" / "corpus"

STAGE2_MODULES = ("B-01", "B-02", "B-03", "B-04", "B-05", "B-06", "B-07", "B-08", "B-09", "B-10")
STAGE3_FAMILIES = ("X-01", "X-02", "X-03", "X-04", "X-05")
CONSUMER_METHODS = ("pkg-config", "find_package", "hand-written")
INSTALL_KINDS = ("shared", "static")

# The one invocation the task fixes: the kind of library and where it goes, and nothing else.
SHARED_FLAG = {"shared": "ON", "static": "OFF"}


def frozen() -> dict[str, Any]:
    """State A's own records, frozen into the image: the only thing a submission is compared to."""
    return json.loads((ROOT / "statea" / "manifest.json").read_text())


def _xz_text(path: Path) -> str:
    with lzma.open(path, "rt", encoding="utf-8", errors="replace") as handle:
        return handle.read()


# ------------------------------------------------------------------------- the graded build


def build_environment(scratch: Path, ledger: Path) -> dict[str, str] | None:
    """The submission's build environment: the recorder in front, the real tools behind it."""
    real = {name: probe.real_tool(name) for name in ("gcc", "g++", "cpp", "as", "cargo")}
    if real["gcc"] is None or real["cargo"] is None:
        return None
    env = base_env(
        PATH=f"{SHIM}:{probe.CLEAN_PATH}",
        CC=str(SHIM / "cc"),
        CXX=str(SHIM / "c++"),
        RECORDER_LEDGER=str(ledger),
        RECORDER_REAL_DEFAULT=str(real["gcc"]),
        RECORDER_REAL_CC=str(real["gcc"]),
        RECORDER_REAL_GCC=str(real["gcc"]),
        RECORDER_REAL_CARGO=str(real["cargo"]),
        CARGO_NET_OFFLINE="true",
        CARGO_HOME=str(scratch / "cargo-home"),
        HOME=str(scratch / "home"),
    )
    for name, variable in (("g++", "RECORDER_REAL_CXX"), ("g++", "RECORDER_REAL_GXX")):
        if real[name]:
            env[variable] = str(real[name])
    for name, variable in (("cpp", "RECORDER_REAL_CPP"), ("as", "RECORDER_REAL_AS")):
        if real[name]:
            env[variable] = str(real[name])
    return env


def graded_build(
    source: Path, build: Path, prefix: Path, kind: str, env: dict[str, str]
) -> tuple[bool, list[tuple[str, Run]]]:
    """Configure, build and install one kind, with the recorder governing every compiler call.

    The same three commands run for both kinds; only `BUILD_SHARED_LIBS` and the prefix
    differ, so a submission cannot be handed a flag that the task did not promise it.
    """
    cmake = probe.real_tool("cmake")
    if cmake is None:
        return False, [("cmake", Run(127, "", "cmake is not installed"))]
    steps = [
        (
            "configure",
            [
                cmake,
                "-S",
                str(source),
                "-B",
                str(build),
                f"-DBUILD_SHARED_LIBS={SHARED_FLAG[kind]}",
                f"-DCMAKE_INSTALL_PREFIX={prefix}",
            ],
        ),
        ("build", [cmake, "--build", str(build), "--parallel"]),
        ("install", [cmake, "--install", str(build)]),
    ]
    log: list[tuple[str, Run]] = []
    for name, argv in steps:
        result = run(argv, cwd=source, env=env, timeout=1800)
        log.append((f"{name}-{kind}", result))
        if not result.ok:
            return False, log
    return True, log


# -------------------------------------------------------------------- stage 2: behavioural


def _objects(where: Path) -> list[str]:
    """The harness objects in the order the image linked them, read from the image.

    Order matters and is not cosmetic: module B-07 drives the library through a fixed-size
    pool over an over-aligned static buffer, and that buffer's alignment inside `.bss`, and
    therefore how much of the pool the pool header consumes, follows the order the objects
    were given to the linker. The image writes the order it used beside the objects so that
    the link producing State A's frozen record and the link producing the submission's are
    the same link.
    """
    order = (where / "link-order.txt").read_text().split()
    return [str(where / name) for name in order]


def link_harness(objects: Path, prefix: Path, output: Path) -> Run:
    """The frozen harness objects, linked against the submission's installed shared library."""
    gcc = probe.real_tool("gcc")
    if gcc is None:
        return Run(127, "", "gcc is not installed")
    library = prefix / "lib"
    return run(
        [
            gcc,
            *_objects(objects),
            "-o",
            str(output),
            f"-L{library}",
            "-lyyjson",
            "-lm",
            f"-Wl,-rpath,{library}",
            "-Wl,--disable-new-dtags",
        ],
        env=base_env(),
    )


def replay(
    binary: Path,
    corpus: Path,
    names: tuple[str, ...],
    frozen_digests: dict[str, str],
    where: Path,
) -> dict[str, bool]:
    """Each module or family against the submission's library, compared to State A's record."""
    outcome: dict[str, bool] = {name: False for name in names}
    for name in names:
        output = where / f"{name}.txt"
        temporary = where / f"tmp-{name}"
        shutil.rmtree(temporary, ignore_errors=True)
        temporary.mkdir(parents=True, exist_ok=True)
        result = run(
            [str(binary), name, str(corpus), str(output), str(temporary)],
            cwd=where,
            env=base_env(),
            timeout=1800,
        )
        shutil.rmtree(temporary, ignore_errors=True)
        if not result.ok:
            continue
        outcome[name] = probe.sha256_file(output) == frozen_digests.get(name)
    return outcome


def first_divergence(name: str, produced: Path, archive: Path) -> str:
    """Where a module's record first disagrees, for the run log and nothing else.

    Both sides are streamed: a module's record runs to a hundred megabytes of text and the
    pipeline never holds one in memory.
    """
    try:
        with lzma.open(archive, "rt", encoding="utf-8", errors="replace") as expected, produced.open(
            encoding="utf-8", errors="replace"
        ) as actual:
            index = 0
            while True:
                index += 1
                left = expected.readline()
                right = actual.readline()
                if not left and not right:
                    return f"{name}: records agree"
                if left != right:
                    kind = "line" if left and right else "record ends at line"
                    return f"{name}: {kind} {index} diverges"
    except (OSError, lzma.LZMAError):
        return f"{name}: record unreadable"


# ----------------------------------------------------------------------- stage: consumers


def consumer_record(prefixes: dict[str, Path], where: Path) -> str:
    """One line per consumer build: the three link routes against both install kinds.

    The program itself only asks the library for its version and compares it with the
    header's, so the line records whether each route built, whether it ran, and what it
    printed, which is exactly what a consumer of an ABI-compatible library observes.
    """
    gcc = probe.real_tool("gcc")
    cmake = probe.real_tool("cmake")
    pkgconf = probe.real_tool("pkg-config") or probe.real_tool("pkgconf")
    lines: list[str] = []
    for kind in INSTALL_KINDS:
        prefix = prefixes[kind]
        for method in CONSUMER_METHODS:
            binary = where / f"use-{method}-{kind}"
            build = Run(127, "", "tool missing")
            if method == "pkg-config" and gcc and pkgconf:
                flags = run(
                    [pkgconf, "--cflags", "--libs", "yyjson"],
                    env=base_env(PKG_CONFIG_PATH=str(prefix / "lib" / "pkgconfig")),
                )
                build = (
                    run(
                        [gcc, "-O2", "-o", str(binary), str(CONSUMER / "use_yyjson.c"), *flags.out.split()],
                        env=base_env(),
                    )
                    if flags.ok
                    else flags
                )
            elif method == "find_package" and cmake:
                tree = where / f"cm-{kind}"
                configure = run(
                    [
                        cmake,
                        "-S",
                        str(CONSUMER),
                        "-B",
                        str(tree),
                        f"-Dyyjson_DIR={prefix / 'lib' / 'cmake' / 'yyjson'}",
                    ],
                    env=base_env(),
                )
                build = run([cmake, "--build", str(tree)], env=base_env()) if configure.ok else configure
                if build.ok:
                    shutil.copy2(tree / "use_yyjson", binary)
            elif method == "hand-written" and gcc:
                build = run(
                    [
                        gcc,
                        "-O2",
                        f"-I{prefix / 'include'}",
                        "-o",
                        str(binary),
                        str(CONSUMER / "use_yyjson.c"),
                        f"-L{prefix / 'lib'}",
                        "-lyyjson",
                    ],
                    env=base_env(),
                )
            execution = (
                run([str(binary)], env=base_env(LD_LIBRARY_PATH=str(prefix / "lib")))
                if build.ok
                else Run(127, "", "not built")
            )
            lines.append(
                "consumer method=%s kind=%s build=%d run=%d out=%s"
                % (
                    method,
                    kind,
                    0 if build.ok else 1,
                    execution.code,
                    urllib.parse.quote(execution.out, safe=""),
                )
            )
    return "".join(line + "\n" for line in lines)


# -------------------------------------------------------------- stage: rebuild idempotence


def _normalized(contents: dict[str, bytes], prefix: Path) -> dict[str, bytes]:
    """Installed bytes with the prefix string folded away: only the prefix may legitimately differ."""
    marker = b"@YYJSON_INSTALL_PREFIX@"
    encoded = str(prefix).encode()
    return {name: data.replace(encoded, marker) for name, data in contents.items()}


def rebuild_matches(
    source: Path, scratch: Path, first: dict[str, Path], env: dict[str, str]
) -> tuple[bool, list[str]]:
    """A second configure, build and install into a fresh prefix, against the first's result.

    The path list must match exactly. The bytes must match once each prefix's own path is
    folded to a marker, because the pkg-config file and the CMake export set record the
    prefix they were configured for and nothing else about them may move.
    """
    notes: list[str] = []
    for kind in INSTALL_KINDS:
        build = scratch / f"rebuild-{kind}"
        prefix = scratch / f"reprefix-{kind}"
        shutil.rmtree(build, ignore_errors=True)
        shutil.rmtree(prefix, ignore_errors=True)
        ok, log = graded_build(source, build, prefix, kind, env)
        if not ok:
            notes.append(f"{kind}: {log[-1][0]} failed")
            return False, notes
        if probe.install_manifest(prefix) != probe.install_manifest(first[kind]):
            notes.append(f"{kind}: install paths differ")
            return False, notes
        before = _normalized(probe.tree_bytes(first[kind]), first[kind])
        after = _normalized(probe.tree_bytes(prefix), prefix)
        differing = sorted(
            name for name in before if before[name] != after.get(name)
        )
        if differing:
            notes.append(f"{kind}: {len(differing)} installed file(s) differ: {differing[:4]}")
            return False, notes
        notes.append(f"{kind}: {len(before)} installed files identical")
    return True, notes
