"""Entry points: record State A, merge recordings into expectations, grade a submission."""

from __future__ import annotations

import argparse
import contextlib
import gzip
import json
import os
import platform
import shutil
import time
import traceback
from pathlib import Path

from . import families, packet, stage1, tree
from .grader import PREFIX, Grader
from .util import grader_dir, read_text, sha256_file, sha256_text

STAGE_ONE = [
    "S1-make-build-absent",
    "S1-cmake-default-installs",
    "S1-no-embedded-old-build",
    "S1-no-build-forwarding",
    "S1-no-verifier-awareness",
]
PHASES = [
    ("reference", ["S2-command-reference", "S2-compiled-features"]),
    ("install", ["S2-install-layout", "S2-binary-abi"]),
    ("consumers", ["S2-consumer-routes"]),
    ("options", ["S2-option-matrix"]),
    ("ci-test-build", ["S2-ci-test-build"]),
    ("suites", ["S2-suite-frontends", "S2-suite-passes", "S2-suite-techmap-arch", "S2-suite-sim-formal"]),
    ("docs", ["S2-ci-docs"]),
    ("vendor", ["S2-ci-source-vendor"]),
    ("compile", ["S2-ci-test-compile"]),
    ("wasi", ["S2-ci-extra-builds"]),
    ("wheels", ["S2-ci-wheels"]),
    ("sanitizers", ["S2-ci-test-sanitizers"]),
]
STAGE_TWO = [group for _, groups in PHASES for group in groups]
CANON_LIMIT = 2048
ARCH = {"aarch64": "arm64", "x86_64": "amd64"}[platform.machine()]


def canon(value: str) -> str:
    return value if len(value) <= CANON_LIMIT else "sha256:" + sha256_text(value)


PHASE_STEPS = {
    "reference": (Grader.command_reference, Grader.compiled_features),
    "install": (Grader.install_layout, Grader.binary_abi),
    "consumers": (Grader.consumer_routes,),
    "options": (Grader.option_matrix, Grader.option_build, Grader.option_static),
    "ci-test-build": (Grader.ci_test_build,),
    "suites": (Grader.suites,),
    "docs": (Grader.docs_prepare,),
    "vendor": (Grader.source_vendor,),
    "compile": (Grader.compile_matrix,),
    "wasi": (Grader.wasi,),
    "wheels": (Grader.wheels,),
    "sanitizers": (Grader.sanitizers,),
}


def run_phase(g: Grader, name: str) -> None:
    for step in PHASE_STEPS[name]:
        step(g)


# --------------------------------------------------------------------------- make stub
class MakeGuard:
    """While active, `make` and `gmake` are replaced by a stub that fails and leaves a marker."""

    def __init__(self, work: Path) -> None:
        # Owned by root with mode 1733: graded code may leave a marker but cannot list the
        # directory to find one.
        self.markers = work / ".make-guard"
        self.markers.mkdir(parents=True, exist_ok=True)
        self.markers.chmod(0o1733)
        self.vault = Path("/opt/grader-vault")
        self.active = os.geteuid() == 0

    @contextlib.contextmanager
    def __call__(self):
        if not self.active:
            yield
            return
        self.vault.mkdir(mode=0o700, exist_ok=True)
        moved = []
        for name in ("make", "gmake"):
            path = Path("/usr/bin") / name
            if path.exists() or path.is_symlink():
                shutil.move(str(path), str(self.vault / name))
                moved.append(name)
            path.write_text(
                "#!/bin/sh\n"
                f"echo \"$0 $*\" > {self.markers}/invoked.$$ 2>/dev/null\n"
                "echo 'make is not available during the graded build' >&2\nexit 127\n"
            )
            path.chmod(0o755)
        try:
            yield
        finally:
            for name in ("make", "gmake"):
                path = Path("/usr/bin") / name
                path.unlink(missing_ok=True)
                if name in moved:
                    shutil.move(str(self.vault / name), str(path))

    def invocations(self) -> list[str]:
        out = []
        for marker in sorted(self.markers.glob("invoked.*")):
            out.append((read_text(marker) or "").strip())
        return out


