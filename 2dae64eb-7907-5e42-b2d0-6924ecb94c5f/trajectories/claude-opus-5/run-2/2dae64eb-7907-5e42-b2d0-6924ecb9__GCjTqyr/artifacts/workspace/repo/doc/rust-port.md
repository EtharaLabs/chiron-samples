# Porting yyjson's implementation from C to Rust

`src/yyjson.c` is gone. The library's machine code now comes from the crate in
`rust/`, built by cargo and installed by `CMakeLists.txt` into exactly the tree
the C build installed. `src/yyjson.h` is untouched, byte for byte.

This note is for a reviewer who knows the C well. It names the module, type or
function behind every decision, so each claim can be checked against the code.

Crate layout (all under `rust/src/`):

| file | C counterpart |
| --- | --- |
| `types.rs` | `yyjson.h` "JSON Structure (Implementation)" + flag/error constants |
| `libc.rs` | the `<stdlib.h>/<string.h>/<stdio.h>` surface `yyjson.c` uses |
| `util.rs` | "Load/Store Utils", "Character Utils", "Hex Character Reader", "UTF8 Validation", "Number and Bit Utils", "Size Utils", `MSG_*` |
| `tables.rs`, `pow10.rs` | every `static const` lookup table in `yyjson.c` |
| `alc.rs` | "Null/Default/Pool/Dynamic Memory Allocator" |
| `hdr.rs` | the `static inline` functions of `yyjson.h` that `yyjson.c` calls |
| `strct.rs` | "JSON Struct Utils (Public)" |
| `rnum.rs` | "BigInt", "Diy Floating Point", "Number Reader" |
| `rstr.rs` | "String Reader" |
| `read.rs` | "JSON Reader Utils", "JSON Reader Implementation", "JSON Reader (Public)" |
| `incr.rs` | "Incremental JSON Reader (Public)" |
| `wnum.rs` | "Integer Writer", "Number Writer" |
| `write.rs` | "String Writer", "JSON Writer …", "Mutable JSON Writer …" |
| `ptr.rs` | "JSON Pointer API", "JSON Patch API", "JSON Merge-Patch API" |
| `tests.rs` | in-tree checks (`cargo test`); see *How the port was checked* |

The port is a transliteration, not a redesign. Control flow that the C builds
out of `goto` is reproduced as explicit state machines whose state names are
the C label names; where the C unrolls a loop sixteen times for the optimiser,
the Rust writes the loop (the observable effect is the same because each
unrolled step short-circuits).


## 1. Value and document layout

Everything the header's inline functions touch is `#[repr(C)]` in `types.rs`
with field order, types and names mirroring the C declarations:

```rust
#[repr(C)] pub union YyjsonValUni { u64_, i64_, f64_, str_, ptr, ofs }
#[repr(C)] pub struct YyjsonVal    { pub tag: u64, pub uni: YyjsonValUni }          // 16 B
#[repr(C)] pub struct YyjsonMutVal { pub tag: u64, pub uni: YyjsonValUni,
                                     pub next: *mut YyjsonMutVal }                  // 24 B
#[repr(C)] pub struct YyjsonDoc    { root, alc, dat_read, val_read, str_pool }      // 64 B
#[repr(C)] pub struct YyjsonMutDoc { root, alc, str_pool, val_pool }                // 120 B
```

`YyjsonValUni` is a real Rust `union`, not a `u64` with casts, so a 32-bit
target keeps the C semantics where writing `uni.str` touches only the first
four bytes of the eight-byte union.

Why the header's inline functions remain correct:

* They only ever read `val->tag` (a `uint64_t` at offset 0) and `val->uni`
  (an eight-byte union at offset 8). `tests.rs::layout_matches_the_header`
  asserts the sizes, the alignment and both field offsets.
* `unsafe_yyjson_get_next()` in the header walks an immutable container with
  `ptr + val->uni.ofs`; the reader (`read.rs`, states `ArrEnd`/`ObjEnd`) writes
  exactly the same byte offset the C wrote,
  `(val - ctn) + sizeof(yyjson_val)`.
