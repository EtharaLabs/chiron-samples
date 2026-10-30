#!/usr/bin/env python3
# FORGE-CANARY-BEGIN
# FORGE-CANARY-SLOT-1: 60ae0f0c079c366733615a2e6551dc4e64c530f1c56ffed9db295862b101ddc8
# FORGE-CANARY-SLOT-2: 87ab65e4ae3bd13b9cfb24e8b811c4ab101c8552a8b3d1f0d91c32bbd429958a
# FORGE-CANARY-SLOT-3: 76135f235d13f30c4446a51ada3f9a19b6fba2e250fa6b2ffc5a14d1b3f55ffb
# FORGE-CANARY-SLOT-4: e415e80ce167e9922758ef5d43162679b0fe51046883994e0fc30102c1e183f5
# FORGE-CANARY-END
"""Derive every emitted file of this task from solution/grounding.yaml, and from nothing else.

Run `python3 solution/recompute.py` from the bundle root to write them, or with `--check` to
prove the bytes on disk equal the projection. The eleven emitted files are
solution/policy.yaml, solution/solve.sh, solution/TRUTH.md, solution/provenance.yaml,
tests/test_output.py, tests/withheld.py, tests/reward.toml, tests/compiled-reward.toml,
tests/judge.toml, tests/reference.md and tests/test.sh.

Nothing here reads a clock, the environment, the network, a locale or a random source, and no
model is consulted, so two runs on two machines produce the same bytes. Only the standard
library is imported, so the bundle needs nothing installed to regenerate itself.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

GENERATED = "GENERATED SECTION. DO NOT HAND-EDIT."
GROUNDING = "solution/grounding.yaml"
# The environment variable tests/test.sh exports from the stager's stdout, and the only place
# tests/test_output.py reads a compiled verdict from. An environment is fixed at `execve`, so
# nothing the submission left running can rewrite it; a file under /logs/verifier can.
VERDICTS_VARIABLE = "VERIFIER_GRADED_VERDICTS"
# Where tests/verifier/stage_note.py also leaves that payload, for the operator's audit trail
# only. Nothing that scores reads this path. The stager names the same literal.
CACHE_PATH = "/logs/verifier/graded-verdicts.json"
# The stager, read by tests/test_output.py for the one function it needs back: the check for a
# process the submission detached. VERIFIER_ROOT names the same directory in the image.
VERIFIER_ROOT = "/opt/verifier"
STAGER = "/opt/verifier/stage_note.py"


# --------------------------------------------------------------------------- the grounding record


def _scalar(text: str) -> object:
    text = text.strip()
    if text.startswith("{") or text.startswith("["):
        return json.loads(text)
    if len(text) >= 2 and text[0] == text[-1] and text[0] in "\"'":
        return text[1:-1]
    return text


def load(path: Path) -> dict[str, Any]:
    """The grounding record, read by the one shape it is written in and no other.

    The shape is deliberately small: block mappings, block sequences whose every item is one
    JSON flow mapping, and one-line scalars. A richer reader would admit documents this task
    never writes, and would need a dependency the bundle does not have.
    """
    lines = [
        line.rstrip()
        for line in path.read_text(encoding="ascii").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    value, index = _parse(lines, 0, 0)
    if index != len(lines):
        raise ValueError(f"trailing content at line {index + 1}: {lines[index][:40]}")
    if not isinstance(value, dict):
        raise ValueError("the grounding record must be a mapping")
    return value


def _indent(line: str) -> int:
    return len(line) - len(line.lstrip())


def _parse(lines: list[str], index: int, indent: int) -> tuple[Any, int]:
    """The container that starts at `lines[index]` and the index just past it."""
    if lines[index].lstrip().startswith("- "):
        items: list[Any] = []
        while index < len(lines) and _indent(lines[index]) == indent:
            body = lines[index].lstrip()
            if not body.startswith("- "):
                break
            items.append(_scalar(body[2:]))
            index += 1
        return items, index
    mapping: dict[str, Any] = {}
    while index < len(lines) and _indent(lines[index]) == indent:
        key, _, text = lines[index].strip().partition(":")
        index += 1
        if text.strip():
            mapping[key] = _scalar(text)
            continue
        if index < len(lines) and _indent(lines[index]) > indent:
            mapping[key], index = _parse(lines, index, _indent(lines[index]))
        else:
            mapping[key] = {}
    return mapping, index


# --------------------------------------------------------------------------- emitted files


def canary(document: dict[str, Any], marker: str, close: str = "") -> list[str]:
    """The leak-tripwire block for one emitted artifact, in the only slot form item 2 normalizes.

    The tokens live in the grounding record, so an emitted artifact carries the same tripwire a
    planted one does and still recomputes byte for byte. Phase build derives them from the
    canonical bundle hash, which is computed with every slot line normalized, so writing a real
    token into this block never moves the hash it came from.
    """
    tail = f" {close}" if close else ""
    rows = [f"{marker} FORGE-CANARY-BEGIN{tail}"]
    for row in document["canaries"]:
        rows.append(f"{marker} {row}{tail}")
    rows.append(f"{marker} FORGE-CANARY-END{tail}")
    return rows


def solve(document: dict[str, Any]) -> str:
    migration = document["migration"]
    payload_root = migration["payload_root"]
    removed = migration["removed"]
    lines = [
        "#!/bin/sh",
        f"# {GENERATED}",
        "# Derived from " + GROUNDING + " by solution/recompute.py.",
        *canary(document, "#"),
        "#",
        "# Turn State A into State B inside the agent's workspace: drop the C implementation,",
        "# then lay the migration payload over the tree. The payload carries the Rust crate, the",
        "# rewritten CMake build, the package-configuration templates and the design note.",
        "set -eu",
        "",
        'repo="${1:-/workspace/repo}"',
        'payload="$(CDPATH= cd -- "$(dirname -- "$0")/../' + str(payload_root) + '" && pwd)"',
        "",
        'test -f "$repo/src/yyjson.h" || { echo "solve: $repo is not the task repository" >&2; exit 1; }',
        "",
    ]
    for path in removed:
        lines.append(f'rm -f -- "$repo/{path}"')
    lines.append("")
    lines.append("# Every payload path, in the order the grounding record lists it.")
    for row in migration["payload"]:
        path = row["path"]
        parent = str(Path(path).parent)
        if parent != ".":
            lines.append(f'mkdir -p -- "$repo/{parent}"')
        lines.append(f'cp -- "$payload/{path}" "$repo/{path}"')
    lines.append("")
    lines.append('printf "solve: migrated %s\\n" "$repo"')
    return "\n".join(lines) + "\n"


def truth(document: dict[str, Any]) -> str:
    compiled = {row["id"]: row for row in document["items"]["compiled"]}
    judged = {row["id"]: row for row in document["items"]["judged"]}
    deliverables = {row["id"]: row["text"] for row in document["deliverables"]}
    red_lines = {row["id"]: row["text"] for row in document["red_lines"]}
    upstream = document["upstream"]
    lines = [
        GENERATED,
        "",
        *canary(document, "<!--", "-->"),
        "",
        "# How this task is solved",
        "",
        f"The repository is {upstream['canonical_repo']} at release {upstream['release']}, commit",
        f"{upstream['base_commit']}, under the {upstream['licence']} licence. The task moves its",
        "implementation from C to Rust and holds everything observable fixed.",
        "",
        "Each step below names what was done, the state it established, the mistake it survives,",
        "and the graded items it satisfies. The steps are ordered: a later step assumes the state",
        "an earlier one established.",
        "",
    ]
    for step in document["path"]:
        lines.append(f"## {step['id']}")
        lines.append("")
        lines.append(f"Action: {step['action']}")
        lines.append("")
        lines.append(f"Established: {step['state']}")
        lines.append("")
        lines.append(f"Survives: {step['mutation']}")
        lines.append("")
        named = step.get("deliverables") or []
        if named:
            lines.append(
                "Deliverable: "
                + "; ".join(f"{identifier} ({deliverables[identifier]})" for identifier in named)
            )
            lines.append("")
        reds = step.get("red_lines") or []
        if reds:
            lines.append(
                "Red line: "
                + "; ".join(f"{identifier} ({red_lines[identifier]})" for identifier in reds)
            )
            lines.append("")
        checkers = step.get("checkers") or []
        if checkers:
            lines.append("Satisfies:")
            lines.append("")
            for identifier in checkers:
                row = compiled[identifier]
                lines.append(f"- {identifier}: {row['criterion']}")
            lines.append("")
        else:
            lines.append(f"Satisfies no graded item: {step['reason']}")
            lines.append("")
    lines.append("## The judged note")
    lines.append("")
    lines.append(
        "The note at "
        + str(document["judge"]["note_path"])
        + " answers the obligations below. Each is read on its own against this record."
    )
    lines.append("")
    for identifier, row in judged.items():
        lines.append(f"- {identifier}: {row['criterion']}")
    lines.append("")
    lines.append("## Routes that do not solve the task")
    lines.append("")
    for control in document["controls"]:
        lines.append(f"- {control['id']}: {control['what']} Expected: {control['expect']}")
    lines.append("")
    lines.append("## What the grading does not separate")
    lines.append("")
    for gap in document["coverage_gaps"]:
        lines.append(f"- {gap['text']}")
    lines.append("")
    return "\n".join(lines)


def provenance(document: dict[str, Any]) -> str:
    upstream = document["upstream"]
    payload = document["migration"]["payload"]
    lines = [
        f"# {GENERATED}",
        "# Derived from " + GROUNDING + " by solution/recompute.py.",
        *canary(document, "#"),
        "schema: forge.provenance/v1",
        f"run_id: {document['run_id']}",
        f"task_name: {document['task_name']}",
        "",
        "binding:",
        "  canonical_bundle_hash:",
        f"  pinned_image_digest: {document['migration']['pinned_image_digest']}",
        "  binding_envelope:",
        "",
        "source:",
        f"  canonical_repo: {upstream['canonical_repo']}",
        f"  release: {upstream['release']}",
        f"  base_commit: {upstream['base_commit']}",
        f'  event_timestamp: "{upstream["event_timestamp"]}"',
        f"  licence: {upstream['licence']}",
        f'  archive_sha256: "{upstream["archive_sha256"]}"',
        f'  fork_ancestry_snapshot_sha256: "{upstream["fork_ancestry_snapshot_sha256"]}"',
        "  origin: original",
        "  source_kind: public_repository",
        "",
        "attestations:",
        "  ancestry_attestation_digest: attestation-unavailable",
        "  ancestry_attestation_signer: attestation-unavailable",
        "  event_attestation_digest: attestation-unavailable",
        "  event_attestation_signer: attestation-unavailable",
        "  reason: no source-attestation trust root is projected, so the task caps at HOLD and never reaches SHIP on this evidence.",
        "",
        "screening:",
        "  status: screening-unverified",
        "  reason: no screening root is projected, so the exclusion relations and the near-duplicate floor carry no measured calibration.",
        "  empty_submission: scores exactly zero, by the fail-closed gate on the graded build.",
        "",
        "payload:",
    ]
    for row in payload:
        lines.append(f"  - {json.dumps(row, sort_keys=True, ensure_ascii=True)}")
    lines.append("")
    return "\n".join(lines)


def test_output(document: dict[str, Any]) -> str:
    rows = document["items"]["compiled"]
    lines = [
        '"""One Reward Kit criterion per compiled rubric item, all served by one graded run.',
        "",
        f"{GENERATED}",
        "Derived from " + GROUNDING + " by solution/recompute.py.",
        "",
        "Nothing happens when this file is imported beyond registering the criteria. The pipeline",
        "itself ran before Reward Kit started, in tests/verifier/stage_note.py, which is also what",
        "decides whether the submitted note reaches the judge. This file reads the verdicts that run",
        "published and runs nothing itself. Reading them rather than the pipeline is what makes the",
        "compiled verdicts and the judged layer's gate one run rather than two builds.",
        "",
        "The verdicts arrive in the environment, never from a file. The verifier container runs",
        "entirely as uid 1000, so any process the submission's build detached could rewrite a file",
        f"under {Path(CACHE_PATH).parent}/ in the window between the stager writing it and this",
        "process reading it. No process can alter another's environment after `execve`, so the",
        "variable below is a channel the submission cannot reach: tests/test.sh captures the",
        f"stager's stdout and exports it, and {Path(CACHE_PATH).name} is left behind for the",
        "operator's audit trail alone.",
        "",
        "One window stays open, and `_foreign` closes it. Reward Kit must read the staged note as a",
        "file, so a process still alive after the stager wrote it could replace that note with a",
        "better-looking one before the judge sees it. The environment cannot protect a file the",
        "judge opens itself. So on its first call this file re-runs the stager's own check for a",
        "process the submission detached, and if anything foreign is alive it scores every item 0,",
        "which also takes the judged layer to 0 through the gate. The stager swept the tree before",
        "it wrote; anything foreign found here appeared after that sweep, which is the only way the",
        "note on disk can differ from the note the stager staged.",
        "",
        "Every outcome is fail-closed. A missing variable, a malformed one, one with no entry for an",
        "item, and a check that cannot run at all, all leave that item false rather than raising, so",
        "a submission is never credited for an item whose evaluation target does not exist. The",
        "criterion carries the item identifier and nothing else: the prose of each item lives in",
        "the rubric, never in the verifier image.",
        '"""',
        "",
        "from __future__ import annotations",
        "",
        "import json",
        "import os",
        "import sys",
        "import threading",
        "from pathlib import Path",
        "",
        "import rewardkit as rk",
        "from rewardkit import criterion",
        "",
        f'VERDICTS = "{VERDICTS_VARIABLE}"',
        f'STAGER_ROOT = "{VERIFIER_ROOT}"',
        "",
        "ITEMS: tuple[tuple[str, int], ...] = (",
    ]
    for row in rows:
        lines.append(f'    ("{row["id"]}", {row["weight"]}),')
    lines += [
        ")",
        "",
        "_lock = threading.Lock()",
        "_verdicts: dict[str, bool] | None = None",
        "",
        "",
        "def _foreign() -> bool:",
        '    """Whether a process this one did not descend from is alive, fail-closed on any error.',
        "",
        f"    The check is {STAGER}'s own, imported rather than restated, so the",
        "    tree this file demands be quiet is the same tree the stager swept. A failure to run it",
        "    at all is reported as hostile: the image ships the stager, so the check failing means",
        "    the verifier is not the one this task built, and nothing may be credited on that",
        "    footing.",
        '    """',
        "    try:",
        '        root = os.environ.get("VERIFIER_ROOT") or STAGER_ROOT',
        "        if root not in sys.path:",
        "            sys.path.insert(0, root)",
        "        import stage_note",
        "",
        "        return bool(stage_note.foreign(stage_note.ancestry(os.getpid())))",
        "    except BaseException:  # noqa: BLE001 - a check that cannot run credits nothing",
        "        return True",
        "",
        "",
        "def _results() -> dict[str, bool]:",
        '    """The published verdicts of the one graded run, fail-closed in every outcome."""',
        "    global _verdicts",
        "    with _lock:",
        "        if _verdicts is None:",
        "            identifiers = [item for item, _ in ITEMS]",
        "            items: object = None",
        "            if not _foreign():",
        "                try:",
        '                    items = json.loads(os.environ[VERDICTS]).get("items")',
        "                except BaseException:  # noqa: BLE001 - an unreadable channel scores nothing",
        "                    items = None",
        "            if not isinstance(items, dict):",
        "                items = {}",
        "            _verdicts = {item: items.get(item) is True for item in identifiers}",
        "        return _verdicts",
        "",
        "",
        '@criterion(description="{item}")',
        "def graded_item(workspace: Path, item: str) -> bool:",
        "    return _results()[item]",
        "",
        "",
        "for _item, _weight in ITEMS:",
        "    rk.graded_item(_item, weight=_weight, name=_item)",
    ]
    return "\n".join(lines) + "\n"


