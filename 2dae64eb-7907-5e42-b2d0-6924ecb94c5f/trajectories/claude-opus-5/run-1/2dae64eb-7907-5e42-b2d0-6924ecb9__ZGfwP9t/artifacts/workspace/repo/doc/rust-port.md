# yyjson 0.13.0: the implementation in Rust

`src/yyjson.c` is gone. The library's machine code now comes from the crate in
`rust/`, built by `cargo` and driven from `CMakeLists.txt`. `src/yyjson.h` is
untouched, byte for byte, and remains the library's contract.

This note is for someone who knows the C well. It is organised around the
decisions that could make the port unsound, and each claim names the Rust item
that carries it.

## 1. Shape of the port

The crate is deliberately a transliteration, not a redesign. Module boundaries
follow the `MARK:` sections of the C file:

| module | C section |
|---|---|
| `rust/src/types.rs` | the header's structs, tags and flags |
| `rust/src/tables.rs` | the `misc/make_tables.c` lookup tables |
| `rust/src/pow10.rs` | `pow10_sig_table` |
| `rust/src/util.rs` | Character / Size / Number-and-Bit / UTF8-Validation utils |
| `rust/src/mem.rs` | the libc `mem*` primitives |
| `rust/src/alc.rs` | Null / Default / Pool / Dynamic allocators |
| `rust/src/doc.rs` | JSON Struct Utils (pools, copies, equality, `yyjson_locate_pos`) |
| `rust/src/read.rs` | JSON Reader Utils, BigInt, Diy FP, Number Reader, String Reader, Reader Implementation, Reader (Public) |
| `rust/src/incr.rs` | Incremental JSON Reader |
| `rust/src/write.rs` | Integer / Number / String Writer, Writer Implementation, Writer (Public), Mutable Writer |
| `rust/src/ptr.rs` | JSON Pointer, JSON Patch, JSON Merge-Patch |
| `rust/src/file.rs` | File Utils |

The C builds its parsers and serialisers out of `goto` labels. Each such
machine became one `loop { match state { ... } }` over an enum whose variants
are named after the C labels — `read::RState` (`ArrBegin`, `ArrValBegin`,
`ArrValEnd`, `ArrEnd`, `ObjBegin`, …), `write::WState`, `incr::IState`,
`read::NumSt`. A `goto fail_x` became `break 'fsm Fail::X` out of a labelled
block whose value is the failure, so the single `return_err` epilogue
(`read::fill_err`, `write::WFail::apply`) still runs in exactly one place, as
the C macro did. `read_str_opt`'s and `write_str`'s internal `goto`s
(`skip_ascii` / `skip_utf8` / `copy_escape` / `copy_ascii` / `copy_utf8`) are
the same idea with a local `enum S`.

The macro-unrolled chains became counted loops with the same trip counts:
`repeat16_incr(expr_jump)` is `while i < 16 { … }` followed by the
`stop_char_i` tail, and `repeat_in_1_18(expr_intg)` / `expr_sepr` / `expr_frac`
/ `expr_stop` is the `i`-counted loop at the top of `read::read_num` plus
`read::frac_scan`, which is entered at index `i` exactly like `goto
digi_frac_i`.

`read_root_minify` and `read_root_pretty` differ only in a memory ratio and a
handful of pretty-print fast paths, so they are one function,
`read::read_root_ctn::<const PRETTY: bool>`, monomorphised into two copies. The
same is done for `write::write_root_ctn` and `write::mut_write_root_ctn`. The
`PRETTY` branches are exactly the C differences: the `if (*cur == '\n') cur++`
after each container begin/end, the sixteen-at-a-time `"  "` indent skipping,
and the `",\n"` / `": "` two-byte matches on the read side; the indent writing,
the `'\n'` after each value and the two-byte rewind in `ctn_end` on the write
side.

## 2. Layout of the value and document types

`rust/src/types.rs` declares every struct the header declares, with
`#[repr(C)]`, in the header's field order:

```rust
#[repr(C)] pub union yyjson_val_uni { u64_, i64_, f64_, str_, ptr, ofs }   // 8 bytes
#[repr(C)] pub struct yyjson_val      { tag: u64, uni: yyjson_val_uni }    // 16 bytes
#[repr(C)] pub struct yyjson_mut_val  { tag: u64, uni: …, next: *mut … }   // 24 bytes
#[repr(C)] pub struct yyjson_doc      { root, alc, dat_read, val_read, str_pool }
#[repr(C)] pub struct yyjson_mut_doc  { root, alc, str_pool, val_pool }
#[repr(C)] pub struct yyjson_str_pool { cur, end, chunk_size, chunk_size_max, chunks }
#[repr(C)] pub struct yyjson_val_pool { cur, end, chunk_size, chunk_size_max, chunks }
#[repr(C)] pub struct yyjson_alc      { malloc, realloc, free, ctx }
```

`#[repr(C)]` on a struct of C-layout members gives C layout: the same offsets,
the same size, the same alignment. `u64`, `usize`, `f64`, `*mut T` and
`Option<unsafe extern "C" fn(…)>` are each ABI-identical to their C
counterparts (`Option<fn ptr>` is guaranteed to be a single nullable pointer).
Nothing is `#[repr(Rust)]`, nothing is reordered, no `PhantomData` or niche is
introduced. This matters because the header's `static inline` functions —
`unsafe_yyjson_get_tag`, `unsafe_yyjson_get_len`, `unsafe_yyjson_get_next`,
`unsafe_yyjson_mut_val`, `unsafe_yyjson_mut_str_alc`, the whole iterator and
mutable-API surface — are compiled into *the caller* and read and write these
fields directly. They see the identical bytes they saw before, because the
bytes are identical.

