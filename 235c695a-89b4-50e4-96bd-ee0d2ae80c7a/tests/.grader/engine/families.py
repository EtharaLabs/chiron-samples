"""Stage III: held-out counterexample families.

A family is one executable probe whose State A result is recorded; it passes only when every
case it records agrees. Families reach behaviour the Stage II groups leave alone: configurations
beyond the default build, procedures (rebuilding after an edit, building from a read-only
source, building inside a git checkout, relocating an install) and routes a consumer takes from
an installed tree.

A host is the build a family runs against. Hosts reuse the Stage II builds where they can (the
default build B0 and its install, the alternate build, the static library) and otherwise make
one build each. The families that edit sources run last, on B0 itself.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import time
from collections.abc import Callable
from functools import partial
from pathlib import Path

from . import wasm
from .grader import CLANG, PREFIX, Grader, elf_facts
from .util import grader_dir, read_text, sha256_file, sha256_text, walk_files

# family id -> host, in the order hosts run
FAMILIES = {
    "F-datdir-copied-binary": "b0",
    "F-static-consumer": "static",
    "F-alt-installed-consumer": "alt",
    "F-alt-noabc-synth": "alt",
    "F-alt-python-tools": "alt",
    "F-alt-relocated-share": "alt",
    "F-debug-banner": "debug",
    "F-debug-backtrace": "debug",
    "F-debug-install": "debug",
    "F-help-source": "debug",
    "F-off-tcl": "features",
    "F-off-zlib": "features",
    "F-off-plugins": "features",
    "F-off-readline": "features",
    "F-usr-libdir": "features",
    "F-version-checkout": "git",
    "F-readonly-outside-build": "readonly",
    "F-wasi-default": "wasi0",
    "F-tools-relocated": "relocate",
    "F-install-without-build": "relocate",
    "F-noop-rebuild": "incremental",
    "F-delete-share-product": "incremental",
    "F-edit-share-file": "incremental",
    "F-edit-cellhelp": "incremental",
    "F-edit-python-tool": "incremental",
    "F-edit-generator": "incremental",
    "F-edit-config-template": "incremental",
}
HOSTS = ["b0", "static", "alt", "debug", "features", "git", "readonly", "wasi0", "relocate", "incremental"]

SOURCE_ROOT = re.compile(r"^.*?((?:kernel|passes|frontends|backends|techlibs|libs|pyosys)/.*)$")
GIT_ENV = {
    "GIT_AUTHOR_NAME": "Grader",
    "GIT_AUTHOR_EMAIL": "grader@example.invalid",
    "GIT_COMMITTER_NAME": "Grader",
    "GIT_COMMITTER_EMAIL": "grader@example.invalid",
    "GIT_AUTHOR_DATE": "2026-06-02T08:07:03Z",
    "GIT_COMMITTER_DATE": "2026-06-02T08:07:03Z",
    "GIT_CONFIG_NOSYSTEM": "1",
}


def run(g: Grader, selected: list[str]) -> dict[str, float]:
    """Run the hosts the selected families need, in order; returns seconds per host."""
    wanted = [f for f in FAMILIES if f in selected]
    timings: dict[str, float] = {}
    if wanted:
        # Installed consumers resolve the real prefix; placing B0 there again is idempotent.
        g.install_live()
    for host in HOSTS:
        mine = [f for f in wanted if FAMILIES[f] == host]
        if not mine:
            continue
        t0 = time.time()
        try:
            HOST_RUNNERS[host](g, mine)
        except Exception as exc:  # noqa: BLE001 - a host crash fails its families, never the run
            for f in mine:
                g.note(f, f"host {host} stopped: {exc!r}"[:500])
        timings[host] = round(time.time() - t0, 1)
    return timings


def _observe(mine: list[str], observers: dict[str, Callable[[str], None]]) -> None:
    """Run the observations of a host's selected families, in the order the host lists them."""
    for family, observe in observers.items():
        if family in mine:
            observe(family)


def grade(g: Grader, selected: list[str], expected: dict[str, dict[str, str]], canon) -> dict:
    timings = run(g, selected)
    out = {}
    for f in selected:
        exp = expected.get(f, {})
        got = {c: canon(v) for c, v in g.cases.get(f, {}).items()}
        bad = sorted(c for c, v in exp.items() if got.get(c) != v)
        out[f] = {
            "pass": bool(exp) and not bad,
            "cases": len(exp),
            "failed": bad[:50],
            "failed_count": len(bad),
            "host": FAMILIES.get(f, "unknown"),
            "seconds": timings.get(FAMILIES.get(f, ""), 0.0),
            "notes": g.notes.get(f, [])[:3],
        }
        dump = g.logs / "cases" / f
        for c in bad[:50]:
            dump.mkdir(parents=True, exist_ok=True)
            (dump / (c.replace("/", "__")[:150] + ".txt")).write_text(
                f"expected:\n{exp.get(c)}\n\nobserved:\n{g.cases.get(f, {}).get(c, '<missing>')}\n"
            )
    return out


