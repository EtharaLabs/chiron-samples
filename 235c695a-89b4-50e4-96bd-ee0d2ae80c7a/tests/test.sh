#!/bin/bash
# Grades a submitted Yosys tree. The compiled grader builds and probes the tree and decides
# Stage I, Stage II and Stage III; Reward Kit turns its verdicts and the judged criteria into
# the reward.
set -uo pipefail
export PYTHONDONTWRITEBYTECODE=1
export PATH="/opt/rewardkit/bin:/usr/sbin:/usr/bin:/sbin:/bin"
umask 077
mkdir -p /logs/verifier/grader
chmod 700 /logs/verifier

# Fail closed: the reward stays 0 unless Reward Kit scores the run. Harbor reads reward.json
# before reward.txt, so on exit Reward Kit's reward.json is kept as reward-keys.json and its
# "reward" value becomes the single reward in reward.txt. The handler runs under umask 077, so
# it repeats the final chmod on what it wrote.
rm -f /logs/verifier/reward.json
echo 0 > /logs/verifier/reward.txt
finish() {
  /usr/bin/python3 - <<'PY'
import json
from pathlib import Path

logs = Path("/logs/verifier")
if (logs / "reward.json").is_file():
    kept = (logs / "reward.json").rename(logs / "reward-keys.json")
    reward = float(json.loads(kept.read_text(encoding="utf-8"))["reward"])
    (logs / "reward.txt").write_text(f"{reward}\n", encoding="utf-8")
else:
    (logs / "reward.txt").write_text("0\n", encoding="utf-8")
PY
  chmod -R go+rX /logs/verifier 2>/dev/null || true
}
trap finish EXIT

# A Stage I failure, or at least veto_k failed Stage II groups, fires the veto.
veto_k=1
cd /opt/grader || exit 1
python3 -m engine grade --tree /app --k "$veto_k" > /logs/verifier/grader/engine.log 2>&1

veto_failed="$(python3 -m engine veto 2>> /logs/verifier/grader/engine.log)" ||
  veto_failed="grader-error: the grader could not report its verdict (exit $?)"
if [ -n "$veto_failed" ]; then
  export VETO_REPORT="$veto_failed"
  veto_route="$(mktemp -d)/route"
  mkdir "$veto_route"
  cp /tests/test_output.py "$veto_route/"
  cat > "$veto_route/reward.toml" <<'VETO_REWARD'
[[reward]]
name = "reward"
aggregation = "weighted-mean"
weights = { test_output = 4, vetoed = 0 }

[[reward]]
name = "compiled"
aggregation = "weighted-mean"
weights = { test_output = 4, vetoed = 0 }

[scoring.test_output]
aggregation = "weighted-mean"
VETO_REWARD
  cat > "$veto_route/vetoed.py" <<'VETO_CRITERION'
import os

import rewardkit as rk


@rk.criterion(description="veto fired: {failed}")
def vetoed(workspace, failed: str) -> float:
    return 0.0


rk.vetoed(os.environ.get("VETO_REPORT") or "unattributed", name="vetoed")
VETO_CRITERION
  rewardkit "$veto_route"
else
# The judged gate is asked up to three times and decided by two matching answers, so one
# misreading by the judge neither zeroes nor clears a run. Each answer's reasoning is kept as
# gate-N-details.json, and the deciding answer is the one the report and the zero route read.
gate_scratch="$(mktemp -d)"
gate_pass=0
gate_fail=0
for vote in 1 2 3; do
  mkdir -p "$gate_scratch/$vote"
  rewardkit /tests/.gate --output "$gate_scratch/$vote/reward.json"
  cp "$gate_scratch/$vote/reward-details.json" "/logs/verifier/gate-$vote-details.json" 2>/dev/null || true
  if grep -Eq '"reward": 1(\.0+)?,?$' "$gate_scratch/$vote/reward.json"; then
    gate_pass=$((gate_pass + 1))
    passed_vote="$vote"
  else
    gate_fail=$((gate_fail + 1))
    failed_vote="$vote"
  fi
  if [ "$gate_pass" -ge 2 ] || [ "$gate_fail" -ge 2 ]; then
    break
  fi
done
if [ "$gate_pass" -ge 2 ]; then
  gate_details="$gate_scratch/$passed_vote/reward-details.json"
  rewardkit /tests
else
  gate_details="$gate_scratch/$failed_vote/reward-details.json"
  export GATE_VERDICT="$gate_details"
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

if ! python3 -m engine ctrf --gate "${gate_details:-}" > /logs/verifier/grader/ctrf.log 2>&1; then
  # The grader itself could not write the report: every declared check is reported unresolved.
  python3 - >> /logs/verifier/grader/ctrf.log 2>&1 <<'GRADER_CTRF'
import json
import time

try:
    with open("/opt/grader/items.json", encoding="utf-8") as handle:
        items = json.load(handle)
except (OSError, ValueError):
    items = {}
names = [name for key in ("stage1", "stage2", "stage3", "gate", "judged") for name in items.get(key, [])]
tests = [
    {"name": name, "status": "other", "duration": 0, "message": "the grader stopped before it reported this check"}
    for name in names
]
tests.append(
    {"name": "grader-error", "status": "failed", "duration": 0, "message": "the grader could not write its report"}
)
now = int(time.time() * 1000)
summary = {"tests": len(tests), "passed": 0, "failed": 1, "skipped": 0, "pending": 0, "other": len(names)}
summary.update(start=now, stop=now)
report = {"results": {"tool": {"name": "yosys-cmake-grader"}, "summary": summary, "tests": tests}}
with open("/logs/verifier/ctrf.json", "w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=1)
GRADER_CTRF
fi

# The verdict stays root-only while submitted code runs. Harbor reads reward.json straight from
# the mounted log directory as the user that runs it, so the finished verdict becomes readable.
chmod -R go+rX /logs/verifier