Two consequences are worth stating explicitly:

* The header's `unsafe_yyjson_mut_val()` / `unsafe_yyjson_mut_str_alc()` bump
  the pool cursors in `yyjson_mut_doc` themselves and only call into the
  library (`unsafe_yyjson_val_pool_grow`, `unsafe_yyjson_str_pool_grow`) when a
  chunk is exhausted. The Rust pool-growth functions therefore set `cur`, `end`
  and `chunks` exactly as the C did, including `pool->cur = (yyjson_mut_val
  *)chunk + 1` (one `yyjson_mut_val`, not one `yyjson_val_chunk`, past the
  chunk header) and `pool->end = (u8 *)chunk + size`.
* `yyjson_doc_free()` and the type/content accessors are *only* in the header.
  The library needs some of them internally; `doc::doc_free`, `doc::ObjIter`,
  `doc::MutObjIter`, `ptr::mut_obj_getn`, `ptr::mut_obj_put`,
  `ptr::mut_arr_append`, `ptr::ptr_ctx_replace`, `ptr::ptr_ctx_remove`,
  `ptr::new_mut_obj`, `ptr::new_mut_strn`, `ptr::new_mut_strncpy` are
  line-by-line copies of the header versions rather than re-exports. Where the
  header has two functions with the same name shape but different bodies, the
  right one is used: `yyjson_mut_obj_getn` (the object API, a plain scan) is
  `ptr::mut_obj_getn`, while `yyjson_mut_obj_iter_getn` (the wrap-around
  iterator scan) is `doc::MutObjIter::getn`; `yyjson_mut_obj_put` uses the
  latter, `yyjson_mut_merge_patch` the former.

## 3. Packing of type, subtype and length

`tag` is a `u64`: the low 8 bits are `type | subtype | reserved`, the upper 56
are the length. `types::unsafe_get_type/_subtype/_tag/_len/_set_len/_set_tag`
reproduce the header's shifts and masks exactly (`tag >> YYJSON_TAG_BIT` for
the length, `tag as u8 & YYJSON_TYPE_MASK` for the type). Bits 32..40 of a real
number's tag also hold the per-value floating-point format
(`YYJSON_WRITE_FP_TO_FLOAT`, `YYJSON_WRITE_FP_TO_FIXED(prec)`), which the
header sets and `write::write_num` reads back as `(val->tag >> 32) as u32`.

The flag constants were transcribed from the header and are worth double
checking, because a wrong bit is silent: the read flags run `INSITU = 1<<0`
… `ALLOW_BOM = 1<<8`, `ALLOW_EXT_NUMBER = 1<<9`, `ALLOW_EXT_ESCAPE = 1<<10`,
`ALLOW_EXT_WHITESPACE = 1<<11`, `ALLOW_SINGLE_QUOTED_STR = 1<<12`,
`ALLOW_UNQUOTED_KEY = 1<<13`; the write flags `PRETTY = 1<<0` …
`LOWERCASE_HEX = 1<<8`, with `FP_TO_FLOAT = 1<<27` and
`FP_TO_FIXED(prec) = prec<<28`.

**Immutable documents.** One allocation holds the `yyjson_doc` header followed
by the value array: `read::HDR_LEN` is `sizeof(yyjson_doc)/sizeof(yyjson_val)`
rounded up, `doc->root` is `val_hdr + HDR_LEN`, and the document pointer *is*
the allocation base (`let doc = val_hdr as *mut yyjson_doc`). Values are stored
in depth-first order. A scalar's successor is one `yyjson_val` later; a
container stores in `uni.ofs` the byte distance to its next sibling, which is
what `unsafe_yyjson_get_next` uses. While parsing, `uni.ofs` temporarily holds
the *backwards* byte distance to the parent container, and `arr_end` / `obj_end`
overwrite it with the forward sibling distance — both halves are in
`read::read_root_ctn`'s `ArrBegin`/`ArrEnd`/`ObjBegin`/`ObjEnd` arms. Object
lengths are stored halved: `ctn_len` counts keys *and* values, so `ObjEnd`
writes `(ctn_len as u64) << (YYJSON_TAG_BIT - 1)`, i.e. a shift of 7, not 8.
`unsafe_yyjson_arr_is_flat` (reimplemented as `ptr::arr_is_flat` for
`ptr_arr_get`) exploits the fact that a flat array's `ofs` equals
`(len + 1) * sizeof(yyjson_val)`.

**Mutable containers.** A container's `uni.ptr` points at its **last** child and
the children form a singly linked *circular* list through `next`; an object
stores key and value as two consecutive list nodes, so the first key is
`((yyjson_mut_val *)ctn->uni.ptr)->next->next`. Every place that walks a
mutable container does exactly that: `doc::mut_val_mut_copy`, `doc::mut_stat`,
`doc::imut_copy`, `write::mut_write_root_ctn` (`val = ctn->uni.ptr; val =
ctn_obj ? val->next->next : val->next`), `ptr::ptr_mut_arr_get`,
`ptr::ptr_mut_obj_get`, `ptr::unsafe_mut_obj_add`, `ptr::mut_arr_append`,
`yyjson_mut_merge_patch`. `doc::yyjson_val_mut_copy` is the interesting one: it
copies a whole immutable subtree into one contiguous block of mutable values
and then rebuilds the links by pointer *arithmetic* on the immutable offsets
(`mm_val.offset(ii_next.offset_from(ii_val))`), which is the C trick verbatim.

