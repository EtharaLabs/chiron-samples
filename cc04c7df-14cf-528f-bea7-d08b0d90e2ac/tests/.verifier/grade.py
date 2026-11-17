"""The graded pipeline, run once per verification by tests/test.sh.

It decides every compiled item (Stage I, Stage II, Stage III) over the collected /app tree and
writes the verdicts that tests/test_output.py relates to Reward Kit criteria, plus the CTRF report.
Nothing here calls a judge. Layout (grader-owned, not visible to the application):

  /tests/.data/items.json        item ids, stages and weights
  /tests/.data/retired.json      retired coordinates, packages, descriptors, grader names
  /tests/.data/stage2/<id>.json  frozen Stage II groups: replay steps plus State A's recorded cases
  /tests/.data/stage3/<id>.json  frozen Stage III families, same shape
  /tests/.data/statea-classes.sha256, /tests/.data/init/*.sql
  /opt/grader-corpus/*.jar       transplant corpus (retired release sources)
"""

from __future__ import annotations

import difflib
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import time
import traceback
import zipfile
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import appctl  # noqa: E402
import replay  # noqa: E402
import scans  # noqa: E402
import transplant  # noqa: E402

TESTS = Path(os.environ.get("GRADER_TESTS", "/tests"))
DATA = TESTS / ".data"
WORKSPACE = Path(os.environ.get("GRADER_WORKSPACE", "/app"))
LOGS = Path(os.environ.get("GRADER_LOGS", "/logs/verifier"))
OUT = LOGS / "grade"
GRADE = Path("/grade")
SRC = GRADE / "src"
M2 = GRADE / "m2"
HOME = GRADE / "home"
RUN = GRADE / "run"
JAR = GRADE / "app.jar"
STATE = Path("/var/lib/grader")
PROBE_ROOT = Path("/var/lib/grader-probe")
PROBE_JAR = "/opt/grader-probe/probe.jar"
CORPUS = Path("/opt/grader-corpus")
JAVA = "/opt/jdk-17/bin/java"
MAVEN = "/opt/maven/bin/mvn"
SETTINGS = "/opt/m2/settings-offline.xml"
TARGET_SET = Path("/opt/m2/target")
DEP_PLUGIN = "org.apache.maven.plugins:maven-dependency-plugin:3.8.1:list"
BUILD_SECONDS = 1200
READY_SECONDS = 180
STRIP_DIRS = {".git", "target", ".m2", ".idea", ".vscode", ".settings", "node_modules", "logs", ".gradle"}
STRIP_SUFFIXES = {".jar", ".war", ".ear", ".class", ".log"}
STRIP_NAMES = {".project", ".classpath", ".factorypath"}
JDK_ENV = {"JAVA_HOME": "/opt/jdk-17", "PATH": "/opt/jdk-17/bin:/opt/maven/bin:/usr/bin:/bin"}

verdict: dict[str, dict[str, Any]] = {}
requests_by_start: dict[str, list[tuple[int, str]]] = {}
current_run: list[str] = [""]


def put(item: str, passed: bool, reason: str) -> None:
    verdict[item] = {"passed": bool(passed), "reason": reason[:4000]}


def log(message: str) -> None:
    print(f"[grade {time.strftime('%H:%M:%S')}] {message}", flush=True)


# ------------------------------------------------------------------ collection and graded build

collect_errors: list[str] = []


def collect() -> None:
    """Copy the collected tree, as root, into a grader-owned tree without build outputs, archives,
    compiled classes or IDE state. Root reads every file whatever its owner or mode (an agent may
    leave files at 0600); the copy is written 0644/0755 and handed to the unprivileged grading user
    before any submitted code runs. A file that cannot be copied is recorded, never fatal."""
    os.umask(0o022)
    if SRC.exists():
        shutil.rmtree(SRC)
    for path in [SRC]:
        path.mkdir(parents=True)
    for root, dirs, files in os.walk(WORKSPACE, followlinks=False,
                                     onerror=lambda e: collect_errors.append(f"{e.filename}: {e.strerror}")):
        rel = Path(root).relative_to(WORKSPACE)
        dirs[:] = [d for d in dirs if d not in STRIP_DIRS and not os.path.islink(os.path.join(root, d))]
        (SRC / rel).mkdir(parents=True, exist_ok=True)
        for name in files:
            src = Path(root) / name
            if src.is_symlink() or not src.is_file():
                continue
            if name in STRIP_NAMES or src.suffix.lower() in STRIP_SUFFIXES:
                continue
            try:
                shutil.copyfile(src, SRC / rel / name)
            except OSError as error:
                collect_errors.append(f"{rel / name}: {error.strerror or error}")