def reward(document: dict[str, Any]) -> str:
    compiled = sum(int(row["weight"]) for row in document["items"]["compiled"])
    judged = sum(int(row["weight"]) for row in document["items"]["judged"])
    bound = str(document["judge"]["source_value"]) != "none"
    total = compiled + judged if bound else compiled
    lines = [
        f"# {GENERATED}",
        "# Derived from " + GROUNDING + " by solution/recompute.py.",
        "#",
        f"# The reward is the weighted mean over the graded rubric items, whose weights sum to {total}.",
    ]
    if bound:
        lines.append(
            f"# `test_output` carries the {compiled} compiled weight and `judge` the {judged} judged weight."
        )
    else:
        lines.append(
            f"# `test_output` carries the {compiled} compiled weight. The judged layer is not bound yet,"
        )
        lines.append(
            "# so it names no second input: a weight for an absent input is refused by Reward Kit."
        )
    lines += [
        "",
        "[[reward]]",
        'name = "reward"',
        'aggregation = "weighted-mean"',
        "",
        "[reward.weights]",
        f"test_output = {compiled}",
    ]
    if bound:
        lines.append(f"judge = {judged}")
    lines += [
        "",
        "[scoring.test_output]",
        'aggregation = "weighted-mean"',
    ]
    return "\n".join(lines) + "\n"


