#!/bin/bash
# The first step runs the graded pipeline once, sweeps away anything the submission's
# build left running, prints its verdicts as one JSON object on stdout, and decides
# whether the judge packet is staged for the judged layer to read, as 3 parts at
#   /logs/verifier/judge-packet-1.txt
#   /logs/verifier/judge-packet-2.txt
#   /logs/verifier/judge-packet-3.txt
# which is 3 files because Reward Kit reads no judge input over 1 MiB and applies
# that limit to each named file on its own. All of them are written or none is.
# That JSON goes into the environment, the only channel the compiled criteria read:
# every step that grades runs as uid 1000, so a file is writable by anything the
# submission left alive, while an environment is fixed at `execve` and no process can
# alter another's.
#
# This script is ENTERED AS ROOT, and runs nothing the submission wrote with that privilege.
# A real agent creates new files through NamedTemporaryFile, which leaves them at mode
# 0600, and Harbor's separate verifier copies the submission in with `docker compose
# cp`, which preserves the numeric owner and the mode. uid 1000 then cannot read those
# files at all: the graded pipeline's copytree fails with Errno 13, every compiled item
# goes false, and a correct port scores 0 for a reason that has nothing to do with the
# migration. So ownership and modes under /workspace/repo are normalised first, and the
# order is the whole of the fix: chown and chmod need the privilege, so they have to
# run BEFORE the drop, because uid 1000 cannot take ownership of a file afterwards and
# would be left unable to read the very tree it has to grade. Normalising can never
# help a submission, because it rewrites no content: it only removes modes and owners
# from the set of things a verdict can depend on.
#
# Everything after that is handed to uid 1000 with setpriv, because the sweep and the
# staging design both assume the grader is NOT root. Root shares no uid with the
# submission's build, so the write-ordering argument the stager rests on, and the
# foreign-process check that argument depends on, would both be reasoning about the
# wrong user. The one step left at root is the compiled veto route below, which runs no
# submitted code and reads no file the submission can reach: its criteria score the
# verdicts the stager published into the environment, and a fired veto has to end this
# script, which a child shell's `exit` could not.
#
# The judged layer reads the staged packet as FILES, which the environment cannot
# protect: a process the submission detached can re-create any part after the stager
# unlinked it, in the window before the judge opens it. So the judged input is gated
# here, on the signals that re-creation cannot forge. The gate opens only when the
# stager reports it actually staged all 3 parts AND its own verdict says the sweep
# converged AND a fresh check finds nothing foreign alive now.
#
# There are five routes, and they are asked in this order.
#
#   environment  the stager found no execution boundary on this host, or its JSON cannot be
#                read. The build was refused rather than run, so no verdict describes the
#                port, and /tests/.zero below scores the run zero and names that reason.
#   veto route   the compiled veto route. It grades the compiled items
#                alone in a scratch copy, with no judge, and when one of the
#                24 stage I/II items tests/.veto/veto.toml declares is false it scores a
#                zero table where every compiled and judged item keeps its identifier and
#                weight and scores zero, and a criterion of weight 0 names each failed item.
#   zero         the stager says zero for another reason: no shipped library, or a judge
#                packet that could not carry the submission's collected source to its last
#                line -- in which case the stager staged no part at all and its reason names
#                the files that did not fit in the 2400000 bytes the 3 parts hold.
#                /tests/.zero carries every compiled item and every judged item at its real
#                identifier and weight, returns zero for all of them, and calls no judge. The
#                reason reaches the criteria as the stager's `route_reason` in this JSON.
#   gate route   all 3 packet parts were staged, the sweep converged, and nothing
#                foreign is alive.
#
#                The block below runs as uid 1000 through the same grade wrapper as
#                everything else. It grades
#                /tests/.gate first; a gate that scores 1 goes on to /tests with the
#                judged layer, and a gate that does not is scored against
#                /tests/.gate/zero.toml, whose `reward` is an all-pass over a gate input
#                the template registers at zero.
#   zero again   the packet was not staged in full, or the tree did not go quiet. The gate
#                cannot be read on a packet the submission may have rewritten, so the run
#                fails closed to the zero route and the zero criteria name that reason,
#                which reaches them through VERIFIER_ZERO_REASON.
#
# The outer table splits `test_output` and `judge` 4 to 1 on every route that pays
# anything, and on the zero route as well, so a zeroed run is reported at the shape it
# would have been paid at rather than as a bare number. The veto route keeps its template's
# own table: every item under one input, at its own weight.
#
# This script contributes no reward bytes of its own, overrides no judge or model, and
# prints nothing from the environment. The stager always exits 0: a stager that failed
# the run would turn a fail-closed outcome into no outcome at all.
set -euo pipefail

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
else:
    (logs / "reward.txt").write_text("0\n", encoding="utf-8")
