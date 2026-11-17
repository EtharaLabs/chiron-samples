"""One Reward Kit criterion per compiled rubric item, all served by one graded run.

Nothing happens when this file is imported beyond registering the criteria. The pipeline
itself ran before Reward Kit started, in tests/verifier/stage_note.py, which is also what
decides whether the submitted note reaches the judge. This file reads the verdicts that run
published and runs nothing itself. Reading them rather than the pipeline is what makes the
compiled verdicts and the judged layer's gate one run rather than two builds.

The verdicts arrive in the environment, never from a file. Every step that grades runs
as uid 1000, so any process the submission's build detached could rewrite a file
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
    ("T-07", 5),
    ("T-08", 5),
    ("T-09", 5),
    ("T-10", 5),
    ("T-11", 5),
    ("T-12", 5),
    ("T-13", 5),
    ("T-14", 5),
    ("T-15", 5),
    ("T-16", 5),
    ("T-17", 5),
    ("S3-X01-bom", 3),
    ("S3-X01-comma", 3),
    ("S3-X01-comment", 3),
    ("S3-X01-errpos", 3),
    ("S3-X01-esc", 3),
    ("S3-X01-esckey", 3),
    ("S3-X01-json5", 3),
    ("S3-X01-trunc", 3),
    ("S3-X01-utf8", 3),
    ("S3-X01-ws", 3),
    ("S3-X01-trunc23", 3),
    ("S3-X02-d17", 3),
    ("S3-X02-d768", 3),
    ("S3-X02-diyfp", 3),
    ("S3-X02-e21", 3),
    ("S3-X02-expext", 3),
    ("S3-X02-fuzz", 3),
    ("S3-X02-halfrnd", 3),
    ("S3-X02-halfway", 3),
    ("S3-X02-hexinf", 3),
    ("S3-X02-intbound", 3),
    ("S3-X02-longdig", 3),
    ("S3-X02-pow2", 3),
    ("S3-X02-rnd", 3),
    ("S3-X02-round", 3),
    ("S3-X02-subnormal", 3),
    ("S3-X03-num", 3),
    ("S3-X03-str", 3),
    ("S3-X04-build_then_patch", 3),
    ("S3-X04-copy_roundtrip", 3),
    ("S3-X04-copy_scalars", 3),
    ("S3-X04-deep_nest", 3),
    ("S3-X04-equality", 3),
    ("S3-X04-mutpatch_all_mutable", 3),
    ("S3-X04-patch_fails_midway", 3),
    ("S3-X04-patch_on_empty", 3),
    ("S3-X04-patch_op_names", 3),
    ("S3-X04-ptr_array_tokens", 3),
    ("S3-X04-ptr_ctx_after_mutation", 3),
    ("S3-X04-ptr_escapes", 3),
    ("S3-X04-ptr_then_mutate", 3),
    ("S3-X04-fuzz", 3),
    ("S3-X05-build_arr_grow", 3),
    ("S3-X05-build_obj", 3),
    ("S3-X05-copy_cycle", 3),
    ("S3-X05-dupkey_copy", 3),
    ("S3-X05-dyn_strings", 3),
    ("S3-X05-merge_patch", 3),
    ("S3-X05-mutpatch_mutable", 3),
    ("S3-X05-numbers", 3),
    ("S3-X05-patch_add_copy", 3),
    ("S3-X05-patch_fail_mid", 3),
    ("S3-X05-ptr_create_parent", 3),
    ("S3-X05-ptr_mod_seq", 3),
    ("S3-X05-read_escapes", 3),
    ("S3-X05-read_write", 3),
    ("S3-X05-strpool", 3),
    ("S3-X05-valpool", 3),
    ("S3-X05-write_big", 3),
    ("S3-X05-fuzz", 3),
    ("T-23", 5),
    ("T-24", 5),
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