# ------------------------------------------------------------------- the judged layer's files


def _toml(text: str) -> str:
    """One TOML basic string, with the two characters that can end it escaped."""
    return '"' + str(text).replace("\\", "\\\\").replace('"', '\\"') + '"'


def _staged(document: dict[str, Any]) -> str:
    """The path the stager copies the note to, named once, in the grounding record."""
    files = document["judge"]["files"]
    if len(files) != 1:
        raise ValueError("the judge block must name exactly one staged file")
    return str(files[0])


def _supports(document: dict[str, Any]) -> dict[str, tuple[list[str], list[str]]]:
    """Per judged item, the compiled items its residue names and the steps establishing them.

    This is the only derivation in this generator that reads one field's prose for the
    identifiers of another: a judged item's residue says which compiled relation already holds
    its subject fixed, and the path's `checkers` say which step established that relation. The
    result is the right-hand side of the judged layer, computed rather than written twice.
    """
    compiled = [row["id"] for row in document["items"]["compiled"]]
    support: dict[str, tuple[list[str], list[str]]] = {}
    for row in document["items"]["judged"]:
        residue = str(row["residue"])
        named = [identifier for identifier in compiled if identifier in residue]
        steps = [
            step["id"]
            for step in document["path"]
            if set(step.get("checkers") or []) & set(named)
        ]
        if not steps:
            # No compiled relation touches the item, so the step that writes the note is the
            # only place the reference migration establishes it.
            steps = [
                step["id"]
                for step in document["path"]
                if row["obligation"] in (step.get("deliverables") or [])
            ]
        support[row["id"]] = (named, steps)
    return support


