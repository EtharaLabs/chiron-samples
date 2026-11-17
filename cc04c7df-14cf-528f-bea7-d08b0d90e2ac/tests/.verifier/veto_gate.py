"""Compiled-veto route input (tests/test.sh copies it to the route as judged_gate.py): scores 0 and names the vetoes."""
import os

import rewardkit as rk

with open(os.environ.get("GRADER_VETOES", "/logs/verifier/grade/vetoes.txt"), encoding="utf-8") as handle:
    VETOES = [line.strip() for line in handle if line.strip()]


@rk.criterion(description="compiled veto: {vetoes}")
def judged_gate(workspace, vetoes: str) -> float:
    return 0.0


rk.judged_gate(" | ".join(VETOES) or "unattributed", name="judged_gate")
