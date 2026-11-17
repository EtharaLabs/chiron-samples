// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 * MARK: - Public types, flags, codes and constants (from yyjson.h)
 *
 * Every typedef, struct, union, constant group, flag, code, mask and helper
 * macro that yyjson.h exposes, with the C spelling preserved. Layouts are
 * `#[repr(C)]` and are checked at compile time against LP64 Linux.
 *
 * The `static const yyjson_*_flag` / `yyjson_*_code` objects of the header are
 * value constants here (CONVENTIONS: "Value macros become pub(crate) const with
 * the same name and the C type"); they have internal linkage in C, so they are
 * not symbols of the library.
 *============================================================================*/

use crate::sys::abi::{FILE, c_char, c_void};

/*==============================================================================
 * MARK: - Private type aliases (yyjson.c:531-561)
 *============================================================================*/

/** Type aliases for primitive types. */
pub(crate) type f32_ = f32;
pub(crate) type f64_ = f64;

/* yyjson.c's `f32 f64 i8 u8 i16 u16 i32 u32 i64 u64 usize` map onto the Rust
   primitives of the same name, so no alias is needed for them. `i128`/`u128`
   likewise map onto Rust's 128-bit integers (YYJSON_HAS_INT128 is 1). */

/*==============================================================================
 * MARK: - Version (yyjson.h:578-593)
 *============================================================================*/

/** The major version of yyjson. */
pub(crate) const YYJSON_VERSION_MAJOR: u32 = 0;
/** The minor version of yyjson. */
pub(crate) const YYJSON_VERSION_MINOR: u32 = 13;
/** The patch version of yyjson. */
pub(crate) const YYJSON_VERSION_PATCH: u32 = 0;
/** The version of yyjson in hex: `(major << 16) | (minor << 8) | (patch)`. */
pub(crate) const YYJSON_VERSION_HEX: u32 = 0x000D00;
/** The version string of yyjson, NUL-terminated like the C literal. */
pub(crate) const YYJSON_VERSION_STRING: &[u8; 7] = b"0.13.0\0";

/*==============================================================================
 * MARK: - JSON Types (yyjson.h:601-655)
 *============================================================================*/

/** Type of a JSON value (3 bit). */
pub(crate) type yyjson_type = u8;
/** No type, invalid. */
pub(crate) const YYJSON_TYPE_NONE: u8 = 0; /* _____000 */
/** Raw string type, no subtype. */
pub(crate) const YYJSON_TYPE_RAW: u8 = 1; /* _____001 */
/** Null type: `null` literal, no subtype. */
pub(crate) const YYJSON_TYPE_NULL: u8 = 2; /* _____010 */
/** Boolean type, subtype: TRUE, FALSE. */
pub(crate) const YYJSON_TYPE_BOOL: u8 = 3; /* _____011 */
/** Number type, subtype: UINT, SINT, REAL. */
pub(crate) const YYJSON_TYPE_NUM: u8 = 4; /* _____100 */
/** String type, subtype: NONE, NOESC. */
pub(crate) const YYJSON_TYPE_STR: u8 = 5; /* _____101 */
/** Array type, no subtype. */
pub(crate) const YYJSON_TYPE_ARR: u8 = 6; /* _____110 */
/** Object type, no subtype. */
pub(crate) const YYJSON_TYPE_OBJ: u8 = 7; /* _____111 */

/** Subtype of a JSON value (2 bit). */
pub(crate) type yyjson_subtype = u8;
/** No subtype. */
pub(crate) const YYJSON_SUBTYPE_NONE: u8 = 0 << 3; /* ___00___ */
/** False subtype: `false` literal. */
pub(crate) const YYJSON_SUBTYPE_FALSE: u8 = 0 << 3; /* ___00___ */
/** True subtype: `true` literal. */
pub(crate) const YYJSON_SUBTYPE_TRUE: u8 = 1 << 3; /* ___01___ */
/** Unsigned integer subtype: `uint64_t`. */
pub(crate) const YYJSON_SUBTYPE_UINT: u8 = 0 << 3; /* ___00___ */
/** Signed integer subtype: `int64_t`. */
pub(crate) const YYJSON_SUBTYPE_SINT: u8 = 1 << 3; /* ___01___ */
/** Real number subtype: `double`. */
pub(crate) const YYJSON_SUBTYPE_REAL: u8 = 2 << 3; /* ___10___ */
/** String that does not need to be escaped for writing (internal use). */
pub(crate) const YYJSON_SUBTYPE_NOESC: u8 = 1 << 3; /* ___01___ */

/** The mask used to extract the type of a JSON value. */
pub(crate) const YYJSON_TYPE_MASK: u8 = 0x07; /* _____111 */
/** The number of bits used by the type. */
pub(crate) const YYJSON_TYPE_BIT: u8 = 3;
/** The mask used to extract the subtype of a JSON value. */
pub(crate) const YYJSON_SUBTYPE_MASK: u8 = 0x18; /* ___11___ */
/** The number of bits used by the subtype. */
pub(crate) const YYJSON_SUBTYPE_BIT: u8 = 2;
/** The mask used to extract the reserved bits of a JSON value. */
pub(crate) const YYJSON_RESERVED_MASK: u8 = 0xE0; /* 111_____ */
/** The number of reserved bits. */
pub(crate) const YYJSON_RESERVED_BIT: u8 = 3;
/** The mask used to extract the tag of a JSON value. */
pub(crate) const YYJSON_TAG_MASK: u8 = 0xFF; /* 11111111 */
/** The number of bits used by the tag. */
pub(crate) const YYJSON_TAG_BIT: u8 = 8;

/** Padding size for JSON reader. */
pub(crate) const YYJSON_PADDING_SIZE: usize = 4;

/*==============================================================================
 * MARK: - Allocator (yyjson.h:669-678)
 *============================================================================*/

/**
 A memory allocator.

 All four members are plain function pointers in C and must stay nullable here,
 because `yyjson_doc_free` zeroes the embedded allocator of a document and
 because callers may hand in a partially-filled struct.
 */
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct yyjson_alc {
    /** Same as libc's malloc(size), should not be NULL. */
    pub malloc: Option<unsafe extern "C" fn(ctx: *mut c_void, size: usize)
        -> *mut c_void>,
    /** Same as libc's realloc(ptr, size), should not be NULL. */
    pub realloc: Option<unsafe extern "C" fn(ctx: *mut c_void,
                                             ptr: *mut c_void,
                                             old_size: usize,
                                             size: usize) -> *mut c_void>,
    /** Same as libc's free(ptr), should not be NULL. */
    pub free: Option<unsafe extern "C" fn(ctx: *mut c_void, ptr: *mut c_void)>,
    /** A context for malloc/realloc/free, can be NULL. */
    pub ctx: *mut c_void,
}

/*==============================================================================
 * MARK: - JSON Reader API (yyjson.h:806-972)
 *============================================================================*/

/** Run-time options for JSON reader. */
pub(crate) type yyjson_read_flag = u32;

/** Default option (RFC 8259 compliant). */
pub(crate) const YYJSON_READ_NOFLAG: yyjson_read_flag = 0;
/** Read the input data in-situ. */
pub(crate) const YYJSON_READ_INSITU: yyjson_read_flag = 1 << 0;
/** Stop when done instead of issuing an error on trailing content. */
pub(crate) const YYJSON_READ_STOP_WHEN_DONE: yyjson_read_flag = 1 << 1;
/** Allow single trailing comma at the end of an object or array. */
pub(crate) const YYJSON_READ_ALLOW_TRAILING_COMMAS: yyjson_read_flag = 1 << 2;
/** Allow C-style single-line and multi-line comments. */
pub(crate) const YYJSON_READ_ALLOW_COMMENTS: yyjson_read_flag = 1 << 3;
/** Allow inf/nan number and literal, case-insensitive. */
pub(crate) const YYJSON_READ_ALLOW_INF_AND_NAN: yyjson_read_flag = 1 << 4;
/** Read all numbers as raw strings. */
pub(crate) const YYJSON_READ_NUMBER_AS_RAW: yyjson_read_flag = 1 << 5;
/** Allow reading invalid unicode when parsing string values. */
pub(crate) const YYJSON_READ_ALLOW_INVALID_UNICODE: yyjson_read_flag = 1 << 6;
/** Read big numbers as raw strings. */
pub(crate) const YYJSON_READ_BIGNUM_AS_RAW: yyjson_read_flag = 1 << 7;
/** Allow UTF-8 BOM and skip it before parsing if any. */
pub(crate) const YYJSON_READ_ALLOW_BOM: yyjson_read_flag = 1 << 8;
/** Allow extended number formats (hex, leading/trailing dot, leading plus). */
pub(crate) const YYJSON_READ_ALLOW_EXT_NUMBER: yyjson_read_flag = 1 << 9;
/** Allow extended escape sequences in strings. */
pub(crate) const YYJSON_READ_ALLOW_EXT_ESCAPE: yyjson_read_flag = 1 << 10;
/** Allow extended whitespace characters. */
pub(crate) const YYJSON_READ_ALLOW_EXT_WHITESPACE: yyjson_read_flag = 1 << 11;
/** Allow strings enclosed in single quotes. */
pub(crate) const YYJSON_READ_ALLOW_SINGLE_QUOTED_STR: yyjson_read_flag = 1 << 12;
/** Allow object keys without quotes. */
pub(crate) const YYJSON_READ_ALLOW_UNQUOTED_KEY: yyjson_read_flag = 1 << 13;
/** Allow JSON5 format, see: [https://json5.org]. */
pub(crate) const YYJSON_READ_JSON5: yyjson_read_flag = (1 << 2)
    | (1 << 3)
    | (1 << 4)
    | (1 << 9)
    | (1 << 10)
    | (1 << 11)
    | (1 << 12)
    | (1 << 13);

/** Result code for JSON reader. */
pub(crate) type yyjson_read_code = u32;

/** Success, no error. */
pub(crate) const YYJSON_READ_SUCCESS: yyjson_read_code = 0;
/** Invalid parameter, such as NULL input string or 0 input length. */
pub(crate) const YYJSON_READ_ERROR_INVALID_PARAMETER: yyjson_read_code = 1;
/** Memory allocation failed. */
pub(crate) const YYJSON_READ_ERROR_MEMORY_ALLOCATION: yyjson_read_code = 2;
/** Input JSON string is empty. */
pub(crate) const YYJSON_READ_ERROR_EMPTY_CONTENT: yyjson_read_code = 3;
/** Unexpected content after document, such as `[123]abc`. */
pub(crate) const YYJSON_READ_ERROR_UNEXPECTED_CONTENT: yyjson_read_code = 4;
/** Unexpected end of input, the parsed part is valid, such as `[123`. */
pub(crate) const YYJSON_READ_ERROR_UNEXPECTED_END: yyjson_read_code = 5;
/** Unexpected character inside the document, such as `[abc]`. */
pub(crate) const YYJSON_READ_ERROR_UNEXPECTED_CHARACTER: yyjson_read_code = 6;
/** Invalid JSON structure, such as `[1,]`. */
pub(crate) const YYJSON_READ_ERROR_JSON_STRUCTURE: yyjson_read_code = 7;
/** Invalid comment, deprecated. */
pub(crate) const YYJSON_READ_ERROR_INVALID_COMMENT: yyjson_read_code = 8;
/** Invalid number, such as `123.e12`, `000`. */
pub(crate) const YYJSON_READ_ERROR_INVALID_NUMBER: yyjson_read_code = 9;
/** Invalid string, such as invalid escaped character inside a string. */
pub(crate) const YYJSON_READ_ERROR_INVALID_STRING: yyjson_read_code = 10;
/** Invalid JSON literal, such as `truu`. */
pub(crate) const YYJSON_READ_ERROR_LITERAL: yyjson_read_code = 11;
/** Failed to open a file. */
pub(crate) const YYJSON_READ_ERROR_FILE_OPEN: yyjson_read_code = 12;
/** Failed to read a file. */
pub(crate) const YYJSON_READ_ERROR_FILE_READ: yyjson_read_code = 13;
/** Incomplete input during incremental parsing. */
pub(crate) const YYJSON_READ_ERROR_MORE: yyjson_read_code = 14;
/** Read depth limit exceeded. */
pub(crate) const YYJSON_READ_ERROR_DEPTH: yyjson_read_code = 15;

/** Error information for JSON reader. */
#[repr(C)]
pub(crate) struct yyjson_read_err {
    /** Error code, see `yyjson_read_code` for all possible values. */
    pub code: yyjson_read_code,
    /** Error message, constant, no need to free (NULL if success). */
    pub msg: *const c_char,
    /** Error byte position for input data (0 if success). */
    pub pos: usize,
}

/*==============================================================================
 * MARK: - JSON Writer API (yyjson.h:1245-1349)
 *============================================================================*/

/** Run-time options for JSON writer. */
pub(crate) type yyjson_write_flag = u32;

/** Default option: minify, error on inf/nan, error on invalid UTF-8. */
pub(crate) const YYJSON_WRITE_NOFLAG: yyjson_write_flag = 0;
/** Write JSON pretty with 4 space indent. */
pub(crate) const YYJSON_WRITE_PRETTY: yyjson_write_flag = 1 << 0;
/** Escape unicode as `uXXXX`, make the output ASCII only. */
pub(crate) const YYJSON_WRITE_ESCAPE_UNICODE: yyjson_write_flag = 1 << 1;
/** Escape '/' as '\/'. */
pub(crate) const YYJSON_WRITE_ESCAPE_SLASHES: yyjson_write_flag = 1 << 2;
/** Write inf and nan number as 'Infinity' and 'NaN' literal. */
pub(crate) const YYJSON_WRITE_ALLOW_INF_AND_NAN: yyjson_write_flag = 1 << 3;
/** Write inf and nan number as null literal. */
pub(crate) const YYJSON_WRITE_INF_AND_NAN_AS_NULL: yyjson_write_flag = 1 << 4;
/** Allow invalid unicode when encoding string values. */
pub(crate) const YYJSON_WRITE_ALLOW_INVALID_UNICODE: yyjson_write_flag = 1 << 5;
/** Write JSON pretty with 2 space indent. */
pub(crate) const YYJSON_WRITE_PRETTY_TWO_SPACES: yyjson_write_flag = 1 << 6;
/** Adds a newline character `\n` at the end of the JSON. */
pub(crate) const YYJSON_WRITE_NEWLINE_AT_END: yyjson_write_flag = 1 << 7;
/** Use lowercase hex digits in `\uXXXX` escape sequences. */
pub(crate) const YYJSON_WRITE_LOWERCASE_HEX: yyjson_write_flag = 1 << 8;

/** The highest 8 bits of `yyjson_write_flag` and real number value's `tag`
    are reserved for controlling the output format of floating-point numbers. */
pub(crate) const YYJSON_WRITE_FP_FLAG_BITS: u32 = 8;
/** The highest 4 bits of flag are reserved for precision value. */
pub(crate) const YYJSON_WRITE_FP_PREC_BITS: u32 = 4;

/** Write floating-point number using fixed-point notation.
    `#define YYJSON_WRITE_FP_TO_FIXED(prec) ((yyjson_write_flag)( \
         (uint32_t)((uint32_t)(prec)) << (32 - 4) ))`
    The C macro takes an `int`; the shift is on `uint32_t` and the high bits of
    `prec` are simply shifted out, so `wrapping_shl` reproduces it exactly. */
#[inline(always)]
pub(crate) const fn YYJSON_WRITE_FP_TO_FIXED(prec: i32) -> yyjson_write_flag {
    (prec as u32).wrapping_shl(32 - 4)
}

/** Write floating-point numbers using single-precision (float).
    `#define YYJSON_WRITE_FP_TO_FLOAT ((yyjson_write_flag)(1 << (32 - 5)))` */
pub(crate) const YYJSON_WRITE_FP_TO_FLOAT: yyjson_write_flag = 1 << (32 - 5);

/** Result code for JSON writer */
pub(crate) type yyjson_write_code = u32;

/** Success, no error. */
pub(crate) const YYJSON_WRITE_SUCCESS: yyjson_write_code = 0;
/** Invalid parameter, such as NULL document. */
pub(crate) const YYJSON_WRITE_ERROR_INVALID_PARAMETER: yyjson_write_code = 1;
/** Memory allocation failed. */
pub(crate) const YYJSON_WRITE_ERROR_MEMORY_ALLOCATION: yyjson_write_code = 2;
/** Invalid value type in JSON document. */
pub(crate) const YYJSON_WRITE_ERROR_INVALID_VALUE_TYPE: yyjson_write_code = 3;
/** NaN or Infinity number occurs. */
pub(crate) const YYJSON_WRITE_ERROR_NAN_OR_INF: yyjson_write_code = 4;
/** Failed to open a file. */
pub(crate) const YYJSON_WRITE_ERROR_FILE_OPEN: yyjson_write_code = 5;
/** Failed to write a file. */
pub(crate) const YYJSON_WRITE_ERROR_FILE_WRITE: yyjson_write_code = 6;
/** Invalid unicode in string. */
pub(crate) const YYJSON_WRITE_ERROR_INVALID_STRING: yyjson_write_code = 7;
/** Nesting depth limit exceeded. */
pub(crate) const YYJSON_WRITE_ERROR_DEPTH: yyjson_write_code = 8;

/** Error information for JSON writer. */
#[repr(C)]
pub(crate) struct yyjson_write_err {
    /** Error code, see `yyjson_write_code` for all possible values. */
    pub code: yyjson_write_code,
    /** Error message, constant, no need to free (NULL if success). */
    pub msg: *const c_char,
}

/*==============================================================================
 * MARK: - JSON Structure (yyjson.h:4968-4996, 5807-5868)
 *============================================================================*/

/** Payload of a JSON value (8 bytes).
    A C union of `u64 / i64 / f64 / const char * / void * / size_t`; all six
    members are 8 bytes on LP64, so a single `u64` cell with typed accessors
    reproduces the layout and every read/write exactly. */
#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) union yyjson_val_uni {
    pub u64_: u64,
    pub i64_: i64,
    pub f64_: f64,
    pub str_: *const c_char,
    pub ptr: *mut c_void,
    pub ofs: usize,
}

