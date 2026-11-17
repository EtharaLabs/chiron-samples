# Migrate Yosys from GNU Make to CMake

`/app` holds the Yosys source tree, with its `abc` and `libs/cxxopts` submodules present as plain
directories and no git metadata. It builds with GNU Make: the top-level `Makefile`, the
`Makefile.inc` files it includes, and ABC's own makefiles. Replace that build with a CMake build,
migrate the CI workflows and the Python wheel packaging to it, and keep everything the build
produces behaving exactly as it does now.

There is no network access. Everything the task needs is installed in the image:
CMake 3.28, Ninja 1.11, clang 18 (the default `clang`/`clang++`), gcc 13, clang-14, gcc-11,
the g++-12 headers, bison, flex, Python 3.12 with `pip` and `venv`, GoogleTest, readline, libffi,
zlib, Tcl 8.6, pkg-config, Icarus Verilog, `vcd2fst`, graphviz, git, GNU Make, the WASI SDK 33 at
`/opt/wasi-sdk`, and a wheelhouse at `/opt/wheelhouse` holding setuptools, wheel, packaging,
pybind11 and cxxheaderparser.

## The specification is the current build

The behaviour to keep is that of the current Make build of this tree, configured the way its CI
configures it: in an out-of-tree `build/` directory, `make -f ../Makefile config-clang`, then
`make -f ../Makefile`, `make -f ../Makefile unit-test` and
`make -f ../Makefile install PREFIX=/usr/local`. GNU Make stays installed, so you can build the
original and compare. Your CMake build at its default settings, and at each option listed below,
must behave like the Make build from the same tree with the equivalent settings. That includes:

- every command's and cell's help text, the usage text of every installed program, and the
  output of designs synthesised, simulated and formally checked through the test harness;
- the version banner (`yosys -V`), the features compiled in, and what the binaries print;
- `yosys-config` and its output, in the build directory and installed;
- the installed file list, file contents and modes, which binaries are stripped, and the
  headers and data under `share/`;
- each ELF's dynamic dependencies, SONAME, RUNPATH or RPATH, exported symbol set, and whether
  it carries debug information (a `.debug_info` section) and a symbol table (a `.symtab`
  section);
- the unit test results, the test harness results, and the documentation examples;
- the static library's global symbols, the WebAssembly module, and the Python wheel.

The tree has no git metadata. With `git` installed but no repository, the current build reports
its version as `0.66+post`; built from a `git archive` export with `git` absent from `PATH`, it
reports `0.66`. Paths of the build, install and scratch directories, timings, and the banner
(everywhere except in the version outputs) may differ, and exported symbol names must match apart
from the string lengths in template arguments (`A<n>_c`).

## Commands that must work

Every command below must work offline, as an unprivileged user, in a fresh copy of your tree.
`<N>` is the number of CPUs available (the CPU affinity, capped by the quota in
`/sys/fs/cgroup/cpu.max`), `<dir>` a scratch directory.

1. Default build, unit tests and staged install:
   `cmake -B build . -G Ninja -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++`,
   `cmake --build build -j<N>`, `ctest --test-dir build/tests/unit` with
   `GTEST_OUTPUT=json:<dir>/` (each GoogleTest program under `tests/unit` writes its results
   there; a test whose `status` is `NOTRUN` or whose `result` is `SKIPPED` need not match), and `cmake --install build --prefix /usr/local` with `DESTDIR=<dir>`. The staged
   install is then placed at `/usr/local`, and the programs and `yosys-config` there are used.
2. The CI artifact, from inside `build/`: `tar -cvf build.tar share/ yosys yosys-* libyosys.so`.
3. The test job: the artifact unpacked into `build/` of a copy whose `abc` and `libs/cxxopts`
   directories are empty, then `./build/yosys-config` and
   `make -C tests -j<N> vanilla-test SEED=1`. The cell job: the artifact unpacked at the root of
   such a copy, then `./yosys -p '<script>'` for each `test_cell` script of the `test-cells` job.
   The documentation job: the artifact unpacked into `build/`, then `make -C docs test -j<N>`.
