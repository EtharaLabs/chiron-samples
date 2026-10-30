"""One Reward Kit criterion per judged rubric item, each withheld at zero.

GENERATED SECTION. DO NOT HAND-EDIT.
Derived from solution/grounding.yaml by solution/recompute.py.

The judged layer was withheld because the tree did not go quiet, so each judged item
scores zero here while still carrying its weight.
"""

from __future__ import annotations

from pathlib import Path

import rewardkit as rk
from rewardkit import criterion

ITEMS: tuple[tuple[str, int], ...] = (
    ("J-01", 5),
    ("J-02", 3),
    ("J-03", 3),
    ("J-04", 3),
    ("J-05", 3),
    ("J-06", 5),
    ("J-07", 3),
    ("J-08", 5),
    ("J-09", 3),
    ("J-10", 3),
    ("J-11", 3),
    ("J-12", 3),
    ("J-13", 1),
    ("J-14", 3),
)


@criterion(description="{item}")
def withheld_item(workspace: Path, item: str) -> float:
    return 0.0


for _item, _weight in ITEMS:
    rk.withheld_item(_item, weight=_weight, name=_item)