/** Immutable JSON value, 16 bytes. */
#[repr(C)]
pub(crate) struct yyjson_val {
    /** type, subtype and length */
    pub tag: u64,
    /** payload */
    pub uni: yyjson_val_uni,
}

/** An immutable document for reading JSON. */
#[repr(C)]
pub(crate) struct yyjson_doc {
    /** Root value of the document (nonnull). */
    pub root: *mut yyjson_val,
    /** Allocator used by document (nonnull). */
    pub alc: yyjson_alc,
    /** The total number of bytes read when parsing JSON (nonzero). */
    pub dat_read: usize,
    /** The total number of values read when parsing JSON (nonzero). */
    pub val_read: usize,
    /** The string pool used by JSON values (nullable). */
    pub str_pool: *mut c_char,
}

/** Mutable JSON value, 24 bytes.
    The 'tag' and 'uni' fields are the same as immutable value.
    The 'next' field links all elements inside the container to be a cycle. */
#[repr(C)]
pub(crate) struct yyjson_mut_val {
    /** type, subtype and length */
    pub tag: u64,
    /** payload */
    pub uni: yyjson_val_uni,
    /** the next value in circular linked list */
    pub next: *mut yyjson_mut_val,
}

/** A memory chunk in string memory pool. */
#[repr(C)]
pub(crate) struct yyjson_str_chunk {
    /** next chunk linked list */
    pub next: *mut yyjson_str_chunk,
    /** chunk size in bytes */
    pub chunk_size: usize,
    /* char str[]; flexible array member */
}