PY
}
trap finish EXIT

# Normalise while the privilege to do it still exists. See above: after the drop this
# is impossible, so doing it in this order is what keeps the graded read working.
chown -R 1000:1000 /workspace/repo
chmod -R u+rwX /workspace/repo

# From here on every step that runs or reads the submission runs as uid 1000. The veto
# route does neither, and stays at root (see above).
grade() { setpriv --reuid 1000 --regid 1000 --clear-groups -- "$@"; }

VERIFIER_GRADED_VERDICTS="$(grade python3 /opt/verifier/stage_note.py /workspace/repo)"
export VERIFIER_GRADED_VERDICTS

# The environment, asked first: on a host that offers no execution boundary the stager
# refused the build rather than run it, so no verdict describes the port and the veto
# route would only name items that were never measured. The stager decided it; this step
# only reads that decision, and fails closed to the zero route if it cannot.
if grade python3 - <<"ENVIRONMENT"
import json, os, sys
try:
    v = json.loads(os.environ["VERIFIER_GRADED_VERDICTS"])
    refused = v.get("boundary_available") is not True
except BaseException:
    refused = True
sys.exit(0 if refused else 1)
ENVIRONMENT
then
    grade rewardkit /tests/.zero --workspace /workspace/repo --output /logs/verifier/reward.json
    exit 0
fi

# The compiled veto route, below, at root (see above). It reads the
# compiled veto from /tests/.veto/veto.toml and writes nothing outside its own mktemp
# directory and, when a veto fires, the default /logs/verifier/reward.json.
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

# The zero route, asked next because it answers a different question from the judged
# gate below: not whether the packet may be read, but whether anything may be credited at
# all. The stager decided it from what it measured; this step only reads that
# decision, and fails closed to the zero route if it cannot.
if grade python3 - <<"ZERO"
import json, os, sys
try:
    v = json.loads(os.environ["VERIFIER_GRADED_VERDICTS"])
    gated = v.get("route") == "zero"
except BaseException:
    gated = True
sys.exit(0 if gated else 1)
ZERO
then
    grade rewardkit /tests/.zero --workspace /workspace/repo --output /logs/verifier/reward.json
    exit 0
fi

# The gate: all 3 packet parts were staged, the verdict converged, and nothing
# foreign is alive now.
if grade python3 - <<"GATE"
import json, os, sys
sys.path.insert(0, "/opt/verifier")
import stage_note
v = json.loads(os.environ["VERIFIER_GRADED_VERDICTS"])
staged = bool(v.get("judged", {}).get("staged"))
converged = bool(v.get("judged", {}).get("sweep_converged"))
quiet = not stage_note.foreign(stage_note.ancestry(os.getpid()))
sys.exit(0 if (staged and converged and quiet) else 1)
GATE
then
# The judged-gate route. It takes the reward table it needs from
# /tests/.gate and writes nothing outside its own mktemp directory and the default
# /logs/verifier/reward.json, which uid 1000 owns.
grade bash -s <<'GATE_ROUTE'
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
GATE_ROUTE
else
    # Fail closed. The packet is the judged layer's whole input and the gate's as well, so
    # a packet that was never staged, or a tree that would not go quiet around it, leaves
    # nothing a judge could be asked about honestly.
    export VERIFIER_ZERO_REASON="the judge packet was not staged, or the tree did not go quiet after the sweep, so the judged gate could not be read on a packet the submission cannot have rewritten"
    grade rewardkit /tests/.zero --workspace /workspace/repo --output /logs/verifier/reward.json
fi
