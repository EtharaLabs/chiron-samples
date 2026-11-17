#!/usr/bin/env bash
# Verifier of the Argus Flask-to-FastAPI migration.
#
# R = 1[Stage I] x 1[Stage II] x (0.8 x compiled share + 0.2 x q)
#
# 1. The graded pipeline runs once: it rebuilds /app from clean, offline, against the target
#    wheel set alone, serves it, and decides every compiled item (Stage I S1-S10, Stage II
#    G01-G17, the Stage III families, K1-K5) into /logs/verifier/graded.json.
# 2. The compiled vetoes are decided first. When any Stage I or Stage II item failed, Reward Kit
#    scores the vetoed route: every compiled item and every q criterion registered at its own id
#    and weight, all at 0, each q criterion naming every failed Stage I or Stage II item, and the
#    judge is never called.
# 3. Only a run that clears both vetoes reaches the gate route, which judges J0 and then scores
#    /tests with every item and the q judge, or routes a failed gate to the zero route.
# 4. /logs/verifier/ctrf.json names every compiled item, including the declared cheat checks,
#    on every route.
set -uo pipefail

export PATH=/opt/grader/venv/bin:/opt/python/bin:/usr/local/bin:/usr/bin:/usr/sbin:/bin
mkdir -p /logs/verifier
# Fail closed: the reward stays 0 unless Reward Kit scores the run. Harbor reads reward.json before
# reward.txt, so on exit Reward Kit's reward.json is kept as reward-keys.json and its "reward"
# value becomes the single reward in reward.txt.
rm -f /logs/verifier/reward.json
echo 0 > /logs/verifier/reward.txt
finish() {
  /opt/grader/venv/bin/python - <<'PY'
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

timeout --kill-after=30 3000 /opt/grader/venv/bin/python /opt/grader/grade.py
pipeline=$?
if [ ! -s /logs/verifier/graded.json ]; then
  /opt/grader/venv/bin/python /opt/grader/grade.py --fallback "pipeline exited $pipeline"
fi

if ! /opt/grader/venv/bin/python /opt/grader/grade.py --cleared; then
  veto_route="$(mktemp -d)/route"
  mkdir -p "$veto_route"
  cp /tests/test_output.py /tests/reward.toml /tests/prompt.md "$veto_route/"
  /opt/grader/venv/bin/python /opt/grader/grade.py --veto-judge "$veto_route/judge.py"
  rewardkit "$veto_route"
  exit 0
fi

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