/** A memory pool to hold all strings in a mutable document. */
#[repr(C)]
pub(crate) struct yyjson_str_pool {
    /** cursor inside current chunk */
    pub cur: *mut c_char,
    /** the end of current chunk */
    pub end: *mut c_char,
    /** chunk size in bytes while creating new chunk */
    pub chunk_size: usize,
    /** maximum chunk size in bytes */
    pub chunk_size_max: usize,
    /** a linked list of chunks, nullable */
    pub chunks: *mut yyjson_str_chunk,
}

/** A memory chunk in value memory pool.
    `sizeof(yyjson_val_chunk)` should not be larger than
    `sizeof(yyjson_mut_val)`. */
#[repr(C)]
pub(crate) struct yyjson_val_chunk {
    /** next chunk linked list */
    pub next: *mut yyjson_val_chunk,
    /** chunk size in bytes */
    pub chunk_size: usize,
    /* char pad[sizeof(yyjson_mut_val) - sizeof(yyjson_val_chunk)]; padding */
    /* yyjson_mut_val vals[]; flexible array member */
}

/** A memory pool to hold all values in a mutable document. */
#[repr(C)]
pub(crate) struct yyjson_val_pool {
    /** cursor inside current chunk */
    pub cur: *mut yyjson_mut_val,
    /** the end of current chunk */
    pub end: *mut yyjson_mut_val,
    /** chunk size in bytes while creating new chunk */
    pub chunk_size: usize,
    /** maximum chunk size in bytes */
    pub chunk_size_max: usize,
    /** a linked list of chunks, nullable */
    pub chunks: *mut yyjson_val_chunk,
}

