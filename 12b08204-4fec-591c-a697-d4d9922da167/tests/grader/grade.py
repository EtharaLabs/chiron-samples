"""The graded pipeline, run once per grading by tests/test.sh as root in the separate verifier.

It rebuilds the submitted tree from clean and offline against the target wheel set, decides the
Stage I compiled items S1 to S10, replays the Stage II groups G01 to G17 and the Stage III families
against the State A answers recorded in /opt/grader/data, reads K1 to K4 from the graded server
configuration and K5 from the command-line help, and writes /logs/verifier/graded.json and
/logs/verifier/ctrf.json. Every process
that runs submitted code runs as the unprivileged submission user, with a clean environment that
carries no grading credential. Reward Kit, run afterwards by tests/test.sh, only reads graded.json.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import client  # noqa: E402
import compare  # noqa: E402
import packet  # noqa: E402
import rig  # noqa: E402
import stage1  # noqa: E402

GRADER = Path("/opt/grader")
DATA = GRADER / "data"
LOGS = Path(os.environ.get("GRADED_LOGS", "/logs/verifier"))
WORK = Path(os.environ.get("GRADED_WORK", "/var/tmp/graded"))
APP = Path(os.environ.get("GRADED_APP", "/app"))
WHEELS = Path("/opt/wheels/target")
PYTHON = Path("/opt/python/bin/python3")
SUB_UID = SUB_GID = 2000
COPY_EXCLUDE = {".venv", "venv", "__pycache__", "build", "dist", "node_modules", ".git",
                ".pytest_cache", ".ruff_cache", ".mypy_cache"}
METRICS_DIR = Path("/tmp/promdb-argus-metrics")
S9_THRESHOLD = 0.95
VETO_WEIGHT, FAMILY_WEIGHT = 5, 3
K_WEIGHTS = {"K1": 1, "K2": 1, "K3": 3, "K4": 1, "K5": 1}
CLI_COMMANDS = ("sync-models", "refresh-issues", "scan-jenkins")
STATIC_IDS = ("S2", "S3", "S4", "S6", "S7", "S8")
PACKETS = "/var/tmp/judge-packet"
STATE_A_SUITE = DATA / "statea-suite.json"
VETO_IDS = tuple(f"S{i}" for i in range(1, 11)) + tuple(f"G{i:02d}" for i in range(1, 18))


class Pipeline:
    def __init__(self) -> None:
        self.items: dict[str, dict] = {}
        self.extra: dict[str, object] = {}
        self.tree = WORK / "tree"
        self.venv = WORK / "venv"
        self.log = LOGS / "graded"
        self.server: rig.Server | None = None
        self.port = 0
        self.db: rig.Database | None = None

    # results ------------------------------------------------------------------------------
    def put(self, item: str, passed: bool, reason: str, weight: int, kind: str, **extra) -> None:
        self.items[item] = {"pass": bool(passed), "reason": reason, "weight": weight,
                            "kind": kind, **extra}

    # submission processes -----------------------------------------------------------------
    def env(self) -> dict[str, str]:
        return {
            "PATH": f"{self.venv}/bin:/opt/python/bin:/usr/local/bin:/usr/bin:/bin",
            "HOME": "/home/submission",
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
            "TZ": "UTC",
            "PYTHONDONTWRITEBYTECODE": "1",
            "CQLENG_ALLOW_SCHEMA_MANAGEMENT": "1",
            "PROMETHEUS_MULTIPROC_DIR": str(METRICS_DIR),
            "SETUPTOOLS_SCM_PRETEND_VERSION": "0.0.0",
            "UV_CACHE_DIR": str(WORK / "uv-cache"),
            "UV_OFFLINE": "1",
            "UV_NO_INDEX": "1",
            "UV_PYTHON_DOWNLOADS": "never",
        }

    def run(self, argv: list[str], name: str, timeout: float, env: dict | None = None,
            cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        log = self.log / f"{name}.log"
        try:
            done = subprocess.run(
                argv, cwd=cwd or self.tree, env=env or self.env(), user=SUB_UID, group=SUB_GID,
                extra_groups=[], capture_output=True, text=True, timeout=timeout,
                start_new_session=True,
            )
        except subprocess.TimeoutExpired as exc:
            log.write_text(f"timeout after {timeout} s\n{exc.stdout or ''}\n{exc.stderr or ''}")
            return subprocess.CompletedProcess(argv, 124, exc.stdout or "", f"timeout after {timeout} s")
        log.write_text(f"$ {' '.join(argv)}\nexit {done.returncode}\n--- stdout\n{done.stdout}\n"
                       f"--- stderr\n{done.stderr}")
        return done

    # tree and build -----------------------------------------------------------------------
    def prepare(self) -> None:
        if WORK.exists():
            shutil.rmtree(WORK)
        WORK.mkdir(parents=True)
        os.chmod(WORK, 0o755)
        self.log.mkdir(parents=True, exist_ok=True)

        def ignore(directory: str, names: list[str]) -> set[str]:
            return {n for n in names if n in COPY_EXCLUDE or n.endswith(".egg-info")}

        shutil.copytree(APP, self.tree, symlinks=True, ignore=ignore)
        (self.tree / "argus_web.yaml").write_text(rig.CONFIG.format(keyspace=rig.KEYSPACE))
        for path in (self.tree, WORK / "uv-cache", self.venv):
            path.mkdir(exist_ok=True)
        for directory in (METRICS_DIR, Path("/var/log/argus"), Path("/var/lib/argus")):
            if directory.exists():
                shutil.rmtree(directory)
            directory.mkdir(parents=True)
        # WORK itself stays root's, so submission code can replace nothing the grader reads from it
        # (the probe, the rules, the declared-dependency snapshot); only these belong to it.
        subprocess.run(["chown", "-R", f"{SUB_UID}:{SUB_GID}", str(self.tree), str(WORK / "uv-cache"),
                        str(self.venv), str(METRICS_DIR), "/var/log/argus", "/var/lib/argus"], check=True)

    def build(self) -> tuple[str, str]:
        """S1's resolve, build and install phases; returns (failed phase, detail) or ("", "")."""
        made = self.run([str(PYTHON), "-m", "venv", str(self.venv)], "s1-venv", 120)
        if made.returncode != 0:
            return "install", "could not create the virtual environment"
        done = self.run(
            ["uv", "pip", "install", "--python", f"{self.venv}/bin/python", "--offline", "--no-index",
             "--find-links", str(WHEELS), ".[web-backend,dev]"], "s1-install", 900)
        if done.returncode == 0:
            return "", ""
        text = done.stderr + done.stdout
        lowered = text.lower()
        if "no solution found" in lowered or "because" in lowered and "unsatisfiable" in lowered \
                or "not found in the package registry" in lowered or "was not found in the provided" in lowered:
            phase = "resolve"
        elif "failed to build" in lowered or "build backend" in lowered or "failed to prepare" in lowered:
            phase = "build"
        else:
            phase = "install"
        tail = " ".join(text.strip().splitlines()[-3:])[:400]
        return phase, tail

    def installed(self) -> list[str]:
        done = self.run(["uv", "pip", "list", "--python", f"{self.venv}/bin/python", "--format", "json"],
                        "s3-installed", 120)
        try:
            return [row["name"] for row in json.loads(done.stdout)]
        except (ValueError, KeyError, TypeError):
            return []

    def python(self, code: str, name: str, timeout: float = 180) -> subprocess.CompletedProcess[str]:
        return self.run([f"{self.venv}/bin/python", "-c", code], name, timeout)

    def probe(self, *args: str, name: str, timeout: float = 300) -> dict | None:
        done = self.run([f"{self.venv}/bin/python", str(WORK / "probe.py"), *args], name, timeout)
        for line in reversed(done.stdout.splitlines()):
            if line.startswith("PROBE-RESULT "):
                return json.loads(line.split(" ", 1)[1])
        return None

    # server -------------------------------------------------------------------------------
    def start_server(self) -> str:
        if METRICS_DIR.exists():
            shutil.rmtree(METRICS_DIR)
        METRICS_DIR.mkdir()
        os.chown(METRICS_DIR, SUB_UID, SUB_GID)
        self.port = rig.free_port()
        command = [f"{self.venv}/bin/gunicorn", "--worker-class", "uvicorn.workers.UvicornWorker",
                   "--workers", "1", "--bind", f"127.0.0.1:{self.port}", "argus_backend:create_app()"]
        self.server = rig.Server(self.tree, command, rig.clock_env(self.env()),
                                 self.log / "server.log")
        self.server.prefix = []
        return self._spawn()

    def _spawn(self) -> str:
        assert self.server is not None
        server = self.server
        with server.log.open("ab") as out:
            server.process = subprocess.Popen(
                server.command, cwd=server.tree, env=server.env, stdout=out,
                stderr=subprocess.STDOUT, user=SUB_UID, group=SUB_GID, extra_groups=[],
                start_new_session=True,
            )
        return _wait_ready(server, self.port)

    def stop_server(self) -> None:
        if self.server is not None:
            self.server.stop()
            subprocess.run(["pkill", "-9", "-u", str(SUB_UID)], capture_output=True)
            time.sleep(1)

    # stages -------------------------------------------------------------------------------
    def replay(self, case: dict, known: frozenset[str], window: compare.Window) -> tuple[bool, str]:
        exchanges = client.run_case(self.port, case)
        mapper = compare.Mapper(known=known, window=window)
        got = [compare.reduce(x, mapper, case.get("profile", "main")) for x in exchanges]
        for index, (want, have) in enumerate(zip(case["expect"], got, strict=False)):
            reason = compare.diff(want, have)
            if reason:
                step = case["steps"][index]
                return False, f"{case['id']} step {index} {step.get('method', 'GET')} {step['path']}: {reason}"
        if len(got) != len(case["expect"]):
            return False, f"{case['id']}: {len(got)} exchanges, {len(case['expect'])} recorded"
        return True, ""


