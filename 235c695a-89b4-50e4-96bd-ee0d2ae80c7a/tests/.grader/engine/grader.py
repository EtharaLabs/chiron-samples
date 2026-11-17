"""Builds and observations shared by State A recording and grading.

One `Grader` runs either flavour over one source tree:

* `make`  - State A's own GNU Make commands, used only to record expectations;
* `cmake` - the post-migration commands the task statement fixes, used to grade a submission.

Both flavours feed the same observation code, so a recorded case and a graded case are produced
the same way from the same kind of artifact.
"""

from __future__ import annotations

import contextlib
import json
import os
import re
import shutil
import tarfile
import zipfile
from pathlib import Path

from . import inputs, wasm
from .norm import normalize
from .util import Result, Runner, chown_tree, cpu_count, grader_dir, read_text, sha256_file, sha256_text, walk_files

PREFIX = "/usr/local"
CLANG = ["-DCMAKE_C_COMPILER=clang", "-DCMAKE_CXX_COMPILER=clang++"]
BINARIES = ["yosys", "yosys-abc", "yosys-filterlib"]
SYMBOL_LITERAL = re.compile(r"A[0-9]+_c")
TRIPLET = {"aarch64": "aarch64-linux-gnu", "x86_64": "x86_64-linux-gnu"}[os.uname().machine]