/** A mutable document for building JSON. */
#[repr(C)]
pub(crate) struct yyjson_mut_doc {
    /** root value of the JSON document, nullable */
    pub root: *mut yyjson_mut_val,
    /** a valid allocator, nonnull */
    pub alc: yyjson_alc,
    /** string memory pool */
    pub str_pool: yyjson_str_pool,
    /** value memory pool */
    pub val_pool: yyjson_val_pool,
}

/*==============================================================================
 * MARK: - Iterators (yyjson.h:2202, 2327, 2929, 3755)
 *============================================================================*/

/** An iterator for an immutable array. */
#[repr(C)]
pub(crate) struct yyjson_arr_iter {
    /** next value's index */
    pub idx: usize,
    /** maximum index (arr.size) */
    pub max: usize,
    /** next value */
    pub cur: *mut yyjson_val,
}

/** An iterator for an immutable object. */
#[repr(C)]
pub(crate) struct yyjson_obj_iter {
    /** next key's index */
    pub idx: usize,
    /** maximum key index (obj.size) */
    pub max: usize,
    /** next key */
    pub cur: *mut yyjson_val,
    /** the object being iterated */
    pub obj: *mut yyjson_val,
}

/** An iterator for a mutable array. */
#[repr(C)]
pub(crate) struct yyjson_mut_arr_iter {
    /** next value's index */
    pub idx: usize,
    /** maximum index (arr.size) */
    pub max: usize,
    /** current value */
    pub cur: *mut yyjson_mut_val,
    /** previous value */
    pub pre: *mut yyjson_mut_val,
    /** the array being iterated */
    pub arr: *mut yyjson_mut_val,
}

