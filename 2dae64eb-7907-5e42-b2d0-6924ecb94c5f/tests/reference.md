<!-- GENERATED SECTION. DO NOT HAND-EDIT. -->
<!-- Derived from solution/grounding.yaml by solution/recompute.py. -->

# Reference answer for the judged design note

The submission is a migration of github.com/ibireme/yyjson at release
0.13.0, commit 6447536015f3d600f3d65323b10976103b337ca7, under the
MIT licence, from C to Rust, with the public header, the C ABI, the
CMake build, the installed tree and every observable behaviour held fixed. The note
under judgement is doc/rust-port.md of the submitted repository, handed to you as the
staged note and the only thing you read about the submission.

If the staged note reads `[not found]`, the submission did not reach the judged layer:
its graded build did not produce the shipped library, or it broke a prohibition, or it
shipped no note. Answer no to every item.

## How to read this answer

This is the migration as the reference performs it, not a form of words. A note may
state the same facts in any order, at any length and in its own terms.

For each item, answer yes when the note states what the item asks for and states it
correctly against this answer. Answer no when the note omits it, restates the item
without the fact, hedges instead of committing, or contradicts this answer. Length,
confidence and fluency are not evidence. A decision the note does not place in a named
Rust module, type or function is a decision this answer does not credit.

Each item is read on its own. A fact stated once in the note counts for every item that
asks for it; an item no part of the note addresses is a no, whatever else the note does
well.

## The port this answer describes

Each step of the reference migration, with what it did, the state it established, and
the mistake that state survives. A note describes the same port.

### S-06

Did: Keep src/yyjson.h byte for byte and derive the Rust layouts from it: repr(C) yyjson_val, yyjson_mut_val, yyjson_doc, yyjson_mut_doc, the string and value pools, and the tag encoding, with compile-time size and offset assertions (src/types.rs, src/hdr.rs).

Established: Every static inline function in the unchanged header reads and writes the port's memory correctly.

Survives: A reordered field or a changed tag layout leaves the header compiling and silently wrong; the header-interop module reads every accessor against State A.

Red line: R-04 (any change to src/yyjson.h)

Held fixed by compiled relation, not by the note:

- P-04: The submitted src/yyjson.h differs from State A's by at least one byte.
- T-14: Every recorded case of module B-09 (header interop) agrees with State A on every recorded metric.

### S-07

Did: Delete src/yyjson.c and leave no other C, C++, Objective-C or assembly translation unit, in any file name, and no verbatim copy of the implementation under another extension.

Established: The library's machine code can only come from Rust.

Survives: Renaming the C file, or embedding it as a resource and generating Rust from it during the build, both reproduce State A without migrating it.

Red line: R-06 (src/yyjson.c or any other State A translation unit left in the tree)

Held fixed by compiled relation, not by the note:

- P-06: src/yyjson.c or another State A translation unit remains in the submitted tree, or the tree carries at least a quarter of State A's implementation as verbatim spans under any name.

### S-08

Did: Create the crate as #![no_std], edition 2024, with no dependency, no alloc crate and no global allocator, and export exactly the 52 entry points as #[unsafe(no_mangle)] pub unsafe extern "C" (src/lib.rs, src/ffi.rs).

Established: The shipped library offers State A's symbol set and nothing else, and resolves no crate from a registry, a git source or a vendored copy.

Survives: Pulling in libc or any convenience crate changes the dependency surface and the import set.

Red line: R-05 (a registry, git or vendored third-party dependency)

Held fixed by compiled relation, not by the note:

- T-02: The shipped shared library exports exactly State A's 52 dynamic symbols.
- P-05: The graded build compiles a crate from a registry, a git source or a vendored third-party copy.

### S-09

Did: Port the reader: trivia and comment skipping, literals, strings, containers, the flag matrix and the in-situ path (src/reader.rs, src/util.rs, src/utf8_seq.rs, src/tables.rs).

Established: Every document and every malformed input produces State A's value tree, return value and consumed length.

Survives: Reader mutants that shift a bound, accept a trailing comma or mis-handle a BOM are separated by the parse-and-error family.

Held fixed by compiled relation, not by the note:

- T-06: Every recorded case of module B-01 (reader) agrees with State A on every recorded metric.
- T-18: Every stage-3 case in family X-01 (parse and error) agrees with State A on every recorded metric.

### S-10

Did: Port the number reader: the digit scanner, the big-integer and diy floating-point slow paths, the two fast paths, and the hex and raw forms (src/read_num.rs).

Established: Every numeric literal yields State A's exact bits, subtype and error.

Survives: Rounding mutants inside diy_fp and the exactness windows of the fast paths are separated by the numeric round-trip family.

Held fixed by compiled relation, not by the note:

- T-08: Every recorded case of module B-03 (numbers) agrees with State A on every recorded metric.
- T-19: Every stage-3 case in family X-02 (numeric round-trip) agrees with State A on every recorded metric.

### S-11

Did: Port the incremental reader so a document split at an arbitrary byte resumes with the same state (src/incr.rs).

Established: Chunked reading returns MORE at the same boundaries and completes to the same document as a single read.

Survives: A resume that re-scans or drops a partial token diverges on inputs split inside a literal, a string escape or a number.

Held fixed by compiled relation, not by the note:

- T-13: Every recorded case of module B-08 (incremental reader) agrees with State A on every recorded metric.

### S-12

Did: Port error reporting: the code, the constant message text and the byte position for every failure path (src/reader.rs, src/util.rs).

Established: Every failure reports State A's code, its exact message bytes and its exact position.

Survives: A message reworded or a position off by one is invisible to a caller that only checks success.

Held fixed by compiled relation, not by the note:

- T-07: Every recorded case of module B-02 (errors) agrees with State A on every recorded metric.

### S-13

Did: Port the writer and the number writer: the escaping tables, minified and pretty layouts, the flag matrix, and digit generation including the fixed and float forms (src/writer.rs, src/write_num.rs).

Established: Serialising any document produces State A's bytes exactly, including its escaping and formatting choices.

Survives: Writer mutants that change an escape table entry, a separator or the fixed-notation cut-over are separated by the escaping and formatting family.

Held fixed by compiled relation, not by the note:

- T-09: Every recorded case of module B-04 (writer) agrees with State A on every recorded metric.
- T-20: Every stage-3 case in family X-03 (writer escaping and formatting) agrees with State A on every recorded metric.

### S-14

Did: Port the mutable document API: the string and value pools, the circular child lists, the copies and the equality relation (src/value.rs).

Established: Building, copying and comparing mutable documents matches State A, including iteration order and duplicate keys.

Survives: List surgery that loses the last-child invariant appears correct until a container is iterated twice.

Held fixed by compiled relation, not by the note:

- T-10: Every recorded case of module B-05 (mutable documents) agrees with State A on every recorded metric.

### S-15

Did: Port JSON Pointer, JSON Patch and JSON Merge Patch, including the context struct and the parent-creating forms (src/pointer.rs, src/patch.rs).

Established: Every pointer and patch operation returns State A's result, error code and error position.

Survives: Pointer mutants that mis-parse an escaped token or an index are separated by the mutation and patch family.

Held fixed by compiled relation, not by the note:

- T-11: Every recorded case of module B-06 (pointer and patch) agrees with State A on every recorded metric.
- T-21: Every stage-3 case in family X-04 (mutation, pointer and patch sequences) agrees with State A on every recorded metric.

### S-16

Did: Honour the allocator contract: the three callbacks, the context pointer, the old size handed to realloc, and the order and size of every request, including the null, default, pool and dynamic allocators (src/alloc.rs).

Established: A caller tracing its allocator sees State A's exact request sequence, and a pool too small to serve it fails where State A fails.

Survives: Allocation-failure injection separates a port that allocates different sizes or in a different order even when its output is identical.

Held fixed by compiled relation, not by the note:

- T-12: Every recorded case of module B-07 (allocators) agrees with State A on every recorded metric.
- T-22: Every stage-3 case in family X-05 (allocation-failure injection) agrees with State A on every recorded metric.

### S-17

Did: Keep file input and output on the same libc stdio calls, with the same error mapping (src/reader.rs, src/writer.rs, src/ffi.rs).

Established: Reading and writing files reports State A's codes and leaves the same bytes on disk.

Survives: Buffering the whole file where State A streams it changes the failure point on a short write.

Held fixed by compiled relation, not by the note:

- T-15: Every recorded case of module B-10 (files) agrees with State A on every recorded metric.

### S-18

Did: Replace CMakeLists.txt: project(... LANGUAGES NONE) so no compiled language is probed, cargo driven through add_custom_command with the source directory as its working directory, the SONAME set by a link argument in .cargo/config.toml, and two hand-written package-config templates in place of the export set CMake would have generated.

Established: The unchanged CMake commands configure, build and install both library kinds with no C compiler present, producing State A's install tree and package configuration.

Survives: Leaving LANGUAGES C in place makes the configure step itself ask for a compiler, which the graded build refuses.

Red line: R-01 (any compile of a C, C++, Objective-C or assembly translation unit by the graded build)

Held fixed by compiled relation, not by the note:

- T-03: The shared library's DT_SONAME equals State A's.
- T-04: The install prefix holds exactly State A's installed paths: the header, the libraries and their links, the CMake export set and the pkg-config file.
- T-16: Every recorded case of module B-12 (consumers) agrees with State A on every recorded metric.
- P-01: The graded build attempts to compile a C, C++, Objective-C or assembly translation unit.

### S-19

Did: Make the build idempotent: reconfiguring, rebuilding with nothing changed and reinstalling leave the installed tree as it was.

Established: The install tree is a function of the source, not of the build directory's history.

Survives: A build that regenerates a timestamped or path-dependent artefact fails a second install without failing the first.

Held fixed by compiled relation, not by the note:

- T-17: Reconfiguring, a no-op rebuild and reinstalling leave the installed tree byte-identical (module B-11).

### S-20

Did: Keep the shipped library's dependency and import surface inside State A's: only libc, and none of the dynamic-loading or process-spawning entry points.

Established: The library parses and writes in its own code, with State A's shared-library dependencies.

Survives: Delegating the work through dlopen or a helper process would pass behavioural comparison while moving the implementation outside the library.

Red line: R-03 (dlopen, LD_PRELOAD or a helper process doing the parsing or writing)

Held fixed by compiled relation, not by the note:

- T-05: Every DT_NEEDED entry of the shared library is in State A's set.
- P-03: The shipped library imports dlopen, dlmopen, dlsym, fork, vfork, execve, posix_spawn, system or popen.

### S-21

Did: Build only from submitted source: ship no object file, archive or shared library for the build to link.

Established: Everything the library is made of is rebuilt from the tree the grading collects.

Survives: A prebuilt archive linked into the library reproduces State A's behaviour with none of its source.

Red line: R-02 (a prebuilt object, archive or shared library shipped in the submission and linked in)

Held fixed by compiled relation, not by the note:

- P-02: The graded build links an object, archive or shared library it did not build from the submitted source.

### S-22

Did: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.

Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

Survives: A port that passes every recorded comparison by accident cannot explain why its unsafe operations are sound.

## The files the reference migration ships

A note names its decisions in these files. A name that is not one of them, or not a
module, type or function inside one of them, places nothing.

- .cargo/config.toml
- CMakeLists.txt
- Cargo.lock
- Cargo.toml
- cmake/yyjson-config-release.cmake.in
- cmake/yyjson-config.cmake.in
- doc/rust-port.md
- src/alloc.rs
- src/ffi.rs
- src/hdr.rs
- src/incr.rs
- src/lib.rs
- src/patch.rs
- src/pointer.rs
- src/read_num.rs
- src/reader.rs
- src/tables.rs
- src/types.rs
- src/utf8_seq.rs
- src/util.rs
- src/value.rs
- src/write_num.rs
- src/writer.rs

## Item by item

### J-01 (weight 5, outcome, VALUE)

What a correct note states: The note gives a Rust layout for yyjson_val, yyjson_mut_val, yyjson_doc and yyjson_mut_doc whose field order, sizes and alignment are the ones the reference states keep every inline function in the unchanged yyjson.h valid.

What this answer states, from the reference note's own account:

Because the header is unchanged, its inline functions dereference these structs
in the consumer's translation unit. The Rust definitions therefore are not a
design choice at all: they are the only layouts that keep those inlines valid.
All four live in `crate::types`, all are `#[repr(C)]`, and every size and every
field offset is pinned by a `const _: () = assert!(...)` in the same file (99
such assertions in `types.rs`), so a layout regression is a compile error, not
a runtime surprise.

#### `yyjson_val`: 16 bytes, align 8

```rust
#[repr(C)]
pub(crate) struct yyjson_val {
    pub tag: u64,              // offset 0
    pub uni: yyjson_val_uni,   // offset 8
}
```

The C `uni` is a union of `uint64_t / int64_t / double / const char * /
void * / size_t`. All six members are 8 bytes and align 8 on LP64, so
`types::yyjson_val_uni` is a `#[repr(C)] union` with exactly those six members
(`u64_`, `i64_`, `f64_`, `str_`, `ptr`, `ofs`; the trailing underscores avoid
Rust keyword and primitive-name collisions and are the only renaming anywhere
in the port). A union rather than a single `u64` cell with casts is the right
choice: it reproduces the C aliasing exactly, so `uni.f64_` reads the same eight
bytes `uni.u64_` wrote, which several reader and writer paths rely on, and it
makes `uni.ofs` (used only by containers) and `uni.str_` (used only by strings
and raws) distinct named fields rather than casts.

Asserted: `size_of::<yyjson_val>() == 16`, `align_of == 8`,
`offset_of!(tag) == 0`, `offset_of!(uni) == 8`, `size_of::<yyjson_val_uni>() == 8`.
These are the numbers `unsafe_yyjson_get_first` (`ctn + 1`),
`unsafe_yyjson_get_next` (`ptr + sizeof(yyjson_val)`) and
`unsafe_yyjson_arr_is_flat` (`len * sizeof(yyjson_val) + sizeof(yyjson_val)`)
compute with, all three of them in the header.

#### `yyjson_mut_val`: 24 bytes, align 8

```rust
#[repr(C)]
pub(crate) struct yyjson_mut_val {
    pub tag: u64,                   // offset 0
    pub uni: yyjson_val_uni,        // offset 8
    pub next: *mut yyjson_mut_val,  // offset 16
}
```

The first two fields are at the same offsets as `yyjson_val`'s, which is not a
coincidence the port may discard: the header's `unsafe_yyjson_get_tag`,
`unsafe_yyjson_get_len`, `unsafe_yyjson_get_str`, `unsafe_yyjson_get_uint` and
friends all take `const void *` and cast to `const yyjson_val *` regardless of
whether the caller handed in an immutable or a mutable value
(yyjson.h:5069-5198). Every `yyjson_mut_*` type predicate in the header goes
through that same `const void *` path. So `yyjson_mut_val` must be a prefix-
compatible extension of `yyjson_val`, and `crate::hdr` mirrors the C by taking
`*mut c_void` in exactly those functions (`hdr::unsafe_yyjson_get_tag`,
`hdr::unsafe_yyjson_get_len`, `hdr::unsafe_yyjson_set_len`,
`hdr::unsafe_yyjson_set_tag`) and `*mut yyjson_val` only where the C signature
is typed (`hdr::unsafe_yyjson_get_first`, `hdr::unsafe_yyjson_get_next`,
`hdr::unsafe_yyjson_arr_is_flat`).

#### `yyjson_doc`: 64 bytes

```rust
#[repr(C)]
pub(crate) struct yyjson_doc {
    pub root: *mut yyjson_val,  //  0
    pub alc: yyjson_alc,        //  8  (32 bytes, by value, not a pointer)
    pub dat_read: usize,        // 40
    pub val_read: usize,        // 48
    pub str_pool: *mut c_char,  // 56
}
```

`alc` is embedded **by value**, a 32-byte copy of the caller's allocator, not a
pointer to it. That matters for correctness, not just layout: the header's
`yyjson_doc_free` (yyjson.h:5347) copies `doc->alc` to a local, `memset`s
`doc->alc` to zero, then calls `alc.free(alc.ctx, doc->str_pool)` and
`alc.free(alc.ctx, doc)`. So the caller's `yyjson_alc` need not outlive the
document, the free order is header-dictated (string pool first, document
second), and the document's own allocation must begin exactly at the
`yyjson_doc *`. `crate::reader`'s `yyjson_read_opts` honours all three.

#### `yyjson_mut_doc`: 120 bytes

