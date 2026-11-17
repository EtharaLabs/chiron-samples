This is an incomplete, hand-written Rust port of yyjson's implementation.
It is not yet the implementation built by the root CMake project.

Implemented so far:

- C layouts for allocator, value, document, chunk, and pool structures.
- Fixed-buffer and dynamic allocators, including their allocation and reuse policies.
- Mutable document creation, pool growth, pool size setters, and destruction.
- Immutable/mutable document and value copies in both directions.
- Immutable and mutable equality, including duplicate-key iterator behavior and
  bitwise floating-point comparisons.
- Version and byte-based position lookup.

The crate has no dependencies. Build both experimental artifacts with:

```
cargo build --offline --release --manifest-path rust/Cargo.toml
```

The differential checks require an independently built, original yyjson shared
library. They use its reader/writer as test oracles, never as implementation
dependencies of the Rust crate:

```
python3 migration-tests/foundation.py /path/to/original/libyyjson.so rust/target/release/libyyjson_port.so
```

The checks cover randomized fixed-pool allocation offsets and retained bytes,
dynamic allocator list ordering and retained bytes, document allocation sizes
and release order with injected allocation failures, copy output and metadata,
duplicate-key equality, numeric bit patterns, and position lookup on arbitrary
bytes. These checks are evidence for the implemented subset, not proof of full
API compatibility.

Remaining work before this can replace yyjson:

- Port the reader, incremental reader, writer, number conversions, file APIs,
  JSON Pointer, JSON Patch, and JSON Merge Patch. Currently 19 of the original
  52 exported symbols are implemented.
- Replace the CMake compilation path with Rust compilation and linking; preserve
  options or reject unsupported configurations; preserve install exports,
  package metadata, SONAME, symlinks, and both static and shared consumer usage.
- Eliminate the extra libgcc_s dependency currently present in the experimental
  Cargo-built shared object and verify the exact final export/dependency surface.
- Check reproducible installation from different build directories and prefixes.
- Remove C and other forbidden translation units after all entry points are
  replaced. The original src/yyjson.c is still present and active today.

The public header and existing CMake files have not been changed. The standalone
Rust crate builds with the /opt/graded/bin compiler policy active, but the root
CMake build still needs a C compiler.