/** An iterator for a mutable object. */
#[repr(C)]
pub(crate) struct yyjson_mut_obj_iter {
    /** next key's index */
    pub idx: usize,
    /** maximum key index (obj.size) */
    pub max: usize,
    /** current key */
    pub cur: *mut yyjson_mut_val,
    /** previous key */
    pub pre: *mut yyjson_mut_val,
    /** the object being iterated */
    pub obj: *mut yyjson_mut_val,
}

/*==============================================================================
 * MARK: - JSON Pointer API (yyjson.h:4286-4361)
 *============================================================================*/

/** JSON Pointer error code. */
pub(crate) type yyjson_ptr_code = u32;

/** No JSON pointer error. */
pub(crate) const YYJSON_PTR_ERR_NONE: yyjson_ptr_code = 0;
/** Invalid input parameter, such as NULL input. */
pub(crate) const YYJSON_PTR_ERR_PARAMETER: yyjson_ptr_code = 1;
/** JSON pointer syntax error, such as invalid escape or missing prefix. */
pub(crate) const YYJSON_PTR_ERR_SYNTAX: yyjson_ptr_code = 2;
/** JSON pointer resolve failed, such as index out of range, key not found. */
pub(crate) const YYJSON_PTR_ERR_RESOLVE: yyjson_ptr_code = 3;
/** Document's root is NULL, but it is required for the function call. */
pub(crate) const YYJSON_PTR_ERR_NULL_ROOT: yyjson_ptr_code = 4;
/** Cannot set root as the target is not a document. */
pub(crate) const YYJSON_PTR_ERR_SET_ROOT: yyjson_ptr_code = 5;
/** The memory allocation failed and a new value could not be created. */
pub(crate) const YYJSON_PTR_ERR_MEMORY_ALLOCATION: yyjson_ptr_code = 6;