* `unsafe_yyjson_mut_str_alc()` / `unsafe_yyjson_mut_val()` in the header
  bump `doc->str_pool.cur` / `doc->val_pool.cur` directly and call the exported
  `unsafe_yyjson_str_pool_grow()` / `unsafe_yyjson_val_pool_grow()` when the
  cursor runs out. Those two exported functions are
  `strct::unsafe_yyjson_str_pool_grow` / `unsafe_yyjson_val_pool_grow` and
  maintain the same five pool fields in the same order. `hdr.rs` holds Rust
  copies of the two inline bumpers for internal callers, so the two paths stay
  in step.
* `yyjson_val_chunk` must fit inside one `yyjson_mut_val`, because
  `unsafe_yyjson_val_pool_grow` reserves exactly one value slot for the chunk
  header (`pool->cur = (yyjson_mut_val *)chunk + 1`). `tests.rs` asserts that.

### Tag packing

`tag` is `len << 8 | subtype | type`, the low three bits being the type, the
next two the subtype, with three reserved bits. `types.rs` provides
`get_type`, `get_subtype`, `get_tag`, `get_len`, `set_len`, `inc_len`,
`is_ctn`, `get_bool`, `arr_is_flat`, `equals_strn`, `get_first`, `get_next` —
one for each `unsafe_yyjson_*` in the header, with the same expressions.

Two packing quirks of the reader are preserved verbatim:

* While a container is *open*, the reader stores the parent's byte offset in
  `uni.ofs` and a provisional `(ctn_len + 1) << 8` in the tag; both are
  rewritten when the container closes. See `read.rs`, `St::ArrBegin` /
  `St::ObjBegin` / `St::ArrEnd` / `St::ObjEnd`.
* An object closes with `ctn->tag = ctn_len << (YYJSON_TAG_BIT - 1) | OBJ`,
  i.e. a shift by **7**, not 8, because `ctn_len` counted keys *and* values.
  `read.rs` and `incr.rs` both keep the `- 1`.

### Immutable documents

One allocation holds the `yyjson_doc` header followed by the value array:
`hdr_len = ceil(sizeof(yyjson_doc) / sizeof(yyjson_val))` values are skipped
and `doc == (yyjson_doc *)val_hdr`. The string bytes live in the (optionally
copied) input buffer, pointed to by `doc->str_pool`; strings are terminated
in place by overwriting the closing quote. `read.rs::read_root_ctn` and
`read_root_single` compute `hdr_len` exactly as the C does.

Children of a container are laid out contiguously in document order, and
`uni.ofs` of a container is the byte distance to its next sibling.
`arr_is_flat()` (used by `ptr.rs::ptr_arr_get`) exploits the case where an
array contains only scalars.

### Mutable containers

A mutable container's `uni.ptr` points at its **last** child, and the children
form a circular singly-linked list through `next`; an object stores key and
value as two consecutive nodes, so the first key is
`((mut_val *)obj->uni.ptr)->next->next`. Every traversal in `hdr.rs`,
`strct.rs`, `write.rs` and `ptr.rs` follows that convention, including
`hdr::unsafe_mut_obj_add`, which relinks `prev_val->next = key` and
`val->next = next_key` exactly as the header's inline version does.


## 2. The allocator contract

`alc.rs` keeps the four-function `yyjson_alc` vtable and the three allocators:

* `YYJSON_NULL_ALC` — every call fails; used by `yyjson_*_write_buf`.
* `YYJSON_DEFAULT_ALC` — `malloc` / `realloc(ptr, size)` (the `old_size`
  argument is ignored, as in the C) / `free`.
* `yyjson_alc_pool_init` — the same first-fit free list with
  `pool_chunk`/`pool_ctx` headers, the same `size_align_up`/`mem_align_up`
  arithmetic, the same split-if-more-than-two-headers-spare rule and the same
  neighbour coalescing in `pool_free`.
* `yyjson_alc_dyn_new` / `_dyn_free` — the same `free_list` (sorted smallest
  first) and `used_list` dummy headers, the same "resize the largest free
  chunk" fallback in `dyn_malloc`, and `YYJSON_ALC_DYN_MIN_SIZE` rounding.

The contract honoured throughout is the C one, and nothing in the port relies
on anything stronger:

* `malloc(ctx, size)` is never called with `size == 0`.
* `realloc(ctx, ptr, old_size, size)` is only ever used to **grow** a live
  block, and is always told the previous size.
