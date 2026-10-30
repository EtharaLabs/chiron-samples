"""The image build's two frozen artefacts: the stage-3 corpus check and State A's manifest.

Both run only at image build time. The manifest is written with the very functions the
graded pipeline later uses on the submission (`probe.install_manifest`,
`probe.exported_symbols`, `probe.dynamic_section`, `stages.consumer_record`), so the two
sides of every comparison are produced by one implementation and a difference can only come
from the library under test.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, "/opt/verifier")

import probe  # noqa: E402
import stages  # noqa: E402

OUT = Path("/opt/verifier")


def check_corpus(manifest: Path, corpus: Path) -> int:
    """Every stage-3 case, against the digest and size its admission recorded.

    A case that no longer hashes to what the adversarial search admitted is not evidence of
    anything, so the image refuses to be built rather than ship it.
    """
    problems = 0
    listed: set[str] = set()
    cases = 0
    for line in manifest.read_text().splitlines():
        if not line.strip():
            continue
        record = json.loads(line)
        cases += 1
        listed.add(record["path"])
        whole = corpus / record["path"]
        try:
            data = whole.read_bytes()
        except OSError:
            print(f"  MISSING {record['path']}")
            problems += 1
            continue
        if hashlib.sha256(data).hexdigest() != record["sha256"]:
            print(f"  SHA256 MISMATCH {record['path']}")
            problems += 1
        if len(data) != record["bytes"]:
            print(f"  SIZE MISMATCH {record['path']}")
            problems += 1
        if not record["kills"]:
            print(f"  NO KILLS RECORDED {record['path']}")
            problems += 1
    on_disk = {
        str(Path(directory, name).relative_to(corpus))
        for directory, _, names in os.walk(corpus)
        for name in names
    }
    for extra in sorted(on_disk - listed):
        print(f"  NOT IN MANIFEST {extra}")
        problems += 1
    print(f"  {cases} cases, {problems} problems")
    return problems


def write_manifest(work: Path, scratch: Path) -> int:
    """State A's own record of itself, the only right-hand side the pipeline ever reads."""
    prefixes = {kind: scratch / f"prefix-{kind}" for kind in stages.INSTALL_KINDS}
    library = probe.shipped_library(prefixes["shared"], "shared")
    if library is None:
        print("  State A's libyyjson.so does not resolve to a file")
        return 1
    dynamic = probe.dynamic_section(library)
    exports = probe.exported_symbols(library)
    if dynamic is None or exports is None:
        print("  State A's symbol table could not be read")
        return 1
    consumers = (work / "consumer")
    consumers.mkdir(parents=True, exist_ok=True)
    manifest = {
        "header_sha256": probe.sha256_file(OUT / "statea" / "yyjson.h"),
        "install": {kind: probe.install_manifest(prefixes[kind]) for kind in prefixes},
        "exports": exports,
        "soname": dynamic["soname"],
        "needed": dynamic["needed"],
        "consumers": stages.consumer_record(prefixes, consumers),
        "stage2": {
            path.stem: probe.sha256_file(path) for path in sorted((work / "out2").glob("*.txt"))
        },
        "stage3": {
            path.stem: probe.sha256_file(path) for path in sorted((work / "out3").glob("*.txt"))
        },
    }
    missing = [key for key, value in manifest.items() if not value]
    if missing or len(manifest["exports"]) != 52:  # noqa: PLR2004 - State A's export count
        print(f"  manifest is incomplete: {missing or manifest['exports']}")
        return 1
    if any("run=0" not in line for line in manifest["consumers"].splitlines()):
        print("  a State A consumer route did not run:\n" + manifest["consumers"])
        return 1
    (OUT / "statea" / "manifest.json").write_text(json.dumps(manifest, indent=1, sort_keys=True))
    print(
        "  %d exports, soname %s, needed %s, %d consumer routes, %d + %d records"
        % (
            len(manifest["exports"]),
            manifest["soname"],
            manifest["needed"],
            len(manifest["consumers"].splitlines()),
            len(manifest["stage2"]),
            len(manifest["stage3"]),
        )
    )
    return 0


def main(argv: list[str]) -> int:
    if argv[1] == "corpus":
        return 1 if check_corpus(Path(argv[2]), Path(argv[3])) else 0
    if argv[1] == "manifest":
        return write_manifest(Path(argv[2]), Path(argv[3]))
    print(f"freeze.py: unknown step {argv[1]!r}")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