/** Error information for JSON pointer. */
#[repr(C)]
pub(crate) struct yyjson_ptr_err {
    /** Error code, see `yyjson_ptr_code` for all possible values. */
    pub code: yyjson_ptr_code,
    /** Error message, constant, no need to free (NULL if no error). */
    pub msg: *const c_char,
    /** Error byte position for input JSON pointer (0 if no error). */
    pub pos: usize,
}

/** A context for JSON pointer operation. */
#[repr(C)]
pub(crate) struct yyjson_ptr_ctx {
    /** The container (parent) of the target value, or NULL. */
    pub ctn: *mut yyjson_mut_val,
    /** The previous sibling of the target value, or NULL. */
    pub pre: *mut yyjson_mut_val,
    /** The removed value if the operation is `set`, `replace` or `remove`. */
    pub old: *mut yyjson_mut_val,
}

/*==============================================================================
 * MARK: - JSON Patch API (yyjson.h:4868-4904)
 *============================================================================*/

/** Result code for JSON patch. */
pub(crate) type yyjson_patch_code = u32;

/** Success, no error. */
pub(crate) const YYJSON_PATCH_SUCCESS: yyjson_patch_code = 0;
/** Invalid parameter, such as NULL input or non-array patch. */
pub(crate) const YYJSON_PATCH_ERROR_INVALID_PARAMETER: yyjson_patch_code = 1;
/** Memory allocation failed. */
pub(crate) const YYJSON_PATCH_ERROR_MEMORY_ALLOCATION: yyjson_patch_code = 2;
/** JSON patch operation is not object type. */
pub(crate) const YYJSON_PATCH_ERROR_INVALID_OPERATION: yyjson_patch_code = 3;
/** JSON patch operation is missing a required key. */
pub(crate) const YYJSON_PATCH_ERROR_MISSING_KEY: yyjson_patch_code = 4;
/** JSON patch operation member is invalid. */
pub(crate) const YYJSON_PATCH_ERROR_INVALID_MEMBER: yyjson_patch_code = 5;
/** JSON patch `test` operation failed (values not equal). */
pub(crate) const YYJSON_PATCH_ERROR_EQUAL: yyjson_patch_code = 6;
/** JSON patch operation failed on JSON pointer. */
pub(crate) const YYJSON_PATCH_ERROR_POINTER: yyjson_patch_code = 7;

