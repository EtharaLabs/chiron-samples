#!/usr/bin/env python3
"""Run the graded pipeline once, publish its verdicts, and decide whether the note is judged.

This runs before Reward Kit, as the first of the three lines of /tests/test.sh, and it is the
only place the graded pipeline runs. It produces three things:

  * the verdicts, printed as one JSON object and nothing else on stdout. /tests/test.sh
    captures that stdout and exports it, so /tests/test_output.py reads the 27 compiled
    verdicts out of its own environment. An environment is fixed at `execve`: no process can
    alter another's, so this channel cannot be rewritten by anything the submission left
    running, which a file in a directory writable by uid 1000 can.
  * the verdict cache at /logs/verifier/graded-verdicts.json, for the operator's audit trail
    only. Nothing that scores reads it. It carries the same payload, including the sweep count
    and the one boolean the judged layer needs: whether the build produced the shipped library.
  * the staged note, which every criterion of /tests/judge.toml names as its only input.

The staged note is the gate. The submission's own doc/rust-port.md is copied there only for a
submission whose graded build produced the shipped library and whose six prohibition items all
hold. Otherwise the path is left absent, Reward Kit renders it to the judge as the text
`[not found]`, and all 46 judged weight scores zero. An unbuildable tree, a tree that compiles
C, that links a prebuilt archive, that loads code dynamically, that edits the public header,
that resolves a third-party crate, or that carries State A's implementation, therefore earns
nothing in the judged dimension, however good its prose is.

Write ordering, not path privacy, is what makes that hold. The verifier container runs entirely
as uid 1000: the submission's build scripts, this stager and Reward Kit are the same user, so a
build can create, chmod or symlink anything this process can, including both paths below.
Hiding the path would buy nothing, because a build that knows the image knows the path, and the
image ships with the task. What the submission cannot do is write last.

Writing last takes three things, because a file planted during the build is not the only way in.
A build step can also leave a *process* alive that writes after this stager has finished:

  1. `probe.run` gives every graded command a session of its own and SIGKILLs that command's
     whole process group when it returns, so an orphan a build double-forked dies with it.
  2. `sweep` below walks /proc and SIGKILLs anything that escaped that group by calling
     `setsid` for itself. It runs after the graded build, before the unlink, and before
     anything is written, so the tree is quiet before this process writes at all.
  3. The verdicts then travel by environment rather than by file, so even a process that
     somehow outlived the sweep has nothing left to rewrite that the compiled score reads.

What `sweep` must not do is kill a process that was already running when this stager started.
Under a stock Harbor docker environment the task container's `command` is
`sh -c "sleep infinity"`, so PID 1 is a shell and the keepalive is a separate `sleep` child of
it, and /tests/test.sh arrives by `docker exec`, whose process tree is rooted outside this PID
namespace. The `ancestry` walk from this stager therefore reaches parent 0 without ever passing
through that `sleep`: the ancestor chain alone does not describe it. Killing it stops the
container, because PID 1 exits once its only child dies, and the exec dies with it before any
verdict is published.

So the exclusion is widened by one term: `snapshot`, taken at stager start, before the graded
build runs. It is keyed on `(pid, starttime)` and not on the bare pid, because `starttime`
(field 22 of /proc/<pid>/stat) is the only field that distinguishes a pid the kernel has since
reused. A process is foreign only when its `(pid, starttime)` is absent from that snapshot, it
is not PID 1, and it is not an ancestor. Reparenting changes `PPid` but neither the pid nor the
start time, so a writer the build detached and the kernel then adopted into PID 1 stays
killable: it is new, and newness rather than parentage is what the snapshot measures.

The snapshot travels in the payload, under `judged.proc_snapshot`, so the later `foreign` calls
that /tests/test.sh and /tests/test_output.py make -- both after this process has exited --
reuse the tree this stager saw rather than snapshotting a tree the submission has already had
its chance to populate. `judged.proc_snapshot_size` records how many pids it held, so a reader
can see the guard was active.

`sweep` is a budget and not a guarantee, so it is not trusted to have worked. It gives up after
`SWEEP_ATTEMPTS` passes, and a writer that spawns a replacement faster than one pass takes
outlives it: every pass kills what it listed, and anything created after that listing lives on
to spawn the next. `foreign` is therefore asked once more after the loop, and once more at the
last moment this process holds, immediately before its payload is published. Either answer that
still names a pid fails the submission closed: the note is not staged, a note already staged is
removed again, the surviving pids and the reason go into the payload, and the judged layer
scores zero through the absent-file path. Only the judged input is withheld that way. The
compiled verdicts are still reported as measured, because they travelled by environment and a
survivor cannot reach them; the survivor that is still alive when Reward Kit runs is caught
there instead, by /tests/test_output.py re-running `foreign` on its first call.

The unlink is unconditional and comes before any decision: skipping it for a submission about
to fail would leave a planted file in place for exactly the submission the gate is meant to
stop. The judged layer still reads the note as a file, which is the one window these cannot
close, and /tests/test_output.py closes it by re-running `foreign` itself.

Every failure is closed. Any exception while deciding leaves the staged path absent, and the
process exits 0 in every case: /tests/test.sh runs under `set -e`, and a stager that failed the
run would replace a zero score with no score at all.
"""

