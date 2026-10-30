"""One graded pipeline: a submitted repository in, one boolean per compiled rubric item out.

The pipeline is fail-closed. Every item starts false and becomes true only when a record
exists that shows it. The graded build is the gate: an item's evaluation target is a record
the pipeline writes after that build completes, so a submission whose build does not run
earns nothing, including the prohibition items, whose absence is only creditable for a
submission that actually built something.

Nothing is compared against a constant written here. State A's header digest, install
manifests, symbol tables, behavioural records and consumer records are all frozen into the
image from State A itself at image build time, and `stages.frozen()` is the only source of
the right-hand side of every relation.
"""

from __future__ import annotations

import json
import os
import shutil
import time
from pathlib import Path
from typing import Any

import probe
import stages

# Each behavioural module and each differential family is one item, in the order the rubric
# numbers them. The mapping is the whole of what this pipeline knows about the rubric.
STAGE2_ITEMS: dict[str, str] = dict(
    zip(
        stages.STAGE2_MODULES,
        ("T-06", "T-07", "T-08", "T-09", "T-10", "T-11", "T-12", "T-13", "T-14", "T-15"),
    )
)
STAGE3_ITEMS: dict[str, str] = dict(
    zip(stages.STAGE3_FAMILIES, ("T-18", "T-19", "T-20", "T-21", "T-22"))
)

LOG_DIRECTORY = Path(os.environ.get("VERIFIER_LOG_DIR", "/logs/verifier"))


class Journal:
    """The run's narrative: what each stage did, written beside the reward for the operator."""

    def __init__(self) -> None:
        self.notes: list[str] = []
        self.started = time.monotonic()

    def say(self, note: str) -> None:
        self.notes.append(f"[{time.monotonic() - self.started:7.2f}s] {note}")

    def save(self, results: dict[str, bool]) -> None:
        payload = {
            "items": results,
            "seconds": round(time.monotonic() - self.started, 2),
            "log": self.notes,
        }
        for destination in (LOG_DIRECTORY / "graded-pipeline.json",):
            try:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_text(json.dumps(payload, indent=2, sort_keys=True))
            except OSError:
                pass


def _scratch() -> Path:
    where = Path(os.environ.get("VERIFIER_SCRATCH", "/tmp/verifier-run"))  # noqa: S108
    shutil.rmtree(where, ignore_errors=True)
    for child in ("home", "stage2", "stage3", "consumer"):
        (where / child).mkdir(parents=True, exist_ok=True)
    return where


def evaluate(workspace: Path, item_ids: list[str]) -> dict[str, bool]:
    """Run every stage once and return each item's verdict, never raising."""
    results = {item: False for item in item_ids}
    journal = Journal()
    try:
        _run(workspace, results, journal)
    except BaseException as error:  # noqa: BLE001 - a broken pipeline scores nothing, loudly
        journal.say(f"pipeline aborted: {type(error).__name__}: {error}")
    journal.save(results)
    return results