class Grader:
    def __init__(self, flavour: str, tree: Path, work: Path, logs: Path, statea: Path) -> None:
        self.flavour = flavour
        self.tree = tree
        self.work = work
        self.logs = logs
        self.statea = statea
        self.jobs = cpu_count()
        self.cases: dict[str, dict[str, str]] = {}
        self.notes: dict[str, list[str]] = {}
        self.banner = ""
        self.subs: list[tuple[str, str]] = [(str(work), "<work>")]
        home = grader_dir(work / "home")
        tmp = grader_dir(work / "tmp", 0o1777)
        self.run = Runner(logs, home, tmp)
        self.guard = contextlib.nullcontext
        self.probe_dir = grader_dir(work / "probe")
        inputs.write_all(self.probe_dir)

    # ----------------------------------------------------------------- recording helpers
    def case(self, group: str, case: str, value: str) -> None:
        self.cases.setdefault(group, {})[case] = value

    def note(self, group: str, text: str) -> None:
        self.notes.setdefault(group, []).append(text)

    def norm(self, text: str, keep_banner: bool = False) -> str:
        return normalize(text, self.subs, None if keep_banner else (self.banner or None))

    @staticmethod
    def share_subs(build: Path) -> list[tuple[str, str]]:
        """A build that makes only the yosys target may or may not fill its own share directory,
        so the binary reads the build tree's data files or the installed ones; a design run from it
        is compared on what it does, not on that location."""
        return [(str(build / "share"), "<share>"), ("/usr/local/share/yosys", "<share>")]

    def step(self, name: str, argv: list[str], cwd: Path, env: dict[str, str] | None = None, timeout: int = 5400) -> Result:
        return self.run.run(name, argv, cwd, env, timeout)

    def fresh(self, name: str, without_submodules: bool = False) -> Path:
        """A grader-owned copy of the prepared tree at work/<name>/src."""
        base = grader_dir(self.work / name)
        src = base / "src"
        if src.exists():
            shutil.rmtree(src)
        self.run.run(f"{name}-copy", ["cp", "-a", f"{self.tree}/.", str(src)], self.work, timeout=900)
        if without_submodules:
            for sub in ("abc", "libs/cxxopts"):
                path = src / sub
                if path.is_dir():
                    shutil.rmtree(path)
                    grader_dir(path)
        self.subs.append((str(src), "<src>"))
        return src

    # ----------------------------------------------------------------- default build
    def build_default(self) -> dict[str, object]:
        """B0: the default configuration, its unit tests and its staged install."""
        src = self.fresh("b0")
        self.b0 = src
        self.b0_build = src / "build"
        self.destdir = grader_dir(self.work / "b0" / "destdir")
        gtest = grader_dir(self.work / "b0" / "gtest")
        self.subs += [(str(self.destdir), "<destdir>"), (str(self.b0_build), "<build>")]
        steps: dict[str, int] = {}
        if self.flavour == "make":
            grader_dir(self.b0_build)
            make = ["make", "-f", "../Makefile"]
            steps["configure"] = self.step("b0-config", make + ["config-clang"], self.b0_build).rc
            steps["build"] = self.step("b0-build", make + [f"-j{self.jobs}"], self.b0_build).rc
            steps["unit"] = self.step("b0-unit", make + ["unit-test", f"-j{self.jobs}"], self.b0_build, {"GTEST_OUTPUT": f"json:{gtest}/"}).rc
            steps["install"] = self.step("b0-install", make + ["install", f"DESTDIR={self.destdir}", f"PREFIX={PREFIX}"], self.b0_build).rc
        else:
            steps["configure"] = self.step("b0-configure", ["cmake", "-B", "build", ".", "-G", "Ninja", *CLANG], src).rc
            steps["build"] = self.step("b0-build", ["cmake", "--build", "build", f"-j{self.jobs}"], src).rc if steps["configure"] == 0 else 1
            steps["unit"] = self.step("b0-unit", ["ctest", "--test-dir", "build/tests/unit"], src, {"GTEST_OUTPUT": f"json:{gtest}/"}).rc if steps["build"] == 0 else 1
            steps["install"] = self.step("b0-install", ["cmake", "--install", "build", "--prefix", PREFIX], src, {"DESTDIR": str(self.destdir)}).rc if steps["build"] == 0 else 1
        self.b0_steps = steps
        yosys = self.b0_build / "yosys"
        if yosys.exists():
            banner = self.step("b0-banner", [str(yosys), "-V"], self.b0_build)
            self.banner = banner.out.strip()
        self.gtest = gtest
        return steps

    def install_live(self) -> int:
        """Place the staged B0 install at the real prefix so installed consumers resolve their paths.

        The engine copies the staged files itself, so no submitted code runs with the rights the
        real prefix needs, and the prefix never becomes writable by the account that builds.
        """
        staged = self.destdir / PREFIX.lstrip("/")
        if not staged.is_dir():
            return 1
        rc = self.run.run("b0-install-live", ["cp", "-a", "--no-preserve=ownership", f"{staged}/.", PREFIX], self.work, as_grader=False).rc
        for rel in walk_files(staged):
            path = Path(PREFIX) / rel
            if not path.is_symlink() and path.exists():
                path.chmod(path.stat().st_mode & 0o777)
        return rc

    # ----------------------------------------------------------------- CI: test-build
    # ----------------------------------------------------------------- CI: workflow inventory
    # Jobs that only drove the removed Make build may go, with their artifacts and the needs
    # edges that point at them; the statement names each one.
    INVENTORY_EXEMPT = {
        "extra-builds.yml": {"jobs": {"vs-prep"}, "artifacts": {"vcxsrc"}},
    }

    def inventory(self, g: str, workflow: str) -> None:
        """What the original workflow declares still exists: jobs, runners, matrix entries, needs, artifacts.

        Every element is a presence case, so a migrated workflow may add jobs or entries but not
        drop the recorded ones. Build steps themselves are graded by running them, never read here.
        """
        path = self.tree / ".github" / "workflows" / workflow
        text = read_text(path)
        if text is None:
            self.case(g, f"inventory/{workflow}", "missing")
            return
        try:
            import yaml  # noqa: PLC0415 - only the verifier image needs a YAML reader
            data = yaml.safe_load(text)
        except Exception as exc:  # noqa: BLE001 - an unreadable workflow fails every inventory case
            self.case(g, f"inventory/{workflow}", f"unreadable: {type(exc).__name__}")
            return
        jobs = data.get("jobs") if isinstance(data, dict) else None
        if not isinstance(jobs, dict):
            self.case(g, f"inventory/{workflow}", "no jobs")
            return
        exempt = self.INVENTORY_EXEMPT.get(workflow, {})
        for job_id, job in sorted(jobs.items(), key=lambda item: str(item[0])):
            if job_id in exempt.get("jobs", set()) or not isinstance(job, dict):
                continue
            key = f"inventory/{workflow}/{job_id}"
            self.case(g, f"{key}/runs-on", json.dumps(job.get("runs-on"), sort_keys=True))
            needs = job.get("needs") or []
            for need in [needs] if isinstance(needs, str) else needs:
                if need not in exempt.get("jobs", set()):
                    self.case(g, f"{key}/needs/{need}", "present")
            matrix = (job.get("strategy") or {}).get("matrix") if isinstance(job.get("strategy"), dict) else None
            if isinstance(matrix, dict):
                for axis, values in sorted(matrix.items()):
                    for value in values if isinstance(values, list) else [values]:
                        self.case(g, f"{key}/matrix/{axis}/{json.dumps(value, sort_keys=True)}", "present")
            for step in job.get("steps") or []:
                uses = str(step.get("uses", "")) if isinstance(step, dict) else ""
                if "upload-artifact" in uses or "download-artifact" in uses:
                    name = str((step.get("with") or {}).get("name", ""))
                    if name and name not in exempt.get("artifacts", set()):
                        self.case(g, f"{key}/artifact/{'upload' if 'upload' in uses else 'download'}/{name}", "present")

    def ci_test_build(self) -> None:
        g = "S2-ci-test-build"
        self.inventory(g, "test-build.yml")
        for name in ("configure", "build", "unit", "install"):
            self.case(g, f"build-yosys/{name}-exit", str(self.b0_steps.get(name)))
        self.case(g, "build-yosys/yosys-config-log", self.config_help(self.b0_build / "yosys-config"))
        for suite, tests in sorted(self.gtest_results(self.gtest).items()):
            self.case(g, f"build-yosys/unit/{suite}", "\n".join(tests))
        tar = self.work / "b0" / "build.tar"
        packed = self.step("ci-pack", ["bash", "-o", "pipefail", "-c", f"tar -cvf {tar} share/ yosys yosys-* libyosys.so"], self.b0_build)
        self.case(g, "build-yosys/pack-exit", str(packed.rc))
        members = []
        if tar.exists():
            with tarfile.open(tar) as archive:
                members = sorted(m.name.rstrip("/") + ("/" if m.isdir() else "") for m in archive.getmembers())
        self.case(g, "build-yosys/artifact-members", "\n".join(members))
        self.artifact = tar
        # test-yosys: a fresh checkout without submodules, the artifact unpacked, the harness run.
        ci = self.fresh("ci", without_submodules=True)
        if self.flavour == "make":
            self.step("ci-unpack", ["tar", "-xf", str(tar)], ci)
            cfg = self.step("ci-yosys-config", ["./yosys-config"], ci)
            run = self.step("ci-vanilla", ["make", f"-j{self.jobs}", "vanilla-test", "TARGETS=", "EXTRA_TARGETS=", "CONFIG=clang", "SEED=1"], ci, timeout=7200)
        else:
            grader_dir(ci / "build")
            self.step("ci-unpack", ["tar", "-xf", str(tar), "-C", "build"], ci)
            cfg = self.step("ci-yosys-config", ["./build/yosys-config"], ci)
            run = self.step("ci-vanilla", ["make", "-C", "tests", f"-j{self.jobs}", "vanilla-test", "SEED=1"], ci, timeout=7200)
        self.case(g, "test-yosys/yosys-config-exit", str(cfg.rc))
        self.case(g, "test-yosys/vanilla-test-exit", str(run.rc))
        for module, results in sorted(self.harness_results(ci / "tests").items()):
            self.case(g, f"test-yosys/{module}", results)
        # test-cells: the artifact unpacked at the root of a fresh checkout.
        cells = self.fresh("cells", without_submodules=True)
        self.step("cells-unpack", ["tar", "-xf", str(tar)], cells)
        for i, script in enumerate([
            "test_cell -n 20 -s 1 all",
            "test_cell -n 20 -s 1 -nosat -aigmap $pow $pmux",
            "test_cell -n 20 -s 1 -nosat -aigmap $eqx $nex $bweqx",
            "test_cell -n 20 -s 1 -aigmap $buf",
        ]):
            res = self.step(f"cells-{i}", ["./yosys", "-p", script], cells, timeout=3600)
            text = self.norm(res.out)
            self.case(g, f"test-cells/{i}", f"exit={res.rc}\n{digest_text(text)}")
        # test-docs: the documentation examples and macro checks.
        docs = self.fresh("docsci", without_submodules=True)
        if self.flavour == "make":
            self.step("docs-unpack", ["tar", "-xf", str(tar)], docs)
        else:
            grader_dir(docs / "build")
            self.step("docs-unpack", ["tar", "-xf", str(tar), "-C", "build"], docs)
        res = self.step("docs-test", ["make", "-C", "docs", "test", f"-j{self.jobs}"], docs, timeout=3600)
        self.case(g, "test-docs/exit", str(res.rc))

    def gtest_results(self, folder: Path) -> dict[str, list[str]]:
        suites: dict[str, set[str]] = {}
        for path in sorted(folder.glob("*.json")):
            try:
                data = json.loads(path.read_text())
            except (OSError, ValueError):
                continue
            for suite in data.get("testsuites", []):
                for test in suite.get("testsuite", []):
                    if test.get("status") == "NOTRUN" or test.get("result") == "SKIPPED":
                        continue
                    ok = not test.get("failures")
                    suites.setdefault(suite.get("name", "?"), set()).add(f"{test.get('name')} {'PASS' if ok else 'FAIL'}")
        return {k: sorted(v) for k, v in suites.items()}

    def harness_results(self, tests: Path) -> dict[str, str]:
        """module -> sorted `<test> <PASS|FAIL>` lines from the harness's `.result` files."""
        modules: dict[str, list[str]] = {}
        for path in sorted(tests.rglob("*.result")):
            rel = path.relative_to(tests).as_posix()
            module = rel.rsplit("/", 1)[0] if "/" in rel else "."
            status = (read_text(path) or "").strip() or "EMPTY"
            modules.setdefault(module, []).append(f"{rel[:-7]} {status}")
        return {k: "\n".join(sorted(v)) for k, v in modules.items()}

    def config_help(self, script: Path) -> str:
        res = self.step("yosys-config-help", [str(script)], script.parent)
        text = res.out.replace(str(script), "<yosys-config>")
        return f"exit={res.rc}\n{self.norm(text)}"

    # ----------------------------------------------------------------- the private State A harness
    def suites(self) -> None:
        """Run State A's own test harness against the graded build, one case per test module."""
        harness = grader_dir(self.work / "harness")
        tests = harness / "tests"
        if tests.exists():
            shutil.rmtree(tests)
        shutil.copytree(self.statea / "tests", tests, symlinks=True)
        # The harness reads the build products at the tree root, where the CI test job unpacks the
        # `share/ yosys yosys-* libyosys.so` artifact; everything else comes from State A's tree.
        for product in sorted(self.b0_build.iterdir()):
            if product.name in ("share", "yosys", "libyosys.so") or product.name.startswith("yosys-"):
                link = harness / product.name
                if link.is_symlink() or link.exists():
                    link.unlink()
                os.symlink(product, link)
        for rel in self.statea.iterdir():
            if rel.name != "tests" and not (harness / rel.name).is_symlink() and not (harness / rel.name).exists():
                os.symlink(rel, harness / rel.name)
        chown_tree(tests)
        res = self.step(
            "suite-vanilla",
            ["make", "-C", str(tests), f"-j{self.jobs}", "vanilla-test", "SEEDOPT=-S 1"],
            harness, timeout=7200,
        )
        self.note("suites", f"vanilla-test exit {res.rc}")
        results = self.harness_results(tests)
        for module, lines in sorted(results.items()):
            top = module.split("/", 1)[0]
            self.case(suite_group(top), f"suite/{module}", lines)

    # ----------------------------------------------------------------- command reference
    def command_reference(self) -> None:
        g = "S2-command-reference"
        yosys = self.b0_build / "yosys"
        listing = self.step("cmd-list", [str(yosys), "-Q", "-T", "-p", "help"], self.probe_dir)
        commands = parse_help_list(listing.out)
        self.case(g, "commands", "\n".join(commands))
        cells_listing = self.step("cell-list", [str(yosys), "-Q", "-T", "-p", "help -cells"], self.probe_dir)
        cells = parse_help_list(cells_listing.out)
        self.case(g, "cells", "\n".join(cells))
        for kind, names in (("cmd", commands), ("cell", cells)):
            script = "".join(f"log __MARK__ {n}\nhelp {n}\n" for n in names)
            path = self.probe_dir / f"help-{kind}.ys"
            path.write_text(script)
            res = self.step(f"help-{kind}", [str(yosys), "-Q", "-T", "-s", str(path)], self.probe_dir, timeout=1800)
            for name, text in split_marks(res.out).items():
                self.case(g, f"{kind}/{name}", self.norm(text))
        usage = self.step("yosys-h", [str(yosys), "-h"], self.probe_dir)
        self.case(g, "usage/yosys", f"exit={usage.rc}\n{self.norm(usage.out.replace(str(yosys), '<yosys>'))}")
        for tool, args in (("yosys-smtbmc", ["--help"]), ("yosys-witness", ["--help"]), ("yosys-filterlib", []), ("yosys-abc", ["-h"])):
            path = self.b0_build / tool
            res = self.step(f"usage-{tool}", [str(path), *args], self.probe_dir, timeout=120)
            self.case(g, f"usage/{tool}", f"exit={res.rc}\n{self.norm(res.out.replace(str(path), '<' + tool + '>'))}")

    # ----------------------------------------------------------------- compiled features
    def compiled_features(self) -> None:
        g = "S2-compiled-features"
        self.case(g, "banner", self.banner)
        yosys = str(self.b0_build / "yosys")
        for name, argv in inputs.feature_probes(yosys, self.probe_dir, TRIPLET):
            res = self.step(f"feat-{name}", argv, self.probe_dir, timeout=600)
            text = res.out.replace(yosys, "<yosys>")
            if name == "xtrace":
                frames = {re.sub(r"0x[0-9a-f]+", "<addr>", line) for line in text.split("\n") if line.startswith("-X-")}
                text = "\n".join(sorted(frames))
            self.case(g, f"feature/{name}", f"exit={res.rc}\n{self.norm(text)}")
        for name, script in inputs.number_cases(self.probe_dir):
            res = self.step(f"num-{name}", [yosys, "-Q", "-T", "-p", script], self.probe_dir, timeout=300)
            self.case(g, f"number/{name}", f"exit={res.rc}\n{self.norm(res.out)}")

    # ----------------------------------------------------------------- install layout and ABI
    def install_layout(self) -> None:
        g = "S2-install-layout"
        files = walk_files(self.destdir)
        self.case(g, "paths", "\n".join(files))
        for rel in files:
            path = self.destdir / rel
            if path.is_symlink():
                self.case(g, f"file/{rel}", f"symlink -> {os.readlink(path)}")
                continue
            mode = oct(path.stat().st_mode & 0o7777)
            head = path.read_bytes()[:4]
            if head == b"\x7fELF":
                self.case(g, f"file/{rel}", f"elf mode={mode}")
            else:
                data = read_text(path)
                body = sha256_text(self.norm(data)) if data is not None else sha256_file(path)
                self.case(g, f"file/{rel}", f"mode={mode} sha256={body}")

    def binary_abi(self) -> None:
        g = "S2-binary-abi"
        targets = [(f"build/{n}", self.b0_build / n) for n in BINARIES + ["libyosys.so"]]
        targets += [(f"install/bin/{n}", self.destdir / PREFIX.lstrip("/") / "bin" / n) for n in BINARIES]
        for label, path in targets:
            for key, value in elf_facts(path, self.run, self.probe_dir).items():
                self.case(g, f"{label}/{key}", value)

    # ----------------------------------------------------------------- consumer routes
    def consumer_routes(self) -> None:
        g = "S2-consumer-routes"
        for where, script in (("build", self.b0_build / "yosys-config"), ("install", Path(PREFIX) / "bin" / "yosys-config")):
            for opt in ("--cxx", "--cxxflags", "--linkflags", "--ldflags", "--libs", "--ldlibs", "--bindir", "--datdir"):
                res = self.step(f"cfg-{where}{opt}", [str(script), opt], self.probe_dir)
                self.case(g, f"yosys-config/{where}/{opt}", f"exit={res.rc}\n{self.norm(res.out)}")
        plugin_dir = grader_dir(self.work / "plugin")
        shutil.copy(self.probe_dir / "plugin.cc", plugin_dir / "plugin.cc")
        cfg = str(Path(PREFIX) / "bin" / "yosys-config")
        built = self.step("plugin-build", [cfg, "--build", "plugin.so", "plugin.cc"], plugin_dir, timeout=900)
        self.case(g, "plugin/build-exit", str(built.rc))
        loaded = self.step("plugin-load", [str(Path(PREFIX) / "bin" / "yosys"), "-Q", "-T", "-m", "./plugin.so", "-p", "grader_probe"], plugin_dir)
        self.case(g, "plugin/run", f"exit={loaded.rc}\n{self.norm(loaded.out)}")
        demo = grader_dir(self.work / "cxxapi")
        shutil.copy(self.probe_dir / "example.v", demo / "example.v")
        shutil.copy(self.statea / "examples" / "cxx-api" / "demomain.cc", demo / "demomain.cc")
        flags = self.step("cxxapi-flags", [cfg, "--cxxflags"], demo).out.split()
        libs = self.step("cxxapi-libs", [cfg, "--ldlibs"], demo).out.split()
        compiled = self.step("cxxapi-build", ["clang++", *flags, "-o", "demomain", "demomain.cc", f"-L{self.b0_build}", "-lyosys", *libs], demo, timeout=900)
        self.case(g, "libyosys/build-exit", str(compiled.rc))
        ran = self.step("cxxapi-run", ["./demomain"], demo, {"LD_LIBRARY_PATH": str(self.b0_build)}, timeout=600)
        self.case(g, "libyosys/run", f"exit={ran.rc}\n{self.norm(ran.out)}")
        blif = demo / "example.blif"
        self.case(g, "libyosys/blif", sha256_text(self.norm(read_text(blif) or "")) if blif.exists() else "missing")
        for tool, args in (
            ("yosys-witness", ["--help"]),
            ("yosys-smtbmc", ["--help"]),
            ("yosys-abc", ["-c", "read_blif example.blif; strash; print_stats -s"]),
            ("yosys-filterlib", ["-verilogsim", str(self.probe_dir / "cells.lib")]),
        ):
            path = str(Path(PREFIX) / "bin" / tool)
            res = self.step(f"installed-{tool}", [path, *args], demo, timeout=300)
            text = re.sub(r"\(compiled [^)]*\)", "(compiled <date>)", res.out.replace(path, f"<{tool}>"))
            self.case(g, f"installed/{tool}", f"exit={res.rc}\n{self.norm(text)}")

    # ----------------------------------------------------------------- option matrix
    OPTIONS = [
        ("tcl-off", ["ENABLE_TCL=0"], ["-DYOSYS_WITHOUT_TCL=ON"]),
        ("readline-off", ["ENABLE_READLINE=0"], ["-DYOSYS_WITHOUT_READLINE=ON"]),
        ("zlib-off", ["ENABLE_ZLIB=0"], ["-DYOSYS_WITHOUT_ZLIB=ON"]),
        ("plugins-off", ["ENABLE_PLUGINS=0"], ["-DYOSYS_WITHOUT_PLUGINS=ON"]),
        ("abc-off", ["ENABLE_ABC=0"], ["-DYOSYS_WITHOUT_ABC=ON"]),
        ("threads-off", ["ENABLE_THREADS=0"], ["-DYOSYS_DISABLE_THREADS=ON"]),
        ("debug", ["ENABLE_DEBUG=1"], ["-DCMAKE_BUILD_TYPE=Debug"]),
        ("prefix", ["PREFIX=/opt/yosys"], ["-DCMAKE_INSTALL_PREFIX=/opt/yosys"]),
        ("program-prefix", ["PROGRAM_PREFIX=gr-"], ["-DYOSYS_PROGRAM_PREFIX=gr-"]),
        ("cxx23", ["CXXSTD=c++23"], ["-DCMAKE_CXX_STANDARD=23"]),
        ("help-source", ["ENABLE_HELP_SOURCE=1"], ["-DYOSYS_ENABLE_HELP_SOURCE=ON"]),
    ]

    def option_matrix(self) -> None:
        g = "S2-option-matrix"
        for name, make_vars, cmake_args in self.OPTIONS:
            base = grader_dir(self.work / f"opt-{name}")
            # The configure step writes yosys-config under the program prefix the option sets.
            prefix = "".join(v.split("=", 1)[1] for v in make_vars if v.startswith("PROGRAM_PREFIX="))
            if self.flavour == "make":
                build = grader_dir(base / "build")
                conf = "".join(f"{v.split('=')[0]} := {v.split('=', 1)[1]}\n" for v in make_vars)
                self.step(f"opt-{name}-config", ["make", "-f", f"{self.b0}/Makefile", "config-clang"], build)
                with open(build / "Makefile.conf", "a") as handle:
                    handle.write(conf)
                script = build / f"{prefix}yosys-config"
                configured = self.step(f"opt-{name}-script", ["make", "-f", f"{self.b0}/Makefile", script.name], build)
            else:
                build = base / "build"
                with self.guard():
                    configured = self.step(f"opt-{name}-configure", ["cmake", "-B", str(build), "-S", str(self.b0), "-G", "Ninja", *CLANG, *cmake_args], base)
                script = build / f"{prefix}yosys-config"
            self.case(g, f"{name}/exit", str(configured.rc))
            for opt in ("--cxxflags", "--linkflags", "--libs", "--bindir", "--datdir"):
                res = self.step(f"opt-{name}{opt}", [str(script), opt], base)
                self.case(g, f"{name}/{opt}", f"exit={res.rc}\n{normalize(res.out, self.subs + [(str(self.b0), '<src>')])}")

    def option_build(self) -> None:
        """One alternate full build: prefix, program prefix and the installed library together.

        ABC stays off: State A's make build cannot combine a program prefix with ABC, because its abc
        sub-make runs in `<prefix>abc`, a directory it never creates.
        """
        g = "S2-option-matrix"
        src = self.fresh("alt")
        dest = grader_dir(self.work / "alt" / "destdir")
        if self.flavour == "make":
            build = grader_dir(src / "build")
            make = ["make", "-f", "../Makefile"]
            vars_ = ["PREFIX=/opt/yosys", "PROGRAM_PREFIX=gr-", "ENABLE_LIBYOSYS=1", "ENABLE_ABC=0"]
            rc = self.step("alt-config", make + ["config-clang"], build).rc
            rc = rc or self.step("alt-build", make + [f"-j{self.jobs}", *vars_], build).rc
            rc = rc or self.step("alt-install", make + ["install", f"DESTDIR={dest}", *vars_], build).rc
        else:
            args = ["-DCMAKE_INSTALL_PREFIX=/opt/yosys", "-DYOSYS_PROGRAM_PREFIX=gr-", "-DYOSYS_INSTALL_LIBRARY=ON",
                    "-DYOSYS_WITHOUT_ABC=ON"]
            with self.guard():
                rc = self.step("alt-configure", ["cmake", "-B", "build", ".", "-G", "Ninja", *CLANG, *args], src).rc
                rc = rc or self.step("alt-build", ["cmake", "--build", "build", f"-j{self.jobs}"], src).rc
                rc = rc or self.step("alt-install", ["cmake", "--install", "build"], src, {"DESTDIR": str(dest)}).rc
        self.case(g, "alt/exit", str(rc))
        files = walk_files(dest)
        self.case(g, "alt/paths", "\n".join(files))
        for rel in files:
            path = dest / rel
            if path.is_symlink():
                continue
            if path.read_bytes()[:4] == b"\x7fELF":
                facts = elf_facts(path, self.run, self.probe_dir)
                self.case(g, f"alt/elf/{rel}", "\n".join(f"{k}={v}" for k, v in sorted(facts.items()) if k in ("needed", "soname", "runpath", "rpath", "debug", "symtab")))
            elif rel.endswith(("yosys-config", "yosys-smtbmc", "yosys-witness")):
                self.case(g, f"alt/script/{rel}", sha256_text(self.norm(read_text(path) or "")))
        yosys = dest / "opt/yosys/bin/gr-yosys"
        if yosys.exists():
            res = self.step("alt-run", [str(yosys), "-Q", "-T", "-p", f"read_verilog {self.probe_dir}/example.v; synth -top top -noabc; stat"], self.probe_dir)
            self.case(g, "alt/run", f"exit={res.rc}\n{self.norm(res.out)}")

    def option_static(self) -> None:
        """The static library: State A's ENABLE_LIBYOSYS_STATIC=1 against BUILD_SHARED_LIBS=OFF.

        Only the library is built. Its archive members are named differently by each build, so
        the case is the set of global symbols the archive defines.
        """
        g = "S2-option-matrix"
        base = grader_dir(self.work / "static")
        build = base / "build"
        if self.flavour == "make":
            build = grader_dir(build)
            self.step("static-config", ["make", "-f", f"{self.b0}/Makefile", "config-clang"], build)
            with open(build / "Makefile.conf", "a") as handle:
                handle.write("ENABLE_LIBYOSYS_STATIC := 1\n")
            rc = self.step("static-build", ["make", "-f", f"{self.b0}/Makefile", f"-j{self.jobs}", "libyosys.a"], build, timeout=7200).rc
        else:
            with self.guard():
                rc = self.step("static-configure", ["cmake", "-B", str(build), "-S", str(self.b0), "-G", "Ninja", *CLANG, "-DBUILD_SHARED_LIBS=OFF"], base).rc
                rc = rc or self.step("static-build", ["cmake", "--build", str(build), "--target", "libyosys", f"-j{self.jobs}"], base, timeout=7200).rc
        self.case(g, "static/exit", str(rc))
        archive = build / "libyosys.a"
        self.case(g, "static/archive", "present" if archive.is_file() else "absent")
        if archive.is_file():
            res = self.step("static-symbols", ["nm", "--defined-only", "-g", "--format=posix", str(archive)], base)
            names = sorted({re.sub(r"A[0-9]+_c", "A_c", line.split()[0]) for line in res.out.split("\n")
                            if line and not line.endswith(":") and len(line.split()) >= 2})
            self.case(g, "static/symbols", f"exit={res.rc} names={len(names)} sha256={sha256_text(chr(10).join(names))}")

    # ----------------------------------------------------------------- CI: test-compile matrix
    def compile_matrix(self) -> None:
        g = "S2-ci-test-compile"
        self.inventory(g, "test-compile.yml")
        entries = [
            ("clang-14", {"clang": "/usr/bin/clang-14", "clang++": "/usr/bin/clang++-14"}, "clang",
             f"--gcc-toolchain=/usr -isystem /usr/include/c++/12 -isystem /usr/include/{TRIPLET}/c++/12"),
            ("gcc-11", {"gcc": "/usr/bin/gcc-11", "g++": "/usr/bin/g++-11"}, "gcc", "-Wp,-D_GLIBCXX_ASSERTIONS"),
        ]
        for name, links, short, cxxflags in entries:
            bindir = grader_dir(self.work / f"cc-{name}" / "bin")
            for link, target in links.items():
                if not (bindir / link).exists():
                    os.symlink(target, bindir / link)
            cc, cxx = ("clang", "clang++") if short == "clang" else ("gcc", "g++")
            env = {"PATH": f"{bindir}:{self.run.path}", "CC": cc, "CXX": cxx, "CXXFLAGS": cxxflags}
            src = self.fresh(f"cc-{name}")
            if self.flavour == "make":
                rc_c = self.step(f"cc-{name}-config", ["make", f"config-{short}"], src, env).rc
                rc_b = self.step(f"cc-{name}-build", ["make", f"-j{self.jobs}", "CXXSTD=c++20"], src, env, timeout=7200).rc
                build = src
            else:
                with self.guard():
                    rc_c = self.step(f"cc-{name}-configure", ["cmake", "-B", "build", "-G", "Ninja", "-DCMAKE_CXX_STANDARD=20", "."], src, env).rc
                    rc_b = self.step(f"cc-{name}-build", ["cmake", "--build", "build", "--target", "yosys", f"-j{self.jobs}"], src, env, timeout=7200).rc if rc_c == 0 else 1
                build = src / "build"
            self.case(g, f"{name}/configure-exit", str(rc_c))
            self.case(g, f"{name}/build-exit", str(rc_b))
            yosys = build / "yosys"
            if yosys.exists():
                self.case(g, f"{name}/banner", self.step(f"cc-{name}-banner", [str(yosys), "-V"], build, env).out.strip())
                res = self.step(f"cc-{name}-run", [str(yosys), "-Q", "-T", "-p", f"read_verilog {self.probe_dir}/example.v; synth -top top -noabc; stat"], self.probe_dir, env)
                self.case(g, f"{name}/run", f"exit={res.rc}\n{normalize(res.out, self.share_subs(build) + self.subs, self.banner or None)}")
            script = build / "yosys-config"
            if self.flavour == "make" and not script.exists():
                self.step(f"cc-{name}-script", ["make", "yosys-config"], src, env)
            res = self.step(f"cc-{name}-cxxflags", [str(script), "--cxxflags"], build, env)
            self.case(g, f"{name}/cxxflags", f"exit={res.rc}\n{normalize(res.out, self.subs)}")

    # ----------------------------------------------------------------- CI: sanitizers
    def sanitizers(self) -> None:
        g = "S2-ci-test-sanitizers"
        self.inventory(g, "test-sanitizers.yml")
        env = {"ASAN_OPTIONS": "halt_on_error=1", "UBSAN_OPTIONS": "halt_on_error=1"}
        src = self.fresh("san")
        if self.flavour == "make":
            rc_c = self.step("san-config", ["make", "config-clang"], src, env).rc
            with open(src / "Makefile.conf", "a") as handle:
                handle.write("SANITIZER = undefined,address\n")
            rc_b = self.step("san-build", ["make", f"-j{self.jobs}"], src, env, timeout=7200).rc
            build = src
            run = self.step("san-vanilla", ["make", f"-j{self.jobs}", "vanilla-test", "TARGETS=", "EXTRA_TARGETS=", "SEED=1"], src, env, timeout=14400)
        else:
            with self.guard():
                rc_c = self.step("san-configure", ["cmake", "-B", "build", ".", "-G", "Ninja", *CLANG, "-DCMAKE_BUILD_TYPE=Sanitize", "-DSANITIZE=undefined,address"], src, env).rc
                rc_b = self.step("san-build", ["cmake", "--build", "build", f"-j{self.jobs}"], src, env, timeout=7200).rc if rc_c == 0 else 1
            build = src / "build"
            run = self.step("san-vanilla", ["make", "-C", "tests", f"-j{self.jobs}", "vanilla-test", "SEED=1"], src, env, timeout=14400)
        self.case(g, "configure-exit", str(rc_c))
        self.case(g, "build-exit", str(rc_b))
        self.case(g, "vanilla-test-exit", str(run.rc))
        if (build / "yosys").exists():
            self.case(g, "banner", self.step("san-banner", [str(build / "yosys"), "-V"], build, env).out.strip())
        for opt in ("--cxxflags", "--linkflags"):
            res = self.step(f"san-cfg{opt}", [str(build / "yosys-config"), opt], build, env)
            self.case(g, f"yosys-config{opt}", f"exit={res.rc}\n{normalize(res.out, self.subs)}")
        for module, lines in sorted(self.harness_results(src / "tests").items()):
            self.case(g, f"vanilla/{module}", lines)

    # ----------------------------------------------------------------- CI: extra builds (WASI)
    def wasi(self) -> None:
        """extra-builds.yml wasi-build: the job's own configuration, translated for CMake.

        The original job wrote build/Makefile.conf (CONFIG wasi, PREFIX /, Tcl, readline, plugins
        and zlib off, the flex include directory, and a larger stack with --stack-first and
        --strip-all); the CMake form passes the same settings as cache variables.
        """
        g = "S2-ci-extra-builds"
        self.inventory(g, "extra-builds.yml")
        sdk = Path("/opt/wasi-sdk")
        src = self.fresh("wasi")
        flex = grader_dir(self.work / "wasi" / "flex-prefix")
        grader_dir(flex / "include")
        shutil.copy("/usr/include/FlexLexer.h", flex / "include" / "FlexLexer.h")
        env = {"PATH": f"{sdk}/bin:{flex}/bin:{self.run.path}"}
        if self.flavour == "make":
            build = grader_dir(src / "build")
            (build / "Makefile.conf").write_text(
                f"export PATH := {sdk}/bin:{flex}/bin:{self.run.path}\nWASI_SYSROOT := {sdk}/share/wasi-sysroot\n\n"
                "CONFIG := wasi\nPREFIX := /\n\n"
                "ENABLE_TCL := 0\nENABLE_READLINE := 0\nENABLE_PLUGINS := 0\nENABLE_ZLIB := 0\n\n"
                f"CXXFLAGS += -I{flex}/include\n"
                "LINKFLAGS += -Wl,-z,stack-size=8388608 -Wl,--stack-first -Wl,--strip-all\n"
            )
            rc_c = 0
            rc_b = self.step("wasi-build", ["make", "-C", "build", "-f", "../Makefile", "CXX=clang", f"-j{self.jobs}"], src, env, timeout=7200).rc
            out = build
        else:
            configure = [
                "cmake", "-B", "build", "-G", "Ninja", f"-DCMAKE_TOOLCHAIN_FILE={sdk}/share/cmake/wasi-sdk-p1.cmake",
                "-DCMAKE_INSTALL_PREFIX=/", "-DYOSYS_WITHOUT_TCL=ON", "-DYOSYS_WITHOUT_READLINE=ON",
                "-DYOSYS_WITHOUT_PLUGINS=ON", "-DYOSYS_WITHOUT_ZLIB=ON", f"-DCMAKE_CXX_FLAGS=-I{flex}/include",
                "-DCMAKE_EXE_LINKER_FLAGS=-Wl,-z,stack-size=8388608 -Wl,--stack-first -Wl,--strip-all", ".",
            ]
            with self.guard():
                rc_c = self.step("wasi-configure", configure, src, env).rc
                rc_b = self.step("wasi-build", ["cmake", "--build", "build", f"-j{self.jobs}"], src, env, timeout=7200).rc if rc_c == 0 else 1
            out = src / "build"
        self.case(g, "wasi/configure-exit", str(rc_c))
        self.case(g, "wasi/build-exit", str(rc_b))
        modules = sorted(p.name for p in out.glob("*.wasm"))
        self.case(g, "wasi/modules", "\n".join(modules))
        for name in modules:
            for key, value in wasm.facts(out / name).items():
                self.case(g, f"wasi/{name}/{key}", value)

    # ----------------------------------------------------------------- CI: wheels
    def wheels(self) -> None:
        g = "S2-ci-wheels"
        self.inventory(g, "wheels.yml")
        src = self.fresh("wheel")
        dist = grader_dir(self.work / "wheel" / "dist")
        # The Linux wheel job builds with gcc after deleting -Werror=unused from the build file; offline,
        # the wheel is built with the macOS job's compiler, clang, so no build file needs editing.
        env = {"PIP_NO_INDEX": "1", "PIP_FIND_LINKS": "/opt/wheelhouse", "PIP_DISABLE_PIP_VERSION_CHECK": "1",
               "CC": "clang", "CXX": "clang++"}
        with self.guard():
            built = self.step("wheel-build", ["python3", "-m", "pip", "wheel", "--no-index", "--find-links", "/opt/wheelhouse", "--no-deps", "-w", str(dist), "."], src, env, timeout=7200)
        self.case(g, "build-exit", str(built.rc))
        wheels = sorted(dist.glob("*.whl"))
        self.case(g, "wheel-files", "\n".join(p.name for p in wheels))
        if len(wheels) != 1:
            return
        with zipfile.ZipFile(wheels[0]) as archive:
            names = sorted(archive.namelist())
            self.case(g, "record-paths", "\n".join(names))
            for name in names:
                if name.endswith(".dist-info/METADATA"):
                    meta = archive.read(name).decode("utf-8", "replace")
                    head, _, body = meta.partition("\n\n")
                    for field in sorted(set(line.split(":", 1)[0] for line in head.split("\n") if ":" in line)):
                        values = [line.split(":", 1)[1].strip() for line in head.split("\n") if line.startswith(field + ":")]
                        self.case(g, f"metadata/{field}", "\n".join(values))
                    readme = (read_text(src / "README.md") or "").strip()
                    self.case(g, "metadata/description", f"empty={'yes' if not body.strip() else 'no'} matches-readme={'yes' if body.strip() == readme else 'no'}")
                elif name.endswith(".dist-info/WHEEL") or name.endswith(".dist-info/top_level.txt"):
                    self.case(g, f"dist-info/{name.split('/')[-1]}", archive.read(name).decode("utf-8", "replace").strip())
                elif name.startswith("pyosys/share/") or name == "pyosys/__init__.py":
                    data = archive.read(name)
                    self.case(g, f"file/{name}", sha256_text(self.norm(data.decode("utf-8", "replace"))) if b"\0" not in data[:4096] else "binary")
            unpacked = grader_dir(self.work / "wheel" / "unpacked")
            archive.extractall(unpacked)
        for name in ("pyosys/libyosys.so", "pyosys/yosys-abc"):
            path = unpacked / name
            for key, value in elf_facts(path, self.run, self.probe_dir).items():
                if key in ("needed", "soname", "runpath", "rpath", "debug", "symtab", "type"):
                    self.case(g, f"elf/{name}/{key}", value)
        venv = self.work / "wheel" / "venv"
        self.step("wheel-venv", ["python3", "-m", "venv", str(venv)], self.work, env)
        inst = self.step("wheel-install", [str(venv / "bin/pip"), "install", "--no-index", "--no-deps", str(wheels[0])], self.work, env)
        self.case(g, "install-exit", str(inst.rc))
        smoke = self.step("wheel-import", [str(venv / "bin/python"), str(self.probe_dir / "pyosys_smoke.py")], self.probe_dir, env)
        self.case(g, "import-smoke", f"exit={smoke.rc}\n{self.norm(smoke.out)}")
        pytests = self.statea / "tests" / "pyosys"
        for test in sorted(pytests.glob("test_*.py")):
            res = self.step(f"pyosys-{test.stem}", [str(venv / "bin/python"), str(test)], pytests.parent.parent, env, timeout=600)
            self.case(g, f"pyosys/{test.stem}", f"exit={res.rc}")

    # ----------------------------------------------------------------- CI: source vendor
    def source_vendor(self) -> None:
        g = "S2-ci-source-vendor"
        self.inventory(g, "source-vendor.yml")
        src = self.fresh("vendor")
        gitenv = {
            "GIT_AUTHOR_NAME": "grader", "GIT_AUTHOR_EMAIL": "grader@invalid", "GIT_AUTHOR_DATE": "2026-06-02T08:07:03Z",
            "GIT_COMMITTER_NAME": "grader", "GIT_COMMITTER_EMAIL": "grader@invalid", "GIT_COMMITTER_DATE": "2026-06-02T08:07:03Z",
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
        }
        (src / ".gitcommit").write_text("$Format:%H$\n")
        script = (
            "git init -q -b main . && git add -A -f . && git commit -q -m vendored && "
            "git archive --format=tar HEAD -o ../vendored.tar && git rev-parse HEAD"
        )
        made = self.run.shell("vendor-archive", script, src, gitenv, timeout=900)
        commit = made.out.strip().split("\n")[-1]
        self.case(g, "archive-exit", str(made.rc))
        out = grader_dir(self.work / "vendor" / "unpacked")
        self.step("vendor-unpack", ["tar", "-xf", "../vendored.tar", "-C", str(out)], src)
        stamp = (read_text(out / ".gitcommit") or "").strip()
        self.case(g, "gitcommit-expanded", "commit" if stamp == commit else f"other: {stamp[:60]}")
        hidden = Path("/usr/bin/git")
        moved = Path("/opt/grader-hidden-git")
        if os.geteuid() == 0 and hidden.exists():
            shutil.move(str(hidden), str(moved))
        try:
            if self.flavour == "make":
                build = grader_dir(out / "build")
                rc = self.step("vendor-config", ["make", "-f", "../Makefile", "config-clang"], build).rc
                rc = rc or self.step("vendor-build", ["make", "-f", "../Makefile", f"-j{self.jobs}", "yosys"], build, timeout=7200).rc
            else:
                build = out / "build"
                with self.guard():
                    rc = self.step("vendor-configure", ["cmake", "-B", "build", ".", "-G", "Ninja", *CLANG], out).rc
                    rc = rc or self.step("vendor-build", ["cmake", "--build", "build", "--target", "yosys", f"-j{self.jobs}"], out, timeout=7200).rc
        finally:
            if moved.exists():
                shutil.move(str(moved), str(hidden))
        self.case(g, "build-without-git-exit", str(rc))
        yosys = build / "yosys"
        if yosys.exists():
            banner = self.step("vendor-banner", [str(yosys), "-V"], build).out.strip()
            self.case(g, "banner-without-git", banner.replace(commit, "<archive-commit>"))
            res = self.step("vendor-run", [str(yosys), "-Q", "-T", "-p", f"read_verilog {self.probe_dir}/example.v; synth -top top -noabc; stat"], self.probe_dir)
            self.case(g, "run", f"exit={res.rc}\n{normalize(res.out, self.share_subs(build) + self.subs, banner)}")

    # ----------------------------------------------------------------- CI: documentation inputs
    def docs_prepare(self) -> None:
        """prepare-docs, offline: the documentation inputs generated from the build.

        The job's image conversion needs TeX, faketime and pdf2svg, which are not installed, so the
        cases are the code-example outputs of the test-docs run and the command and cell references
        the binary dumps for the documentation.
        """
        g = "S2-ci-docs"
        self.inventory(g, "prepare-docs.yml")
        examples = self.work / "docsci" / "src" / "docs" / "source" / "code_examples"
        source = self.tree / "docs" / "source" / "code_examples"
        produced = [rel for rel in walk_files(examples) if not (source / rel).exists()] if examples.exists() else []
        self.case(g, "examples/outputs", "\n".join(produced))
        for rel in produced:
            # A compiled example plugin and its dependency file record the include paths of the
            # build that made them, so they are checked for presence; their behaviour is in the logs.
            if rel.endswith((".so", ".o", ".d")):
                self.case(g, f"examples/{rel}", "compiled")
                continue
            text = read_text(examples / rel)
            value = sha256_text(self.norm(text)) if text is not None else sha256_file(examples / rel)
            self.case(g, f"examples/{rel}", value)
        yosys = self.b0_build / "yosys"
        for kind in ("cmds", "cells"):
            out = self.probe_dir / f"{kind}.json"
            res = self.step(f"dump-{kind}", [str(yosys), "-Q", "-T", "-p", f"help -dump-{kind}-json {out}"], self.probe_dir)
            try:
                data = json.loads(read_text(out) or "")
            except ValueError:
                self.case(g, f"reference/{kind}", f"exit={res.rc} unreadable")
                continue
            entries = data.get(kind, data) if isinstance(data, dict) else data
            self.case(g, f"reference/{kind}", f"exit={res.rc} entries={len(entries)} sha256={sha256_text(json.dumps(data, sort_keys=True))}")


