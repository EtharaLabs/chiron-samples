<!-- CANARY-BEGIN -->
<!-- CANARY-SLOT-1: 34f0b97a4489d4563e45a8fa359f8255e984bbb35d3f62fc57865d6a1254ec74 -->
<!-- CANARY-SLOT-2: e58cc12e370dd698e69912df2f06f1d37184f6b5b1f00c775976cb520f7ef719 -->
<!-- CANARY-END -->

# Ground truth: Yosys from GNU Make to CMake

## Source

- Repository: github.com/YosysHQ/yosys
- State A: 8125af88d61c61dc47c23537abb32e87bd1206e8
- Upstream migration: https://github.com/YosysHQ/yosys/pull/5895, merged as 337df7fc75de3eda07bec2f62251aadb2119c2cb on 2026-06-03T11:08:45Z; opened 2026-05-18T13:55:42Z
- Submodule abc: github.com/YosysHQ/abc@5d51a5e420f5de493d07bf61109a977248c86ffb
- Submodule libs/cxxopts: github.com/jarro2783/cxxopts@4bf61f08697b110d9e3991864650a405b3dd515d

Upstream later repaired parts of its own merge:

- 7d40a0b6: the CMake build reported version 0.64 instead of 0.66
- 81bf182a: yosys-config dropped the link flags plugins need
- 1277640b: synth passes lacked dependency edges on generated sources
- ce359ea8: the CI release build needed its own build type preset

## Reference

The upstream merge 337df7fc plus every repair listed under repairs, applied to the State A tree as one patch. Each repair restores State A behaviour the merge changed.

- Patch: solution/reference.patch, applied in /app with `git apply --binary --whitespace=nowarn`
- Patch body digest (from its first diff line): 926f3ec14332c1f0916510636c03b9f7e2ef520aab992c725f7552e5b26c1400

## Assets

Every asset names its payload, its modality, who sees it, how it is made, what pins it, the licence chain that covers it and where each licence is stated, and its fidelity. No asset's licence is uncertain: each is stated by a file inside the asset or by its upstream package metadata, read at authoring. The task carries no decoy assets.

### A01-state-a-tree

- Payload: The Yosys source tree at State A with its two submodules as plain directories: the agent's workdir /app and the verifier's private copy at /opt/grader/statea.
- Modality: source tree; seen by: agent-visible, verifier-only
- Made by: git archive of the commit with export-subst (the .gitcommit file carries the commit id), plus git archive of each submodule commit into its path; no object database, refs, remote or history is copied.
- Pin repository: github.com/YosysHQ/yosys@8125af88d61c61dc47c23537abb32e87bd1206e8
- Pin abc: github.com/YosysHQ/abc@5d51a5e420f5de493d07bf61109a977248c86ffb
- Pin libs/cxxopts: github.com/jarro2783/cxxopts@4bf61f08697b110d9e3991864650a405b3dd515d
- Licence: ISC (Yosys, COPYING)
- Licence: University of California permission notice (ABC, abc/copyright.txt)
- Licence: MIT (cxxopts, libs/cxxopts/LICENSE)
- Licence: public domain (libs/bigint/README; libs/sha1/sha1.h)
- Licence: MIT (libs/dlfcn-win32/dlfcn.h; libs/json11/json11.hpp; libs/minisat/LICENSE; libs/fst/fstapi.cc and fastlz.cc)
- Licence: BSD-2-Clause (libs/fst/lz4.cc)
- Licence: ISC permission notice (libs/ezsat/ezsat.h; libs/subcircuit/subcircuit.h)
- Fidelity: real upstream bytes, unmodified; decoys: none

### A02-reference-patch

- Payload: The reference: the upstream merge of pull request 5895 applied to State A, with every repair that restores State A behaviour, as one binary git patch.
- Modality: patch; seen by: oracle-only
- Made by: git diff of the State A tree against the reference tree, both written as git trees in a scratch repository; solve.sh checks the body digest and applies it with git apply.
- Pin upstream_merge: github.com/YosysHQ/yosys@337df7fc75de3eda07bec2f62251aadb2119c2cb
- Pin body_sha256: the reference body_sha256 above
- Licence: ISC (Yosys, as the upstream merge)
- Licence: BSD-3-Clause-style notice of the University of California (libs/flex/FlexLexer.h, vendored by the upstream merge)
- Licence: the repairs are authored for this task and add no third-party text
- Fidelity: derived; the upstream merge plus the disclosed repairs R01 to R29; decoys: none

### A03-base-images

- Payload: The base images both Dockerfiles build from, and the Dockerfile frontend; the Alpine image only lends its CA bundle to the build stages that fetch over TLS and is in no final stage.
- Modality: container images; seen by: agent-visible, verifier-only
- Made by: pulled by digest for the target platform
- Pin ubuntu: ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852
- Pin alpine: alpine:3@sha256:be2656061b536c1883c4034790e21d0a09c5aaaf51968b89368a3c0371ea2c9a
- Pin frontend: docker/dockerfile:1@sha256:4edf897a3ffa55b89f906fc8cc78afdb3f1834cc9c7083565e611a8a7d5fe99e
- Licence: each Ubuntu package's /usr/share/doc/<package>/copyright, shipped in the image
- Licence: MPL-2.0 AND MIT (the Alpine ca-certificates-bundle package, used only at build time)
- Licence: Apache-2.0 (the BuildKit Dockerfile frontend)
- Fidelity: real upstream bytes, unmodified; decoys: none

### A04-ubuntu-packages

- Payload: Every apt package of both images: the toolchain (CMake, Ninja, clang, gcc, bison, flex, Python), the libraries Yosys links, Icarus Verilog, vcd2fst, graphviz and the CA store.
- Modality: operating-system packages; seen by: agent-visible, verifier-only
- Made by: apt from the dated snapshot with every version written in the Dockerfile, each file checked against the hash the snapshot's signed index gives it.
- Pin snapshot: https://snapshot.ubuntu.com/ubuntu/20261101T000000Z (noble, noble-updates, noble-security; main, universe)
- Licence: each package's /usr/share/doc/<package>/copyright, shipped in the image (Ubuntu main and universe)
- Fidelity: real upstream bytes, unmodified; decoys: none

### A05-wheelhouse