# --------------------------------------------------------------------------- helpers
def _dir(g: Grader, name: str) -> Path:
    path = g.work / name
    if path.exists():
        shutil.rmtree(path)
    return grader_dir(path)


def _out(g: Grader, res, *pairs: tuple[str, str]) -> str:
    text = res.out
    for old, new in pairs:
        text = text.replace(old, new)
    return f"exit={res.rc}\n{g.norm(text)}"


def _probe_inputs(g: Grader, folder: Path, *names: str) -> None:
    for name in names:
        shutil.copy(g.probe_dir / name, folder / name)


def _place(g: Grader, staged: Path, target: Path) -> int:
    """Copy a staged install to its real prefix as root, as `install_live` does for B0."""
    if not staged.is_dir():
        return 1
    rc = g.run.run(f"place-{target.name}", ["cp", "-a", "--no-preserve=ownership", f"{staged}/.", str(target)], g.work, as_grader=False).rc
    for rel in walk_files(staged):
        path = target / rel
        if not path.is_symlink() and path.exists():
            path.chmod(path.stat().st_mode & 0o777)
    return rc


def _cmake_build(g: Grader, name: str, src: Path, build: Path, args: list[str], target: str | None = None) -> tuple[int, int]:
    with g.guard():
        rc_c = g.step(f"{name}-configure", ["cmake", "-S", str(src), "-B", str(build), "-G", "Ninja", *CLANG, *args], src).rc
        argv = ["cmake", "--build", str(build), f"-j{g.jobs}"] + (["--target", target] if target else [])
        rc_b = g.step(f"{name}-build", argv, src, timeout=7200).rc if rc_c == 0 else 1
    return rc_c, rc_b


def _make_build(g: Grader, name: str, src: Path, build: Path, conf: str, targets: list[str]) -> tuple[int, int]:
    grader_dir(build)
    make = ["make", "-f", f"{src}/Makefile"]
    rc_c = g.step(f"{name}-config", make + ["config-clang"], build).rc
    with open(build / "Makefile.conf", "a") as handle:
        handle.write(conf)
    rc_b = g.step(f"{name}-build", make + [f"-j{g.jobs}", *targets], build, timeout=7200).rc
    return rc_c, rc_b


def _demo(g: Grader, folder: Path, cfg: str, link: list[str], env: dict[str, str]) -> tuple[str, str, str]:
    """Compile and run State A's C++ API example against the given library."""
    _probe_inputs(g, folder, "example.v")
    shutil.copy(g.statea / "examples" / "cxx-api" / "demomain.cc", folder / "demomain.cc")
    flags = g.step(f"{folder.name}-cxxflags", [cfg, "--cxxflags"], folder).out.split()
    libs = g.step(f"{folder.name}-ldlibs", [cfg, "--ldlibs"], folder).out.split()
    built = g.step(f"{folder.name}-build", ["clang++", *flags, "-o", "demomain", "demomain.cc", *link, *libs], folder, timeout=900)
    ran = g.step(f"{folder.name}-run", ["./demomain"], folder, env, timeout=600)
    blif = folder / "example.blif"
    digest = sha256_text(g.norm(read_text(blif) or "")) if blif.exists() else "missing"
    return str(built.rc), f"exit={ran.rc}\n{g.norm(ran.out)}", digest


def _state(path: Path, source: Path) -> str:
    if not path.is_file():
        return "missing"
    return "fresh" if sha256_file(path) == sha256_file(source) else "stale"


# --------------------------------------------------------------------------- host: b0
def host_b0(g: Grader, mine: list[str]) -> None:
    f = "F-datdir-copied-binary"
    folder = _dir(g, "fam-copied")
    binary = g.b0_build / "yosys"
    if binary.is_file():
        shutil.copy2(binary, folder / "yosys")
        os.chown(folder / "yosys", *_owner(folder))
    _probe_inputs(g, folder, "example.v")
    mine_bin = str(folder / "yosys")
    for case, script in (
        ("share", "read_verilog -lib +/simcells.v; ls"),
        ("synth", "read_verilog example.v; synth -top top -noabc; stat"),
    ):
        res = g.step(f"copied-{case}", [mine_bin, "-Q", "-T", "-p", script], folder)
        g.case(f, case, _out(g, res, (mine_bin, "<yosys>")))