```rust
#[repr(C)]
pub(crate) struct yyjson_mut_doc {
    pub root: *mut yyjson_mut_val,  //   0
    pub alc: yyjson_alc,            //   8 (32 bytes)
    pub str_pool: yyjson_str_pool,  //  40 (40 bytes)
    pub val_pool: yyjson_val_pool,  //  80 (40 bytes)
}
```

Both pools are embedded by value, each a 5-field struct
(`cur`, `end`, `chunk_size`, `chunk_size_max`, `chunks`) at offsets 0, 8, 16,
24, 32 within the pool. The header reaches into them: the inline
`unsafe_yyjson_mut_val` (and therefore every `yyjson_mut_null`,
`yyjson_mut_arr`, `yyjson_mut_obj`, `yyjson_mut_strcpy`, ... in the header)
bump-allocates straight out of `doc->val_pool.cur` and only calls into the
library's `unsafe_yyjson_val_pool_grow` when `cur + count > end`. Likewise the
inline `yyjson_mut_strncpy` bumps `doc->str_pool.cur` and falls back to
`unsafe_yyjson_str_pool_grow`. Those two growth functions are among the 52
exported symbols precisely because the header's fast path is inline and only
the slow path is a call. `value.rs` owns both:
`value::unsafe_yyjson_val_pool_grow` and `value::unsafe_yyjson_str_pool_grow`.

Also pinned in `types.rs`, for the same reason: `yyjson_alc` 32 bytes with
`malloc` at 0, `realloc` at 8, `free` at 16, `ctx` at 24;
`yyjson_read_err` 24; `yyjson_write_err` 16; `yyjson_ptr_err` 24;
`yyjson_ptr_ctx` 24; `yyjson_patch_err` 48 (with the nested `yyjson_ptr_err`
at 24); `yyjson_arr_iter` 24; `yyjson_obj_iter` 32; `yyjson_mut_arr_iter` and
`yyjson_mut_obj_iter` 40 each. The iterators matter because the header
constructs and steps them inline; a wrong `pre` offset would corrupt a
caller's loop with no library call involved.

One subtlety in `yyjson_alc`: the three callbacks are declared
`Option<unsafe extern "C" fn(...)>`, not bare `fn` pointers, because
`yyjson_doc_free` zeroes the struct and because a caller can hand in a
partially filled one. Rust's null-pointer optimisation makes
`Option<extern "C" fn(..)>` exactly 8 bytes with no discriminant, and
`types.rs` asserts that:

```rust
const _: () = assert!(size_of::<Option<unsafe extern "C" fn(*mut c_void, usize)
    -> *mut c_void>>() == 8);
```

Without that assertion the nullability would be a layout risk rather than a
free convenience.

Where this answer establishes it:

- S-06: Keep src/yyjson.h byte for byte and derive the Rust layouts from it: repr(C) yyjson_val, yyjson_mut_val, yyjson_doc, yyjson_mut_doc, the string and value pools, and the tag encoding, with compile-time size and offset assertions (src/types.rs, src/hdr.rs).
  Established: Every static inline function in the unchanged header reads and writes the port's memory correctly.

Already decided by compiled relation, so the note's account of it is
judged for correctness and not for whether the behaviour works:

- T-14: Every recorded case of module B-09 (header interop) agrees with State A on every recorded metric.

What only the note can settle: whether the stated layout is the one the header's inline functions require; T-14 compiles the behaviour, not the account

### J-02 (weight 3, outcome, VALUE)

What a correct note states: The note states how the 64-bit tag packs a value's type, subtype and length, as the reference states it.

What this answer states, from the reference note's own account:

One `u64` carries type, subtype and length. The low 8 bits are the tag byte and
the high 56 bits are the length:

```
 63                                    8   7 6 5   4 3   2 1 0
+---------------------------------------+-------+-----+-------+
|              length (56 bits)         | resvd | sub | type  |
+---------------------------------------+-------+-----+-------+
```

- bits 0-2, `YYJSON_TYPE_MASK` 0x07, 3 bits: type. `NONE` 0, `RAW` 1,
  `NULL` 2, `BOOL` 3, `NUM` 4, `STR` 5, `ARR` 6, `OBJ` 7.
- bits 3-4, `YYJSON_SUBTYPE_MASK` 0x18, 2 bits: subtype, already shifted left
  by 3 in the constants. `FALSE`/`NONE`/`UINT` are `0 << 3`,
  `TRUE`/`SINT`/`NOESC` are `1 << 3`, `REAL` is `2 << 3`.
- bits 5-7, `YYJSON_RESERVED_MASK` 0xE0: reserved, always zero in this version.
- bits 8-63: the length, `YYJSON_TAG_BIT == 8`.

What the length means depends on the type: for `STR` and `RAW` it is the byte
length of the string excluding its NUL terminator; for `ARR` it is the element
count; for `OBJ` it is the number of key/value **pairs**, not the number of
`yyjson_mut_val` cells; for `NULL`, `BOOL` and `NUM` it is unused and the C
leaves it 0.

Note that the same numeric subtype value is reused across types, which is why
the predicates compare whole tag bytes rather than masked subtypes:
`YYJSON_SUBTYPE_TRUE`, `YYJSON_SUBTYPE_SINT` and `YYJSON_SUBTYPE_NOESC` are all
`1 << 3`. `types.rs` keeps all three names with their shared value rather than
deduplicating them, because the C code reads according to the type it already
knows.

The port's construction and extraction all route through `crate::hdr`, which
mirrors the header's inlines line for line:

```rust
pub(crate) unsafe fn unsafe_yyjson_get_len(val: *mut c_void) -> usize {
    unsafe { ((*(val as *mut yyjson_val)).tag >> YYJSON_TAG_BIT) as usize }
}

pub(crate) unsafe fn unsafe_yyjson_set_tag(val: *mut c_void,
                                           type_: yyjson_type,
                                           subtype: yyjson_subtype,
                                           len: usize) {
    unsafe {
        let mut tag = (len as u64).wrapping_shl(YYJSON_TAG_BIT as u32);
        tag |= (type_ | subtype) as u64;
        (*(val as *mut yyjson_val)).tag = tag;
    }
}
```

Two deliberate details there. First, `wrapping_shl` not `<<`: the C shift is on
a `uint64_t` and a length with bits in the top 8 positions would simply lose
them; Rust's `<<` would be an overflow panic in a debug build and the port must
not introduce a check the C lacks. Second, `hdr::unsafe_yyjson_set_len` ANDs the
old tag with `YYJSON_TAG_MASK as u64` before OR-ing the new length in, which
preserves type and subtype and clears the old length, exactly as
yyjson.h:5230.

`hdr::unsafe_yyjson_is_ctn` reproduces the header's trick of testing a single
bit: `mask = YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ` is `6 & 7 == 6`, and
`(tag & 6) == 6` is true for exactly `ARR` and `OBJ`. `types.rs` asserts
`YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ == 6` so that the numbering cannot drift out
from under that test. This matters beyond tidiness: `unsafe_yyjson_get_next`
branches on `is_ctn` to decide whether to use `uni.ofs` or
`sizeof(yyjson_val)`, so a wrong container test would walk the arena wrongly.

Where this answer establishes it:

- S-06: Keep src/yyjson.h byte for byte and derive the Rust layouts from it: repr(C) yyjson_val, yyjson_mut_val, yyjson_doc, yyjson_mut_doc, the string and value pools, and the tag encoding, with compile-time size and offset assertions (src/types.rs, src/hdr.rs).
  Established: Every static inline function in the unchanged header reads and writes the port's memory correctly.

Already decided by compiled relation, so the note's account of it is
judged for correctness and not for whether the behaviour works:

- T-14: Every recorded case of module B-09 (header interop) agrees with State A on every recorded metric.

What only the note can settle: whether the stated encoding is correct; T-14 compiles its effect

### J-03 (weight 3, outcome, VALUE)

What a correct note states: The note states how an immutable document stores its values in one contiguous arena and how a container's offset skips its children, as the reference states it.

What this answer states, from the reference note's own account:

An immutable document is two allocations, no more, regardless of how large the
JSON is or how deeply nested.

#### Allocation 1: the input copy, which doubles as the string pool

