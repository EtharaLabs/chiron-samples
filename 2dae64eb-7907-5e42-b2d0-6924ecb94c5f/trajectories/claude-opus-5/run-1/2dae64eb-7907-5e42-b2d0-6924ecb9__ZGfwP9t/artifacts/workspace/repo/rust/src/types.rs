//! The C ABI types of `src/yyjson.h`, re-declared in Rust with `#[repr(C)]`.
//!
//! Every struct here must keep the exact layout the header's `static inline`
//! functions assume, because those functions are compiled into the *caller*
//! and poke at these fields directly.

use core::ffi::{c_char, c_int, c_void};

/* ---------------------------------------------------------------- JSON types */

pub type yyjson_type = u8;
pub const YYJSON_TYPE_NONE: u8 = 0;
pub const YYJSON_TYPE_RAW: u8 = 1;
pub const YYJSON_TYPE_NULL: u8 = 2;
pub const YYJSON_TYPE_BOOL: u8 = 3;
pub const YYJSON_TYPE_NUM: u8 = 4;
pub const YYJSON_TYPE_STR: u8 = 5;
pub const YYJSON_TYPE_ARR: u8 = 6;
pub const YYJSON_TYPE_OBJ: u8 = 7;

pub type yyjson_subtype = u8;
pub const YYJSON_SUBTYPE_NONE: u8 = 0 << 3;
pub const YYJSON_SUBTYPE_FALSE: u8 = 0 << 3;
pub const YYJSON_SUBTYPE_TRUE: u8 = 1 << 3;
pub const YYJSON_SUBTYPE_UINT: u8 = 0 << 3;
pub const YYJSON_SUBTYPE_SINT: u8 = 1 << 3;
pub const YYJSON_SUBTYPE_REAL: u8 = 2 << 3;
pub const YYJSON_SUBTYPE_NOESC: u8 = 1 << 3;

pub const YYJSON_TYPE_MASK: u8 = 0x07;
pub const YYJSON_TYPE_BIT: u8 = 3;
pub const YYJSON_SUBTYPE_MASK: u8 = 0x18;
pub const YYJSON_SUBTYPE_BIT: u8 = 2;
pub const YYJSON_RESERVED_MASK: u8 = 0xE0;
pub const YYJSON_TAG_MASK: u8 = 0xFF;
pub const YYJSON_TAG_BIT: u8 = 8;

pub const YYJSON_PADDING_SIZE: usize = 4;

/* ----------------------------------------------------------------- allocator */

pub type yyjson_malloc_fn = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type yyjson_realloc_fn =
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> *mut c_void>;
pub type yyjson_free_fn = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct yyjson_alc {
    pub malloc: yyjson_malloc_fn,
    pub realloc: yyjson_realloc_fn,
    pub free: yyjson_free_fn,
    pub ctx: *mut c_void,
}

/// The two allocator statics (`YYJSON_NULL_ALC`, `YYJSON_DEFAULT_ALC`) are
/// immutable and only ever read, so sharing them across threads is sound.
unsafe impl Sync for yyjson_alc {}

impl yyjson_alc {
    /// `alc->malloc(alc->ctx, size)`.
    ///
    /// The three function pointers are documented as "should not be NULL" and
    /// the C code calls them unconditionally; a NULL pointer is as much a
    /// programming error here as it is there.
    #[inline(always)]
    pub unsafe fn malloc(&self, size: usize) -> *mut c_void {
        (self.malloc.unwrap_unchecked())(self.ctx, size)
    }
    #[inline(always)]
    pub unsafe fn realloc(&self, ptr: *mut c_void, old_size: usize, size: usize) -> *mut c_void {
        (self.realloc.unwrap_unchecked())(self.ctx, ptr, old_size, size)
    }
    #[inline(always)]
    pub unsafe fn free(&self, ptr: *mut c_void) {
        (self.free.unwrap_unchecked())(self.ctx, ptr)
    }
    pub const ZERO: yyjson_alc = yyjson_alc {
        malloc: None,
        realloc: None,
        free: None,
        ctx: core::ptr::null_mut(),
    };
}

