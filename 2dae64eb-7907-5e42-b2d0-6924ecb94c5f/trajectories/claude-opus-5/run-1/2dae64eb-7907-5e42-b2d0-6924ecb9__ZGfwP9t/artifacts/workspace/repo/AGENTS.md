# Repository notes

## What this repository is

yyjson 0.13.0 with its implementation ported from C to Rust. `src/yyjson.h` is
the original header, unchanged; `src/yyjson.c` no longer exists. The
implementation is the `no_std` crate in `rust/`, built by cargo and driven from
`CMakeLists.txt`, which declares `LANGUAGES NONE` so that no compiler is
probed for.

Read `doc/rust-port.md` first: it maps every C section to its Rust module and
records the decisions (layout, allocator contract, unsafe invariants, retained
quirks, build integration).

## Layout

```
src/yyjson.h              the public header - DO NOT MODIFY
rust/Cargo.toml           no dependencies; `eh-personality` feature, default on
rust/build.rs             cdylib link args: -soname libyyjson.so.0, -lc
rust/src/{types,tables,pow10,util,mem,alc,doc,read,incr,write,ptr,file}.rs
cmake/yyjson-config*.in   the package files install(EXPORT) used to generate
test/compare/run.sh       differential test against the original C
test/compare/consumer.sh  checks an installed prefix three ways
test/compare/inputs/      curated JSON inputs
```

## Invariants to preserve

* `src/yyjson.h` must stay byte-identical. The header's `static inline`
  functions are compiled into callers and poke at the structs directly, so
  every `#[repr(C)]` type in `rust/src/types.rs` must keep its exact layout.
* The shared library must export exactly 52 symbols and depend only on
  `libc.so.6`, with `SONAME libyyjson.so.0`. The shared build therefore uses
  `--no-default-features` so that `rust_eh_personality` is absent; the static
  build keeps it so a bare `cc prog.c -lyyjson` resolves.
* No C, C++, Objective-C or assembly translation unit may be added. The test
  drivers are heredocs inside `test/compare/run.sh` for that reason.
* Behaviour must stay bit-identical to the original, including its quirks;
  `doc/rust-port.md` §6 lists the ones that look like defects.

## Commands

```bash
# build / install, both configurations
cmake -S . -B build-shared -DBUILD_SHARED_LIBS=ON  -DCMAKE_INSTALL_PREFIX=/tmp/psh
cmake --build build-shared && cmake --install build-shared
cmake -S . -B build-static -DBUILD_SHARED_LIBS=OFF -DCMAKE_INSTALL_PREFIX=/tmp/pst
cmake --build build-static && cmake --install build-static

# the same under the no-compiler policy
PATH=/opt/graded/bin:$PATH cmake -S . -B build-check -DBUILD_SHARED_LIBS=ON
PATH=/opt/graded/bin:$PATH cmake --build build-check

# differential test against the original C (taken from commit 40440d6)
test/compare/run.sh [extra-input-dir ...]

# installed-prefix checks
test/compare/consumer.sh /tmp/psh shared
test/compare/consumer.sh /tmp/pst static
```

`cargo` runs offline; `CARGO_HOME` is inside the workspace. The crate has no
dependencies, so `--offline --locked` always succeeds.