`reader::yyjson_read_opts` requests `len + YYJSON_PADDING_SIZE` bytes
(`YYJSON_PADDING_SIZE == 4`), copies the caller's `len` bytes in with
`core::ptr::copy_nonoverlapping`, and zeroes the 4 padding bytes with
`core::ptr::write_bytes(eof, 0, YYJSON_PADDING_SIZE)`. That block is `hdr`. The
parser then unescapes and NUL-terminates strings **in place inside this copy**,
so `yyjson_val::uni.str_` for every string and raw value points into it; it is
handed to `doc->str_pool` so that the header's `yyjson_doc_free` releases it.
Under `YYJSON_READ_INSITU` no copy is made at all: `hdr` is the caller's own
buffer, the 4 padding bytes are written into the caller's memory past `len`
(which is why the flag's contract demands the caller provide them), and
`doc->str_pool` is set to `NULL` so `yyjson_doc_free` does not free memory it
does not own. `reader.rs` writes exactly that:

```rust
(*doc).str_pool = if has_rflag(flg, YYJSON_READ_INSITU, false) {
    core::ptr::null_mut()
} else { hdr as *mut c_char };
```

#### Allocation 2: the document header and the value arena, as one block

The second allocation is a single contiguous array of `yyjson_val`, and the
`yyjson_doc` lives in its first few elements. `reader.rs` computes

```rust
hdr_len = size_of::<yyjson_doc>() / size_of::<yyjson_val>()
        + ((size_of::<yyjson_doc>() % size_of::<yyjson_val>()) > 0) as usize;
```

which is `ceil(64 / 16) == 4` here: the document header occupies arena slots
0..3, and the root value is slot `hdr_len`. Hence
`doc = val_hdr as *mut yyjson_doc` and `(*doc).root = val_hdr.add(hdr_len)`:
the `yyjson_doc *` and the arena base are literally the same address, which is
what makes the header's `alc.free(alc.ctx, doc)` correct for the whole arena.
This is not an optimisation the port chose; `yyjson_doc_free` being inline in
the unchanged header makes it mandatory.

Sizing, growth and the two-slot tail guard are all State A's:

- `read_root_single` (a scalar document) asks for exactly `hdr_len + 1` values.
- `read_root_minify` and `read_root_pretty` estimate
  `alc_len = hdr_len + dat_len / YYJSON_READER_ESTIMATED_MINIFY_RATIO + 4`,
  clamped by `yyjson_min(alc_len, alc_max)` with
  `alc_max = USIZE_MAX / size_of::<yyjson_val>()`, then ask for
  `alc_len * size_of::<yyjson_val>()` bytes from `alc.malloc`.
- `val_end = val_hdr.add(alc_len - 2)`, two slots short of the end. The
  comment in the C says "padding for key-value pair reading" and it means it:
  the object loop can place a key and a value without re-checking in between,
  so the guard must leave room for both.
- Growth is geometric at 1.5x: `alc_len = alc_len.wrapping_add(alc_len / 2)`,
  then one `alc.realloc(ctx, val_hdr, alc_old * 16, alc_len * 16)`. Because
  the arena moves, `reader.rs` saves `val_ofs = val.offset_from(val_hdr)` and
  `ctn_ofs = ctn.offset_from(val_hdr)` **before** the realloc and rebuilds both
  cursors from the new base after it. Note that the old size passed to
  `realloc` is `alc_old * size_of::<yyjson_val>()`, in bytes, not in slots.

#### How a container's offset skips its children

`uni.ofs` is used for two different things at two different times in the same
8 bytes, and this is the single most important invariant of the immutable
representation.

**While the container is open**, `uni.ofs` holds the distance **backwards to
the parent container**, so the parser can pop without a separate stack. On
`ST_ARR_BEGIN` / `ST_OBJ_BEGIN`:

```rust
(*val).uni.ofs = (val as *mut u8).offset_from(ctn as *mut u8) as usize;
ctn = val;
```

and on close the parent is recovered by
`ctn_parent = (ctn as *mut u8).sub((*ctn).uni.ofs)`. That is how yyjson parses
arbitrarily deep documents without any auxiliary stack and therefore without
any allocation proportional to depth: the open containers form a chain threaded
through the arena itself. It is also why a depth limit of 0 genuinely means
unlimited rather than meaning "no nesting".

**When the container closes**, the parent offset is no longer needed and the
same field is immediately overwritten with the distance **forwards to the next
sibling**, which is the whole subtree size:

```rust
(*ctn).uni.ofs = ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
    .wrapping_add(size_of::<yyjson_val>());
(*ctn).tag = ((ctn_len as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_ARR as u64;
```

`val` is the last value written inside the container, so `val - ctn + 16` is
exactly the byte span from the container header through its last descendant,
inclusive. The port must do the overwrite in this order because the tag write
on the next line is what finally stamps the container's type and length.

With that invariant in place, traversal is pointer arithmetic with no
indirection, which is what the header's two inlines do and what
`hdr::unsafe_yyjson_get_first` and `hdr::unsafe_yyjson_get_next` reproduce:

```rust
pub(crate) unsafe fn unsafe_yyjson_get_first(ctn: *mut yyjson_val)
    -> *mut yyjson_val { unsafe { ctn.add(1) } }

pub(crate) unsafe fn unsafe_yyjson_get_next(val: *mut yyjson_val)
    -> *mut yyjson_val {
    unsafe {
        let is_ctn = unsafe_yyjson_is_ctn(val as *mut c_void);
        let ctn_ofs = (*val).uni.ofs;
        let ofs = if is_ctn { ctn_ofs } else { size_of::<yyjson_val>() };
        (val as *mut u8).add(ofs) as *mut yyjson_val
    }
}
```

The first child is always the immediately following slot. The next sibling is
16 bytes on for a scalar and `uni.ofs` bytes on for a container, which is
precisely "skip my children". An object's children alternate key, value, key,
value in the arena, and `tag >> 8` counts pairs, so the header's object
iterator steps `unsafe_yyjson_get_next` twice per pair.

`hdr::unsafe_yyjson_arr_is_flat` is the cheap test for "this array contains no
containers": `len * 16 + 16 == ofs` holds exactly when every element occupied
one slot. `writer.rs` uses it to take a tighter loop, and `value.rs` uses it
when copying.

`(*doc).val_read` is computed as
`val.offset_from((*doc).root) as usize + 1`, that is, the number of arena slots
actually consumed, and `(*doc).dat_read` as `cur.offset_from(hdr) as usize`,
the number of input bytes consumed. Both are read directly by header inlines,
so both must be byte-exact; see the J-08 notes on what `dat_read` counts under
`YYJSON_READ_STOP_WHEN_DONE`.

Where this answer establishes it:

- S-22: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.
  Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

What only the note can settle: whether the stated arena scheme is correct

### J-04 (weight 3, outcome, VALUE)

What a correct note states: The note states how mutable containers keep their children in a circular list reached through the last child, as the reference states it.

What this answer states, from the reference note's own account:

A mutable document has no arena ordering to exploit, so children are linked.
The representation is a **circular singly-linked list through
`yyjson_mut_val::next`, whose head pointer `uni.ptr` points at the LAST
child**. Therefore:

- last child: `(*ctn).uni.ptr as *mut yyjson_mut_val`
- first child: `(*(last)).next`
- end of iteration: coming back around to `last`
- `tag >> 8` is the child count for an array, the pair count for an object
- an empty container has `uni.ptr == NULL` and length 0

Pointing at the last child rather than the first is what makes `O(1)` append
possible with one pointer per value: appending is "splice after the current
last, then make me the last", which touches two `next` fields and one
`uni.ptr`. Prepending is just as cheap, because the first child is one hop from
the last: "splice after the current last, leave `uni.ptr` alone". That
asymmetry is visible in the header's `yyjson_mut_arr_append` versus
`yyjson_mut_arr_prepend`, and the port keeps it.

For objects, keys and values share one ring and alternate strictly key, value,
key, value, so a pair is two cells and the ring length is `2 * size`. The
canonical splice is `hdr::unsafe_yyjson_mut_obj_add`, a faithful copy of
yyjson.h:7190:

```rust
pub(crate) unsafe fn unsafe_yyjson_mut_obj_add(obj: *mut yyjson_mut_val,
                                               key: *mut yyjson_mut_val,
                                               val: *mut yyjson_mut_val,
                                               len: usize) {
    unsafe {
        if len != 0 {
            let prev_val = (*((*obj).uni.ptr as *mut yyjson_mut_val)).next;
            let next_key = (*prev_val).next;
            (*prev_val).next = key;
            (*val).next = next_key;
        } else {
            (*val).next = key;
        }
        (*key).next = val;
        (*obj).uni.ptr = key as *mut c_void;
        unsafe_yyjson_set_len(obj as *mut c_void, len.wrapping_add(1));
    }
}
```

Read that against the invariant: for a non-empty object `uni.ptr` is the last
**key**, so `(*last_key).next` is the last **value** (`prev_val`), and
`(*prev_val).next` is the first key (`next_key`). The new key is spliced after
the last value, the new value points at the old first key to re-close the ring,
and `uni.ptr` becomes the new key. For an empty object the single statement
`(*val).next = key` closes a two-cell ring onto itself. The `len` is passed in
by the caller rather than re-read, because the header's wrappers have usually
just read it; `hdr::yyjson_mut_obj_add` is the wrapper that supplies
`unsafe_yyjson_get_len(obj)`.

A scoping point that is easy to get wrong: **most of the mutable container
API is not in the library at all.** `yyjson_mut_arr_prepend`,
`yyjson_mut_arr_insert`, `yyjson_mut_arr_remove`, `_remove_first`,
`_remove_last`, `_remove_range`, `_clear`, `_replace`, `_rotate` and the
object equivalents are all `yyjson_api_inline` in the unchanged header, so
they are compiled into the consumer and the port neither defines them nor
needs to. They are, however, the reason the layout obligations of J-01 are
strict: they manipulate the ring from outside the library, so `uni.ptr`,
`next` and the `tag` length field must mean exactly what the header thinks
they mean.

What `crate::hdr` does provide is the subset `yyjson.c` itself calls, plus
their transitive callees, which is what `CONVENTIONS.md` scopes that module
to: `hdr::unsafe_yyjson_mut_obj_add`, `hdr::yyjson_mut_obj_add`,
`hdr::yyjson_mut_obj_put`, `hdr::yyjson_mut_obj_get`,
`hdr::yyjson_mut_obj_getn`, `hdr::yyjson_mut_obj_size`,
`hdr::yyjson_mut_arr_append`, the iterators
(`hdr::yyjson_mut_arr_iter_init`, `yyjson_mut_arr_iter_next`,
`yyjson_mut_obj_iter_init`, `yyjson_mut_obj_iter_next`,
`yyjson_mut_obj_iter_getn`, `yyjson_mut_obj_iter_remove`), the value and
string constructors (`hdr::unsafe_yyjson_mut_val`,
`hdr::unsafe_yyjson_mut_str_alc`, `hdr::unsafe_yyjson_mut_strncpy`,
`hdr::yyjson_mut_strn`, `hdr::yyjson_mut_strncpy`, `hdr::yyjson_mut_obj`),
`hdr::yyjson_mut_doc_set_root`, `hdr::yyjson_mut_equals`, and the pointer
entry points `hdr::yyjson_mut_ptr_getx` / `_addx` / `_replacex` /
`_removex`.

The iterators carry both `cur` and `pre` because removing during iteration
needs the predecessor and a singly-linked ring cannot find it otherwise;
`types.rs` pins `pre` at offset 24 in both `yyjson_mut_arr_iter` and
`yyjson_mut_obj_iter` for exactly that reason.

Ring traversal for copying lives in `value.rs`:
`value::yyjson_mut_val_mut_copy`, `value::yyjson_val_mut_copy`,
`value::yyjson_doc_mut_copy`, `value::yyjson_mut_doc_mut_copy`,
`value::yyjson_mut_val_imut_copy`, `value::yyjson_mut_doc_imut_copy`. The
imut copies are what flatten a ring back into a contiguous arena with the
`uni.ofs` encoding of the previous section, which makes them the one place
both representations have to be understood at once.

Removal from a ring with only a `next` pointer costs a walk from the first
child to the predecessor, which is why `hdr::yyjson_mut_arr_remove` is O(n) and
why the C offers the iterator form. The port does not add a prev pointer or a
hash index to make any of this faster: both would change `yyjson_mut_val`'s
size, which the header's inline bump allocator computes with.

Where this answer establishes it:

- S-22: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.
  Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

What only the note can settle: whether the stated list scheme is correct

### J-05 (weight 3, outcome, VALUE)

What a correct note states: The note states the yyjson_alc contract the port honours, namely the three callbacks, the context pointer, the old size passed to realloc and the order of requests, as the reference states it.

What this answer states, from the reference note's own account:

`yyjson_alc` is a vtable of three callbacks plus an opaque context, embedded by
value in both document types:

```rust
pub malloc:  Option<unsafe extern "C" fn(ctx: *mut c_void, size: usize)
                 -> *mut c_void>,
pub realloc: Option<unsafe extern "C" fn(ctx: *mut c_void, ptr: *mut c_void,
                                         old_size: usize, size: usize)
                 -> *mut c_void>,
pub free:    Option<unsafe extern "C" fn(ctx: *mut c_void, ptr: *mut c_void)>,
pub ctx:     *mut c_void,
```

The contract the port honours, point by point:

1. **`ctx` is first, always, and is never interpreted.** Every call site passes
   `alc.ctx` as the first argument and the port never dereferences it except
   inside its own three allocators, where it is a `*mut pool_ctx` or
   `*mut dyn_ctx` the port itself created. A caller's `ctx` is opaque.
2. **`realloc` takes the old size.** This is the unusual part of the contract
   and the easiest thing to get wrong, because libc `realloc` does not take
   one. It exists so that a bump or pool allocator can know how much to copy
   without a per-block header. Every `realloc` call in the port passes the
   byte count it previously asked for, in bytes, not in elements. In
   `reader.rs` that is `alc_old * size_of::<yyjson_val>()` where `alc_old` is
   the previous slot count; in `writer.rs` it is the previous buffer byte
   length; in `value.rs`'s pool growth it is the previous chunk size.
   `alloc::default_realloc` deliberately ignores it (`_old_size`) and calls
   `ffi::realloc(ptr, size)`, exactly as State A's wrapper does, because glibc
   tracks the size itself. `alloc::dyn_realloc` also ignores the parameter and
   uses its own `(*chunk).size`, again matching State A.
3. **The three callbacks are never NULL from the library's side.** When a
   caller passes `alc == NULL`, `reader::yyjson_read_opts` and every other
   entry point substitute `alloc::YYJSON_DEFAULT_ALC` by value, so the
   document's embedded copy always has three live pointers. This is why the
   port calls through `alc.malloc.unwrap_unchecked()` rather than matching on
   the `Option`: the `Option` exists for layout and for `yyjson_doc_free`'s
   zeroing, not as a runtime check the C code does not perform, and
   `unwrap_unchecked` compiles to nothing.
4. **The order of requests is fixed and observable.** The differential harness
   compares the full allocator call trace with exact sizes (see J-12), so the
   port's order is not merely similar, it is identical. For a parse that is:
   `malloc(len + 4)` for the input copy, then
   `malloc(alc_len * 16)` for the arena, then zero or more
   `realloc(arena, old_bytes, new_bytes)` at 1.5x growth, and on error
   `free(arena)` before `free(input_copy)`. On success the frees happen later,
   in the header's `yyjson_doc_free`, which frees `str_pool` first and the
   document second. For a write it is one `malloc` of the estimated size and
   then, on some paths, one `realloc` down or up; see J-08 on why there is no
   final shrink. For `yyjson_mut_doc_new` it is `malloc(sizeof(yyjson_mut_doc))`
   and nothing else, with the pools starting empty so the first value request
   triggers the first chunk.

`alloc.rs` provides four allocators, all of them State A's:

- **The null allocator**, `alloc::YYJSON_NULL_ALC`, with `null_malloc` and
  `null_realloc` returning NULL and `null_free` doing nothing. It is a
  placeholder so the function pointers are never NULL; `yyjson_alc_pool_init`
  writes it into the caller's `alc` before validating, so a failed init leaves
  a safe-to-call allocator rather than garbage. Like the C object, which has
  internal linkage, it is a `const` and contributes no symbol.
- **The default allocator**, `alloc::YYJSON_DEFAULT_ALC`, whose
  `default_malloc` / `default_realloc` / `default_free` are thin wrappers over
  `ffi::malloc` / `ffi::realloc` / `ffi::free`. They ignore `ctx`, which is
  NULL in the static.
- **The pool allocator**, `alloc::yyjson_alc_pool_init(alc, buf, size)`, a
  first-fit free list over a caller-supplied buffer. Layout:
  `pool_ctx { size, free_list }` at the aligned start of `buf`, then
  `pool_chunk { size, next }` headers, each chunk's payload beginning at
  `chunk.add(1)`. `pool_size_align` rounds the request up to a multiple of
  `size_of::<pool_chunk>()` (16) and adds one header. `pool_malloc` walks the
  free list, splits a chunk only when the remainder would be at least two
  headers (`(*cur).size >= size + size_of::<pool_chunk>() * 2`), and otherwise
  hands over the whole chunk. `pool_free` reinserts in **address order** and
  coalesces with the higher neighbour first, then the lower, which is what
  keeps the pool from fragmenting under the reader's realloc pattern.
  `pool_realloc` tries to grow in place by absorbing the next free chunk and
  falls back to `pool_malloc` plus `ffi::memcpy` plus `pool_free`. The minimum
  buffer is `size_of::<pool_ctx>() * 4`, that is 64 bytes, below which init
  returns false; `mem_align_up` / `size_align_down` round the base and the
  length. All of it is O(n) in the free list, deliberately: it is a small
  fixed-buffer allocator, not a general one.
- **The dynamic allocator**, `alloc::yyjson_alc_dyn_new` /
  `yyjson_alc_dyn_free`. One `default_malloc` of
  `size_of::<yyjson_alc>() + size_of::<dyn_ctx>()` holds the returned `alc`
  and, immediately after it, a `dyn_ctx` of two dummy `dyn_chunk` list heads
  (`free_list`, `used_list`). `dyn_malloc` rounds up through
  `dyn_size_align` to a multiple of `YYJSON_ALC_DYN_MIN_SIZE`, scans
  `free_list` (kept sorted small to large by `dyn_free`) for the first chunk
  big enough, and otherwise goes to `default_malloc`. `dyn_free` does not
  return memory to libc at all: it moves the chunk from `used_list` to the
  size-sorted `free_list`. Only `yyjson_alc_dyn_free` releases everything, by
  walking `free_list` then `used_list` then freeing the `alc` block itself.
  That retention is the documented point of the allocator and the port keeps
  it; a "fix" that freed eagerly would change the libc call trace.

Two consequences worth stating explicitly. First, the four private callbacks
(`null_*`, `default_*`, `pool_*`, `dyn_*`) are `unsafe extern "C" fn`
definitions even though they are not among the 52 exported symbols, because a
C consumer holds their addresses in a `yyjson_alc` and calls them directly.
They are C ABI entry points without being C ABI symbols, and J-14 treats them
as entry points for that reason. Second, `(*alc).ctx = alc.add(1)` in
`yyjson_alc_dyn_new` means the context is interior to the same allocation as
the `yyjson_alc`, so `yyjson_alc_dyn_free(alc)` can recover it as
`alc.add(1)`; the port must not separate them.

Where this answer establishes it:

- S-16: Honour the allocator contract: the three callbacks, the context pointer, the old size handed to realloc, and the order and size of every request, including the null, default, pool and dynamic allocators (src/alloc.rs).
  Established: A caller tracing its allocator sees State A's exact request sequence, and a pool too small to serve it fails where State A fails.

Already decided by compiled relation, so the note's account of it is
judged for correctness and not for whether the behaviour works:

- T-12: Every recorded case of module B-07 (allocators) agrees with State A on every recorded metric.

What only the note can settle: whether the account of the contract is correct; T-12 compiles the request traces

### J-06 (weight 5, outcome, VALUE)

What a correct note states: For every category of unsafe operation the reference lists, the note gives the invariant that makes the port's use of it sound.

What this answer states, from the reference note's own account:

Everything in this crate is `unsafe` in the Rust sense, because the C it
replaces is: 848 `unsafe` tokens, and all 52 exported functions are
`unsafe extern "C"`. Blanket `unsafe` is not an argument, so the categories
below are enumerated from the code and each is paired with the invariant that
makes it sound. What the port does **not** use is as informative as what it
does: zero `core::mem::transmute`, zero `core::slice::from_raw_parts`, zero
`get_unchecked`. Type punning is done only through `#[repr(C)]` unions and
`to_bits` / `from_bits`, and unchecked indexing only through raw pointers.

**1. Unaligned integer loads and stores.** 12 `ptr::read_unaligned` and 6
`ptr::write_unaligned`, all confined to `util.rs`: `util::byte_load_2`,
`byte_load_3`, `byte_load_4` read `u16`/`u32`; `util::byte_move_2`,
`byte_move_4`, `byte_move_8`, `byte_move_16`, `byte_match_2`, `byte_match_4`
read and write `u16`/`u32`/`u64`. These are the lowering of yyjson.c's
fixed-size `memcpy`-into-a-union idiom. Sound because `read_unaligned` and
`write_unaligned` impose no alignment requirement at all (that is their
contract), and because the caller guarantees the N bytes lie in one allocated
object. The port confines these to one module precisely so that the
in-bounds argument has to be made in one place.

**2. Reading past the logical end of the input, into zero padding.** The
reason category 1 is in bounds near end-of-input. `YYJSON_PADDING_SIZE == 4`.
`reader::yyjson_read_opts` allocates `len + 4` and does
`ptr::write_bytes(eof, 0, YYJSON_PADDING_SIZE)`; `incr::yyjson_incr_new` does
the same for its buffer. So a 2, 3 or 4 byte probe starting at the last real
byte reads at worst 3 zero bytes the port itself wrote, inside the same
allocation. Under `YYJSON_READ_INSITU` the padding is written into the
caller's buffer, which is what the flag's documented contract requires the
caller to provide; this is the one invariant the port cannot check and
inherits verbatim from State A.

**3. Fixed-size non-overlapping copies.** 9 `ptr::copy_nonoverlapping`:
`util::byte_copy_2`, `byte_copy_4`, `byte_copy_8`, `byte_copy_16`,
`byte_load_3`, plus one each in `reader.rs` (the input copy of `len` bytes
into the freshly allocated `hdr`), `writer.rs` and `incr.rs`. Sound because
each is the direct equivalent of a `memcpy(dst, src, N)` whose regions the
C already proves disjoint and in bounds: the input copy's destination is a
fresh allocation of `len + 4` and its source is the caller's `len` bytes,
which cannot overlap.

**4. Raw pointer arithmetic.** `.add`, `.sub`, `.offset` everywhere, and 80
`offset_from` (reader 36, pointer 14, incr 10, read_num 8, writer 6, value 3,
write_num 2, hdr 1). Sound because every such pointer stays inside one
allocated object: the parse buffer, the value arena, a pool chunk, or a
`static` table. `offset_from` additionally requires both operands to share
provenance, which holds in every use: they are a cursor and a base derived
from the same allocation (`cur` and `hdr`, `val` and `val_hdr`, `cur` and
`buf`). Note the realloc discipline of J-03: offsets are taken *before* a
realloc and reapplied to the new base after, never carried across as
pointers.

**5. Dereferencing caller-supplied pointers at the FFI boundary.** All 52
exported functions, for example `value::yyjson_mut_doc_free`,
`value::unsafe_yyjson_equals`, `value::yyjson_locate_pos`,
`alloc::yyjson_alc_pool_init`, `reader::yyjson_read_opts`,
`writer::yyjson_write_opts`. Soundness is a contract, not a proof, and it is
exactly State A's contract: NULL is checked wherever the C checks it
(`alc.is_null()` in `yyjson_alc_pool_init`, `dat.is_null()` and `len == 0` in
`yyjson_read_opts`, `doc.is_null()` in the writers) and nothing more is
checked, because adding a check would change behaviour on inputs where State
A has undefined behaviour and the port must not diverge on defined inputs by
paying for checks the C does not make. The port does not promote these
pointers to references, which is what keeps Rust's aliasing rules out of the
picture entirely.

**6. Casting an allocator's `*mut c_void` to a typed header.**
`alloc::pool_malloc` / `pool_free` / `pool_realloc` cast `ctx_ptr` to
`*mut pool_ctx` and recover a block's header with
`(ptr as *mut pool_chunk).sub(1)`; `alloc::dyn_malloc` / `dyn_realloc` /
`dyn_free` do the same with `dyn_ctx` and `dyn_chunk`. Sound because the port
created both: every payload pointer it ever returns is `chunk.add(1)`, so
`ptr.sub(1)` lands on the header of the same allocation, and the headers are
`#[repr(C)]` with `size_of == 16` (both chunk types), `size_of::<dyn_ctx>()
== 32` and `offset_of!` assertions in `alloc.rs`. Alignment is established
before first use by `util::mem_align_up` in `yyjson_alc_pool_init` and by
`default_malloc` for the dynamic allocator.

**7. `#[repr(C)]` union field reads.** `types::yyjson_val_uni` is read and
written across `value.rs`, `hdr.rs`, `reader.rs`, `read_num.rs`,
`write_num.rs`, `writer.rs`, `pointer.rs`, `patch.rs`, `incr.rs`, plus the
small `v16_uni` / `v32_uni` / `v64_uni` helpers in `util.rs`. Sound because
all six members are exactly 8 bytes and align 8, so every bit pattern is a
valid inhabitant of every member; there is no niche, no padding and no
uninhabited case, which is what makes a union read of a differently written
member well defined rather than merely conventional. Which member is live is
determined by the `tag` type bits, exactly as in the C.

**8. Float and integer bit reinterpretation.** `util::f64_from_bits`,
`f64_to_bits`, `f32_to_bits`, `f64_is_inf`, and 2 `to_bits` plus 2
`from_bits` in `write_num.rs`. These replace the C's
`memcpy(&u, &f, 8)` union trick. Sound because `f64::to_bits` and
`f64::from_bits` are total, defined bit casts that do not canonicalise NaN,
which matters: yyjson round-trips NaN payloads through `uni.u64_` and a
canonicalising conversion would change output.

**9. Defined-but-lossy float narrowing.** `util::f64_to_f32` is
`val as f32`, and there are two `as f64` casts (`hdr.rs`'s
`unsafe_yyjson_u64_to_f64`, one in `read_num.rs`). The C version of
`f64_to_f32` carries a `no_sanitize("undefined")` attribute because
`(float)double` on overflow is C undefined behaviour. Rust's `as` is defined
to saturate to infinity, which on x86-64 and aarch64 is what the hardware
instruction the C compiler emits already does. So the port is strictly better
defined here while producing identical bits on the supported targets, and
`CONVENTIONS.md` records that this equivalence is target-specific.

**10. Calling libc through `extern "C"`.** `ffi.rs` declares exactly the
functions `yyjson.c` calls and no others: `malloc`, `realloc`, `free`,
`memcpy`, `memmove`, `memset`, `memcmp`, `fopen`, `fclose`, `fread`,
`fwrite`, `ferror`, `fseek`, `ftell`, `abort`. Sound because the signatures
match the glibc LP64 ABI, `FILE` is an opaque type the port never
introspects (`size_of::<FILE>() == 0` is asserted in `types.rs` to document
that it is never instantiated, only pointed at), and each call site
establishes libc's preconditions. The one difference visible in the linked
object is that LLVM lowers the equality-only `memcmp` in
`hdr::unsafe_yyjson_equals_strn` to `bcmp`; both are glibc symbols with the
same semantics for an equality test, and `bcmp` is what the port imports
where State A imports `memcmp`.

**11. Calling `Option<fn>` callbacks with `unwrap_unchecked`.** Every
allocator invocation, for example
`(alc.malloc.unwrap_unchecked())(alc.ctx, size)` in `reader.rs` and
`(def.realloc.unwrap_unchecked())(def.ctx, chunk, (*chunk).size, size)` in
`alloc::dyn_realloc`. Sound because the port never calls through an allocator
it has not first established as complete: `YYJSON_DEFAULT_ALC` and
`YYJSON_NULL_ALC` are `const`s with all three fields `Some`, and a
caller-supplied `alc` is either used as given (where State A also calls
through it unchecked) or replaced wholesale by `YYJSON_DEFAULT_ALC` when
NULL. `unwrap_unchecked` rather than `unwrap` is required, not stylistic: it
is what keeps the `Option` a pure layout device with no branch and no panic
path.

**12. `static` table lookups indexed by a byte.** `util::char_is_space`,
`char_is_num`, `char_is_ctn`, `char_is_digit`, `char_is_hex`,
`char_is_id_start` and friends index `tables::char_table1` / `char_table2` /
`char_table3`; `util::hex_load_2` / `hex_load_4` index
`tables::hex_conv_table`. Sound because all four are `[u8; 256]` and the
index is `c as usize` with `c: u8`, so the index is structurally in
`0..256`. These are ordinary array indices, so rustc's bounds check is
present but provably dead and is removed; the port does not reach for
`get_unchecked` to shave it, because the bound is already a compile-time
fact.

**13. Power-of-ten table lookups bounded by exponent range.**
`util::pow10_table_get_sig` and `pow10_table_get_exp` index
`tables::pow10_sig_table`, a `[u64; 1336]`, at `(exp10 -
POW10_SIG_TABLE_MIN_EXP) * 2` and `* 2 + 1`. Sound because
`POW10_SIG_TABLE_MIN_EXP == -343` and `POW10_SIG_TABLE_MAX_EXP == 324`, so
the valid index range is `0 ..= 1335`, and `util.rs` carries
`const _: () = assert!` checks tying the table length to those two
constants. The callers in `read_num.rs` have already clamped `exp10` into
`F64_MIN_DEC_EXP ..= F64_MAX_DEC_EXP` (-324 to 308) before calling.
Similarly `tables::f64_pow10_table` is `[f64; 23]` and is indexed only by
`0 ..= F64_POW10_MAX_EXACT_EXP` (22), and `tables::u64_pow10_table` is
`[u64; 20]` indexed by `0 ..= U64_POW10_MAX_EXACT_EXP` (19).

**14. Flexible-array-member access past a struct header.**
`alloc::yyjson_alc_pool_init` does `ctx.add(1) as *mut pool_chunk`,
`pool_malloc` returns `cur.add(1)`, `yyjson_alc_dyn_new` does
`alc.add(1) as *mut dyn_ctx`, `value.rs`'s pool growth does
`chunk.add(1) as *mut yyjson_mut_val` for `yyjson_val_chunk`, and the string
pool does the same for `yyjson_str_chunk`. These are C flexible array
members (`char mem[]`, `yyjson_mut_val vals[]`), which Rust has no type for.
Sound because the preceding allocation was sized `header + payload` in the
same statement that allocated it, so `.add(1)` lands at the start of the
payload inside the same object. `types.rs` additionally asserts the C's own
stated invariant `size_of::<yyjson_val_chunk>() <= size_of::<yyjson_mut_val>()`,
which is what makes the value pool's `chunk + 1` cursor arithmetic correct.

**15. Self-referential and circular linked-list pointer surgery.**
`alloc::pool_free` splices into an address-sorted free list and coalesces
with both neighbours; `alloc::dyn_chunk_list_add` / `dyn_chunk_list_remove`
maintain a size-sorted free list and a used list; `hdr::unsafe_yyjson_mut_obj_add`
and the whole mutable container family rewrite the circular `next` ring of
J-04; `value.rs` rebuilds rings while copying. Sound because each loop
null-checks before dereferencing (`while !cur.is_null()`), because adjacency
is tested on addresses before a merge
(`(cur as *mut u8).add((*cur).size) == next as *mut u8`), and because the ring
invariant "`uni.ptr` is the last child and `(*last).next` is the first" is
re-established before any read in every mutator. The circularity is the
reason the port never forms a `&mut yyjson_mut_val`: two reachable `&mut` to
the same cell would be instant undefined behaviour, whereas raw pointers
carry no such rule.

**16. In-place mutation of memory the library hands out or is handed.** Two
shapes. First, `YYJSON_READ_INSITU`: the parser unescapes strings and writes
NUL terminators directly into the caller's buffer
(`reader.rs` and `incr.rs` branch on the flag in a dozen places). Sound
because unescaping is never expanding (`\n` is two bytes in, one out;
`\uXXXX` is six in, at most three out; a surrogate pair is twelve in, four
out), so every write lands at or before the position its source byte came
from and therefore inside the caller's `len` bytes. Second, the deferred raw
NUL: when a number is read as a raw string, the terminator must be written
*after* the following token has been examined, so `read_num_raw` records the
position in a `pre` out-parameter and the byte is written one token later.
`incr.rs` funnels this through its `raw_ptr` field with a one-byte dummy
`raw_end` target for the very first deferred write, so the first token has
somewhere harmless to write to.

**17. All-zero POD stack temporaries.** 11 uses of
`core::mem::MaybeUninit::zeroed().assume_init()`: `patch.rs` 6, `value.rs` 2,
`pointer.rs` 2, `hdr.rs` 1, standing in for C declarations like
`yyjson_mut_obj_iter iter = { 0 }`. Sound because every target is a
`#[repr(C)]` plain-old-data struct of integers, raw pointers and unions, for
which all-zero is a valid inhabitant (`usize` 0, `*mut T` null, union all
zero bits). There is no `bool`, no reference, no enum with a niche, and no
`NonNull` in any of them, so no invalid-value rule is being skirted. Note
the deliberate restriction: `MaybeUninit` is used only in the
`zeroed().assume_init()` form and never to read genuinely uninitialised
memory.

**18. Address arithmetic for alignment, with provenance preserved.**
`util::mem_align_up` computes the aligned address as a `usize` and then
returns `mem.cast::<u8>().wrapping_add(aligned.wrapping_sub(addr))`;
`alloc::yyjson_alc_pool_init` computes the alignment delta as
`(ctx as usize).wrapping_sub(buf as usize)`. Sound because the arithmetic is
done with `wrapping_*` on integers, matching the C's non-UB wrapping on
`uintptr_t`, while the returned pointer is produced by offsetting the
*original* pointer rather than by casting an integer back to a pointer. That
is the distinction that keeps provenance intact and keeps the result usable
under strict provenance rules, not merely under the current compiler.

Two structural choices support all of the above. There is no
`core::slice::from_raw_parts` anywhere, so no `&[T]` or `&mut [T]` is ever
created from a raw pointer and Rust's slice validity and aliasing rules never
apply to parser memory. And no `&mut` to any yyjson struct is ever formed:
the port works in raw pointers end to end, which is what lets a circular
container ring and an arena with interior pointers exist at all.

Where this answer establishes it:

- S-22: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.
  Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

What only the note can settle: soundness arguments, which no check over the binary or its outputs decides

### J-07 (weight 3, robustness, VALUE)

What a correct note states: The note states how an allocation failure at any request returns State A's error result instead of aborting, as the reference states it.

What this answer states, from the reference note's own account:

There is no infallible allocation anywhere in the crate. There is no global
allocator and no `alloc` crate, so there is no `handle_alloc_error` and nothing
that can abort on OOM: every byte comes from a `yyjson_alc` callback that
returns a possibly-NULL `*mut c_void`, and every call site tests it. The
failure paths, by subsystem:

**Reader.** `reader::yyjson_read_opts` has two allocations before parsing
begins. If `malloc(len + YYJSON_PADDING_SIZE)` returns NULL it reports
`YYJSON_READ_ERROR_MEMORY_ALLOCATION` with `MSG_MALLOC` ("memory allocation
failed") at `pos` 0 and returns NULL. The preceding overflow guard
`len >= USIZE_MAX - YYJSON_PADDING_SIZE` reports the same code and message,
also at `pos` 0, without calling the allocator at all. Inside
`read_root_single`, `read_root_minify` and `read_root_pretty`, a failed arena
`malloc` or a failed 1.5x `realloc` takes the `fail_alloc` path, which reports
`YYJSON_READ_ERROR_MEMORY_ALLOCATION` / `MSG_MALLOC` at the **current cursor
position** `cur.offset_from(hdr)`, not at 0: the position tells the caller how
far parsing had got. The shared cleanup in those three functions frees the
arena if it was allocated (`if !val_hdr.is_null()`), and the `return_err!`
macro in `yyjson_read_opts` then frees the input copy `hdr`, but only when
`YYJSON_READ_INSITU` is off, because under in-situ the buffer is the caller's.
So on a mid-parse allocation failure the allocator sees `free(arena)` then
`free(input_copy)`: arena first, input second, which is the reverse of the
allocation order and the opposite order from the success path's
`yyjson_doc_free` (string pool then document). Both orders are State A's and
both are compared in the harness.

A failed realloc does **not** lose the old arena: `reader.rs` assigns the
result to a temporary `val_tmp` and only overwrites `val_hdr` when it is
non-NULL, so the cleanup path still has a valid pointer to free. Writing
`val_hdr = realloc(...)` directly would have leaked on failure, and that is a
real difference in the observable allocator trace, not just a leak.

**Mutable documents.** `value::yyjson_mut_doc_new` returns NULL if the single
`malloc(size_of::<yyjson_mut_doc>())` fails. Thereafter the header's inline
`unsafe_yyjson_mut_val` bump-allocates from `val_pool` and calls
`value::unsafe_yyjson_val_pool_grow` only when the pool is exhausted; that
function returns `false` on allocator failure, the header's inline returns
NULL, and every caller in the header and in the port checks for NULL. The
same holds for `value::unsafe_yyjson_str_pool_grow` and string copies. Because
a mutable document never frees individual values, a failure part-way through
building leaves a consistent document with fewer values, and the caller's
`yyjson_mut_doc_free` releases every chunk; nothing is left dangling. Chunk
growth doubles `chunk_size` up to `chunk_size_max`, and a request larger than
the next chunk size gets its own chunk, so a huge single string cannot wedge
the pool.

**Writer.** `writer.rs` allocates one estimated buffer per write
(`yyjson_write_opts`, `yyjson_mut_write_opts`, `yyjson_val_write_opts`,
`yyjson_mut_val_write_opts` and the `_buf` / `_fp` / `_file` variants). A
failed `malloc` or a failed growth `realloc` sets
`YYJSON_WRITE_ERROR_MEMORY_ALLOCATION` with `MSG_MALLOC`, writes `0` through
the `len` out-parameter when one was supplied, frees whatever had been
allocated, and returns NULL. The returned length is zero on every error path,
not a partial length, which matters because a caller that checks only the
pointer and then trusts `len` must not see a stale value.

**Pointer and patch.** `pointer.rs`'s put and replace operations can need new
values; a failure there yields `YYJSON_PTR_ERR_MEMORY_ALLOCATION` in the
`yyjson_ptr_err` out-parameter and a false or NULL result, and the
`yyjson_ptr_ctx` out-parameter is left in the state State A leaves it.
`patch.rs`'s `yyjson_patch`, `yyjson_mut_patch`, `yyjson_merge_patch` and
`yyjson_mut_merge_patch` report `YYJSON_PATCH_ERROR_MEMORY_ALLOCATION` with
the offending operation's `idx`, and when the failure happened inside a
pointer operation they also fill the nested `ptr` sub-error, which is why
`yyjson_patch_err` embeds a `yyjson_ptr_err` at offset 24.

**The allocators themselves.** `pool_malloc` returns NULL when the request is
`>= (*ctx).size` or when no chunk fits; `pool_realloc` returns NULL rather
than freeing the old block, so the caller's pointer stays valid; `dyn_malloc`
and `dyn_realloc` return NULL when `dyn_size_align` detects overflow or when
the underlying `default_malloc` fails. `dyn_realloc` is careful to re-add the
chunk to `used_list` even when the realloc failed, so the chunk is still
tracked and `yyjson_alc_dyn_free` will release it. That is why its tail reads
`dyn_chunk_list_add(&raw mut (*ctx).used_list, chunk)` before deciding what to
return.

The null allocator is the ergonomic consequence of all this: because
`YYJSON_NULL_ALC` makes every request fail, it is a legitimate way for a caller
to force every one of these paths, and the harness's allocation-failure
injection family (see J-12) uses a counting allocator built the same way to
fail the Nth request and compare the resulting error code, message, position
and free sequence against State A.

Where this answer establishes it:

- S-16: Honour the allocator contract: the three callbacks, the context pointer, the old size handed to realloc, and the order and size of every request, including the null, default, pool and dynamic allocators (src/alloc.rs).
  Established: A caller tracing its allocator sees State A's exact request sequence, and a pool too small to serve it fails where State A fails.

Already decided by compiled relation, so the note's account of it is
judged for correctness and not for whether the behaviour works:

- T-22: Every stage-3 case in family X-05 (allocation-failure injection) agrees with State A on every recorded metric.

What only the note can settle: whether the account of failure handling is correct; T-22 compiles the behaviour

### J-08 (weight 5, coverage, VALUE)

What a correct note states: Every State A behaviour the reference lists as easy to mistake for a defect is named in the note as kept on purpose.

What this answer states, from the reference note's own account:

These are the places where State A does something a reimplementer would
plausibly call a bug and "fix". Each is load-bearing: a caller, a test suite,
or another part of the library depends on it, and the migration contract makes
every one of them observable. They are grouped by subsystem and numbered for
reference.

#### Reader: error codes, messages and positions

**1. `len == 0` is `INVALID_PARAMETER`, not `EMPTY_CONTENT`.**
`reader::yyjson_read_opts` returns code 1 with "input length is 0" for a
zero length, while whitespace-only input returns code 3
`YYJSON_READ_ERROR_EMPTY_CONTENT` with "input data is empty". Two spellings
of "empty" that look like they should be merged are two distinct public codes
that callers switch on. A NULL `dat` is also code 1, with the different
message "input data is NULL".

**2. Several error positions are forced to 0 even though the cursor moved.**
The `EMPTY_CONTENT` path reports `pos` 0 after having skipped whitespace and
comments; the initial `malloc` failure reports `pos` 0. Looks like a missing
`cur - hdr`. It is what State A does and the harness compares `pos` on every
error.

**3. A truncation at end of input replaces the specific error and moves the
position to EOF.** `reader::is_truncated_end` is consulted by the
`return_err!` paths of `read_root_single`, `read_root_minify` and
`read_root_pretty`. When the real failure (a bad literal `tru`, an incomplete
`\u12`, a lone high surrogate, a partial UTF-8 sequence) sits at the buffer
end, the reported code becomes `YYJSON_READ_ERROR_UNEXPECTED_END` with
`MSG_NOT_END` and `pos` becomes `eof - hdr`, discarding both the more
specific code and the token-start position. This looks like it throws away
better diagnostics. It is the mechanism that lets a streaming caller
distinguish "need more bytes" from "genuinely malformed", and the incremental
reader is built directly on top of it.

**4. `read_root_*` is chosen by peeking two bytes past the first character.**
`reader::yyjson_read_opts` calls `read_root_pretty` only when
`char_is_space(*cur.add(1)) && char_is_space(*cur.add(2))`, else
`read_root_minify` for a container root, else `read_root_single`. Reading
`cur + 2` on a one-byte document looks like an overread. It is in bounds only
because of the 4-byte zero padding, and all three functions are semantically
equivalent, so the heuristic is a pure performance choice whose misprediction
costs nothing.

**5. `YYJSON_READ_STOP_WHEN_DONE` skips the trailing-content check
entirely.** With the flag, `123trailing` parses successfully, returns the
value, and sets `doc->dat_read` to the bytes consumed. Looks like accepting
invalid input. It is the documented "parse one value and tell me where you
stopped" contract, and `dat_read` is the resume cursor the caller needs.

**6. A BOM without `YYJSON_READ_ALLOW_BOM` is an error whose message is
rewritten after the fact.** `reader::yyjson_read_opts` runs the parse, and
only afterwards, if the parse failed at `pos == 0` and not with an allocation
error, replaces the message with `MSG_ERR_BOM`, `MSG_ERR_UTF16` or
`MSG_ERR_UTF32` depending on which BOM `util::is_utf8_bom` /
`is_utf16_bom` / `is_utf32_bom` recognises, and with `MSG_ERR_UTF8` for
generic non-UTF-8. Post-hoc message mutation looks like a code smell; it is
how State A gives a specific diagnostic without adding error codes.

**7. A leading-zero error points at the zero, not at the offending digit.**
`read_num.rs` reports "number with leading zero is not allowed" at the
position of the `0` in `012`, which reads like an off-by-one. The position
deliberately marks the start of the bad token, not the character that proved
it bad. The same convention governs the other number errors.

**8. An unterminated `/*` comment surfaces as two different errors depending
on where it is.** `reader::skip_trivia` sets the cursor to `eof` and returns
false; depending on the call site that becomes
`YYJSON_READ_ERROR_INVALID_COMMENT` with `MSG_COMMENT` or, through
`is_truncated_end`, `YYJSON_READ_ERROR_UNEXPECTED_END`. Note also that
`YYJSON_READ_ERROR_INVALID_COMMENT` is marked deprecated in the header and is
still produced; the port produces it.

**9. Comment contents are not UTF-8 validated.** `skip_trivia` scans for the
terminator and nothing else, so invalid bytes inside a comment are accepted
even when the same bytes in a string would be rejected. Adding validation
there is the obvious "fix" and would change which documents parse.

**10. `\u0000` produces a real embedded NUL.** `reader::read_uni_esc` writes
byte `0x00` into the string. The result is a value where
`strlen(yyjson_get_str(val)) != yyjson_get_len(val)`. This looks like a
truncation bug. U+0000 is legal JSON, `tag >> 8` is the authoritative length,
and the NUL terminator the reader also writes is a convenience for C callers,
not the length.

**11. Strings are NUL-terminated by overwriting the input, and raw numbers
have their terminator deferred.** The closing quote of a string becomes its
NUL. A raw number has no delimiter to steal, so `read_num::read_num_raw`
cannot terminate it until the following token has been inspected; it records
the position in the `pre` out-parameter and the NUL is written one token
later (`**pre = b'\0'` at the end of `read_root_*`). A reimplementer who
writes the NUL eagerly corrupts the next token. This is why `read_num`'s
signature carries `pre: *mut *mut u8`.

#### Numbers

**12. The default build does not call `strtod`, and the port must not
either.** `yyjson.c` has two complete floating-point readers behind
`#if !YYJSON_DISABLE_FAST_FP_CONV`. The default build uses the fast one; the
`#else` branch, which calls `strtod` twice with an in-place decimal-point
swap to defeat locale, is dead in State A's default configuration. Reaching
for `strtod` in the port would be the natural shortcut and would give
different results on ties and different behaviour under `fesetround`.

**13. Integer `-0` prints as `0`, but real `-0.0` keeps its sign.**
`read_num.rs`'s zero path stores a SINT with value 0, and `write_num.rs`
computes the sign as `is_sint && (i64) 0 < 0`, which is false, so `-0` round
trips to `0`. The double `-0.0` keeps its minus, and the writer documents
this as intentional ("Keep the negative sign of `-0.0` to preserve input
information"). The asymmetry looks like a bug on one side or the other.

**14. `INT64_MIN` is preserved by a strict `>` comparison.** For
`-9223372036854775808` the accumulated magnitude is exactly `2^63`, and the
overflow test is `sign && sig > 2^63`, which is false at equality, so the
value is returned as a SINT. Changing `>` to `>=`, the "safer" form, silently
turns `INT64_MIN` into a double.

**15. The u64 and i64 boundaries are exact and asymmetric.**
`18446744073709551615` is a UINT; `18446744073709551616` falls through to the
double path. `+9223372036854775808` is a UINT, not an overflow. The 19 and 20
digit cases are separate code paths in `read_num.rs` with a
`sig < U64_MAX / 10` test, and the cutover must be preserved digit for digit.

**16. `1e999` has three different outcomes depending on flags.** With no
flags it is an error, "number is infinity when parsed as double"; with
`YYJSON_READ_ALLOW_INF_AND_NAN` it is an f64 infinity; with
`YYJSON_READ_BIGNUM_AS_RAW` it is a raw string carrying the original text.
Collapsing these to a single behaviour is the natural simplification.

**17. `.5`, `5.`, a leading `+`, and hex are rejected unless
`YYJSON_READ_ALLOW_EXT_NUMBER` is set, and `5.` keeps its trailing dot.**
Without the flag these report "no digit after decimal point" or "no digit
after sign". With the flag, `read_num` breaks out of the fraction loop
leaving the dot in the raw text, which matters under
`YYJSON_READ_NUMBER_AS_RAW`. Hex goes through `read_num::read_num_hex`, which
accepts at most 16 hex digits and errors on overflow unless
`YYJSON_READ_BIGNUM_AS_RAW` turns it into a raw string.

**18. `inf` and `nan` spellings are case-insensitive and include the short
forms.** `reader::read_inf_or_nan` accepts `inf`, `Infinity`, `nan` and `NaN`
in any case mixture; a leading `+` only with `ALLOW_EXT_NUMBER`.
`is_truncated_end` even has a dedicated case for `infin`, which is `inf`
followed by `in`, so that a chunk boundary inside `Infinity` is reported as
truncation rather than as garbage. Tightening to exact-case `Infinity` is the
obvious cleanup and breaks real inputs.

**19. Raw numbers keep the original text byte for byte.** Under
`YYJSON_READ_NUMBER_AS_RAW` or `YYJSON_READ_BIGNUM_AS_RAW`, `1E+07`,
`0.10` and `1e999` are preserved exactly as written, pointing into the input
buffer. Normalising them (to `1e7`, `0.1`) is the obvious improvement and
destroys the entire point of the flag.

#### Writer

**20. The exponent never carries a `+`.** `write_num::write_f64_exp` and
`write_f32_exp` do

```rust
byte_copy_2(buf, b"e-");
buf = buf.add(2 - (exp >= 0) as usize);
```

that is, they write `"e-"` and then back the cursor up by one when the
exponent is non-negative, so the `-` is overwritten by the first digit. The
output is `1e100` and `1e-100`, never `1e+100`. This directly contradicts
ECMAScript `Number.prototype.toString`, which a reimplementer is likely to
reach for, and State A documents the deviation explicitly: "Remove positive
sign in the exponent part". It also means a port that formats through any
standard shortest-double routine will be wrong on every number with a
positive exponent.

**21. Integral reals print a trailing `.0`, except in scientific form.**
`write_num::write_f64_raw` emits `0.0` for zero and `1.0` for an integral
double, because State A documents "Keep the decimal point to indicate that
the number is floating-point". But the scientific branch removes it, so
`2e34` is printed rather than `2.0e34`, and the fixed-notation path
`write_f64_raw_fixed` trims trailing zeros and a dangling dot. Three
different rules that look inconsistent and are each deliberate.

**22. Exponent digit count differs between f64 and f32 and is not padded.**
`write_f64_exp` emits two digits below 100 (with the leading zero suppressed,
so `1e-7` not `1e-07`) and three digits at 100 and above; `write_f32_exp`
emits one or two. Zero padding to a fixed width is the obvious tidy-up.

**23. `/` and DEL (0x7F) are not escaped by default.** `writer.rs`'s
`enc_table_cpy` has `0` (copy verbatim) at index 0x2F and `0` across the
whole 0x70..0x7F row. Only `"`, `\` and the C0 controls are escaped.
`YYJSON_WRITE_ESCAPE_SLASHES` switches to `enc_table_cpy_slash`. Many JSON
writers escape `/` and DEL by default; State A does not, and the tables are
generated data the port copied element by element.

**24. Invalid UTF-8 has three distinct outcomes, and the replacement
character is assembled from the hex table.** Without
`YYJSON_WRITE_ALLOW_INVALID_UNICODE` the whole write fails with
`YYJSON_WRITE_ERROR_INVALID_STRING`. With it, the copy path emits the
offending byte verbatim and the escape path emits U+FFFD as an escape built
by indexing the hex digit table, which is what makes
`YYJSON_WRITE_LOWERCASE_HEX` produce `\ufffd` rather than `\uFFFD`.
Hard-coding the uppercase spelling is the natural simplification and silently
breaks the lowercase-hex flag.

**25. The `NOESC` subtype is trusted on output and never revalidated.**
`reader::read_str` sets `YYJSON_SUBTYPE_NOESC` only for a double-quoted
string that contained no escapes and no invalid bytes.
`writer.rs`'s value dispatch then uses `write_str_noesc`, which copies the
bytes with no escaping and no UTF-8 check at all. This looks like a missing
validation. It is a deliberate parse-time-to-write-time optimisation, and it
means the subtype is a correctness obligation: setting it too eagerly
produces output with unescaped control bytes. The port sets and clears it at
exactly State A's points.

**26. The output buffer is over-allocated by a worst-case factor and never
shrunk.** The single-value writer sizes the buffer at `str_len * 6 + 2 +
end_len`, the worst case where every byte becomes a `\uXXXX` escape, and does
not realloc down before returning. The container writer grows by
`max(alc_len / 2, ext_len)`. So the returned pointer is backed by a much
larger block than the reported length. Adding a shrinking realloc is an
obvious memory win and changes the observable allocator trace.

**27. The returned length excludes the NUL, but a NUL is always written.**
Every writer writes a terminating `\0` past the reported `dat_len`, so the
result is usable as a C string, but `dat_len` is the JSON length. With
`YYJSON_WRITE_NEWLINE_AT_END` the `\n` goes *before* that NUL and *is*
counted.

**28. Empty containers print `[]` and `{}` even in pretty mode.** Not `[\n]`
or `{\n}`. The pretty writer special-cases a zero-length container, writing
the indent, then two bytes, then `,\n`. Looks like the pretty path was
forgotten for empty containers.

**28a. The bracket characters are computed by a bit trick, not chosen by a
branch.** `writer.rs` writes `*cur = b'[' | ((ctn_obj as u8) << 5)` and
`*cur = b']' | ((ctn_obj as u8) << 5)`, relying on `'[' | 0x20 == '{'` and
`']' | 0x20 == '}'` in ASCII. Replacing it with an `if ctn_obj { b'{' } else
{ b'[' }` is the obvious readability fix and is behaviour-preserving only as
long as `ctn_obj` is exactly 0 or 1, which is why the port keeps the `as u8`
cast of a `bool` rather than widening the type.

**28b. Nested values in pretty mode suppress their indent via another
arithmetic trick.** `writer.rs` computes
`no_indent = ((ctn_obj as u8) & (ctn_len as u8)) != 0`, which is true exactly
for an object's *value* position (object context, odd position in the
key-value stream), because that value follows a `": "` on the same line and
must not be indented again. The expression truncates `ctn_len` to `u8` to
test its low bit, which looks like a bug when `ctn_len` exceeds 255 and is
not one, because only bit 0 is consulted.

**29. A RAW value is copied into the output with zero validation.**
`writer.rs`'s `YYJSON_TYPE_RAW` case calls `write_raw`, which is a `memcpy`.
A raw value containing arbitrary bytes, including unbalanced quotes, is
emitted as-is and produces invalid JSON. Validating raw on write is the
obvious hardening and breaks the documented "I know what I am doing" escape
hatch.

**30. Pretty indentation is 4 spaces by default and 2 with
`YYJSON_WRITE_PRETTY_TWO_SPACES`, and the separator is `": "` with the space
after the colon only in pretty mode.** Minified output uses `":"` and `","`
with no spaces anywhere. These are per-byte observable.

#### Mutable API

**31. `yyjson_mut_obj_add*` does not check for duplicate keys.** The header
documents this as deliberate ("allows duplicate keys"), and it is what makes
the add an O(1) splice. `yyjson_mut_obj_remove` then removes **all** matching
keys, while `yyjson_mut_obj_get` returns the first. Deduplicating on insert
is the single most tempting "fix" in the mutable API and changes both the
complexity and the semantics.

**32. Mutable values are never individually freed, and `set_root` does not
free the old root.** Everything comes from the document's value and string
pools and is reclaimed only by `yyjson_mut_doc_free`. Replacing the root
leaks nothing in State A's model but does look like a leak, and adding a
per-value free would be unsound because pool memory is not individually
owned.

**33. Object equality is knowingly inaccurate under duplicate keys.**
`value::unsafe_yyjson_equals`'s object case matches each left-hand key
against the first right-hand occurrence, which can give the wrong answer when
keys repeat. The C carries a comment saying exactly this. It is accepted, not
a defect to repair.

**34. Pool chunks double with a cap and a deliberate overflow fallback.**
`value::unsafe_yyjson_str_pool_grow` and `unsafe_yyjson_val_pool_grow`
compute `chunk_size = min(chunk_size * 2, chunk_size_max)`, and the
multiplication is allowed to wrap: the following `if (size < chunk_size)`
test catches the wrap and snaps to `chunk_size_max`. Replacing this with a
checked multiply changes the chunk sizes the allocator sees.

**35. The value pool's cursor arithmetic depends on the chunk header being
exactly one value wide.** The C writes
`(yyjson_mut_val *)(void *)((u8 *)chunk) + 1`, which reads like an operator
precedence bug but is correct because `sizeof(yyjson_val_chunk)` is padded to
`sizeof(yyjson_mut_val)`. `types.rs` asserts
`size_of::<yyjson_val_chunk>() <= size_of::<yyjson_mut_val>()` to pin the
invariant the expression relies on.

**36. Iterators are invalidated by mutation, except through the iterator's
own remove.** The mutable iterators cache `max` at init and carry `pre`, so
`yyjson_mut_arr_iter_remove` is safe mid-loop while a direct
`yyjson_mut_arr_remove` is not. Making the iterators revalidate is the
obvious safety improvement and would change their size, which the header's
inline code computes with.

#### Allocator

**37. `doc->alc` is a by-value copy, and `yyjson_doc_free` zeroes it before
using its own local copy.** A caller may free or mutate its `yyjson_alc`
after the document is created. Storing a pointer instead is the natural
space saving and is wrong.

**38. A NULL allocator argument silently falls back to libc.** Every entry
point does `if alc_ptr.is_null() { YYJSON_DEFAULT_ALC } else { *alc_ptr }`.
No error, no diagnostic.

**39. The dynamic allocator never returns memory to libc until it is
destroyed.** `alloc::dyn_free` moves a chunk from `used_list` to a size-sorted
`free_list`; only `yyjson_alc_dyn_free` calls libc `free`. Under a watchful
leak checker this looks like a leak for the allocator's whole lifetime. It is
the documented design.

**40. `yyjson_alc_pool_init` writes the null allocator into the caller's
struct before validating.** So a failed init leaves an allocator whose
callbacks are safe to call and always fail, rather than leaving the caller's
struct untouched or garbage. It also silently rounds the base up and the
length down, and rejects any buffer smaller than `sizeof(pool_ctx) * 4`.

**41. The pool allocator's free list is address-sorted and coalesces the
higher neighbour before the lower.** `alloc::pool_free` does both merges in
that order. The order is not observable in the pool's own output but it is
observable in which subsequent requests succeed, which the allocation-failure
family of the harness exercises.

#### Pointer and patch

**42. JSON Pointer escape decoding is order-sensitive.** `pointer.rs`'s token
comparison decodes `~0` to `~` and `~1` to `/` scanning left to right, so
`~01` decodes to the literal two characters `~1` and **not** to `/`.
Swapping the two rules, or decoding `~0` after `~1`, corrupts every key
containing a tilde. RFC 6901 specifies exactly this order.

**43. `-` as an array index means "append" and therefore can never be
resolved by a get.** `pointer.rs` maps the `-` token to `USIZE_MAX`, which
then fails every bounds check, so `/arr/-` works for add and fails for get.
Leading-zero indices such as `/arr/01` are rejected as syntax errors while
`/arr/0` is fine. The empty pointer `""` selects the whole document.

**44. JSON Patch `test` considers `1` and `1.0` unequal.**
`value::unsafe_yyjson_num_equals` compares subtypes first: equal subtypes
compare the raw `u64` bits, SINT against UINT compares only when the signed
side is non-negative, and REAL against either integer subtype is always
false. So a `test` of `1.0` against a document containing `1` fails.
Numerically this looks wrong. RFC 6902's `test` is a structural comparison
and callers depend on the distinction. The same function makes a negative
SINT never equal to a large UINT, which looks like a missing case and is
exact.

**45. Merge patch treats `null` as removal and a non-object patch as total
replacement.** `patch.rs`'s `yyjson_merge_patch` and `yyjson_mut_merge_patch`
skip (that is, delete) any member whose patch value is JSON null, and return
a copy of the patch, discarding the original entirely, when the patch is not
an object. Both look like dropped inputs; both are RFC 7386.

#### API edges

**46. NULL and wrong-type arguments return silent defaults.** The header's
accessors return 0, `NULL`, `false` or 0.0 rather than signalling:
`yyjson_get_len` on a non-string returns 0, `yyjson_get_str` on a number
returns `NULL`, `yyjson_equals` with a NULL argument returns whether both are
NULL. These are header inlines, so the port does not implement them, but
`value::unsafe_yyjson_equals` and the exported functions must match the
convention at the boundary.

**47. `null` and boolean equality compares the whole tag byte, so subtype
matters.** `true` and `false` are both `YYJSON_TYPE_BOOL` and are
distinguished only by the subtype bit, so equality cannot mask the subtype
off. This is why `value::unsafe_yyjson_equals` compares tags rather than
types for those cases.

**48. `yyjson_read_number` returns the end pointer and does not reject
trailing content.** `reader::yyjson_read_number` parses one number from a
NUL-terminated string and returns a pointer to the first byte it did not
consume, so `"123abc"` succeeds and returns a pointer at `abc`. It is the
caller's job to check that the returned pointer is at the NUL. Returning NULL
on trailing content is the natural "stricter" reading of the function and
would break every caller that uses it as a scanner.

**49. `yyjson_read_number` ignores its `alc` parameter.** The signature takes
`const yyjson_alc *alc`, and in State A's default build nothing in the
function allocates: the parameter is only used by the
`YYJSON_DISABLE_FAST_FP_CONV` branch, which is dead here. The port binds it
as `_alc` and makes no call, which is why a `yyjson_read_number` call appears
in the harness's allocator trace with zero allocator activity. Inventing an
allocation to "use" the parameter would be a visible divergence.

**50. `yyjson_read_number`'s error position is clamped at the low end.** Its
`return_err!` computes `pos` as `p - hdr` only when `p > hdr`, and reports 0
otherwise. The guard looks redundant, since the cursor cannot precede the
header; it is State A's code and costs nothing to keep.

Where this answer establishes it:

- S-22: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.
  Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

What only the note can settle: recognising which behaviours were kept deliberately

### J-09 (weight 3, outcome, VALUE)

What a correct note states: The note states how numbers are read and written so that output matches State A digit for digit, as the reference states it.

What this answer states, from the reference note's own account:

No part of this is delegated to `core`. `#![no_std]` with no `alloc` means
there is no `format!`, no `Display for f64`, and no float parser available
even if the port wanted one, which is fortunate, because Rust's shortest-
round-trip formatter and State A's differ on exactly the points item 20 and
item 21 of J-08 describe. The port reimplements both directions from the C.

#### Reading

`read_num.rs` owns the whole reader. The entry point is the fixed
cross-module signature

```rust
pub(crate) unsafe fn read_num(ptr: *mut *mut u8, pre: *mut *mut u8,
                              flg: yyjson_read_flag, val: *mut yyjson_val,
                              msg: *mut *const c_char) -> bool;
```

with the bulk of the state machine in `read_num::read_num_tail`. The C's
`digi_*` goto labels become a `read_num::St` enum (`DigiSepr`, `DigiFrac`,
`DigiIntgMore`, `DigiFracMore`, `DigiFracEnd`, `DigiExpMore`,
`DigiExpFinish`, `DigiFinish`) dispatched by a `loop { match state }`, which
preserves the exact control flow including which error message and position
each path reports. The error, zero, integer and double returns are
function-local `macro_rules!` (`return_err`, `return_0`, `return_i64`,
`return_f64`, `return_f64_bin`, `return_raw`) because the C macros capture
`sign`, `cur` and `hdr` from the enclosing scope.

**Integer path.** Digits are accumulated by an unrolled 1 to 18 step
`sig = num + sig * 10`, mirroring the C's `expr_intg` repeat macro. A
19-digit run that is followed by a non-number character is an integer; a
negative 19-digit value greater than `2^63` overflows (to a raw string under
`YYJSON_READ_BIGNUM_AS_RAW`, otherwise to a double via
`hdr::unsafe_yyjson_u64_to_f64`); the 20th digit path tests
`sig < U64_MAX / 10` before the final multiply. Subtype selection:
non-negative values are stored as `YYJSON_SUBTYPE_UINT` with `uni.u64_`,
negatives as `YYJSON_SUBTYPE_SINT` with the two's-complement bit pattern in
`uni.i64_`. `U64_SAFE_DIG` is 19 and `util.rs` carries it as a named
constant rather than a literal.

**Floating-point path, three tiers, in State A's order.**

1. *Exact tier.* When `sig < 2^53` and `|exp10| <= F64_POW10_MAX_EXACT_EXP`
   (22), one native `f64` multiply or divide against
   `tables::f64_pow10_table` (`[f64; 23]`) is provably exact, so the result
   is correct by a single hardware operation. This is the only place the port
   performs ordinary `f64` arithmetic, and it is where State A relies on
   `YYJSON_DOUBLE_MATH_CORRECT`.
2. *Fast tier (Eisel-Lemire style).* With no cut digits and an exponent in
   the normal range, `10^exp10` is fetched as a 64-bit significand plus a
   binary exponent from `tables::pow10_sig_table` via
   `util::pow10_table_get_sig` and `pow10_table_get_exp`, normalised, and
   multiplied by the parsed significand with a 64x64 to 128 bit product
   (`util::u128_mul`). The product's bits above position 54 decide the
   result; when they are ambiguous a second `u128_mul` against the table's
   extended significand resolves it, and the rounding bit is applied before
   the IEEE fields are assembled and returned through `return_f64_bin`.
   `pow10_sig_table` is `[u64; 1336]`, which is
   `(POW10_SIG_TABLE_MAX_EXP - POW10_SIG_TABLE_MIN_EXP + 1) * 2` with
   `POW10_SIG_TABLE_MIN_EXP == -343` and `POW10_SIG_TABLE_MAX_EXP == 324`:
   two `u64` per exponent, significand then extension.
3. *Exact fallback.* When the fast tier cannot decide, State A follows
   Google's double-conversion: a `read_num::diy_fp`
   (`#[repr(C)] { sig: u64, exp: i32, pad: i32 }`, 16 bytes) approximation
   with an error budget (`ERR_ULP` 8, `ERR_CACHED_POW` and `ERR_MUL_FIXED` 4)
   via exactly three functions, `read_num::diy_fp_get_cached_pow10`,
   `diy_fp_mul` and `diy_fp_to_ieee_raw` (normalisation is inlined into the
   latter two, as in the C), and if the candidate is still
   within half an ULP of the boundary, an exact big-integer comparison.
   `read_num::bigint` is `#[repr(C)] { used: u32, bits: [u64; 64] }`.
   `bigint_set_buf` builds the exact decimal from the (possibly truncated)
   digit string, rounding up at `>= '5'`, capping at `F64_MAX_DEC_DIG + 1`
   (769), and forcing the final digit odd so the subsequent comparison cannot
   land exactly on a tie by accident. `bigint_set_u64`, `bigint_add_u64`,
   `bigint_mul_u64`, `bigint_mul_pow2`, `bigint_mul_pow10` (which steps by
   `U64_POW10_MAX_EXACT_EXP` 19 using `tables::u64_pow10_table`) and
   `bigint_cmp` then decide round-down, round-up or round-to-even.

The C's `__int128` becomes Rust's `u128` in `util::u128_mul` and
`util::u128_mul_add`, which is the `YYJSON_HAS_INT128` branch;
`util::u64_lz_bits` and `u64_tz_bits` are `leading_zeros` and
`trailing_zeros`, the `__builtin_clzll` / `__builtin_ctzll` branch. All three
tiers are in `read_num.rs`, all three exist in State A, and all three are
reachable, which is why the numeric comparison in J-12 ran 83,726,941 checks
rather than a sample.

Raw-number reading is `read_num::read_num_raw`, which validates the syntax
without converting and emits a `YYJSON_TYPE_RAW` value pointing into the
input; `read_num::read_num_hex` handles `0x` under
`YYJSON_READ_ALLOW_EXT_NUMBER`, accepting at most 16 hex digits through
`tables::hex_conv_table` with its `0xF0` sentinel.

#### Writing

`write_num.rs` owns the writer; the cross-module entry point is

```rust
pub(crate) unsafe fn write_num(cur: *mut u8, val: *mut yyjson_val,
                               flg: yyjson_write_flag) -> *mut u8;
```

**Integers.** `write_num::digit_table` is the `[u8; 200]` table of the
two-character decimal pairs `"00"` through `"99"`, written two bytes at a
time by `util::byte_copy_2`. The ladder is
`write_num::write_u32_len_8`, `write_u32_len_4`, `write_u32_len_1_to_8`,
`write_u32_len_5_to_8` and `write_num::write_u64`, which dispatches on 1 to
8, 9 to 16 and 17 to 20 digit groups. The division by powers of ten is done
with State A's magic-constant multiply-shift sequences, not with `/`, because
that is what determines the exact code path and therefore, on the boundary
cases, nothing observable but is still what the port must reproduce to keep
the output identical for every input.

**Doubles.** The shortest-round-trip conversion is Schubfach, named in the
source (`write_num.rs:811`, "Raffaello Giulietti, The Schubfach way to
render doubles, 2022"). The pieces:

- `write_num::f64_bin_to_dec` and `f32_bin_to_dec` produce the shortest
  decimal significand and exponent, using `util::u64_round_to_odd` and
  `u128_round_to_odd` against `tables::pow10_sig_table`.
- `write_num::f64_bin_to_dec_fast` produces a non-shortest decimal for the
  fixed-notation path.
- `write_num::write_f64_raw` and `write_f32_raw` (each documented as
  requiring a 40-byte buffer) decode the IEEE fields, route inf and NaN to
  `write_inf_or_nan`, special-case zero to `"0.0"`, take a small-integer fast
  path that appends `".0"`, and otherwise format the Schubfach result.
- `write_num::write_f64_raw_fixed` implements
  `YYJSON_WRITE_FP_TO_FIXED(prec)`, dividing by powers of ten with
  `div_pow10` and `div_pow10_table`, then trimming trailing zeros while
  keeping one digit after the dot.
- Trailing-zero trimming uses `write_num::dec_trailing_zero_table`
  (`[u8; 100]`) with `write_u32_len_7_to_9_trim`,
  `write_u64_len_16_to_17_trim`, `write_u64_len_1_to_16` and
  `write_u64_len_1_to_17`.
- `write_num::write_f64_exp` (range `e-324` to `e308`) and `write_f32_exp`
  (`e-45` to `e38`) format the exponent, with the no-plus trick of J-08 item
  20 and the digit-count rule of item 22.
- The decimal-versus-scientific decision is on `dot_ofs = sig_len +
  exp_dec`: fixed notation when `-6 < dot_ofs <= 21`, scientific otherwise,
  with an inline `1e21` cutoff expressed as a comparison on the shifted raw
  bits (`raw << 1 < 0x444B1AE4D6E2EF50 << 1`) rather than on the value, so
  the test is integral and cannot be perturbed by rounding.

`write_num::write_num` dispatches on the tag: integers to `write_u64` with a
sign, reals by the format bits in the high byte of the write flag
(`YYJSON_WRITE_FP_FLAG_BITS` 8, `YYJSON_WRITE_FP_PREC_BITS` 4,
`YYJSON_WRITE_FP_TO_FLOAT`) to `write_f64_raw`, `write_f64_raw_fixed` or
`write_f32_raw`. The exported `write_num::yyjson_write_number` wraps it.

The `f32` path deserves a note: `YYJSON_WRITE_FP_TO_FLOAT` narrows through
`util::f64_to_f32`, which is `val as f32`. In C that conversion is undefined
on overflow and State A silences the sanitiser for it; in Rust it is defined
to saturate to infinity, which is the same instruction result on the
supported targets. `write_f32_raw` then formats the narrowed value with the
`f32` shortest algorithm and the `f32` exponent range, so
`YYJSON_WRITE_FP_TO_FLOAT` does not merely round a double, it changes the
digit count.

The only `core` float facilities used anywhere are `f64::to_bits`,
`f64::from_bits`, `f32::to_bits` (bit reinterpretation, in `util.rs` and
`write_num.rs`) and the `as` casts. No `core` formatting, no `core` math
methods such as `powi`, `log10`, `round` or `floor`, because none of them is
an exact equivalent of what the C computes.

Where this answer establishes it:

- S-10: Port the number reader: the digit scanner, the big-integer and diy floating-point slow paths, the two fast paths, and the hex and raw forms (src/read_num.rs).
  Established: Every numeric literal yields State A's exact bits, subtype and error.

Already decided by compiled relation, so the note's account of it is
judged for correctness and not for whether the behaviour works:

- T-08: Every recorded case of module B-03 (numbers) agrees with State A on every recorded metric.
- T-19: Every stage-3 case in family X-02 (numeric round-trip) agrees with State A on every recorded metric.

What only the note can settle: whether the account is correct; T-08 and T-19 compile the output

### J-10 (weight 3, outcome, VALUE)

What a correct note states: The note states how the incremental reader resumes across arbitrary chunk boundaries, as the reference states it.

What this answer states, from the reference note's own account:

Three exported functions, matching yyjson.h:1111-1139:
`incr::yyjson_incr_new(buf, buf_len, flg, alc)`,
`incr::yyjson_incr_read(state, len, err)` and
`incr::yyjson_incr_free(state)`. The contract is that the caller owns one
buffer of `buf_len` bytes and calls `yyjson_incr_read` with a growing `len`;
each call either finishes the document (returns a `yyjson_doc *`), asks for
more (returns NULL with `err->code == YYJSON_READ_ERROR_MORE` and the
message "need more data"), or fails hard.

The important design point: **the parser does not re-parse the buffer from
the beginning on each call.** It saves and restores a complete parser state.
`incr::yyjson_incr_state` is `#[repr(C)]`, 160 bytes, with offsets pinned by
the 18 `const _: () = assert!` checks in `incr.rs`:

| field | offset | meaning |
|---|---|---|
| `label: u32` | 0 | which resume point to jump to |
| `alc: yyjson_alc` | 8 | the allocator copy |
| `flg: yyjson_read_flag` | 40 | sanitised read flags |
| `hdr: *mut u8` | 48 | buffer header, owned copy unless in-situ |
| `cur: *mut u8` | 56 | saved parse cursor |
| `buf_len: usize` | 64 | total buffer length, excluding padding |
| `hdr_len: usize` | 72 | document header value count |
| `alc_len: usize` | 80 | values allocated in the arena |
| `ctn_len: usize` | 88 | elements so far in the current container |
| `val_hdr`, `val_end`, `val`, `ctn` | 96, 104, 112, 120 | the value arena and the container cursor |
| `str_con: [*mut u8; 2]` | 128 | partial-string state for `read_str_con` |
| `raw_ptr: *mut u8` | 144 | the deferred raw NUL position |
| `raw_end: [u8; 1]` | 152 | harmless target for the first deferred NUL |

The type is opaque in `yyjson.h` (a forward declaration only), so the layout
is the port's to choose, and the port chose State A's so the sizes the
allocator sees match: `yyjson_incr_new` asks for exactly
`size_of::<yyjson_incr_state>()` bytes.

`yyjson_incr_new` strips the flags incremental parsing cannot honour
(`YYJSON_READ_JSON5`, `YYJSON_READ_ALLOW_BOM`,
`YYJSON_READ_ALLOW_INVALID_UNICODE`), allocates the state, and unless
`YYJSON_READ_INSITU` allocates a private buffer of
`buf_len + YYJSON_PADDING_SIZE` and copies the caller's bytes in. Either way
it writes `YYJSON_PADDING_SIZE` zero bytes after `buf_len`, which is what
makes the multi-byte probes of J-06 category 1 in bounds for every chunk
boundary including the last.

**The resume mechanism.** `yyjson_incr_read` restores every cursor from the
state and enters a single `'fsm: loop { match lbl }` whose arms are the
`I_DOC_BEGIN .. I_DOC_END` constants, the direct translation of the C's goto
labels. At every stable stopping point a function-local
`save_incr_state!(LABEL_x)` macro stores `label`, `cur`, `val`, `ctn_len`,
`hdr_len` and `raw_ptr` back into the state, and if the cursor has reached
the end of the supplied prefix it takes the `goto_unexpected_end!` path.

`goto_unexpected_end!` is the hinge. It sets `err->pos = len`. If
`len >= buf_len` there is nothing more to come, so this is a real
`YYJSON_READ_ERROR_UNEXPECTED_END` with `MSG_NOT_END`. Otherwise it
additionally saves `val_end`, `ctn` and `alc_len`, sets
`YYJSON_READ_ERROR_MORE`, restores the input byte it had temporarily
overwritten, and returns NULL. The caller raises `len` and calls again;
parsing resumes at the saved `cur` with the saved arena and container stack.

**The temporary terminator.** Before parsing a prefix, the reader writes a
NUL at `hdr + len` and remembers the displaced byte, so that the ordinary
token readers (`reader::read_str`, `read_true`, `read_false`, `read_null`,
`read_num`) stop naturally at the chunk boundary with no awareness that they
are in an incremental parse. Every exit path restores the byte. That is what
lets `incr.rs` reuse `reader.rs`'s token readers unchanged instead of
maintaining a second set.

**Partial tokens, case by case.**

- *A split literal* (`tru` at the boundary): `read_true` fails at the NUL,
  and the error goes through `reader::is_truncated_end`, which recognises a
  boundary-truncated literal and reroutes to `goto_unexpected_end!`, hence
  `MORE` rather than `LITERAL`. This is J-08 item 3 being used for its
  designed purpose.
- *A split string or a split escape*: `reader::read_str_con` is the
  continuation-capable string reader, and its progress lives in the state's
  `str_con: [*mut u8; 2]`. On resume it continues the same string, including
  mid-`\uXXXX` and mid-surrogate-pair.
- *A split number*: a number has no terminator, so digits already consumed
  may or may not be the whole number. The `I_ARR_VAL_MAYBE_END` and
  `I_OBJ_VAL_MAYBE_END` states, plus the root case, run
  `check_maybe_truncated_number!`: if the cursor reached the end of the
  prefix the number is assumed incomplete and `MORE` is requested, so the
  next call re-reads it from its first digit with more bytes available. A
  guard on `INCR_NUM_MAX_LEN` turns a pathologically long run of digits into
  an error rather than an unbounded loop.
- *A split comment*: `skip_trivia` reaching the prefix end is the same
  `MORE` path.

**The container stack across calls.** The arena pointers (`val_hdr`,
`val_end`, `val`) and the container cursor (`ctn`) are state fields, and the
container stack is the same `uni.ofs`-threaded chain described in J-03, so it
lives in the arena and survives automatically. The one hazard is arena
growth: `incr.rs`'s `val_incr!` macro, which calls `alc.realloc` on the
1.5x schedule, rewrites `state.val_hdr`, `state.val_end`, `state.val` and
`state.ctn` from saved offsets after the block moves. `ctn_len` is saved and
restored alongside.

**The final call.** Once `len == buf_len`, `MORE` can no longer be returned,
so the document either completes or reports garbage or `UNEXPECTED_END`.
`YYJSON_READ_STOP_WHEN_DONE` short-circuits the trailing-content check as in
the one-shot reader. On success the deferred raw NUL is committed, the
document header is filled in (`root`, `alc`, `dat_read`, `val_read`,
`str_pool`) exactly as `read_root_*` does, and `state.hdr` and
`state.val_hdr` are set to NULL so a subsequent `yyjson_incr_free` cannot
double-free memory now owned by the document. `yyjson_incr_free` zeroes the
allocator first, then frees the arena if any, the private buffer unless
in-situ, and the state.

The only differences from the C are structural, in the ways
`CONVENTIONS.md` prescribes: the goto graph is a constant-dispatch loop,
the `#define` and `#undef` scoped macros are function-local
`macro_rules!` (`return_err`, `return_err_inv_param`,
`goto_unexpected_end`, `val_incr`, `save_incr_state`,
`check_maybe_truncated_number`), and the C's `ctn_depth` member and its
checks are absent because `YYJSON_READER_DEPTH_LIMIT` is 0 in this
configuration and the same code is absent from State A's translation unit.
The deferred raw NUL is routed through `pre = &raw mut raw_ptr` so that
`read_num`'s write and `save_incr_state`'s read of the same field cannot be
reordered by the optimiser.

Where this answer establishes it:

- S-11: Port the incremental reader so a document split at an arbitrary byte resumes with the same state (src/incr.rs).
  Established: Chunked reading returns MORE at the same boundaries and completes to the same document as a single read.

Already decided by compiled relation, so the note's account of it is
judged for correctness and not for whether the behaviour works:

- T-13: Every recorded case of module B-08 (incremental reader) agrees with State A on every recorded metric.

What only the note can settle: whether the account is correct; T-13 compiles the behaviour

### J-11 (weight 3, outcome, VALUE)

What a correct note states: The note states how the unchanged CMake invocation drives cargo and installs State A's tree, SONAME and export set while hiding Rust runtime symbols, as the reference states it.

What this answer states, from the reference note's own account:

`CMakeLists.txt` was rewritten and the invocation was not. A consumer still
runs `cmake -S . -B build && cmake --build build && cmake --install build`
and gets State A's tree.

**No C compiler is probed.** The declaration is

```cmake
project(yyjson VERSION 0.13.0 LANGUAGES NONE)
```

`LANGUAGES NONE` is what keeps CMake from looking for a C compiler it has no
use for, which matters on a machine that has cargo and no `cc`. The version
is still 0.13.0, so `PROJECT_VERSION` and everything derived from it are
unchanged, and `YYJSON_SOVERSION` is still 0. `find_program(... NAMES cargo)`
replaces the compiler check and fails the configure step with a clear message
if cargo is missing.

**The options still exist, and a non-default value is refused rather than
ignored.** Every `YYJSON_DISABLE_*` plus `YYJSON_FREESTANDING` is still
declared with `option(... OFF)` so a consumer's cache entries and scripts
keep working, and a `foreach` loop issues `FATAL_ERROR` if any is switched
on. `YYJSON_READER_DEPTH_LIMIT` and `YYJSON_WRITER_DEPTH_LIMIT` are still
cache strings and must stay empty. This is a deliberate choice over silence:
the Rust implementation provides one configuration, and a consumer who asked
for `YYJSON_DISABLE_WRITER` and got a full-featured library would be
mis-served without knowing it.

**cargo is driven by one `add_custom_command`.**

```cmake
add_custom_command(
    OUTPUT "${YYJSON_CARGO_ARTIFACT}"
    COMMAND "${CMAKE_COMMAND}" -E env
            "CARGO_TARGET_DIR=${YYJSON_CARGO_DIR}"
            "CARGO_HOME=${YYJSON_CARGO_DIR}/home"
            "CARGO_NET_OFFLINE=true"
            "${YYJSON_CARGO_EXECUTABLE}" build --release --offline --locked
            --manifest-path "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    DEPENDS ${YYJSON_RUST_SOURCES} .../Cargo.toml .../Cargo.lock
            .../.cargo/config.toml
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    COMMENT "Building yyjson with cargo"
    VERBATIM)
add_custom_target(yyjson ALL DEPENDS "${YYJSON_CARGO_ARTIFACT}")
```

Four details carry weight. `--offline` with `CARGO_NET_OFFLINE=true` and
`--locked` mean the build never touches the network and never resolves a
dependency; with zero dependencies in `Cargo.toml` there is nothing to
resolve, and `--locked` makes a drifted `Cargo.lock` an error rather than a
silent update. `CARGO_TARGET_DIR` and `CARGO_HOME` are redirected into the
CMake binary directory so the build tree stays self-contained and an
out-of-source build does not write into the source tree. `WORKING_DIRECTORY
${CMAKE_CURRENT_SOURCE_DIR}` is load-bearing and not cosmetic: cargo
discovers `.cargo/config.toml` by walking up from the **current working
directory**, not from the manifest path, so running cargo from the build
directory would silently drop the rustflags in that file. And `DEPENDS` lists
the globbed `src/*.rs` plus the three build inputs, so an edit to a module or
to `.cargo/config.toml` rebuilds.

**The SONAME comes from `.cargo/config.toml`.**

```toml
[target.'cfg(target_os = "linux")']
rustflags = ["-C", "link-arg=-Wl,-soname,libyyjson.so.0"]
```

rustc does not set a SONAME for a `cdylib` by default, so without this the
installed library would have no `DT_SONAME` and every consumer would link
against the literal path. With it, the `cdylib` carries
`DT_SONAME = libyyjson.so.0`, matching State A's `SOVERSION 0`. The target
guard keeps the Linux-only `-Wl` syntax off other platforms.

**The install tree is reproduced rule by rule.** `BUILD_SHARED_LIBS` selects
`libyyjson.so` or `libyyjson.a` from `${CARGO_TARGET_DIR}/release`, and the
shared case installs it renamed to `libyyjson.so.0.13.0` with
`install(PROGRAMS)` (so the executable bit survives), then creates the two
symlinks with `install(CODE)` and `file(CREATE_LINK ... SYMBOLIC)`:

```
libyyjson.so       -> libyyjson.so.0
libyyjson.so.0     -> libyyjson.so.0.13.0
```

honouring `$ENV{DESTDIR}`. `install(FILES src/yyjson.h DESTINATION
${CMAKE_INSTALL_INCLUDEDIR})` installs the unchanged header, and
`yyjson.pc.in` is still `configure_file`d and installed to
`${CMAKE_INSTALL_LIBDIR}/pkgconfig`, so `pkg-config --cflags --libs yyjson`
is unchanged.

**The export set is hand-written, and that is the one honest substitution.**
State A used `install(EXPORT)` to generate `yyjson-config.cmake` and
`yyjson-config-release.cmake`. With `LANGUAGES NONE` there is no library
target and therefore nothing to export, so the port ships
`cmake/yyjson-config.cmake.in` and `cmake/yyjson-config-release.cmake.in`
and `configure_file`s them. They reproduce what CMake would have generated:
the `yyjson::yyjson` imported target of the right kind (`SHARED` or
`STATIC`), `INTERFACE_INCLUDE_DIRECTORIES` pointing at the installed header,
the prefix computed relatively from `CMAKE_CURRENT_LIST_FILE`, the
`yyjson-config-*.cmake` glob and include, the `IMPORTED_LOCATION_RELEASE`
and `IMPORTED_SONAME_RELEASE` (or
`IMPORTED_LINK_INTERFACE_LANGUAGES_RELEASE "C"` for the static case), the
re-entrancy guard, the policy push and pop, and the referenced-file
existence check with CMake's own diagnostic text. A consumer's
`find_package(yyjson)` followed by
`target_link_libraries(app PRIVATE yyjson::yyjson)` behaves identically.

**Symbol visibility: the shared library is exact, and the static archive
diverges in a way that does not reach a final link.**

The shared library is the clean case. It exports exactly the 52 names, equal
to State A's exported dynamic symbol set, and needs only `libc.so.6`. Rust
gives DEFAULT visibility to `#[unsafe(no_mangle)] pub` items and hides
everything else, and `compiler_builtins` is compiled with hidden visibility,
so no Rust runtime symbol, no `compiler_builtins` symbol and no monomorphised
internal leaks out. The imports are State A's set with one substitution,
`bcmp` where State A has `memcmp`, because LLVM lowers an equality-only
comparison to `bcmp`; both are glibc, and the semantics for an equality test
are identical.

The static archive is where honesty is required. `libyyjson.a` has **406
members** and **603 defined global text symbols**, against State A's single
`yyjson.c.o` with 52. That is a real structural difference and it would be
wrong to present it as equivalence. What makes it harmless is the visibility
breakdown: of the 603, exactly **52 are DEFAULT visibility** and they are
precisely State A's names; the other **551 are HIDDEN**, and they are
`compiler_builtins` members (the 128-bit arithmetic helpers, memory
intrinsics and similar) that rustc links into every `staticlib`. Hidden
symbols are not promoted to dynamic symbols when the archive is linked into
a shared object or an executable, and they do not participate in global
symbol resolution, so a program that links `libyyjson.a` ends up exporting
exactly State A's 52 names and cannot collide with any other library's
symbols through them. The member count is a packaging artifact of how rustc
emits a `staticlib`, not an interface change. A consumer who inspects the
archive with `nm` will see the difference; a consumer who links it will not.
Shrinking the archive would require either `--gc-sections`-style
post-processing or giving up `staticlib`, and both would be changes to the
build rather than to the interface, so the port leaves it visible and
explained.

Where this answer establishes it:

- S-08: Create the crate as #![no_std], edition 2024, with no dependency, no alloc crate and no global allocator, and export exactly the 52 entry points as #[unsafe(no_mangle)] pub unsafe extern "C" (src/lib.rs, src/ffi.rs).
  Established: The shipped library offers State A's symbol set and nothing else, and resolves no crate from a registry, a git source or a vendored copy.
- S-18: Replace CMakeLists.txt: project(... LANGUAGES NONE) so no compiled language is probed, cargo driven through add_custom_command with the source directory as its working directory, the SONAME set by a link argument in .cargo/config.toml, and two hand-written package-config templates in place of the export set CMake would have generated.
  Established: The unchanged CMake commands configure, build and install both library kinds with no C compiler present, producing State A's install tree and package configuration.

Already decided by compiled relation, so the note's account of it is
judged for correctness and not for whether the behaviour works:

- T-02: The shipped shared library exports exactly State A's 52 dynamic symbols.
- T-04: The install prefix holds exactly State A's installed paths: the header, the libraries and their links, the CMake export set and the pkg-config file.

What only the note can settle: whether the account is correct; T-02 to T-04 compile the result

### J-12 (weight 3, process, EFFECT)

What a correct note states: The note states how the port was checked against State A, naming the comparisons that were run.

What this answer states, from the reference note's own account:

The port was not reviewed into correctness; it was differentially tested
against a gcc-built State A. The instrument is a harness compiled **once**
and linked **twice**, against State A's shared library and against the
port's, emitting a record stream that is compared byte for byte. Linking the
same object twice is what rules out the harness itself as a source of
difference: any divergence is in the library.

Each record line compares, for one case:

- the function's return value
- the full output bytes, when the call produces output
- the error code
- the exact error message text, not just its presence
- the error byte position
- the complete allocator call trace, with the exact size of every request

That last item is why the J-05 and J-07 sections can make claims about
request order: the order is measured, not asserted.

**Stage 2, behavioural coverage.** Ten modules, one per subsystem: reader,
error reporting, numbers, writer, mutable API, pointer and patch,
allocators, incremental reader, header interop, and file I/O.
**280,974 record lines** over **280,954 cases**. Every module agreed, and
every module was byte-identical across two independent runs, which
establishes that the records are deterministic and that a passing comparison
is not an accident of ordering or of uninitialised padding.

The "header interop" module deserves a mention because it is what tests
J-01: it exercises the `yyjson.h` inline functions from the consumer side
against both libraries, so a layout or offset mistake in `types.rs` would
show up as a wrong value read by inline code the library never executes.

**Stage 3, a discriminating corpus.** 1,535 hand-constructed cases over five
families chosen for their ability to tell two implementations apart rather
than for breadth:

1. parse and error (malformed inputs where the code, the message and the
   position are all at stake)
2. numeric round trip
3. writer escaping and formatting
4. mutation, pointer and patch sequences
5. allocation-failure injection

**27,527 record lines**, all agreeing, reproducible three times. Stage 3 also
carries a mutation score: **104 hand-written mutants of the C** were
generated, each a small plausible change of the kind a reimplementer might
make, and **90 were killed** by the corpus. The 14 survivors are the honest
part of that number: they are mutations the corpus does not distinguish,
which bounds the corpus's resolving power rather than the port's correctness.

**Exhaustive numeric and writer sweeps.** Separately from the record-stream
comparison, the numeric modules were compared over **83,726,941 checks** and
the writer over **7,095,044 comparisons**, both with **zero failures**. These
are the volumes needed to reach the three floating-point tiers of J-09 and
the trimming and exponent-boundary paths of the writer, which a hand-written
corpus cannot cover by construction: the interesting cases are the ones where
tier 2 is ambiguous and tier 3 has to adjudicate, and those are found by
volume.

**What the layout assertions add.** The 142 `const _: () = assert!(...)`
items (99 in `types.rs`, 18 in `incr.rs`, 12 in `alloc.rs`, 6 in `util.rs`,
3 in `write_num.rs`, 2 each in `read_num.rs` and `writer.rs`) are a static
complement to all of the above: they fail the build, not a test, if a size,
an offset, a table length or the little-endian assumption ever stops
holding.

**What was not done, stated plainly.** There is no fuzzing campaign and no
formal proof. The claims in this note that rest on measurement are the ones
listed above; the claims that rest on reading the code are the soundness
invariants of J-06, and those are arguments, not measurements.

Where this answer establishes it:

- S-22: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.
  Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

What only the note can settle: the trajectory's checking process, which no artifact check observes

### J-13 (weight 1, craft, INVARIANT)

What a correct note states: Each design decision the note states names the Rust module, type or function that implements it.

What this answer states, from the reference note's own account:

Every design decision above, with the module, type or function that
implements it:

| Decision | Where |
|---|---|
| Public layouts, flags, codes, constants | `types.rs`: `yyjson_val`, `yyjson_val_uni`, `yyjson_mut_val`, `yyjson_doc`, `yyjson_mut_doc`, `yyjson_alc`, `yyjson_str_pool`, `yyjson_val_pool`, `yyjson_str_chunk`, `yyjson_val_chunk`, the four iterators, the four error structs, `yyjson_ptr_ctx` |
| Layout cannot drift | 142 `const _: () = assert!` items, 99 of them in `types.rs` |
| Tag pack and unpack | `hdr::unsafe_yyjson_get_tag`, `get_type`, `get_subtype`, `get_len`, `set_len`, `set_tag`, `set_strn`, `is_ctn` |
| Immutable arena allocation and growth | `reader::yyjson_read_opts`, `read_root_single`, `read_root_minify`, `read_root_pretty` |
| Sibling-skip offset | `uni.ofs` written in `reader.rs`'s `ST_ARR_END` / `ST_OBJ_END`; read by `hdr::unsafe_yyjson_get_next`, `unsafe_yyjson_get_first`, `unsafe_yyjson_arr_is_flat` |
| Mutable child ring | `hdr::unsafe_yyjson_mut_obj_add`, `hdr::yyjson_mut_obj_add`, `yyjson_mut_obj_put`, `yyjson_mut_obj_get`, `yyjson_mut_obj_getn`, `yyjson_mut_obj_size`, `hdr::yyjson_mut_arr_append`, the four iterator functions, `hdr::unsafe_yyjson_mut_val`, `unsafe_yyjson_mut_str_alc`, `unsafe_yyjson_mut_strncpy`, `hdr::yyjson_mut_doc_set_root` (the rest of the container API is header-inline and not in the library) |
| Mutable pools and growth | `value::unsafe_yyjson_val_pool_grow`, `value::unsafe_yyjson_str_pool_grow`, `value::yyjson_mut_doc_new`, `yyjson_mut_doc_free`, `yyjson_mut_doc_set_val_pool_size`, `yyjson_mut_doc_set_str_pool_size` |
| Copies and equality | `value::yyjson_doc_mut_copy`, `yyjson_val_mut_copy`, `yyjson_mut_val_mut_copy`, `yyjson_mut_doc_mut_copy`, `yyjson_mut_val_imut_copy`, `yyjson_mut_doc_imut_copy`, `value::unsafe_yyjson_equals`, `unsafe_yyjson_mut_equals`, `unsafe_yyjson_num_equals` |
| Error position reporting | `value::yyjson_locate_pos` plus the per-module `return_err!` macros |
| Four allocators | `alloc::YYJSON_NULL_ALC`, `alloc::YYJSON_DEFAULT_ALC`, `alloc::yyjson_alc_pool_init` (`pool_ctx`, `pool_chunk`, `pool_malloc`, `pool_realloc`, `pool_free`, `pool_size_align`), `alloc::yyjson_alc_dyn_new` / `yyjson_alc_dyn_free` (`dyn_ctx`, `dyn_chunk`, `dyn_malloc`, `dyn_realloc`, `dyn_free`, `dyn_size_align`, `dyn_chunk_list_add`, `dyn_chunk_list_remove`) |
| Unaligned access, tables, UTF-8, alignment, file and size helpers | `util.rs` (`byte_load_*`, `byte_copy_*`, `byte_move_*`, `byte_match_*`, `char_is_*`, `hex_load_*`, `mem_align_up`, `size_align_up`, `size_align_down`, `u128_mul`, `u128_mul_add`, `u64_lz_bits`, `u64_tz_bits`, `f64_to_f32`, `f64_from_bits`, `pow10_table_get_sig`, `pow10_table_get_exp`), `tables.rs` (`char_table1..3`, `hex_conv_table`, `f64_pow10_table`, `u64_pow10_table`, `pow10_sig_table`), `utf8_seq.rs` |
| libc surface | `ffi.rs` |
| Number reading | `read_num::read_num`, `read_num_tail`, `read_num_raw`, `read_num_hex`, `read_num::St`, `read_num::bigint` (`bigint_set_u64`, `bigint_set_buf`, `bigint_add_u64`, `bigint_mul_u64`, `bigint_mul_pow2`, `bigint_mul_pow10`, `bigint_cmp`), `read_num::diy_fp` (`diy_fp_get_cached_pow10`, `diy_fp_mul`, `diy_fp_to_ieee_raw`) |
| inf and nan reading | `reader::read_inf_or_nan` |
| Number writing | `write_num::write_num`, `yyjson_write_number`, `digit_table`, `write_u32_len_*`, `write_u64`, `f32_bin_to_dec`, `f64_bin_to_dec`, `f64_bin_to_dec_fast`, `write_f32_raw`, `write_f64_raw`, `write_f64_raw_fixed`, `write_f32_exp`, `write_f64_exp`, `dec_trailing_zero_table`, `div_pow10_table`, `write_u64_len_1_to_16`, `write_u64_len_1_to_17`, `write_u32_len_7_to_9_trim`, `write_u64_len_16_to_17_trim` |
| String reading and NOESC | `reader::read_str`, `read_str_con`, `read_uni_esc`, `reader::is_truncated_end`, `reader::skip_trivia` |
| String writing and escape tables | `writer.rs`'s `enc_table_cpy`, `enc_table_cpy_slash`, `enc_table_esc`, `write_str`, `write_str_noesc`, `write_raw`, `write_inf_or_nan`, `hex_table` |
| Writers | `writer::yyjson_write_opts`, `yyjson_write_buf`, `yyjson_write_fp`, `yyjson_write_file`, `yyjson_val_write_*`, `yyjson_mut_write_*`, `yyjson_mut_val_write_*` |
| Incremental reader | `incr::yyjson_incr_state`, `yyjson_incr_new`, `yyjson_incr_read`, `yyjson_incr_free`, and the `I_*` state constants with `save_incr_state!`, `goto_unexpected_end!`, `val_incr!`, `check_maybe_truncated_number!` |
| JSON Pointer | `pointer::unsafe_yyjson_ptr_getx`, `unsafe_yyjson_mut_ptr_getx`, `unsafe_yyjson_mut_ptr_putx`, `unsafe_yyjson_mut_ptr_replacex`, `unsafe_yyjson_mut_ptr_removex` |
| JSON Patch and Merge Patch | `patch::yyjson_patch`, `yyjson_mut_patch`, `yyjson_merge_patch`, `yyjson_mut_merge_patch` |
| Version | `alloc::yyjson_version` |
| Panic cannot escape | `Cargo.toml` `panic = "abort"` in both profiles, `lib.rs`'s `#[panic_handler]` calling `ffi::abort`, and the absence of `abort` from the linked library's imports |
| Build, SONAME, install, export set | `CMakeLists.txt`, `.cargo/config.toml`, `cmake/yyjson-config.cmake.in`, `cmake/yyjson-config-release.cmake.in`, `yyjson.pc.in`, `Cargo.toml`, `Cargo.lock` |

Where this answer establishes it:

- S-22: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.
  Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

What only the note can settle: whether the named item is the one that implements the decision

### J-14 (weight 3, robustness, VALUE)

What a correct note states: The note states how the port keeps a Rust panic from unwinding into a C caller at every exported entry point, as the reference states it.

What this answer states, from the reference note's own account:

The C library has no unwind tables, and its callers are C code that was
compiled without any. A Rust panic that unwound across one of the 52 exported
functions would be undefined behaviour, and `extern "C"` alone does not
prevent it: since Rust 1.81 an unwind reaching an `extern "C"` boundary
aborts rather than unwinding, but "aborts" is still a behaviour State A never
exhibits. The port therefore closes the question in four layers, strongest
first.

**1. No unwinding mechanism exists.** `Cargo.toml` sets
`panic = "abort"` in **both** profiles:

```toml
[profile.release]
panic = "abort"
[profile.dev]
panic = "abort"
```

Both, not just release, so a debug build of the crate cannot grow unwind
tables either, and a consumer who builds the library in a debug
configuration gets the same guarantee. With `panic = "abort"` the compiler
emits no landing pads and no `.eh_frame` unwind information for Rust frames,
so there is no mechanism by which a panic could propagate anywhere, let alone
across a language boundary.

**2. No function is declared unwindable.** Every exported function is
`extern "C"`, never `extern "C-unwind"`; there are zero occurrences of
`C-unwind` in the crate. The private allocator callbacks
(`alloc::null_malloc`, `default_malloc`, `pool_malloc`, `dyn_malloc` and
their realloc and free siblings) are also plain `extern "C"`, which matters
because a C consumer calls those through a `yyjson_alc` function pointer, so
they are C ABI entry points even though they are not exported symbols. They
are covered by the same guarantee as the 52.

**3. There is a `#[panic_handler]`, and it aborts.** `#![no_std]` requires
one. `lib.rs` provides:

```rust
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { crate::ffi::abort() }
}
```

So even if a panic were somehow reached, control goes to libc `abort`, the
same thing a failed C assertion does, with no unwinding and no Rust runtime
involvement. This is the backstop, not the plan.

**4. No panic is reachable, and the linker proves it.** The port is written
so that no panicking construct exists in the first place: zero `unwrap()`,
zero `expect(`, zero `panic!`, zero `unreachable!`, zero `todo!`, zero
`assert_eq!`. All 142 `assert!` occurrences are inside
`const _: () = assert!(...)` items, which are compile-time checks that
produce a compilation error and no runtime code. Arithmetic that can wrap
uses `wrapping_add` / `wrapping_sub` / `wrapping_mul` / `wrapping_neg` /
`wrapping_shl` rather than the operators, so a release-profile overflow
cannot become a debug-profile panic; slice indexing is confined to the
`[T; 256]` and `[T; N]` tables of J-06 categories 12 and 13 where the index
is structurally in range; and nothing is divided by a value that could be
zero.

The direct evidence is in the linked artifact: **the shared library imports
no `abort`.** After LTO the compiler discovered that the `#[panic_handler]`
is unreachable from every exported entry point and deleted it, which it can
only do if no code path in the crate can panic. `ffi.rs` declares `abort`
and `lib.rs` calls it, yet it does not appear in the dynamic symbol
dependencies of `libyyjson.so.0.13.0`; the library needs only `libc.so.6`
and imports State A's set of libc functions with the single `memcmp` to
`bcmp` substitution of J-06 category 10. That absence is a mechanical check
anyone can repeat with `readelf -r` or `nm -u`, and it is a stronger
statement than any amount of code review: the panic path is not merely
unlikely, it is not in the binary.

Where this answer establishes it:

- S-22: Write doc/rust-port.md: the layouts and why the header stays valid, the tag encoding, the immutable arena, the mutable child lists, the allocator contract, every category of unsafe operation with its soundness invariant, the behaviours of State A kept on purpose, number handling, incremental resume, the build integration, the comparisons run, and how a panic is kept from unwinding into C.
  Established: A reviewer who cannot run the code can judge whether the port is sound rather than merely passing.

What only the note can settle: whether the account of the panic boundary holds at each entry point

## What this answer does not settle

These are the recorded limits of the grading. None of them is a reason to credit or
discredit a note, and none of them is a question to put to the note.

- Incremental reading past completion is undefined behaviour in State A, not an error path, so no case exercises it.
- State A's "number too long" message is unreachable through the public API, because truncation is reported first.
- YYJSON_READ_ERROR_DEPTH never compiles at the default depth limits, so no case reaches it.
- Permission-denied file paths are left out of the file module: the outcome depends on the effective user id, which would fail a correct port.
- unsafe_yyjson_mut_str_alc with a null document and unsafe_yyjson_mut_strncpy with a null string are unguarded in State A, so neither is compared.
- yyjson_mem_* and yyjson_strlen are not declared in the public header and are not part of the compared surface.
- The scratch contents of a failing buffer writer are undocumented in State A and are not compared.
- Stage 3 separates no mutant in mutable list surgery or the merge-patch walk: array ordering, duplicate-key handling, rotation, renaming, merge-patch sequences, raw and no-escape writing, the non-copying string constructors and pointer parent creation are exercised but discriminate nothing. Stage 2 modules B-05, B-06 and B-07 cover them.
- Stage 3 separates no mutant in incremental reading, file input and output, or the pool and dynamic allocators; stage 2 modules B-07, B-08 and B-10 cover them.
- Fourteen of the one hundred and four mutants survive the stage-3 corpus. Three are output-equivalent by construction. Six are rounding decisions no candidate reached. Two turn a bounds guard into an out-of-bounds read observable only for bytes outside the buffer. One needs an array of 10^18 elements. Two are reachable only within one chunk header of the maximum object size.
- The verbatim-span relation behind P-06 does not detect a transformed embedding of State A's implementation, such as a base64, compressed or reformatted copy. The judged note and the Phase attack anti-shortcut lane are the remaining defences.
- T-04 compares installed paths, kinds and link targets rather than the contents of the CMake export set; wrong contents are separated instead by the consumer module's find_package route.
- T-17 compares installed bytes after folding the install prefix string, because the pkg-config file and the CMake package configuration legitimately record their own prefix.
- P-03 reads the shared library's dynamic imports; the static archive's undefined symbols are not an import relation and are not read.
- The reference port is verified on linux/arm64 only. State A's ten behavioural record files are byte-identical on linux/amd64 and both images build for both platforms, but rustc cannot run under the authoring host's emulator, so the port itself was not exercised there. The frozen baseline is recorded per platform inside the verifier image, so an amd64-only difference would surface as an oracle control failure rather than as a wrong baseline.
- T-05 is a subset relation, so a shipped library declaring no dynamic dependency at all would satisfy it. No working submission can avoid the C runtime, so the relation is left as the contract binds it.
- The static archive carries rustc's compiler-builtins members, so it holds 406 members and 603 defined global text symbols against State A's single member with 52. Exactly 52 carry default visibility and 551 are hidden, so a final link still exports State A's set, and no compiled criterion reads archive membership.