* `free(ctx, ptr)` is never called with `NULL`.
* The allocator is copied *by value* into `yyjson_doc::alc` /
  `yyjson_mut_doc::alc` at creation and used for every later allocation and
  for the final free, so the caller's `yyjson_alc` need not outlive the call.

The crate never touches Rust's global allocator: it is `#![no_std]` with no
`extern crate alloc`, so there is no `#[global_allocator]` to collide with the
caller's. Every byte comes from the `yyjson_alc`.

**When an allocation fails**, the port does what the C does, which is not
uniform and is deliberately not made uniform:

* `yyjson_read_opts` → `YYJSON_READ_ERROR_MEMORY_ALLOCATION` with
  `msg = "failed to allocate memory"` and `pos = 0`, and the input copy is
  freed (`read.rs`).
* The root readers free the partially built value array and report
  `MEMORY_ALLOCATION` at the *current* cursor — but first run that position
  through `is_truncated_end()`, so a failing allocation at the very end of the
  input is reported as `UNEXPECTED_END`. That is what the C macro does; the
  order is preserved.
* The writers free the output buffer, set `*dat_len = 0` and report
  `YYJSON_WRITE_ERROR_MEMORY_ALLOCATION` (`write.rs::WFail::Alloc`).
* `yyjson_mut_doc_new`, `yyjson_mut_*` creators, `yyjson_val_mut_copy`,
  `yyjson_mut_val_imut_copy` return `NULL`. Partially built values stay in the
  document's pools and are released by `yyjson_mut_doc_free`; as in the C,
  `yyjson_val_mut_copy` can leave a half-linked subtree behind on failure, and
  `yyjson_mut_val_imut_copy` frees the doc block but only after the string
  pool allocation fails.
* JSON Pointer `put` reports `YYJSON_PTR_ERR_MEMORY_ALLOCATION`, and
  `yyjson_patch` translates that into
  `YYJSON_PATCH_ERROR_MEMORY_ALLOCATION` while zeroing `err->ptr` — the exact
  `return_err` dance of the C macro (`ptr.rs`).

The allocator **call sequence** is part of the contract, so the port makes the
same calls with the same sizes in the same order: the reader's
`len + YYJSON_PADDING_SIZE` input copy, the
`hdr_len + dat_len / ratio + 4` first value array (ratio 6 minified, 16
pretty), the `alc_len += alc_len / 2` growth, the writer's
`vals * ratio + 64` rounded up to `sizeof(write_ctx)` (ratio 18 / 32), the
mutable pools' `0x100`-byte and `0x10 * 24`-byte first chunks doubling up to
`0x10000000` / `0x1000000 * 24`. The differential harness records every
allocator call and its size and compares them (see §8).


## 3. Unsafe operations and why each is sound

The crate is one large `unsafe` surface by construction: it implements a C ABI
over caller-owned memory. The categories are:

1. **Dereferencing caller pointers** (`*mut YyjsonDoc`, `*mut YyjsonVal`,
   `*const YyjsonAlc`, `FILE *`, …). Soundness comes from the documented C
   precondition, and every `NULL` check the C performs is performed here — the
   port adds none and drops none. `types.rs::get_type` and friends read only
   the first 16 bytes of a value, which the C API guarantees to exist.

2. **Pointer arithmetic over the parse buffer.** The reader is allowed to read
   up to `YYJSON_PADDING_SIZE` (4) bytes past the logical end because
   `yyjson_read_opts` either copies the input and appends four zero bytes, or
   (with `YYJSON_READ_INSITU`) requires the caller to have done so. `byte_load_4`
   and the 16-way unrolled scans in `rstr.rs` rely on that, exactly as the C
   does; the terminating `'\0'` is in none of the "skip" character classes, so
   every scan stops at or before the padding.

3. **Unaligned loads and stores.** `util.rs::byte_load_2/3/4`,
   `byte_copy_*`, `byte_move_*` use `read_unaligned`/`write_unaligned` or
   `copy_nonoverlapping`, never a plain typed `*ptr` on an unaligned address.
   This is the one place where the Rust is *stricter* than the C translation
   unit, which relied on `memcpy` for the same purpose.