def _owner(folder: Path) -> tuple[int, int]:
    st = folder.stat()
    return st.st_uid, st.st_gid


# --------------------------------------------------------------------------- host: static
def host_static(g: Grader, mine: list[str]) -> None:
    f = "F-static-consumer"
    archive = g.work / "static" / "build" / "libyosys.a"
    if not archive.is_file():
        g.option_static()
    g.case(f, "archive", "present" if archive.is_file() else "absent")
    if not archive.is_file():
        return
    folder = _dir(g, "fam-static")
    cfg = str(Path(PREFIX) / "bin" / "yosys-config")
    link = ["-Wl,--whole-archive", str(archive), "-Wl,--no-whole-archive"]
    built, ran, blif = _demo(g, folder, cfg, link, {})
    g.case(f, "build-exit", built)
    g.case(f, "run", ran)
    g.case(f, "blif", blif)


# --------------------------------------------------------------------------- host: alt
ALT_PREFIX = Path("/opt/yosys")


def host_alt(g: Grader, mine: list[str]) -> None:
    staged = g.work / "alt" / "destdir" / ALT_PREFIX.relative_to("/")
    if not staged.is_dir():
        g.option_build()
    placed = _place(g, staged, ALT_PREFIX)
    libdir = ALT_PREFIX / "lib" / "gr-yosys"

    def installed_consumer(f: str) -> None:
        g.case(f, "placed", str(placed))
        folder = _dir(g, "fam-alt-cxxapi")
        cfg = str(ALT_PREFIX / "bin" / "gr-yosys-config")
        built, ran, blif = _demo(g, folder, cfg, [f"-L{libdir}", "-lyosys"], {"LD_LIBRARY_PATH": str(libdir)})
        g.case(f, "build-exit", built)
        g.case(f, "run", ran)
        g.case(f, "blif", blif)

    def noabc_synth(f: str) -> None:
        folder = _dir(g, "fam-alt-noabc")
        _probe_inputs(g, folder, "example.v")
        yosys = str(ALT_PREFIX / "bin" / "gr-yosys")
        for case, script in (("synth", "read_verilog example.v; synth -top top; stat"), ("help-abc", "help abc")):
            res = g.step(f"alt-noabc-{case}", [yosys, "-Q", "-T", "-p", script], folder)
            g.case(f, case, _out(g, res, (yosys, "<yosys>")))

    def python_tools(f: str) -> None:
        for tool in ("gr-yosys-smtbmc", "gr-yosys-witness"):
            path = str(ALT_PREFIX / "bin" / tool)
            res = g.step(f"alt-{tool}", [path, "--help"], g.probe_dir)
            g.case(f, tool, _out(g, res, (path, f"<{tool}>")))

    _observe(mine, {
        "F-alt-installed-consumer": installed_consumer,
        "F-alt-noabc-synth": noabc_synth,
        "F-alt-python-tools": python_tools,
        "F-alt-relocated-share": partial(_alt_relocated, g, staged),
    })


def _alt_relocated(g: Grader, staged: Path, f: str) -> None:
    """A copy of the prefixed install, moved elsewhere, finds its data beside itself."""
    root = _dir(g, "fam-altmoved")
    g.run.run("altmoved-copy", ["cp", "-a", "--no-preserve=ownership", f"{staged}/.", str(root)], g.work, as_grader=False)
    folder = _dir(g, "fam-altmoved-run")
    _probe_inputs(g, folder, "example.v")
    yosys = str(root / "bin" / "gr-yosys")
    # The real prefix is set aside while the copy runs, so only a lookup beside the copy works.
    parked = ALT_PREFIX.with_name(ALT_PREFIX.name + ".grader-parked")
    moved = False
    try:
        if ALT_PREFIX.is_dir():
            os.rename(ALT_PREFIX, parked)
            moved = True
        for case, script in (
            ("share", "read_verilog -lib +/simcells.v; ls"),
            ("synth", "read_verilog example.v; synth -top top -noabc; stat"),
        ):
            res = g.step(f"altmoved-{case}", [yosys, "-Q", "-T", "-p", script], folder)
            g.case(f, case, _out(g, res, (yosys, "<yosys>"), (str(root), "<moved>")))
    finally:
        if moved:
            os.rename(parked, ALT_PREFIX)


