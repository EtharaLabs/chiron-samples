"""One Reward Kit criterion per compiled rubric item, each scoring zero.

The grading host does not offer the execution boundary the graded build is declared to
run in, the graded build shipped no library, the judge packet could not carry the
submission's whole collected source, or it could not be staged and swept so the judged
gate could not be read at all. Every item keeps its identifier and its weight and
scores zero, and no judge is called. A false Stage I or Stage II item never reaches this
module: the compiled veto route in tests/test.sh scores that run first.

Each criterion's description carries WHY the run was gated, because a bare zero cannot
be told apart from a port that failed to build. The reason comes from the environment,
either VERIFIER_ZERO_REASON when tests/test.sh failed the gate closed or the stager's
own route reason otherwise, and it is the only thing this module reads.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import rewardkit as rk
from rewardkit import criterion

ITEMS: tuple[tuple[str, int], ...] = (
    ("T-02", 5),
    ("T-03", 5),
    ("T-04", 5),
    ("T-05", 5),
    ("T-06", 5),
    ("T-07", 5),
    ("T-08", 5),
    ("T-09", 5),
    ("T-10", 5),
    ("T-11", 5),
    ("T-12", 5),
    ("T-13", 5),
    ("T-14", 5),
    ("T-15", 5),
    ("T-16", 5),
    ("T-17", 5),
    ("S3-X01-bom", 3),
    ("S3-X01-comma", 3),
    ("S3-X01-comment", 3),
    ("S3-X01-errpos", 3),
    ("S3-X01-esc", 3),
    ("S3-X01-esckey", 3),
    ("S3-X01-json5", 3),
    ("S3-X01-trunc", 3),
    ("S3-X01-utf8", 3),
    ("S3-X01-ws", 3),
    ("S3-X01-trunc23", 3),
    ("S3-X02-d17", 3),
    ("S3-X02-d768", 3),
    ("S3-X02-diyfp", 3),
    ("S3-X02-e21", 3),
    ("S3-X02-expext", 3),
    ("S3-X02-fuzz", 3),
    ("S3-X02-halfrnd", 3),
    ("S3-X02-halfway", 3),
    ("S3-X02-hexinf", 3),
    ("S3-X02-intbound", 3),
    ("S3-X02-longdig", 3),
    ("S3-X02-pow2", 3),
    ("S3-X02-rnd", 3),
    ("S3-X02-round", 3),
    ("S3-X02-subnormal", 3),
    ("S3-X03-num", 3),
    ("S3-X03-str", 3),
    ("S3-X04-build_then_patch", 3),
    ("S3-X04-copy_roundtrip", 3),
    ("S3-X04-copy_scalars", 3),
    ("S3-X04-deep_nest", 3),
    ("S3-X04-equality", 3),
    ("S3-X04-mutpatch_all_mutable", 3),
    ("S3-X04-patch_fails_midway", 3),
    ("S3-X04-patch_on_empty", 3),
    ("S3-X04-patch_op_names", 3),
    ("S3-X04-ptr_array_tokens", 3),
    ("S3-X04-ptr_ctx_after_mutation", 3),
    ("S3-X04-ptr_escapes", 3),
    ("S3-X04-ptr_then_mutate", 3),
    ("S3-X04-fuzz", 3),
    ("S3-X05-build_arr_grow", 3),
    ("S3-X05-build_obj", 3),
    ("S3-X05-copy_cycle", 3),
    ("S3-X05-dupkey_copy", 3),
    ("S3-X05-dyn_strings", 3),
    ("S3-X05-merge_patch", 3),
    ("S3-X05-mutpatch_mutable", 3),
    ("S3-X05-numbers", 3),
    ("S3-X05-patch_add_copy", 3),
    ("S3-X05-patch_fail_mid", 3),
    ("S3-X05-ptr_create_parent", 3),
    ("S3-X05-ptr_mod_seq", 3),
    ("S3-X05-read_escapes", 3),
    ("S3-X05-read_write", 3),
    ("S3-X05-strpool", 3),
    ("S3-X05-valpool", 3),
    ("S3-X05-write_big", 3),
    ("S3-X05-fuzz", 3),
    ("T-23", 5),
    ("T-24", 5),
    ("P-01", 5),
    ("P-02", 5),
    ("P-03", 5),
    ("P-04", 5),
    ("P-05", 5),
    ("P-06", 5),
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


@criterion(description="{item} scored zero on the gated route. " + WHY)
def gated_item(workspace: Path, item: str) -> float:
    return 0.0


for _item, _weight in ITEMS:
    rk.gated_item(_item, weight=_weight, name=_item)