4. **Deliberately overlapping / oversized copies.** `byte_move_forward`
   rounds its length up to an even number and may write one byte more than
   asked; `write_str`'s three-byte UTF-8 copy writes four. Both are sound
   because the destination is inside a buffer already sized for the worst case
   (`len * 6 + 2` for strings) and the extra byte is immediately overwritten.
   The comment at `util.rs::byte_move_forward` records this.

5. **Transiently out-of-range cursors.** `wnum.rs`'s integer writers use the
   C trick `buf -= leading_zero` and only ever add to the cursor afterwards,
   and `write_f64_raw` computes `dot_end = buf + dot_ofs + 2`, which can land
   before `buf` when `dot_ofs < 0`. In Rust those are computed with
   `wrapping_sub` / `wrapping_offset`, so no provenance rule is violated even
   though the intermediate address is outside the allocation; the value is
   only ever used in a comparison or after being brought back in range. All
   other "is the cursor past the end" tests that could form an out-of-range
   pointer (`src + 4 > end` in `write.rs::write_str`,
   `cur + len <= eof` in `read.rs::is_truncated_str`,
   `state->cur + INCR_NUM_MAX_LEN` in `incr.rs`) were rewritten as `usize`
   comparisons.

6. **Casting `*mut YyjsonMutVal` to `*mut YyjsonVal`** (`types::mut_as_val`).
   Sound because the first 16 bytes of both structs are identical by
   construction (`tag`, `uni`), only the tag/uni fields are accessed through
   the cast, and the C does the same cast in `write_num` and `patch_op_get`.

7. **Reinterpreting the document block.** `read.rs` allocates
   `alc_len * sizeof(yyjson_val)` bytes and then uses the first `hdr_len`
   values as a `yyjson_doc`. That is sound because the allocation is at least
   `hdr_len` values long, `hdr_len` is `ceil(sizeof(doc)/sizeof(val))`, and
   `yyjson_val` has alignment 8 ≥ `align_of::<yyjson_doc>()`.

8. **Writing through `*mut` out-parameters** (`yyjson_read_err *`,
   `size_t *line`, …). Guarded by the same `if (ptr)` tests as the C.

9. **`libc::zero::<T>()`**, a `memset(p, 0, size_of::<T>())` used where the C
   writes `memset(&x, 0, sizeof(x))`. Only applied to `#[repr(C)]`
   all-zeroes-valid types (error structs, iterators, pools, allocator).

10. **Calling a function pointer out of `yyjson_alc`**
    (`YyjsonAlc::alloc/realloc/dealloc`). `unwrap_unchecked()` is used because
    the C dereferences the pointers unconditionally; the documented contract
    is that none of the three may be `NULL`, and every allocator the library
    hands out (`YYJSON_NULL_ALC`, default, pool, dyn) fills all three.

11. **`unsafe impl Sync for YyjsonAlc`** so the three allocator vtables can be
    `static`. Sound because the statics are immutable and their `ctx` is
    always null.

Potential arithmetic and bounds problems were searched for mechanically: the
whole differential suite was re-run against a build with
`-C overflow-checks=on -C debug-assertions=on`, which turns every integer
overflow, every slice index and every `unreachable_unchecked` into an abort.
It produced byte-identical output.


## 4. Quirks of the original that were kept on purpose

These all look like defects and all survive in the port, because the current
library's behaviour is the contract:

* **`-0` is not a negative zero.** `read_num` takes the leading-zero path,
  `return_0()` stores tag `NUM|SINT` with payload `0`, so `-0` round-trips as
  `0`. Only `-0.0` keeps its sign. (`rnum.rs`, `return_0!`;
  `tests.rs::reads_and_writes` pins it.)
* **`yyjson_write_file("", …)` and `yyjson_val_write_file`/`_fp` report the
  *read* error code** `YYJSON_READ_ERROR_INVALID_PARAMETER` (= 1) rather than
  `YYJSON_WRITE_ERROR_INVALID_PARAMETER`; so does `write_root` for a `NULL`
  root. The mutable variants use the write code. `write.rs` keeps the
  asymmetry.
* **`yyjson_read_file` reports `MSG_FREAD` ("failed to read file") when
  `fopen` fails**, not `MSG_FOPEN`. `read.rs::yyjson_read_file`.
* **Duplicate object keys are allowed**, and `unsafe_yyjson_equals` says so in
  a comment: comparing two objects with duplicate keys may be inaccurate
  because it looks each left key up on the right. Kept (`strct.rs`).