def _wait_ready(server: rig.Server, port: int, seconds: float = 60) -> str:
    import http.client

    deadline = time.time() + seconds
    status = None
    while time.time() < deadline:
        if server.process is not None and server.process.poll() is not None:
            return f"server exited with {server.process.returncode}"
        try:
            connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
            connection.request("GET", "/auth/login", headers={"Host": client.FIXED_HOST})
            status = connection.getresponse().status
            connection.close()
            if status == 200:
                return ""
        except OSError:
            pass
        time.sleep(0.5)
    return f"no HTTP 200 on GET /auth/login within {seconds} s (last status {status})"


def state_a_suite() -> tuple[Path, list[str]]:
    """The original suite and its collected test ids, held in one packed record (path -> exact
    text) and written out into the grader's root-owned work directory on first use."""
    record = json.loads(STATE_A_SUITE.read_text(encoding="utf-8"))
    target = WORK / "statea-suite"
    if not target.is_dir():
        for relative, text in record["files"].items():
            path = (target / relative).resolve()
            if not path.is_relative_to(target.resolve()):
                raise ValueError(f"suite path escapes its directory: {relative}")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="")
    return target, record["ids"]


def window() -> compare.Window:
    from datetime import UTC, datetime

    start = datetime.fromisoformat(rig.FIXED_NOW).replace(tzinfo=UTC).timestamp()
    return compare.Window(start - 60, start + 6 * 3600)


