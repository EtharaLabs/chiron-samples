"""Each compiled item scores its own verdict from the graded pipeline's report on the clear
route and 0 on the veto route; each description carries the verdict reason."""
import json
import os
from pathlib import Path

import rewardkit as rk

GRADE = Path("/logs/verifier/grade")
ITEMS = [["s1_graded_build", 5], ["s1_retired_stack_absent", 5], ["s1_new_stack_serves", 5], ["s1_no_transplant", 5], ["s1_no_forwarding", 5], ["s1_no_verifier_detection", 5], ["g01_hardware_lifecycle", 5], ["g02_team_lifecycle", 5], ["g03_user_lifecycle", 5], ["g04_batch_create", 5], ["g05_authn_authz", 5], ["g06_system_state_gating", 5], ["g07_cache_validation_headers", 5], ["g08_cors", 5], ["g09_invalid_url_redirect", 5], ["g10_competition_user_stats", 5], ["g11_leaderboards", 5], ["g12_historic_stats", 5], ["g13_monthly_results", 5], ["g14_lars_hardware_sync", 5], ["g15_external_stats_parsing", 5], ["g16_reset_and_rollover", 5], ["g17_configuration_binding", 5], ["g18_request_error_representation", 5], ["g19_persistence_across_restart", 5], ["g20_scheduled_parsing", 5], ["s3_family_01", 3], ["s3_family_02", 3], ["s3_family_03", 3], ["s3_family_04", 3], ["s3_family_05", 3], ["s3_family_06", 3], ["s3_family_07", 3], ["s3_family_08", 3], ["s3_family_09", 3], ["s3_family_10", 3], ["s3_family_11", 3], ["s3_family_12", 3], ["s3_family_13", 3], ["s3_family_14", 3], ["s3_family_15", 3], ["s3_family_16", 3], ["s3_family_17", 3], ["s3_family_18", 3], ["s3_family_19", 3], ["s3_family_20", 3], ["s3_family_21", 3], ["s3_family_22", 3], ["s3_family_23", 3], ["s3_family_24", 3], ["s3_family_25", 3], ["s3_family_26", 3], ["s3_family_27", 3], ["s3_family_28", 3], ["s3_family_29", 3], ["s3_family_30", 3], ["s3_family_31", 3], ["s3_family_32", 3], ["s3_family_33", 3], ["s3_family_34", 3], ["s3_family_35", 3], ["s3_family_36", 3], ["s3_family_37", 3], ["s3_family_38", 3], ["s3_family_39", 3], ["s3_family_40", 3], ["s3_family_41", 3], ["s3_family_42", 3], ["s3_family_43", 3], ["s3_family_44", 3], ["s3_family_45", 3], ["s3_family_46", 3], ["s3_family_47", 3], ["s3_family_48", 3], ["s3_family_49", 3], ["s3_family_50", 3], ["s3_family_51", 3], ["s3_family_52", 3], ["s3_family_53", 3], ["s3_family_54", 3], ["s3_family_55", 3], ["s3_family_56", 3], ["s3_family_57", 3], ["s3_family_58", 3], ["s3_family_59", 3], ["s3_family_60", 3], ["s3_family_61", 3], ["s3_family_62", 3], ["s3_family_63", 3], ["s3_family_64", 3], ["s3_family_65", 3]]


def _read(name: str) -> str:
    try:
        return (GRADE / name).read_text(encoding="utf-8")
    except OSError:
        return ""


try:
    VERDICTS = json.loads(_read("verdicts.json"))["items"]
except (ValueError, KeyError, TypeError):
    VERDICTS = {}
ROUTE = os.environ.get("GRADER_ROUTE", "clear")
if ROUTE == "veto":
    SCALE, NOTE = 0.0, " [veto route: a Stage I item or a Stage II group failed]"
else:
    SCALE, NOTE = 1.0, ""


@rk.criterion(description="{item}: {reason}")
def compiled_item(workspace: Path, item: str, reason: str) -> float:
    row = VERDICTS.get(item)
    return SCALE if isinstance(row, dict) and row.get("passed") is True else 0.0


for item, weight in ITEMS:
    row = VERDICTS.get(item)
    verdict = ("passed: " if row.get("passed") else "failed: ") + str(row.get("reason", ""))[:400] \
        if isinstance(row, dict) else "failed: no verdict was written for this item"
    rk.compiled_item(item, verdict + NOTE, weight=weight, name=item)
