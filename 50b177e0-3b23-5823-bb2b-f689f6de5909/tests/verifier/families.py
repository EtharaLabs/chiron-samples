#!/usr/bin/env python3
"""The stage-3 differential families: which recorded line belongs to which graded item.

Stage 3 used to be five items, one per harness module. It is sixty now, cut out of the same
1543-case corpus with no new recording: each module still runs ONCE through the harness
exactly as before, and this module decides which of the record lines that run produced belong
to which family. The split is the corpus's own shape -- a subdirectory for the three directory
modules, a script for the two script modules -- so a family is a group of cases that were
already recorded together, never a new measurement.

The table below is the whole of the split and it is a literal, not a directory scan. A scan
would make the graded item set depend on what the image happens to carry, and `validate`
exists so the image build fails rather than the grading drifting: it is run from
tests/verifier/freeze.sh against tests/verifier/stage3/manifest.jsonl and refuses the image
unless every one of the 1543 cases lands in exactly one family and every family is non-empty.

A verdict is a comparison of record lines, not of digests. `partition` reads one record stream
and returns, per family, the lines whose `case=` names a case of that family, in the order the
stream produced them. The expected side is the frozen archive the pipeline already reads for
first_divergence, and the produced side is the submission's. A family agrees when the two
sequences are equal. HDR and END lines carry no `case=` and belong to no family, so a
difference in them is no family's failure -- the library version they report is held by T-14
and T-02 instead. A record that stops early, which is what a crash looks like, leaves the
families whose lines never arrived short and so failing, and leaves the families that had
already been written through intact: that asymmetry is the point of cutting the module up.
"""

from __future__ import annotations

import json
import sys
import urllib.parse
from pathlib import Path

# The three directory modules: one family per corpus subdirectory, except trunc2 and trunc3,
# which are one family together. Order is this tuple's order, and it is what numbers the
# items of the rubric, so it may not be sorted on a whim.
DIRECTORY_FAMILIES: dict[str, tuple[tuple[str, tuple[str, ...]], ...]] = {
    "X-01": (
        ("bom", ("bom",)),
        ("comma", ("comma",)),
        ("comment", ("comment",)),
        ("errpos", ("errpos",)),
        ("esc", ("esc",)),
        ("esckey", ("esckey",)),
        ("json5", ("json5",)),
        ("trunc", ("trunc",)),
        ("utf8", ("utf8",)),
        ("ws", ("ws",)),
        ("trunc23", ("trunc2", "trunc3")),
    ),
    "X-02": (
        ("d17", ("d17",)),
        ("d768", ("d768",)),
        ("diyfp", ("diyfp",)),
        ("e21", ("e21",)),
        ("expext", ("expext",)),
        ("fuzz", ("fuzz",)),
        ("halfrnd", ("halfrnd",)),
        ("halfway", ("halfway",)),
        ("hexinf", ("hexinf",)),
        ("intbound", ("intbound",)),
        ("longdig", ("longdig",)),
        ("pow2", ("pow2",)),
        ("rnd", ("rnd",)),
        ("round", ("round",)),
        ("subnormal", ("subnormal",)),
    ),
    "X-03": (
        ("num", ("num",)),
        ("str", ("str",)),
    ),
}

# The two script modules: one family per named .vm3 script, plus one family for every
# fuzz_NNN.vm3 script of that module together. The fuzz family is last in both.
SCRIPT_FAMILIES: dict[str, tuple[str, ...]] = {
    "X-04": (
        "build_then_patch",
        "copy_roundtrip",
        "copy_scalars",
        "deep_nest",
        "equality",
        "mutpatch_all_mutable",
        "patch_fails_midway",
        "patch_on_empty",
        "patch_op_names",
        "ptr_array_tokens",
        "ptr_ctx_after_mutation",
        "ptr_escapes",
        "ptr_then_mutate",
    ),
    "X-05": (
        "build_arr_grow",
        "build_obj",
        "copy_cycle",
        "dupkey_copy",
        "dyn_strings",
        "merge_patch",
        "mutpatch_mutable",
        "numbers",
        "patch_add_copy",
        "patch_fail_mid",
        "ptr_create_parent",
        "ptr_mod_seq",
        "read_escapes",
        "read_write",
        "strpool",
        "valpool",
        "write_big",
    ),
}

MODULES: tuple[str, ...] = ("X-01", "X-02", "X-03", "X-04", "X-05")

# The one family of a script module that is not a single script: every fuzz_NNN.vm3 of it.
FUZZ = "fuzz"
FUZZ_PREFIX = "fuzz_"

# Where a family's identifier comes from, named once. "X-01" plus "bom" is "S3-X01-bom", which
# is what the rubric carries, what tests/test_output.py registers and what the
# CTRF report joins on. Reward Kit requires a criterion name matching ^[a-zA-Z0-9_-]{1,64}$,
# which every identifier this builds satisfies.
def identifier(module: str, key: str) -> str:
    return f"S3-{module.replace('-', '')}-{key}"


def _directory_label(keys: tuple[str, ...]) -> str:
    return " and ".join(f"{key}/" for key in keys)