def grade() -> dict:
    p = Pipeline()
    p.prepare()
    shutil.copy(GRADER / "probe.py", WORK / "probe.py")
    os.chmod(WORK / "probe.py", 0o644)
    started = time.time()
    fixture = json.loads((DATA / "fixture.json").read_text())
    known = frozenset((DATA / "known_ids.txt").read_text().split())
    stage2 = [json.loads(path.read_text()) for path in sorted((DATA / "stage2").glob("G*.json"))]
    families = json.loads((DATA / "stage3" / "families.json").read_text())["families"]
    rules = DATA / "statea-rules.json"
    shutil.copy(rules, WORK / "rules.json")
    os.chmod(WORK / "rules.json", 0o644)

    # static Stage I over the rebuilt tree --------------------------------------------------
    corpus = GRADER / "retired"
    baseline = {(row[0], row[1]) for row in json.loads((DATA / "s8-baseline.json").read_text())}
    ok, why = stage1.s2_retired_imports(p.tree)
    p.put("S2", ok, why, VETO_WEIGHT, "veto")
    ok, why = stage1.s4_retired_launch(p.tree)
    p.put("S4", ok, why, VETO_WEIGHT, "veto")
    ok, why = stage1.s6_no_bridge(p.tree)
    p.put("S6", ok, why, VETO_WEIGHT, "veto")
    ok, why, shares = stage1.s7_transplant(p.tree, corpus)
    p.put("S7", ok, why, VETO_WEIGHT, "veto", shares=shares)
    ok, why = stage1.s8_verifier_unaware(p.tree, baseline)
    p.put("S8", ok, why, VETO_WEIGHT, "veto")

    # Evidence the judge and S3 read is taken now, before any submission code runs (its build
    # backend, CLI, server or test collection), so nothing the submission executes can rewrite
    # the bytes they read.
    packets = Path(PACKETS)
    shutil.rmtree(packets, ignore_errors=True)
    packet.write(packet.app_packet(p.tree), packets / "app", "app")
    packet.write(packet.tests_packet(state_a_suite()[0], p.tree), packets / "tests", "tests")
    declared = WORK / "declared"
    shutil.rmtree(declared, ignore_errors=True)
    declared.mkdir()
    for name in ("pyproject.toml", "uv.lock"):
        if (p.tree / name).is_file() and not (p.tree / name).is_symlink():
            shutil.copy(p.tree / name, declared / name)

    # database and build ---------------------------------------------------------------------
    rig.start_upstream()
    p.db = rig.Database(WORK / "db", p.log / "scylla.log")
    try:
        p.db.start()
        db_error = ""
    except Exception as exc:  # an infrastructure fault, not the submission's
        db_error = f"database did not start: {exc}"
    phase, detail = p.build() if not db_error else ("infrastructure", db_error)
    installed = p.installed() if not phase else []
    ok, why = stage1.s3_retired_dependencies(declared, installed)
    p.put("S3", ok, why, VETO_WEIGHT, "veto")

    s1_reason = f"{phase}: {detail}" if phase else ""
    if not phase:
        done = p.python("import argus_backend", "s1-import")
        if done.returncode != 0:
            s1_reason = "import: " + " ".join(done.stderr.strip().splitlines()[-2:])[:400]

    # G17 schema-sync on an empty keyspace -----------------------------------------------------
    schema_ok, schema_reason = False, "not reached: " + (s1_reason or "no database")
    if not s1_reason and p.db.session is not None:
        p.db.create_keyspace()
        done = p.run([f"{p.venv}/bin/python", "-m", "argus.backend.cli", "sync-models"], "g17-cli", 900)
        if done.returncode != 0:
            schema_reason = "the graded CLI failed: " + " ".join(done.stderr.strip().splitlines()[-2:])[:300]
        else:
            described = _describe(p.db.describe())
            expected = _describe((DATA / "schema.cql").read_text())
            schema_ok = sorted(_statements(described)) == sorted(_statements(expected))
            schema_reason = "" if schema_ok else _first_difference(_statements(expected), _statements(described))
    p.put("G17", schema_ok, schema_reason or "schema matches State A", VETO_WEIGHT, "veto")

    # fixture and serve ------------------------------------------------------------------------
    fixture_error = ""
    if not s1_reason:
        try:
            p.db.restore(fixture)
        except Exception as exc:
            fixture_error = f"the fixture does not load into the synchronised schema: {exc}"[:400]
        done = p.python("import argus_backend; argus_backend.create_app()", "s1-create-app")
        if done.returncode != 0:
            s1_reason = "import: create_app() raised " + " ".join(done.stderr.strip().splitlines()[-2:])[:300]
    if not s1_reason:
        started_error = p.start_server()
        if started_error:
            s1_reason = f"start: {started_error}"
    p.put("S1", not s1_reason, s1_reason or "rebuilt offline against the target set and served",
          VETO_WEIGHT, "veto")

    # S5, S10 and K1 to K4 through probes, K5 from the CLI help ---------------------------------
    if not phase:
        routes = p.probe("routes", str(WORK / "rules.json"), name="s5-routes")
        if routes is None:
            p.put("S5", False, "create_app() could not be walked", VETO_WEIGHT, "veto")
        elif not routes["fastapi"]:
            p.put("S5", False, f"create_app() returns {routes['type']}, not fastapi.FastAPI", VETO_WEIGHT, "veto")
        elif routes["foreign_mounts"]:
            p.put("S5", False, f"mount wraps a foreign application: {routes['foreign_mounts'][0]}",
                  VETO_WEIGHT, "veto")
        else:
            p.put("S5", True, "create_app() returns a FastAPI application with no foreign mount, "
                  "served through UvicornWorker", VETO_WEIGHT, "veto")
        if routes is None:
            p.put("S10", False, "create_app() could not be walked", VETO_WEIGHT, "veto")
        elif routes["unmatched"]:
            p.put("S10", False, f"{len(routes['unmatched'])} State A rules not served by the new stack, "
                  f"first {routes['unmatched'][0]}", VETO_WEIGHT, "veto")
        else:
            p.put("S10", True, "every State A rule is matched by an APIRoute or the static mount",
                  VETO_WEIGHT, "veto")
        config = p.probe("server-config", name="k-server-config")
        _k_items(p, config)
        done = p.run([f"{p.venv}/bin/python", "-m", "argus.backend.cli", "--help"], "k5-cli-help", 120)
        listed = set(re.findall(r"[\w-]+", done.stdout))
        missing = [c for c in CLI_COMMANDS if c not in listed]
        p.put("K5", done.returncode == 0 and not missing,
              f"cli help exited {done.returncode}" + (f", missing {', '.join(missing)}" if missing else ""),
              K_WEIGHTS["K5"], "quality")
    else:
        p.put("S5", False, f"not reached: S1 {s1_reason}", VETO_WEIGHT, "veto")
        p.put("S10", False, f"not reached: S1 {s1_reason}", VETO_WEIGHT, "veto")
        _k_items(p, None)
        p.put("K5", False, f"not reached: S1 {s1_reason}", K_WEIGHTS["K5"], "quality")

    # S9 test retention -------------------------------------------------------------------------
    baseline_ids = state_a_suite()[1]
    baseline_skips = set(json.loads((DATA / "s9-skips.json").read_text()))
    if not phase:
        done = p.run([f"{p.venv}/bin/python", "-m", "pytest", "--collect-only", "-q",
                      "-p", "no:cacheprovider"], "s9-collect", 900)
        collected = {line.strip() for line in done.stdout.splitlines() if "::" in line}
        ok, why, share = stage1.s9_tests_retained(collected, baseline_ids,
                                                  stage1.module_skips(p.tree), baseline_skips,
                                                  S9_THRESHOLD)
        p.put("S9", ok, why, VETO_WEIGHT, "veto", share=share)
    else:
        p.put("S9", False, f"not reached: S1 {s1_reason}", VETO_WEIGHT, "veto")

    # Stage II groups G01 to G16 ------------------------------------------------------------------
    serving = not s1_reason and not fixture_error
    clock = window()
    for group in stage2:
        gid = group["group"]
        if not serving:
            p.put(gid, False, f"not reached: {fixture_error or 'S1 ' + s1_reason}", VETO_WEIGHT, "veto")
            continue
        if group.get("fresh_server"):
            p.stop_server()
            error = p.start_server()
            if error:
                p.put(gid, False, f"server restart failed: {error}", VETO_WEIGHT, "veto")
                serving = False
                continue
        failures = []
        for case in group["cases"]:
            try:
                p.db.restore(fixture)
            except Exception as exc:
                failures.append(f"{case['id']}: fixture restore failed: {exc}"[:300])
                break
            ok, why = p.replay(case, known, clock)
            if not ok:
                failures.append(why)
        p.put(gid, not failures, failures[0] if failures else f"{len(group['cases'])} cases agree",
              VETO_WEIGHT, "veto", failed_cases=len(failures), cases=len(group["cases"]))

    # Stage III families ---------------------------------------------------------------------------
    for family in families:
        fid = family["id"]
        if not serving:
            p.put(fid, False, "not reached", FAMILY_WEIGHT, "family")
            continue
        if family.get("fresh_server"):
            # a family that reads the metrics exposition starts from a server with no history
            p.stop_server()
            error = p.start_server()
            if error:
                p.put(fid, False, f"server restart failed: {error}", FAMILY_WEIGHT, "family")
                serving = False
                continue
        try:
            p.db.restore(fixture)
            ok, why = p.replay(family, known, clock)
        except Exception as exc:
            ok, why = False, f"replay failed: {exc}"[:300]
        p.put(fid, ok, why or "agrees with State A", FAMILY_WEIGHT, "family")

    p.stop_server()
    if p.db is not None:
        p.db.stop()
    p.extra["seconds"] = round(time.time() - started, 1)
    p.extra["infrastructure"] = db_error
    return {"items": p.items, "extra": p.extra}