def _as_app(argv: list[str], log_path: Path, timeout: float, cwd: Path) -> int:
    env = {**JDK_ENV, "HOME": str(HOME), "LANG": "C.UTF-8", "MAVEN_OPTS": "-Xmx2g"}
    command = [appctl.RUNUSER, "-u", "app", "--", "/usr/bin/env", *[f"{k}={v}" for k, v in env.items()], *argv]
    with log_path.open("ab") as out:
        try:
            return subprocess.run(command, cwd=cwd, stdout=out, stderr=subprocess.STDOUT, timeout=timeout,
                                  env=env).returncode
        except subprocess.TimeoutExpired:
            out.write(b"\n[grader] build step timed out\n")
            return 124


def _first_error(log_path: Path) -> str:
    try:
        lines = log_path.read_text("utf-8", "replace").splitlines()
    except OSError:
        return "no build log"
    for line in lines:
        if line.startswith("[ERROR]") and len(line) > 8:
            return line[:500]
    return (lines[-1] if lines else "empty build log")[:500]


def _boot_jars() -> list[Path]:
    found = []
    for jar in sorted(SRC.rglob("target/*.jar")):
        try:
            with zipfile.ZipFile(jar) as archive:
                manifest = archive.read("META-INF/MANIFEST.MF").decode("utf-8", "replace")
        except (KeyError, zipfile.BadZipFile, OSError):
            continue
        fields = dict(re.findall(r"^([A-Za-z-]+): *(.*?)\r?$", manifest, re.M))
        if fields.get("Main-Class", "").startswith("org.springframework.boot.loader.") and fields.get("Start-Class"):
            found.append(jar)
    return found


def start_class(jar: Path) -> str:
    with zipfile.ZipFile(jar) as archive:
        manifest = archive.read("META-INF/MANIFEST.MF").decode("utf-8", "replace")
    return dict(re.findall(r"^([A-Za-z-]+): *(.*?)\r?$", manifest, re.M)).get("Start-Class", "")


def build() -> tuple[bool, str, list[tuple[str, str]]]:
    for path in (M2, HOME, RUN):
        if path.exists():
            shutil.rmtree(path)
    shutil.copytree(TARGET_SET, M2, symlinks=False)
    HOME.mkdir(parents=True)
    RUN.mkdir(parents=True)
    subprocess.run(["chown", "-R", "app:app", str(GRADE)], check=True)
    build_log = LOGS / "build.log"
    build_log.unlink(missing_ok=True)
    base = [MAVEN, "-s", SETTINGS, "-gs", SETTINGS, f"-Dmaven.repo.local={M2}", "-o", "-B", "-ntp"]
    deps_file = GRADE / "deps.txt"
    deps_file.unlink(missing_ok=True)
    # one reactor run: package, then the resolved graph of every module against the reactor's own
    # artifacts (a separate dependency:list run cannot see unpublished sibling modules offline)
    log("graded build: mvn package + dependency:list")
    code = _as_app([*base, "-DskipTests", "package", DEP_PLUGIN, f"-DoutputFile={deps_file}",
                    "-DappendOutput=true", "-DoutputAbsoluteArtifactFilename=false"],
                   build_log, BUILD_SECONDS, SRC)
    if code != 0:
        return False, f"build failed (mvn exit {code}): {_first_error(build_log)}", []
    jars = _boot_jars()
    if len(jars) != 1:
        names = ", ".join(str(j.relative_to(SRC)) for j in jars) or "none"
        return False, f"expected exactly one Spring Boot executable jar, found {len(jars)}: {names}", []
    shutil.copyfile(jars[0], JAR)
    os.chmod(JAR, 0o644)
    deps = []
    text = deps_file.read_text("utf-8", "replace") if deps_file.exists() else ""
    shutil.copyfile(deps_file, LOGS / "resolved-dependencies.txt") if deps_file.exists() else None
    for line in text.splitlines():
        parts = line.strip().split(":")
        if len(parts) >= 4 and not line.startswith("The following"):
            deps.append((parts[0], parts[1]))
    return True, f"built {jars[0].relative_to(SRC)}", deps