def judge_toml(document: dict[str, Any]) -> str:
    judge = document["judge"]
    staged = _staged(document)
    lines = [
        f"# {GENERATED}",
        "# Derived from " + GROUNDING + " by solution/recompute.py.",
        "#",
        "# One criterion per judged rubric item, each read on its own against /tests/reference.md.",
        "# `name` is the item identifier because Reward Kit's default name is the first forty",
        "# characters of the description, which collides across these items.",
        "#",
        "# Every criterion names the staged note rather than the submitted one. The staged path is",
        "# written only by tests/verifier/stage_note.py, after the graded build, and only for a",
        "# submission that built the shipped library and broke no prohibition; otherwise the path is",
        "# absent and the judge is handed the text `[not found]`, which scores every item zero.",
        "",
        "[judge]",
        f"judge = {_toml(judge['judge'])}",
        f"mode = {_toml(judge['mode'])}",
        f"reasoning_effort = {_toml(judge['reasoning_effort'])}",
        f"timeout = {int(judge['timeout'])}",
        f"files = [{_toml(staged)}]",
        f"reference = {_toml(judge['reference'])}",
    ]
    for row in document["items"]["judged"]:
        lines += [
            "",
            "[[criterion]]",
            f"id = {_toml(row['id'])}",
            f"name = {_toml(row['id'])}",
            f"description = {_toml(row['criterion'])}",
            'type = "binary"',
            f"weight = {int(row['weight'])}",
            f"negate = {'true' if row['negate'] else 'false'}",
            f"files = [{_toml(staged)}]",
            "",
            "[criterion.annotations]",
            'mode = "judged"',
            f"type = {_toml(row['class'])}",
            f"obligation = {_toml(row['obligation'])}",
            f"dimension = {_toml(row['dimension'])}",
            "evidence = [" + ", ".join(_toml(item) for item in row["evidence"]) + "]",
            f"residue = {_toml(row['residue'])}",
        ]
    lines += [
        "",
        "[scoring]",
        'aggregation = "weighted-mean"',
    ]
    return "\n".join(lines) + "\n"