def _k_items(p: Pipeline, config: dict | None) -> None:
    if config is None:
        for item in ("K1", "K2", "K3", "K4"):
            p.put(item, False, "the server configuration could not be loaded", K_WEIGHTS[item], "quality")
        return
    p.put("K1", config["max_requests"] == 65535,
          f"max_requests resolves to {config['max_requests']}", K_WEIGHTS["K1"], "quality")
    p.put("K2", config["graceful_timeout"] == 60,
          f"graceful_timeout resolves to {config['graceful_timeout']}", K_WEIGHTS["K2"], "quality")
    configured = str(config.get("configured_worker_class") or "")
    p.put("K4", configured.startswith("uvicorn.workers.") or configured.startswith("uvicorn_worker."),
          f"the repository's configuration loads worker_class {configured or 'unset'}",
          K_WEIGHTS["K4"], "quality")
    keep = {"gauge_livesum_4102.db", "gauge_liveall_4102.db", "counter_4101.db", "counter_4102.db"}
    outcomes = []
    for hook, seen in (config.get("exit_hooks") or {}).items():
        left = set(seen["left"])
        cleaned = not any(name.endswith("_4101.db") and "live" in name for name in left)
        if cleaned and keep <= left and not seen["error"]:
            outcomes = [f"{hook} removes the exiting worker's live-gauge files and keeps the others"]
            break
        outcomes.append(f"{hook} left {sorted(left)}{' after ' + seen['error'] if seen['error'] else ''}")
    p.put("K3", bool(outcomes) and outcomes[0].endswith("keeps the others"),
          "; ".join(outcomes) or "no exit hook reached", K_WEIGHTS["K3"], "quality")


