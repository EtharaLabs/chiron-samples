//! ABI types and bit-level value helpers.
//!
//! Every type here is `#[repr(C)]` and laid out exactly like the corresponding
//! C declaration in `src/yyjson.h`, because the header's `static inline`
//! functions are compiled into user code and poke at these fields directly.

use core::ffi::{c_char, c_void};

pub type Usize = usize;

/* ===== tag bits (yyjson.h, "JSON Types") ================================= */

pub const YYJSON_TYPE_NONE: u8 = 0;
pub const YYJSON_TYPE_RAW: u8 = 1;
pub const YYJSON_TYPE_NULL: u8 = 2;
pub const YYJSON_TYPE_BOOL: u8 = 3;
pub const YYJSON_TYPE_NUM: u8 = 4;
pub const YYJSON_TYPE_STR: u8 = 5;
pub const YYJSON_TYPE_ARR: u8 = 6;
pub const YYJSON_TYPE_OBJ: u8 = 7;

pub const YYJSON_SUBTYPE_NONE: u8 = 0;
pub const YYJSON_SUBTYPE_FALSE: u8 = 0;
pub const YYJSON_SUBTYPE_TRUE: u8 = 1 << 3;
pub const YYJSON_SUBTYPE_UINT: u8 = 0;
pub const YYJSON_SUBTYPE_SINT: u8 = 1 << 3;
pub const YYJSON_SUBTYPE_REAL: u8 = 2 << 3;
pub const YYJSON_SUBTYPE_NOESC: u8 = 1 << 3;

pub const YYJSON_TYPE_MASK: u8 = 0x07;
pub const YYJSON_TYPE_BIT: u32 = 3;
pub const YYJSON_SUBTYPE_MASK: u8 = 0x18;
pub const YYJSON_TAG_MASK: u8 = 0xFF;
pub const YYJSON_TAG_BIT: u32 = 8;

pub const YYJSON_PADDING_SIZE: usize = 4;

/* ===== read flags ======================================================== */

pub type YyjsonReadFlag = u32;

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
pub const YYJSON_READ_JSON5: u32 = (1 << 2)
    | (1 << 3)
    | (1 << 4)
    | (1 << 9)
    | (1 << 10)
    | (1 << 11)
    | (1 << 12)
    | (1 << 13);
/// `YYJSON_READ_ALLOW_TRIVIA` from `yyjson.c`: comments or extended whitespace.
pub const YYJSON_READ_ALLOW_TRIVIA: u32 =
    YYJSON_READ_ALLOW_COMMENTS | YYJSON_READ_ALLOW_EXT_WHITESPACE;

/* ===== read error codes ================================================== */

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
pub const YYJSON_READ_ERROR_DEPTH: u32 = 15;

/* ===== write flags ======================================================= */

pub type YyjsonWriteFlag = u32;

pub const YYJSON_WRITE_PRETTY: u32 = 1 << 0;
pub const YYJSON_WRITE_ESCAPE_UNICODE: u32 = 1 << 1;
pub const YYJSON_WRITE_ESCAPE_SLASHES: u32 = 1 << 2;
pub const YYJSON_WRITE_ALLOW_INF_AND_NAN: u32 = 1 << 3;
pub const YYJSON_WRITE_INF_AND_NAN_AS_NULL: u32 = 1 << 4;
pub const YYJSON_WRITE_ALLOW_INVALID_UNICODE: u32 = 1 << 5;
pub const YYJSON_WRITE_PRETTY_TWO_SPACES: u32 = 1 << 6;
pub const YYJSON_WRITE_NEWLINE_AT_END: u32 = 1 << 7;
pub const YYJSON_WRITE_LOWERCASE_HEX: u32 = 1 << 8;
pub const YYJSON_WRITE_FP_FLAG_BITS: u32 = 8;
pub const YYJSON_WRITE_FP_PREC_BITS: u32 = 4;
pub const YYJSON_WRITE_FP_TO_FLOAT: u32 = 1 << (32 - 5);

pub const YYJSON_WRITE_SUCCESS: u32 = 0;
pub const YYJSON_WRITE_ERROR_INVALID_PARAMETER: u32 = 1;
pub const YYJSON_WRITE_ERROR_MEMORY_ALLOCATION: u32 = 2;
pub const YYJSON_WRITE_ERROR_INVALID_VALUE_TYPE: u32 = 3;
pub const YYJSON_WRITE_ERROR_NAN_OR_INF: u32 = 4;
pub const YYJSON_WRITE_ERROR_FILE_OPEN: u32 = 5;
pub const YYJSON_WRITE_ERROR_FILE_WRITE: u32 = 6;
pub const YYJSON_WRITE_ERROR_INVALID_STRING: u32 = 7;
pub const YYJSON_WRITE_ERROR_DEPTH: u32 = 8;

/* ===== JSON pointer / patch codes ======================================== */