from __future__ import annotations

import json
import os
import signal
import sys
import time
from pathlib import Path

PIPELINE = Path(os.environ.get("VERIFIER_ROOT", "/opt/verifier"))
SCRATCH = Path(os.environ.get("VERIFIER_SCRATCH", "/tmp/verifier-run"))  # noqa: S108
LOGS = Path(os.environ.get("VERIFIER_LOG_DIR", "/logs/verifier"))

# Both literals are named in the generated artifacts too: the cache in solution/recompute.py,
# which writes /tests/test_output.py, and the staged note in solution/grounding.yaml, which is
# where the judge block's `files` entry comes from and so what /tests/judge.toml carries.
CACHE = LOGS / "graded-verdicts.json"
STAGED_NOTE = LOGS / "staged-note.md"

SUBMITTED_NOTE = "doc/rust-port.md"
PROHIBITIONS = ("P-01", "P-02", "P-03", "P-04", "P-05", "P-06")
COMPILED = (
    "T-02",
    "T-03",
    "T-04",
    "T-05",
    "T-06",
    "T-07",
    "T-08",
    "T-09",
    "T-10",
    "T-11",
    "T-12",
    "T-13",
    "T-14",
    "T-15",
    "T-16",
    "T-17",
    "T-18",
    "T-19",
    "T-20",
    "T-21",
    "T-22",
    *PROHIBITIONS,
)


PROC = Path("/proc")
SWEEP_ATTEMPTS = 8
SWEEP_PAUSE = 0.2

# The environment channel /tests/test.sh exports this process's payload into. `_pre_existing`
# reads the stager's own snapshot back out of it, so a `foreign` call made in a later process
# excludes the tree this stager saw rather than the tree that process finds.
VERDICT_CHANNEL = "VERIFIER_GRADED_VERDICTS"

_BASELINE: frozenset[tuple[int, str]] | None = None


def _status(pid: int) -> dict[str, str]:
    """`/proc/<pid>/status` as a mapping, empty when the process is gone or unreadable."""
    try:
        text = (PROC / str(pid) / "status").read_text(encoding="ascii", errors="replace")
    except (OSError, ValueError):
        return {}
    fields: dict[str, str] = {}
    for line in text.splitlines():
        name, _, value = line.partition(":")
        fields[name.strip()] = value.strip()
    return fields


def _starttime(pid: int) -> str:
    """Field 22 of `/proc/<pid>/stat`, the clock ticks since boot at which `pid` was started.

    This is the one field of the stat line that separates a live process from an earlier one
    the kernel gave the same pid, so it is what keys the snapshot `foreign` excludes.

    Field 2 is `comm`, which is bracketed and may itself contain spaces or brackets, so the
    line is split only from after its last `)`. That tail begins at field 3, which puts field
    22 at index 19 and leaves the index unshiftable by any process name.
    """
    try:
        text = (PROC / str(pid) / "stat").read_text(encoding="ascii", errors="replace")
    except (OSError, ValueError):
        return ""
    _, bracket, tail = text.rpartition(")")
    if not bracket:
        return ""
    fields = tail.split()
    return fields[19] if len(fields) > 19 else ""