## 4. The allocator contract

Everything the library allocates goes through a `yyjson_alc`. The contract it
honours is the one the C honoured, and the differential tests check the
*sequence* of calls and their sizes, not just the results:

* `malloc(ctx, size)` is never called with `size == 0`.
* `realloc(ctx, ptr, old_size, size)` is only called with a non-null `ptr` and
  with `size > old_size`; `old_size` is the size previously requested.
* `free(ctx, ptr)` is only called with a non-null `ptr`.
* `alc == NULL` means the default allocator, resolved by
  `alc::alc_or_default` / the `if alc_ptr.is_null()` at the head of every
  public entry point, exactly where the C did it.

The default allocator (`alc::YYJSON_DEFAULT_ALC`) forwards to libc `malloc`,
`realloc(ptr, size)` (ignoring `old_size`, as the C did) and `free`. The pool
allocator (`yyjson_alc_pool_init`, `alc::pool_malloc/realloc/free`) keeps the
same `pool_chunk { size; next; }` header, the same `size_align_up(size,
sizeof(pool_chunk)) + sizeof(pool_chunk)` rounding, the same split/merge rules
and the same `size < sizeof(pool_ctx) * 4` minimum. The dynamic allocator
(`yyjson_alc_dyn_new`, `alc::dyn_*`) keeps the `dyn_chunk` header, the
`YYJSON_ALC_DYN_MIN_SIZE` rounding, the free list sorted small-to-large, the
"grow the largest free chunk" fallback and the single `def.free` sweep in
`yyjson_alc_dyn_free`. `YYJSON_NULL_ALC` is still three functions that return
`NULL` / do nothing, and is still what `yyjson_val_write_buf` and
`yyjson_mut_val_write_buf` pass down so that the writer's error path can "free"
the user's buffer harmlessly.

The three function-pointer fields are `Option<unsafe extern "C" fn …>` so that
a C `yyjson_alc` with zeroed fields round-trips bit-for-bit. They are invoked
through `yyjson_alc::malloc/realloc/free`, which use `unwrap_unchecked()`: the
header documents them as "should not be NULL" and the C called them
unconditionally, so a null pointer is the same programming error here as there.

**Allocation failure** is propagated, never papered over. Every allocating
function has the same failure path as the C: `yyjson_read_opts` returns `NULL`
with `YYJSON_READ_ERROR_MEMORY_ALLOCATION` / `"failed to allocate memory"` and
frees the input copy; the readers' `val_incr!` jumps to `Fail::Alloc`, which
frees `val_hdr`; the writers' `incr_len!` jumps to `WFail::Alloc`, which sets
`*dat_len = 0` and frees `hdr`; `doc::mut_val` / `doc::mut_str_alc` return
`NULL`, which `yyjson_val_mut_copy`, `doc::mut_val_mut_copy`, `ptr_new_key` and
`unsafe_yyjson_mut_ptr_putx` turn into `NULL` / `false` (with
`YYJSON_PTR_ERR_MEMORY_ALLOCATION`); `yyjson_patch` then maps a pending
`YYJSON_PTR_ERR_MEMORY_ALLOCATION` to `YYJSON_PATCH_ERROR_MEMORY_ALLOCATION`
and clears the nested pointer error (`ptr::patch_set_err`). The port never
aborts on allocation failure: there is no global allocator, no `alloc` crate,
no `Vec`, no `Box` — `rust/src/lib.rs` is `#![no_std]` and uses `core` only.

## 5. Unsafe operations, by category

Essentially the whole crate is `unsafe`: it is a C library. The categories, and
why each is sound:

1. **Dereferencing caller-supplied pointers** (`(*doc).root`, `(*val).tag`, …).
   The invariant is the published C contract: a non-null `yyjson_doc *` /
   `yyjson_val *` / `yyjson_alc *` points at a live object of that type. Every
   public entry point performs the same null checks the C performed, in the
   same order, before dereferencing (e.g. `yyjson_read_opts` checks `dat` and
   `len`; `yyjson_write_opts` checks `doc`; `write_root` checks `root`;
   `yyjson_mut_doc_free` checks `doc`). Where the C dereferenced without
   checking — `unsafe_yyjson_*` take a value the caller has already
   type-checked — so does this port, and the function keeps the `unsafe_`
   prefix that says so.

2. **Raw pointer walks over the parse buffer** (`*cur`, `cur.add(n)` in
   `read::*`). Soundness rests on the same invariant the C relies on: the
   reader always owns a buffer with `YYJSON_PADDING_SIZE` (4) zero bytes after
   `eof`, created by `yyjson_read_opts` (it allocates `len + 4` and
   `memset(eof, 0, 4)`) or promised by the caller under `YYJSON_READ_INSITU`.
   The scanners can therefore read up to four bytes past the logical end
   without leaving the allocation, and every scanner terminates on the NUL that
   the padding guarantees. `read_inf` is the deepest look-ahead (`cur[7]`), and
   it is short-circuited: it can only reach `cur[4]` once `cur[0..3]` were
   `inf`/`infi`, so at most one byte of padding is read before a zero stops it.

3. **Writing into the parse buffer** (`*src = 0` at the end of a string,
   `**pre = 0` for raw numbers, the `dst` copy loop for escaped strings).
   Allowed by the same contract — the reader owns, or is explicitly given, a
   mutable buffer. The destination never overtakes the source: `dst` starts at
   the first escape and an escape always consumes at least as many input bytes
   as it produces. The one subtlety the C documents is `byte_move_forward`,
   which rounds the length up to an even number and so writes one byte past the
   requested end; `util::byte_move_forward` does the same, which is in bounds
   because `dst <= src` and the extra byte is overwritten by the next
   iteration.