/* ------------------------------------------------------------ value & document */

#[repr(C)]
#[derive(Copy, Clone)]
pub union yyjson_val_uni {
    pub u64_: u64,
    pub i64_: i64,
    pub f64_: f64,
    pub str_: *const c_char,
    pub ptr: *mut c_void,
    pub ofs: usize,
}

impl yyjson_val_uni {
    pub const ZERO: yyjson_val_uni = yyjson_val_uni { u64_: 0 };
}

/// Immutable JSON value, 16 bytes.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yyjson_val {
    pub tag: u64,
    pub uni: yyjson_val_uni,
}

#[repr(C)]
pub struct yyjson_doc {
    pub root: *mut yyjson_val,
    pub alc: yyjson_alc,
    pub dat_read: usize,
    pub val_read: usize,
    pub str_pool: *mut c_char,
}

/// Mutable JSON value, 24 bytes.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct yyjson_mut_val {
    pub tag: u64,
    pub uni: yyjson_val_uni,
    pub next: *mut yyjson_mut_val,
}

#[repr(C)]
pub struct yyjson_str_chunk {
    pub next: *mut yyjson_str_chunk,
    pub chunk_size: usize,
}

#[repr(C)]
pub struct yyjson_str_pool {
    pub cur: *mut c_char,
    pub end: *mut c_char,
    pub chunk_size: usize,
    pub chunk_size_max: usize,
    pub chunks: *mut yyjson_str_chunk,
}

#[repr(C)]
pub struct yyjson_val_chunk {
    pub next: *mut yyjson_val_chunk,
    pub chunk_size: usize,
}

#[repr(C)]
pub struct yyjson_val_pool {
    pub cur: *mut yyjson_mut_val,
    pub end: *mut yyjson_mut_val,
    pub chunk_size: usize,
    pub chunk_size_max: usize,
    pub chunks: *mut yyjson_val_chunk,
}

#[repr(C)]
pub struct yyjson_mut_doc {
    pub root: *mut yyjson_mut_val,
    pub alc: yyjson_alc,
    pub str_pool: yyjson_str_pool,
    pub val_pool: yyjson_val_pool,
}

/* ----------------------------------------------------------------- iterators */

#[repr(C)]
pub struct yyjson_arr_iter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut yyjson_val,
}

#[repr(C)]
pub struct yyjson_obj_iter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut yyjson_val,
    pub obj: *mut yyjson_val,
}

#[repr(C)]
pub struct yyjson_mut_arr_iter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut yyjson_mut_val,
    pub pre: *mut yyjson_mut_val,
    pub arr: *mut yyjson_mut_val,
}

#[repr(C)]
pub struct yyjson_mut_obj_iter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut yyjson_mut_val,
    pub pre: *mut yyjson_mut_val,
    pub obj: *mut yyjson_mut_val,
}

/* ------------------------------------------------------------- read/write err */

pub type yyjson_read_code = u32;
pub const YYJSON_READ_SUCCESS: u32 = 0;
pub const YYJSON_READ_ERROR_INVALID_PARAMETER: u32 = 1;
pub const YYJSON_READ_ERROR_MEMORY_ALLOCATION: u32 = 2;
pub const YYJSON_READ_ERROR_EMPTY_CONTENT: u32 = 3;
pub const YYJSON_READ_ERROR_UNEXPECTED_CONTENT: u32 = 4;
pub const YYJSON_READ_ERROR_UNEXPECTED_END: u32 = 5;
pub const YYJSON_READ_ERROR_UNEXPECTED_CHARACTER: u32 = 6;
pub const YYJSON_READ_ERROR_JSON_STRUCTURE: u32 = 7;
pub const YYJSON_READ_ERROR_INVALID_COMMENT: u32 = 8;
pub const YYJSON_READ_ERROR_INVALID_NUMBER: u32 = 9;
pub const YYJSON_READ_ERROR_INVALID_STRING: u32 = 10;
pub const YYJSON_READ_ERROR_LITERAL: u32 = 11;
pub const YYJSON_READ_ERROR_FILE_OPEN: u32 = 12;
pub const YYJSON_READ_ERROR_FILE_READ: u32 = 13;
pub const YYJSON_READ_ERROR_MORE: u32 = 14;