4. The test harness as it is in `/app` now, run with `make -C tests -j<N> vanilla-test` against
   the products of step 1, laid out as the artifact lays them out at the tree root
   (`share/`, `yosys`, `yosys-*`, `libyosys.so`).
5. Consumers of the install: `yosys-config --build plugin.so plugin.cc` and
   `yosys -m ./plugin.so`; a C++ program compiled with
   `clang++ $(yosys-config --cxxflags) ... -L<build> -lyosys $(yosys-config --ldlibs)` and run
   with `LD_LIBRARY_PATH=<build>`; the installed `yosys-witness`, `yosys-smtbmc`, `yosys-abc` and
   `yosys-filterlib`; and `yosys -Q -T -p 'help -dump-cmds-json <file>'` and
   `help -dump-cells-json`.
6. Configure-only option builds,
   `cmake -B <dir> -S <tree> -G Ninja -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ <option>`,
   followed by `<dir>/yosys-config --cxxflags`, `--linkflags`, `--libs`, `--bindir` and
   `--datdir`. Configuring alone must write `yosys-config` (named with the program prefix, if one
   is set) into the build directory, as `make yosys-config` could without building anything.
   Each option and the make variable it stands for:

   | CMake option | Default | Current make variable |
   |---|---|---|
   | `-DYOSYS_WITHOUT_TCL=ON` | `OFF` | `ENABLE_TCL=0` |
   | `-DYOSYS_WITHOUT_READLINE=ON` | `OFF` | `ENABLE_READLINE=0` |
   | `-DYOSYS_WITHOUT_ZLIB=ON` | `OFF` | `ENABLE_ZLIB=0` |
   | `-DYOSYS_WITHOUT_PLUGINS=ON` | `OFF` | `ENABLE_PLUGINS=0` |
   | `-DYOSYS_WITHOUT_ABC=ON` | `OFF` | `ENABLE_ABC=0` |
   | `-DYOSYS_DISABLE_THREADS=ON` | `OFF` | `ENABLE_THREADS=0` |
   | `-DYOSYS_ENABLE_HELP_SOURCE=ON` | `OFF` | `ENABLE_HELP_SOURCE=1` |
   | `-DCMAKE_BUILD_TYPE=Debug` | empty | `ENABLE_DEBUG=1` |
   | `-DCMAKE_INSTALL_PREFIX=/opt/yosys` | `/usr/local` | `PREFIX=/opt/yosys` |
   | `-DYOSYS_PROGRAM_PREFIX=gr-` | empty | `PROGRAM_PREFIX=gr-` |
   | `-DCMAKE_CXX_STANDARD=23` | `20` | `CXXSTD=c++23` |
   | `-DYOSYS_INSTALL_LIBRARY=ON` | `OFF` | `ENABLE_LIBYOSYS=1` |
   | `-DBUILD_SHARED_LIBS=OFF` | `ON` | `ENABLE_LIBYOSYS_STATIC=1` |

7. One full alternate build and install with
   `-DCMAKE_INSTALL_PREFIX=/opt/yosys -DYOSYS_PROGRAM_PREFIX=gr- -DYOSYS_INSTALL_LIBRARY=ON -DYOSYS_WITHOUT_ABC=ON`,
   then `cmake --build build -j<N>` and `cmake --install build` with `DESTDIR=<dir>`.
   The static library: `-DBUILD_SHARED_LIBS=OFF` and `cmake --build <dir> --target libyosys`,
   which must leave `<dir>/libyosys.a`.
8. The `test-compile` matrix entries this machine can run, each with `CC`, `CXX` and `CXXFLAGS`
   set as the job sets them and the matrix compiler first on `PATH` as `clang`/`clang++` or
   `gcc`/`g++`: clang-14 with
   `CXXFLAGS=--gcc-toolchain=/usr -isystem /usr/include/c++/12 -isystem /usr/include/<triplet>/c++/12`,
   where `<triplet>` is `x86_64-linux-gnu` or `aarch64-linux-gnu`, and gcc-11 with
   `CXXFLAGS=-Wp,-D_GLIBCXX_ASSERTIONS`; then
   `cmake -B build -G Ninja -DCMAKE_CXX_STANDARD=20 .` and
   `cmake --build build --target yosys -j<N>`.