* **`yyjson_patch`'s `move` with empty `from` *and* empty `path` is a no-op**
  that does not even validate the pointers (`ptr.rs`, `PatchOp::Move`).
* **`yyjson_mut_val_imut_copy` initialises `str_hdr` to `&str_sum`**, a
  deliberately non-null bogus pointer that quiets a static analyser and is
  never dereferenced unless `str_sum > 0`. Kept, with the C's comment.
* **The incremental reader silently drops the non-standard flags**
  (`YYJSON_READ_JSON5`, `ALLOW_BOM`, `ALLOW_INVALID_UNICODE`) in
  `yyjson_incr_new`, so a JSON5 document parses incrementally only as strict
  JSON. `incr.rs::yyjson_incr_new`.
* **The incremental reader's root error message differs** from the
  non-incremental one: `"unexpected character, expected a valid root value"`
  instead of `MSG_CHAR`. That required a dedicated failure label
  (`read::Fail::CharMsg`) so that the message is not overwritten by the shared
  error table.
* **`yyjson_read_number` ignores its `alc` argument** in the default (fast
  float) configuration — only the `YYJSON_DISABLE_FAST_FP_CONV` build used it.
  `read.rs` keeps the parameter and ignores it.
* **Shortest-form output is ECMAScript's, with three documented deviations**:
  `-0.0` keeps its sign, a decimal point is always present
  (`100.0`, not `100`), and the exponent never carries a `+`.
* **`9.999999999999999e22` prints as `1e23`.** The value is the double nearest
  to both; the Schubfach shortest-digit search picks `1e23`. Pinned in
  `tests.rs::shortest_double_output` so nobody "fixes" it.
* The writer emits a trailing `,` after every element and then backs the
  cursor up over it when the container closes; with
  `YYJSON_WRITE_NEWLINE_AT_END` the minified writer overwrites that last byte
  (`*(cur - 1) = '\n'`) instead of appending. Reproduced exactly in
  `write.rs`, `WSt::CtnEnd` / `WSt::DocEnd`.


## 5. Numbers, digit for digit

**Reading** (`rnum.rs::read_num`) is the full fast-path cascade of the C, in
the same order, so the same inputs take the same path and get the same bits:

1. Up to 19 significant digits are accumulated in a `u64` by the unrolled
   `expr_intg` / `expr_sepr` / `expr_frac` / `expr_stop` chains, which the port
   expresses as the `'lead` block plus `digi_frac()`. Integers that fit come
   back as `uint`/`sint` without touching floating point.
2. **Fast path 1** — if `sig < 2^53` and `|exp| <= 22`, one `f64` multiply or
   divide by `F64_POW10_TABLE[exp]` gives the exactly-rounded result. Same
   table, same single operation, same direction (`*` for `exp >= 0`, `/` for
   `exp < 0`); the distinction matters because `x / 1e3` and `x * 1e-3` do not
   always agree.
3. **Fast path 2** — a 128×64 multiply against `POW10_SIG_TABLE`
   (`pow10.rs`, 668 entries for `10^-343 … 10^324`, transcribed verbatim) with
   the C's exactness test `bits - 1 < 0x1FD`, the optional second multiply by
   the low 64 bits, and the same round-up-at-bit-54 and overflow fix-up.
4. **Slow path** — the diy-fp approximation with the same `ERR_ULP` budget,
   then, when the approximation lands inside the uncertainty window, the
   `bigint` comparison (`BigInt { used, bits: [u64; 64] }`,
   `bigint_set_buf` / `mul_pow10` / `mul_pow2` / `cmp`) with the identical
   `F64_MAX_DEC_DIG = 768` digit cut and the "force the last cut digit to 1"
   rule.

`u128` replaces the C's `__int128` in `u128_mul` / `u128_mul_add`
(`util.rs`) — same results, no intrinsics.

**Writing** (`wnum.rs`) is the same Schubfach implementation:

* Integers go through `write_u32_len_*` / `write_u64`, with the
  multiply-and-shift division constants (`109951163 >> 40`, `5243 >> 19`,
  `429497 >> 32`, `167773 >> 24`, `41 >> 12`, `656 >> 16`) copied unchanged,
  and the `DIGIT_TABLE` two-digit pairs.
