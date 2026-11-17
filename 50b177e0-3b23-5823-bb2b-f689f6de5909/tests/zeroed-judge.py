"""One Reward Kit criterion per judged rubric item, each scoring zero.

This is the judged half of the zero route. It lands in /tests/.zero as judge.py, so the
judged weight is carried at its real identifiers and weights while no judge is called:
a run on a host with no execution boundary, or one that shipped no library or whose judge
packet could not carry the whole submission or could not be staged and swept, has
nothing a judge could add.

Each criterion's description carries WHY the run was gated, read from the same
environment channel the compiled half reads.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import rewardkit as rk
from rewardkit import criterion

ITEMS: tuple[tuple[str, int], ...] = (
    ("Q-01", 5),
    ("Q-02", 1),
    ("Q-03", 5),
    ("Q-04", 5),
    ("Q-05", 3),
    ("Q-06", 3),
    ("Q-07", 3),
    ("Q-08", 3),
    ("Q-09", 1),
    ("Q-10", 1),
)


def _why() -> str:
    """Why the run was gated, fail-closed to a reason that is still attributable."""
    direct = os.environ.get("VERIFIER_ZERO_REASON", "").strip()
    if direct:
        reason = direct
    else:
        try:
            payload = json.loads(os.environ["VERIFIER_GRADED_VERDICTS"])
            reason = str(payload.get("route_reason") or "").strip()
        except BaseException:  # noqa: BLE001 - an unreadable channel is still reportable
            reason = ""
    if not reason:
        reason = "the gating reason could not be read from the verdict channel"
    # The description is a format template, so a brace in the reason would raise at
    # registration and take down a route whose whole job is to report a refusal.
    return reason.replace("{", "(").replace("}", ")")


WHY = _why()


@criterion(description="{item} scored zero on the gated route, with no judge call. " + WHY)
def gated_judged_item(workspace: Path, item: str) -> float:
    return 0.0


for _item, _weight in ITEMS:
    rk.gated_judged_item(_item, weight=_weight, name=_item)