# ------------------------------------------------------------------ runtime

def _wrap_send() -> None:
    original = replay.send

    def send(base: str, spec: dict[str, Any], timeout: float = 60.0) -> replay.Response:
        if base == appctl.BASE:
            path = spec.get("path", "")
            requests_by_start.setdefault(current_run[0], []).append((current_app[0].starts, path))
        return original(base, spec, timeout)

    replay.send = send


current_app: list[Any] = [None]


def new_app(run_id: str) -> appctl.App:
    probe_dir = PROBE_ROOT / run_id

    def on_start(n: int) -> None:
        target = probe_dir / str(n)
        target.mkdir(parents=True, exist_ok=True)
        shutil.chown(probe_dir, "app", "app")
        shutil.chown(target, "app", "app")

    argv = [JAVA, f"-javaagent:{PROBE_JAR}={probe_dir}/{{start}}", "-jar", str(JAR)]
    (STATE / "trace").mkdir(parents=True, exist_ok=True)
    app = appctl.App(argv, user="app", workdir=RUN, log=OUT / "app" / f"{run_id}.log",
                     trace=STATE / "trace" / f"{run_id}.strace", extra_env={"JAVA_HOME": "/opt/jdk-17"},
                     on_start=on_start)
    current_app[0] = app
    current_run[0] = run_id
    return app


# Every start runs on the replay clock of appctl.py, a restart on the same database included, as State A
# was recorded, so a row written after a restart can land on the very millisecond of an earlier start's
# row for the same user, and the table's (user_id, utc_timestamp) key refuses it (tests/.data/init).
# A unit that disagrees while the grader's own PostgreSQL log shows that refusal is replayed once more
# from a fresh database; a second disagreement stands.
PG_LOG = appctl.PG_DATA.parent / "pg.log"
CLOCK_COLLISION = re.compile(r'duplicate key value violates unique constraint '
                             r'"(?:user_initial_stats|user_offset_tc_stats|user_total_stats|user_tc_stats_hourly)_pkey"')


def replay_unit(data: dict[str, Any], kind: str) -> dict[str, Any]:
    mark = PG_LOG.stat().st_size if PG_LOG.exists() else 0
    result = _replay_once(data, kind, data["id"])
    if result.get("agrees") or not _clock_collision(mark):
        return result
    first = result.get("error") or [c["case"] for c in result.get("cases", []) if not c.get("agrees")][:8]
    log(f"{kind} {data['id']}: key collision from the replay clock in pg.log; replaying once more")
    result = _replay_once(data, kind, f"{data['id']}.retry")
    result["retried"] = f"replayed twice: the first replay hit a replay-clock key collision (pg.log) and disagreed on {first}"
    return result


def _clock_collision(mark: int) -> bool:
    try:
        with PG_LOG.open("rb") as handle:
            handle.seek(mark)
            return bool(CLOCK_COLLISION.search(handle.read().decode("utf-8", "replace")))
    except OSError:
        return False


def _replay_once(data: dict[str, Any], kind: str, run_id: str) -> dict[str, Any]:
    appctl.db_fresh(DATA / "init")
    appctl.stub_reset()
    app = new_app(run_id)
    try:
        if not app.start(ready_seconds=READY_SECONDS):
            return {"id": data["id"], "agrees": False, "error": "application did not answer within 180 s", "cases": []}
        return replay.run(data, base=appctl.BASE, stub=appctl.STUB, psql=replay.psql_runner(appctl.DSN),
                          restart=app.restart, reset=lambda env: app.reset(DATA / "init", env),
                          clock_offset=lambda: app.clock_offset, recorded=data)
    except Exception as error:  # noqa: BLE001 - an undecidable replay fails its item with the reason
        return {"id": data["id"], "agrees": False, "error": f"{kind} replay failed: {error!r}", "cases": [],
                "trace": traceback.format_exc()[-2000:]}
    finally:
        app.stop()