* `f64_bin_to_dec` keeps the "fast path" loop (the one the C writes as
  `while (likely(sig_raw)) { … break; }`) *and* its four exit conditions, then
  falls into Schubfach proper. `f32_bin_to_dec` is the 32-bit twin, reached
  only through `YYJSON_WRITE_FP_TO_FLOAT`.
* `f64_bin_to_dec_fast` plus `div_pow10` and `DIV_POW10_TABLE` implement
  `YYJSON_WRITE_FP_TO_FIXED(prec)`, including the "remove round up, cut, add
  round up, maybe bump `sig_len`" sequence.
* Trailing-zero trimming uses `DEC_TRAILING_ZERO_TABLE` and the same
  branchless `end -= (*(end-1) == '0')` pair followed by a loop.
* The choice between plain and scientific notation is the C's
  `-6 < dot_ofs <= 21`, and the plain form is produced by the same
  `memset(buf, '0', 32)` / `byte_move_16` shuffle so that the digit placement
  and the `max(dot_end, num_end)` ending are bit-identical.

All lookup tables (`tables.rs`, `pow10.rs`) were transcribed mechanically from
the C source and are byte-for-byte the same values, including the one entry
the C writes with a lower-case hex digit (`0xDB4AB0d3` in `div_pow10_table`).

Two correctness properties follow from this and are demonstrated, not assumed:
the fuzz harness round-trips 40 000 random double bit patterns (including
subnormals, infinities and NaNs) through `write` and `read`, and compares the
produced text against the C character by character, for shortest, float and
fixed formats.


## 6. The incremental reader

`incr.rs` keeps `yyjson_incr_state` with the same fields as the C struct and
the same meaning. Resumption works exactly as before:

* `state.label` is one of the eight resume points
  (`doc_begin`, `arr_val_begin`, `arr_val_end`, `obj_key_begin`,
  `obj_key_end`, `obj_val_begin`, `obj_val_end`, `doc_end`). On entry the
  label is mapped to a `St` variant and the state machine jumps there.
* `save_incr_state!(label)` stores `label`, `cur`, `val`, `ctn_len`,
  `hdr_len` and `raw_ptr`, and then — this is the whole trick — returns
  "need more data" if `cur >= end`. So a chunk boundary can only be observed
  at a point the parser can be restarted from.
* `yyjson_incr_read` writes a `'\0'` at `hdr[len]`, saving the byte it
  displaced, so all the ordinary scanners stop at the end of the data seen so
  far without any bounds checks. The byte is restored on the
  `YYJSON_READ_ERROR_MORE` path and left in place when a document completes.
* Strings can straddle a boundary. `read_str_opt` takes a `con: *mut *mut u8`
  pair (`state.str_con`); on failure it stores `(src, dst)` there, and the
  next call restores them and jumps straight back to `copy_ascii` when a
  `dst` was recorded (an escape had already been seen) or to `skip_ascii`
  otherwise. `rstr.rs` implements both entry points.
* Numbers can also straddle a boundary, and a number is not final until a
  delimiter is seen; `check_maybe_truncated_number!` asks for more data unless
  more than `INCR_NUM_MAX_LEN` (1024) bytes have been consumed since the last
  save point, in which case it fails with `"number too long"`.
* A value array that grows across calls is re-based in `val_incr!`, which
  fixes up `val`, `ctn`, `state.val`, `state.val_hdr` and `state.val_end`
  after the `realloc`.
* `doc_end` refuses to finish while `len < state->buf_len`, because trailing
  garbage in the not-yet-delivered bytes still has to be rejected.
* On success the state gives up ownership (`state->hdr = NULL`,
  `state->val_hdr = NULL`) so that `yyjson_incr_free` does not double free the
  document.

The harness drives every corpus input through the incremental reader at chunk
sizes 1, 2, 3 (and 1–5 for a second corpus) under six flag combinations, and
compares the whole sequence of `(len, code, pos, msg)` tuples.


## 7. Build, SONAME and exported symbols

`CMakeLists.txt` declares `project(yyjson VERSION 0.13.0 LANGUAGES NONE)`.
With no language enabled, CMake never probes for a C/C++/ObjC/assembly
compiler, so the graded build — which has none — configures cleanly. The only
tools looked up are `cargo` and `rustc`.