def _note_sections(document: dict[str, Any]) -> dict[str, str]:
    """Each judged item's answer, taken from the reference note's own section for it.

    The note is a payload file with a recorded digest, so the right-hand side the judge
    compares against is the reference implementation's own account rather than a restatement
    of the question. A digest mismatch is fatal: an answer derived from unrecorded bytes would
    grade submissions against something this bundle does not carry.
    """
    rows = [
        row
        for row in document["migration"]["payload"]
        if str(row["path"]).endswith("rust-port.md")
    ]
    if len(rows) != 1:
        raise SystemExit("recompute: the migration payload names no single design note")
    path = Path("solution/port") / str(rows[0]["path"])
    text = path.read_text(encoding="ascii")
    recorded = str(rows[0]["sha256"]).removeprefix("sha256:")
    # The manifest records every payload file over its bytes without the leak-canary block,
    # so a digest stays fixed while each mint plants fresh tokens.
    carried = "".join(
        line for line in text.splitlines(keepends=True) if "FORGE-CANARY" not in line
    )
    actual = hashlib.sha256(carried.encode("ascii")).hexdigest()
    if actual != recorded:
        raise SystemExit(f"recompute: {path} is {actual}, grounding records {recorded}")
    judged = [str(row["id"]) for row in document["items"]["judged"]]
    sections: dict[str, list[str]] = {}
    current: str | None = None
    for line in text.splitlines():
        if line.startswith("## "):
            current = next((item for item in judged if f"({item})" in line), None)
            if current is not None:
                sections[current] = []
            continue
        if current is not None:
            # The section lands under a `###` item heading, so its own subheadings move down
            # one level to keep the answer's structure readable to the judge.
            sections[current].append("#" + line if line.startswith("###") else line)
    missing = [item for item in judged if item not in sections]
    if missing:
        raise SystemExit("recompute: the design note has no section for " + ", ".join(missing))
    return {item: "\n".join(body).strip("\n") for item, body in sections.items()}