def smoke_start() -> tuple[bool, str]:
    appctl.db_fresh(DATA / "init")
    appctl.stub_reset()
    app = new_app("graded-start")
    t0 = time.time()
    try:
        ok = app.start(ready_seconds=READY_SECONDS)
        return ok, (f"started and answered GET /folding/hardware in {time.time() - t0:.1f} s" if ok
                    else "the built jar did not answer GET /folding/hardware with 200 within 180 s")
    finally:
        app.stop()


STRACE_LINE = re.compile(r"^(\d+)\s+(\w+)\((.*)$")
INET = re.compile(r'sa_family=AF_INET6?, sin6?_port=htons\((\d+)\).*?(?:inet_addr\("([^"]+)"\)|inet_pton\(AF_INET6, "([^"]+)")')
UNIX_PATH = re.compile(r'sa_family=AF_UNIX, sun_path=@?"([^"]*)"')
EXEC = re.compile(r'^execve\("([^"]+)"')
OPEN_PATH = re.compile(r'"(/[^"]*)"')


CLONE_RESULT = re.compile(r"^(\d+)\s+(?:(?:clone3?|v?fork)\(|<\.\.\. (?:clone3?|v?fork) resumed>).*= (\d+)\s*$")
RETIRED_MARKER = re.compile(r"wildfly|jboss|standalone\.(sh|conf|xml)|resteasy|weld|glassfish|payara|tomee"
                            r"|/opt/jdk-11|/opt/m2/original|javaee", re.I)


def analyse_traces(spec: dict[str, Any]) -> dict[str, list[str]]:
    """What the application's process tree did that reaches the retired stack, from strace.

    A finding needs the retired stack on the far side: a process whose executable or arguments
    name it, or a connection to a socket such a process listens on. The application's own
    traffic (its listener, PostgreSQL, the stub, its own address, syslog) is no finding, and an
    exec strace could not decode is recorded but proves nothing."""
    out: dict[str, list[str]] = {"spawn": [], "connect": [], "open": [], "observed": []}
    for trace in sorted((STATE / "trace").glob("*.strace*")):
        name = trace.name
        retired_pids: set[str] = set()
        retired_ports: set[str] = set()
        binds: dict[tuple[str, str], str] = {}
        connects: list[tuple[str, str]] = []
        for raw in trace.read_text("utf-8", "replace").splitlines():
            child = CLONE_RESULT.match(raw)
            if child and child.group(1) in retired_pids:
                retired_pids.add(child.group(2))
            match = STRACE_LINE.match(raw)
            if not match:
                continue
            pid, call, rest = match.groups()
            if call == "execve":
                exe = EXEC.match(f"execve({rest}")
                if exe is None:
                    out["observed"].append(f"{name}: an exec strace could not decode")
                elif RETIRED_MARKER.search(rest.split("], ", 1)[0]):
                    retired_pids.add(pid)
                    out["spawn"].append(f"{name}: spawned {exe.group(1)} naming the retired stack")
            elif call == "bind":
                inet = INET.search(rest)
                if inet:
                    binds[(pid, rest.split(",", 1)[0])] = inet.group(1)
            elif call == "listen":
                port = binds.get((pid, rest.split(",", 1)[0]))
                if port and pid in retired_pids:
                    retired_ports.add(port)
            elif call == "connect":
                inet = INET.search(rest)
                if inet:
                    connects.append((inet.group(1), f"{inet.group(2) or inet.group(3)}:{inet.group(1)}"))
            elif call in ("open", "openat"):
                opened = OPEN_PATH.search(rest)
                if opened and re.match(r"^/(tests|logs)(/|$)", opened.group(1)):
                    out["open"].append(f"{name}: opened {opened.group(1)}")
        for port, where in connects:
            if port in retired_ports:
                out["connect"].append(f"{name}: connect to {where}, served by a retired-stack process")
    for key in out:
        out[key] = sorted(set(out[key]))[:30]
    return out