- Payload: The offline wheelhouse the wheel build installs from, at /opt/wheelhouse in both images.
- Modality: Python wheels; seen by: agent-visible, verifier-only
- Made by: downloaded from files.pythonhosted.org and checked by sha256 in the Dockerfile
- Pin pybind11-3.0.4-py3-none-any.whl: 961720ee652da51d531b7b2451a6bd2bc042b0106e6d9baa48ecb7d58034ce63
- Pin cxxheaderparser-1.9.2-py3-none-any.whl: db2abd1467c43a03d4e1c3b357ba87095aee6d673b7284538d8e0d82adc83637
- Pin setuptools-82.0.1-py3-none-any.whl: a59e362652f08dcd477c78bb6e7bd9d80a7995bc73ce773050228a348ce2e5bb
- Pin wheel-0.48.0-py3-none-any.whl: 3217dcc807155e45db462d7ef2431f5ddda0d7273b700d05a67b271ceb1287ab
- Pin packaging-26.3-py3-none-any.whl: d7193f7c8e4e93f444fde0262bf90af30e16fa0ad0ad44cb553c87339b23cd1c
- Licence: BSD-3-Clause (pybind11, cxxheaderparser; License-Expression in each wheel's METADATA)
- Licence: MIT (setuptools, wheel; License-Expression in each wheel's METADATA)
- Licence: Apache-2.0 OR BSD-2-Clause (packaging; License-Expression in its METADATA)
- Fidelity: real upstream bytes, unmodified; decoys: none

### A06-wasi-sdk

- Payload: WASI SDK 33 for the image's own architecture, at /opt/wasi-sdk in both images, which the WASI job builds with.
- Modality: toolchain archive; seen by: agent-visible, verifier-only
- Made by: the release archive for the build platform's architecture from the WebAssembly/wasi-sdk release wasi-sdk-33, checked by sha256 and unpacked.
- Pin wasi-sdk-33.0-arm64-linux.tar.gz: 4f98ee738c7abb45c81a94d1461fc53cc569d1cd01498951c8184d841a027844
- Pin wasi-sdk-33.0-x86_64-linux.tar.gz: 0ba8b5bfaeb2adf3f29bab5841d76cf5318ab8e1642ea195f88baba1abd47bce
- Licence: Apache-2.0 WITH LLVM-exception (the wasi-sdk repository's LICENSE at tag wasi-sdk-33; its VERSION file names LLVM 22.1.0 and wasi-libc 161b3195fc25)
- Fidelity: real upstream bytes, unmodified; decoys: none

### A07-verifier-python

- Payload: Reward Kit and the libraries it imports, in the verifier image's own environment at /opt/rewardkit.
- Modality: Python wheels; seen by: verifier-only
- Made by: pip with --require-hashes from tests/.grader/rewardkit.lock.txt, binary wheels only.
- Pin harbor-rewardkit-0.2.1: e0108e9c4367f9850c77feea50ee7428ce6e443dc91c5d73c0fc19b9de56936b
- Pin lock: tests/.grader/rewardkit.lock.txt, every distribution by hash
- Licence: Apache-2.0 (harbor-rewardkit; License-Expression in its METADATA)
- Licence: MIT, Apache-2.0, BSD, PSF-2.0 and MPL-2.0 (used unmodified) for the 58 locked dependencies that install on Linux, each stated in its METADATA
- Fidelity: real upstream bytes, unmodified; decoys: none

### A08-recorded-expectations

- Payload: What the State A build does, case by case, for every Stage II group and Stage III family, per architecture, with the cases excluded or carried and why.
- Modality: recorded outputs; seen by: verifier-only
- Made by: the grader engine records the State A make build three times on arm64 and twice on amd64, and keeps only the cases every recording of that architecture agrees on; the test harness is run as make -C tests -j<N> vanilla-test SEEDOPT=-S 1, the seeded form of the SEED=1 the CI test job uses, both when recording and when grading; amd64 is recorded under emulation and carries the arm64 sanitizer group, which emulation cannot run.
- Pin files: tests/.grader/expect/, bound by the canonical bundle hash
- Licence: outputs of ISC-licensed Yosys run on authored inputs
- Fidelity: derived from State A by recording; decoys: none

### A09-authored-grading

- Payload: The statement, the grader engine and its inputs, the checkers, the judge layer and its prompts, test.sh and both Dockerfiles.
- Modality: authored text and code; seen by: agent-visible, verifier-only
- Made by: written for this task.
- Pin files: instruction.md, environment/, tests/, bound by the canonical bundle hash
- Licence: authored for this task and its client; no third-party text
- Fidelity: authored; decoys: none

## Golden trajectory

### T1: D-MAKE-GONE

- Action: Delete the top-level Makefile and every Makefile.inc outside abc/, and rewrite every workflow, action, packaging script, Nix expression and Brewfile step that ran make on the repository root or on abc/.
- Established state: No make-driven build of Yosys or ABC remains; the tests/ and docs/ harness Makefiles, the example and third-party Makefiles and everything inside abc/ stay.
- Must survive: a workflow step that changes to the root and runs make, and a setup.py that still shells out to make
- Checkers: S1-make-build-absent

### T2: D-CMAKE

- Action: Write CMakeLists.txt files and cmake/ modules that build yosys, yosys-abc from ABC's sources, libyosys, yosys-filterlib, yosys-config, the Python tools, the generated sources and the share data with Ninja, and install them under the prefix.
- Established state: The default configure, build, unit tests and install succeed offline from a read-only source tree, and incremental builds pick up every edited input.
- Must survive: a read-only source tree, a no-op rebuild, and edits to share files, cell help, generators, the config template and Python tools
- Checkers: S1-cmake-default-installs, F-readonly-outside-build, F-edit-share-file, F-edit-python-tool, F-edit-cellhelp, F-delete-share-product, F-edit-config-template, F-noop-rebuild, F-edit-generator, J-hardcoded-host-path

### T3: R-NO-FORWARD

- Action: Compile and link every product with CMake's own rules, building ABC from its module lists rather than through ABC's Makefile.
- Established state: yosys, yosys-abc and libyosys are linked by native rules from native objects.
- Must survive: a custom command that runs make in abc/, or replays recorded compile lines
- Checkers: S1-no-build-forwarding, G-native-build

### T4: R-NO-EMBED

- Action: Generate every derived source and binary at build time and commit none of them.
- Established state: The tree holds no copy of the old build's files or outputs under any name or encoding.
- Must survive: a prebuilt binary or a generated parser hidden in an archive or a base64 blob
- Checkers: S1-no-embedded-old-build

### T5: R-NO-VERIFIER

- Action: Build and test only through the commands the statement names.
- Established state: Nothing in the tree reads or detects the grader.
- Must survive: a build step that checks for grader paths or users
- Checkers: S1-no-verifier-awareness

### T6: D-BEHAVIOUR

- Action: Reproduce the Makefile's silent defaults: -O3 -ggdb without NDEBUG, -std=c++20, -fPIC, the version banner, the absolute data directory, install stripping, exported symbols, the yosys-config script, libyosys built by default with SONAME libyosys.so, the lib/<prefix>yosys library directory, and every option's make-variable equivalent.
- Established state: Binaries, library, installed files, options and consumer routes behave as State A's.
- Must survive: CMake's default empty build type, GNUInstallDirs, generated config headers, pkg-config flag leakage and default symbol visibility
- Checkers: S2-suite-frontends, S2-suite-passes, S2-suite-techmap-arch, S2-suite-sim-formal, S2-command-reference, S2-compiled-features, S2-install-layout, S2-binary-abi, S2-consumer-routes, S2-option-matrix, F-alt-installed-consumer, F-off-readline, F-alt-relocated-share, F-static-consumer, F-off-tcl, F-alt-python-tools, F-datdir-copied-binary, F-install-without-build, F-wasi-default, F-usr-libdir, F-help-source, F-off-plugins, F-off-zlib, F-debug-install, F-alt-noabc-synth, F-version-checkout, F-tools-relocated, F-debug-backtrace, F-debug-banner, J-tests-weakened

### T7: D-CI

- Action: Migrate every workflow job to the CMake build while keeping its jobs, runners, matrices, dependencies, artifacts, options and triggers.
- Established state: Every job that ran before runs the CMake build with the same coverage.
- Must survive: dropped artifact members, Release builds that add NDEBUG, weakened sanitizer options and harness configuration lost with Makefile.conf
- Checkers: S2-ci-test-build, S2-ci-test-compile, S2-ci-test-sanitizers, S2-ci-extra-builds, S2-ci-docs, J-ci-job-dropped, J-ci-failure-masked, J-ci-triggers-changed

### T8: D-PACKAGING

- Action: Build the Python wheel and the vendored source package through CMake with State A's metadata, version, contents and supported Python versions.
- Established state: The wheel and the vendored archive match State A's.
- Must survive: an empty long description, a raised Python floor and a version read from the wrong place
- Checkers: S2-ci-wheels, S2-ci-source-vendor, J-wheels-config-changed, J-version-duplicated

## Compiled items

| id | stage | obligation | dimension | weight | target | check |
| --- | --- | --- | --- | --- | --- | --- |
| S1-make-build-absent | stage1 | D-MAKE-GONE | outcome | 5 | submission_tree | The collected tree has no top-level Makefile, makefile, GNUmakefile or Makefile.conf, no Makefile.inc or other make build file of Yosys outside abc/, libs/cxxopts/ and the harness and example Makefiles the statement names, and no workflow, action, packaging script, flake.nix, nix/*.nix or Brewfile recipe that runs make on the repository root or on abc/. |
| S1-cmake-default-installs | stage1 | D-CMAKE | outcome | 5 | graded_build | The statement's default configure (clang, Ninja, no build type), build, unit-test run and install into a staging prefix all exit 0 offline, and the installed yosys prints its version. |
| S1-no-embedded-old-build | stage1 | R-NO-EMBED | constraints | 5 | submission_tree | No file new or changed against State A, directly or inside a gzip, bzip2, xz, zip or tar archive or a long base64 run, carries the old Make build's text, an executable, object, archive or WebAssembly binary, or a source the old build generated (parser and lexer outputs, version source, help includes, generated techlib files). |
| S1-no-build-forwarding | stage1 | R-NO-FORWARD | constraints | 5 | graded_build | No make process runs during the graded configure, build and install; no CMake file or script the CMake build runs invokes make, gmake or CMAKE_MAKE_PROGRAM outside the tests and docs harnesses, and no ExternalProject or FetchContent appears; in the generated Ninja graph yosys, yosys-abc and, when defined, libyosys.so are linked by CMake's native rules from at least 100 objects its native compile rules produce. |
| S1-no-verifier-awareness | stage1 | R-NO-VERIFIER | constraints | 5 | submission_tree | No file new or changed against State A names a grader path (/logs/verifier, /opt/grader, /var/grader, an absolute /tests path), a reward or report file name, the reward tool, or a grader environment variable prefix that its State A version did not already name. |
| S2-ci-test-build | stage2 | D-CI | coverage | 5 | ci_step_logs | Workflow inventory kept; build, unit tests, yosys-config log and packed artifact members as State A; the submission's own harness passes every State A test module from the artifact; the four test_cell runs and the docs test job match State A. |
| S2-ci-test-compile | stage2 | D-CI | coverage | 5 | ci_step_logs | Workflow inventory kept; each runnable compiler entry configures, builds yosys with the job's CXXFLAGS, reports the same version banner and yosys-config flags and runs a synthesis. |
| S2-ci-test-sanitizers | stage2 | D-CI | coverage | 5 | ci_step_logs | Workflow inventory kept; the undefined,address sanitizer build passes every State A test module under halt_on_error=1 with State A's version banner and yosys-config flags. |
| S2-ci-extra-builds | stage2 | D-CI | coverage | 5 | ci_step_logs | Workflow inventory kept except vs-prep and its vcxsrc artifact; the WASI job's configuration builds the same WebAssembly modules with the same imports, exports, memory limits and stack size as State A. |
| S2-ci-wheels | stage2 | D-PACKAGING | coverage | 5 | ci_step_logs | Workflow inventory kept; the offline wheel build gives State A's wheel name, file list, RECORD paths, METADATA fields and description, packaged share files and binaries, and the installed wheel imports and passes every State A pyosys test. |
| S2-ci-source-vendor | stage2 | D-PACKAGING | coverage | 5 | ci_step_logs | Workflow inventory kept; the vendored source archive expands .gitcommit and builds without git, reporting State A's version banner and running a synthesis. |
| S2-ci-docs | stage2 | D-CI | coverage | 5 | ci_step_logs | Workflow inventory kept; the docs test run produces State A's code example outputs and the command and cell reference dumps match State A. |
| S2-suite-frontends | stage2 | D-BEHAVIOUR | coverage | 5 | graded_build | State A's own harness run against the default build passes the same tests in each frontend module (verilog, svtypes, svinterfaces, simple, hana, asicworld, realmath, blif, rtlil, aiger, liberty, bind). |
| S2-suite-passes | stage2 | D-BEHAVIOUR | coverage | 5 | graded_build | State A's own harness run against the default build passes the same tests in each pass module (opt, proc, fsm, share, select, peepopt and the rest). |
| S2-suite-techmap-arch | stage2 | D-BEHAVIOUR | coverage | 5 | graded_build | State A's own harness run against the default build passes the same tests in arch, techmap, memlib, bram, memories, arith_tree and alumacc. |
| S2-suite-sim-formal | stage2 | D-BEHAVIOUR | coverage | 5 | graded_build | State A's own harness run against the default build passes the same tests in sim, sat, xprop, cxxrtl, fmt, memfile, sdc, bugpoint and rpc. |
| S2-command-reference | stage2 | D-BEHAVIOUR | coverage | 5 | graded_build | The command and cell lists, every command's and cell's help text and the usage of yosys, yosys-smtbmc, yosys-witness, yosys-filterlib and yosys-abc equal State A's. |
| S2-compiled-features | stage2 | D-BEHAVIOUR | coverage | 5 | graded_build | The git-less version banner, Tcl, zlib, glob, plugins and DPI, ABC, spawn, NDEBUG-visible debug output, backtraces, share lookup and number formatting over boundary inputs equal State A's. |
| S2-install-layout | stage2 | D-BEHAVIOUR | coverage | 5 | graded_install | The installed paths, modes, symlinks and normalised file contents equal State A's. |
| S2-binary-abi | stage2 | D-BEHAVIOUR | coverage | 5 | graded_install | Built and installed yosys, yosys-abc, yosys-filterlib and libyosys.so have State A's ELF type, NEEDED set, SONAME, RUNPATH, debug and symbol tables and exported symbol names. |
| S2-consumer-routes | stage2 | D-BEHAVIOUR | coverage | 5 | graded_install | yosys-config answers as State A's, a plugin builds with it and loads, the C++ API example builds against libyosys and runs, and the installed Python tools, yosys-abc and yosys-filterlib behave as State A's. |
| S2-option-matrix | stage2 | D-BEHAVIOUR | coverage | 5 | graded_build | Each named option configures alone and its yosys-config flags equal State A's under the make-variable equivalent; the alternate prefix, program prefix and library install build and the static library match State A's. |
| F-alt-installed-consumer | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | Under /opt/yosys with program prefix gr- and the library installed, the C++ API example builds against the installed library and runs. |
| F-readonly-outside-build | stage3 | D-CMAKE | robustness | 3 | graded_build | A read-only source tree builds in an outside build directory and its generated share files load. |
| F-edit-share-file | stage3 | D-CMAKE | robustness | 3 | graded_build | An edited techlib source reaches the build tree and the install on the next build and install. |
| F-off-readline | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | With readline off, the interactive prompt reads piped commands as State A's without readline. |
| F-alt-relocated-share | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | The program-prefixed install moved to another directory finds its own share data. |
| F-static-consumer | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | The C++ API example links the static libyosys.a with the installed yosys-config flags and runs as State A's. |
| F-off-tcl | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | With Tcl off, the Tcl front end and help tcl behave as State A's without Tcl. |
| F-edit-python-tool | stage3 | D-CMAKE | robustness | 3 | graded_build | An edited yosys-smtbmc source reaches the built tool on the next build. |
| F-edit-cellhelp | stage3 | D-CMAKE | robustness | 3 | graded_build | An edited cell help comment reaches help on the next build. |
| F-alt-python-tools | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | The program-prefixed yosys-smtbmc and yosys-witness installed under /opt/yosys run. |
| F-datdir-copied-binary | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | A copy of the built yosys run from an empty directory finds the installed share data and synthesises. |
| F-install-without-build | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | With the build directory gone, the installed tools synthesise with ABC, build and load a plugin and find their data. |
| F-delete-share-product | stage3 | D-CMAKE | robustness | 3 | graded_build | A deleted share file in the build tree comes back on the next build. |
| F-wasi-default | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | The WASI build without the CI's link flags gives State A's modules, memory limits and stack size. |
| F-edit-config-template | stage3 | D-CMAKE | robustness | 3 | graded_build | An edited yosys-config template reaches the built yosys-config on the next build. |
| F-usr-libdir | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | With prefix /usr and the library installed, the installed paths equal State A's. |
| F-help-source | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | With help source enabled, every command's recorded source file and function equal State A's. |
| F-off-plugins | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | With plugins off, the plugin command and DPI imports behave as State A's without plugins. |
| F-off-zlib | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | With zlib off, compressed reads and writes behave as State A's without zlib. |
| F-debug-install | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | The Debug install keeps debug information and symbols in yosys, yosys-abc and yosys-filterlib as the debug make install does. |
| F-alt-noabc-synth | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | A build without ABC answers synth with ABC and help abc as State A's. |
| F-noop-rebuild | stage3 | D-CMAKE | robustness | 3 | graded_build | A second build with no change leaves every product untouched as State A's does. |
| F-version-checkout | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | Built from a dirty git checkout three commits past the v0.66 tag on a branch of a fork, the banner names version, revision, dirty state and origin as State A's. |
| F-edit-generator | stage3 | D-CMAKE | robustness | 3 | graded_build | An edited techlib generator script reruns on the next build and its output loads. |
| F-tools-relocated | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | The installed yosys-smtbmc and yosys-witness copied with their prefix tree run without the original install's Python modules. |
| F-debug-backtrace | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | The Debug build prints the same backtrace frames under -X as the debug make build. |
| F-debug-banner | stage3 | D-BEHAVIOUR | robustness | 3 | graded_build | The Debug build's version banner equals the debug make build's. |

Veto: any failed Stage I item, or at least 1 failed Stage II group, scores the task 0.
Cheat checks: S1-no-embedded-old-build, S1-no-build-forwarding, S1-no-verifier-awareness.

## Judged items

| id | layer | obligation | dimension | weight | act |
| --- | --- | --- | --- | --- | --- |
| G-native-build | gate | R-NO-FORWARD | process | 5 | The CMake build of Yosys or ABC, or a script or command that build runs, produces a product without compiling its sources: it invokes make or gmake on the project's or ABC's own Makefiles, replays compile or link command lines recorded from the old build, or copies, downloads or unpacks prebuilt binaries, libraries or object files. |
| J-ci-job-dropped | judged | D-CI | process | 3 | A job of the original CI that needs macOS, Windows, Nix, a self-hosted runner, Verific, CodeQL or cibuildwheel was deleted, still builds Yosys or ABC with make, or no longer builds the project. Removing the Visual Studio source-preparation job vs-prep and its vcxsrc artifact does not count. |
| J-ci-failure-masked | judged | D-CI | constraints | 5 | A migrated workflow step hides a failure its original step would have reported: it adds an `|| true`, a continue-on-error, a set +e or an ignored exit status, drops a build, test or check command the original step ran, or passes sanitizer, warning or test options weaker than the original's. |
| J-ci-triggers-changed | judged | D-CI | constraints | 3 | A migrated workflow changes when or whether a job runs for a reason the new build does not require: its on triggers, a job or step if condition, its concurrency, its permissions or its duplicate-run skipping differ from the original's. |
| J-wheels-config-changed | judged | D-PACKAGING | process | 3 | The migrated wheel packaging changes what the wheel jobs build or test beyond replacing the build commands: the cibuildwheel version, its build or skip selectors, the Python versions, the runner matrix, the manylinux images, the dependencies prepared before the build or the wheel test command differ from the original's. |
| J-version-duplicated | judged | D-PACKAGING | craft | 1 | The Python package version is written as a literal in the packaging files instead of being read from the place the CMake build defines the Yosys version. |
| J-hardcoded-host-path | judged | D-CMAKE | craft | 3 | A CMake file, or a script the CMake build runs, hard-codes an absolute path of one machine or installation, such as a compiler, interpreter, library, include directory or prefix, where the build should find the tool or use CMake's own variables. A #!/usr/bin/env shebang does not count. |
| J-tests-weakened | judged | D-BEHAVIOUR | constraints | 5 | The migrated test harness under tests/ removes, skips or weakens an original test: a test directory or script no longer runs, an expected output or reference file changed, a comparison was loosened, or a test now runs only under a condition the original did not have. Changes that only point the harness at the new build directory and tool names do not count. |

## Recorded State A literals

Every exit code, golden string, error text, flag and option a graded case asserts was recorded from the State A build into tests/.grader/expect/<arch>/cases.json. Each graded group is listed with its case count, the cases left out because the recordings disagreed, and the SHA-256 of its cases in canonical JSON (sorted keys, no whitespace).

### amd64

| item | stage | cases | left out | sha256 | source |
| --- | --- | --- | --- | --- | --- |
| S2-ci-test-build | stage2 | 115 | 0 | 1c7c230db88c2dd3f845b8d4f96ef25c6e53e7d43b7586ebeebacf2f03aa6240 | recorded |
| S2-ci-test-compile | stage2 | 22 | 0 | f9ae7de8529d8908c6484c431a505f6d00cabe5e197771778174449166096310 | recorded |
| S2-ci-test-sanitizers | stage2 | 67 | 0 | 23a94f6635c54e9717bb20574892328355aadb3051f6ac786fa73d26427648c4 | carried from arm64 |
| S2-ci-extra-builds | stage2 | 21 | 0 | 6978886f2c4f24c2c36a929e9e129a94e016363b4e6c8fa027fb12a4042b829f | recorded |
| S2-ci-wheels | stage2 | 387 | 0 | addbc3256a126b8bc42f3ea4aebede6dc09fb6f13534fcf85d23a8f23dc3623b | recorded |
| S2-ci-source-vendor | stage2 | 7 | 0 | 1cdc61e7e4d2fc44a68c99fb499b61633025bce0a2830115d0ea331ae0ac9b7a | recorded |
| S2-ci-docs | stage2 | 40 | 0 | 763544524a11e7f1320787972d9747d943f6b9653af0eb597a8d78c5e95f08f6 | recorded |
| S2-suite-frontends | stage2 | 12 | 0 | af607e3629d89ac3fcce4e5acd21814ee58b3295fe44ca536272193a254ec559 | recorded |
| S2-suite-passes | stage2 | 9 | 0 | f896be61bbace5e02b00782c573b5e26f598bc342adda70068dd94d5b42713da | recorded |
| S2-suite-techmap-arch | stage2 | 23 | 0 | 18f955f5cac4aa1fdc0732c3bfea890e066649922c3305b898bd19f34b71c5e7 | recorded |
| S2-suite-sim-formal | stage2 | 9 | 0 | bb67d2b7a219c99c2220e2ecc4520a3564cd361be19383b8e1a9122b1720cee3 | recorded |
| S2-command-reference | stage2 | 552 | 0 | 0d58d27ac2c44b9f85d8aa8738ff7717d7fce696a810f7a99696e24fec4de83f | recorded |
| S2-compiled-features | stage2 | 304 | 0 | d1803c1851028e71df0c5048dfb82732f3eed4f6859a0e659cb90c485161ecea | recorded |
| S2-install-layout | stage2 | 340 | 0 | 4e07a82afe9dc45a6cb838e6bf83c84e3fd9c498c2d662bad03889baeea26834 | recorded |
| S2-binary-abi | stage2 | 63 | 0 | d0168889746b0d9e178fa63b98ca7a6cc4428d524436bd29fca13dbddea080fb | recorded |
| S2-consumer-routes | stage2 | 25 | 0 | 1460ffee55893fedaa49e2f8ff17238dd13636d087cc3f78963354d533b99a26 | recorded |
| S2-option-matrix | stage2 | 78 | 0 | e671662cf34cd70aadd5623506e7da777a8b1d6f2a7230da020e1c68ab5c10b4 | recorded |
| F-alt-installed-consumer | stage3 | 4 | 0 | ea7d630f5cef014df05cba7bdb50971c2db1d18e5f1f0b1553722fd47e29ba40 | recorded |
| F-readonly-outside-build | stage3 | 2 | 0 | bb8282634fb70058b538e8a5e3fb388e7ad19cff03f21185e3cd5ed79894e94b | recorded |
| F-edit-share-file | stage3 | 2 | 0 | 29e83380fd5d7c819c01e76a60f0cebbe41ef0706880618fd683153aee8fdf43 | recorded |
| F-off-readline | stage3 | 2 | 0 | b8155730ce27d3ac59f84dc37cbe2aaabd09d00d8e8f9a95104aa729a1bb6ade | recorded |
| F-alt-relocated-share | stage3 | 2 | 0 | f90e7d4e414d89d2d2c7cd778fb907481ae80032f6428a523e3056e8ef1d0223 | recorded |
| F-static-consumer | stage3 | 4 | 0 | 91e689407f763ec4723fee1259b13b72277f27dac185f4052345e423cdbc76a2 | recorded |
| F-off-tcl | stage3 | 3 | 0 | d982b60aa2bfa0335c7de2a377003c5263d7cecfcac12d7608d0c4c4e15bce0f | recorded |
| F-edit-python-tool | stage3 | 1 | 0 | abf6875544cd69785218d61b5399649fc61fc99cb33b876d791c81859203b49f | recorded |
| F-edit-cellhelp | stage3 | 1 | 0 | 569a2d4025724c154ba3fb66f2386fe0ade67ec00e29d7075a78e545fa0da964 | recorded |
| F-alt-python-tools | stage3 | 2 | 0 | 143824181d9ebd2be8e45780d09892958c557b44b1b61a20947e6b3456cd03bc | recorded |
| F-datdir-copied-binary | stage3 | 2 | 0 | 556e2440f0372424d94d3bb353221195abb00aeb0efdfcadd016d9adfc6b2e51 | recorded |
| F-install-without-build | stage3 | 7 | 0 | 07599b8808674f9cb96d660b407c9589eb55eee52d79eaccf2fd02af39cb6763 | recorded |
| F-delete-share-product | stage3 | 1 | 0 | 77c31693ea05cf97b53e93697a18a024e840f732e2410e2b6d4a2606b02f9e07 | recorded |
| F-wasi-default | stage3 | 7 | 0 | 90fae5cd5faa8098486dd2450407a8b3d79ee3855a8f8dc997bb5e1e2aba4d8f | recorded |
| F-edit-config-template | stage3 | 1 | 0 | 77186ad6f6ca4a5283bb3e2a5f3b3d1b23869df7da1d04905d3a3f246f7a20d9 | recorded |
| F-usr-libdir | stage3 | 2 | 0 | 7795816e9a7144184905e42040c041cf42fbe2fcaa1346bc899dcf2e3c106fbd | recorded |
| F-help-source | stage3 | 2 | 0 | f6bf3009a546c0bd61501f18c4dba74686866b4a895c0c00266bd2faaf9d71a4 | recorded |
| F-off-plugins | stage3 | 3 | 0 | 0be9a5040ccd1b9d4930f17660e0ef008098f2085b146e914297f573816da716 | recorded |
| F-off-zlib | stage3 | 3 | 0 | cc5f52285af39d6154b33ac150ed828a0b8d6c13a7f09d108e0a2055114d29f6 | recorded |
| F-debug-install | stage3 | 4 | 0 | b4eb350cae9254f0096204fc409de30eb188d93c0dd4bf13bfc6cc06478af785 | recorded |
| F-alt-noabc-synth | stage3 | 2 | 0 | ecac8da0c52fe166dee7da5a72fb23ae169c5fa5fda4d495c2ed36a05b8d2c2d | recorded |
| F-noop-rebuild | stage3 | 1 | 0 | 427d8d81a3f40d4f9154c0e84f435cbe099eb9e96add003639a4a1b9604e8925 | recorded |
| F-version-checkout | stage3 | 2 | 0 | 4922f4f5b5557a8b560e1ff22b7ebd40e47aa1f963855bbebfe0b428d865fe6d | recorded |
| F-edit-generator | stage3 | 1 | 0 | 3d95872da430099ed732a1c763d021db5ccfd62fe5398945e6c847b2bd4a9bca | recorded |
| F-tools-relocated | stage3 | 2 | 0 | 55fa797a688f463e5db9733ee40ee17deea1ce0bd32dada4e29bb6297e859e76 | recorded |
| F-debug-backtrace | stage3 | 2 | 0 | 65b769eedb52494ffdd078018c644ef1967979fcec7c9b1e385b0697e75ab1f3 | recorded |
| F-debug-banner | stage3 | 1 | 0 | 0db30ab457e1e86855a330cc16814d0f6f5b0a922a79b93f019279dcb92a331e | recorded |

### arm64

| item | stage | cases | left out | sha256 | source |
| --- | --- | --- | --- | --- | --- |
| S2-ci-test-build | stage2 | 115 | 0 | 1c7c230db88c2dd3f845b8d4f96ef25c6e53e7d43b7586ebeebacf2f03aa6240 | recorded |
| S2-ci-test-compile | stage2 | 22 | 0 | adeb1571603706e2cc3e1c6b13b5f939554dcc3099a350490d04577274527075 | recorded |
| S2-ci-test-sanitizers | stage2 | 67 | 0 | 23a94f6635c54e9717bb20574892328355aadb3051f6ac786fa73d26427648c4 | recorded |
| S2-ci-extra-builds | stage2 | 21 | 0 | 6978886f2c4f24c2c36a929e9e129a94e016363b4e6c8fa027fb12a4042b829f | recorded |
| S2-ci-wheels | stage2 | 387 | 0 | 107ec253143adc301c12119f13c60c5a51dc9f990fc90cacc466f67663e81fe1 | recorded |
| S2-ci-source-vendor | stage2 | 7 | 0 | 1cdc61e7e4d2fc44a68c99fb499b61633025bce0a2830115d0ea331ae0ac9b7a | recorded |
| S2-ci-docs | stage2 | 40 | 0 | 763544524a11e7f1320787972d9747d943f6b9653af0eb597a8d78c5e95f08f6 | recorded |
| S2-suite-frontends | stage2 | 12 | 0 | af607e3629d89ac3fcce4e5acd21814ee58b3295fe44ca536272193a254ec559 | recorded |
| S2-suite-passes | stage2 | 9 | 0 | f896be61bbace5e02b00782c573b5e26f598bc342adda70068dd94d5b42713da | recorded |
| S2-suite-techmap-arch | stage2 | 23 | 0 | 18f955f5cac4aa1fdc0732c3bfea890e066649922c3305b898bd19f34b71c5e7 | recorded |
| S2-suite-sim-formal | stage2 | 9 | 0 | bb67d2b7a219c99c2220e2ecc4520a3564cd361be19383b8e1a9122b1720cee3 | recorded |
| S2-command-reference | stage2 | 552 | 0 | 0d58d27ac2c44b9f85d8aa8738ff7717d7fce696a810f7a99696e24fec4de83f | recorded |
| S2-compiled-features | stage2 | 304 | 0 | 8df342979a462dca01d3c88d08b8d05271a0648a4c3935282939fbeca33bd666 | recorded |
| S2-install-layout | stage2 | 340 | 0 | 4e07a82afe9dc45a6cb838e6bf83c84e3fd9c498c2d662bad03889baeea26834 | recorded |
| S2-binary-abi | stage2 | 63 | 0 | c9e4faf52333bf555e90abe694158ac67b145256ecf3d813294f5faf9f7470fe | recorded |
| S2-consumer-routes | stage2 | 25 | 0 | 1460ffee55893fedaa49e2f8ff17238dd13636d087cc3f78963354d533b99a26 | recorded |
| S2-option-matrix | stage2 | 78 | 0 | 382d4e3c73822b08559d407e9c0e9aca658584260cd09f5ddc8ac7bd8fa583c4 | recorded |
| F-alt-installed-consumer | stage3 | 4 | 0 | ea7d630f5cef014df05cba7bdb50971c2db1d18e5f1f0b1553722fd47e29ba40 | recorded |
| F-readonly-outside-build | stage3 | 2 | 0 | bb8282634fb70058b538e8a5e3fb388e7ad19cff03f21185e3cd5ed79894e94b | recorded |
| F-edit-share-file | stage3 | 2 | 0 | 29e83380fd5d7c819c01e76a60f0cebbe41ef0706880618fd683153aee8fdf43 | recorded |
| F-off-readline | stage3 | 2 | 0 | b8155730ce27d3ac59f84dc37cbe2aaabd09d00d8e8f9a95104aa729a1bb6ade | recorded |
| F-alt-relocated-share | stage3 | 2 | 0 | f90e7d4e414d89d2d2c7cd778fb907481ae80032f6428a523e3056e8ef1d0223 | recorded |
| F-static-consumer | stage3 | 4 | 0 | 91e689407f763ec4723fee1259b13b72277f27dac185f4052345e423cdbc76a2 | recorded |
| F-off-tcl | stage3 | 3 | 0 | d982b60aa2bfa0335c7de2a377003c5263d7cecfcac12d7608d0c4c4e15bce0f | recorded |
| F-edit-python-tool | stage3 | 1 | 0 | abf6875544cd69785218d61b5399649fc61fc99cb33b876d791c81859203b49f | recorded |
| F-edit-cellhelp | stage3 | 1 | 0 | 569a2d4025724c154ba3fb66f2386fe0ade67ec00e29d7075a78e545fa0da964 | recorded |
| F-alt-python-tools | stage3 | 2 | 0 | 143824181d9ebd2be8e45780d09892958c557b44b1b61a20947e6b3456cd03bc | recorded |
| F-datdir-copied-binary | stage3 | 2 | 0 | 556e2440f0372424d94d3bb353221195abb00aeb0efdfcadd016d9adfc6b2e51 | recorded |
| F-install-without-build | stage3 | 7 | 0 | 07599b8808674f9cb96d660b407c9589eb55eee52d79eaccf2fd02af39cb6763 | recorded |
| F-delete-share-product | stage3 | 1 | 0 | 77c31693ea05cf97b53e93697a18a024e840f732e2410e2b6d4a2606b02f9e07 | recorded |
| F-wasi-default | stage3 | 7 | 0 | 90fae5cd5faa8098486dd2450407a8b3d79ee3855a8f8dc997bb5e1e2aba4d8f | recorded |
| F-edit-config-template | stage3 | 1 | 0 | 77186ad6f6ca4a5283bb3e2a5f3b3d1b23869df7da1d04905d3a3f246f7a20d9 | recorded |
| F-usr-libdir | stage3 | 2 | 0 | 7795816e9a7144184905e42040c041cf42fbe2fcaa1346bc899dcf2e3c106fbd | recorded |
| F-help-source | stage3 | 2 | 0 | f6bf3009a546c0bd61501f18c4dba74686866b4a895c0c00266bd2faaf9d71a4 | recorded |
| F-off-plugins | stage3 | 3 | 0 | 0be9a5040ccd1b9d4930f17660e0ef008098f2085b146e914297f573816da716 | recorded |
| F-off-zlib | stage3 | 3 | 0 | cc5f52285af39d6154b33ac150ed828a0b8d6c13a7f09d108e0a2055114d29f6 | recorded |
| F-debug-install | stage3 | 4 | 0 | b4eb350cae9254f0096204fc409de30eb188d93c0dd4bf13bfc6cc06478af785 | recorded |
| F-alt-noabc-synth | stage3 | 2 | 0 | ecac8da0c52fe166dee7da5a72fb23ae169c5fa5fda4d495c2ed36a05b8d2c2d | recorded |
| F-noop-rebuild | stage3 | 1 | 0 | 427d8d81a3f40d4f9154c0e84f435cbe099eb9e96add003639a4a1b9604e8925 | recorded |
| F-version-checkout | stage3 | 2 | 0 | 4922f4f5b5557a8b560e1ff22b7ebd40e47aa1f963855bbebfe0b428d865fe6d | recorded |
| F-edit-generator | stage3 | 1 | 0 | 3d95872da430099ed732a1c763d021db5ccfd62fe5398945e6c847b2bd4a9bca | recorded |
| F-tools-relocated | stage3 | 2 | 0 | 55fa797a688f463e5db9733ee40ee17deea1ce0bd32dada4e29bb6297e859e76 | recorded |
| F-debug-backtrace | stage3 | 2 | 0 | 65b769eedb52494ffdd078018c644ef1967979fcec7c9b1e385b0697e75ab1f3 | recorded |
| F-debug-banner | stage3 | 1 | 0 | 0db30ab457e1e86855a330cc16814d0f6f5b0a922a79b93f019279dcb92a331e | recorded |

## Discovery values

Established by: Running the State A make build the environment holds, as the recorder does three times: make -f ../Makefile config-clang, make, make unit-test, make install PREFIX=/usr/local and the tools that install brings, the original test harness against those products, and the same build with the settings and in the situations the held-back checks use.

| id | value | graded by |
| --- | --- | --- |
| DV-default-flags | The default build compiles with -O3 and -ggdb and without -DNDEBUG. | S2-compiled-features, S2-consumer-routes, S2-option-matrix |
| DV-config-script | yosys-config answers --cxx with clang++, --linkflags with -rdynamic and --libs with -lstdc++ -lm -lrt -lreadline -lffi -ldl -lz -ltcl8.6 -ltclstub8.6 -lpthread. | S2-consumer-routes, S2-option-matrix, S2-ci-test-compile, S2-ci-test-sanitizers |
| DV-banner-compiler | The version banner names the compiler as clang++ 18.1.3 -fPIC -O3. | S2-compiled-features, S2-ci-source-vendor, S2-ci-test-compile |
| DV-install-strip | The install strips debug information from yosys and strips yosys-abc and yosys-filterlib fully. | S2-install-layout, S2-binary-abi |
| DV-library | The CI build flow produces libyosys.so, which the unit tests link, with the SONAME libyosys.so; an install of the library puts it in lib/<program prefix>yosys. | S2-ci-test-build, S2-binary-abi, S2-option-matrix, F-usr-libdir, F-alt-installed-consumer |
| DV-exports | yosys and yosys-filterlib export their symbols and yosys-abc exports none. | S2-binary-abi, F-install-without-build |
| DV-wheel | The pyosys wheel declares version 0.66, Requires-Python >=3.8, the MIT license and README.md as its description. | S2-ci-wheels |
| DV-wasi | The WASI job's build produces yosys.wasm and no other module. | S2-ci-extra-builds, F-wasi-default |
| DV-share-lookup | yosys looks for its data in share/ beside its binary, then in ../share/<program prefix>yosys from its binary, then in the absolute data directory built into it. | F-datdir-copied-binary, F-alt-relocated-share, F-install-without-build, F-static-consumer, F-alt-installed-consumer |
| DV-debug-build | ENABLE_DEBUG builds with -Og in place of -O3, defines DEBUG and installs without stripping, and ENABLE_HELP_SOURCE records the source file and function of every command. | F-debug-banner, F-debug-backtrace, F-debug-install, F-help-source |
| DV-features-off | With Tcl, zlib, plugins or readline turned off the build still succeeds; the tcl command and the -c option go, gzip input and output stop with an error naming zlib, DPI imports stop with an error naming plugins while the plugin command stays, and commands read from standard input still run. | F-off-tcl, F-off-zlib, F-off-plugins, F-off-readline |
| DV-version-git | Inside a git checkout the banner gives 0.66+ and the number of commits since the v0.66 tag, the abbreviated head hash with -dirty for a modified tree, and the origin's lowercased owner/repository with the branch in brackets unless the origin is YosysHQ/yosys. | F-version-checkout |
| DV-wasi-defaults | A WASI build without the job's linker flags gets a 1 MiB stack and keeps its debug and name sections. | F-wasi-default |
| DV-generated-share | The generated techlib data, the gatemate lut tree files and quicklogic's bram_types_sim.v, is written into the build tree, never into the source tree, and installed from there. | F-readonly-outside-build, F-edit-generator |
| DV-incremental | A rebuild with nothing changed rewrites no product but yosys-abc, and a rebuild after a source file is edited or a product deleted remakes exactly what comes from it: techlib data, cell help, the Python tools, yosys-config and generated techlib files. | F-noop-rebuild, F-delete-share-product, F-edit-share-file, F-edit-cellhelp, F-edit-python-tool, F-edit-config-template, F-edit-generator |
| DV-abc-off | Without ABC every synthesis pass and its techlib data stay; only the abc, abc9, abc9_exe, abc9_ops and abc_new passes go. | F-alt-noabc-synth, S2-option-matrix |
| DV-python-tools | The installed yosys-smtbmc and yosys-witness load their Python modules from share/python3 beside them or from ../share/<program prefix>yosys/python3, so they run from a moved installation. | F-alt-python-tools, F-tools-relocated, F-install-without-build |
| DV-help-texts | help lists 300 commands and 245 cell types, each with the help text the current build prints, and its JSON dumps describe 304 commands and 253 cells. | S2-command-reference, S2-ci-docs |
| DV-harness | The original test harness passes every test in its 53 module directories against the current build's products. | S2-suite-frontends, S2-suite-passes, S2-suite-techmap-arch, S2-suite-sim-formal, S2-ci-test-build |

## Rejected routes

| route | control |
| --- | --- |
| Leave the Make build in place. | C-original |
| Deliver nothing. | C-empty |
| Wrap the Make build in a CMake custom target. | C-make-forwarding |
| Ship the old build's outputs inside the tree. | C-embedded-outputs |
| Detect the grader and adapt to it. | C-planted-hook |
| Take the upstream merge as it landed. | C-merge |
| Build with inverted defaults. | C-negated-gold |
| Ship stub products that print the expected text. | C-stubbed-services |
| Ship CMake files of the right shape that build nothing. | C-format-only |

## Repairs to the upstream merge

### R01: version

- Upstream merge: cmake/YosysVersionData.cmake set the minor version to 64, so the banner read 0.64.
- Reference: The version data reads 0.66, the version State A's Makefile set.

### R02: version

- Upstream merge: The banner said +post even without git, named the compiler by its CMake id and path, and added the build type.
- Reference: cmake/YosysVersion.cmake follows the Makefile: a plain version when git is absent, +post outside a repository, the commit distance only when it is not zero, .gitcommit before git, a -dirty mark, the lowercased owner/repo origin with its branch, and the compiler as its file name, version and -f, -m, -O and -DNDEBUG flags.

### R03: default flags

- Upstream merge: A configure without a build type compiled without optimisation or debug information.
- Reference: Without a build type every Yosys object gets -O3 -ggdb and no NDEBUG, as the Makefile's default.

### R04: default flags

- Upstream merge: Compiles used -std=gnu++20 and ignored a CMAKE_CXX_STANDARD given on the command line, so the C++23 and C++26 configurations still built C++20.
- Reference: Compiles use -std=c++20 unless CMAKE_CXX_STANDARD is given, which is honoured.

### R05: default flags

- Upstream merge: pkg-config's readline flags put -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600 on every compile, and pkg_check_modules added -I/usr/include and -L paths to libffi and Tcl flags.
- Reference: Readline adds no compile flags, and libffi and Tcl flags are read from pkg-config plainly.

### R06: default flags

- Upstream merge: Executables were compiled -fPIE and the driver target carried a leftover -fsanitize=undefined.
- Reference: Every Yosys object is compiled -fPIC except on WASI, and the leftover sanitizer flag is gone.

### R07: sources

- Upstream merge: A generated kernel/yosys_config.h replaced the feature definitions, log.cc's backtrace was compiled out because nothing set YOSYS_ENABLE_DLOPEN, help source locations were always recorded, DPI support followed libffi rather than plugins, and the spawn guard changed.
- Reference: kernel, pass, frontend and backend sources are State A's, keeping only Windows-only hunks, and every object gets the version and feature definitions on its command line.

### R08: data directory

- Upstream merge: yosys.cc fell back to a relative share/yosys, so a copied binary lost its data.
- Reference: yosys.cc gets the absolute install data directory and the program prefix.

### R09: install

- Upstream merge: The install stripped nothing.
- Reference: Outside Debug, the install strips debug information from yosys and libyosys and strips yosys-abc and yosys-filterlib fully, as the Makefile's install did.

### R10: binaries

- Upstream merge: yosys-filterlib linked fewer libraries without exported symbols, and yosys-abc exported symbols.
- Reference: yosys-filterlib links like yosys and exports its symbols; yosys-abc exports none.

### R11: yosys-config

- Upstream merge: Two yosys-config scripts carried build-tree paths, the compiler's full path, a short CXXFLAGS without the feature definitions, no -rdynamic and no libraries.
- Reference: One yosys-config script carries the install paths, the compiler's file name, the full CXXFLAGS with any CXXFLAGS given to the configure first, -rdynamic, every library and the sanitizer link flags of a Sanitize build.

### R12: library

- Upstream merge: libyosys was not built by default and took the program prefix in its file name.
- Reference: libyosys.so is built by default except on WASI, keeps its name under a program prefix and has the SONAME libyosys.so; the Python module is linked with the same SONAME.

### R13: install layout

- Upstream merge: The library directory followed GNUInstallDirs, a multiarch directory under /usr.
- Reference: The library installs into lib/<program prefix>yosys under the prefix.

### R14: install layout

- Upstream merge: The CXXRTL runtime README.txt was installed.
- Reference: It is not installed, as before.

### R15: Python tools

- Upstream merge: yosys-smtbmc and yosys-witness sources were changed to configure-time placeholders.
- Reference: Their sources are State A's and the build substitutes the sys.path line as the Makefile's sed did.

### R16: generated sources

- Upstream merge: The gatemate and quicklogic generators wrote into the source tree, failing on a read-only one.
- Reference: Both generators write into the build directory.

### R17: ABC

- Upstream merge: ABC was always compiled with ABC_NO_RLIMIT and ABC_NO_DYNAMIC_LINKING and without -g -O.
- Reference: ABC gets its Makefile's definitions and -g -O, C++17 with GNU extensions, readline and pthreads by feature, and the two definitions only on Windows, OpenBSD and WASI.

### R18: options

- Upstream merge: YOSYS_WITHOUT_LIBFFI stood in for the plugins switch and help source locations had no option.
- Reference: YOSYS_WITHOUT_PLUGINS and YOSYS_ENABLE_HELP_SOURCE match ENABLE_PLUGINS and ENABLE_HELP_SOURCE.

### R19: WASI

- Upstream merge: The WASI build linked an 8 MiB stack with --stack-first by default.
- Reference: The WASI default stack is 1 MiB with -Os ABC and no -fPIC, as the Makefile's wasi configuration.

### R20: Sanitize build

- Upstream merge: The Sanitize build type passed no -g.
- Reference: Sanitize compiles and links with -g, as SANITIZER did.

### R21: docs examples

- Upstream merge: The extensions and stubnets example Makefiles used the install include path of yosys-config.
- Reference: They map the installed data directory to the build tree's share directory.

### R22: packaging

- Upstream merge: A custom build backend raised Requires-Python to 3.10, and the wheel workflow moved to cibuildwheel 3.4.1, skipped CPython 3.8 and 3.9 and dropped its helper script.
- Reference: setup.py, pyproject.toml and pyosys/__init__.py are State A's with the two make calls replaced by CMake, and the wheel workflow, cibw_before_build.sh and _run_cibw_linux.py are State A's with their make steps translated.

### R23: CI

- Upstream merge: The build job used a Release build, which adds NDEBUG, packed no libyosys.so, and the docs test job lost CC=clang.
- Reference: The build job uses the default configuration, packs libyosys.so, and the docs job keeps CC=clang.

### R24: CI

- Upstream merge: The docs jobs lost ENABLE_HELP_SOURCE.
- Reference: They configure YOSYS_ENABLE_HELP_SOURCE=ON.

### R25: CI

- Upstream merge: The WASI job dropped its feature switches and link flags for a Release build.
- Reference: The WASI job passes the original configuration's switches and link flags.

### R26: CI

- Upstream merge: The sanitizer job added detect_container_overflow=0 to ASAN_OPTIONS.
- Reference: ASAN_OPTIONS is halt_on_error=1 again.

### R27: CI

- Upstream merge: tests/Makefile stopped reading Makefile.conf, so the Verific job's test run silently skipped the Verific and functional tests.
- Reference: The Verific job passes ENABLE_VERIFIC=1 ENABLE_FUNCTIONAL_TESTS=1 to the test run.

### R28: WASI

- Upstream merge: The WASI build named its program yosys, without the .wasm extension.
- Reference: Compiled WASI programs carry .wasm, as the Makefile's wasi configuration named them.

### R29: options

- Upstream merge: Without ABC, every pass that runs ABC and its data files were dropped, while abc9_ops stayed in.
- Reference: Without ABC only abc, abc9, abc9_exe, abc9_ops and abc_new are left out, as the Makefile left them out; the synthesis passes and their data stay.

## Rubric grid

- D-CMAKE x outcome: S1-cmake-default-installs
- D-CMAKE x coverage: not applicable. Every product of the default build is covered by the Stage II behaviour groups.
- D-CMAKE x constraints: not applicable. The CMake build's constraints are the red lines, graded on their own.
- D-CMAKE x robustness: F-readonly-outside-build, F-edit-share-file, F-edit-python-tool, F-edit-cellhelp, F-delete-share-product, F-edit-config-template, F-noop-rebuild, F-edit-generator
- D-CMAKE x process: not applicable. The build's process is graded by the native-build gate.
- D-CMAKE x craft: J-hardcoded-host-path
- D-MAKE-GONE x outcome: S1-make-build-absent
- D-MAKE-GONE x coverage: not applicable. The single removal check already enumerates every build file, workflow and recipe.
- D-MAKE-GONE x constraints: not applicable. Removal has no separate constraint beyond the red lines graded on their own.
- D-MAKE-GONE x robustness: not applicable. A removed build has no behaviour to perturb.
- D-MAKE-GONE x process: not applicable. How the files were removed leaves no residue beyond the removal itself.
- D-MAKE-GONE x craft: not applicable. Deleted files carry no craft.
- D-BEHAVIOUR x outcome: not applicable. Behaviour is graded by divergence from State A, not by a separate outcome.
- D-BEHAVIOUR x coverage: S2-suite-frontends, S2-suite-passes, S2-suite-techmap-arch, S2-suite-sim-formal, S2-command-reference, S2-compiled-features, S2-install-layout, S2-binary-abi, S2-consumer-routes, S2-option-matrix
- D-BEHAVIOUR x constraints: J-tests-weakened
- D-BEHAVIOUR x robustness: F-alt-installed-consumer, F-off-readline, F-alt-relocated-share, F-static-consumer, F-off-tcl, F-alt-python-tools, F-datdir-copied-binary, F-install-without-build, F-wasi-default, F-usr-libdir, F-help-source, F-off-plugins, F-off-zlib, F-debug-install, F-alt-noabc-synth, F-version-checkout, F-tools-relocated, F-debug-backtrace, F-debug-banner
- D-BEHAVIOUR x process: not applicable. Behaviour has no process apart from the build that produces it.
- D-BEHAVIOUR x craft: not applicable. Behaviour equal to State A leaves no room for craft.
- D-CI x outcome: not applicable. The CI outcome is graded by its jobs' steps, which are coverage.
- D-CI x coverage: S2-ci-test-build, S2-ci-test-compile, S2-ci-test-sanitizers, S2-ci-extra-builds, S2-ci-docs
- D-CI x constraints: J-ci-failure-masked, J-ci-triggers-changed
- D-CI x robustness: not applicable. Hosted CI services cannot be perturbed from the grading host.
- D-CI x process: J-ci-job-dropped
- D-CI x craft: not applicable. Workflow wording carries no graded craft beyond its behaviour.
- D-PACKAGING x outcome: not applicable. Packaging outcome is graded by divergence from State A's wheel and archive.
- D-PACKAGING x coverage: S2-ci-wheels, S2-ci-source-vendor
- D-PACKAGING x constraints: not applicable. Packaging constraints are State A's metadata, graded as coverage.
- D-PACKAGING x robustness: not applicable. No families perturb packaging beyond the wheel and archive groups.
- D-PACKAGING x process: J-wheels-config-changed
- D-PACKAGING x craft: J-version-duplicated
- R-NO-FORWARD x outcome: not applicable. A red line is an act to avoid, not an outcome.
- R-NO-FORWARD x coverage: not applicable. The forwarding check spans every product the build links.
- R-NO-FORWARD x constraints: S1-no-build-forwarding
- R-NO-FORWARD x robustness: not applicable. Forwarding has no behaviour to perturb.
- R-NO-FORWARD x process: G-native-build
- R-NO-FORWARD x craft: not applicable. A red line carries no craft.
- R-NO-EMBED x outcome: not applicable. A red line is an act to avoid, not an outcome.
- R-NO-EMBED x coverage: not applicable. The embedding check spans every new or changed file.
- R-NO-EMBED x constraints: S1-no-embedded-old-build
- R-NO-EMBED x robustness: not applicable. Embedded files have no behaviour to perturb.
- R-NO-EMBED x process: not applicable. Embedding is graded by the files it leaves, not by a process.
- R-NO-EMBED x craft: not applicable. A red line carries no craft.
- R-NO-VERIFIER x outcome: not applicable. A red line is an act to avoid, not an outcome.
- R-NO-VERIFIER x coverage: not applicable. The awareness check spans every new or changed file.
- R-NO-VERIFIER x constraints: S1-no-verifier-awareness
- R-NO-VERIFIER x robustness: not applicable. Awareness has no behaviour to perturb.
- R-NO-VERIFIER x process: not applicable. Awareness is graded by the files that carry it.
- R-NO-VERIFIER x craft: not applicable. A red line carries no craft.