def reference(document: dict[str, Any]) -> str:
    note = _note_sections(document)
    compiled = {row["id"]: row for row in document["items"]["compiled"]}
    steps = {step["id"]: step for step in document["path"]}
    red_lines = {row["id"]: row["text"] for row in document["red_lines"]}
    support = _supports(document)
    upstream = document["upstream"]
    judge = document["judge"]
    note_path = str(judge["note_path"])
    described = [
        step
        for step in document["path"]
        if {"D-06", "D-07"} & set(step.get("deliverables") or [])
    ]
    lines = [
        f"<!-- {GENERATED} -->",
        "<!-- Derived from " + GROUNDING + " by solution/recompute.py. -->",
        "",
        "# Reference answer for the judged design note",
        "",
        f"The submission is a migration of {upstream['canonical_repo']} at release",
        f"{upstream['release']}, commit {upstream['base_commit']}, under the",
        f"{upstream['licence']} licence, from C to Rust, with the public header, the C ABI, the",
        "CMake build, the installed tree and every observable behaviour held fixed. The note",
        f"under judgement is {note_path} of the submitted repository, handed to you as the",
        "staged note and the only thing you read about the submission.",
        "",
        "If the staged note reads `[not found]`, the submission did not reach the judged layer:",
        "its graded build did not produce the shipped library, or it broke a prohibition, or it",
        "shipped no note. Answer no to every item.",
        "",
        "## How to read this answer",
        "",
        "This is the migration as the reference performs it, not a form of words. A note may",
        "state the same facts in any order, at any length and in its own terms.",
        "",
        "For each item, answer yes when the note states what the item asks for and states it",
        "correctly against this answer. Answer no when the note omits it, restates the item",
        "without the fact, hedges instead of committing, or contradicts this answer. Length,",
        "confidence and fluency are not evidence. A decision the note does not place in a named",
        "Rust module, type or function is a decision this answer does not credit.",
        "",
        "Each item is read on its own. A fact stated once in the note counts for every item that",
        "asks for it; an item no part of the note addresses is a no, whatever else the note does",
        "well.",
        "",
        "## The port this answer describes",
        "",
        "Each step of the reference migration, with what it did, the state it established, and",
        "the mistake that state survives. A note describes the same port.",
        "",
    ]
    for step in described:
        lines.append(f"### {step['id']}")
        lines.append("")
        lines.append(f"Did: {step['action']}")
        lines.append("")
        lines.append(f"Established: {step['state']}")
        lines.append("")
        lines.append(f"Survives: {step['mutation']}")
        lines.append("")
        reds = step.get("red_lines") or []
        if reds:
            lines.append(
                "Red line: "
                + "; ".join(f"{identifier} ({red_lines[identifier]})" for identifier in reds)
            )
            lines.append("")
        checkers = step.get("checkers") or []
        if checkers:
            lines.append("Held fixed by compiled relation, not by the note:")
            lines.append("")
            for identifier in checkers:
                lines.append(f"- {identifier}: {compiled[identifier]['criterion']}")
            lines.append("")
    lines.append("## The files the reference migration ships")
    lines.append("")
    lines.append(
        "A note names its decisions in these files. A name that is not one of them, or not a"
    )
    lines.append("module, type or function inside one of them, places nothing.")
    lines.append("")
    for row in document["migration"]["payload"]:
        lines.append(f"- {row['path']}")
    lines.append("")
    lines.append("## Item by item")
    lines.append("")
    for row in document["items"]["judged"]:
        named, establishing = support[row["id"]]
        lines.append(
            f"### {row['id']} (weight {row['weight']}, {row['dimension']}, {row['class']})"
        )
        lines.append("")
        lines.append(f"What a correct note states: {row['criterion']}")
        lines.append("")
        lines.append("What this answer states, from the reference note's own account:")
        lines.append("")
        lines.extend(note[row["id"]].splitlines())
        lines.append("")
        if establishing:
            lines.append("Where this answer establishes it:")
            lines.append("")
            for identifier in establishing:
                step = steps[identifier]
                lines.append(f"- {identifier}: {step['action']}")
                lines.append(f"  Established: {step['state']}")
            lines.append("")
        if named:
            lines.append("Already decided by compiled relation, so the note's account of it is")
            lines.append("judged for correctness and not for whether the behaviour works:")
            lines.append("")
            for identifier in named:
                lines.append(f"- {identifier}: {compiled[identifier]['criterion']}")
            lines.append("")
        lines.append(f"What only the note can settle: {row['residue']}")
        lines.append("")
    lines.append("## What this answer does not settle")
    lines.append("")
    lines.append(
        "These are the recorded limits of the grading. None of them is a reason to credit or"
    )
    lines.append("discredit a note, and none of them is a question to put to the note.")
    lines.append("")
    for gap in document["coverage_gaps"]:
        lines.append(f"- {gap['text']}")
    lines.append("")
    return "\n".join(lines)