# --------------------------------------------------------------------------- host: debug
def host_debug(g: Grader, mine: list[str]) -> None:
    """ENABLE_DEBUG=1 ENABLE_HELP_SOURCE=1 against CMAKE_BUILD_TYPE=Debug YOSYS_ENABLE_HELP_SOURCE=ON."""
    src = g.fresh("dbg")
    build = src / "build"
    dest = grader_dir(g.work / "dbg" / "destdir")
    if g.flavour == "make":
        rc_c, rc_b = _make_build(g, "dbg", src, build, "ENABLE_DEBUG := 1\nENABLE_HELP_SOURCE := 1\n", [])
        rc_i = g.step("dbg-install", ["make", "-f", f"{src}/Makefile", "install", f"DESTDIR={dest}", f"PREFIX={PREFIX}"], build).rc
    else:
        rc_c, rc_b = _cmake_build(g, "dbg", src, build, ["-DCMAKE_BUILD_TYPE=Debug", "-DYOSYS_ENABLE_HELP_SOURCE=ON"])
        with g.guard():
            rc_i = g.step("dbg-install", ["cmake", "--install", str(build), "--prefix", PREFIX], src, {"DESTDIR": str(dest)}).rc if rc_b == 0 else 1
    g.subs.append((str(dest), "<destdir>"))
    yosys = str(build / "yosys")
    steps = f"configure={rc_c} build={rc_b}"

    def banner(f: str) -> None:
        res = g.step("dbg-banner", [yosys, "-V"], g.probe_dir)
        g.case(f, "banner", f"{steps}\nexit={res.rc}\n{g.norm(res.out, keep_banner=True)}")

    def backtrace(f: str) -> None:
        # Two xtrace levels print each traced call's first frame. At three, a build without the
        # debug define prints a hint in place of the second frame, while a build with it walks
        # deeper frames, which may end the process; only whether the hint appears is compared.
        res = g.step("dbg-xtrace-2", [yosys, "-Q", "-T", "-X", "-X", "-p", "read_verilog example.v; proc"], g.probe_dir)
        first = bool(re.search(r"^-X- frame #1:", res.out, re.M))
        g.case(f, "first-frame", f"{steps}\nexit={res.rc} first-frame={first}")
        res = g.step("dbg-xtrace-3", [yosys, "-Q", "-T", "-X", "-X", "-X", "-p", "read_verilog example.v; proc"], g.probe_dir)
        g.case(f, "deeper", f"hint={'build Yosys with ENABLE_DEBUG' in res.out}")

    def install(f: str) -> None:
        g.case(f, "install-exit", str(rc_i))
        for name in ("yosys", "yosys-abc", "yosys-filterlib"):
            facts = elf_facts(dest / PREFIX.lstrip("/") / "bin" / name, g.run, g.probe_dir)
            g.case(f, f"bin/{name}", " ".join(f"{k}={facts.get(k, '')}" for k in ("type", "debug", "symtab")))

    def help_source(f: str) -> None:
        out = g.work / "dbg" / "cmds.json"
        res = g.step("dbg-cmds-json", [yosys, "-Q", "-T", "-p", f"help -dump-cmds-json {out}"], g.probe_dir)
        g.case(f, "dump", f"{steps}\nexit={res.rc}")
        g.case(f, "sources", _help_sources(out))

    _observe(mine, {
        "F-debug-banner": banner,
        "F-debug-backtrace": backtrace,
        "F-debug-install": install,
        "F-help-source": help_source,
    })


def _help_sources(path: Path) -> str:
    """Each command's recorded source file and function, with the checkout prefix dropped.

    Line numbers are left out: they follow the source text, which a migration may edit.
    """
    try:
        data = json.loads(path.read_text())
    except (OSError, ValueError):
        return "unreadable"
    cmds = data.get("cmds", {}) if isinstance(data, dict) else {}
    rows = {}
    for name, entry in sorted(cmds.items()):
        if not isinstance(entry, dict):
            continue
        row = {}
        for key, value in sorted(entry.items()):
            if not key.startswith("source") or key == "source_line":
                continue
            if isinstance(value, str):
                match = SOURCE_ROOT.match(value)
                value = match.group(1) if match else value
            row[key] = value
        rows[name] = row
    text = json.dumps(rows, sort_keys=True)
    return f"commands={len(rows)} sha256={sha256_text(text)}"


# --------------------------------------------------------------------------- host: features
FEATURES_MAKE = (
    "ENABLE_TCL := 0\nENABLE_READLINE := 0\nENABLE_ZLIB := 0\nENABLE_PLUGINS := 0\n"
    "ENABLE_LIBYOSYS := 1\nPREFIX := /usr\n"
)
FEATURES_CMAKE = [
    "-DYOSYS_WITHOUT_TCL=ON", "-DYOSYS_WITHOUT_READLINE=ON", "-DYOSYS_WITHOUT_ZLIB=ON", "-DYOSYS_WITHOUT_PLUGINS=ON",
    "-DYOSYS_INSTALL_LIBRARY=ON", "-DCMAKE_INSTALL_PREFIX=/usr",
]


