"""The local database, the fixture, and one served application, for recording and for grading.

The database is the ScyllaDB the image carries, one node on 127.0.0.1:9042 with password
authentication. The fixture is one snapshot of every base table of the application keyspace, taken
once from State A's own seeding script and restored before each Stage II case and each family by truncating
every base table and inserting the snapshot rows again. The served application runs under a pinned
clock (libfaketime), so answers that depend on the current time are the same at recording and at
grading: one fixed offset from the real clock, chosen when this module loads, puts the application's
clock at FIXED_NOW then and keeps it advancing monotonically across every server start.
"""

from __future__ import annotations

import json
import os
import re
import signal
import socket
import subprocess
import threading
import time
from datetime import UTC, datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

KEYSPACE = "argus"
TABLETS = "argus_tablets"
CQL_PORT = 9042
FIXED_NOW = "2026-11-01 12:00:00"
# Seconds added to the real clock to give the application's clock; FIXED_NOW when this module loads.
CLOCK_OFFSET = int(datetime.fromisoformat(FIXED_NOW).replace(tzinfo=UTC).timestamp() - time.time())


def fake_now_us() -> int:
    """The application's clock now, in microseconds: the write timestamp of a restore."""
    return int((time.time() + CLOCK_OFFSET) * 1_000_000)
FAKETIME_LIB = "/opt/faketime/lib/faketime/libfaketime.so.1"
SCYLLA_FLAGS = [
    "--developer-mode=1", "--smp=1", "--memory=1G", "--reserve-memory=256M",
    "--overprovisioned", "--listen-address=127.0.0.1", "--rpc-address=127.0.0.1",
    "--api-address=127.0.0.1", "--prometheus-address=127.0.0.1",
    "--seed-provider-parameters=seeds=127.0.0.1", "--rf-rack-valid-keyspaces=1",
    "--endpoint-snitch=SimpleSnitch", "--authenticator=PasswordAuthenticator",
    "--reactor-backend=epoll",
]

CONFIG = """BASE_URL: "http://argus.test"
SCYLLA_CONTACT_POINTS:
    - 127.0.0.1
SCYLLA_USERNAME: cassandra
SCYLLA_PASSWORD: cassandra
SCYLLA_KEYSPACE_NAME: {keyspace}
SCYLLA_REPLICATION_FACTOR: 1
LOGIN_METHODS:
    - password
APP_LOG_LEVEL: WARNING
SECRET_KEY: argus-graded-build-secret
S3_ALLOWED_BUCKETS:
    - argus-test-bucket
S3_ALLOWED_MIME:
    - image/png
    - application/zstd
    - application/gzip
AWS_CLIENT_ID: none
AWS_CLIENT_SECRET: none
GITHUB_CLIENT_ID: none
GITHUB_CLIENT_SECRET: none
GITHUB_ACCESS_TOKEN: none
GITHUB_ENABLED: false
JIRA_SERVER: http://jira.invalid
JIRA_EMAIL: none@argus.test
JIRA_TOKEN: none
JIRA_ENABLED: false
JENKINS_URL: http://jenkins.invalid
JENKINS_USER: none
JENKINS_API_TOKEN_NAME: none
JENKINS_API_TOKEN: none
EMAIL_SENDER: "argus@argus.test"
EMAIL_SENDER_USER: ""
EMAIL_SENDER_PASS: ""
EMAIL_SERVER: "smtp.invalid"
EMAIL_SERVER_PORT: "587"
EMAIL_ENABLED: false
JOB_VALIDITY_PERIOD_DAYS: 30
APP_DOC_LINK:
ZEUS_SCHEMA: http
ZEUS_HOST: 127.0.0.1:9
ZEUS_TOKEN: none
EVENT_SUMMARIZATION_ENABLED: false
"""