#[repr(C)]
pub struct yyjson_read_err {
    pub code: yyjson_read_code,
    pub msg: *const c_char,
    pub pos: usize,
}

pub type yyjson_write_code = u32;
pub const YYJSON_WRITE_SUCCESS: u32 = 0;
pub const YYJSON_WRITE_ERROR_INVALID_PARAMETER: u32 = 1;
pub const YYJSON_WRITE_ERROR_MEMORY_ALLOCATION: u32 = 2;
pub const YYJSON_WRITE_ERROR_INVALID_VALUE_TYPE: u32 = 3;
pub const YYJSON_WRITE_ERROR_NAN_OR_INF: u32 = 4;
pub const YYJSON_WRITE_ERROR_FILE_OPEN: u32 = 5;
pub const YYJSON_WRITE_ERROR_FILE_WRITE: u32 = 6;
pub const YYJSON_WRITE_ERROR_INVALID_STRING: u32 = 7;

#[repr(C)]
pub struct yyjson_write_err {
    pub code: yyjson_write_code,
    pub msg: *const c_char,
}

/* ------------------------------------------------------------------- pointer */

pub type yyjson_ptr_code = u32;
pub const YYJSON_PTR_ERR_NONE: u32 = 0;
pub const YYJSON_PTR_ERR_PARAMETER: u32 = 1;
pub const YYJSON_PTR_ERR_SYNTAX: u32 = 2;
pub const YYJSON_PTR_ERR_RESOLVE: u32 = 3;
pub const YYJSON_PTR_ERR_NULL_ROOT: u32 = 4;
pub const YYJSON_PTR_ERR_SET_ROOT: u32 = 5;
pub const YYJSON_PTR_ERR_MEMORY_ALLOCATION: u32 = 6;

#[repr(C)]
pub struct yyjson_ptr_err {
    pub code: yyjson_ptr_code,
    pub msg: *const c_char,
    pub pos: usize,
}

#[repr(C)]
pub struct yyjson_ptr_ctx {
    pub ctn: *mut yyjson_mut_val,
    pub pre: *mut yyjson_mut_val,
    pub old: *mut yyjson_mut_val,
}

pub type yyjson_patch_code = u32;
pub const YYJSON_PATCH_SUCCESS: u32 = 0;
pub const YYJSON_PATCH_ERROR_INVALID_PARAMETER: u32 = 1;
pub const YYJSON_PATCH_ERROR_MEMORY_ALLOCATION: u32 = 2;
pub const YYJSON_PATCH_ERROR_INVALID_OPERATION: u32 = 3;
pub const YYJSON_PATCH_ERROR_MISSING_KEY: u32 = 4;
pub const YYJSON_PATCH_ERROR_INVALID_MEMBER: u32 = 5;
pub const YYJSON_PATCH_ERROR_EQUAL: u32 = 6;
pub const YYJSON_PATCH_ERROR_POINTER: u32 = 7;

#[repr(C)]
pub struct yyjson_patch_err {
    pub code: yyjson_patch_code,
    pub idx: usize,
    pub msg: *const c_char,
    pub ptr: yyjson_ptr_err,
}

/* --------------------------------------------------------------- read flags */

