#!/bin/bash
# GENERATED SECTION. DO NOT HAND-EDIT.
# Derived from solution/grounding.yaml by solution/recompute.py.
#
# The first step runs the graded pipeline once, sweeps away anything the submission's
# build left running, prints its verdicts as one JSON object on stdout, and decides
# whether the submitted note is staged for the judged layer to read at
# /logs/verifier/staged-note.md
# That JSON goes into the environment, the only channel the compiled criteria read:
# this container runs as uid 1000 throughout, so a file is writable by anything the
# submission left alive, while an environment is fixed at `execve` and no process can
# alter another's.
#
# The judged layer reads the staged note as a FILE, which the environment cannot
# protect: a process the submission detached can re-create the note after the stager
# unlinked it, in the window before the judge opens it. So the judged input is gated
# here, on the one signal that re-creation cannot forge. The gate opens only when the
# stager's own verdict says the sweep converged AND a fresh check finds nothing foreign
# alive now. When it opens, Reward Kit scores /tests, which carries the judge. When it
# does not, Reward Kit scores /tests/.compiled, a root-owned sibling the submission
# cannot write that carries the compiled layer alone, so no note is ever judged on a
# tree that would not go quiet. The compiled layer is 0 there anyway, because its own
# foreign check fails closed. A non-converged run is therefore scored with every
# judged item withheld at zero, so its ceiling is the compiled share alone, and it
# scores exactly 0 whenever that route's own foreign check fires as well.
#
# This script contributes no reward bytes of its own, overrides no judge or model, and
# prints nothing from the environment. The stager always exits 0: a stager that failed
# the run would turn a fail-closed outcome into no outcome at all.
#
# --max-concurrent-llm bounds how many judged items are in flight at once. Without it
# Reward Kit fires all fourteen judged calls together, and a judge-sized call that
# finishes in about twelve seconds alone was measured taking sixty to over three
# hundred seconds under that load, with one call passing the bound 600 second judge
# timeout. A single judged item that times out fails the whole judge reward, which
# writes no reward.json at all, so the bound is what makes the judged layer land. The
# value comes from judge.max_concurrent_llm in
# solution/grounding.yaml and from nowhere else. The compiled-only
# route needs no bound because it makes no model call.
set -euo pipefail
# Harness fix (permissions): Harbor uploads the agent's files with their original modes, and
# OpenHands' file editor creates every new file 0600, so uid 1000 cannot read them. The
# verifier therefore starts as root, hands the submitted tree to uid 1000 without following
# any symlink, and re-executes this script as uid 1000 before running anything the
# submission wrote.
if [ "$(id -u)" = 0 ]; then
    chown -R -P 1000:1000 /workspace/repo
    chmod -R u+rwX,go+rX /workspace/repo
    exec env HOME=/home/verifier USER=verifier setpriv --reuid=1000 --regid=1000 --init-groups -- /bin/bash "$0" "$@"
fi

VERIFIER_GRADED_VERDICTS="$(python3 /opt/verifier/stage_note.py /workspace/repo)"
export VERIFIER_GRADED_VERDICTS

# The gate: converged verdict in the environment, and nothing foreign alive right now.
if python3 - <<"GATE"
import json, os, sys
sys.path.insert(0, "/opt/verifier")
import stage_note
v = json.loads(os.environ["VERIFIER_GRADED_VERDICTS"])
converged = bool(v.get("judged", {}).get("sweep_converged"))
# Harness fix: with no staged note every judged item scores 0 anyway, so take the withheld
# route and make no judge call; a judge transport failure then cannot abort the reward.
staged = bool(v.get("judged", {}).get("staged"))
quiet = not stage_note.foreign(stage_note.ancestry(os.getpid()))
sys.exit(0 if (converged and quiet and staged) else 1)
GATE
then
    rewardkit /tests --workspace /workspace/repo --output /logs/verifier/reward.json
else
    rewardkit /tests/.compiled --workspace /workspace/repo --output /logs/verifier/reward.json
fi