4. **Writing into the output buffer** (`write::*`). Every write is preceded by
   the C's own reservation: `incr_len!(n)` grows the buffer so that
   `cur + n < ctx`, and the per-type `n` values are copied from the C
   (`str_len * 6 + 16`, `FP_BUF_LEN`, `16`, `str_len + 2`, `2 * sizeof(ctx)`,
   plus `level * 4` for pretty indentation). The number writers are documented
   as needing 40 bytes (`FP_BUF_LEN`) and are only ever called with at least
   that much; `write_f64_raw`'s `memset(buf, '0', 32)` fits inside it.

5. **Pointer–integer round trips** (`val as usize - ctn as usize`,
   `(ctn as *mut u8).sub((*ctn).uni.ofs)`). These recreate C pointer
   differences and offsets that stay inside one allocation (the value pool, the
   output buffer). Byte differences are computed as `usize` subtraction rather
   than `offset_from` wherever either side can be null — notably the pool
   capacity checks in `doc::mut_val` / `doc::mut_str_alc`, where a fresh
   `yyjson_mut_doc` has `cur == end == NULL` and `offset_from` would be
   undefined while the C's pointer subtraction yields 0.

6. **Reading a union field** (`(*val).uni.u64_`, `(*val).uni.str_`). A
   `#[repr(C)]` union read is a reinterpretation of 8 bytes, which is what the
   C does. The field chosen is always the one the value's tag selects. The one
   place that reads a *possibly uninitialised* payload is
   `yyjson_merge_patch`'s `local_orig.uni.u64_ = (*builder).uni.u64_`, copied
   from the C (`local_orig.uni = builder->uni`) — `builder` is a fresh empty
   object whose `uni` was never set. The value is never used, because
   `local_orig`'s length is zero; the port keeps the copy so that the shape of
   the code and of the allocator traffic matches.

7. **Calling C function pointers** (the allocator callbacks, §4).

8. **`#[no_mangle] extern "C"` definitions**: these are the ABI boundary.
   Signatures were transcribed one by one from `src/yyjson.h`; a mismatch would
   be caught immediately by the differential drivers, which are C programs
   compiled against the unmodified header.

There is no `transmute` of references, no aliasing `&mut`, no `Vec`/`Box`
crossing the boundary, no `static mut`. The two statics (`YYJSON_NULL_ALC`,
`YYJSON_DEFAULT_ALC`) are immutable and get `unsafe impl Sync for yyjson_alc`
because they contain a (null) raw pointer; they are only ever read.

## 6. Behaviour kept on purpose, although it looks like a defect

These are all exercised by the differential tests, so "fixing" any of them
would show up as a mismatch.

* **`write_root` reports a write error with a read code.** `write::write_root`,
  `yyjson_val_write_file` and `yyjson_val_write_fp` set
  `err->code = YYJSON_READ_ERROR_INVALID_PARAMETER` on a NULL input, not the
  write constant. Both are `1`, so it is invisible — but it is what the C
  writes, and the mutable twins (`mut_write_root`,
  `yyjson_mut_val_write_file/fp`) use the *write* constant, so the asymmetry is
  preserved too.
* **`yyjson_incr_read` leaves a NUL in the caller's buffer.** It writes
  `*end = '\0'` to stop the parser at the supplied length and only restores the
  saved byte on the `YYJSON_READ_ERROR_MORE` path. On success, on a hard error,
  and on `UNEXPECTED_END` the byte stays clobbered. `incr::yyjson_incr_read`
  restores `saved_end` in exactly the one place the C does.
* **`yyjson_mut_arr_iter_next` returns the cursor *after* advancing it.** The
  header's iterator sets `iter->cur = val->next` and then returns `iter->cur`,
  not `val`. That is correct only because `uni.ptr` is the *tail*, so the
  post-advance cursor is the next element in order. `yyjson_mut_patch` walks its
  patch array that way, and `ptr::yyjson_mut_patch` reproduces the same
  `iter_cur = (*iter_cur).next; obj = iter_cur;` order.
* **Object length is stored halved.** `ObjEnd` shifts `ctn_len` by
  `YYJSON_TAG_BIT - 1`. An odd `ctn_len` (a trailing key with no value) would
  therefore lose its low bit — unreachable, because `obj_val_end` is the only
  path to `obj_end` with a non-zero length, but the shift is kept as written.
* **`unsafe_yyjson_equals` is inaccurate for duplicate keys.** Object
  comparison looks each left-hand key up in the right-hand object with an
  iterator, so `{"a":1,"a":2}` and `{"a":1,"a":1}` can compare equal. The C says
  so in a comment; `doc::unsafe_yyjson_equals` keeps the algorithm.
* **`yyjson_patch`'s error index is `iter.idx ? iter.idx - 1 : 0`**, which
  reports index 0 both for "the first operation failed" and for "the failure
  happened before iteration started". `ptr::patch_set_err` keeps it.
* **`yyjson_read_number` ignores its allocator argument** in the default
  (fast-float) configuration — the parameter only matters when
  `YYJSON_DISABLE_FAST_FP_CONV` is set, which this build does not support. The
  Rust signature keeps the parameter and ignores it (`_alc`).
* **`read_num_hex` accepts a leading `+`** and computes
  `cur += (*cur == '-' || *cur == '+') + 2` before reading digits, so `+0x10`
  parses while `0x` alone is "invalid hex number". Transcribed as written.