pub type yyjson_read_flag = u32;
pub const YYJSON_READ_NOFLAG: u32 = 0;
pub const YYJSON_READ_INSITU: u32 = 1 << 0;
pub const YYJSON_READ_STOP_WHEN_DONE: u32 = 1 << 1;
pub const YYJSON_READ_ALLOW_TRAILING_COMMAS: u32 = 1 << 2;
pub const YYJSON_READ_ALLOW_COMMENTS: u32 = 1 << 3;
pub const YYJSON_READ_ALLOW_INF_AND_NAN: u32 = 1 << 4;
pub const YYJSON_READ_NUMBER_AS_RAW: u32 = 1 << 5;
pub const YYJSON_READ_ALLOW_INVALID_UNICODE: u32 = 1 << 6;
pub const YYJSON_READ_BIGNUM_AS_RAW: u32 = 1 << 7;
pub const YYJSON_READ_ALLOW_BOM: u32 = 1 << 8;
pub const YYJSON_READ_ALLOW_EXT_NUMBER: u32 = 1 << 9;
pub const YYJSON_READ_ALLOW_EXT_ESCAPE: u32 = 1 << 10;
pub const YYJSON_READ_ALLOW_EXT_WHITESPACE: u32 = 1 << 11;
pub const YYJSON_READ_ALLOW_SINGLE_QUOTED_STR: u32 = 1 << 12;
pub const YYJSON_READ_ALLOW_UNQUOTED_KEY: u32 = 1 << 13;
pub const YYJSON_READ_JSON5: u32 = YYJSON_READ_ALLOW_TRAILING_COMMAS
    | YYJSON_READ_ALLOW_COMMENTS
    | YYJSON_READ_ALLOW_INF_AND_NAN
    | YYJSON_READ_ALLOW_EXT_ESCAPE
    | YYJSON_READ_ALLOW_SINGLE_QUOTED_STR
    | YYJSON_READ_ALLOW_EXT_NUMBER
    | YYJSON_READ_ALLOW_EXT_WHITESPACE
    | YYJSON_READ_ALLOW_UNQUOTED_KEY;

/* -------------------------------------------------------------- write flags */

pub type yyjson_write_flag = u32;
pub const YYJSON_WRITE_NOFLAG: u32 = 0;
pub const YYJSON_WRITE_PRETTY: u32 = 1 << 0;
pub const YYJSON_WRITE_ESCAPE_UNICODE: u32 = 1 << 1;
pub const YYJSON_WRITE_ESCAPE_SLASHES: u32 = 1 << 2;
pub const YYJSON_WRITE_ALLOW_INF_AND_NAN: u32 = 1 << 3;
pub const YYJSON_WRITE_INF_AND_NAN_AS_NULL: u32 = 1 << 4;
pub const YYJSON_WRITE_ALLOW_INVALID_UNICODE: u32 = 1 << 5;
pub const YYJSON_WRITE_PRETTY_TWO_SPACES: u32 = 1 << 6;
pub const YYJSON_WRITE_NEWLINE_AT_END: u32 = 1 << 7;
pub const YYJSON_WRITE_LOWERCASE_HEX: u32 = 1 << 8;

/// The highest 8 bits of `yyjson_write_flag` and of a real value's `tag`
/// control the floating-point output format.
pub const YYJSON_WRITE_FP_FLAG_BITS: u32 = 8;
/// The highest 4 bits of those hold the fixed-point precision.
pub const YYJSON_WRITE_FP_PREC_BITS: u32 = 4;
pub const YYJSON_WRITE_FP_TO_FLOAT: u32 = 1 << (32 - 5);

#[inline(always)]
pub const fn yyjson_write_fp_to_fixed(prec: u32) -> u32 {
    prec << (32 - 4)
}

/* ---------------------------------------------- unsafe value accessors (C) */

#[inline(always)]
pub unsafe fn unsafe_get_type(val: *const yyjson_val) -> u8 {
    ((*val).tag as u8) & YYJSON_TYPE_MASK
}

#[inline(always)]
pub unsafe fn unsafe_get_subtype(val: *const yyjson_val) -> u8 {
    ((*val).tag as u8) & YYJSON_SUBTYPE_MASK
}