def analyse_probe(spec: dict[str, Any], start_cls: str) -> dict[str, list[str]]:
    out = {"serves": [], "foreign": [], "retired_loaded": []}
    jar_url = f"file:{JAR}"
    allowed_prefixes = (f"jar:{jar_url}!/BOOT-INF/classes!/", f"jar:{jar_url}!/BOOT-INF/lib/")
    retired = tuple(p + "." for p in spec["retired_packages"])
    groups = sorted({c.split(":", 1)[0].replace("*", "").strip(".") for c in spec["retired_coordinates"]
                     if "." in c.split(":", 1)[0]})
    retired_paths = re.compile("|".join(re.escape("/" + g.strip(".").replace(".", "/") + "/") for g in groups)
                               or r"(?!)")
    saw_dispatcher = saw_start = False
    starts = 0
    for run_dir in sorted(PROBE_ROOT.glob("*")):
        for start_dir in sorted(run_dir.glob("*")):
            starts += 1
            classes = start_dir / "classes.tsv"
            if not classes.exists():
                out["serves"].append(f"{run_dir.name} start {start_dir.name}: the probe recorded no loaded classes")
                continue
            for line in classes.read_text("utf-8", "replace").splitlines():
                name, _, where = line.partition("\t")
                if name == "org.springframework.web.servlet.DispatcherServlet":
                    saw_dispatcher = True
                if name == start_cls:
                    saw_start = True
                if name.startswith(retired):
                    out["retired_loaded"].append(f"{name} ({where or 'bootstrap'})")
                if where and where not in (jar_url, f"file:{PROBE_JAR}") and not where.startswith(allowed_prefixes) \
                        and not where.startswith("jrt:") and (RETIRED_MARKER.search(where) or retired_paths.search(where)):
                    out["foreign"].append(f"{name} from {where}")
            dispatch = start_dir / "dispatch.tsv"
            lines = dispatch.read_text("utf-8", "replace").splitlines() if dispatch.exists() else []
            servlets = [line.partition("\t")[2] for line in lines]
            others = sorted({s for s in servlets if s != "org.springframework.web.servlet.DispatcherServlet"})
            if others:
                out["serves"].append(f"{run_dir.name} start {start_dir.name}: requests dispatched to {others}")
            paths = [line.partition("\t")[0] for line in lines]
            sent = [p for n, p in requests_by_start.get(run_dir.name, []) if str(n) == start_dir.name]
            missing = _unmatched(sent, paths)
            if missing:
                out["serves"].append(f"{run_dir.name} start {start_dir.name}: {len(missing)} replayed request(s) "
                                     f"never reached a servlet, e.g. {missing[:3]}")
    if starts == 0:
        out["serves"].append("no application start was probed")
    else:
        if not saw_dispatcher:
            out["serves"].append("org.springframework.web.servlet.DispatcherServlet was never loaded")
        if not saw_start:
            out["serves"].append(f"the Start-Class {start_cls} was never loaded")
    for key in out:
        out[key] = sorted(set(out[key]))[:30]
    return out


def _unmatched(sent: list[str], seen: list[str]) -> list[str]:
    pool: dict[str, int] = {}
    for uri in seen:
        pool[uri] = pool.get(uri, 0) + 1
    missing = []
    for path in sent:
        uri = "/folding" + path.split("?", 1)[0]
        if pool.get(uri, 0) > 0:
            pool[uri] -= 1
        else:
            missing.append(uri)
    return missing


# ------------------------------------------------------------------ verdicts and reports