def _describe(text: str) -> list[str]:
    return [" ".join(line.split()) for line in text.splitlines() if line.strip()]


def _statements(lines: list[str]) -> list[str]:
    # Operator measurement patch 2026-11-04: a schema dump's statement order is not behaviour a
    # port controls, so compare the set of statements. Line order inside a statement still counts.
    out: list[str] = []
    cur: list[str] = []
    for line in lines:
        cur.append(line)
        if line.endswith(";"):
            out.append(" ".join(cur))
            cur = []
    if cur:
        out.append(" ".join(cur))
    return out


def _first_difference(expected: list[str], actual: list[str]) -> str:
    missing = [line for line in expected if line not in actual]
    extra = [line for line in actual if line not in expected]
    if missing:
        return f"schema differs from State A: missing {missing[0][:200]}"
    if extra:
        return f"schema differs from State A: adds {extra[0][:200]}"
    return "schema differs from State A in statement order"


def ctrf(items: dict[str, dict], cheat_ids: list[str]) -> dict:
    tests = []
    for name, item in items.items():
        tests.append({"name": name, "status": "passed" if item["pass"] else "failed",
                      "message": item["reason"], "extra": {"weight": item["weight"],
                                                           "kind": item["kind"],
                                                           "cheat_check": name in cheat_ids}})
    passed = sum(1 for t in tests if t["status"] == "passed")
    return {"results": {"tool": {"name": "argus-graded-pipeline"},
                        "summary": {"tests": len(tests), "passed": passed,
                                    "failed": len(tests) - passed, "pending": 0, "skipped": 0,
                                    "other": 0, "start": 0, "stop": 0},
                        "tests": tests}}


