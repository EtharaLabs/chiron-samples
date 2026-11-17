#!/bin/bash
# Grading route. The compiled Stage I and Stage II vetoes declared in tests/.veto/veto.toml are decided
# first; a vetoed run is scored on a route where every item scores zero and the judge is never called;
# only a run that clears both reaches the gate route block below.
set -uo pipefail
if [ "$(id -u)" = 0 ]; then
  # Read every collected file whatever mode or owner the agent left.
  # The stock file editor creates new files at mode 600 and Harbor's upload keeps the agent's
  # owner and mode, so a grader running as its own uid cannot copy the tree. Normalize as root,
  # then run every later step, which executes submitted code, as the grading uid 10002, which is
  # not the agent uid 10001.
  chown -R 10002:10002 /app/pydatalab /logs/verifier 2>/dev/null || true
  chmod -R u+rwX,go+rX /app/pydatalab 2>/dev/null || true
  exec env HOME=/home/verifier USER=verifier LOGNAME=verifier \
    setpriv --reuid=10002 --regid=10002 --clear-groups -- /bin/bash "$0" "$@"
fi
export GRADE_RESULTS=/tmp/grade/results.json
export GRADE_JUDGE_INPUT=/tmp/grade/judge
mkdir -p /tmp/grade /logs/verifier
# Fail closed: the reward stays 0 unless Reward Kit scores the run. Harbor reads reward.json before
# reward.txt, so on exit Reward Kit's reward.json is kept as reward-keys.json and its "reward"
# value becomes the single reward in reward.txt.
rm -f /logs/verifier/reward.json
echo 0 > /logs/verifier/reward.txt
finish() {
  /opt/rewardkit/bin/python - <<'PY'
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
}
trap finish EXIT
/opt/rewardkit/bin/python /opt/grader/grade.py /app/pydatalab "$GRADE_RESULTS" --work /tmp/grade > /logs/verifier/grade.log 2>&1
/opt/rewardkit/bin/python /opt/grader/route.py ctrf "$GRADE_RESULTS" /logs/verifier/ctrf.json
cp "$GRADE_RESULTS" /logs/verifier/grade-results.json 2>/dev/null || true
# python3 for the veto route below: the grader's own Python 3.12.
export PATH="/opt/grader-python/bin:$PATH"
veto_scratch="$(mktemp -d)" || exit 1
mkdir "$veto_scratch/compiled" || exit 1
cp -R /tests/. "$veto_scratch/compiled/" || exit 1
rm -rf "$veto_scratch/compiled/.gate" "$veto_scratch/compiled/.veto" || exit 1
find "$veto_scratch/compiled" -name '*.toml' -exec rm -f {} + || exit 1
rewardkit "$veto_scratch/compiled" --output "$veto_scratch/reward.json" || exit 1
veto_status=0
VETO_SCRATCH="$veto_scratch" python3 - <<'VETO_ROUTE' || veto_status=$?
import json
import os
import sys
import tomllib
from pathlib import Path

TESTS = Path("/tests")
SCRATCH = Path(os.environ["VETO_SCRATCH"])


def load(path):
    with path.open("rb") as handle:
        return tomllib.load(handle)


def walk(node):
    if not isinstance(node, dict):
        return
    yield from node.get("criteria") or []
    for part in node.get("components") or []:
        yield from walk(part.get("detail") if isinstance(part, dict) else None)


try:
    sets = load(TESTS / ".veto/veto.toml")["veto"]
    details = json.loads((SCRATCH / "reward-details.json").read_text(encoding="utf-8"))
    rows, values = [], {}
    for detail in details.values():
        for row in walk(detail):
            name, value = str(row["name"]), float(row["value"])
            rows.append((name, float(row.get("weight", 1.0))))
            values[name] = min(value, values.get(name, value))
    for path in sorted(TESTS.rglob("*.toml")):
        parts = path.relative_to(TESTS).parts
        if path.name == "reward.toml" or parts[0] in (".gate", ".veto"):
            continue
        for item in load(path).get("criterion") or []:
            if not isinstance(item.get("id"), str):
                sys.exit(2)
            rows.append((item["id"], float(item.get("weight", 1.0))))
    root = TESTS / "reward.toml"
    tables = load(root).get("reward") or [] if root.exists() else []
    names = ["reward"] + (["compiled"] if any(t.get("name") == "compiled" for t in tables) else [])
    failed, fired = [], False
    for veto in sets:
        broken = [g for g in veto["groups"] if any(values.get(i, 0.0) <= 0 for i in g)]
        fired = fired or len(broken) >= veto["k"]
        failed += [i for g in veto["groups"] for i in g if values.get(i, 0.0) <= 0]
except (OSError, KeyError, TypeError, ValueError, AttributeError):
    sys.exit(2)
if not fired:
    sys.exit(0)
route = SCRATCH / "route"
route.mkdir()
(route / "zeroed.py").write_text(
    "import json\n\nimport rewardkit as rk\n\n\n"
    '@rk.criterion(description="zeroed by the veto: {item}")\n'
    "def zeroed_item(workspace, item: str) -> float:\n    return 0.0\n\n\n"
    f"for name, weight in json.loads({json.dumps(rows)!r}):\n"
    "    rk.zeroed_item(name, name=name, weight=weight)\n",
    encoding="utf-8",
)
(route / "vetoed.py").write_text(
    "import rewardkit as rk\n\n\n"
    '@rk.criterion(description="veto failed: {items}")\n'
    "def vetoed(workspace, items: str) -> float:\n    return 0.0\n\n\n"
    f'rk.vetoed({", ".join(failed)!r}, name="vetoed", weight=0)\n',
    encoding="utf-8",
)
(route / "reward.toml").write_text(
    "".join(
        f'[[reward]]\nname = "{name}"\naggregation = "weighted-mean"\n'
        "weights = { zeroed = 1, vetoed = 0 }\n"
        for name in names
    ),
    encoding="utf-8",
)
sys.exit(10)
VETO_ROUTE
if [ "$veto_status" -eq 10 ]; then
  rewardkit "$veto_scratch/route"
  exit $?
elif [ "$veto_status" -ne 0 ]; then
  exit 1
fi
# The judged gate is asked up to three times and decided by two matching answers, so one misreading
# by the judge neither zeroes nor clears a run. Each answer's reasoning is kept as gate-N-details.json.
gate_scratch="$(mktemp -d)"
gate_pass=0
gate_fail=0
for vote in 1 2 3; do
  mkdir -p "$gate_scratch/$vote"
  rewardkit /tests/.gate --output "$gate_scratch/$vote/reward.json"
  cp "$gate_scratch/$vote/reward-details.json" "/logs/verifier/gate-$vote-details.json" 2>/dev/null || true
  if grep -Eq '"reward": 1(\.0+)?,?$' "$gate_scratch/$vote/reward.json"; then
    gate_pass=$((gate_pass + 1))
  else
    gate_fail=$((gate_fail + 1))
    failed_vote="$vote"
  fi
  if [ "$gate_pass" -ge 2 ] || [ "$gate_fail" -ge 2 ]; then
    break
  fi
done
if [ "$gate_pass" -ge 2 ]; then
  rewardkit /tests
else
  export GATE_VERDICT="$gate_scratch/$failed_vote/reward-details.json"
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
