"""Grader-owned services and the application process for one replay.

PostgreSQL 16 runs as the ``postgres`` account on 127.0.0.1:5432; every scenario starts on a
database freshly initialised from State A's docker/postgres/scripts/init SQL. The stub runs on
127.0.0.1:8082. The application runs as the unprivileged ``app`` account under strace (sockets,
processes and file opens) with the bound environment.
"""

from __future__ import annotations

import calendar
import glob
import math
import os
import shutil
import signal
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

PG_BIN = Path("/usr/lib/postgresql/16/bin")
PG_DATA = Path("/var/lib/grader-pg/data")
PG_PORT = 5432
DB = "folding_db"
DB_USER = "folding_user"
DB_PASSWORD = "shroot"
STUB_PORT = 8082
APP_PORT = 8080
CONTEXT = "/folding"
BASE = f"http://127.0.0.1:{APP_PORT}{CONTEXT}"
STUB = f"http://127.0.0.1:{STUB_PORT}"
DSN = f"postgresql://{DB_USER}:{DB_PASSWORD}@127.0.0.1:{PG_PORT}/{DB}"

# Application clock: every start runs under libfaketime with a whole-second offset fixed at
# launch, so the application's clock reads CLOCK_START (plus the launch's sub-second fraction)
# at the launch instant and advances in real time; the grader knows the clock exactly as
# real time + clock_offset.
CLOCK_START = calendar.timegm((2026, 3, 15, 11, 0, 0))
RUNUSER = shutil.which("runuser", path="/usr/sbin:/usr/bin:/sbin:/bin") or "/usr/sbin/runuser"
STRACE = shutil.which("strace", path="/usr/bin:/bin") or "/usr/bin/strace"
# libfaketime 0.9.10 built from its release tarball (sha256 729ad33b...) without FAKE_PTHREAD: the
# stock build's pthread_cond_timedwait interception misconverts the JVM's condition-variable deadlines
# and spins it (State A: 581 s boot, ~394% idle CPU, against 5 s and 1% without it).
FAKETIME_LIB = "/opt/faketime/libfaketime-nopthread.so.1"

CLOCK_ENV_EXTRA: dict[str, str] = {}


def clock_env(offset: int) -> dict[str, str]:
    # NO_FAKE_STAT: file timestamps stay real. With stat faked, WildFly's deployment scanner sees
    # the marker files it timestamps itself shifted a second time and redeploys the application
    # mid-scenario (measured on State A at Phase build); the clock the application reads is unchanged.
    return {"LD_PRELOAD": FAKETIME_LIB, "FAKETIME": f"{offset:+d}", "FAKETIME_DONT_FAKE_MONOTONIC": "1",
            "NO_FAKE_STAT": "1", **CLOCK_ENV_EXTRA}


SETTLE_SECONDS = 3

BOUND_ENV = {
    "LARS_URL_ROOT": STUB,
    "STATS_URL_ROOT": STUB,
    "REDIRECT_URL": "http://frontend.example",
    "ENABLE_LARS_HARDWARE_UPDATE": "false",
    "ENABLE_MONTHLY_RESULT_STORAGE": "false",
    "ENABLE_STATS_MONTHLY_RESET": "false",
    "ENABLE_STATS_SCHEDULED_PARSING": "false",
    "STATS_PARSING_SCHEDULE_HOUR": "*",
    "STATS_PARSING_SCHEDULE_MINUTE": "55",
    "STATS_PARSING_SCHEDULE_SECOND": "0",
    "STATS_PARSING_SCHEDULE_FIRST_DAY_OF_MONTH": "3",
    "STATS_PARSING_SCHEDULE_LAST_DAY_OF_MONTH": "31",
    "USERS_IN_AMD_GPU": "1",
    "USERS_IN_NVIDIA_GPU": "1",
    "USERS_IN_WILDCARD": "1",
    "DEPLOYED_DATABASE": "postgresql",
    "JDBC_CONNECTION_URL": f"jdbc:postgresql://127.0.0.1:{PG_PORT}/{DB}",
    "JDBC_CONNECTION_USER": DB_USER,
    "JDBC_CONNECTION_PASSWORD": DB_PASSWORD,
    "JDBC_CONNECTION_DRIVER": "org.postgresql.Driver",
}


