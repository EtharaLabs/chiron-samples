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
  tree. The library's machine code comes from Rust. A listing assembled through `global_asm!`,
  `naked_asm!` or `asm!` is an assembly translation unit however it is named or carried, so a
  tree holding assembly, a compiler's assembly output, or a compiler of its own
  breaks this rule even if no compiler runs during the build.

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
directory, with no network. Configured, built and installed again from a new build directory to
a new prefix, the same tree must give the same installed tree byte for byte, apart from the
prefix path itself.

A build option this project compiles into the library today must either keep doing what it
does today or make the configure step fail; it may not be accepted and ignored.

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

## The final build has no C compiler

Your workspace has a working C toolchain, and the original implementation is still in the tree.

The build that produces the shipped artifacts is different. It runs with no C, C++,
Objective-C or assembly compiler at all. Any attempt to compile such a translation unit is
refused, and a build that needs one produces nothing. Linking is not compiling,
and the Rust toolchain links through `cc`, so linking still works.

That same policy is present in this image as the compiler drivers installed under
`/opt/graded/bin`.

## Requirements

Three things must hold.

1. **The library as shipped.** It builds both ways with the commands above, installs the same
   tree, and carries the same SONAME, exported symbols and library dependencies.
2. **Behaviour against the original.** Every observable behaviour of every part of the public
   API that `src/yyjson.h` declares, under any input and any combination of flags, must match
   the original library. Nothing the header declares is outside
   this, and a difference anywhere in it is a difference.
3. **The Rust itself.** The requirements below apply to your Rust sources and your build files.

One condition here is absolute. The tree must not carry the original
implementation in another form and turn it into the library during the build.
That covers an encoded, compressed, split, reformatted or
tool-translated copy of the C, a build step or a macro that turns any of those into Rust,
object code or assembly during the build, and a large embedded payload that is run or
reinterpreted as code. A port written by hand is not this, however closely it follows the
original. Lookup tables you transcribe into plain Rust ahead of time are not this. A few lines
of the C quoted in a comment are not this.

Past that condition, ten faults must be absent from the code you ship.

- Memory for the documents, values, strings, write buffers or pools comes from Rust's global
  allocator instead of from the allocator in effect for the call.
- The library writes to the process's standard output or standard error.
- Code compiled into the library carries an explicit panic path: `.unwrap()`, `.expect()`,
  `panic!`, `unreachable!`, `todo!` or `unimplemented!`.
- A type standing for one the header exposes is declared without `#[repr(C)]` or
  `#[repr(transparent)]`, so its layout is not the one the header's inline functions assume.
- The port keeps mutable global state the original does not have: a `static mut`, a static
  holding an `Atomic`, `Mutex`, `RwLock`, `OnceCell`, `OnceLock`, `RefCell` or `UnsafeCell`,
  or a `thread_local!`.
- When memory cannot be obtained the library ends the process instead of returning the failure
  value the original returns.
- The reader or the writer walks nested arrays and objects by recursion, where the original
  walks them with a loop and an explicit stack.
- The reader validates or decodes input bytes through Rust's UTF-8 machinery instead of
  handling the bytes as the original does.
- Library behaviour depends on the process environment: it reads environment variables, or
  opens files the original's code does not open.
- Doubles are written through Rust's formatting machinery instead of your own digit
  generation.

Each of the ten concerns code in your own files; prose, comments and documentation are not
code and are none of them.

Together, the Rust, Cargo, CMake and script files you add or change must come to no more than
2,400,000 bytes, counting every line with its line number in front of it.

The original implementation is in front of you; it is the specification.

## Practical notes

- The repository is a git checkout with a single commit. Commit or not, as you prefer; the tree
  on disk is what is collected.
- Build directories, `target/`, object files, archives, shared objects, installed prefixes and
  `.git` are not collected. Do not leave anything the build needs in them, and do not ship a
  prebuilt binary and link it: the final build builds from source, so a library it cannot
  rebuild is a library that does not exist.
- Do not ship a copy of the original implementation in another form and generate Rust from it
  during the build. Quoting a few lines of the C in a comment to explain a
  decision is fine and expected.
- `cargo` runs offline. `CARGO_HOME` is set inside the workspace.
- The library's own test suite is not present in this tree. If you want tests, write them, and
  keep them out of the way of the two builds above.