9. The sanitizer job, with `ASAN_OPTIONS=halt_on_error=1` and `UBSAN_OPTIONS=halt_on_error=1`:
   `cmake -B build . -G Ninja -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ -DCMAKE_BUILD_TYPE=Sanitize -DSANITIZE=undefined,address`,
   `cmake --build build -j<N>`, then `make -C tests -j<N> vanilla-test SEED=1` in the same tree.
10. The WASI job, with `/opt/wasi-sdk/bin` first on `PATH` and `<flex-prefix>/include` holding
    flex's `FlexLexer.h`, as the job prepares it:
    `cmake -B build -G Ninja -DCMAKE_TOOLCHAIN_FILE=/opt/wasi-sdk/share/cmake/wasi-sdk-p1.cmake -DCMAKE_INSTALL_PREFIX=/ -DYOSYS_WITHOUT_TCL=ON -DYOSYS_WITHOUT_READLINE=ON -DYOSYS_WITHOUT_PLUGINS=ON -DYOSYS_WITHOUT_ZLIB=ON -DCMAKE_CXX_FLAGS=-I<flex-prefix>/include "-DCMAKE_EXE_LINKER_FLAGS=-Wl,-z,stack-size=8388608 -Wl,--stack-first -Wl,--strip-all" .`
    and `cmake --build build -j<N>`; every `build/*.wasm` must match the job's current
    output.
11. The wheel, with `CC=clang`, `CXX=clang++` and pip offline:
    `python3 -m pip wheel --no-index --find-links /opt/wheelhouse --no-deps -w <dir> .`; its name,
    file list, metadata (including the long description), `WHEEL` and `top_level.txt`, packaged
    data (`pyosys/__init__.py` and everything under `pyosys/share/`) and the ELF facts (type,
    needed libraries, soname, runpath, rpath, debug sections, symbol table) of its
    `pyosys/libyosys.so` and `pyosys/yosys-abc`; then the wheel
    installed into a fresh venv, imported, and the tests under `tests/pyosys` run against it.
12. The vendored source package: the tree committed to a scratch repository with `.gitcommit`
    reset to `$Format:%H$`, exported with `git archive`, then built with `git` absent from
    `PATH`: `cmake -B build . -G Ninja -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++` and
    `cmake --build build --target yosys -j<N>`.

These commands and options must also behave like the Make build in these situations: other
designs and inputs; full builds
and installs at the options above (a debug build, Tcl, readline, zlib and plugins switched off,
another install prefix); the installed programs and libraries used by consumers, after the
installation is moved and after the build directory is gone; a binary copied out of the build
directory; a build from a read-only source tree into a separate build directory; a build inside a
git checkout of the tree; the WASI build without the job's linker flags; and a rebuild after files
in the tree or the build directory are edited or deleted, or with nothing changed.

## Rules

- **The Make build is gone.** Remove the top-level `Makefile`, every `Makefile.inc` outside
  `abc/`, and every other piece of the Make build of Yosys and ABC; no `Makefile`, `makefile`,
  `GNUmakefile` or `Makefile.conf` may remain at the root, and no `Makefile.inc` or
  `Makefile.conf` outside `abc/` and `libs/cxxopts/`. No workflow, action, packaging script,
  `flake.nix`, `nix/*.nix` or `Brewfile` may run `make` on the repository root, on `abc/`, or on a
  makefile there. These Make uses may stay: `tests/Makefile`, `tests/common.mk`,
  `tests/sva/Makefile`, `tests/unit/Makefile`, the Sphinx and code-example makefiles under `docs/`,
  `examples/osu035/Makefile`, `examples/smtbmc/Makefile`, the makefiles of `libs/bigint`,
  `libs/ezsat` and `libs/subcircuit`, the files inside `abc/` and `libs/cxxopts/`, workflow steps that run the test or
  documentation harness with `make -C tests` or `make -C docs`, and `make` used to build
  third-party sources.