* **`read_inf` / `read_nan` restart from `hdr`, not from the current cursor.**
  In `read_num` and `read_num_raw` the inf/nan retry is
  `read_inf_or_nan(ptr, …)` where `*ptr` is still the start of the number, so a
  `+` that was skipped is re-examined. The Rust code passes the same `ptr_`
  rather than the local `cur`.
* **`yyjson_mut_val_write_buf` passes `estimated_val_num = 0`**, which only
  works because the fixed-buffer path never uses the estimate.
* **`write_str`'s invalid-UTF-8 replacement is assembled from the hex table**
  (two lookups for `FF` and `FD`) rather than from a constant `"FFFD"`, so
  `YYJSON_WRITE_LOWERCASE_HEX` also lowercases the replacement character. The C
  comment explains the trade-off; `write::write_str`'s `err_esc!` keeps it.
* **The `0 with any exponent is still 0` path consumes the exponent and returns
  `+0.0`/`-0.0` by bit pattern**, so `-0e9` keeps its sign. Kept in
  `read::read_num`'s `zero_tail` block.

## 7. Numbers

**Reading** (`read::read_num`) is the C algorithm, in the C's order, with the
C's cut-offs:

1. The unrolled scan accumulates up to 19 significant digits into `sig: u64`,
   with wrapping arithmetic (`wrapping_mul(10)`, `wrapping_add`) exactly where
   the C relies on unsigned wraparound, and `(*cur.add(i)).wrapping_sub(b'0')
   as u64 <= 9` as the digit test (the C writes `(u64)(cur[i] - '0') <= 9`;
   both are true only for digits).
2. 20-digit integers are admitted if `sig < U64_MAX/10 || (sig == U64_MAX/10 &&
   num <= U64_MAX%10)`; otherwise the number becomes a double (or a raw string
   under `YYJSON_READ_BIGNUM_AS_RAW`).
3. Fast path 1: if `sig < 2^53` and `|exp| <= 22`, one multiply or divide by
   `F64_POW10_TABLE[exp]` — exact in IEEE-754 double, so the result is the
   correctly rounded one.
4. Fast path 2: `sig * 10^exp` via the cached 128-bit `POW10_SIG_TABLE`, with
   the C's `bits - 1 < 0x1FD` "is there a zero bit I can stop at" test, the
   optional second 64×64 multiply against `sig2_ext`, the bit-54 rounding and
   the overflow fix-up.
5. Slow path: `diy_fp` approximation with an error bound
   (`ERR_ULP`/`ERR_CACHED_POW`/`ERR_MUL_FIXED`), and, when the approximation is
   within the error interval, an exact comparison against the upper boundary
   using the `BigInt` (`read::BigInt`, a `[u64; 64]` with `add_u64`, `mul_u64`,
   `mul_pow2`, `mul_pow10`, `cmp`, `set_u64`, `set_buf`). `set_buf` reproduces
   the digit-cut bookkeeping, including `sig -= (*sig_cut >= '5')` to undo the
   earlier rounding and the "force the last kept digit to 1" trick when more
   than `F64_MAX_DEC_DIG` digits were present.

`u128_mul` / `u128_mul_add` (`util.rs`) use Rust's `u128`, which is what the C
does on any compiler with `__int128`; `u64_lz_bits` / `u64_tz_bits` are
`leading_zeros` / `trailing_zeros`, matching `__builtin_clzll` / `__builtin_ctzll`.

**Writing** (`write::write_f64_raw`, `write_f32_raw`, `write_f64_raw_fixed`) is
the same Schubfach implementation. `f64_bin_to_dec` keeps both branches: the
"regular spacing" fast path with its four early-exit tests, and the Schubfach
body with `u128_round_to_odd`, `cbl`/`cb`/`cbr`, the `irregular` correction
(`k = (exp_bin * 315653 - 131237) >> 20`) and the `s >= 10` trim. The digits
themselves come out of the same magic-multiply routines over the same
`DIGIT_TABLE` and `DEC_TRAILING_ZERO_TABLE`: `write_u32_len_8`,
`write_u32_len_1_to_8` (including the `buf -= lz` trick that writes two bytes
from `DIGIT_TABLE[val*2 + lz]` and then rewinds), `write_u32_len_7_to_9_trim`,
`write_u64_len_16_to_17_trim`, `write_u64_len_1_to_17`, `write_f32_exp`,
`write_f64_exp`. The `-6 < dot_ofs <= 21` decision between fixed and scientific
notation, the `memset(buf, '0', 32)` + `byte_move_8`/`byte_move_16` digit
shuffle that opens a slot for the decimal point, the `yyjson_max(dot_end,
num_end)` return, and the trailing-zero trimming (two branchless steps then a
loop, then the `'.'` fix-up) are all preserved. `write_f64_raw_fixed` keeps the
`fabs(num) < 1e21` test written as a raw-bits comparison against
`0x444B1AE4D6E2EF50 << 1`, the `div_pow10` magic-number division
(`DIV_POW10_TABLE`, all 20 entries), the "remove round up, cut, re-round" dance
and the `sig_len += (sig_len >= 0) && (sig_dec >= p10[sig_len])` carry fix.

Every lookup table was extracted mechanically from the C source rather than
re-derived: `POW10_SIG_TABLE` (668 entries × 2 `u64`), `DIGIT_TABLE`,
`DEC_TRAILING_ZERO_TABLE`, `DIV_POW10_TABLE`, `U64_POW10_TABLE`,
`CHAR_TABLE1..3`, `HEX_CONV_TABLE`, `ENC_TABLE_*`, `ESC_HEX_CHAR_TABLE`,
`ESC_HEX_CHAR_TABLE_LOWER`, `ESC_SINGLE_CHAR_TABLE`.

