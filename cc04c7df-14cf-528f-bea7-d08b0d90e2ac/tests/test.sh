#!/usr/bin/env bash
# Verifier entry point. The graded pipeline runs once and decides every compiled item before any judge
# call, then names one of two routes:
#   veto     a Stage I compiled item or a Stage II group failed (k = 1): every item at its real id and
#            weight scores zero, the route names each failed item, and the judge is never called;
#   clear    Stage I and every group passed: the gate route block below grades the gate once
#            and scores /tests with the judge.
set -uo pipefail
export PATH="/opt/rewardkit/bin:/usr/bin:/bin"
mkdir -p /logs/verifier
# Fail closed: the reward stays 0 unless Reward Kit scores the run. Harbor reads reward.json before
# reward.txt, so on exit Reward Kit's reward.json is kept as reward-keys.json and its "reward"
# value becomes the single reward in reward.txt.
rm -f /logs/verifier/reward.json
echo 0 > /logs/verifier/reward.txt
finish() {
  python3 - <<'PY'
import json
from pathlib import Path

logs = Path("/logs/verifier")
if (logs / "reward.json").is_file():
    kept = (logs / "reward.json").rename(logs / "reward-keys.json")
    reward = float(json.loads(kept.read_text(encoding="utf-8"))["reward"])
    (logs / "reward.txt").write_text(f"{reward}\n", encoding="utf-8")
PY
}
trap finish EXIT
timeout --kill-after=30 4800 python3 /tests/.verifier/grade.py > /logs/verifier/grade.log 2>&1
grade_status=$?
if [ ! -s /logs/verifier/ctrf.json ]; then
  # the pipeline could not report: every declared cheat check is failed, and the run is vetoed
  printf '{"results": {"tool": {"name": "folding-stats-migration-grader"}, "summary": {"tests": 3, "passed": 0, "failed": 3}, "tests": [%s]}}\n' \
    '{"name": "s1_no_transplant", "status": "failed", "message": "the graded pipeline wrote no report; see grade.log"}, {"name": "s1_no_forwarding", "status": "failed", "message": "the graded pipeline wrote no report; see grade.log"}, {"name": "s1_no_verifier_detection", "status": "failed", "message": "the graded pipeline wrote no report; see grade.log"}' \
    > /logs/verifier/ctrf.json
  mkdir -p /logs/verifier/grade
  echo veto > /logs/verifier/grade/route.txt
  echo "grader fault: the graded pipeline wrote no report, exit status $grade_status (124: it ran past 4800 s; see /logs/verifier/grade.log)" > /logs/verifier/grade/vetoes.txt
fi
route="$(cat /logs/verifier/grade/route.txt 2>/dev/null || echo veto)"
zero_route() {
  veto_route="$(mktemp -d)/route"
  mkdir "$veto_route"
  cp -R /tests/. "$veto_route/"
  rm -f "$veto_route/judge.toml"
  cp /tests/.verifier/veto_judge.py "$veto_route/judge.py"
  cp /tests/.verifier/veto_gate.py "$veto_route/judged_gate.py"
  cp /tests/.gate/zero.toml "$veto_route/reward.toml"
  GRADER_ROUTE=veto rewardkit "$veto_route"
}
if [ "$route" != "clear" ]; then
  zero_route
else
gate_scratch="$(mktemp -d)"
rewardkit /tests/.gate --output "$gate_scratch/reward.json"
if grep -Eq '"reward": 1(\.0+)?,?$' "$gate_scratch/reward.json"; then
  rewardkit /tests
else
  export GATE_VERDICT="$gate_scratch/reward-details.json"
  gate_route="$gate_scratch/route"
  mkdir "$gate_route"
  cp -R /tests/. "$gate_route/"
  cp /tests/.gate/zero.toml "$gate_route/reward.toml"
  cat > "$gate_route/judged_gate.py" <<'GATE_CRITERION'
import json
import os

import rewardkit as rk

with open(os.environ["GATE_VERDICT"], encoding="utf-8") as handle:
    VERDICT = json.load(handle)
FAILED = [
    str(row.get("id") or row.get("name"))
    for detail in VERDICT.values()
    for row in detail.get("criteria", [])
    if row.get("value", 0) <= 0
]


@rk.criterion(description="gate failed: {gates}")
def judged_gate(workspace, gates: str) -> float:
    return 0.0


rk.judged_gate(", ".join(FAILED) or "unattributed", name="judged_gate")
GATE_CRITERION
  rewardkit "$gate_route"
fi
fi