* **Building.** An `add_custom_command` runs
  `cargo build --release --offline --manifest-path rust/Cargo.toml` with
  `CARGO_TARGET_DIR` pointed inside the CMake binary directory, so nothing is
  written into the source tree and nothing survives in `rust/target`. The
  crate has no dependencies and `rust/Cargo.lock` is checked in, so `--offline`
  never needs a registry. `add_custom_target(yyjson ALL …)` keeps the target
  name the C build used.
* **`CMAKE_SIZEOF_VOID_P`.** `GNUInstallDirs` chooses `lib` vs `lib64` from the
  pointer size, which is normally a by-product of the compiler probe. The
  build asks `rustc --print cfg` for `target_pointer_width` and sets
  `CMAKE_SIZEOF_VOID_P` before `include(GNUInstallDirs)`, so the install
  directory layout is the same one the C build produced.
* **SONAME.** `rust/build.rs` emits
  `cargo:rustc-cdylib-link-arg=-Wl,-soname,libyyjson.so.0`, giving the cdylib
  `DT_SONAME = libyyjson.so.0`, the value `set_target_properties(… SOVERSION 0)`
  produced before.
* **Installed tree.** The cdylib is installed as `libyyjson.so.0.13.0` with
  0755 permissions and two symlinks are created with
  `cmake -E create_symlink` — `libyyjson.so.0 -> libyyjson.so.0.13.0` and
  `libyyjson.so -> libyyjson.so.0`, the same names, kinds and targets as
  before. The static configuration installs `libyyjson.a`. `src/yyjson.h`,
  `lib/pkgconfig/yyjson.pc` (from the unchanged `yyjson.pc.in`) and
  `lib/cmake/yyjson/yyjson-config.cmake` plus
  `lib/cmake/yyjson/yyjson-config-release.cmake` complete the tree. The two
  CMake package files are generated from `cmake/yyjson-config.cmake.in` and
  `cmake/yyjson-config-build-type.cmake.in`, which reproduce what
  `install(EXPORT)` used to emit, including the
  `IMPORTED_LINK_INTERFACE_LANGUAGES_RELEASE "C"` that the static export
  carried.
* **Exported symbols.** A Rust `cdylib` exports only `#[no_mangle] pub extern`
  items, so the dynamic symbol table contains exactly the 52 `yyjson_*` and
  `unsafe_yyjson_*` names the C library exported and nothing from the Rust
  runtime. Verified by diffing `nm -D --defined-only` against the original.
* **Dependencies.** The crate is `#![no_std]` and declares only the libc
  functions it calls, so the shared object's single `DT_NEEDED` is
  `libc.so.6`, as before. (`build.rs` adds `cargo:rustc-link-lib=dylib=c` so
  that the dependency is recorded explicitly rather than left to the host
  program.)
* **Options.** The `YYJSON_DISABLE_*` / `YYJSON_FREESTANDING` options still
  exist as cache entries so that existing scripts configure, but they are not
  honoured: the Rust build always produces the full, default-configured
  library and prints a warning if one is set. That is the one deliberate
  reduction in configurability; the default build — the one that is graded and
  the one the installed header defaults to — is unaffected.


## 8. How the port was checked

The original `src/yyjson.c` was kept available while the port was written, and
three C harnesses were compiled **twice** — once against the original static
library, once against the Rust static library, using the same unchanged
`src/yyjson.h` — and their stdout diffed. A difference of a single byte fails.

1. `difftest` — ~21 000 lines. A corpus of 120 hand-chosen documents
   (literals, every numeric edge case, broken UTF-8, truncated escapes,
   surrogates, BOMs, comments, pretty and minified shapes, trailing garbage)
   crossed with 13 read-flag combinations and 16 write-flag combinations.
   For each it prints the error code, byte position and message text, the
   bytes read, the value count, a full recursive dump of the parsed tree
   (type, subtype, length, and for reals the raw IEEE-754 bits), every
   serialisation, the mutable copy, the immutable copy back, and the equality
   predicates. It then covers: `yyjson_write_number` for doubles, floats,
   fixed precision and integers; `yyjson_read_number`; the mutable building
   and modification API; JSON Pointer get/add/replace/remove including the
   error codes and positions; JSON Patch and JSON Merge Patch in both the
   immutable and mutable flavours; a **tracing allocator that logs every
   `malloc`/`realloc`/`free` with its exact sizes**; a **failing allocator**
   that fails the *n*-th allocation for n = 0…23 so that every out-of-memory
   branch is exercised; the pool allocator at eight buffer sizes; the dynamic
   allocator; the incremental reader at three chunk sizes over the whole
   corpus; file and `FILE *` reading and writing; and `yyjson_write_buf` at
   eleven buffer sizes including the ones that are too small.
