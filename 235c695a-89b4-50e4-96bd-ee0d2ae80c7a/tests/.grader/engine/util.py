"""Process, file and text helpers for the grading engine (standard library only)."""

from __future__ import annotations

import hashlib
import os
import re
import shutil
import signal
import stat
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

GRADER_UID = 10002
GRADER_GID = 10002
LOG_CAP = 400_000

ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_text(text: str) -> str:
    return sha256_bytes(text.encode("utf-8", "surrogateescape"))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def cpu_count() -> int:
    """CPUs this container may use: the cgroup quota when one is set, else the affinity set."""
    count = len(os.sched_getaffinity(0))
    try:
        quota, period = Path("/sys/fs/cgroup/cpu.max").read_text().split()
        if quota != "max":
            count = min(count, max(1, -(-int(quota) // int(period))))
    except (OSError, ValueError):
        pass
    return max(1, count)


@dataclass
class Result:
    rc: int
    out: str
    seconds: float


class Runner:
    """Runs commands with a fixed environment, as the grader account when the engine is root."""

    def __init__(self, log_dir: Path, home: Path, tmp: Path, path: str | None = None) -> None:
        self.log_dir = log_dir
        self.home = home
        self.tmp = tmp
        # The real prefix (/usr/local) receives the graded install, so it is never searched.
        self.path = path or "/usr/bin:/bin"
        self.drop = os.geteuid() == 0
        self.serial = 0
        # Steps the host cut short (a timeout, the kernel's out-of-memory kill, a full disk): the
        # trial then measured the host rather than the submission and is reported for a rerun.
        self.faults: list[str] = []

    def env(self, extra: dict[str, str] | None = None) -> dict[str, str]:
        env = {
            "PATH": self.path,
            "HOME": str(self.home),
            "TMPDIR": str(self.tmp),
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
            "TZ": "UTC",
            "USER": "grader",
            "LOGNAME": "grader",
            "SHELL": "/bin/bash",
            "PYTHONDONTWRITEBYTECODE": "1",
        }
        if extra:
            env.update(extra)
        return env

    def run(
        self,
        name: str,
        argv: list[str],
        cwd: Path,
        env: dict[str, str] | None = None,
        timeout: int = 3600,
        stdin: bytes | None = None,
        as_grader: bool = True,
    ) -> Result:
        self.serial += 1
        prefix = []
        if as_grader and self.drop:
            prefix = ["setpriv", f"--reuid={GRADER_UID}", f"--regid={GRADER_GID}", "--clear-groups", "--"]
        started = time.monotonic()
        try:
            # Its own session, so a timeout kills every process the step started, not only the first.
            proc = subprocess.Popen(
                prefix + argv,
                cwd=cwd,
                env=self.env(env),
                stdin=subprocess.DEVNULL if stdin is None else subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
        except OSError as exc:
            rc, raw = 127, f"[grader: cannot run {argv[0]}: {exc}]\n".encode()
        else:
            try:
                raw, _ = proc.communicate(stdin, timeout=timeout)
                rc = proc.returncode
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                raw, _ = proc.communicate()
                rc, raw = 124, (raw or b"") + b"\n[grader: timed out]\n"
                self.faults.append(f"step {name} ran past its {timeout} s limit")
            else:
                if rc == -signal.SIGKILL:
                    self.faults.append(f"step {name} was killed, as the kernel does when memory runs out")
                elif b"No space left on device" in raw:
                    self.faults.append(f"step {name} ran out of disk space")
        seconds = time.monotonic() - started
        out = raw.decode("utf-8", "replace")
        log = self.log_dir / f"{self.serial:04d}-{safe_name(name)}.log"
        body = out if len(out) <= LOG_CAP else out[:LOG_CAP // 4] + "\n[...]\n" + out[-LOG_CAP // 2:]
        log.write_text(f"$ {' '.join(argv)}\n# cwd {cwd}\n# rc {rc} in {seconds:.1f}s\n{body}", encoding="utf-8")
        return Result(rc, out, seconds)

    def shell(self, name: str, script: str, cwd: Path, env: dict[str, str] | None = None, timeout: int = 3600) -> Result:
        return self.run(name, ["bash", "-o", "pipefail", "-c", script], cwd, env, timeout)


def safe_name(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", text)[:80]


def grader_dir(path: Path, mode: int = 0o755) -> Path:
    path.mkdir(parents=True, exist_ok=True)
    if os.geteuid() == 0:
        os.chown(path, GRADER_UID, GRADER_GID)
    path.chmod(mode)
    return path


def chown_tree(root: Path) -> None:
    if os.geteuid() != 0:
        return
    for base, dirs, files in os.walk(root):
        os.lchown(base, GRADER_UID, GRADER_GID)
        for name in dirs + files:
            os.lchown(os.path.join(base, name), GRADER_UID, GRADER_GID)


def is_executable(path: Path) -> bool:
    try:
        return bool(path.stat().st_mode & stat.S_IXUSR)
    except OSError:
        return False


def read_text(path: Path, limit: int = 8_000_000) -> str | None:
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if len(data) > limit or b"\0" in data[:8192]:
        return None
    return data.decode("utf-8", "surrogateescape")


def walk_files(root: Path) -> list[str]:
    """Relative POSIX paths of every regular file or symlink under root, sorted."""
    found = []
    for base, dirs, files in os.walk(root):
        dirs.sort()
        rel = os.path.relpath(base, root)
        for name in sorted(files):
            found.append(name if rel == "." else f"{rel}/{name}".replace(os.sep, "/"))
        for name in dirs:
            full = os.path.join(base, name)
            if os.path.islink(full):
                found.append(name if rel == "." else f"{rel}/{name}".replace(os.sep, "/"))
    return sorted(found)


def copy_tree(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(src, dst, symlinks=True)


def strip_ansi(text: str) -> str:
    return ANSI.sub("", text)