def vetoed(result: dict) -> str:
    """The first failed or undecided Stage I or Stage II item, or "" when both vetoes clear."""
    for item in VETO_IDS:
        verdict = result["items"].get(item)
        if not verdict or verdict.get("pass") is not True:
            return f"{item}: {verdict['reason'] if verdict else 'not decided'}"
    return ""


def write(result: dict) -> None:
    cheat_ids = json.loads((GRADER / "items.json").read_text())["cheat_checks"]
    for name in cheat_ids:
        result["items"].setdefault(name, {"pass": False, "reason": "not decided: the pipeline failed",
                                          "weight": VETO_WEIGHT, "kind": "veto"})
    for row in json.loads((GRADER / "items.json").read_text())["items"]:
        result["items"].setdefault(row["id"], {"pass": False, "reason": "not decided by the pipeline",
                                               "weight": row["weight"], "kind": row["stage"]})
        verdict = result["items"][row["id"]]
        decided = not str(verdict["reason"]).startswith(("not decided", "not reached"))
        # the criterion as phrased: a negated item's criterion is the act, so it is observed
        # exactly when the item failed; an undecided item has no observation and scores 0
        verdict["observed"] = (not verdict["pass"] if row["negate"] else verdict["pass"]) if decided else None
    reason = vetoed(result)
    result["vetoed"] = bool(reason)
    result["veto_reason"] = reason
    (LOGS / "graded.json").write_text(json.dumps(result, indent=1, sort_keys=True))
    (LOGS / "ctrf.json").write_text(json.dumps(ctrf(result["items"], cheat_ids), indent=1))