def host_features(g: Grader, mine: list[str]) -> None:
    """Tcl, readline, zlib and plugins off, the library installed, prefix /usr."""
    src = g.fresh("feat")
    build = src / "build"
    dest = grader_dir(g.work / "feat" / "destdir")
    if g.flavour == "make":
        rc_c, rc_b = _make_build(g, "feat", src, build, FEATURES_MAKE, [])
        rc_i = g.step("feat-install", ["make", "-f", f"{src}/Makefile", "install", f"DESTDIR={dest}"], build).rc
    else:
        rc_c, rc_b = _cmake_build(g, "feat", src, build, FEATURES_CMAKE)
        with g.guard():
            rc_i = g.step("feat-install", ["cmake", "--install", str(build)], src, {"DESTDIR": str(dest)}).rc if rc_b == 0 else 1
    g.subs.append((str(dest), "<destdir>"))
    steps = f"configure={rc_c} build={rc_b}"
    yosys = str(build / "yosys")
    q = [yosys, "-Q", "-T"]
    probes = {
        "F-off-tcl": [("tcl-script", q + ["-c", "probe.tcl"], None), ("help-tcl", q + ["-p", "help tcl"], None)],
        "F-off-zlib": [
            ("read-gz", q + ["-p", "read_verilog example.v.gz; hierarchy -top top; stat"], None),
            ("write-gz", q + ["-p", "read_verilog example.v; write_rtlil feat-roundtrip.il.gz; design -reset; read_rtlil feat-roundtrip.il.gz; stat"], None),
        ],
        "F-off-plugins": [
            ("help-plugin", q + ["-p", "help plugin"], None),
            ("dpi", q + ["-p", "read_verilog -sv dpi.v; proc; opt; write_verilog -noattr"], None),
        ],
        "F-off-readline": [("stdin", q, b"echo on\nhelp proc_rom\n")],
    }
    for f, rows in probes.items():
        if f not in mine:
            continue
        g.case(f, "build", steps)
        for case, argv, stdin in rows:
            res = g.run.run(f"feat-{case}", argv, g.probe_dir, stdin=stdin, timeout=600)
            g.case(f, case, _out(g, res, (yosys, "<yosys>")))

    def usr_libdir(f: str) -> None:
        g.case(f, "install-exit", str(rc_i))
        g.case(f, "paths", "\n".join(walk_files(dest)))

    _observe(mine, {"F-usr-libdir": usr_libdir})


# --------------------------------------------------------------------------- host: git
GIT_SCRIPT = """
set -e
printf '%s\\n' '$Format:%H$' > .gitcommit
git init -q -b main
git add -A -f .
git commit -q -m base
git tag v0.66
for i in 1 2 3; do echo "change $i" >> grader-history.txt; git add grader-history.txt; git commit -q -m "change $i"; done
git remote add origin https://github.com/Example/Yosys-Fork.git
git checkout -q -b feature-x
echo "local edit" >> README.md
"""


def host_git(g: Grader, mine: list[str]) -> None:
    """A build inside a git checkout: tag distance, dirty tree, a fork remote and a branch."""
    f = "F-version-checkout"
    src = g.fresh("git")
    made = g.run.shell("git-history", GIT_SCRIPT, src, GIT_ENV, timeout=900)
    g.case(f, "history", str(made.rc))
    build = src / "build"
    if g.flavour == "make":
        rc_c, rc_b = _make_build(g, "git", src, build, "", ["yosys"])
    else:
        rc_c, rc_b = _cmake_build(g, "git", src, build, [], target="yosys")
    head = g.step("git-head", ["git", "rev-parse", "--short=9", "HEAD"], src, GIT_ENV).out.strip()
    res = g.step("git-banner", [str(build / "yosys"), "-V"], src)
    text = res.out.replace(head, "<head>") if head else res.out
    g.case(f, "banner", f"configure={rc_c} build={rc_b}\nexit={res.rc}\n{g.norm(text, keep_banner=True)}")