/** Error information for JSON patch. */
#[repr(C)]
pub(crate) struct yyjson_patch_err {
    /** Error code, see `yyjson_patch_code` for all possible values. */
    pub code: yyjson_patch_code,
    /** Index of the error operation (0 if no error). */
    pub idx: usize,
    /** Error message, constant, no need to free (NULL if no error). */
    pub msg: *const c_char,
    /** JSON pointer error if `code == YYJSON_PATCH_ERROR_POINTER`. */
    pub ptr: yyjson_ptr_err,
}

/*==============================================================================
 * MARK: - Incremental reader state (yyjson.h:1086)
 *
 * The header leaves `struct yyjson_incr_state` opaque, so only the forward
 * declaration belongs here. The real layout lives in yyjson.c and therefore in
 * `crate::parse::incremental`, which owns it.
 *============================================================================*/

#[allow(unused_imports)] /* re-export for the modules that take `*mut` it */
pub(crate) use crate::parse::incremental::yyjson_incr_state;

/* Re-exported for exported signatures that mention `FILE *`. */
#[allow(unused_imports)] /* re-export for exported `FILE *` signatures */
pub(crate) use crate::sys::abi::FILE as yyjson_FILE;

/*==============================================================================
 * MARK: - Layout assertions (LP64 Linux, little-endian)
 *
 * Every public struct's size and every field offset is pinned here, and the
 * same numbers are printed by the C and Rust smoke tests and diffed.
 *============================================================================*/

use core::mem::{align_of, size_of};

/* --- scalar and pointer widths this port assumes --- */
const _: () = assert!(size_of::<usize>() == 8);
const _: () = assert!(size_of::<*mut c_void>() == 8);
const _: () = assert!(size_of::<c_char>() == 1);
const _: () = assert!(size_of::<FILE>() == 0);
const _: () = assert!(size_of::<u128>() == 16);
const _: () = assert!(size_of::<f64>() == 8 && size_of::<f32>() == 4);
const _: () = assert!(u64::from_le_bytes(1u64.to_ne_bytes()) == 1,
                      "the byte order of this target is not little-endian, which this port assumes");

/* --- yyjson_alc: 4 pointers --- */
const _: () = assert!(size_of::<yyjson_alc>() == 32);
const _: () = assert!(align_of::<yyjson_alc>() == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_alc, malloc) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_alc, realloc) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_alc, free) == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_alc, ctx) == 24);
/* The C members are plain (non-nullable in practice) function pointers; the
   Option wrapper must be a bare pointer with no discriminant. */
const _: () = assert!(size_of::<Option<unsafe extern "C" fn(*mut c_void, usize)
    -> *mut c_void>>() == 8);

/* --- yyjson_val_uni / yyjson_val: 16 bytes --- */
const _: () = assert!(size_of::<yyjson_val_uni>() == 8);
const _: () = assert!(align_of::<yyjson_val_uni>() == 8);
const _: () = assert!(size_of::<yyjson_val>() == 16);
const _: () = assert!(align_of::<yyjson_val>() == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_val, tag) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_val, uni) == 8);

/* --- yyjson_mut_val: 24 bytes --- */
const _: () = assert!(size_of::<yyjson_mut_val>() == 24);
const _: () = assert!(align_of::<yyjson_mut_val>() == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_val, tag) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_val, uni) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_val, next) == 16);