def _as(user: str, argv: list[str], **kw) -> subprocess.CompletedProcess:
    done = subprocess.run([RUNUSER, "-u", user, "--", *argv], capture_output=True, **kw)
    if done.returncode:
        raise RuntimeError(f"{argv[0]} as {user} exited {done.returncode}: {done.stderr.decode(errors='replace')[-2000:]}")
    return done


def pg_start() -> None:
    # The server's lock file goes in the package's socket directory. postgresql-common creates it at
    # install, but an image built with Kaniko keeps nothing under /run, so create it the same way.
    run_dir = Path("/run/postgresql")
    if not run_dir.is_dir():
        run_dir.mkdir()
        shutil.chown(run_dir, "postgres", "postgres")
        run_dir.chmod(0o2775)
    if not (PG_DATA / "PG_VERSION").exists():
        PG_DATA.mkdir(parents=True, exist_ok=True)
        shutil.chown(PG_DATA.parent, "postgres", "postgres")
        shutil.chown(PG_DATA, "postgres", "postgres")
        pwfile = PG_DATA.parent / "pwfile"
        pwfile.write_text("postgres\n")
        shutil.chown(pwfile, "postgres", "postgres")
        _as("postgres", [str(PG_BIN / "initdb"), "-D", str(PG_DATA), "-A", "md5", "-U", "postgres",
                         f"--pwfile={pwfile}", "-E", "UTF8", "--locale=C.UTF-8"])
        pwfile.unlink()
    _as("postgres", [str(PG_BIN / "pg_ctl"), "-D", str(PG_DATA), "-w", "-l", str(PG_DATA.parent / "pg.log"),
                     "-o", f"-c listen_addresses=127.0.0.1 -c port={PG_PORT} -c fsync=off "
                           "-c synchronous_commit=off -c full_page_writes=off", "start"])


def pg_stop() -> None:
    subprocess.run([RUNUSER, "-u", "postgres", "--", str(PG_BIN / "pg_ctl"), "-D", str(PG_DATA),
                    "-m", "fast", "-w", "stop"], check=False, capture_output=True)


def _super(sql: str, db: str = "postgres") -> None:
    env = {**os.environ, "PGPASSWORD": "postgres"}
    subprocess.run(["psql", "-h", "127.0.0.1", "-p", str(PG_PORT), "-U", "postgres", "-d", db,
                    "-v", "ON_ERROR_STOP=1", "-q", "-c", sql], check=True, capture_output=True, env=env)


def db_fresh(init_dir: Path) -> None:
    """Drop and recreate folding_db from State A's init SQL (system users root/root2)."""
    _super(f"DROP DATABASE IF EXISTS {DB} WITH (FORCE)")
    _super(f"DO $$BEGIN IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname='{DB_USER}') THEN "
           f"CREATE ROLE {DB_USER} LOGIN PASSWORD '{DB_PASSWORD}'; END IF; END$$")
    _super(f"CREATE DATABASE {DB} OWNER {DB_USER}")
    for sql in sorted(init_dir.glob("*.sql")):
        text = sql.read_text()
        for key, value in {"%ADMIN_USERNAME%": "root", "%ADMIN_PASSWORD%": "shroot",
                           "%READ_ONLY_USERNAME%": "root2", "%READ_ONLY_PASSWORD%": "shroot2"}.items():
            text = text.replace(key, value)
        env = {**os.environ, "PGPASSWORD": DB_PASSWORD}
        subprocess.run(["psql", "-h", "127.0.0.1", "-p", str(PG_PORT), "-U", DB_USER, "-d", DB,
                        "-v", "ON_ERROR_STOP=1", "-q"], input=text.encode(), check=True,
                       capture_output=True, env=env)


def stub_start(stub_py: Path, log: Path) -> subprocess.Popen:
    proc = subprocess.Popen(["python3", str(stub_py), str(STUB_PORT)], stdout=log.open("ab"),
                            stderr=subprocess.STDOUT)
    _wait(f"{STUB}/__stub/reset", 30, method="POST")
    return proc


def stub_reset() -> None:
    urllib.request.urlopen(urllib.request.Request(f"{STUB}/__stub/reset", method="POST"), timeout=10).read()


def _wait(url: str, seconds: float, method: str = "GET", ok=lambda s: s < 500) -> int:
    deadline = time.monotonic() + seconds
    last = 0
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(urllib.request.Request(url, method=method), timeout=5) as r:
                last = r.status
        except urllib.error.HTTPError as e:
            last = e.code
        except OSError:
            last = 0
        if last and ok(last):
            return last
        time.sleep(0.5)
    return last