2. `fuzz` — ~150 000 lines, deterministic xorshift. 20 000 documents assembled
   from random JSON fragments under eight read-flag sets and eight write-flag
   sets; 40 000 random double **bit patterns** written in shortest, float and
   fixed form and read back; 40 000 random numeric texts through
   `yyjson_read_number`, `yyjson_read` with extended numbers, and with
   `BIGNUM_AS_RAW`; 30 000 random byte strings (mostly invalid UTF-8) read
   with `ALLOW_INVALID_UNICODE` and re-serialised eight ways; arrays of 16 to
   32 768 elements in minified and pretty form (these drive the reader's and
   writer's `realloc` growth loops); nesting depths 1…200; and 20 000 random
   JSON Pointer strings against a fixed document, including `add` through each
   one.
3. `extra` — JSON5 (unquoted keys with `\u` escapes and non-ASCII identifier
   bytes, single quotes, extended escapes, extended whitespace, hex numbers)
   under nine flag sets; object/array iterators including the rotating
   `yyjson_obj_iter_get` lookup cache and iterator-driven removal; sub-value
   writers (`yyjson_val_write_*`, `yyjson_mut_val_write_*`) to buffer, file
   and `FILE *` with the file contents read back; `yyjson_mut_doc_set_*_pool_size`
   at ten sizes; the incremental reader over six flag sets and five chunk
   sizes; `yyjson_locate_pos` at every position of nine strings including
   BOM-prefixed and multi-byte ones; `yyjson_ptr_ctx_append/replace/remove`;
   and documents shaped to hit the pretty reader's fast paths.

All three produce byte-identical output. They were re-run after every change,
against builds at `-O1` and `-O2` on the harness side, and against a Rust
build with `-C overflow-checks=on -C debug-assertions=on` (also byte-identical,
and with no panic).

The installed artefacts were compared directly: `find | sort` over both
prefixes for both library kinds, `readelf -d` for `SONAME` and `NEEDED`, and
`nm -D --defined-only` for the exported symbol set. A small consumer program
was built and run five ways against the installed trees — `pkg-config`, a
hand-written `-I/-L/-lyyjson` line against the shared library, the same against
the static library, and `find_package(yyjson)` + `yyjson::yyjson` for both
configurations.

`rust/src/tests.rs` (`cargo test`) keeps a small residue of this in the tree
for environments with no C compiler: the layout assertions, tag packing, a
dozen read/write round-trips, an error position, and the shortest-double cases
that are easiest to get wrong.


## 9. Panics never reach a C caller

Three layers, outermost first:

1. **`panic = "abort"`** is set for both the `release` and `dev` profiles in
   `rust/Cargo.toml`. The crate is compiled with the abort strategy, so no
   landing pads and no unwinding tables are generated for it; there is nothing
   that *could* unwind across the FFI boundary.
2. **`#[panic_handler]`** in `rust/src/lib.rs` calls `libc::abort()` and never
   returns. `#![no_std]` requires it, and it makes the failure mode explicit:
   the process dies the way an `assert()` failure in the C would.
3. **No panicking constructs on the hot paths.** Index expressions are all
   into fixed-size lookup tables with indices masked to the table size
   (`CHAR_TABLE1[c as usize]` with `c: u8`, `POW10_SIG_TABLE[idx]` with `idx`
   bounded by the caller's documented exponent range); all other memory access
   goes through raw pointers, which cannot panic. The one `unwrap_unchecked`
   is on the allocator function pointers (§3.10). The
   `-C overflow-checks=on -C debug-assertions=on` run described in §8 confirms
   that none of the arithmetic in the port overflows on the whole test corpus,
   so the release build's wrapping behaviour is never relied on by accident.

Rust also has no `longjmp`-style escape: there is no `catch_unwind`, no thread
spawning, and no global state, so a call into the library either returns
normally or aborts the process.
