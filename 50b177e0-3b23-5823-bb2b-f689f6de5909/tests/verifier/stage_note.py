#!/usr/bin/env python3
"""Run the graded pipeline once, publish its verdicts, and build the judge packet.

This runs before Reward Kit, as the one step of /tests/test.sh that measures anything, and it
is the only place the graded pipeline runs. It produces three things:

  * the verdicts, printed as one JSON object and nothing else on stdout. /tests/test.sh
    captures that stdout and exports it, so /tests/test_output.py reads the compiled verdicts
    out of its own environment. An environment is fixed at `execve`: no process can alter
    another's, so this channel cannot be rewritten by anything the submission left running,
    which a file in a directory writable by uid 1000 can.
  * the verdict cache at /logs/verifier/graded-verdicts.json, for the operator's audit trail
    only. Nothing that scores reads it. It carries the same payload, including the sweep count
    and the one boolean the judged layer needs: whether the build produced the shipped library.
  * the judge packet, in the three part files /tests/judge.toml and /tests/.gate/judge.toml
    both name, in that order, as their only input.

The packet is the submission's own Rust and build files, written against State A's tree by
packet.py beside this file. It replaced a design note the agent used to write: the judged layer
reads the port now, not an account of it. Nothing else about the staging changed, and the
security model below is the note's own, word for word, with "packet" where it said "note".

It is three files rather than one because Reward Kit does not read a judge input over 1 MiB,
and applies that limit to each entry of a judge's `files` list separately. All three are
written, always, as one act: either every part is on disk or none is. A part that received no
file carries a line saying so, so an absent part always means this stager refused to stage and
never means that the packet happened to end early.

The packet reads every collected text file the submission added or changed to its LAST LINE,
in chunks across the parts when one part cannot hold it, so no file is withheld for being
large. What the parts cannot do is hold more than `packet.INLINE_CAP` bytes of numbered source
in total. A submission over that is NOT staged in part: `packet.build` reports the paths whose
last line never reached a part, and this stager then stages nothing at all and routes the run
to zero naming them. Staging the rest would hand a judge a negated question about source it was
never shown, which it would answer "no act found" -- full credit for the part of the port that
was hidden. A refusal is the only honest answer, and `route` below gives it.

The staged packet is the gate. It is written only for a submission whose graded build produced
the shipped library and whose six prohibition items all hold. Otherwise the paths are left
absent. An unbuildable tree, a tree that compiles C, that links a prebuilt archive, that loads
code dynamically, that edits the public header, that resolves a third-party crate, or that
carries State A's implementation, therefore stages nothing, and /tests/test.sh routes such a
run to the zero route rather than to the judge: every judged criterion is negated, so a judge
handed `[not found]` could cite no act and would answer no to all of them, which on a negated
item is full credit. Withholding the input is not enough on its own; the routing is what makes
it a refusal.

Write ordering, not path privacy, is what makes that hold. Every step of the verifier that
grades runs as uid 1000: the submission's build scripts, this stager and Reward Kit are the same
user, so a build can create, chmod or symlink anything this process can, including every path
below.
Hiding the paths would buy nothing, because a build that knows the image knows them, and the
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
still names a pid fails the submission closed: the packet is not staged, every part of a packet
already staged is removed again, the surviving pids and the reason go into the payload, and
/tests/test.sh sends the run to the zero route. Only the judged input is withheld that way. The
compiled verdicts are still reported as measured, because they travelled by environment and a
survivor cannot reach them; the survivor that is still alive when Reward Kit runs is caught
there instead, by /tests/test_output.py re-running `foreign` on its first call.

The unlink is unconditional, covers all three parts, and comes before any decision: skipping it
for a submission about to fail would leave a planted file in place for exactly the submission
the gate is meant to stop. The judged layer still reads the packet as files, which is the one
window these cannot close, and /tests/test_output.py closes it by re-running `foreign` itself.

Every failure is closed. Any exception while deciding leaves every staged path absent, and the
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

# The two modules beside this one in the verifier image: the stage-3 family table, which is
# where the compiled item set below comes from, and the judge packet builder. They are imported
# at module scope because COMPILED is a module-level constant and the CTRF report needs it.
if str(PIPELINE) not in sys.path:
    sys.path.insert(0, str(PIPELINE))
import families as _families  # noqa: E402
import packet as _packet  # noqa: E402

SCRATCH = Path(os.environ.get("VERIFIER_SCRATCH", "/tmp/verifier-run"))  # noqa: S108
LOGS = Path(os.environ.get("VERIFIER_LOG_DIR", "/logs/verifier"))

# Both literals are named elsewhere too: the cache in /tests/test_output.py's own comment,
# and the staged packet in the `files` entry of /tests/judge.toml and
# /tests/.gate/judge.toml.
CACHE = LOGS / "graded-verdicts.json"

# The three part files of the packet, in order, and the second literal the comment above is
# about. The names come from packet.py beside this file, so the stager and the builder cannot
# disagree about which paths the packet is, and both judge tables name those same three in that
# same order. They are joined onto LOGS rather than taken whole, so an operator who moves the
# log directory moves the packet with it.
STAGED_PARTS = tuple(LOGS / name for name in _packet.PART_NAMES)

# The part count spelled out, for the one sentence that reads as prose to a judge and to a
# reader of reward-details.json rather than as a field. Keyed on the number so the sentence
# follows packet.PART_COUNT instead of restating it.
_PART_COUNT_WORDS = {1: "one", 2: "two", 3: "three", 4: "four"}

# The CTRF report, written from this process because it is the only one that runs on every
# route and holds the measured verdicts. The zero route scores every criterion 0, so
# reward-details.json stops being able to say which item actually failed; this report is where
# that evidence survives, and it is also what binds the declared cheat checks, which are joined
# by test name. Status is only ever "passed" or "failed": a third value is not a weaker verdict
# to a reader of this file, it is a malformed report.
CTRF = LOGS / "ctrf.json"

# State A's own tree, as tests/verifier/freeze.sh installs it: the left-hand side the packet
# builder compares the submission against, so a file the submission did not touch is left out
# of the packet and named `unchanged` in its listing.
STATE_A_TREE = PIPELINE / "statea" / "tree"

PROHIBITIONS = ("P-01", "P-02", "P-03", "P-04", "P-05", "P-06")

# Stage I and Stage II: the behavioural modules, the artifact relations and the prohibitions.
# One of these false fires the compiled veto that tests/.veto/veto.toml declares, which
# tests/test.sh's veto route reads from the published verdicts.
STAGE_1_2 = (
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
    "T-23",
    "T-24",
    *PROHIBITIONS,
)

# Stage III: the sixty differential families, derived from families.py rather than listed, so
# the item set and the line partitioning that decides those items can never disagree. The
# emulated benchmark's third stage costs a share of the score rather than the run, so these
# stay in the weighted mean and out of the gate.
STAGE_3 = tuple(
    identifier for identifier, _module, _key, _scope in _families.families()
)

COMPILED = (*STAGE_1_2, *STAGE_3)


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


def boundary_available() -> bool:
    """Whether this kernel grants the execution boundary the graded build is declared to run in.

    The graded build is confined by Landlock, so a host whose kernel does not offer it cannot
    grade this bundle at all: tests/verifier/stages.py refuses the build rather than run it
    unconfined. That refusal is correct and must stay, but it looks exactly like a port that
    failed to build, so it is reported separately here and carried into the CTRF report and
    into every criterion description on the zero route. A platform missing a kernel feature is
    not a wrong answer, and nothing downstream should be able to read it as one.
    """
    try:
        if str(PIPELINE) not in sys.path:
            sys.path.insert(0, str(PIPELINE))
        import sandbox

        return bool(sandbox.available())
    except BaseException:  # noqa: BLE001 - an unaskable boundary is reported as absent
        return False


def incomplete_reason(paths: list[str]) -> str:
    """Why a run whose source outgrew the packet scores zero, naming the files that did not fit.

    This is the text the zero criteria carry, so it names the paths rather than a count: a
    reader of reward-details.json has to be able to see WHICH source the judged review never
    read. The two numbers are taken from the builder's own constants, so a change to the cap or
    to the part count cannot leave this sentence claiming the old ones.
    """
    return (
        "the judged review could not read the whole submission: "
        + ", ".join(paths)
        + f" did not fit in the {_packet.INLINE_CAP:,} bytes of line-numbered source the "
        + f"{_PART_COUNT_WORDS.get(_packet.PART_COUNT, str(_packet.PART_COUNT))} packet parts hold"
    )


def route(
    shipped: bool,
    boundary: bool = True,
    incomplete: list[str] | None = None,
) -> tuple[str, str]:
    """Which reward route this run is scored on, and why.

    A false Stage I or Stage II item is not decided here. The emulated benchmark multiplies its
    stages rather than averaging them, and that multiplication is the compiled veto
    tests/.veto/veto.toml declares, which tests/test.sh's veto route reads from the published
    verdicts. What is decided here is everything else that zeroes a run: a host with no
    execution boundary (which tests/test.sh asks before the veto, because a refused build
    measured nothing), a build that shipped no library, and a packet that could not carry the
    submission. The zero route registers every item at its real identifier and weight scoring
    zero, and it calls no judge, because such a run has nothing a judge could add.

    There is one precondition on the judged input, and it is the last question asked: whether
    the packet could carry the submission's collected source to its last line. `incomplete`
    holds the paths it could not, straight from `packet.build`, and a non-empty list routes the
    run to zero. Every judged criterion is a NEGATED prohibited act, answered only from a path,
    a line and a quote, so source no part of the packet carries is source that scores as clean:
    a one-file port larger than the cap would otherwise have its faults, and the anti-cheat gate
    G-01's evidence, hidden by the harness and paid for in full. It is asked last because it can
    only arise on a run that got that far: a tree that shipped no library or broke a prohibition
    has no packet built for it at all, so the list is empty by construction.

    Whether the packet that WAS built actually reached the judge is still decided after this, in
    /tests/test.sh, from `judged.staged` and the sweep: a run that staged nothing fails closed
    there too.
    """
    if not boundary:
        return "zero", (
            "VERIFIER ENVIRONMENT, NOT THE SUBMISSION: this host does not offer the execution "
            "boundary the graded build is declared to run in, so the build was refused rather "
            "than run unconfined. This score describes the grading platform and says nothing "
            "about the port. Required: Linux with Landlock at an ABI that handles REFER, and a "
            "container runtime that permits the three Landlock calls."
        )
    if not shipped:
        return "zero", "the graded build produced no shipped library"
    if incomplete:
        return "zero", incomplete_reason(list(incomplete))
    return "pass", "nothing the stager decides zeroes this run"


def decide(verdicts: dict[str, bool], shipped: bool) -> tuple[bool, str]:
    if not shipped:
        return False, "the graded build produced no shipped library"
    broken = [item for item in PROHIBITIONS if not verdicts.get(item)]
    if broken:
        return False, "prohibition item(s) false: " + ", ".join(broken)
    return True, "the graded build shipped the library and every prohibition item holds"


class _Incomplete(Exception):
    """Raised so an incomplete packet leaves by the SAME path an unbuildable one leaves by.

    The staging block's handler is what unlinks every part again, and reusing it rather than
    writing a second withdrawal is the point: there is one place parts are withdrawn, so a
    change to the withdrawal cannot apply to some refusals and not to others.
    """

    def __init__(self, paths: list[str]) -> None:
        super().__init__(", ".join(paths))
        self.paths = list(paths)


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

    # Unconditional, before any decision, and over every part: the last writer wins and this
    # process is it. Anything the submission's build left at any of the three paths is gone,
    # whatever it was. All three are cleared even though the parts are written as one act,
    # because a build that planted only part 2 must not be able to leave it standing.
    planted = any(part.exists() or part.is_symlink() for part in STAGED_PARTS)
    try:
        for part in STAGED_PARTS:
            part.unlink(missing_ok=True)
    except OSError as error:
        planted = True
        reason = f"a staged path could not be cleared: {error}"

    if survivors:
        hostile = _hostile("after the sweep", survivors, attempts)
        reason = f"{hostile}; {reason}" if reason else hostile

    staged = False
    packet_bytes = 0
    part_bytes: list[int] = []
    incomplete: list[str] = []
    if not reason:
        allowed, reason = decide(verdicts, shipped)
        if allowed:
            try:
                built = _packet.build(workspace, STATE_A_TREE)
                parts = built.parts
                if len(parts) != len(STAGED_PARTS):
                    raise ValueError(
                        f"the packet builder returned {len(parts)} part(s), "
                        f"not {len(STAGED_PARTS)}"
                    )
                incomplete = list(built.incomplete)
                if incomplete:
                    # A packet that cannot carry the whole submission is not staged in part.
                    # The paths go absent exactly as on every other fail-closed path, and
                    # `route` below reads the same list and sends the run to zero naming it: a
                    # judge given the readable half would score the unread half as clean.
                    raise _Incomplete(incomplete)
                for part, text in zip(STAGED_PARTS, parts):
                    part.write_text(text, encoding="utf-8")
                part_bytes = [len(text.encode("utf-8")) for text in parts]
                packet_bytes = sum(part_bytes)
                # `staged` means every part is on disk, which is what both judge tables need:
                # a judge handed two parts out of three reads `[not found]` for the third and
                # could cite nothing from the files that did not arrive.
                staged = all(part.is_file() for part in STAGED_PARTS)
            except BaseException as error:  # noqa: BLE001 - an unbuildable packet stages nothing
                if isinstance(error, _Incomplete):
                    reason = incomplete_reason(error.paths)
                else:
                    reason = (
                        f"the judge packet could not be built from {workspace}: "
                        f"{type(error).__name__}: {error}"
                    )
                # A write that failed halfway leaves earlier parts on disk. They go again, so
                # the all-or-nothing rule holds on the failure path too.
                staged = False
                part_bytes = []
                packet_bytes = 0
                for part in STAGED_PARTS:
                    try:
                        part.unlink(missing_ok=True)
                    except OSError:
                        continue

    # The last look this process can take, after the parts were written and immediately before
    # the payload below is published. A submission whose writer appeared in the window between
    # the check above and the write loses the whole packet again here, every part of it: a
    # partial withdrawal would leave the judge citing lines from a file the submission may own.
    late = foreign(keep, started)
    if late:
        hostile = _hostile("when the verdicts were published", late, attempts)
        if staged:
            staged = False
            for part in STAGED_PARTS:
                try:
                    part.unlink(missing_ok=True)
                except OSError as error:
                    hostile = (
                        f"{hostile}, and the staged packet part {part} could not be "
                        f"removed again: {error}"
                    )
        reason = f"{hostile}; {reason}" if reason else hostile

    # The route is decided from the measured verdicts alone, independently of the sweep: a
    # hostile tree already scores zero through every criterion's own foreign check, so routing
    # it is not what makes it fail closed. Whether the packet reached the judge is a separate
    # question, decided in /tests/test.sh from `judged.staged` and `judged.sweep_converged`.
    boundary = boundary_available()
    taken, why = route(shipped, boundary, incomplete)

    print(
        f"stage_note: staged={staged} packet_bytes={packet_bytes} "
        f"part_bytes={part_bytes} incomplete={incomplete} swept={swept} "
        f"attempts={attempts} "
        f"survivors={survivors} late={late} "
        f"snapshot={len(started)} snapshot_alive={len(alive)} "
        f"route={taken} why={why} "
        f"paths={[str(part) for part in STAGED_PARTS]} reason={reason}"
    )
    return {
        "items": verdicts,
        "shipped_library": shipped,
        "swept": swept,
        "route": taken,
        "route_reason": why,
        "boundary_available": boundary,
        "judged": {
            "staged": staged,
            "staged_paths": [str(part) for part in STAGED_PARTS],
            "staged_bytes": packet_bytes,
            "staged_part_bytes": part_bytes,
            "incomplete_paths": incomplete,
            "packet_incomplete": bool(incomplete),
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


def ctrf(payload: dict[str, object]) -> dict[str, object]:
    """The measured verdicts as a CTRF report, one test per compiled item, joined by name.

    This is the only place a gated run records which item failed: the zero route scores every
    criterion 0 by design, so a reader of reward-details.json alone cannot tell a port that
    missed one differential family from one that never built. The report also binds the
    declared cheat checks, which are matched on `name`, so the identifiers here are the rubric's
    own and carry no prefix or decoration.
    """
    items = payload.get("items")
    verdicts = items if isinstance(items, dict) else {}
    tests = [
        {
            "name": item,
            "status": "passed" if verdicts.get(item) is True else "failed",
            "duration": 0,
        }
        for item in COMPILED
    ]
    passed = sum(1 for test in tests if test["status"] == "passed")
    now = int(time.time() * 1000)
    return {
        "results": {
            "tool": {"name": "chiron-verifier"},
            "summary": {
                "tests": len(tests),
                "passed": passed,
                "failed": len(tests) - passed,
                "pending": 0,
                "skipped": 0,
                "other": 0,
                "start": now,
                "stop": now,
            },
            "tests": tests,
            "extra": {
                "route": payload.get("route"),
                "route_reason": payload.get("route_reason"),
                "staged_packet": bool((payload.get("judged") or {}).get("staged")),
                "shipped_library": payload.get("shipped_library"),
                "boundary_available": payload.get("boundary_available"),
            },
        }
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

    try:
        CTRF.parent.mkdir(parents=True, exist_ok=True)
        CTRF.write_text(json.dumps(ctrf(payload), indent=2, sort_keys=True), encoding="utf-8")
    except OSError as error:
        print(f"stage_note: the CTRF report could not be written: {error}", file=sys.stderr)

    print(json.dumps(payload, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