def snapshot() -> frozenset[tuple[int, str]]:
    """Every process alive right now, as `(pid, starttime)` pairs.

    Taken once at stager start, before the graded build runs, and excluded from every later
    sweep: the container's own PID 1 and the keepalive sibling it holds open are in here, and
    neither is reachable through this process's ancestor chain when /tests/test.sh arrives by
    `docker exec`. A pid whose start time cannot be read is one that vanished mid-walk and is
    left out rather than recorded under an empty key, so it cannot match a later process.
    """
    try:
        entries = os.listdir(PROC)
    except OSError:
        return frozenset()
    pairs: set[tuple[int, str]] = set()
    for entry in entries:
        if not entry.isdigit():
            continue
        pid = int(entry)
        started = _starttime(pid)
        if started:
            pairs.add((pid, started))
    return frozenset(pairs)


def _pre_existing() -> frozenset[tuple[int, str]]:
    """The stager's snapshot, read back from the payload it published, cached per process.

    A `foreign` caller that passes no snapshot of its own is one of the two that run after the
    stager exited: the gate in /tests/test.sh and /tests/test_output.py. Both hold the stager's
    payload in their environment, which is fixed at `execve` and so is the one channel the
    submission cannot reach. Taking a fresh snapshot there instead would baseline in whatever
    the submission had left running, which is exactly what those two callers exist to catch.

    A channel that is absent or malformed falls back to a fresh snapshot. That is the weaker
    answer, but it is not a credit: both callers read their verdicts from the same variable, so
    a channel this function could not parse is one neither of them can score from either, and
    both already treat that as zero.
    """
    global _BASELINE
    if _BASELINE is None:
        try:
            published = json.loads(os.environ[VERDICT_CHANNEL])
            pairs = published["judged"]["proc_snapshot"]
            _BASELINE = frozenset((int(pid), str(started)) for pid, started in pairs)
        except BaseException:  # noqa: BLE001 - an unreadable channel re-snapshots instead
            _BASELINE = snapshot()
    return _BASELINE


def ancestry(pid: int) -> frozenset[int]:
    """`pid` and every process above it, walked through the `PPid` chain of /proc.

    Walking the chain rather than trusting `getppid` alone is what makes the sweep safe: every
    process this one descends from is excluded by construction, so the shell that launched this
    stager, the shell that launched that one, and PID 1 are all out of range however deep the
    chain happens to be. The visited set makes the walk terminate even on a malformed `PPid`.
    """
    chain: set[int] = set()
    current = pid
    while current > 0 and current not in chain:
        chain.add(current)
        parent = _status(current).get("PPid", "0")
        try:
            current = int(parent)
        except ValueError:
            break
    chain.add(1)
    return frozenset(chain)


def foreign(keep: frozenset[int], started: frozenset[tuple[int, str]] | None = None) -> list[int]:
    """Every live process that the graded build is the only possible origin of.

    Four things are excluded, and a process is foreign only when it is none of them:

      * `keep`, the caller's live `ancestry` walk;
      * PID 1, named outright rather than left to that walk, because the walk reaches it only
        when this process descends from it and under `docker exec` it does not;
      * `started`, the `(pid, starttime)` snapshot taken at stager start, which is what keeps
        the container's keepalive alive. Matching on the pair and not the pid is what keeps a
        build-spawned process killable after the kernel reparents it into PID 1: adoption
        rewrites `PPid` and leaves both halves of the key untouched, so such a process is
        still absent from the snapshot and still foreign.
      * a pid whose start time has become unreadable, which is one that exited during the
        walk. Treating it as foreign would make the sweep chase a process that is gone.

    A zombie is skipped deliberately. It has already been killed and can execute nothing, but
    it stays listed in /proc until something reaps it, and in this container PID 1 is a plain
    shell that does not reap the orphans the kernel reparents to it. Counting a zombie as
    foreign would make the sweep loop until its attempts ran out and then report the tree as
    still hostile, which would fail a submission closed for a process that cannot write.
    """
    known = _pre_existing() if started is None else started
    found: list[int] = []
    try:
        entries = os.listdir(PROC)
    except OSError:
        return found
    for entry in entries:
        if not entry.isdigit():
            continue
        pid = int(entry)
        if pid == 1 or pid in keep:
            continue
        state = _status(pid).get("State", "")
        if not state or state.startswith("Z"):
            continue
        current = _starttime(pid)
        if not current or (pid, current) in known:
            continue
        found.append(pid)
    return sorted(found)