SUITES = {
    "S2-suite-frontends": {"verilog", "svtypes", "svinterfaces", "simple", "hana", "asicworld", "realmath", "blif", "rtlil", "aiger", "liberty", "bind"},
    "S2-suite-techmap-arch": {"arch", "techmap", "memlib", "bram", "memories", "arith_tree", "alumacc"},
    "S2-suite-sim-formal": {"sim", "sat", "xprop", "cxxrtl", "fmt", "memfile", "sdc", "bugpoint", "rpc"},
}


def suite_group(top: str) -> str:
    for group, names in SUITES.items():
        if top in names:
            return group
    return "S2-suite-passes"


def parse_help_list(text: str) -> list[str]:
    names = []
    for line in text.split("\n"):
        match = re.match(r"^\s{4}(\S+)\s{2,}\S", line) or re.match(r"^\s{4}(\$\S+)\s*$", line)
        if match:
            names.append(match.group(1))
    return sorted(set(names))


def split_marks(text: str) -> dict[str, str]:
    parts: dict[str, str] = {}
    current, buf = None, []
    for line in text.split("\n"):
        match = re.match(r"^__MARK__ (\S+)$", line.strip())
        if match:
            if current is not None:
                parts[current] = "\n".join(buf).strip("\n")
            current, buf = match.group(1), []
        elif current is not None:
            buf.append(line)
    if current is not None:
        parts[current] = "\n".join(buf).strip("\n")
    return parts


