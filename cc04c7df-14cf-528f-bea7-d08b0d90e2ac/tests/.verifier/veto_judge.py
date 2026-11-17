"""The veto route: every judged item at its real id and weight, scoring zero, so the judge is never
called for it."""
from pathlib import Path

import rewardkit as rk

ITEMS = [["q_collaborators_injected", 5], ["q_hand_written_singleton", 3], ["q_authentication_single_point", 5], ["q_state_gating_single_point", 3], ["q_cors_single_point", 1], ["q_scheduling_support", 3], ["q_async_execution", 3], ["q_startup_lifecycle", 3], ["q_hand_maintained_route_table", 1], ["q_request_state_in_shared_field", 5]]
WHY = "not judged: a Stage I item or a Stage II group failed"


@rk.criterion(description="{item}: {why}")
def vetoed_item(workspace: Path, item: str, why: str) -> float:
    return 0.0


for item, weight in ITEMS:
    rk.vetoed_item(item, WHY, weight=weight, name=item)