#[inline(always)]
pub unsafe fn unsafe_get_tag(val: *const yyjson_val) -> u8 {
    (*val).tag as u8
}

#[inline(always)]
pub unsafe fn unsafe_get_len(val: *const yyjson_val) -> usize {
    ((*val).tag >> YYJSON_TAG_BIT) as usize
}

#[inline(always)]
pub unsafe fn unsafe_set_len(val: *mut yyjson_val, len: usize) {
    let tag = (*val).tag & (YYJSON_TAG_MASK as u64);
    (*val).tag = tag | ((len as u64) << YYJSON_TAG_BIT);
}

#[inline(always)]
pub unsafe fn unsafe_set_tag(val: *mut yyjson_val, ty: u8, subtype: u8, len: usize) {
    let mut tag = (len as u64) << YYJSON_TAG_BIT;
    tag |= (ty | subtype) as u64;
    (*val).tag = tag;
}

#[inline(always)]
pub unsafe fn unsafe_is_ctn(val: *const yyjson_val) -> bool {
    let mask = YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ;
    (unsafe_get_tag(val) & mask) == mask
}

/// `unsafe_yyjson_get_first`: the first child of an immutable container.
#[inline(always)]
pub unsafe fn unsafe_get_first(ctn: *const yyjson_val) -> *mut yyjson_val {
    (ctn as *mut yyjson_val).add(1)
}

/// `unsafe_yyjson_get_next`: the next sibling of an immutable value.
#[inline(always)]
pub unsafe fn unsafe_get_next(val: *const yyjson_val) -> *mut yyjson_val {
    let is_ctn = unsafe_is_ctn(val);
    let ctn_ofs = (*val).uni.ofs;
    let ofs = if is_ctn {
        ctn_ofs
    } else {
        core::mem::size_of::<yyjson_val>()
    };
    (val as *mut u8).add(ofs) as *mut yyjson_val
}

#[inline(always)]
pub unsafe fn unsafe_equals_strn(val: *const yyjson_val, str_: *const c_char, len: usize) -> bool {
    unsafe_get_len(val) == len && crate::mem::memcmp_eq((*val).uni.str_ as *const u8, str_ as *const u8, len)
}

/* the same accessors for mutable values (identical prefix layout) */

#[inline(always)]
pub unsafe fn mut_get_type(val: *const yyjson_mut_val) -> u8 {
    unsafe_get_type(val as *const yyjson_val)
}

#[inline(always)]
pub unsafe fn mut_get_len(val: *const yyjson_mut_val) -> usize {
    unsafe_get_len(val as *const yyjson_val)
}

#[inline(always)]
pub unsafe fn mut_set_len(val: *mut yyjson_mut_val, len: usize) {
    unsafe_set_len(val as *mut yyjson_val, len)
}

#[inline(always)]
pub unsafe fn mut_get_tag(val: *const yyjson_mut_val) -> u8 {
    unsafe_get_tag(val as *const yyjson_val)
}

#[inline(always)]
pub unsafe fn mut_get_subtype(val: *const yyjson_mut_val) -> u8 {
    unsafe_get_subtype(val as *const yyjson_val)
}

/* `FILE *` is opaque to us. */
pub type FILE = c_void;

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(file: *mut FILE) -> c_int;
    pub fn fread(buf: *mut c_void, size: usize, count: usize, file: *mut FILE) -> usize;
    pub fn fwrite(buf: *const c_void, size: usize, count: usize, file: *mut FILE) -> usize;
    pub fn fseek(file: *mut FILE, offset: core::ffi::c_long, whence: c_int) -> c_int;
    pub fn ftell(file: *mut FILE) -> core::ffi::c_long;
    pub fn ferror(file: *mut FILE) -> c_int;
}

pub const SEEK_SET: c_int = 0;
pub const SEEK_END: c_int = 2;
