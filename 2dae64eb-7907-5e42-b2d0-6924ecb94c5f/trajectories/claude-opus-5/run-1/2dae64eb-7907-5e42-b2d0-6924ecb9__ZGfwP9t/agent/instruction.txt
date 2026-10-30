# Move yyjson's implementation from C to Rust

`/workspace/repo` holds yyjson 0.13.0, a JSON library written in C. Its single implementation
file is `src/yyjson.c`. Your job is to make the library's implementation Rust instead, and to
leave everything a user of the library can observe exactly as it is today.

This is a migration, not a rewrite of the interface. The C API stays. The header stays. The
build command stays. The installed files stay. The behaviour stays, down to the bytes.

## State B: what the library must become

The implementation is Rust, written against `std`, `core` and `alloc` only.

- No crates. No package registry, no git dependency, no vendored third-party source, no
  workspace member that is not your own code. The environment is offline and there is no
  registry to reach.
- The Rust toolchain installed here is the one to build with. `cargo` and `rustc` are on
  `PATH`.
- `src/yyjson.c` and every other C, C++, Objective-C and assembly translation unit leaves the
  tree. The library's machine code comes from Rust.

## What must not change

**The public header.** `src/yyjson.h` must still be there, byte for byte as you found it. It is
the library's contract, and a large part of it is `static inline` functions that read and write
the library's data structures directly. Those functions are compiled into every program that
includes the header, so your Rust types must have the same memory layout the header assumes.
Changing the header is not a migration; it is a different library.

**The build.** The library is configured, built and installed with the same CMake commands that
work today, once for a shared library and once for a static one, with nothing else set:

```
cmake -S . -B <build-dir> -DBUILD_SHARED_LIBS=ON  -DCMAKE_INSTALL_PREFIX=<prefix>
cmake --build <build-dir>
cmake --install <build-dir>
```

and the same three with `-DBUILD_SHARED_LIBS=OFF`. The build must need no flag, variable,
environment setting or manual step beyond those. It must work offline, from a clean build
directory, with no network.

**The installed tree.** After installing, every path the library installs today must exist, with
the same name, the same kind (file or symbolic link) and the same link target: the header, the
shared library and its two version symlinks, the static archive, the CMake package
configuration, and the pkg-config file. A program that finds yyjson through `pkg-config`,
through `find_package(yyjson)` and the `yyjson::yyjson` target, or through a hand-written
`-lyyjson` link line must still compile and link and run against your library, in both the
shared and the static configuration.

**The shared library's surface.** It must carry the same SONAME, export exactly the symbols the
library exports today and no others, and depend on no shared library that yyjson does not
already depend on. No symbol from the Rust runtime may be exported. No dynamic loading, no
preloaded library, no helper process: the parsing and writing happen in the library itself.

**The behaviour.** Every observable result must match the current library exactly: return
values, output bytes, error codes, error message text, error byte positions, and the sequence
of allocator calls with their exact sizes. That includes the behaviour you may be tempted to
call a bug. The current library's quirks are its contract. Read the C before you decide that
something is wrong, and keep what it does.

## The build that is graded has no C compiler

Your workspace has a working C toolchain, because you need it: the original implementation is
still there, and building and running it is the most direct way to learn what your port must
do. Use it freely while it lasts.

The build that produces the graded artifacts is different. It runs with no C, C++,
Objective-C or assembly compiler at all. Any attempt to compile such a translation unit is
recorded and refused, and a build that needs one produces nothing. Linking is not compiling,
and the Rust toolchain links through `cc`, so linking still works.

This has a consequence worth planning for: the build must not ask CMake to enable a compiled
language it cannot get. You can check your build under exactly that policy at any time:

```
PATH=/opt/graded/bin:$PATH cmake -S . -B build-check -DBUILD_SHARED_LIBS=ON
PATH=/opt/graded/bin:$PATH cmake --build build-check
PATH=/opt/graded/bin:$PATH cmake --install build-check
```

If that sequence works from a clean build directory for both library kinds, your build is
compatible with the graded one.

## Write down what you did and why

Leave a note at `doc/rust-port.md` in the repository. It is read by a reviewer who knows the C
library well, is deciding whether your port is sound rather than merely passing, and cannot run
your code.

Cover, as directly as you can:

- the layout you chose for the library's value and document types, and why the header's inline
  functions are still correct against it;
- how a value's type, subtype and length are packed, how an immutable document's values sit in
  memory, and how a mutable container links its children;
- the allocator contract you honour, and what happens when an allocation fails;
- every category of unsafe operation your port performs, and the invariant that makes each one
  sound;
- the behaviours of the original you kept deliberately even though they look like defects;
- how numbers are read and written so that the output matches digit for digit;
- how the incremental reader resumes when input arrives in arbitrary pieces;
- how the build drives the Rust toolchain and still installs the same tree, SONAME and exported
  symbols;
- how you checked your port against the original, naming the comparisons you ran;
- how a Rust panic is prevented from unwinding into a C caller.

Name the module, type or function behind each decision. Claims a reviewer cannot place in your
code are worth little.

## How the work is judged

Three things are looked at, in this order.

1. **The library as shipped.** It builds both ways with the commands above, installs the same
   tree, and carries the same SONAME, exported symbols and library dependencies. If this does
   not hold, nothing else is examined.
2. **Behaviour against the original**, across the whole public API: reading, error reporting,
   numbers, writing, the mutable API, JSON Pointer and Patch, custom allocators, the
   incremental reader, the header's inline functions, and file input and output.
3. **A set of inputs chosen to separate a faithful port from an approximate one**, compared
   against results recorded from the original.

Your note is reviewed alongside them.

You are not told which inputs are used or what the recorded results are. The original
implementation is in front of you; it is the specification.

## Practical notes

- You have one working session. There is no second turn.
- The repository is a git checkout with a single commit. Commit or not, as you prefer; the tree
  on disk is what is collected.
- Build directories, `target/`, object files, archives, shared objects, installed prefixes and
  `.git` are not collected. Do not leave anything the build needs in them, and do not ship a
  prebuilt binary and link it: the graded build builds from source, so a library it cannot
  rebuild is a library that does not exist.
- Do not ship a copy of the original implementation in another form and generate Rust from it
  during the build. Quoting a few lines of the C in your note or in a comment to explain a
  decision is fine and expected.
- `cargo` runs offline. `CARGO_HOME` is set inside the workspace.
- The library's own test suite is not present in this tree. If you want tests, write them, and
  keep them out of the way of the two builds above.