def veto_judge(target: Path) -> None:
    """The vetoed route's stand-in for tests/judge.toml: every q criterion at its weight, at 0,
    each naming every Stage I and Stage II item that failed or went undecided."""
    register = json.loads((GRADER / "items.json").read_text())["q"]
    try:
        items = json.loads((LOGS / "graded.json").read_text()).get("items", {})
        vetoes = ", ".join(item for item in VETO_IDS
                           if (items.get(item) or {}).get("pass") is not True) or "unattributed"
    except (OSError, ValueError):
        vetoes = "unattributed: no graded report"
    lines = ["import rewardkit as rk", "", "",
             '@rk.criterion(description="{item}: not judged, the Stage I or Stage II veto failed on {vetoes}")',
             "def vetoed_criterion(workspace, item: str, vetoes: str) -> float:", "    return 0.0", "", ""]
    lines += [f"rk.vetoed_criterion({row['id']!r}, {vetoes!r}, name={row['id']!r}, weight={row['weight']})"
              for row in register]
    target.write_text("\n".join(lines) + "\n")


def main() -> int:
    LOGS.mkdir(parents=True, exist_ok=True)
    args = sys.argv[1:]
    if args[:1] == ["--cleared"]:
        result = json.loads((LOGS / "graded.json").read_text())
        return 0 if not result.get("vetoed", True) else 1
    if args[:1] == ["--veto-judge"]:
        veto_judge(Path(args[1]))
        return 0
    if args[:1] == ["--fallback"]:
        write({"items": {}, "extra": {"pipeline_error": " ".join(args[1:])}})
        return 0
    try:
        result = grade()
    except Exception as exc:  # the pipeline itself failed: every item is undecided and scores 0
        import traceback

        (LOGS / "graded-error.txt").write_text(traceback.format_exc())
        result = {"items": {}, "extra": {"pipeline_error": f"{type(exc).__name__}: {exc}"}}
    write(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