def sweep(keep: frozenset[int], started: frozenset[tuple[int, str]] | None = None) -> tuple[int, int]:
    """SIGKILL every foreign process until a pass finds none; say how many died and how often.

    This cannot stop the container. Between them `keep`, PID 1 and `started` hold everything
    that was running before this stager began, so what is left to kill is only what the graded
    build created, and no such pid is load-bearing: the graded build is over, every command it ran
    went through `probe.run`, which already SIGKILLed each command's own process group, and
    Reward Kit has not started. Anything still alive and able to run is therefore something a
    build step detached on purpose.

    The two counts returned are evidence, not a verdict. A pass kills exactly what the `/proc`
    listing at the head of that pass named, so a writer that creates its replacement after that
    listing survives the pass, and `SWEEP_ATTEMPTS` of them. Whether the tree actually went
    quiet is therefore decided by the caller, which asks `foreign` again rather than reading a
    count: a non-zero kill count says only that killing happened, and an exhausted attempt
    budget says nothing at all about what is left.
    """
    killed = 0
    attempts = 0
    for _ in range(SWEEP_ATTEMPTS):
        targets = foreign(keep, started)
        if not targets:
            break
        attempts += 1
        for pid in targets:
            try:
                os.kill(pid, signal.SIGKILL)
            except OSError:
                continue
            killed += 1
        time.sleep(SWEEP_PAUSE)
    return killed, attempts


def _hostile(when: str, pids: list[int], attempts: int) -> str:
    """The reason a tree that is still not quiet stages nothing, naming the surviving pids."""
    listed = ", ".join(str(pid) for pid in pids)
    return (
        f"the process sweep did not converge: {attempts} pass(es) of at most {SWEEP_ATTEMPTS} "
        f"left pid(s) {listed} alive {when}"
    )


def graded(workspace: Path) -> tuple[dict[str, bool], bool]:
    """Every compiled verdict of the one graded run, and whether it shipped a library.

    `pipeline.evaluate` never raises and returns false for anything it could not establish. It
    leaves its install prefixes in the scratch directory, so the shipped library is read back
    from where the graded build installed it rather than inferred from the verdicts: no single
    compiled item distinguishes a build that produced no library from a library that differs
    from State A's.
    """
    if str(PIPELINE) not in sys.path:
        sys.path.insert(0, str(PIPELINE))
    import pipeline
    import probe

    verdicts = pipeline.evaluate(workspace, list(COMPILED))
    library = probe.shipped_library(SCRATCH / "prefix-shared", "shared")
    return {item: bool(verdicts.get(item, False)) for item in COMPILED}, library is not None


def decide(verdicts: dict[str, bool], shipped: bool) -> tuple[bool, str]:
    if not shipped:
        return False, "the graded build produced no shipped library"
    broken = [item for item in PROHIBITIONS if not verdicts.get(item)]
    if broken:
        return False, "prohibition item(s) false: " + ", ".join(broken)
    return True, "the graded build shipped the library and every prohibition item holds"