# --------------------------------------------------------------------------- record
def record(args: argparse.Namespace) -> int:
    work = grader_dir(Path(args.work))
    logs = grader_dir(Path(args.out) / "logs")
    statea = Path(args.tree)
    src = work / "src"
    tree.prepare(statea, src, statea)
    g = Grader("make", src, work, logs, statea)
    g.guard = contextlib.nullcontext
    started = time.time()
    g.build_default()
    # Every phase sees the default build installed at its prefix, whichever phases run.
    g.install_live()
    timings = {}
    for name, _ in PHASES:
        if args.phases is not None and name not in args.phases:
            continue
        t0 = time.time()
        try:
            run_phase(g, name)
        except Exception:  # noqa: BLE001 - a recording records the failure and goes on
            g.note(name, traceback.format_exc())
        timings[name] = round(time.time() - t0, 1)
    timings.update({f"family-host/{host}": secs for host, secs in families.run(g, list(families.FAMILIES) if args.families is None else args.families).items()})
    meta = {
        "arch": ARCH,
        "recorded_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "seconds": round(time.time() - started, 1),
        "timings": timings,
        "banner": g.banner,
        "generated": generated_files(g.b0_build, statea),
        "make_files": make_files(statea),
        "notes": g.notes,
        "faults": g.run.faults,
    }
    out = Path(args.out)
    (out / "observed.json").write_text(json.dumps({k: {c: canon(v) for c, v in sorted(cs.items())} for k, cs in sorted(g.cases.items())}, indent=1, sort_keys=True))
    (out / "meta.json").write_text(json.dumps(meta, indent=1, sort_keys=True))
    # The full text behind every digest, for diagnosing a graded divergence; never shipped.
    with gzip.open(out / "full.json.gz", "wt", encoding="utf-8") as handle:
        json.dump(g.cases, handle, sort_keys=True)
    # A recording the host cut short measured the host, so it is reported and never merged.
    for fault in g.run.faults:
        print(f"infrastructure fault: {fault}")
    return 1 if g.run.faults else 0


def make_files(statea: Path) -> list[str]:
    found = []
    for path in sorted(statea.rglob("*")):
        rel = path.relative_to(statea).as_posix()
        if rel.startswith(("abc/", "tests/", "docs/", "examples/", "libs/")):
            continue
        if path.name in ("Makefile", "Makefile.inc") or path.name.endswith(".mk"):
            found.append(rel)
    return found


def generated_files(build: Path, statea: Path) -> dict[str, str]:
    """Text files the State A build generates beside its objects, by relative path and digest."""
    out = {}
    if not build.exists():
        return out
    for path in sorted(build.rglob("*")):
        if not path.is_file() or path.is_symlink():
            continue
        rel = path.relative_to(build).as_posix()
        if rel.startswith(("share/", "abc/", "tests/")) or path.suffix in (".o", ".d", ".so", ".a", ".log"):
            continue
        if path.name in ("Makefile.conf", "yosys-config", "yosys-smtbmc", "yosys-witness") or "/" not in rel:
            continue
        if (statea / rel).exists():
            continue
        data = path.read_bytes()
        if b"\0" in data[:4096] or not data:
            continue
        out[rel] = sha256_file(path)
    return out


def merge(args: argparse.Namespace) -> int:
    runs = [json.loads((Path(r) / "observed.json").read_text()) for r in args.recordings]
    metas = [json.loads((Path(r) / "meta.json").read_text()) for r in args.recordings]
    cut = [r for r, m in zip(args.recordings, metas) if m.get("faults")]
    if cut:
        raise SystemExit(f"recordings the host cut short cannot be merged: {', '.join(cut)}")
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    cases: dict[str, dict[str, str]] = {}
    excluded: dict[str, dict[str, str]] = {}
    groups = sorted(set().union(*[set(r) for r in runs]))
    for group in groups:
        names = sorted(set().union(*[set(r.get(group, {})) for r in runs]))
        for name in names:
            values = [r.get(group, {}).get(name) for r in runs]
            if any(v is None for v in values):
                excluded.setdefault(group, {})[name] = "missing from at least one State A recording"
            elif len(set(values)) != 1:
                excluded.setdefault(group, {})[name] = "differs across State A recordings"
            else:
                cases.setdefault(group, {})[name] = values[0]
    (out / "cases.json").write_text(json.dumps(cases, indent=1, sort_keys=True))
    (out / "excluded.json").write_text(json.dumps(excluded, indent=1, sort_keys=True))
    meta = {
        "arch": metas[0]["arch"],
        "recordings": [m["recorded_at"] for m in metas],
        "banner": metas[0]["banner"],
        "generated": metas[0]["generated"],
        "make_files": metas[0]["make_files"],
    }
    (out / "meta.json").write_text(json.dumps(meta, indent=1, sort_keys=True))
    total = sum(len(v) for v in cases.values())
    print(f"kept {total} cases in {len(cases)} groups; excluded {sum(len(v) for v in excluded.values())}")
    return 0