pub const YYJSON_PTR_ERR_NONE: u32 = 0;
pub const YYJSON_PTR_ERR_PARAMETER: u32 = 1;
pub const YYJSON_PTR_ERR_SYNTAX: u32 = 2;
pub const YYJSON_PTR_ERR_RESOLVE: u32 = 3;
pub const YYJSON_PTR_ERR_NULL_ROOT: u32 = 4;
pub const YYJSON_PTR_ERR_SET_ROOT: u32 = 5;
pub const YYJSON_PTR_ERR_MEMORY_ALLOCATION: u32 = 6;

pub const YYJSON_PATCH_SUCCESS: u32 = 0;
pub const YYJSON_PATCH_ERROR_INVALID_PARAMETER: u32 = 1;
pub const YYJSON_PATCH_ERROR_MEMORY_ALLOCATION: u32 = 2;
pub const YYJSON_PATCH_ERROR_INVALID_OPERATION: u32 = 3;
pub const YYJSON_PATCH_ERROR_MISSING_KEY: u32 = 4;
pub const YYJSON_PATCH_ERROR_INVALID_MEMBER: u32 = 5;
pub const YYJSON_PATCH_ERROR_EQUAL: u32 = 6;
pub const YYJSON_PATCH_ERROR_POINTER: u32 = 7;

/* ===== structures ======================================================== */

pub type AlcMalloc = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type AlcRealloc =
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> *mut c_void>;
pub type AlcFree = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

/// `yyjson_alc`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct YyjsonAlc {
    pub malloc: AlcMalloc,
    pub realloc: AlcRealloc,
    pub free: AlcFree,
    pub ctx: *mut c_void,
}

/// The allocator vtables below are immutable statics; the `ctx` pointer in
/// them is always null, so sharing them across threads is sound.
unsafe impl Sync for YyjsonAlc {}

impl YyjsonAlc {
    pub const ZERO: YyjsonAlc = YyjsonAlc {
        malloc: None,
        realloc: None,
        free: None,
        ctx: core::ptr::null_mut(),
    };

    #[inline(always)]
    pub unsafe fn alloc(&self, size: usize) -> *mut c_void {
        (self.malloc.unwrap_unchecked())(self.ctx, size)
    }
    #[inline(always)]
    pub unsafe fn realloc(&self, ptr: *mut c_void, old: usize, new: usize) -> *mut c_void {
        (self.realloc.unwrap_unchecked())(self.ctx, ptr, old, new)
    }
    #[inline(always)]
    pub unsafe fn dealloc(&self, ptr: *mut c_void) {
        (self.free.unwrap_unchecked())(self.ctx, ptr)
    }
}

/// `yyjson_val_uni`
#[repr(C)]
#[derive(Clone, Copy)]
pub union YyjsonValUni {
    pub u64_: u64,
    pub i64_: i64,
    pub f64_: f64,
    pub str_: *const c_char,
    pub ptr: *mut c_void,
    pub ofs: usize,
}

/// `yyjson_val` (16 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct YyjsonVal {
    pub tag: u64,
    pub uni: YyjsonValUni,
}

/// `yyjson_doc`
#[repr(C)]
pub struct YyjsonDoc {
    pub root: *mut YyjsonVal,
    pub alc: YyjsonAlc,
    pub dat_read: usize,
    pub val_read: usize,
    pub str_pool: *mut c_char,
}

/// `yyjson_mut_val` (24 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct YyjsonMutVal {
    pub tag: u64,
    pub uni: YyjsonValUni,
    pub next: *mut YyjsonMutVal,
}

/// `yyjson_str_chunk`
#[repr(C)]
pub struct YyjsonStrChunk {
    pub next: *mut YyjsonStrChunk,
    pub chunk_size: usize,
}

/// `yyjson_str_pool`
#[repr(C)]
pub struct YyjsonStrPool {
    pub cur: *mut c_char,
    pub end: *mut c_char,
    pub chunk_size: usize,
    pub chunk_size_max: usize,
    pub chunks: *mut YyjsonStrChunk,
}

/// `yyjson_val_chunk`
#[repr(C)]
pub struct YyjsonValChunk {
    pub next: *mut YyjsonValChunk,
    pub chunk_size: usize,
}

/// `yyjson_val_pool`
#[repr(C)]
pub struct YyjsonValPool {
    pub cur: *mut YyjsonMutVal,
    pub end: *mut YyjsonMutVal,
    pub chunk_size: usize,
    pub chunk_size_max: usize,
    pub chunks: *mut YyjsonValChunk,
}

/// `yyjson_mut_doc`
#[repr(C)]
pub struct YyjsonMutDoc {
    pub root: *mut YyjsonMutVal,
    pub alc: YyjsonAlc,
    pub str_pool: YyjsonStrPool,
    pub val_pool: YyjsonValPool,
}

/// `yyjson_read_err`
#[repr(C)]
pub struct YyjsonReadErr {
    pub code: u32,
    pub msg: *const c_char,
    pub pos: usize,
}

/// `yyjson_write_err`
#[repr(C)]
pub struct YyjsonWriteErr {
    pub code: u32,
    pub msg: *const c_char,
}