def _stage(workspace: Path) -> dict[str, object]:
    # First, before the graded build gets to create anything: whatever is running now is the
    # container's own and is never a sweep target, however the sweep later reaches it.
    global _BASELINE
    _BASELINE = started = snapshot()

    verdicts: dict[str, bool] = {item: False for item in COMPILED}
    shipped = False
    reason = ""
    try:
        verdicts, shipped = graded(workspace)
    except BaseException as error:  # noqa: BLE001 - an unrunnable pipeline scores nothing
        reason = f"the graded pipeline did not run: {type(error).__name__}: {error}"

    # After the graded build and before the staged path is touched: anything the submission
    # detached is killed here, so this process writes into a tree the sweep has been through.
    keep = ancestry(os.getpid())
    swept, attempts = sweep(keep, started)

    # Whether the sweep worked, asked of /proc rather than assumed from the count above. A
    # writer that outran the attempt budget is still here, and a tree that is not quiet is
    # failed closed below: the staged path stays absent and the survivors are recorded.
    survivors = foreign(keep, started)

    # How much of the pre-existing tree the sweep left standing, read back from /proc after
    # the sweep rather than assumed. Equality with the snapshot size is the whole guard stated
    # as one measurement, and it is the only place a reader can see that the container's own
    # keepalive outlived a sweep rather than merely that the sweep found nothing to report.
    alive = sorted(pid for pid, start in started if _starttime(pid) == start)

    # Unconditional, before any decision: the last writer wins and this process is it. Anything
    # the submission's build left here is gone, whatever it was.
    planted = STAGED_NOTE.exists() or STAGED_NOTE.is_symlink()
    try:
        STAGED_NOTE.unlink(missing_ok=True)
    except OSError as error:
        planted = True
        reason = f"the staged path could not be cleared: {error}"

    if survivors:
        hostile = _hostile("after the sweep", survivors, attempts)
        reason = f"{hostile}; {reason}" if reason else hostile

    staged = False
    if not reason:
        allowed, reason = decide(verdicts, shipped)
        if allowed:
            try:
                source = workspace / SUBMITTED_NOTE
                STAGED_NOTE.write_bytes(source.read_bytes())
                staged = True
            except OSError as error:
                reason = f"the submission ships no readable {SUBMITTED_NOTE}: {error}"

    # The last look this process can take, after the note was written and immediately before
    # the payload below is published. A submission whose writer appeared in the window between
    # the check above and the write loses the note again here.
    late = foreign(keep, started)
    if late:
        hostile = _hostile("when the verdicts were published", late, attempts)
        if staged:
            staged = False
            try:
                STAGED_NOTE.unlink(missing_ok=True)
            except OSError as error:
                hostile = f"{hostile}, and the staged note could not be removed again: {error}"
        reason = f"{hostile}; {reason}" if reason else hostile

    print(
        f"stage_note: staged={staged} swept={swept} attempts={attempts} "
        f"survivors={survivors} late={late} "
        f"snapshot={len(started)} snapshot_alive={len(alive)} "
        f"path={STAGED_NOTE} reason={reason}"
    )
    return {
        "items": verdicts,
        "shipped_library": shipped,
        "swept": swept,
        "judged": {
            "staged": staged,
            "staged_path": str(STAGED_NOTE),
            "cleared_a_pre_existing_file": planted,
            "sweep_attempts": attempts,
            "sweep_converged": not survivors and not late,
            "survivors_after_sweep": survivors,
            "survivors_at_publish": late,
            "proc_snapshot_size": len(started),
            "proc_snapshot": sorted([pid, start] for pid, start in started),
            "proc_snapshot_alive_after_sweep": alive,
            "reason": reason,
        },
    }


def main(argv: list[str]) -> int:
    workspace = Path(argv[1]) if len(argv) > 1 else Path("/workspace/repo")

    # stdout is the verdict channel and carries one JSON object, so every diagnostic this
    # process or the pipeline under it writes is routed to stderr for the duration.
    speaking = sys.stdout
    sys.stdout = sys.stderr
    try:
        payload = _stage(workspace)
    finally:
        sys.stdout = speaking

    try:
        CACHE.parent.mkdir(parents=True, exist_ok=True)
        CACHE.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")
    except OSError as error:
        print(f"stage_note: the verdict cache could not be written: {error}", file=sys.stderr)

    print(json.dumps(payload, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