def _group_alive(pgid: int) -> bool:
    """Whether a process of group ``pgid`` other than a zombie still exists (PID 1 may not reap)."""
    for stat in Path("/proc").glob("[0-9]*/stat"):
        try:
            fields = stat.read_text().rsplit(")", 1)[1].split()
        except OSError:
            continue
        if fields[0] != "Z" and int(fields[2]) == pgid:
            return True
    return False


def _wait_group_gone(pgid: int, seconds: float) -> None:
    for signum, limit in ((None, seconds), (signal.SIGKILL, 30)):
        if signum is not None:
            try:
                os.killpg(pgid, signum)
            except ProcessLookupError:
                return
        deadline = time.monotonic() + limit
        while time.monotonic() < deadline:
            if not _group_alive(pgid):
                return
            time.sleep(0.1)


class App:
    """One application process: ``argv`` run as ``user`` with the bound environment."""

    def __init__(self, argv: list[str], *, user: str, workdir: Path, log: Path,
                 trace: Path | None = None, extra_env: dict[str, str] | None = None,
                 on_start=None) -> None:
        self.argv, self.user, self.workdir, self.log, self.trace = argv, user, workdir, log, trace
        self.on_start = on_start  # called with the start number before each launch
        self.env = {"PATH": "/usr/bin:/bin", "HOME": str(workdir), "LANG": "C.UTF-8", "TZ": "UTC",
                    **BOUND_ENV, **(extra_env or {})}
        self.base_env = dict(self.env)
        self.proc: subprocess.Popen | None = None
        self.starts = 0
        self.clock_offset = 0

    def app_clock(self, real: float) -> float:
        return real + self.clock_offset

    def start(self, ready_seconds: float = 180) -> bool:
        self.starts += 1
        self.clock_offset = CLOCK_START - math.floor(time.time())
        env = {**self.env, **clock_env(self.clock_offset)}
        if self.on_start is not None:
            self.on_start(self.starts)
        command = [part.replace("{start}", str(self.starts)) for part in self.argv]
        argv = [RUNUSER, "-u", self.user, "--", "/usr/bin/env", *[f"{k}={v}" for k, v in env.items()], *command]
        if self.trace is not None:
            trace = self.trace.with_name(f"{self.trace.stem}.{self.starts}{self.trace.suffix}")
            argv = [STRACE, "-f", "-qq", "-e", "trace=execve,?clone,?clone3,?fork,?vfork,connect,bind,listen,openat,?open",
                    "-e", "signal=none", "-o", str(trace), *argv]
        with self.log.open("ab") as out:
            self.proc = subprocess.Popen(argv, cwd=self.workdir, env=self.env, stdout=out,
                                         stderr=subprocess.STDOUT, start_new_session=True)
        status = _wait(f"{BASE}/hardware", ready_seconds, ok=lambda s: s == 200)
        if status != 200:
            return False
        # Start-up work the original runs asynchronously (its statistics parse when no hourly
        # statistics exist) may still hold the system state after the first answer; every start,
        # recorded or graded, waits the same settle time before the next request.
        time.sleep(SETTLE_SECONDS)
        return True

    def stop(self) -> None:
        if self.proc is None:
            return
        try:
            os.killpg(self.proc.pid, signal.SIGTERM)
            self.proc.wait(timeout=60)
        except subprocess.TimeoutExpired:
            os.killpg(self.proc.pid, signal.SIGKILL)
            self.proc.wait(timeout=30)
        except ProcessLookupError:
            pass
        # The traced parent can exit while the application it started is still shutting down and
        # still answering on the bound port; the next start must not be answered by it.
        _wait_group_gone(self.proc.pid, 60)
        self.proc = None

    def reset(self, init_dir: Path, env: dict[str, str] | None = None) -> None:
        """Stop, recreate the database and the stub state, start with the bound environment + ``env``."""
        self.stop()
        db_fresh(init_dir)
        stub_reset()
        self.env = {**self.base_env, **(env or {})}
        if not self.start():
            raise RuntimeError("application did not answer after reset")

    def restart(self, env: dict[str, str] | None = None) -> None:
        self.stop()
        if env:
            self.env.update(env)
        if not self.start():
            raise RuntimeError("application did not answer after restart")