/// `yyjson_arr_iter`
#[repr(C)]
pub struct YyjsonArrIter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut YyjsonVal,
}

/// `yyjson_obj_iter`
#[repr(C)]
pub struct YyjsonObjIter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut YyjsonVal,
    pub obj: *mut YyjsonVal,
}

/// `yyjson_mut_arr_iter`
#[repr(C)]
pub struct YyjsonMutArrIter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut YyjsonMutVal,
    pub pre: *mut YyjsonMutVal,
    pub arr: *mut YyjsonMutVal,
}

/// `yyjson_mut_obj_iter`
#[repr(C)]
pub struct YyjsonMutObjIter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut YyjsonMutVal,
    pub pre: *mut YyjsonMutVal,
    pub obj: *mut YyjsonMutVal,
}

/// `yyjson_ptr_err`
#[repr(C)]
pub struct YyjsonPtrErr {
    pub code: u32,
    pub msg: *const c_char,
    pub pos: usize,
}

/// `yyjson_ptr_ctx`
#[repr(C)]
pub struct YyjsonPtrCtx {
    pub ctn: *mut YyjsonMutVal,
    pub pre: *mut YyjsonMutVal,
    pub old: *mut YyjsonMutVal,
}

/// `yyjson_patch_err`
#[repr(C)]
pub struct YyjsonPatchErr {
    pub code: u32,
    pub idx: usize,
    pub msg: *const c_char,
    pub ptr: YyjsonPtrErr,
}

/* ===== tag accessors (mirror of the header's `unsafe_yyjson_*`) ========== */

/// Reads the type bits of a value. `val` must point to a live `yyjson_val`
/// or `yyjson_mut_val`; both start with the same `tag` field.
#[inline(always)]
pub unsafe fn get_type(val: *const YyjsonVal) -> u8 {
    ((*val).tag as u8) & YYJSON_TYPE_MASK
}

#[inline(always)]
pub unsafe fn get_subtype(val: *const YyjsonVal) -> u8 {
    ((*val).tag as u8) & YYJSON_SUBTYPE_MASK
}

#[inline(always)]
pub unsafe fn get_tag(val: *const YyjsonVal) -> u8 {
    (*val).tag as u8
}

#[inline(always)]
pub unsafe fn get_len(val: *const YyjsonVal) -> usize {
    ((*val).tag >> YYJSON_TAG_BIT) as usize
}

#[inline(always)]
pub unsafe fn set_len(val: *mut YyjsonVal, len: usize) {
    let tag = (*val).tag & (YYJSON_TAG_MASK as u64);
    (*val).tag = tag | ((len as u64) << YYJSON_TAG_BIT);
}

#[inline(always)]
pub unsafe fn inc_len(val: *mut YyjsonVal) {
    (*val).tag = (*val).tag.wrapping_add(1u64 << YYJSON_TAG_BIT);
}

#[inline(always)]
pub unsafe fn is_ctn(val: *const YyjsonVal) -> bool {
    let mask = YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ;
    (get_tag(val) & mask) == mask
}

#[inline(always)]
pub unsafe fn get_bool(val: *const YyjsonVal) -> bool {
    (get_tag(val) & YYJSON_SUBTYPE_MASK) >> YYJSON_TYPE_BIT != 0
}

#[inline(always)]
pub unsafe fn get_str(val: *const YyjsonVal) -> *const c_char {
    (*val).uni.str_
}

/// `unsafe_yyjson_get_first`: the first child of an immutable container.
#[inline(always)]
pub unsafe fn get_first(ctn: *const YyjsonVal) -> *mut YyjsonVal {
    (ctn as *mut YyjsonVal).add(1)
}

/// `unsafe_yyjson_get_next`: the next sibling of an immutable value.
#[inline(always)]
pub unsafe fn get_next(val: *const YyjsonVal) -> *mut YyjsonVal {
    let is_ctn = is_ctn(val);
    let ctn_ofs = (*val).uni.ofs;
    let ofs = if is_ctn {
        ctn_ofs
    } else {
        core::mem::size_of::<YyjsonVal>()
    };
    (val as *mut u8).add(ofs) as *mut YyjsonVal
}

#[inline(always)]
pub unsafe fn arr_is_flat(val: *const YyjsonVal) -> bool {
    let ofs = (*val).uni.ofs;
    let len = get_len(val);
    len * core::mem::size_of::<YyjsonVal>() + core::mem::size_of::<YyjsonVal>() == ofs
}

#[inline(always)]
pub unsafe fn equals_strn(val: *const YyjsonVal, str_: *const c_char, len: usize) -> bool {
    get_len(val) == len && crate::libc::memcmp_eq((*val).uni.str_ as *const u8, str_ as *const u8, len)
}

#[inline(always)]
pub unsafe fn mut_as_val(val: *const YyjsonMutVal) -> *mut YyjsonVal {
    val as *mut YyjsonVal
}