# --------------------------------------------------------------------------- host: readonly
def host_readonly(g: Grader, mine: list[str]) -> None:
    """A build from a source tree nobody may write, into a build directory outside it."""
    f = "F-readonly-outside-build"
    base = _dir(g, "ro")
    src = base / "src"
    g.run.run("ro-copy", ["cp", "-a", "--no-preserve=ownership", f"{g.tree}/.", str(src)], g.work, as_grader=False, timeout=900)
    g.run.run("ro-lock", ["chmod", "-R", "a-w", str(src)], g.work, as_grader=False)
    build = grader_dir(g.work / "ro-out")
    g.subs += [(str(src), "<src>"), (str(build), "<build>")]
    if g.flavour == "make":
        rc_c, rc_b = _make_build(g, "ro", src, build, "", [])
    else:
        rc_c, rc_b = _cmake_build(g, "ro", src, build, [])
    g.case(f, "build", f"configure={rc_c} build={rc_b}")
    yosys = str(build / "yosys")
    script = "read_verilog -lib +/quicklogic/qlf_k6n10f/bram_types_sim.v; read_verilog -lib +/gatemate/lut_tree_map.v; ls"
    res = g.step("ro-share", [yosys, "-Q", "-T", "-p", script], g.probe_dir)
    g.case(f, "generated-share", _out(g, res, (yosys, "<yosys>")))


# --------------------------------------------------------------------------- host: wasi0
def host_wasi0(g: Grader, mine: list[str]) -> None:
    """The WASI build with only what it cannot do without: no stack or strip flags of the CI job."""
    f = "F-wasi-default"
    sdk = Path("/opt/wasi-sdk")
    src = g.fresh("wasi0")
    flex = grader_dir(g.work / "wasi0" / "flex-prefix")
    grader_dir(flex / "include")
    shutil.copy("/usr/include/FlexLexer.h", flex / "include" / "FlexLexer.h")
    env = {"PATH": f"{sdk}/bin:{flex}/bin:{g.run.path}"}
    if g.flavour == "make":
        build = grader_dir(src / "build")
        (build / "Makefile.conf").write_text(
            f"export PATH := {sdk}/bin:{flex}/bin:{g.run.path}\nWASI_SYSROOT := {sdk}/share/wasi-sysroot\n\n"
            "CONFIG := wasi\nPREFIX := /\n\n"
            "ENABLE_TCL := 0\nENABLE_READLINE := 0\nENABLE_PLUGINS := 0\nENABLE_ZLIB := 0\n\n"
            f"CXXFLAGS += -I{flex}/include\n"
        )
        rc_c = 0
        rc_b = g.step("wasi0-build", ["make", "-C", "build", "-f", "../Makefile", "CXX=clang", f"-j{g.jobs}"], src, env, timeout=7200).rc
    else:
        build = src / "build"
        configure = [
            "cmake", "-B", "build", "-G", "Ninja", f"-DCMAKE_TOOLCHAIN_FILE={sdk}/share/cmake/wasi-sdk-p1.cmake",
            "-DCMAKE_INSTALL_PREFIX=/", "-DYOSYS_WITHOUT_TCL=ON", "-DYOSYS_WITHOUT_READLINE=ON",
            "-DYOSYS_WITHOUT_PLUGINS=ON", "-DYOSYS_WITHOUT_ZLIB=ON", f"-DCMAKE_CXX_FLAGS=-I{flex}/include", ".",
        ]
        with g.guard():
            rc_c = g.step("wasi0-configure", configure, src, env).rc
            rc_b = g.step("wasi0-build", ["cmake", "--build", "build", f"-j{g.jobs}"], src, env, timeout=7200).rc if rc_c == 0 else 1
    g.case(f, "build", f"configure={rc_c} build={rc_b}")
    modules = sorted(p.name for p in build.glob("*.wasm"))
    g.case(f, "modules", "\n".join(modules))
    for name in modules:
        for key, value in wasm.facts(build / name).items():
            g.case(f, f"{name}/{key}", value)


# --------------------------------------------------------------------------- host: relocate
TOOLS = ["yosys", "yosys-abc", "yosys-config", "yosys-filterlib", "yosys-smtbmc", "yosys-witness"]


def host_relocate(g: Grader, mine: list[str]) -> None:
    _observe(mine, {
        "F-tools-relocated": partial(_tools_relocated, g),
        "F-install-without-build": partial(_install_without_build, g),
    })


def _tools_relocated(g: Grader, f: str) -> None:
    """The installed Python tools find their modules from a copy of the prefix moved elsewhere."""
    staged = g.destdir / PREFIX.lstrip("/")
    root = _dir(g, "fam-reloc")
    g.run.run("reloc-copy", ["cp", "-a", "--no-preserve=ownership", f"{staged}/.", str(root)], g.work, as_grader=False)
    hidden = Path(PREFIX) / "share" / "yosys" / "python3"
    parked = hidden.with_name("python3.grader-parked")
    moved = False
    try:
        if hidden.is_dir():
            os.rename(hidden, parked)
            moved = True
        for tool in ("yosys-smtbmc", "yosys-witness"):
            path = str(root / "bin" / tool)
            res = g.step(f"reloc-{tool}", [path, "--help"], g.probe_dir)
            g.case(f, tool, _out(g, res, (path, f"<{tool}>"), (str(root), "<reloc>")))
    finally:
        if moved:
            os.rename(parked, hidden)