# --------------------------------------------------------------------------- grade
def grade(args: argparse.Namespace) -> int:
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    results: dict[str, object] = {"stage1": {}, "stage2": {}, "stage3": {}, "veto": {"fired": False, "failed": []}, "error": None}
    held: dict[str, Grader] = {}
    try:
        _grade(args, results, held)
    except Exception:  # noqa: BLE001 - every grader stop is named, never a silent zero
        results["error"] = traceback.format_exc()[-4000:]
        results["veto"] = {"fired": True, "failed": ["grader-error"]}
    faults = held["grader"].run.faults if "grader" in held else []
    if faults:
        # The host cut a step short, so the trial measured the host: it is reported for one
        # rerun and never stands as a grading result.
        results["error"] = "infrastructure fault: " + "; ".join(faults[:5]) + "; rerun the trial once"
        results["veto"] = {"fired": True, "failed": ["infrastructure-fault"]}
    out_path.write_text(json.dumps(results, indent=1, sort_keys=True))
    return 0


def _grade(args: argparse.Namespace, results: dict, held: dict[str, Grader]) -> None:
    expect_dir = Path(args.expect) / ARCH
    expected = json.loads((expect_dir / "cases.json").read_text())
    meta = json.loads((expect_dir / "meta.json").read_text())
    statea = Path(args.statea)
    work = grader_dir(Path(args.work))
    # The verdict directory stays root's: graded code runs as the grading account.
    logs = Path(args.logs)
    logs.mkdir(parents=True, exist_ok=True)
    src = work / "src"
    kept = tree.prepare(Path(args.tree), src, statea)
    changed = tree.changed(kept, src, statea)
    # The judges read line-numbered projections of the migrated tree, written before any build.
    packet.write(src, kept, statea, logs / "packet")
    s1 = results["stage1"]
    ok, ev = stage1.make_build_absent(src, kept)
    s1["S1-make-build-absent"] = {"pass": ok, "evidence": ev}
    ok, ev = stage1.no_embedded_old_build(src, changed, statea, meta["make_files"], meta["generated"])
    s1["S1-no-embedded-old-build"] = {"pass": ok, "evidence": ev}
    ok, ev = stage1.no_verifier_awareness(src, changed, statea)
    s1["S1-no-verifier-awareness"] = {"pass": ok, "evidence": ev}
    forwarding = stage1.forwarding_static(src, kept)

    g = Grader("cmake", src, work, logs, statea)
    held["grader"] = g
    guard = MakeGuard(work)
    g.guard = guard
    with guard():
        steps = g.build_default()
    yosys = g.destdir / PREFIX.lstrip("/") / "bin" / "yosys"
    runs = g.step("s1-installed-yosys", [str(yosys), "-V"], g.probe_dir).rc if yosys.exists() else 127
    installs_ok = all(steps.get(k) == 0 for k in ("configure", "build", "install")) and runs == 0
    s1["S1-cmake-default-installs"] = {
        "pass": installs_ok,
        "evidence": [f"{k} exit {v}" for k, v in steps.items()] + [f"installed bin/yosys -V exit {runs}"],
    }
    invoked = guard.invocations()
    forwarding += [f"make was invoked during the graded build: {line[:200]}" for line in invoked]
    # The link-graph check needs a build that succeeded: a build that fails to configure or
    # compile is graded by S1-cmake-default-installs, and is no evidence of forwarding.
    built = steps.get("configure") == 0 and steps.get("build") == 0
    graph = stage1.ninja_products(g.b0_build / "build.ninja") if built and (g.b0_build / "build.ninja").exists() else {}
    # libyosys is held to the same rule whenever the build defines it.
    for name, minimum, required in (("yosys", 100, True), ("yosys-abc", 100, True), ("libyosys.so", 100, False)) if built else ():
        target = stage1.find_target(graph, name)
        if target is None and not required:
            continue
        count = stage1.native_objects(graph, target) if target else 0
        if count < minimum:
            forwarding.append(f"build.ninja: {name} is not linked by CMake native rules from compiled objects ({count} objects)")
    s1["S1-no-build-forwarding"] = {"pass": not forwarding, "evidence": forwarding}
    failed = [item for item in STAGE_ONE if not s1[item]["pass"]]
    # Authoring only: the discovery search runs the named families whatever the vetoes say.
    search = args.families is not None
    if failed:
        results["veto"] = {"fired": True, "failed": failed}
        if not search:
            return
    # Every phase sees the default build installed at its prefix, whichever phases run.
    g.install_live()
    s2 = results["stage2"]
    for name, groups in PHASES:
        if args.phases is not None and name not in args.phases:
            for group in groups:
                s2[group] = {"pass": False, "run": False, "cases": len(expected.get(group, {})), "failed": [], "failed_count": 0, "seconds": 0.0, "notes": ["phase not selected"]}
            continue
        t0 = time.time()
        try:
            run_phase(g, name)
        except Exception:  # noqa: BLE001 - a probe crash fails its group with the trace as evidence
            g.note(name, traceback.format_exc()[-2000:])
        for group in groups:
            exp = expected.get(group, {})
            got = {c: canon(v) for c, v in g.cases.get(group, {}).items()}
            bad = sorted(c for c, v in exp.items() if got.get(c) != v)
            s2[group] = {"pass": not bad and bool(exp), "run": True, "cases": len(exp), "failed": bad[:200], "failed_count": len(bad), "seconds": round(time.time() - t0, 1), "notes": g.notes.get(name, [])[:5]}
            dump = logs / "cases" / group
            dump.mkdir(parents=True, exist_ok=True)
            for c in bad[:200]:
                (dump / (c.replace("/", "__")[:150] + ".txt")).write_text(f"expected:\n{exp.get(c)}\n\nobserved:\n{g.cases.get(group, {}).get(c, '<missing>')}\n")
    # Every group runs once Stage I passes, so each trial reports every group; the veto then
    # fires when at least k groups failed, and Stage III runs only when it did not.
    failed_groups = [grp for grp, v in s2.items() if not v["pass"] and v["run"]]
    if len(failed_groups) >= args.k and not search:
        results["veto"] = {"fired": True, "failed": failed_groups}
        return
    if search:
        selected = args.families if args.families else list(families.FAMILIES)
    else:
        # The frozen selection is the Stage III list of the generated item table.
        selected = json.loads(Path(args.items).read_text(encoding="utf-8"))["stage3"]
    results["stage3"] = families.grade(g, selected, expected, canon)