def wait_port(port: int, seconds: float, host: str = "127.0.0.1") -> bool:
    deadline = time.time() + seconds
    while time.time() < deadline:
        with socket.socket() as sock:
            sock.settimeout(1)
            if sock.connect_ex((host, port)) == 0:
                return True
        time.sleep(0.5)
    return False


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class Database:
    """One ScyllaDB node and a driver session on it."""

    def __init__(self, workdir: Path, log: Path) -> None:
        self.workdir, self.log = workdir, log
        self.process: subprocess.Popen[bytes] | None = None
        self.session = None
        self.cluster = None

    def start(self, seconds: float = 240) -> None:
        for name in ("data", "commitlog", "hints", "view_hints", "saved_caches"):
            (self.workdir / name).mkdir(parents=True, exist_ok=True)
        command = [
            "/usr/bin/scylla", "--options-file", "/etc/scylla/scylla.yaml",
            "--workdir", str(self.workdir), *SCYLLA_FLAGS,
        ]
        with self.log.open("ab") as out:
            self.process = subprocess.Popen(command, stdout=out, stderr=subprocess.STDOUT)
        deadline = time.time() + seconds
        last: Exception | None = None
        while time.time() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError(f"scylla exited with {self.process.returncode}")
            if wait_port(CQL_PORT, 2):
                try:
                    self._connect()
                    return
                except Exception as exc:  # the auth subsystem comes up after the port
                    last = exc
            time.sleep(1)
        raise RuntimeError(f"scylla did not accept CQL within {seconds} s: {last}")

    def _connect(self) -> None:
        from cassandra.auth import PlainTextAuthProvider
        from cassandra.cluster import Cluster

        self.cluster = Cluster(
            ["127.0.0.1"], port=CQL_PORT,
            auth_provider=PlainTextAuthProvider("cassandra", "cassandra"),
            connect_timeout=10, control_connection_timeout=10,
        )
        self.session = self.cluster.connect()
        self.session.default_timeout = 120
        self.session.execute("SELECT release_version FROM system.local")

    def stop(self) -> None:
        if self.cluster is not None:
            self.cluster.shutdown()
        if self.process is not None and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(30)
            except subprocess.TimeoutExpired:
                self.process.kill()

    def create_keyspace(self, keyspace: str = KEYSPACE) -> None:
        """The application keyspace and argus_tablets, created as State A's own test suite does."""
        for name in (keyspace, TABLETS):
            self.session.execute(f"DROP KEYSPACE IF EXISTS {name}")
        self.session.execute(
            f"CREATE KEYSPACE {keyspace} WITH replication = "
            "{'class': 'NetworkTopologyStrategy', 'replication_factor': 1}"
        )
        self.session.execute(
            f"CREATE KEYSPACE {TABLETS} WITH replication = "
            "{'class': 'NetworkTopologyStrategy', 'replication_factor': 1} AND tablets = {'enabled': true}"
        )

    def describe(self, keyspace: str = KEYSPACE) -> str:
        out = []
        for name in (keyspace, TABLETS):
            rows = self.session.execute(f"DESCRIBE KEYSPACE {name}")
            out.extend(row.create_statement for row in rows)
        return "\n".join(out)

    def base_tables(self, keyspace: str = KEYSPACE) -> list[str]:
        rows = self.session.execute(
            "SELECT table_name FROM system_schema.tables WHERE keyspace_name = %s", [keyspace]
        )
        return sorted(row.table_name for row in rows)

    def dump(self, keyspace: str = KEYSPACE) -> dict[str, list[str]]:
        out = {}
        for table in self.base_tables(keyspace):
            rows = self.session.execute(f"SELECT JSON * FROM {keyspace}.{table}")
            out[table] = sorted(row[0] for row in rows)
        return out

    def restore(self, fixture: dict[str, list[str]], keyspace: str = KEYSPACE) -> list[str]:
        """Return every base table to the fixture; returns the tables that had changed.

        A table whose rows already equal the fixture is left alone. A changed table is truncated
        and its fixture rows written at the application's clock now (fake_now_us), which is newer
        than every earlier write, so no earlier deletion in an index shadows the restored rows, and
        older than every later write the application makes, so its own updates to fixture rows
        are never shadowed by the restore. A TRUNCATE that overlaps one of ScyllaDB's tablet merges
        can leave an index entry behind, so when the indexes do not settle the changed tables are
        truncated and written again, up to three times in all.
        """
        tables = self.base_tables(keyspace)
        reads = {t: self.session.execute_async(f"SELECT JSON * FROM {keyspace}.{t}") for t in tables}
        changed = [t for t, f in reads.items()
                   if sorted(row[0] for row in f.result()) != sorted(fixture.get(t, []))]
        for rewrite in range(3):
            for attempt in range(4):
                futures = [self.session.execute_async(f"TRUNCATE {keyspace}.{t}") for t in changed]
                errors = []
                for future in futures:
                    try:
                        future.result()
                    except Exception as exc:  # TRUNCATE can time out on a loaded host; repeating it is harmless
                        errors.append(exc)
                if not errors:
                    break
                if attempt == 3:
                    raise errors[0]
                time.sleep(15)
            pending = []
            stamp = fake_now_us()
            for table in changed:
                statement = self.session.prepare(
                    f"INSERT INTO {keyspace}.{table} JSON ? USING TIMESTAMP ?")
                for row in fixture.get(table, []):
                    pending.append(self.session.execute_async(statement, [row, stamp]))
                    if len(pending) >= 256:
                        for future in pending:
                            future.result()
                        pending = []
            for future in pending:
                future.result()
            try:
                self._settle_views(keyspace, changed, fixture)
                break
            except RuntimeError:
                if rewrite == 2:
                    raise
        return changed

    def _indexes(self, keyspace: str) -> dict[str, list[tuple[str, str, str]]]:
        """Per base table, each secondary index's backing view with its target kind and column."""
        rows = self.session.execute(
            "SELECT table_name, index_name, kind, options FROM system_schema.indexes"
            " WHERE keyspace_name = %s",
            [keyspace],
        )
        out: dict[str, list[tuple[str, str, str]]] = {}
        for row in rows:
            if row.kind == "CUSTOM":
                continue
            target = row.options["target"]
            match = re.fullmatch(r"(keys|values|entries|full)\((.+)\)", target)
            kind, column = (match.group(1), match.group(2)) if match else ("column", target)
            out.setdefault(row.table_name, []).append((f"{row.index_name}_index", kind, column.strip('"')))
        return out

    @staticmethod
    def _entries(rows: list[str], kind: str, column: str) -> int:
        """How many index entries the fixture rows give: one per row holding the column, or one
        per element of a collection column indexed by keys, values or entries."""
        count = 0
        for raw in rows:
            value = json.loads(raw).get(column)
            if value is None:
                continue
            count += len(value) if kind in ("keys", "values", "entries") else 1
        return count

    def _view_counts(self, keyspace: str, views: list[str]) -> dict[str, int]:
        futures = {v: self.session.execute_async(f"SELECT count(*) FROM {keyspace}.{v}") for v in views}
        return {v: f.result().one()[0] for v, f in futures.items()}

    def _settle_views(self, keyspace: str, changed: list[str], fixture: dict[str, list[str]],
                      seconds: float = 60) -> None:
        """Wait until the secondary indexes of restored tables hold exactly the fixture's entries.

        ScyllaDB updates the views behind secondary indexes asynchronously, so a query through an
        index straight after a restore can read a partial index. Each index's entry count is
        computed from the fixture rows and the restore waits until the index holds that many.
        """
        indexes = self._indexes(keyspace)
        expected = {view: self._entries(fixture.get(table, []), kind, column)
                    for table in changed for view, kind, column in indexes.get(table, [])}
        deadline = time.monotonic() + seconds
        views = sorted(expected)
        while views:
            counts = self._view_counts(keyspace, views)
            views = [v for v in views if counts[v] != expected[v]]
            if views and time.monotonic() > deadline:
                detail = ", ".join(f"{v} {counts[v]}/{expected[v]}" for v in views)
                raise RuntimeError(f"indexes not settled after restore: {detail}")
            if views:
                time.sleep(0.1)


