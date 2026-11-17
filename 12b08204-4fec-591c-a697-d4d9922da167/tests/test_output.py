"""Compiled items, one Reward Kit criterion per item at its id and weight.

The graded pipeline (/opt/grader/grade.py) decides every item once and records whether
each item's criterion, as phrased, was observed. Generated from the grounding record.
"""

from __future__ import annotations

import json
from pathlib import Path

import rewardkit as rk

try:
    _GRADED = json.loads(Path("/logs/verifier/graded.json").read_text(encoding="utf-8"))
except (OSError, ValueError):
    _GRADED = {"vetoed": True, "items": {}}
_VETOED = _GRADED.get("vetoed", True) is not False
_ITEMS = _GRADED.get("items", {})


def _observed(item: str) -> bool | None:
    verdict = _ITEMS.get(item)
    if _VETOED or not isinstance(verdict, dict):
        return None
    value = verdict.get("observed")
    return value if isinstance(value, bool) else None


@rk.criterion(description="{item} holds")
def holds(workspace: Path, item: str) -> bool:
    return _observed(item) is True


@rk.criterion(description="{item} is not committed")
def not_committed(workspace: Path, item: str) -> bool:
    return _observed(item) is False


rk.holds("S1", name="S1", weight=5)
rk.not_committed("S2", name="S2", weight=5)
rk.not_committed("S3", name="S3", weight=5)
rk.not_committed("S4", name="S4", weight=5)
rk.holds("S5", name="S5", weight=5)
rk.not_committed("S6", name="S6", weight=5)
rk.not_committed("S7", name="S7", weight=5)
rk.not_committed("S8", name="S8", weight=5)
rk.holds("S9", name="S9", weight=5)
rk.holds("S10", name="S10", weight=5)
rk.holds("G01", name="G01", weight=5)
rk.holds("G02", name="G02", weight=5)
rk.holds("G03", name="G03", weight=5)
rk.holds("G04", name="G04", weight=5)
rk.holds("G05", name="G05", weight=5)
rk.holds("G06", name="G06", weight=5)
rk.holds("G07", name="G07", weight=5)
rk.holds("G08", name="G08", weight=5)
rk.holds("G09", name="G09", weight=5)
rk.holds("G10", name="G10", weight=5)
rk.holds("G11", name="G11", weight=5)
rk.holds("G12", name="G12", weight=5)
rk.holds("G13", name="G13", weight=5)
rk.holds("G14", name="G14", weight=5)
rk.holds("G15", name="G15", weight=5)
rk.holds("G16", name="G16", weight=5)
rk.holds("G17", name="G17", weight=5)
rk.holds("F001", name="F001", weight=3)
rk.holds("F002", name="F002", weight=3)
rk.holds("F003", name="F003", weight=3)
rk.holds("F004", name="F004", weight=3)
rk.holds("F005", name="F005", weight=3)
rk.holds("F006", name="F006", weight=3)
rk.holds("F007", name="F007", weight=3)
rk.holds("F008", name="F008", weight=3)
rk.holds("F009", name="F009", weight=3)
rk.holds("F010", name="F010", weight=3)
rk.holds("F011", name="F011", weight=3)
rk.holds("F012", name="F012", weight=3)
rk.holds("F013", name="F013", weight=3)
rk.holds("F014", name="F014", weight=3)
rk.holds("F015", name="F015", weight=3)
rk.holds("F016", name="F016", weight=3)
rk.holds("F017", name="F017", weight=3)
rk.holds("F018", name="F018", weight=3)
rk.holds("F019", name="F019", weight=3)
rk.holds("F020", name="F020", weight=3)
rk.holds("F021", name="F021", weight=3)
rk.holds("F022", name="F022", weight=3)
rk.holds("F023", name="F023", weight=3)
rk.holds("F024", name="F024", weight=3)
rk.holds("F025", name="F025", weight=3)
rk.holds("F026", name="F026", weight=3)
rk.holds("F027", name="F027", weight=3)
rk.holds("F028", name="F028", weight=3)
rk.holds("F029", name="F029", weight=3)
rk.holds("F030", name="F030", weight=3)
rk.holds("F031", name="F031", weight=3)
rk.holds("F032", name="F032", weight=3)
rk.holds("F033", name="F033", weight=3)
rk.holds("F034", name="F034", weight=3)
rk.holds("F035", name="F035", weight=3)
rk.holds("F036", name="F036", weight=3)
rk.holds("F037", name="F037", weight=3)
rk.holds("F038", name="F038", weight=3)
rk.holds("F039", name="F039", weight=3)
rk.holds("F040", name="F040", weight=3)
rk.holds("F041", name="F041", weight=3)
rk.holds("F042", name="F042", weight=3)
rk.holds("F043", name="F043", weight=3)
rk.holds("F044", name="F044", weight=3)
rk.holds("F045", name="F045", weight=3)
rk.holds("F046", name="F046", weight=3)
rk.holds("F047", name="F047", weight=3)
rk.holds("F048", name="F048", weight=3)
rk.holds("F049", name="F049", weight=3)
rk.holds("F050", name="F050", weight=3)
rk.holds("F051", name="F051", weight=3)
rk.holds("F052", name="F052", weight=3)
rk.holds("F053", name="F053", weight=3)
rk.holds("F054", name="F054", weight=3)
rk.holds("F055", name="F055", weight=3)
rk.holds("F056", name="F056", weight=3)
rk.holds("F057", name="F057", weight=3)
rk.holds("F058", name="F058", weight=3)
rk.holds("F059", name="F059", weight=3)
rk.holds("F060", name="F060", weight=3)
rk.holds("F061", name="F061", weight=3)
rk.holds("F062", name="F062", weight=3)
rk.holds("F063", name="F063", weight=3)
rk.holds("F064", name="F064", weight=3)
rk.holds("F065", name="F065", weight=3)
rk.holds("K1", name="K1", weight=1)
rk.holds("K2", name="K2", weight=1)
rk.holds("K3", name="K3", weight=3)
rk.holds("K4", name="K4", weight=1)
rk.holds("K5", name="K5", weight=1)