- **CMake builds the project itself.** The build is the `CMakeLists.txt` files and the CMake
  modules they include. No `make` process may run while configuring, building or installing, ABC
  included: no CMake file, and no script the build runs, may start `make`, `gmake`,
  `${CMAKE_MAKE_PROGRAM}` or `$(MAKE)` through `execute_process`, `add_custom_command`,
  `add_custom_target` or otherwise, except to run the test or documentation harness in `tests/`
  or `docs/`, and `ExternalProject` and `FetchContent` are not used. The `yosys` and `yosys-abc` executables and `libyosys` must be linked
  by CMake's own rules from objects CMake compiles. Do not wrap, replay or call the old build in
  any form.
- **Nothing of the old build is embedded.** Do not add a copy of the Make build, or of anything it
  generated or built (parser and lexer sources, the version source, generated help, generated
  techmap libraries, binaries, objects, archives), under any name or encoding. Generated sources
  are generated while building.
- **Nothing probes its surroundings.** The build, its scripts and the tree must not look for, read
  or adapt to anything on the machine beyond the toolchain and the inputs this statement names, and
  no file you add or change may name a path under `/tests`.
- **The build is portable.** Tools, libraries and include directories are found through CMake's
  discovery, its variables and the install prefix, never through an absolute path of this
  machine (a `#!/usr/bin/env` line is fine).
- **CI keeps its shape.** Every workflow keeps its triggers, its `if` conditions, concurrency,
  permissions and duplicate-run skipping, its jobs, runner labels, matrix entries, `needs`
  edges and the artifacts its `actions/upload-artifact` and `actions/download-artifact` steps
  pass, by name, and every step that drove the Make build runs the CMake build with the
  same meaning. Nothing may hide a failure the original would have reported: no `|| true`,
  `continue-on-error`, `set +e` or ignored exit status the original did not have, no build, test
  or check command left out, and no sanitizer, warning or test option weaker than before. The
  one exception is `vs-prep` in `extra-builds.yml`, which only prepared sources for the Make
  build: it may go, with its `vcxsrc` artifact and the `needs` edge to it. Jobs that cannot run
  here (macOS, Windows, Nix, self-hosted, Verific, CodeQL, cibuildwheel, publishing) keep their
  shape, and those that built the project build it with CMake.
- **The tests keep their strength.** The harness under `tests/` must not lose, skip or loosen
  any test, expected output, reference file or comparison; pointing it at the new build
  directory and program names is expected.
- **Packaging keeps its configuration.** The wheel workflow and its scripts keep their
  cibuildwheel version, build and skip selectors, Python versions, runner matrix, manylinux
  images, the dependencies they prepare before building, and the wheel test command; only the
  commands that build Yosys change. The Python package keeps reading its version from the
  build's own definition of the Yosys version, as it now reads it from the `Makefile`.

## What counts as the tree

`/app` is copied without `.git`, build products and caches:
the root entries `build`, `share`, `yosys`, `yosys-abc`, `yosys-config`, `yosys-filterlib`,
`yosys-smtbmc`, `yosys-witness`, `libyosys.so`, `libyosys.a`, `dist`, `venv`, `.venv`, `.cache`,
`Testing`, `compile_commands.json` and `build.tar`; any `CMakeFiles`, `__pycache__`,
`.pytest_cache`, `.mypy_cache`, `_skbuild` or `*.egg-info` directory; any `*.o`, `*.d`, `*.a`,
`*.so`, `*.so.*`, `*.pyc`, `*.pyo`, `CMakeCache.txt`, `*.ninja`, `.ninja_deps`, `.ninja_log`,
`cmake_install.cmake`, `CTestTestfile.cmake` or `*.whl` file; and test outputs under
`tests/<dir>/`: `*.result`, `*.log`, `*.err` and `*.out` files, `run-test.mk`, and the
`Makefile` of a directory whose `generate_mk.py` writes it. Owners and modes are not kept: directories become `0755`, and a file becomes
`0755` when it starts with `#!` or is executable at the same path in the current tree, `0644`
otherwise.