The integer writer (`write::write_u64`) and `yyjson_write_number` keep the C's
sign handling: `*cur = '-'` is written unconditionally and the cursor advanced
by `sign`, and the negative value is formed as `~pos + 1`.

## 8. Endianness

The C loads 2/3/4 bytes into a native-endian integer and compares against masks
built in native order (`utf8_seq_def`), and copies such integers back out. The
port always uses little-endian loads (`util::byte_load_2/3/4` via
`from_le_bytes`) and little-endian stores (`uni.to_le_bytes()`), with the UTF-8
masks built by `util::seq(a,b,c,d) = a | b<<8 | c<<16 | d<<24`. Load, mask and
store agree with each other, so the result is identical to the C's on a
little-endian machine and equally correct on a big-endian one.

## 9. The incremental reader

`yyjson_incr_state` is opaque in the header, so `incr::yyjson_incr_state` is a
plain `#[repr(C)]` struct with the same fields as the C one (`label`, `alc`,
`flg`, `hdr`, `cur`, `buf_len`, `hdr_len`, `alc_len`, `ctn_len`, `val_hdr`,
`val_end`, `val`, `ctn`, `str_con[2]`, `raw_ptr`, `raw_end[1]`).

Resumption works as it does in the C, and the mechanism is the reason the port
had to keep the state machine shape:

* There are eight **save points** — `LABEL_DOC_BEGIN`, `ARR_VAL_BEGIN`,
  `ARR_VAL_END`, `OBJ_KEY_BEGIN`, `OBJ_KEY_END`, `OBJ_VAL_BEGIN`,
  `OBJ_VAL_END`, `DOC_END`. The `save_incr_state!` macro stores `label`, `cur`,
  `val`, `ctn_len`, `hdr_len` and `raw_ptr`, and then checks `cur >= end`; if so
  it leaves through `IOut::NeedMore`, which stores `val_end`, `ctn` and
  `alc_len`, restores the clobbered byte at `end`, and returns
  `YYJSON_READ_ERROR_MORE` / `"need more data"` (or `UNEXPECTED_END` if no more
  input can arrive). The next call re-enters at that label, so a chunk boundary
  can fall anywhere *between* tokens.
* A **string** can be suspended in the middle. `read_str_opt` takes the
  `con: *mut [*mut u8; 2]` continuation: on failure it stores `(end_position,
  dst)` there, and on entry it restores `src`/`dst` from it and jumps straight
  to `CopyAscii` when `dst` is non-null. On success it clears both slots. That
  is how `"abc` + `def"` across two calls still produces one string, and why the
  escape-collapsing `dst` cursor survives the boundary.
* A **number** is never suspended inside. Instead, a number that ends exactly at
  the supplied end "maybe did not end yet":
  `check_maybe_truncated_number!()` (states `ArrValMaybeEnd`,
  `ObjValMaybeEnd`, and the root-number case in `DocBegin`) returns
  `NeedMore` *without* committing, so the whole number is re-read from the
  preceding save point once more bytes arrive. The guard against unbounded
  re-reading is the C's `cur > state->cur + INCR_NUM_MAX_LEN` (1024) check,
  which turns into `"number too long"`.
* `DOC_END` is also a save point, because a complete document followed by
  pending input may still turn out to have trailing garbage; the C asks for the
  remaining bytes before finalising, and so does this port.
* The deferred raw-string terminator (`raw_ptr`, `raw_end`) is part of the
  saved state, so `YYJSON_READ_NUMBER_AS_RAW` works across chunks.
* `yyjson_incr_new` strips `YYJSON_READ_JSON5`, `ALLOW_BOM` and
  `ALLOW_INVALID_UNICODE`, as documented; on success `yyjson_incr_read` nulls
  `state->hdr` and `state->val_hdr` so that `yyjson_incr_free` does not free the
  returned document.

## 10. The build

`CMakeLists.txt` declares `project(yyjson VERSION 0.13.0 LANGUAGES NONE)`. That
is the load-bearing change: CMake never probes for a compiler, so configuring
and building works on a machine that has none. Linking still happens — rustc
invokes `cc` as a linker driver — but no translation unit is compiled.

The library is produced by a single `add_custom_command` that runs

```
cargo rustc --offline --locked --manifest-path rust/Cargo.toml \
      --crate-type <cdylib|staticlib> --release [--no-default-features]
```

with `CARGO_TARGET_DIR` pointed inside the CMake build directory (nothing is
written into the source tree), copies the artifact to the build directory under
the name the C target used, and — for the shared library — creates the two
version symlinks. `add_custom_target(yyjson-build ALL …)` makes it part of the
default build; an `INTERFACE` library `yyjson` (+ `yyjson::yyjson` alias) is
provided for in-tree `add_subdirectory` users.

`BUILD_SHARED_LIBS` selects the crate type, which is the only place the two
configurations differ:

* `ON` → `cdylib` → `libyyjson.so.0.13.0`, plus `libyyjson.so.0 →
  libyyjson.so.0.13.0` and `libyyjson.so → libyyjson.so.0`.
* `OFF` → `staticlib` → `libyyjson.a`.

**SONAME.** `rust/build.rs` emits
`cargo:rustc-link-arg-cdylib=-Wl,-soname,libyyjson.so.0`, reproducing what
`set_target_properties(yyjson PROPERTIES SOVERSION 0)` used to do.
`rustc-link-arg-cdylib` affects only the shared artifact, so the archive is
untouched.

