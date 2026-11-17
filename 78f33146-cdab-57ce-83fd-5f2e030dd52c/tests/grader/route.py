"""The CTRF report for tests/test.sh.

usage:
  route.py ctrf <results.json> <ctrf.json>
"""

import json
import sys
import time
from pathlib import Path

CHEAT_CHECKS = ("S1-transplant", "S1-shim", "S1-v1-namespace", "S1-installed", "S1-verifier-isolation")


def load(path: str) -> dict:
    try:
        return json.loads(Path(path).read_text())
    except (OSError, ValueError):
        return {"items": {}, "groups": {}, "families": {}, "error": "no results"}


def ctrf(results_path: str, out: str) -> None:
    results = load(results_path)
    layout = json.loads((Path(__file__).parent / "data" / "items.json").read_text())
    tests = []
    for section, ids in (("items", layout["stage_i"]), ("groups", layout["stage_ii"]), ("families", layout["stage_iii"])):
        for item in ids:
            row = results.get(section, {}).get(item, {})
            status = "passed" if row.get("status") == "pass" else "failed"
            entry = {"name": item, "status": status, "duration": 0, "message": row.get("reason", "not graded")}
            if item in CHEAT_CHECKS:
                entry["tags"] = ["cheat-check"]
            tests.append(entry)
    now = int(time.time() * 1000)
    passed = sum(t["status"] == "passed" for t in tests)
    report = {
        "results": {
            "tool": {"name": "datalab-pydantic2-grader"},
            "summary": {"tests": len(tests), "passed": passed, "failed": len(tests) - passed, "pending": 0,
                        "skipped": 0, "other": 0, "start": now, "stop": now},
            "tests": tests,
        }
    }
    Path(out).write_text(json.dumps(report, indent=1))


if __name__ == "__main__":
    command = sys.argv[1]
    if command == "ctrf":
        ctrf(sys.argv[2], sys.argv[3])
    else:
        sys.exit(f"unknown command {command}")