def write_reports(items: list[dict[str, Any]]) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "verdicts.json").write_text(json.dumps({"items": verdict}, indent=1, sort_keys=True))
    tests = []
    for item in items:
        if item["mode"] == "judged" and item["id"] not in verdict:
            # Judged items are scored by Reward Kit after this report, on the routes that call the judge.
            tests.append({"name": item["id"], "status": "pending", "duration": 0, "suite": item["stage"],
                          "message": "judged by Reward Kit after this report on the clear route; scored 0 with no judge call on the veto route (reward-details.json)"})
            continue
        row = verdict.get(item["id"], {"passed": False, "reason": "not decided"})
        tests.append({"name": item["id"], "status": "passed" if row["passed"] else "failed",
                      "message": row["reason"], "duration": 0, "suite": item["stage"]})
    counts = {status: sum(1 for t in tests if t["status"] == status) for status in ("passed", "failed", "pending")}
    now = int(time.time() * 1000)
    report = {"results": {"tool": {"name": "folding-stats-migration-grader"},
                          "summary": {"tests": len(tests), **counts,
                                      "skipped": 0, "other": 0, "start": STARTED, "stop": now},
                          "tests": tests}}
    (LOGS / "ctrf.json").write_text(json.dumps(report, indent=1))


EVIDENCE = OUT / "evidence"
EVIDENCE_LIMIT = 2_000_000
# The judged items read the application's code and build, never its tests or front-end assets.
EVIDENCE_SUFFIXES = {".java", ".kt", ".kts", ".groovy", ".scala", ".xml", ".properties", ".yml", ".yaml",
                     ".toml", ".gradle", ".sh", ".sql", ".factories", ".imports"}


def evidence_source(rel: str) -> bool:
    name = rel.rsplit("/", 1)[-1]
    if "/META-INF/" in f"/{rel}" or name.startswith("Dockerfile"):
        return True
    return any(name.endswith(suffix) for suffix in EVIDENCE_SUFFIXES)


_STATE_A_SOURCE: dict[str, str] | None = None


def state_a_text(rel: str) -> str:
    """State A's text of one file, from the single packed source manifest (not per-file copies)."""
    global _STATE_A_SOURCE
    if _STATE_A_SOURCE is None:
        _STATE_A_SOURCE = json.loads((DATA / "statea-source.json").read_text(encoding="utf-8"))["files"]
    return _STATE_A_SOURCE.get(rel, "")


def changed_hunks(old: str, new: str) -> str:
    """A changed file as hunks against State A: numbered submission lines, unnumbered removals."""
    before, after = old.splitlines(), new.splitlines()
    lines: list[str] = []
    matcher = difflib.SequenceMatcher(None, before, after, autojunk=False)
    for group in matcher.get_grouped_opcodes(3):
        lines.append("@@")
        for tag, i1, i2, j1, j2 in group:
            if tag == "equal":
                lines += [f"{j + 1}  | {after[j]}" for j in range(j1, j2)]
                continue
            lines += [f"   -| {before[i]}" for i in range(i1, i2)]
            lines += [f"{j + 1} +| {after[j]}" for j in range(j1, j2)]
    return "".join(f"{line}\n" for line in lines)