/* --- yyjson_doc --- */
const _: () = assert!(size_of::<yyjson_doc>() == 64);
const _: () = assert!(core::mem::offset_of!(yyjson_doc, root) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_doc, alc) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_doc, dat_read) == 40);
const _: () = assert!(core::mem::offset_of!(yyjson_doc, val_read) == 48);
const _: () = assert!(core::mem::offset_of!(yyjson_doc, str_pool) == 56);

/* --- string / value pools and their chunk headers --- */
const _: () = assert!(size_of::<yyjson_str_chunk>() == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_str_chunk, next) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_str_chunk, chunk_size) == 8);

const _: () = assert!(size_of::<yyjson_val_chunk>() == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_val_chunk, next) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_val_chunk, chunk_size) == 8);
/* yyjson.c's stated invariant for the value pool. */
const _: () = assert!(size_of::<yyjson_val_chunk>()
    <= size_of::<yyjson_mut_val>());

const _: () = assert!(size_of::<yyjson_str_pool>() == 40);
const _: () = assert!(core::mem::offset_of!(yyjson_str_pool, cur) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_str_pool, end) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_str_pool, chunk_size) == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_str_pool, chunk_size_max)
    == 24);
const _: () = assert!(core::mem::offset_of!(yyjson_str_pool, chunks) == 32);

const _: () = assert!(size_of::<yyjson_val_pool>() == 40);
const _: () = assert!(core::mem::offset_of!(yyjson_val_pool, cur) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_val_pool, end) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_val_pool, chunk_size) == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_val_pool, chunk_size_max)
    == 24);
const _: () = assert!(core::mem::offset_of!(yyjson_val_pool, chunks) == 32);

/* --- yyjson_mut_doc --- */
const _: () = assert!(size_of::<yyjson_mut_doc>() == 120);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_doc, root) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_doc, alc) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_doc, str_pool) == 40);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_doc, val_pool) == 80);

/* --- error structs --- */
const _: () = assert!(size_of::<yyjson_read_err>() == 24);
const _: () = assert!(core::mem::offset_of!(yyjson_read_err, code) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_read_err, msg) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_read_err, pos) == 16);

const _: () = assert!(size_of::<yyjson_write_err>() == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_write_err, code) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_write_err, msg) == 8);

const _: () = assert!(size_of::<yyjson_ptr_err>() == 24);
const _: () = assert!(core::mem::offset_of!(yyjson_ptr_err, code) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_ptr_err, msg) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_ptr_err, pos) == 16);

const _: () = assert!(size_of::<yyjson_patch_err>() == 48);
const _: () = assert!(core::mem::offset_of!(yyjson_patch_err, code) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_patch_err, idx) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_patch_err, msg) == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_patch_err, ptr) == 24);

/* --- JSON pointer context --- */
const _: () = assert!(size_of::<yyjson_ptr_ctx>() == 24);
const _: () = assert!(core::mem::offset_of!(yyjson_ptr_ctx, ctn) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_ptr_ctx, pre) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_ptr_ctx, old) == 16);

/* --- iterators --- */
const _: () = assert!(size_of::<yyjson_arr_iter>() == 24);
const _: () = assert!(core::mem::offset_of!(yyjson_arr_iter, idx) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_arr_iter, max) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_arr_iter, cur) == 16);

const _: () = assert!(size_of::<yyjson_obj_iter>() == 32);
const _: () = assert!(core::mem::offset_of!(yyjson_obj_iter, idx) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_obj_iter, max) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_obj_iter, cur) == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_obj_iter, obj) == 24);

const _: () = assert!(size_of::<yyjson_mut_arr_iter>() == 40);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_arr_iter, idx) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_arr_iter, max) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_arr_iter, cur) == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_arr_iter, pre) == 24);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_arr_iter, arr) == 32);

const _: () = assert!(size_of::<yyjson_mut_obj_iter>() == 40);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_obj_iter, idx) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_obj_iter, max) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_obj_iter, cur) == 16);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_obj_iter, pre) == 24);
const _: () = assert!(core::mem::offset_of!(yyjson_mut_obj_iter, obj) == 32);

/* --- tag/mask invariants the header's inline functions rely on --- */
const _: () = assert!(YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ == 6);
const _: () = assert!(YYJSON_WRITE_FP_TO_FIXED(15) == 0xF000_0000);
const _: () = assert!(YYJSON_WRITE_FP_TO_FLOAT == 0x0800_0000);