def _install_without_build(g: Grader, f: str) -> None:
    """Everything installed keeps working once the build directory is gone."""
    folder = _dir(g, "fam-nobuild")
    _probe_inputs(g, folder, "example.v", "plugin.cc", "cells.lib")
    copied = folder / "copied-yosys"
    if (g.b0_build / "yosys").is_file():
        shutil.copy2(g.b0_build / "yosys", copied)
        os.chown(copied, *_owner(folder))
    parked = g.b0_build.with_name("build.grader-parked")
    moved = False
    try:
        if g.b0_build.is_dir():
            os.rename(g.b0_build, parked)
            moved = True
        bin_ = Path(PREFIX) / "bin"
        rows = [
            ("synth", [str(bin_ / "yosys"), "-Q", "-T", "-p", "read_verilog example.v; synth -top top; stat"]),
            ("smtbmc", [str(bin_ / "yosys-smtbmc"), "--help"]),
            ("witness", [str(bin_ / "yosys-witness"), "--help"]),
            ("filterlib", [str(bin_ / "yosys-filterlib"), "-verilogsim", "cells.lib"]),
            ("plugin-build", [str(bin_ / "yosys-config"), "--build", "nobuild-plugin.so", "plugin.cc"]),
            ("plugin-load", [str(bin_ / "yosys"), "-Q", "-T", "-m", "./nobuild-plugin.so", "-p", "grader_probe"]),
            ("copied-share", [str(copied), "-Q", "-T", "-p", "read_verilog -lib +/simcells.v; ls"]),
        ]
        for case, argv in rows:
            res = g.step(f"nobuild-{case}", argv, folder)
            g.case(f, case, _out(g, res, (str(copied), "<copied>")))
    finally:
        if moved:
            os.rename(parked, g.b0_build)


# --------------------------------------------------------------------------- host: incremental
PRODUCTS = ["yosys", "yosys-abc", "yosys-filterlib", "libyosys.so", "yosys-config", "yosys-smtbmc", "yosys-witness",
            "share/simlib.v", "share/techmap.v"]
# The Make build copies yosys-abc out of its ABC tree on every run, so a no-op rebuild is judged on
# the products it leaves untouched.
RECOPIED = {"yosys-abc"}


def _rebuild(g: Grader, name: str) -> int:
    if g.flavour == "make":
        return g.step(name, ["make", "-f", "../Makefile", f"-j{g.jobs}"], g.b0_build, timeout=7200).rc
    with g.guard():
        return g.step(name, ["cmake", "--build", "build", f"-j{g.jobs}"], g.b0, timeout=7200).rc


def _edit(path: Path, change) -> bool:
    """Apply an edit after the clock has moved past every earlier build output."""
    text = read_text(path)
    if text is None:
        return False
    new = change(text)
    if new is None:
        return False
    time.sleep(1.2)
    path.write_text(new, encoding="utf-8", errors="surrogateescape")
    return True