def digest_text(text: str) -> str:
    return f"lines={text.count(chr(10)) + 1} sha256={sha256_text(text)}"


def elf_facts(path: Path, run: Runner, cwd: Path) -> dict[str, str]:
    if not path.exists():
        return {"present": "no"}
    facts = {"present": "yes"}
    head = run.run("readelf-h", ["readelf", "-h", str(path)], cwd).out
    match = re.search(r"Type:\s+(\S+)", head)
    facts["type"] = match.group(1) if match else "?"
    dyn = run.run("readelf-d", ["readelf", "-d", str(path)], cwd).out
    facts["needed"] = " ".join(sorted(re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", dyn)))
    facts["soname"] = " ".join(re.findall(r"\(SONAME\)\s+Library soname: \[([^\]]+)\]", dyn))
    facts["runpath"] = " ".join(re.findall(r"\(RUNPATH\)\s+Library runpath: \[([^\]]+)\]", dyn))
    facts["rpath"] = " ".join(re.findall(r"\(RPATH\)\s+Library rpath: \[([^\]]+)\]", dyn))
    sections = run.run("readelf-S", ["readelf", "-S", "-W", str(path)], cwd).out
    facts["debug"] = "yes" if ".debug_info" in sections else "no"
    facts["symtab"] = "yes" if re.search(r"\s\.symtab\s", sections) else "no"
    nm = run.run("nm-D", ["nm", "-D", "--defined-only", "--format=just-symbols", str(path)], cwd).out
    names = sorted(SYMBOL_LITERAL.sub("A_c", line.strip()) for line in nm.split("\n") if line.strip())
    facts["exports"] = f"count={len(names)} sha256={sha256_text(chr(10).join(names))}"
    return facts