def _read_json(path: str | None) -> object:
    if not path:
        return None
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def veto(args: argparse.Namespace) -> int:
    """Print the failed veto items, or the grader error, or nothing when no veto fired."""
    results = _read_json(args.results)
    if not isinstance(results, dict):
        print("grader-error: no readable grader results")
        return 0
    error = results.get("error")
    if error:
        last = [line for line in str(error).strip().splitlines() if line.strip()]
        print("grader-error: " + (last[-1].strip()[:300] if last else "unnamed"))
        return 0
    fired = results.get("veto") or {}
    if fired.get("fired"):
        print(", ".join(str(item) for item in fired.get("failed") or ["unattributed"]))
    return 0


def _criteria(details: object, inputs: tuple[str, ...]) -> dict[str, float]:
    """Criterion values by name from a Reward Kit details file, for the named inputs.

    The file is keyed by dimension. Under a root reward.toml the inputs are components of the
    `reward` group, so the walk follows group components to any depth."""
    found: dict[str, float] = {}

    def take(detail: dict) -> None:
        for row in detail.get("criteria") or []:
            if not isinstance(row, dict):
                continue
            key = row.get("id") or row.get("name")
            value = row.get("value")
            if isinstance(key, str) and isinstance(value, (int, float)) and not isinstance(value, bool):
                found[key] = float(value)

    def walk(name: str, detail: object) -> None:
        if not isinstance(detail, dict):
            return
        if name in inputs:
            take(detail)
        for part in detail.get("components") or []:
            if isinstance(part, dict):
                walk(str(part.get("name")), part.get("detail"))

    if isinstance(details, dict):
        for name, detail in details.items():
            walk(str(name), detail)
    return found