def host_incremental(g: Grader, mine: list[str]) -> None:
    """Rebuilding the default build: nothing to redo, then one edited input at a time."""

    def noop(f: str) -> None:
        before = {p: (g.b0_build / p).stat().st_mtime_ns for p in PRODUCTS if (g.b0_build / p).exists()}
        time.sleep(1.2)
        rc = _rebuild(g, "inc-noop")
        after = {p: (g.b0_build / p).stat().st_mtime_ns for p in PRODUCTS if (g.b0_build / p).exists()}
        changed = sorted(p for p in set(before) | set(after) if before.get(p) != after.get(p) and p not in RECOPIED)
        g.case(f, "rebuild", f"exit={rc} present={sorted(after)} changed={changed}")

    def delete_share(f: str) -> None:
        product = g.b0_build / "share" / "simcells.v"
        source = g.b0 / "techlibs" / "common" / "simcells.v"
        existed = product.is_file()
        product.unlink(missing_ok=True)
        rc = _rebuild(g, "inc-delete")
        res = g.step("inc-delete-read", [str(g.b0_build / "yosys"), "-Q", "-T", "-p", "read_verilog -lib +/simcells.v; ls"], g.probe_dir)
        g.case(f, "share", f"existed={existed} build-exit={rc} share={_state(product, source)}\n{_out(g, res)}")

    def edit_share(f: str) -> None:
        source = g.b0 / "techlibs" / "common" / "techmap.v"
        edited = _edit(source, lambda t: t + "// grader edit: share copy\n")
        rc = _rebuild(g, "inc-share")
        g.case(f, "build", f"edited={edited} exit={rc} share={_state(g.b0_build / 'share' / 'techmap.v', source)}")
        if g.flavour == "make":
            rc_i = g.step("inc-share-install", ["make", "-f", "../Makefile", "install", f"DESTDIR={g.destdir}", f"PREFIX={PREFIX}"], g.b0_build).rc
        else:
            with g.guard():
                rc_i = g.step("inc-share-install", ["cmake", "--install", "build", "--prefix", PREFIX], g.b0, {"DESTDIR": str(g.destdir)}).rc
        installed = g.destdir / PREFIX.lstrip("/") / "share" / "yosys" / "techmap.v"
        g.case(f, "install", f"exit={rc_i} installed={_state(installed, source)}")

    def edit_cellhelp(f: str) -> None:
        source = g.b0 / "techlibs" / "common" / "simlib.v"
        edited = _edit(source, lambda t: t.replace("module \\$add (", "//- Grader edit: this line documents the edited cell.\n//-\nmodule \\$add (", 1)
                       if "module \\$add (" in t else None)
        rc = _rebuild(g, "inc-cellhelp")
        res = g.step("inc-cellhelp-help", [str(g.b0_build / "yosys"), "-Q", "-T", "-p", "help $add"], g.probe_dir)
        g.case(f, "help", f"edited={edited} build-exit={rc}\n{_out(g, res)}")

    def edit_python_tool(f: str) -> None:
        source = g.b0 / "backends" / "smt2" / "smtbmc.py"
        edited = _edit(source, lambda t: t + "# grader edit: python tool\n")
        rc = _rebuild(g, "inc-tool")
        tool = g.b0_build / "yosys-smtbmc"
        body = read_text(tool) or ""
        res = g.step("inc-tool-help", [str(tool), "--help"], g.probe_dir)
        g.case(f, "tool", f"edited={edited} build-exit={rc} carries-edit={'# grader edit: python tool' in body} help-exit={res.rc}")

    def edit_generator(f: str) -> None:
        source = g.b0 / "techlibs" / "quicklogic" / "qlf_k6n10f" / "generate_bram_types_sim.py"
        marker = "// grader edit: generator"
        # The generator takes its output path as its only argument, as `filename`.
        edited = _edit(source, lambda t: t + f'\nwith open(filename, "a") as grader_handle:\n\tgrader_handle.write("{marker}\\n")\n')
        rc = _rebuild(g, "inc-generator")
        share = g.b0_build / "share" / "quicklogic" / "qlf_k6n10f" / "bram_types_sim.v"
        body = read_text(share) or ""
        script = "read_verilog -lib +/quicklogic/qlf_k6n10f/bram_types_sim.v; ls"
        res = g.step("inc-generator-read", [str(g.b0_build / "yosys"), "-Q", "-T", "-p", script], g.probe_dir)
        g.case(f, "share", f"edited={edited} build-exit={rc} present={share.is_file()} carries-edit={marker in body}\n{_out(g, res)}")

    def edit_config_template(f: str) -> None:
        source = g.b0 / "misc" / "yosys-config.in"
        marker = "# grader edit: config template"
        edited = _edit(source, lambda t: t + marker + "\n")
        rc = _rebuild(g, "inc-config")
        tool = g.b0_build / "yosys-config"
        body = read_text(tool) or ""
        res = g.step("inc-config-cxx", [str(tool), "--cxx"], g.probe_dir)
        g.case(f, "script", f"edited={edited} build-exit={rc} carries-edit={marker in body}\n{_out(g, res)}")

    _observe(mine, {
        "F-noop-rebuild": noop,
        "F-delete-share-product": delete_share,
        "F-edit-share-file": edit_share,
        "F-edit-cellhelp": edit_cellhelp,
        "F-edit-python-tool": edit_python_tool,
        "F-edit-generator": edit_generator,
        "F-edit-config-template": edit_config_template,
    })


HOST_RUNNERS = {
    "b0": host_b0,
    "static": host_static,
    "alt": host_alt,
    "debug": host_debug,
    "features": host_features,
    "git": host_git,
    "readonly": host_readonly,
    "wasi0": host_wasi0,
    "relocate": host_relocate,
    "incremental": host_incremental,
}
