"""The image build's frozen artefacts: the stage-3 corpus check, State A under each build
option, and State A's manifest.

All three run only at image build time. The manifest is written with the very functions the
graded pipeline later uses on the submission (`probe.install_manifest`,
`probe.exported_symbols`, `probe.dynamic_section`, `stages.consumer_record`), so the two
sides of every comparison are produced by one implementation and a difference can only come
from the library under test.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
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


def write_options(repo: Path, work: Path, default_prefix: Path) -> int:
    """State A's own exported symbol set under each option it compiles into the library.

    The option list is read out of State A's CMakeLists.txt rather than written here, and each
    option is then configured, built and installed exactly as the graded pipeline later
    configures, builds and installs the submission under it. Every option must succeed: State A
    honours all of them, so one that does not means this image's State A is not the one the
    item was measured on, and an image that cannot say what honouring an option looks like must
    not be built.

    An option State A passes a value through changes no exported symbol, so for those options a
    symbol table is not a right-hand side at all and module B-13's record of which nesting
    depths the library accepts is frozen beside it. That record is only evidence if it differs
    from the record State A's own default build produces: a record that is the same either way
    would be satisfied by a library that ignores the value entirely. So the default build at
    `default_prefix` is measured once, and an option whose record equals it refuses the image
    rather than freezing a right-hand side that separates nothing.

    Each build directory and prefix is removed as soon as its symbols and its record are read;
    keeping twelve of them would add State A's whole build tree to the image for no later use.
    """
    cmake = probe.real_tool("cmake")
    if cmake is None:
        print("  cmake is not installed")
        return 1
    options = probe.cmake_feature_options(repo / "CMakeLists.txt")
    if len(options) != 12:  # noqa: PLR2004 - State A's compilation-option count
        print(f"  State A declares {len(options)} public-definition options, expected 12")
        return 1
    where = work / "options"
    valued = [name for name, value in options if value != probe.FLAG_OPTION_SETTING]
    if not valued:
        print("  State A declares no option that passes a value through, expected 2")
        return 1
    default_record = stages.option_behaviour_record(
        default_prefix, where / "probe", "default"
    )
    if default_record is None:
        print("  State A's default-build depth record could not be read")
        return 1
    print(f"  default build: depth record is {len(default_record)} bytes")
    cases: dict[str, dict[str, object]] = {}
    for name, value in options:
        build = where / f"build-{name}"
        prefix = where / f"prefix-{name}"
        shutil.rmtree(build, ignore_errors=True)
        shutil.rmtree(prefix, ignore_errors=True)
        steps = (
            [cmake, "-S", str(repo), "-B", str(build), "-DBUILD_SHARED_LIBS=ON",
             f"-DCMAKE_INSTALL_PREFIX={prefix}", f"-D{name}={value}"],
            [cmake, "--build", str(build), "--parallel"],
            [cmake, "--install", str(build)],
        )
        for argv in steps:
            result = probe.run(argv, cwd=repo, env=probe.base_env(), timeout=1800)
            if not result.ok:
                print(f"  State A does not honour {name}={value}:\n{result.tail()}")
                return 1
        library = probe.shipped_library(prefix, "shared")
        exports = probe.exported_symbols(library) if library else None
        behaviour = (
            stages.option_behaviour_record(prefix, where / "probe", name)
            if name in valued
            else None
        )
        shutil.rmtree(build, ignore_errors=True)
        shutil.rmtree(prefix, ignore_errors=True)
        if not exports:
            print(f"  State A's symbol table under {name}={value} could not be read")
            return 1
        cases[name] = {"value": value, "exports": exports}
        if name not in valued:
            print(f"  {name}={value}: {len(exports)} exports")
            continue
        if behaviour is None:
            print(f"  State A's depth record under {name}={value} could not be read")
            return 1
        # The record has to separate the honoured build from the ignoring one. State A's
        # default build is what a submission that accepts the value and drops it produces, so
        # a record equal to the default's is no evidence and must not be frozen as one.
        if behaviour == default_record:
            print(
                f"  State A's depth record under {name}={value} equals its default-build"
                " record, so it separates nothing and must not be frozen"
            )
            return 1
        cases[name]["behaviour"] = behaviour
        print(
            f"  {name}={value}: {len(exports)} exports,"
            f" depth record {len(behaviour)} bytes, differs from the default build"
        )
    record = {"order": [name for name, _ in options], "cases": cases}
    (work / "options.json").write_text(json.dumps(record, indent=1, sort_keys=True))
    return 0


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
        "options": json.loads((work / "options.json").read_text()),
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
        "  %d exports, soname %s, needed %s, %d consumer routes, %d + %d records, %d options"
        % (
            len(manifest["exports"]),
            manifest["soname"],
            manifest["needed"],
            len(manifest["consumers"].splitlines()),
            len(manifest["stage2"]),
            len(manifest["stage3"]),
            len(manifest["options"]["order"]),
        )
    )
    return 0


def main(argv: list[str]) -> int:
    if argv[1] == "corpus":
        return 1 if check_corpus(Path(argv[2]), Path(argv[3])) else 0
    if argv[1] == "options":
        return write_options(Path(argv[2]), Path(argv[3]), Path(argv[4]))
    if argv[1] == "manifest":
        return write_manifest(Path(argv[2]), Path(argv[3]))
    print(f"freeze.py: unknown step {argv[1]!r}")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