def ctrf(args: argparse.Namespace) -> int:
    """Write the CTRF report: every declared id once, on every route, with its real status."""
    items = _read_json(args.items)
    if not isinstance(items, dict):
        items = {}
    results = _read_json(args.results)
    results = results if isinstance(results, dict) else {}
    # The gate directory holds a lone judge input and no reward.toml, so Reward Kit names it `reward`.
    gate = _criteria(_read_json(args.gate), ("judge", "reward"))
    judged = _criteria(_read_json(args.details), ("judge",))
    error = results.get("error")
    fired = (results.get("veto") or {}).get("fired") is True
    tests: list[dict[str, object]] = []
    seen: set[str] = set()

    def add(name: str, status: str, message: str) -> None:
        if name in seen:
            return
        seen.add(name)
        tests.append({"name": name, "status": status, "duration": 0, "message": message[:1000]})

    for stage, reason in (("stage1", "Stage I"), ("stage2", "Stage II"), ("stage3", "Stage III")):
        rows = results.get(stage) or {}
        for name in items.get(stage) or []:
            row = rows.get(name)
            if isinstance(row, dict) and row.get("run", True):
                evidence = row.get("evidence") or row.get("failed") or []
                add(name, "passed" if row.get("pass") is True else "failed", "; ".join(str(e) for e in evidence)[:1000])
            elif error:
                add(name, "other", "grader error before this item was decided")
            else:
                add(name, "skipped", f"{reason} not run: the veto fired earlier" if fired else f"{reason} not run")
    for key, source in (("gate", gate), ("judged", judged)):
        for name in items.get(key) or []:
            if name in source:
                add(name, "passed" if source[name] > 0 else "failed", f"value {source[name]}")
            else:
                add(name, "skipped", "judge not called on this route")
    failed = (results.get("veto") or {}).get("failed") or []
    add("vetoed", "failed" if fired else "passed", ", ".join(str(f) for f in failed) if fired else "no veto")
    if error:
        add("grader-error", "failed", str(error)[-1000:])
    summary = {status: sum(1 for t in tests if t["status"] == status) for status in ("passed", "failed", "skipped", "pending", "other")}
    now = int(time.time() * 1000)
    report = {
        "results": {
            "tool": {"name": "yosys-cmake-grader"},
            "summary": {"tests": len(tests), **summary, "start": now, "stop": now},
            "tests": tests,
        }
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=1))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="engine")
    sub = parser.add_subparsers(dest="cmd", required=True)
    vet = sub.add_parser("veto")
    vet.add_argument("--results", default="/logs/verifier/grader/results.json")
    rep = sub.add_parser("ctrf")
    rep.add_argument("--items", default="/opt/grader/items.json")
    rep.add_argument("--results", default="/logs/verifier/grader/results.json")
    rep.add_argument("--gate", default="")
    rep.add_argument("--details", default="/logs/verifier/reward-details.json")
    rep.add_argument("--out", default="/logs/verifier/ctrf.json")
    rec = sub.add_parser("record")
    rec.add_argument("--tree", required=True)
    rec.add_argument("--work", required=True)
    rec.add_argument("--out", required=True)
    rec.add_argument("--phases", nargs="*")
    rec.add_argument("--families", nargs="*")
    mer = sub.add_parser("merge")
    mer.add_argument("recordings", nargs="+")
    mer.add_argument("--out", required=True)
    gra = sub.add_parser("grade")
    gra.add_argument("--tree", default="/app")
    gra.add_argument("--statea", default="/opt/grader/statea")
    gra.add_argument("--expect", default="/opt/grader/expect")
    gra.add_argument("--work", default="/var/grader")
    gra.add_argument("--logs", default="/logs/verifier/grader")
    gra.add_argument("--out", default="/logs/verifier/grader/results.json")
    gra.add_argument("--k", type=int, default=1)
    gra.add_argument("--items", default="/opt/grader/items.json")
    # Authoring only: grade a subset of the Stage II phases; the others count as not run.
    gra.add_argument("--phases", nargs="*")
    gra.add_argument("--families", nargs="*")
    args = parser.parse_args(argv)
    os.umask(0o022)
    return {"record": record, "merge": merge, "grade": grade, "veto": veto, "ctrf": ctrf}[args.cmd](args)