def test_sh(document: dict[str, Any]) -> str:
    staged = _staged(document)
    concurrency = int(document["judge"]["max_concurrent_llm"])
    lines = [
        "#!/bin/bash",
        f"# {GENERATED}",
        "# Derived from " + GROUNDING + " by solution/recompute.py.",
        "#",
        "# The first step runs the graded pipeline once, sweeps away anything the submission's",
        "# build left running, prints its verdicts as one JSON object on stdout, and decides",
        "# whether the submitted note is staged for the judged layer to read at",
        f"# {staged}",
        "# That JSON goes into the environment, the only channel the compiled criteria read:",
        "# this container runs as uid 1000 throughout, so a file is writable by anything the",
        "# submission left alive, while an environment is fixed at `execve` and no process can",
        "# alter another's.",
        "#",
        "# The judged layer reads the staged note as a FILE, which the environment cannot",
        "# protect: a process the submission detached can re-create the note after the stager",
        "# unlinked it, in the window before the judge opens it. So the judged input is gated",
        "# here, on the one signal that re-creation cannot forge. The gate opens only when the",
        "# stager's own verdict says the sweep converged AND a fresh check finds nothing foreign",
        "# alive now. When it opens, Reward Kit scores /tests, which carries the judge. When it",
        "# does not, Reward Kit scores /tests/.compiled, a root-owned sibling the submission",
        "# cannot write that carries the compiled layer alone, so no note is ever judged on a",
        "# tree that would not go quiet. The compiled layer is 0 there anyway, because its own",
        "# foreign check fails closed. A non-converged run is therefore scored with every",
        "# judged item withheld at zero, so its ceiling is the compiled share alone, and it",
        "# scores exactly 0 whenever that route's own foreign check fires as well.",
        "#",
        "# This script contributes no reward bytes of its own, overrides no judge or model, and",
        "# prints nothing from the environment. The stager always exits 0: a stager that failed",
        "# the run would turn a fail-closed outcome into no outcome at all.",
        "#",
        "# --max-concurrent-llm bounds how many judged items are in flight at once. Without it",
        "# Reward Kit fires all fourteen judged calls together, and a judge-sized call that",
        "# finishes in about twelve seconds alone was measured taking sixty to over three",
        "# hundred seconds under that load, with one call passing the bound 600 second judge",
        "# timeout. A single judged item that times out fails the whole judge reward, which",
        "# writes no reward.json at all, so the bound is what makes the judged layer land. The",
        "# value comes from judge.max_concurrent_llm in",
        f"# {GROUNDING} and from nowhere else. The compiled-only",
        "# route needs no bound because it makes no model call.",
        "set -euo pipefail",
        "",
        f'{VERDICTS_VARIABLE}="$(python3 {STAGER} /workspace/repo)"',
        f"export {VERDICTS_VARIABLE}",
        "",
        "# The gate: converged verdict in the environment, and nothing foreign alive right now.",
        'if python3 - <<"GATE"',
        "import json, os, sys",
        f"sys.path.insert(0, {json.dumps(VERIFIER_ROOT)})",
        "import stage_note",
        f"v = json.loads(os.environ[{json.dumps(VERDICTS_VARIABLE)}])",
        'converged = bool(v.get("judged", {}).get("sweep_converged"))',
        "quiet = not stage_note.foreign(stage_note.ancestry(os.getpid()))",
        "sys.exit(0 if (converged and quiet) else 1)",
        "GATE",
        "then",
        f"    rewardkit /tests --workspace /workspace/repo --output /logs/verifier/reward.json --max-concurrent-llm {concurrency}",
        "else",
        "    rewardkit /tests/.compiled --workspace /workspace/repo --output /logs/verifier/reward.json",
        "fi",
    ]
    return "\n".join(lines) + "\n"


def withheld(document: dict[str, Any]) -> str:
    """The judged layer as /tests/.compiled scores it: every judged item withheld at zero.

    The route test.sh takes when the judged gate stays shut carries no judge TOML, so the
    judged weight would otherwise leave the denominator with it. This module puts that weight
    back by registering one programmatic criterion per judged item, each scoring 0, so the
    ceiling on that route is the compiled weight over the whole rubric weight rather than 1.0.
    Every identifier and weight comes from grounding, never from a literal here.
    """
    rows = document["items"]["judged"]
    lines = [
        '"""One Reward Kit criterion per judged rubric item, each withheld at zero.',
        "",
        f"{GENERATED}",
        "Derived from " + GROUNDING + " by solution/recompute.py.",
        "",
        "The judged layer was withheld because the tree did not go quiet, so each judged item",
        "scores zero here while still carrying its weight.",
        '"""',
        "",
        "from __future__ import annotations",
        "",
        "from pathlib import Path",
        "",
        "import rewardkit as rk",
        "from rewardkit import criterion",
        "",
        "ITEMS: tuple[tuple[str, int], ...] = (",
    ]
    for row in rows:
        lines.append(f'    ("{row["id"]}", {row["weight"]}),')
    lines += [
        ")",
        "",
        "",
        '@criterion(description="{item}")',
        "def withheld_item(workspace: Path, item: str) -> float:",
        "    return 0.0",
        "",
        "",
        "for _item, _weight in ITEMS:",
        "    rk.withheld_item(_item, weight=_weight, name=_item)",
    ]
    return "\n".join(lines) + "\n"