**Library dependencies.** `build.rs` also emits
`cargo:rustc-link-arg-cdylib=-lc`. rustc links a `no_std` cdylib with
`-nodefaultlibs`, so without this the library would reference `malloc`,
`memcpy`, `fopen`, … without recording a dependency on the library that
provides them. With it, the shared object has exactly one `DT_NEEDED`:
`libc.so.6` — the same single dependency the C build had.

**Exported symbols.** rustc already emits an (anonymous) linker version script
for a `cdylib` that makes every symbol local except the crate's `#[no_mangle]`
items, and those are exactly the 52 symbols the C library exported. A second
version script cannot be stacked on top of it ("anonymous version tag cannot be
combined with other version tags"), so the one symbol that would otherwise leak
is handled differently: `core` keeps a reference to `rust_eh_personality` even
under `panic = "abort"`, and a C program linking the static archive with a bare
`-lyyjson` needs a definition for it. It therefore sits behind the default-on
cargo feature `eh-personality` (`rust/src/lib.rs`), and the *shared* build is
the one that passes `--no-default-features`. Net effect: the archive defines it,
the shared object does not contain it at all, and
`nm -D --defined-only libyyjson.so.0.13.0` lists the same 52 names as before.

**The installed tree** is reproduced file by file, with the same names, kinds
and link targets:

```
include/yyjson.h
lib/libyyjson.so.0.13.0          lib/libyyjson.a
lib/libyyjson.so.0 -> libyyjson.so.0.13.0
lib/libyyjson.so   -> libyyjson.so.0
lib/cmake/yyjson/yyjson-config.cmake
lib/cmake/yyjson/yyjson-config-release.cmake
lib/pkgconfig/yyjson.pc
```

`install(EXPORT)` cannot be used without a real CMake target, so the two
package files are generated from `cmake/yyjson-config.cmake.in` and
`cmake/yyjson-config-cfg.cmake.in`. Those templates are the files CMake 3.31
generated for the C target, with only the varying pieces substituted
(`SHARED`/`STATIC`, the configuration name, and the `IMPORTED_LOCATION_*` /
`IMPORTED_SONAME_*` / `IMPORTED_LINK_INTERFACE_LANGUAGES_*` properties). The
generated files are byte-identical to the originals in both configurations
(verified with `diff`). `yyjson.pc` still comes from the unchanged
`yyjson.pc.in`. `GNUInstallDirs` normally learns the pointer size from a
compiler probe; with no language enabled it is taken from `rustc --print cfg`
instead, so `CMAKE_INSTALL_LIBDIR` resolves the same way.

The compile-time `YYJSON_DISABLE_*` / `YYJSON_FREESTANDING` /
`*_DEPTH_LIMIT` options are still declared, but a non-default value is now a
hard `FATAL_ERROR` rather than a silently ignored setting: this port implements
the library's default configuration only, and failing loudly is better than
shipping a library that quietly ignores `YYJSON_DISABLE_WRITER`.

## 11. Panics never cross the boundary

`rust/Cargo.toml` sets `panic = "abort"` for both profiles, so there is no
unwinding machinery at all and no `catch_unwind` is needed: a panic calls the
`#[panic_handler]` in `rust/src/lib.rs`, which calls libc `abort()`. Nothing
can unwind out of an `extern "C"` function into a C caller.

The handler is a backstop, not a strategy. The code is written not to panic:
it is `#![no_std]` with no allocator (so no allocation-failure abort), it uses
raw pointers rather than slices and indexes (so no bounds-check panic),
`[profile.release]` sets `overflow-checks = false` and `debug-assertions =
false`, and the arithmetic that the C expects to wrap is spelled with
`wrapping_*` so that it is wrapping in any profile. The only indexing that
remains is into the fixed lookup tables, with indices that are `u8`-derived or
range-checked by construction (`POW10_SIG_TABLE` is indexed by
`exp10 - POW10_SIG_TABLE_MIN_EXP` where the callers have already clamped
`exp10` to the table's range, exactly as in the C).

## 12. How this was checked against the original

Everything below was run on this machine with the original `src/yyjson.c`
(recovered from the repository's first commit) compiled by gcc, and the Rust
static archive, linked into the *same* driver sources through the *unmodified*
`src/yyjson.h`. Because the drivers use the header, they also exercise the
header's `static inline` functions against the Rust data layout.

`test/compare/run.sh` builds six drivers twice and diffs the output byte for
byte. The drivers live in the script as heredocs so that the repository
contains no C translation unit.

* **`dump`** — `yyjson_read_opts` under 12 flag combinations (default, insitu,
  stop-when-done, trailing commas, comments, inf/nan, number-as-raw,
  invalid-unicode, bignum-as-raw, BOM, JSON5, everything), printing for each:
  the error code, message *pointer contents* and byte position; `dat_read` and
  `val_read`; a full recursive dump of every value's type, subtype, length and
  payload (doubles as raw bit patterns); and `yyjson_locate_pos` for the error
  offset.
* **`rw`** — 17 write-flag combinations (minify, pretty, pretty-2-spaces,
  escape-unicode, escape-unicode + lowercase-hex, escape-slashes, inf/nan,
  inf/nan-as-null, invalid-unicode, invalid-unicode + escape, newline-at-end,
  fp-to-float, fp-to-fixed 1/3/6/15, pretty+all) through
  `yyjson_write_opts`, `yyjson_mut_write_opts`, `yyjson_write_buf` with a large
  buffer and with an 8-byte buffer; plus `yyjson_doc_mut_copy` →
  `yyjson_mut_doc_imut_copy` → write again, `yyjson_equals`,
  `yyjson_mut_equals`, `yyjson_write_number` over every number in the document,
  and a file round trip.
* **`api`** — `yyjson_read_number` over 44 literals × 6 flag sets (reporting
  the end offset, code, position, message, type/subtype, re-serialisation and
  raw bits); the mutable creation API (ints, uints, sints, bools, null, str,
  strcpy, raw, reals, per-value `fp_to_fixed`/`fp_to_float`, floats) plus
  array/object modification (`remove_first`, `remove_last`, `insert`, `rotate`,
  `replace`, `remove_str`, `rename_key`) and `yyjson_mut_write_number`; 24
  JSON pointers × {`getx` immutable, `getx` mutable, `addx`, `setx`,
  `replacex`, `removex`} with the resulting `yyjson_ptr_ctx` fields and the
  resulting document; 20 JSON Patch documents and 8 Merge Patch documents
  through both the immutable and the mutable entry points, with the full
  `yyjson_patch_err`; the incremental reader fed the input in chunks of 1..7
  bytes, printing every intermediate `MORE`/error/result; the pool allocator at
  10 buffer sizes (0, 1, 31, 32, 63, 64, 100, 1 KiB, 64 KiB, 1 MiB); the
  dynamic allocator; and a **trace of every allocator call with its exact
  sizes** for read + write + `doc_mut_copy` + mutable write.
* **`num`** — 89 180 number cases: 4 000 random `double` bit patterns, three
  significands across all 2 047 exponents, 128 subnormal/boundary patterns,
  each serialised under 8 floating-point formats and read back (comparing the
  resulting bits, so a shortest-representation difference cannot hide); 2 000
  random `u64`s plus 192 bit-boundary values as both uint and sint; and
  `1e…`/`9e…`/`1234567890123456789e…`/`4.9406564584124654e…` for every decimal
  exponent from −340 to 320.
* **`extra`** — the exported entry points the other drivers do not reach:
  `yyjson_mut_doc_set_str_pool_size` / `set_val_pool_size` at ten sizes
  (including the overflow-rejecting ones) followed by an allocation that has to
  grow the pool, `yyjson_alc_pool_init` on eight misalignments of the same
  buffer and with NULL arguments, a 4 000-step pool-allocator churn (random
  malloc / realloc / free with the true `old_size`, ending in a check that the
  whole pool became free again), a 2 000-step dynamic-allocator churn,
  `yyjson_locate_pos` at every byte position plus two past the end,
  `yyjson_mut_val_write_buf` / `_write_file` / `_write_fp` / `_write_opts`
  including their NULL and too-small-buffer paths,
  `yyjson_mut_val_imut_copy`, and an allocator trace for
  `yyjson_mut_doc_mut_copy` + `yyjson_mut_doc_imut_copy` +
  `yyjson_doc_mut_copy`.
* **`io`** — `yyjson_read_file`, `yyjson_read_fp` on a *pipe* (which forces the
  unseekable chunked-read path), `yyjson_read_fp` on a file positioned at a
  non-zero offset, a missing file, `yyjson_write_buf` at **every** buffer size
  from 0 to `len + 4` (printing the returned length, code and bytes),
  `yyjson_write_file` to a good path / a bad directory / an empty path,
  `yyjson_write_fp` to `/dev/null` and to `NULL`, and the NULL-argument paths of
  the read and write entry points.

Inputs: the 30 curated files in `test/compare/inputs` (simple, pretty, number
edge cases, escape and UTF-8 edge cases, each distinct reader error, empty
input, BOMs, 200-deep nesting, a 500-element document, JSON5, unclosed comment,
single root values, 800-digit numbers, huge exponents, zero forms) plus a
generated corpus of ~295 files (random double sets, random decimal strings with
exponents in ±330, random JSON trees in minified and both pretty forms, every
3-byte prefix of a valid document, truncated UTF-8 sequences, overlong and
surrogate encodings, malformed fragments). Result: **zero differences** — 1 301
driver×input comparisons over the five input-driven drivers, 225 more for
`extra` (it is slow, so it runs over a subset), and 89 180 number cases.

`test/compare/consumer.sh <prefix> <shared|static>` checks the shipped
artifacts: it compiles the same C program three ways against an installed
prefix — `pkg-config --cflags --libs yyjson`, a hand-written
`-I<prefix>/include -L<prefix>/lib -lyyjson`, and
`find_package(yyjson REQUIRED)` + `yyjson::yyjson` — runs all three, requires
identical output, and prints the `SONAME`, the `DT_NEEDED` list and the number
of exported symbols. It passes for both the shared and the static install.

Beyond that: the installed trees were compared against the trees the C build
produced (same paths, same symlink targets, `diff`-identical
`yyjson-config.cmake` and `yyjson-config-release.cmake`);
`nm -D --defined-only` on the shared library was diffed against the C build's
list (52 symbols, identical); and both configurations were configured, built
and installed from clean build directories with `/opt/graded/bin` first on
`PATH`, i.e. with no C, C++, Objective-C or assembly compiler available.

Finally, every driver was also linked against the Rust archive with
`-fsanitize=address` and run over the whole corpus: no out-of-bounds access, no
invalid free and no leak was reported. (The `extra` driver needs
`ASAN_OPTIONS=allocator_may_return_null=1`, because it deliberately asks for a
`SIZE_MAX`-sized string pool and ASan aborts on such a request instead of
letting `malloc` return NULL as the test — and the original C — expects.)