def _run(workspace: Path, results: dict[str, bool], journal: Journal) -> None:
    reference = stages.frozen()
    scratch = _scratch()

    # ---------------------------------------------------------------- stage 1: the snapshot
    source = scratch / "repo"
    shutil.copytree(workspace, source, symlinks=True, ignore_dangling_symlinks=True)
    tree = probe.snapshot(source)
    journal.say(
        "snapshot: %d files, %d translation unit(s), %d shipped artefact(s)"
        % (len(tree["files"]), len(tree["translation_units"]), len(tree["artifacts"]))
    )

    # ------------------------------------------------------------- stage 2: the graded build
    ledger = scratch / "ledger.jsonl"
    environment = stages.build_environment(scratch, ledger)
    if environment is None:
        journal.say("graded build: the image is missing a real toolchain")
        return
    prefixes = {kind: scratch / f"prefix-{kind}" for kind in stages.INSTALL_KINDS}
    for kind in stages.INSTALL_KINDS:
        built, log = stages.graded_build(
            source, scratch / f"build-{kind}", prefixes[kind], kind, environment
        )
        journal.say(f"graded build {kind}: {'ok' if built else 'FAILED at ' + log[-1][0]}")
        if not built:
            journal.say(log[-1][1].tail())
            return

    # ------------------------------------------------- stage 3: what the build ledger records
    # Read before any later build appends to it: these items judge the graded build alone.
    entries = probe.ledger_entries(ledger)
    refused = [entry for entry in entries if entry.get("refused")]
    shipped = probe.shipped_inputs(entries, tree["artifacts"])
    crates = probe.crate_findings(
        source,
        probe.cargo_manifests(source, entries),
        probe.real_tool("cargo"),
        scratch / "cargo-probe",
    )
    results["P-01"] = not refused
    results["P-02"] = not shipped
    results["P-05"] = not crates
    results["P-04"] = tree["header_sha256"] == reference["header_sha256"]
    original = stages.ROOT / "statea"
    embedded = probe.embedded_implementation(
        source, probe.implementation_spans(original / "yyjson.c", original / "yyjson.h")
    )
    results["P-06"] = not tree["translation_units"] and not embedded["over"]
    journal.say(
        "ledger: %d invocation(s), %d refused, shipped inputs %s, crate findings %s"
        % (len(entries), len(refused), shipped or "none", crates or "none")
    )
    journal.say(
        "source tree: header %s, remaining translation unit(s) %s, original implementation "
        "%d/%d spans (%.2f%%) in %s"
        % (
            "unchanged" if results["P-04"] else "CHANGED",
            tree["translation_units"] or "none",
            embedded["carried"],
            embedded["total"],
            100.0 * embedded["share"],
            embedded["files"] or "no file",
        )
    )

    # --------------------------------------- stage 4: the install trees and the shipped library
    installed = {kind: probe.install_manifest(prefixes[kind]) for kind in stages.INSTALL_KINDS}
    results["T-04"] = all(
        installed[kind] == [list(row) for row in reference["install"][kind]]
        for kind in stages.INSTALL_KINDS
    )
    library = probe.shipped_library(prefixes["shared"], "shared")
    if library is None:
        journal.say("shipped library: libyyjson.so does not resolve to a file")
        return
    exports = probe.exported_symbols(library)
    imports = probe.imported_symbols(library)
    dynamic = probe.dynamic_section(library)
    if exports is None or imports is None or dynamic is None:
        journal.say("shipped library: its symbol table could not be read")
        return
    results["T-02"] = exports == sorted(reference["exports"])
    results["T-03"] = dynamic["soname"] == reference["soname"]
    results["T-05"] = set(dynamic["needed"]) <= set(reference["needed"])
    results["P-03"] = not (set(imports) & probe.FORBIDDEN_IMPORTS)
    journal.say(
        "library: %d export(s), soname %s, needed %s, forbidden imports %s"
        % (
            len(exports),
            dynamic["soname"],
            dynamic["needed"],
            sorted(set(imports) & probe.FORBIDDEN_IMPORTS) or "none",
        )
    )
    journal.say(
        "install trees: %s"
        % ("exactly State A's paths" if results["T-04"] else "paths differ from State A's")
    )

    # ------------------------------------------------------------- stage 5: behavioural replay
    _behaviour(
        stages.OBJECTS / "stage2",
        stages.CORPUS,
        stages.STAGE2_MODULES,
        STAGE2_ITEMS,
        reference["stage2"],
        stages.RECORDS / "stage2",
        prefixes["shared"],
        scratch / "stage2",
        results,
        journal,
    )

    # --------------------------------------------------------------------- stage 6: consumers
    record = stages.consumer_record(prefixes, scratch / "consumer")
    results["T-16"] = record == reference["consumers"]
    journal.say(
        "consumers: %s"
        % ("six routes agree with State A" if results["T-16"] else "a route diverges")
    )
    if not results["T-16"]:
        journal.say("consumer record:\n" + record.rstrip())

    # ------------------------------------------------------------ stage 7: rebuild idempotence
    idempotent, notes = stages.rebuild_matches(
        source,
        scratch,
        prefixes,
        {**environment, "RECORDER_LEDGER": str(scratch / "ledger-rebuild.jsonl")},
    )
    results["T-17"] = idempotent
    journal.say("rebuild: " + "; ".join(notes))

    # ----------------------------------------------------------- stage 8: differential replay
    _behaviour(
        stages.OBJECTS / "stage3",
        stages.STAGE3_CORPUS,
        stages.STAGE3_FAMILIES,
        STAGE3_ITEMS,
        reference["stage3"],
        stages.RECORDS / "stage3",
        prefixes["shared"],
        scratch / "stage3",
        results,
        journal,
    )


def _behaviour(
    objects: Path,
    corpus: Path,
    names: tuple[str, ...],
    items: dict[str, str],
    digests: dict[str, str],
    archives: Path,
    prefix: Path,
    where: Path,
    results: dict[str, bool],
    journal: Journal,
) -> None:
    """Link one frozen harness against the submission's library and replay every record."""
    binary = where / "harness"
    linked = stages.link_harness(objects, prefix, binary)
    if not linked.ok:
        journal.say(f"{where.name}: the harness did not link against the installed library")
        journal.say(linked.tail())
        return
    verdicts = stages.replay(binary, corpus, names, digests, where)
    for name, agreed in verdicts.items():
        results[items[name]] = agreed
    disagreed = [name for name, agreed in verdicts.items() if not agreed]
    journal.say(
        "%s: %d of %d record(s) agree with State A%s"
        % (
            where.name,
            len(names) - len(disagreed),
            len(names),
            "" if not disagreed else f"; diverging: {disagreed}",
        )
    )
    for name in disagreed:
        journal.say(stages.first_divergence(name, where / f"{name}.txt", archives / f"{name}.txt.xz"))