def compiled_reward(document: dict[str, Any]) -> str:
    """The fallback reward spec for /tests/.compiled: the compiled layer, judged items at 0.

    test.sh routes Reward Kit here when the judged gate does not open, so the judge is never
    invoked on a tree that did not go quiet. The judged weight stays in the denominator
    through tests/withheld.py, which scores every judged item 0, so this route can never pay
    more than the compiled weight over the whole rubric weight.
    """
    compiled = sum(int(row["weight"]) for row in document["items"]["compiled"])
    judged = sum(int(row["weight"]) for row in document["items"]["judged"])
    lines = [
        f"# {GENERATED}",
        "# Derived from " + GROUNDING + " by solution/recompute.py.",
        "#",
        "# This route scores the compiled layer with every judged item withheld at zero, so the",
        f"# ceiling on this route is {compiled}/{compiled + judged}. withheld.py carries the judged weight, because the",
        "# directory this spec serves holds no judge TOML and Reward Kit refuses a weight that",
        "# names an absent input. test.sh takes this route only when the tree did not go quiet.",
        "",
        "[[reward]]",
        'name = "reward"',
        'aggregation = "weighted-mean"',
        "",
        "[reward.weights]",
        f"test_output = {compiled}",
        f"withheld = {judged}",
        "",
        "[scoring.test_output]",
        'aggregation = "weighted-mean"',
    ]
    return "\n".join(lines) + "\n"


def policy(document: dict[str, Any]) -> str:
    lines = [
        f"# {GENERATED}",
        "# Derived from " + GROUNDING + " by solution/recompute.py.",
        "#",
        "# Intent only. This manifest never states that a control was enforced: that claim belongs",
        "# to the ronin attestation. It carries no detector rule, threshold, fixture, canary",
        "# material, watched location or audit outcome.",
        *canary(document, "#"),
        "schema: forge.policy/v1",
        f"run_id: {document['run_id']}",
        f"task_name: {document['task_name']}",
        "",
        "channels:",
    ]
    for channel in document["policy_channels"]:
        lines.append(f"  - {json.dumps(channel, sort_keys=True, ensure_ascii=True)}")
    return "\n".join(lines) + "\n"


EMITTED = {
    "solution/policy.yaml": (policy, 0o644),
    "solution/solve.sh": (solve, 0o755),
    "solution/TRUTH.md": (truth, 0o644),
    "solution/provenance.yaml": (provenance, 0o644),
    "tests/test_output.py": (test_output, 0o644),
    "tests/withheld.py": (withheld, 0o644),
    "tests/reward.toml": (reward, 0o644),
    "tests/judge.toml": (judge_toml, 0o644),
    "tests/reference.md": (reference, 0o644),
    "tests/test.sh": (test_sh, 0o755),
    "tests/compiled-reward.toml": (compiled_reward, 0o644),
}


def rebind(text: str, current: str | None) -> str:
    """`text` carrying whatever `binding:` values are already bound on disk.

    Phase build item 2 excludes the binding block from the canonical bundle hash, because the
    hash is what gets written into it. The block is therefore the one part of this artifact the
    mint owns rather than this generator, and regenerating must not clear a bound value.
    """
    if not current:
        return text
    bound = {}
    for line in current.split("\n"):
        stripped = line.strip()
        for key in ("canonical_bundle_hash", "pinned_image_digest", "binding_envelope"):
            if stripped.startswith(key + ":"):
                value = stripped[len(key) + 1 :].strip()
                if value:
                    bound[key] = value
    if not bound:
        return text
    rows = []
    for line in text.split("\n"):
        stripped = line.strip()
        for key, value in bound.items():
            if stripped.startswith(key + ":"):
                indent = line[: len(line) - len(line.lstrip(" \t"))]
                line = f"{indent}{key}: {value}"
                break
        rows.append(line)
    return "\n".join(rows)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare, never write")
    parser.add_argument("--root", default=None, help="the bundle root, its own parent by default")
    arguments = parser.parse_args(argv)
    root = Path(arguments.root) if arguments.root else Path(__file__).resolve().parent.parent
    document = load(root / GROUNDING)
    drift = 0
    for relative, (render, mode) in sorted(EMITTED.items()):
        path = root / relative
        text = render(document)
        current = path.read_text(encoding="ascii") if path.is_file() else None
        if relative == "solution/provenance.yaml":
            text = rebind(text, current)
        if arguments.check:
            state = "same" if current == text else "DRIFT"
            if current != text:
                drift += 1
            digest = hashlib.sha256(text.encode("ascii")).hexdigest()[:16]
            print(f"{state} {relative} {digest}")
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="ascii")
        path.chmod(mode)
        print(f"wrote {relative} ({len(text)} bytes)")
    if not arguments.check:
        judge = document["judge"]
        print(
            f"judge {judge['judge']} in {judge['mode']} mode, reading {_staged(document)}; "
            f"source value: {judge['source_value']}"
        )
    return 1 if drift else 0


if __name__ == "__main__":
    sys.exit(main())