def write_evidence() -> None:
    """The judge's evidence: every file of the submission that is new or changed against State A.

    Reward Kit's judge reads a directory one level deep, so each submission file becomes one
    numbered text file naming its submission path, with every line numbered for citation.
    Files carried over unchanged from State A are listed in the index, not inlined.
    """
    if EVIDENCE.exists():
        shutil.rmtree(EVIDENCE)
    EVIDENCE.mkdir(parents=True)
    state_a = {}
    for line in (DATA / "statea-tree.sha256").read_text().splitlines():
        digest, _, rel = line.partition("  ")
        if rel:
            state_a[rel] = digest
    present: set[str] = set()
    added: list[str] = []
    changed: list[str] = []
    for path in scans.walk(SRC):
        rel = path.relative_to(SRC).as_posix()
        present.add(rel)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if rel not in state_a:
            added.append(rel)
        elif state_a[rel] != digest:
            changed.append(rel)
    removed = sorted(set(state_a) - present)
    used, number, omitted = 0, 0, []
    for kind, rel in sorted([("added", r) for r in added] + [("changed", r) for r in changed],
                            key=lambda pair: pair[1]):
        if "/src/test/" in f"/{rel}" or not evidence_source(rel):
            omitted.append(f"{rel} (test or non-source file)")
            continue
        try:
            text = (SRC / rel).read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            omitted.append(f"{rel} (not text)")
            continue
        if kind == "added":
            body = "".join(f"{n} +| {line}\n" for n, line in enumerate(text.splitlines(), 1))
        else:
            body = changed_hunks(state_a_text(rel), text)
        block = f"=== submission file: {rel} ({kind} against State A) ===\n{body}"
        if used + len(block) > EVIDENCE_LIMIT or len(block) > 1_000_000:
            omitted.append(f"{rel} (evidence size limit)")
            continue
        number += 1
        used += len(block)
        (EVIDENCE / f"{number:04d}.txt").write_text(block, encoding="utf-8")
    index = [
        "=== evidence index: the submission tree against State A ===",
        "Each numbered evidence file holds one submission source or build file that is new or changed",
        "against State A. A new file is shown whole; a changed file is shown as hunks against State A.",
        "Every shown submission line starts with its line number in the submission file, then '+|' when",
        "the line is new or changed against State A or ' |' when it is unchanged; '-|' lines carry no",
        "number and show State A text the submission removed.",
        f"added ({len(added)}): " + ", ".join(sorted(added)),
        f"changed ({len(changed)}): " + ", ".join(sorted(changed)),
        f"removed ({len(removed)}): " + ", ".join(removed),
        f"carried over unchanged from State A: {len(present) - len(added) - len(changed)} files",
        f"not inlined ({len(omitted)}): " + ", ".join(omitted),
    ]
    (EVIDENCE / "0000.txt").write_text("\n".join(index) + "\n", encoding="utf-8")
    os.chmod(EVIDENCE, 0o755)


STARTED = int(time.time() * 1000)