ZEUS_PORT = 9


class _Echo(BaseHTTPRequestHandler):
    """The proxy upstream the configuration names: answers every request with what it received."""

    def _answer(self) -> None:
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if length else b""
        path, _, query = self.path.partition("?")
        payload = json.dumps({
            "method": self.command, "path": path, "query": query,
            "content_type": self.headers.get("Content-Type"),
            "body": body.decode("utf-8", "replace"),
        }, sort_keys=True).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(payload)

    do_GET = do_POST = do_PUT = do_DELETE = do_HEAD = do_PATCH = _answer

    def log_message(self, *_args) -> None:
        return None


def start_upstream(port: int = ZEUS_PORT) -> ThreadingHTTPServer:
    """Start the echo upstream on 127.0.0.1:port in a daemon thread (stopped with the process)."""
    server = ThreadingHTTPServer(("127.0.0.1", port), _Echo)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def clock_env(base: dict[str, str]) -> dict[str, str]:
    env = dict(base)
    env.update(
        LD_PRELOAD=FAKETIME_LIB,
        FAKETIME=f"{CLOCK_OFFSET:+d}",
        FAKETIME_DONT_FAKE_MONOTONIC="1",
        NO_FAKE_STAT="1",  # gunicorn times its worker heartbeat by fstat; faked stat times kill every worker
        TZ="UTC",
    )
    return env


class Server:
    """One served application process group, started from a tree with a launch command."""

    def __init__(self, tree: Path, command: list[str], env: dict[str, str], log: Path,
                 prefix: list[str] | None = None, stop_signal: int = signal.SIGTERM) -> None:
        self.tree, self.command, self.env, self.log = tree, command, env, log
        self.prefix = prefix or []
        self.stop_signal = stop_signal
        self.process: subprocess.Popen[bytes] | None = None

    def start(self, port: int, ready_path: str = "/auth/login", seconds: float = 60) -> str:
        """Start and wait for HTTP 200 on ready_path; returns an empty string or the failure."""
        import http.client

        with self.log.open("ab") as out:
            self.process = subprocess.Popen(
                [*self.prefix, *self.command], cwd=self.tree, env=self.env,
                stdout=out, stderr=subprocess.STDOUT, start_new_session=True,
            )
        deadline = time.time() + seconds
        status = None
        while time.time() < deadline:
            if self.process.poll() is not None:
                return f"server exited with {self.process.returncode}"
            try:
                connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
                connection.request("GET", ready_path, headers={"Host": "argus.test"})
                status = connection.getresponse().status
                connection.close()
                if status == 200:
                    return ""
            except OSError:
                pass
            time.sleep(0.5)
        return f"no HTTP 200 on {ready_path} within {seconds} s (last status {status})"

    def stop(self) -> None:
        if self.process is None or self.process.poll() is not None:
            return
        try:
            os.killpg(self.process.pid, self.stop_signal)
            self.process.wait(20)
        except (ProcessLookupError, subprocess.TimeoutExpired):
            try:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(20)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                pass


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))
