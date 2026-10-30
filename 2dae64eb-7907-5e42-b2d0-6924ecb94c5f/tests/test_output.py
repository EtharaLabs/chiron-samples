"""One Reward Kit criterion per compiled rubric item, all served by one graded run.

GENERATED SECTION. DO NOT HAND-EDIT.
Derived from solution/grounding.yaml by solution/recompute.py.

Nothing happens when this file is imported beyond registering the criteria. The pipeline
itself ran before Reward Kit started, in tests/verifier/stage_note.py, which is also what
decides whether the submitted note reaches the judge. This file reads the verdicts that run
published and runs nothing itself. Reading them rather than the pipeline is what makes the
compiled verdicts and the judged layer's gate one run rather than two builds.

The verdicts arrive in the environment, never from a file. The verifier container runs
entirely as uid 1000, so any process the submission's build detached could rewrite a file
under /logs/verifier/ in the window between the stager writing it and this
process reading it. No process can alter another's environment after `execve`, so the
variable below is a channel the submission cannot reach: tests/test.sh captures the
stager's stdout and exports it, and graded-verdicts.json is left behind for the
operator's audit trail alone.

One window stays open, and `_foreign` closes it. Reward Kit must read the staged note as a
file, so a process still alive after the stager wrote it could replace that note with a
better-looking one before the judge sees it. The environment cannot protect a file the
judge opens itself. So on its first call this file re-runs the stager's own check for a
process the submission detached, and if anything foreign is alive it scores every item 0,
which also takes the judged layer to 0 through the gate. The stager swept the tree before
it wrote; anything foreign found here appeared after that sweep, which is the only way the
note on disk can differ from the note the stager staged.

Every outcome is fail-closed. A missing variable, a malformed one, one with no entry for an
item, and a check that cannot run at all, all leave that item false rather than raising, so
a submission is never credited for an item whose evaluation target does not exist. The
criterion carries the item identifier and nothing else: the prose of each item lives in
the rubric, never in the verifier image.
"""

from __future__ import annotations

import json
import os
import sys
import threading
from pathlib import Path

import rewardkit as rk
from rewardkit import criterion

VERDICTS = "VERIFIER_GRADED_VERDICTS"
STAGER_ROOT = "/opt/verifier"

ITEMS: tuple[tuple[str, int], ...] = (
    ("T-02", 5),
    ("T-03", 5),
    ("T-04", 5),
    ("T-05", 5),
    ("T-06", 5),
    ("T-07", 3),
    ("T-08", 5),
    ("T-09", 5),
    ("T-10", 3),
    ("T-11", 3),
    ("T-12", 3),
    ("T-13", 3),
    ("T-14", 5),
    ("T-15", 3),
    ("T-16", 3),
    ("T-17", 1),
    ("T-18", 3),
    ("T-19", 3),
    ("T-20", 3),
    ("T-21", 3),
    ("T-22", 3),
    ("P-01", 5),
    ("P-02", 5),
    ("P-03", 5),
    ("P-04", 5),
    ("P-05", 5),
    ("P-06", 5),
)

_lock = threading.Lock()
_verdicts: dict[str, bool] | None = None


def _foreign() -> bool:
    """Whether a process this one did not descend from is alive, fail-closed on any error.

    The check is /opt/verifier/stage_note.py's own, imported rather than restated, so the
    tree this file demands be quiet is the same tree the stager swept. A failure to run it
    at all is reported as hostile: the image ships the stager, so the check failing means
    the verifier is not the one this task built, and nothing may be credited on that
    footing.
    """
    try:
        root = os.environ.get("VERIFIER_ROOT") or STAGER_ROOT
        if root not in sys.path:
            sys.path.insert(0, root)
        import stage_note

        return bool(stage_note.foreign(stage_note.ancestry(os.getpid())))
    except BaseException:  # noqa: BLE001 - a check that cannot run credits nothing
        return True


def _results() -> dict[str, bool]:
    """The published verdicts of the one graded run, fail-closed in every outcome."""
    global _verdicts
    with _lock:
        if _verdicts is None:
            identifiers = [item for item, _ in ITEMS]
            items: object = None
            if not _foreign():
                try:
                    items = json.loads(os.environ[VERDICTS]).get("items")
                except BaseException:  # noqa: BLE001 - an unreadable channel scores nothing
                    items = None
            if not isinstance(items, dict):
                items = {}
            _verdicts = {item: items.get(item) is True for item in identifiers}
        return _verdicts


@criterion(description="{item}")
def graded_item(workspace: Path, item: str) -> bool:
    return _results()[item]


for _item, _weight in ITEMS:
    rk.graded_item(_item, weight=_weight, name=_item)