def main() -> int:
    items = json.loads((DATA / "items.json").read_text())["items"]
    spec = json.loads((DATA / "retired.json").read_text())
    stage2 = [i["id"] for i in items if i["stage"] == "II"]
    stage3 = [i["id"] for i in items if i["stage"] == "III"]
    for path in (OUT, OUT / "app", OUT / "stage2", OUT / "stage3"):
        path.mkdir(parents=True, exist_ok=True)
    for path in (STATE, PROBE_ROOT):
        if path.exists():
            shutil.rmtree(path)
        path.mkdir(parents=True)
    os.chmod(STATE, 0o700)
    os.chmod(PROBE_ROOT, 0o755)

    collect()
    log("collected tree copied")
    write_evidence()
    log("judge evidence written")
    built, why, deps = build()
    jar = JAR if built else None

    retired = scans.retired_stack(SRC, deps, spec)
    if jar is not None:
        retired += scans.jar_retired(jar, spec)
    digests = set((DATA / "statea-classes.sha256").read_text().split()) - {""}
    digests = {d for d in digests if len(d) == 64}
    transplant_report = transplant.measure(str(CORPUS), str(SRC), spec["transplant"]["k"],
                                           spec["transplant"]["min_tokens"])
    (OUT / "transplant.json").write_text(json.dumps(transplant_report, indent=1))
    identical = scans.state_a_classes(SRC, jar, digests)
    static_verifier = scans.verifier_names(SRC, spec)

    runtime_ran = False
    started, start_reason = False, "not started: the graded build failed"
    results2: dict[str, dict[str, Any]] = {}
    results3: dict[str, dict[str, Any]] = {}
    if built:
        _wrap_send()
        appctl.pg_start()
        stub = appctl.stub_start(HERE / "stub.py", OUT / "stub.log")
        try:
            started, start_reason = smoke_start()
            runtime_ran = True
            if started:
                for gid in stage2:
                    log(f"Stage II {gid}")
                    results2[gid] = replay_unit(json.loads((DATA / "stage2" / f"{gid}.json").read_text()), "group")
                    (OUT / "stage2" / f"{gid}.json").write_text(json.dumps(results2[gid], indent=1))
                if not retired:
                    for fid in stage3:
                        log(f"Stage III {fid}")
                        results3[fid] = replay_unit(json.loads((DATA / "stage3" / f"{fid}.json").read_text()), "family")
                        (OUT / "stage3" / f"{fid}.json").write_text(json.dumps(results3[fid], indent=1))
        finally:
            stub.terminate()
            appctl.pg_stop()

    note = f" (collection skipped {len(collect_errors)} file(s): {'; '.join(collect_errors[:5])})" if collect_errors else ""
    put("s1_graded_build", built and started, (why if not built else start_reason) + note)
    put("s1_retired_stack_absent", not retired,
        "no retired coordinate, package or descriptor found" if not retired else "; ".join(retired[:15]))
    over = transplant_report["max_file_containment"] >= spec["transplant"]["threshold"]
    put("s1_no_transplant", not over and not identical,
        f"max_file_containment {transplant_report['max_file_containment']} against threshold "
        f"{spec['transplant']['threshold']} (largest: {transplant_report['max_file']})"
        + ("; " + "; ".join(identical[:5]) if identical else ""))
    if runtime_ran:
        traces = analyse_traces(spec)
        probe = analyse_probe(spec, start_class(JAR))
        (OUT / "runtime.json").write_text(json.dumps({"traces": traces, "probe": probe}, indent=1))
        serves = probe["serves"] + probe["retired_loaded"]
        put("s1_new_stack_serves", started and not serves,
            "every replayed request was dispatched through DispatcherServlet" if started and not serves
            else "; ".join((serves or [start_reason])[:10]))
        forwarding = traces["spawn"] + traces["connect"] + probe["foreign"]
        put("s1_no_forwarding", not forwarding,
            "no process, connection or class source reaching the retired stack" if not forwarding
            else "; ".join(forwarding[:10]))
        detect = static_verifier + traces["open"]
    else:
        put("s1_new_stack_serves", False, f"no JVM ran: {start_reason}")
        put("s1_no_forwarding", True, f"runtime part passed vacuously: no JVM ran ({start_reason})")
        detect = static_verifier
    put("s1_no_verifier_detection", not detect,
        "no verifier name in the tree" + ("" if runtime_ran else "; runtime part vacuous: no JVM ran")
        if not detect else "; ".join(detect[:10]))
    for gid in stage2:
        r = results2.get(gid)
        if r is None:
            put(gid, False, "not replayed: " + (why if not built else start_reason))
            continue
        failed = [c["case"] for c in r.get("cases", []) if not c.get("agrees")]
        put(gid, bool(r.get("agrees")), (r.get("error") or (f"{len(r.get('cases', []))} cases agree" if r.get("agrees")
            else f"{len(failed)} case(s) disagree: {failed[:8]}")) + (f" ({r['retried']})" if r.get("retried") else ""))
    for fid in stage3:
        r = results3.get(fid)
        if r is None:
            put(fid, False, "not replayed: " + (why if not built else start_reason if not started
                                                else "a retired-stack finding stopped Stage III"))
            continue
        put(fid, bool(r.get("agrees")), (r.get("error") or ("survived" if r.get("agrees") else "family result differs"))
            + (f" ({r['retried']})" if r.get("retried") else ""))
    write_route(items)
    return 0


def write_route(items: list[dict[str, Any]]) -> None:
    """A failed Stage I item or a failed Stage II group (k = 1): veto (zero, no judge call).
    Otherwise clear. The CTRF report and the route files are written on every path."""
    write_reports(items)
    stage1 = [i["id"] for i in items if i["stage"] == "I" and not verdict[i["id"]]["passed"]]
    failed = [i["id"] for i in items if i["stage"] == "II" and not verdict[i["id"]]["passed"]]
    vetoes = stage1 + failed
    route = "veto" if vetoes else "clear"
    (OUT / "route.txt").write_text(route + "\n")
    (OUT / "vetoes.txt").write_text("".join(f"{v}: {verdict[v]['reason'][:300]}\n" for v in vetoes))
    log(f"route {route}; Stage I vetoes: {stage1 or 'none'}; failed Stage II groups: {failed or 'none'}")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception:  # noqa: BLE001 - an infrastructure fault still leaves reports that fail every item
        traceback.print_exc()
        items = json.loads((DATA / "items.json").read_text())["items"]
        for item in items:
            verdict.setdefault(item["id"], {"passed": False, "reason": "grader fault: " + traceback.format_exc()[-500:]})
        write_route(items)
        raise SystemExit(0)