def families() -> tuple[tuple[str, str, str, str], ...]:
    """Every family in item order: its identifier, its module, its key and its English scope.

    The scope is the phrase the criterion text uses, so the rubric's prose and the runtime's
    partitioning are derived from the same table rather than written twice.
    """
    rows: list[tuple[str, str, str, str]] = []
    for module in MODULES:
        if module in DIRECTORY_FAMILIES:
            for key, directories in DIRECTORY_FAMILIES[module]:
                rows.append(
                    (identifier(module, key), module, key, f"under {_directory_label(directories)}")
                )
            continue
        for script in SCRIPT_FAMILIES[module]:
            rows.append(
                (identifier(module, script), module, script, f"from {script}.vm3")
            )
        rows.append(
            (identifier(module, FUZZ), module, FUZZ, f"from the {FUZZ_PREFIX}NNN.vm3 scripts")
        )
    return tuple(rows)


def module_families(module: str) -> tuple[str, ...]:
    """Every family identifier of one module, in item order."""
    return tuple(row[0] for row in families() if row[1] == module)


def family_of(case: str) -> str | None:
    """The family identifier a corpus-relative case path belongs to, or None.

    `case` is a path as the corpus carries it: "X-01/bom/u16be_alone.json" or
    "X-04/build_then_patch.vm3". A path this cannot place returns None rather than raising,
    and the caller fails every family of the module closed: an unplaceable case means the
    record and this table disagree about the corpus, which is not a thing to credit.
    """
    parts = case.split("/")
    if len(parts) < 2:
        return None
    module = parts[0]
    if module in DIRECTORY_FAMILIES:
        if len(parts) != 3:
            return None
        for key, directories in DIRECTORY_FAMILIES[module]:
            if parts[1] in directories:
                return identifier(module, key)
        return None
    if module in SCRIPT_FAMILIES:
        if len(parts) != 2 or not parts[1].endswith(".vm3"):
            return None
        stem = parts[1][: -len(".vm3")]
        if stem in SCRIPT_FAMILIES[module]:
            return identifier(module, stem)
        if stem.startswith(FUZZ_PREFIX) and stem[len(FUZZ_PREFIX) :].isdigit():
            return identifier(module, FUZZ)
        return None
    return None


def case_of(line: str) -> str | None:
    """The corpus-relative case path a record line names, or None for a line that names none.

    The harness writes the path URL-encoded and appends its own per-case discriminator: an
    index after `%3A`, which is a colon it encoded, or after `#`, which it leaves literal. The
    cut is made on the encoded text and before decoding, so a colon or a hash inside a real
    file name could not be mistaken for the discriminator. HDR and END lines carry no `case=`
    at all and are the reason this returns None rather than raising.
    """
    marker = " case="
    at = line.find(marker)
    if at < 0:
        return None
    rest = line[at + len(marker) :]
    token = rest.split(" ", 1)[0].strip()
    if not token:
        return None
    for cut in ("%3A", "%3a", "#"):
        index = token.find(cut)
        if index >= 0:
            token = token[:index]
    return urllib.parse.unquote(token) if token else None


def partition(lines, module: str) -> tuple[dict[str, list[str]], list[str]]:
    """One record stream split per family, plus every line this table could not place.

    The second return value is the fail-closed channel. It holds the lines that carry a
    `case=` naming something no family of `module` covers, including a case of another
    module. For a record the image froze it is empty; a non-empty one means the stream is not
    the stream this split was cut for, and the caller fails every family of the module.
    """
    buckets: dict[str, list[str]] = {name: [] for name in module_families(module)}
    unplaced: list[str] = []
    for line in lines:
        case = case_of(line)
        if case is None:
            continue
        name = family_of(case)
        if name is None or name not in buckets:
            unplaced.append(line)
            continue
        buckets[name].append(line)
    return buckets, unplaced


def validate(manifest: Path) -> list[str]:
    """Every disagreement between this table and the stage-3 manifest, empty when there is none.

    Run from tests/verifier/freeze.sh at image build time. Each of the manifest's cases must
    land in exactly one family, and each family must hold at least one case: a family with no
    case would be a graded item that passes without measuring anything.
    """
    problems: list[str] = []
    counts = {name: 0 for name, _, _, _ in families()}
    with manifest.open(encoding="ascii") as handle:
        for number, text in enumerate(handle, start=1):
            text = text.strip()
            if not text:
                continue
            try:
                row = json.loads(text)
            except ValueError as error:
                problems.append(f"manifest line {number} is not JSON: {error}")
                continue
            case = str(row.get("path", ""))
            name = family_of(case)
            if name is None:
                problems.append(f"manifest line {number}: {case} lands in no family")
                continue
            declared = str(row.get("family", ""))
            if declared and not name.startswith(f"S3-{declared.replace('-', '')}-"):
                problems.append(
                    f"manifest line {number}: {case} is family {declared} but lands in {name}"
                )
                continue
            counts[name] += 1
    empty = sorted(name for name, count in counts.items() if count == 0)
    problems.extend(f"family {name} holds no case" for name in empty)
    return problems


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"usage: {argv[0]} <stage3-manifest.jsonl>", file=sys.stderr)
        return 2
    rows = families()
    problems = validate(Path(argv[1]))
    for problem in problems:
        print(f"families: {problem}", file=sys.stderr)
    if problems:
        return 1
    print(f"families: {len(rows)} stage-3 families agree with {argv[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
