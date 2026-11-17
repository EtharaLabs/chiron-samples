// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 * MARK: - reader.rs
 *
 * Owns yyjson.c 3120-3602 (reader utils) except `scan_num_raw`, `return_raw`
 * and `scan_num_hex` (which belong to read_num.rs), and 4696-6546 (string
 * reader, reader implementation, public reader including
 * `yyjson_read_number`).
 *
 * The C macros `has_flg(_flg)` / `has_allow(_flg)` (yyjson.c:3123-3124) capture
 * the caller's `flg` local, so they are expanded by hand at every use site as
 * `inner_has_rflag(flg, YYJSON_READ_<X>, false)` and
 * `inner_has_rflag(flg, YYJSON_READ_ALLOW_<X>, true)` respectively.
 *
 * The `return_err` / `expr_*` / `repeat*` macros, which also capture caller
 * locals, become function-local `macro_rules!` declared inside the function
 * that owns the locals, exactly as the C `#define`s are scoped to their
 * function body by a trailing `#undef`.
 *
 * The reader's `goto` state machine (`doc_begin`, `arr_begin`, `arr_val_begin`,
 * ..., `fail_*`) becomes an explicit state-constant dispatch loop so that the
 * control flow, the error message chosen on every path and the byte position
 * reported are identical to the C.
 *============================================================================*/

use crate::abi::libc::{self as ffi, FILE, c_char, c_long, c_void};
use crate::decode::numeric::read_num;
use crate::repr::shape::{
    YYJSON_PADDING_SIZE, YYJSON_READ_ALLOW_BOM, YYJSON_READ_ALLOW_COMMENTS,
    YYJSON_READ_ALLOW_EXT_ESCAPE, YYJSON_READ_ALLOW_EXT_NUMBER,
    YYJSON_READ_ALLOW_EXT_WHITESPACE, YYJSON_READ_ALLOW_INF_AND_NAN,
    YYJSON_READ_ALLOW_INVALID_UNICODE, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR,
    YYJSON_READ_ALLOW_TRAILING_COMMAS, YYJSON_READ_ALLOW_UNQUOTED_KEY,
    YYJSON_READ_ERROR_EMPTY_CONTENT,
    YYJSON_READ_ERROR_FILE_OPEN, YYJSON_READ_ERROR_FILE_READ,
    YYJSON_READ_ERROR_INVALID_COMMENT, YYJSON_READ_ERROR_INVALID_NUMBER,
    YYJSON_READ_ERROR_INVALID_PARAMETER, YYJSON_READ_ERROR_INVALID_STRING,
    YYJSON_READ_ERROR_JSON_STRUCTURE, YYJSON_READ_ERROR_LITERAL,
    YYJSON_READ_ERROR_MEMORY_ALLOCATION,
    YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, YYJSON_READ_ERROR_UNEXPECTED_CONTENT,
    YYJSON_READ_ERROR_UNEXPECTED_END, YYJSON_READ_INSITU,
    YYJSON_READ_NUMBER_AS_RAW, YYJSON_READ_STOP_WHEN_DONE,
    YYJSON_SUBTYPE_FALSE, YYJSON_SUBTYPE_NOESC, YYJSON_SUBTYPE_REAL,
    YYJSON_SUBTYPE_TRUE, YYJSON_TAG_BIT, YYJSON_TAG_MASK, YYJSON_TYPE_ARR,
    YYJSON_TYPE_BOOL, YYJSON_TYPE_MASK, YYJSON_TYPE_NULL, YYJSON_TYPE_NUM,
    YYJSON_TYPE_OBJ, YYJSON_TYPE_RAW, YYJSON_TYPE_STR, yyjson_alc, yyjson_doc,
    yyjson_read_code, yyjson_read_err, yyjson_read_flag, yyjson_val,
};
use crate::mem::bytes::{
    MSG_ARR_END, MSG_CHAR, MSG_CHAR_F, MSG_CHAR_N, MSG_CHAR_T, MSG_COMMA,
    MSG_COMMENT, MSG_ERR_BOM, MSG_ERR_UTF16, MSG_ERR_UTF32, MSG_ERR_UTF8,
    MSG_FREAD, MSG_GARBAGE, MSG_MALLOC, MSG_NOT_END, MSG_OBJ_END, MSG_OBJ_KEY,
    MSG_OBJ_SEP,
    YYJSON_READ_ALLOW_TRIVIA, YYJSON_READER_ESTIMATED_MINIFY_RATIO,
    YYJSON_READER_ESTIMATED_PRETTY_RATIO,
    raw_dup_4, raw_match_2, raw_match_4, glyph_is_ascii_skip,
    glyph_is_ascii_skip_sq, glyph_is_ctn, glyph_is_eol, glyph_is_eol_ext,
    glyph_is_hex, glyph_is_id_ascii, glyph_is_id_next, glyph_is_id_start,
    glyph_is_num,
    glyph_is_space, glyph_is_space_ext, glyph_is_trivia, char_to_lower,
    inner_ext_eol_len, inner_ext_space_len, dbl_bits_inf, dbl_bits_nan, fopen_readonly,
    fread_safe, inner_has_rflag, base16_load_2, base16_load_4, test_utf32_bom, test_utf8_bom,
    test_utf16_bom, test_utf8_seq1, test_utf8_seq2, test_utf8_seq3, test_utf8_seq4,
};

/*==============================================================================
 * MARK: - JSON Reader Utils (Private) (yyjson.c:3137-3596)
 * These functions are used by JSON reader to read literals and comments.
 *============================================================================*/

/** yyjson.c:3143  Read `true` literal, `*ptr[0]` should be `t`. */
#[inline(always)]
pub(crate) unsafe fn scan_true(ptr: *mut *mut u8, val: *mut yyjson_val)
    -> bool {
    unsafe {
        let cur: *mut u8 = *ptr;
        if raw_match_4(cur as *mut c_void, c"true".as_ptr() as *const u8) {
            (*val).tag = (YYJSON_TYPE_BOOL | YYJSON_SUBTYPE_TRUE) as u64;
            *ptr = cur.add(4);
            return true;
        }
        false
    }
}

/** yyjson.c:3154  Read `false` literal, `*ptr[0]` should be `f`. */
#[inline(always)]
pub(crate) unsafe fn scan_false(ptr: *mut *mut u8, val: *mut yyjson_val)
    -> bool {
    unsafe {
        let cur: *mut u8 = *ptr;
        if raw_match_4(cur.add(1) as *mut c_void,
                        c"alse".as_ptr() as *const u8) {
            (*val).tag = (YYJSON_TYPE_BOOL | YYJSON_SUBTYPE_FALSE) as u64;
            *ptr = cur.add(5);
            return true;
        }
        false
    }
}

/** yyjson.c:3165  Read `null` literal, `*ptr[0]` should be `n`. */
#[inline(always)]
pub(crate) unsafe fn scan_null(ptr: *mut *mut u8, val: *mut yyjson_val)
    -> bool {
    unsafe {
        let cur: *mut u8 = *ptr;
        if raw_match_4(cur as *mut c_void, c"null".as_ptr() as *const u8) {
            (*val).tag = YYJSON_TYPE_NULL as u64;
            *ptr = cur.add(4);
            return true;
        }
        false
    }
}

/** yyjson.c:3176  Read `Inf` or `Infinity` literal (ignoring case). */
#[inline(always)]
pub(crate) unsafe fn scan_inf(ptr: *mut *mut u8, pre: *mut *mut u8,
                              flg: yyjson_read_flag, val: *mut yyjson_val)
    -> bool {
    unsafe {
        let hdr: *mut u8 = *ptr;
        let mut cur: *mut u8 = *ptr;
        let end: *mut *mut u8 = ptr;
        let sign: bool = *cur == b'-';
        /* has_allow(EXT_NUMBER) */
        if *cur == b'+'
            && !inner_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true)
        {
            return false;
        }
        cur = cur.add(crate::mem::bytes::glyph_is_sign(*cur) as usize);
        if char_to_lower(*cur.add(0)) == b'i'
            && char_to_lower(*cur.add(1)) == b'n'
            && char_to_lower(*cur.add(2)) == b'f'
        {
            if char_to_lower(*cur.add(3)) == b'i' {
                if char_to_lower(*cur.add(4)) == b'n'
                    && char_to_lower(*cur.add(5)) == b'i'
                    && char_to_lower(*cur.add(6)) == b't'
                    && char_to_lower(*cur.add(7)) == b'y'
                {
                    cur = cur.add(8);
                } else {
                    return false;
                }
            } else {
                cur = cur.add(3);
            }
            *end = cur;
            /* has_flg(NUMBER_AS_RAW) */
            if inner_has_rflag(flg, YYJSON_READ_NUMBER_AS_RAW, false) {
                /* add null-terminator for previous raw string */
                **pre = b'\0';
                /* save end position for current raw string */
                *pre = cur;
                (*val).tag = ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                    | YYJSON_TYPE_RAW as u64;
                (*val).uni.str_ = hdr as *const c_char;
            } else {
                (*val).tag = (YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL) as u64;
                (*val).uni.u64_ = dbl_bits_inf(sign);
            }
            return true;
        }
        false
    }
}

/** yyjson.c:3215  Read `NaN` literal (ignoring case). */
#[inline(always)]
pub(crate) unsafe fn scan_nan(ptr: *mut *mut u8, pre: *mut *mut u8,
                              flg: yyjson_read_flag, val: *mut yyjson_val)
    -> bool {
    unsafe {
        let hdr: *mut u8 = *ptr;
        let mut cur: *mut u8 = *ptr;
        let end: *mut *mut u8 = ptr;
        let sign: bool = *cur == b'-';
        /* has_allow(EXT_NUMBER) */
        if *cur == b'+'
            && !inner_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true)
        {
            return false;
        }
        cur = cur.add(crate::mem::bytes::glyph_is_sign(*cur) as usize);
        if char_to_lower(*cur.add(0)) == b'n'
            && char_to_lower(*cur.add(1)) == b'a'
            && char_to_lower(*cur.add(2)) == b'n'
        {
            cur = cur.add(3);
            *end = cur;
            /* has_flg(NUMBER_AS_RAW) */
            if inner_has_rflag(flg, YYJSON_READ_NUMBER_AS_RAW, false) {
                **pre = b'\0';
                *pre = cur;
                (*val).tag = ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                    | YYJSON_TYPE_RAW as u64;
                (*val).uni.str_ = hdr as *const c_char;
            } else {
                (*val).tag = (YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL) as u64;
                (*val).uni.u64_ = dbl_bits_nan(sign);
            }
            return true;
        }
        false
    }
}

/** yyjson.c:3243  Read `Inf`, `Infinity` or `NaN` literal (ignoring case). */
#[inline(always)]
pub(crate) unsafe fn scan_inf_or_nan(ptr: *mut *mut u8, pre: *mut *mut u8,
                                     flg: yyjson_read_flag,
                                     val: *mut yyjson_val) -> bool {
    unsafe {
        if scan_inf(ptr, pre, flg, val) {
            return true;
        }
        if scan_nan(ptr, pre, flg, val) {
            return true;
        }
        false
    }
}

/**
 yyjson.c:3395  Skip trivia (whitespace and comments).
 This function should be used only when `glyph_is_trivia()` returns true.
 */
#[inline(never)]
pub(crate) unsafe fn skip_trivia(ptr: *mut *mut u8, eof: *mut u8,
                                 flg: yyjson_read_flag) -> bool {
    unsafe {
        let hdr: *mut u8 = *ptr;
        let mut cur: *mut u8 = *ptr;
        let mut len: usize;

        while cur < eof {
            let loop_begin: *mut u8 = cur;

            /* skip standard whitespace */
            while glyph_is_space(*cur) {
                cur = cur.add(1);
            }

            /* skip extended whitespace */
            /* has_allow(EXT_WHITESPACE) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE, true) {
                while glyph_is_space_ext(*cur) {
                    len = inner_ext_space_len(cur);
                    cur = cur.add(len);
                    if len == 0 {
                        break;
                    }
                }
            }

            /* skip comment, do not validate encoding */
            /* has_allow(COMMENTS) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_COMMENTS, true)
                && *cur.add(0) == b'/'
            {
                if *cur.add(1) == b'/' {
                    /* single-line comment */
                    cur = cur.add(2);
                    /* has_allow(EXT_WHITESPACE) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE, true) {
                        while cur < eof {
                            if glyph_is_eol_ext(*cur) {
                                len = inner_ext_eol_len(cur);
                                cur = cur.add(len);
                                if len != 0 {
                                    break;
                                }
                            }
                            cur = cur.add(1);
                        }
                    } else {
                        while cur < eof && !glyph_is_eol(*cur) {
                            cur = cur.add(1);
                        }
                    }
                } else if *cur.add(1) == b'*' {
                    /* multi-line comment */
                    cur = cur.add(2);
                    while !raw_match_2(cur as *mut c_void,
                                        c"*/".as_ptr() as *const u8)
                        && cur < eof
                    {
                        cur = cur.add(1);
                    }
                    if cur == eof {
                        *ptr = eof;
                        return false; /* unclosed comment */
                    }
                    cur = cur.add(2);
                }
            }
            if cur == loop_begin {
                break;
            }
        }
        *ptr = cur;
        cur > hdr
    }
}

/**
 yyjson.c:3448  Check truncated UTF-8 character.
 Return true if `cur` starts a valid UTF-8 sequence that is truncated.
 */
#[inline]
pub(crate) unsafe fn is_truncated_utf8(cur: *mut u8, eof: *mut u8) -> bool {
    unsafe {
        let c0: u8;
        let c1: u8;
        let c2: u8;
        let len: usize = eof.offset_from(cur) as usize;
        if cur >= eof || len >= 4 {
            return false;
        }
        c0 = *cur.add(0);
        c1 = *cur.add(1);
        c2 = *cur.add(2);
        /* 1-byte UTF-8, not truncated */
        if c0 < 0x80 {
            return false;
        }
        if len == 1 {
            /* 2-byte UTF-8, truncated */
            if (c0 & 0xE0) == 0xC0 && (c0 & 0x1E) != 0x00 {
                return true;
            }
            /* 3-byte UTF-8, truncated */
            if (c0 & 0xF0) == 0xE0 {
                return true;
            }
            /* 4-byte UTF-8, truncated */
            if (c0 & 0xF8) == 0xF0 && (c0 & 0x07) <= 0x04 {
                return true;
            }
        } else if len == 2 {
            /* 3-byte UTF-8, truncated */
            if (c0 & 0xF0) == 0xE0 && (c1 & 0xC0) == 0x80 {
                let t: u8 = ((c0 & 0x0F) << 1) | ((c1 & 0x20) >> 5);
                return 0x01 <= t && t != 0x1B;
            }
            /* 4-byte UTF-8, truncated */
            if (c0 & 0xF8) == 0xF0 && (c1 & 0xC0) == 0x80 {
                let t: u8 = ((c0 & 0x07) << 2) | ((c1 & 0x30) >> 4);
                return 0x01 <= t && t <= 0x10;
            }
        } else if len == 3 {
            /* 4 bytes UTF-8, truncated */
            if (c0 & 0xF8) == 0xF0 && (c1 & 0xC0) == 0x80
                && (c2 & 0xC0) == 0x80
            {
                let t: u8 = ((c0 & 0x07) << 2) | ((c1 & 0x30) >> 4);
                return 0x01 <= t && t <= 0x10;
            }
        }
        false
    }
}

/**
 yyjson.c:3488  Check truncated string.
 Returns true if `cur` match `str` but is truncated.
 The `str` should be lowercase ASCII letters.
 */
#[inline]
pub(crate) unsafe fn is_truncated_str(mut cur: *mut u8, eof: *mut u8,
                                      mut str_: *const u8,
                                      case_sensitive: bool) -> bool {
    unsafe {
        let len: usize = crate::repr::inline::strlen(str_ as *mut u8);
        if cur.add(len) <= eof || eof <= cur {
            return false;
        }
        if case_sensitive {
            return crate::mem::bytes::memcmp(cur as *const c_void,
                                       str_ as *const c_void,
                                       eof.offset_from(cur) as usize) == 0;
        }
        while cur < eof {
            if char_to_lower(*cur) != *str_ {
                return false;
            }
            cur = cur.add(1);
            str_ = str_.add(1);
        }
        true
    }
}

/**
 yyjson.c:3505  Check truncated JSON on parsing errors.
 Returns true if the input is valid but truncated.
 */
#[inline(never)]
pub(crate) unsafe fn is_truncated_end(hdr: *mut u8, mut cur: *mut u8,
                                      eof: *mut u8, code: yyjson_read_code,
                                      flg: yyjson_read_flag) -> bool {
    unsafe {
        if cur >= eof {
            return true;
        }
        if code == YYJSON_READ_ERROR_LITERAL {
            if is_truncated_str(cur, eof, c"true".as_ptr() as *const u8, true)
                || is_truncated_str(cur, eof,
                                    c"false".as_ptr() as *const u8, true)
                || is_truncated_str(cur, eof,
                                    c"null".as_ptr() as *const u8, true)
            {
                return true;
            }
        }
        if code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER
            || code == YYJSON_READ_ERROR_INVALID_NUMBER
            || code == YYJSON_READ_ERROR_LITERAL
        {
            /* has_allow(INF_AND_NAN) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true) {
                if *cur == b'-' {
                    cur = cur.add(1);
                }
                if is_truncated_str(cur, eof,
                                    c"infinity".as_ptr() as *const u8, false)
                    || is_truncated_str(cur, eof,
                                        c"nan".as_ptr() as *const u8, false)
                {
                    return true;
                }
            }
        }
        if code == YYJSON_READ_ERROR_UNEXPECTED_CONTENT {
            /* has_allow(INF_AND_NAN) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true) {
                if hdr.add(3) <= cur
                    && is_truncated_str(cur.sub(3), eof,
                                        c"infinity".as_ptr() as *const u8,
                                        false)
                {
                    /* e.g. infin would be read as inf + in */
                    return true;
                }
            }
        }
        if code == YYJSON_READ_ERROR_INVALID_STRING {
            let len: usize = eof.offset_from(cur) as usize;

            /* unicode escape sequence */
            if *cur == b'\\' {
                if len == 1 {
                    return true;
                }
                if len <= 5 {
                    cur = cur.add(1);
                    if *cur != b'u' {
                        return false;
                    }
                    cur = cur.add(1);
                    while cur < eof {
                        if !glyph_is_hex(*cur) {
                            return false;
                        }
                        cur = cur.add(1);
                    }
                    return true;
                } else if len <= 11 {
                    /* incomplete surrogate pair? */
                    let mut hi: u16 = 0;
                    cur = cur.add(1);
                    if *cur != b'u' {
                        return false;
                    }
                    cur = cur.add(1);
                    if !base16_load_4(cur, &mut hi) {
                        return false;
                    }
                    if (hi & 0xF800) != 0xD800 {
                        return false;
                    }
                    cur = cur.add(4);
                    if cur >= eof {
                        return true;
                    }
                    /* valid low surrogate is DC00...DFFF */
                    if *cur != b'\\' {
                        return false;
                    }
                    cur = cur.add(1);
                    if cur >= eof {
                        return true;
                    }
                    if *cur != b'u' {
                        return false;
                    }
                    cur = cur.add(1);
                    if cur >= eof {
                        return true;
                    }
                    if *cur != b'd' && *cur != b'D' {
                        return false;
                    }
                    cur = cur.add(1);
                    if cur >= eof {
                        return true;
                    }
                    if (*cur < b'c' || *cur > b'f')
                        && (*cur < b'C' || *cur > b'F')
                    {
                        return false;
                    }
                    cur = cur.add(1);
                    if cur >= eof {
                        return true;
                    }
                    if !glyph_is_hex(*cur) {
                        return false;
                    }
                    return true;
                }
                return false;
            }

            /* 2 to 4 bytes UTF-8 */
            if is_truncated_utf8(cur, eof) {
                return true;
            }
        }
        /* has_allow(COMMENTS) */
        if inner_has_rflag(flg, YYJSON_READ_ALLOW_COMMENTS, true) {
            if code == YYJSON_READ_ERROR_INVALID_COMMENT {
                /* unclosed multiline comment */
                return true;
            }
            if code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER
                && *cur == b'/' && cur.add(1) == eof
            {
                /* truncated beginning of comment */
                return true;
            }
        }
        /* has_allow(BOM) */
        if code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER
            && inner_has_rflag(flg, YYJSON_READ_ALLOW_BOM, true)
        {
            /* truncated UTF-8 BOM */
            let len: usize = eof.offset_from(cur) as usize;
            if cur == hdr && len < 3
                && crate::mem::bytes::memcmp(
                    hdr as *const c_void,
                    c"\xEF\xBB\xBF".as_ptr() as *const c_void, len) == 0
            {
                return true;
            }
        }
        false
    }
}

/*==============================================================================
 * MARK: - String Reader (Private) (yyjson.c:4696-5235)
 *============================================================================*/

/** yyjson.c:4700  Read unicode escape sequence. */
#[inline(always)]
pub(crate) unsafe fn scan_uni_esc(src_ptr: *mut *mut u8,
                                  dst_ptr: *mut *mut u8,
                                  msg: *mut *const c_char) -> bool {
    unsafe {
        /* yyjson.c:4701  return_err(_end, _msg) */
        macro_rules! return_err {
            ($end_:expr, $m:expr) => {{
                *msg = $m;
                *src_ptr = $end_;
                return false;
            }};
        }

        let mut src: *mut u8 = *src_ptr;
        let mut dst: *mut u8 = *dst_ptr;
        let mut hi: u16 = 0;
        let mut lo: u16 = 0;
        let uni: u32;

        src = src.add(2); /* skip `\u` */
        if !base16_load_4(src, &mut hi) {
            return_err!(src.sub(2),
                c"invalid escaped sequence in string".as_ptr()
                    as *const c_char);
        }
        src = src.add(4); /* skip hex */
        if (hi & 0xF800) != 0xD800 {
            /* a BMP character */
            if hi >= 0x800 {
                *dst = 0xE0 | (hi >> 12) as u8;
                dst = dst.add(1);
                *dst = 0x80 | ((hi >> 6) & 0x3F) as u8;
                dst = dst.add(1);
                *dst = 0x80 | (hi & 0x3F) as u8;
                dst = dst.add(1);
            } else if hi >= 0x80 {
                *dst = 0xC0 | (hi >> 6) as u8;
                dst = dst.add(1);
                *dst = 0x80 | (hi & 0x3F) as u8;
                dst = dst.add(1);
            } else {
                *dst = hi as u8;
                dst = dst.add(1);
            }
        } else {
            /* a non-BMP character, represented as a surrogate pair */
            if (hi & 0xFC00) != 0xD800 {
                return_err!(src.sub(6),
                    c"invalid high surrogate in string".as_ptr()
                        as *const c_char);
            }
            if !raw_match_2(src as *mut c_void,
                             c"\\u".as_ptr() as *const u8) {
                return_err!(src.sub(6),
                    c"no low surrogate in string".as_ptr() as *const c_char);
            }
            if !base16_load_4(src.add(2), &mut lo) {
                return_err!(src.sub(6),
                    c"invalid escape in string".as_ptr() as *const c_char);
            }
            if (lo & 0xFC00) != 0xDC00 {
                return_err!(src.sub(6),
                    c"invalid low surrogate in string".as_ptr()
                        as *const c_char);
            }
            uni = ((((hi as u32) - 0xD800) << 10) | ((lo as u32) - 0xDC00))
                + 0x10000;
            *dst = 0xF0 | (uni >> 18) as u8;
            dst = dst.add(1);
            *dst = 0x80 | ((uni >> 12) & 0x3F) as u8;
            dst = dst.add(1);
            *dst = 0x80 | ((uni >> 6) & 0x3F) as u8;
            dst = dst.add(1);
            *dst = 0x80 | (uni & 0x3F) as u8;
            dst = dst.add(1);
            src = src.add(6);
        }
        *src_ptr = src;
        *dst_ptr = dst;
        true
    }
}

/* States of the `scan_str_opt` goto machine (yyjson.c:4764-5079). */
const RS_SKIP_ASCII: u32 = 0;
const RS_COPY_ESCAPE: u32 = 1;
const RS_COPY_ASCII: u32 = 2;
const RS_COPY_UTF8: u32 = 3;

/**
 yyjson.c:4764  Read a JSON string.
 @param quo The quote character (single quote or double quote).
 @param ptr The head pointer of string before quote (inout).
 @param eof JSON end position.
 @param flg JSON read flag.
 @param val The string value to be written.
 @param msg The error message pointer.
 @param con Continuation for incremental parsing.
 @return Whether success.

 The 16-way unrolled `skip_ascii` / `copy_ascii` `goto` chains and the
 `skip_utf8` / `copy_utf8` / `copy_escape` labels become an explicit state
 dispatch. `copy_ascii` keeps the exact `raw_move_16` / `raw_move_forward`
 write pattern of the C, because those helpers write up to one byte past the
 requested length and the resulting buffer bytes are observable under
 `YYJSON_READ_INSITU`. `skip_ascii` performs no writes and its unrolled form
 reads exactly the same bytes as the plain loop used here.
 */
#[inline(always)]
pub(crate) unsafe fn scan_str_opt(quo: u8, ptr: *mut *mut u8, eof: *mut u8,
                                  flg: yyjson_read_flag, val: *mut yyjson_val,
                                  msg: *mut *const c_char,
                                  con: *mut *mut u8) -> bool {
    unsafe {
        let hdr: *mut u8 = (*ptr).add(1);
        let end: *mut *mut u8 = ptr;
        let mut src: *mut u8 = hdr;
        let mut dst: *mut u8 = core::ptr::null_mut();
        let mut pos: *mut u8;
        let mut uni: u32;

        /* yyjson.c:4775  return_err(_end, _msg) */
        macro_rules! return_err {
            ($end_:expr, $m:expr) => {{
                let e_: *mut u8 = $end_;
                *msg = $m;
                *end = e_;
                if !con.is_null() {
                    *con.add(0) = e_;
                    *con.add(1) = dst;
                }
                return false;
            }};
        }

        let mut state: u32 = RS_SKIP_ASCII;

        /* Resume incremental parsing. */
        if !con.is_null() && !(*con.add(0)).is_null() {
            src = *con.add(0);
            dst = *con.add(1);
            if !dst.is_null() {
                state = RS_COPY_ASCII;
            }
        }

        loop {
            match state {
                RS_SKIP_ASCII => {
                    /* skip_ascii: */
                    if quo == b'"' {
                        while glyph_is_ascii_skip(*src) {
                            src = src.add(1);
                        }
                    } else {
                        while glyph_is_ascii_skip_sq(*src) {
                            src = src.add(1);
                        }
                    }

                    /* skip_ascii_end: */
                    if *src == quo {
                        (*val).tag = ((src.offset_from(hdr) as u64)
                            << YYJSON_TAG_BIT)
                            | YYJSON_TYPE_STR as u64
                            | (if quo == b'"' {
                                YYJSON_SUBTYPE_NOESC as u64
                            } else {
                                0
                            });
                        (*val).uni.str_ = hdr as *const c_char;
                        *src = b'\0';
                        *end = src.add(1);
                        if !con.is_null() {
                            *con.add(1) = core::ptr::null_mut();
                            *con.add(0) = core::ptr::null_mut();
                        }
                        return true;
                    }

                    /* skip_utf8: */
                    if (*src & 0x80) != 0 {
                        /* non-ASCII character */
                        pos = src;
                        uni = crate::mem::bytes::raw_read_4(src as *const c_void);
                        while test_utf8_seq3(uni) {
                            src = src.add(3);
                            uni = crate::mem::bytes::raw_read_4(
                                src as *const c_void);
                        }
                        if test_utf8_seq1(uni) {
                            state = RS_SKIP_ASCII;
                            continue;
                        }
                        while test_utf8_seq2(uni) {
                            src = src.add(2);
                            uni = crate::mem::bytes::raw_read_4(
                                src as *const c_void);
                        }
                        while test_utf8_seq4(uni) {
                            src = src.add(4);
                            uni = crate::mem::bytes::raw_read_4(
                                src as *const c_void);
                        }
                        if pos == src {
                            /* has_allow(INVALID_UNICODE) */
                            if inner_has_rflag(flg,
                                         YYJSON_READ_ALLOW_INVALID_UNICODE,
                                         true) {
                                src = src.add(1);
                            } else {
                                return_err!(src,
                                    c"invalid UTF-8 encoding in string"
                                        .as_ptr() as *const c_char);
                            }
                        }
                        state = RS_SKIP_ASCII;
                        continue;
                    }

                    /* The escape character appears, we need to copy it. */
                    dst = src;
                    state = RS_COPY_ESCAPE;
                    continue;
                }

                RS_COPY_ESCAPE => {
                    /* copy_escape: */
                    if *src == b'\\' {
                        src = src.add(1);
                        let c: u8 = *src;
                        match c {
                            b'"' => {
                                *dst = b'"';
                                dst = dst.add(1);
                                src = src.add(1);
                            }
                            b'\\' => {
                                *dst = b'\\';
                                dst = dst.add(1);
                                src = src.add(1);
                            }
                            b'/' => {
                                *dst = b'/';
                                dst = dst.add(1);
                                src = src.add(1);
                            }
                            b'b' => {
                                *dst = 0x08;
                                dst = dst.add(1);
                                src = src.add(1);
                            }
                            b'f' => {
                                *dst = 0x0C;
                                dst = dst.add(1);
                                src = src.add(1);
                            }
                            b'n' => {
                                *dst = b'\n';
                                dst = dst.add(1);
                                src = src.add(1);
                            }
                            b'r' => {
                                *dst = b'\r';
                                dst = dst.add(1);
                                src = src.add(1);
                            }
                            b't' => {
                                *dst = b'\t';
                                dst = dst.add(1);
                                src = src.add(1);
                            }
                            b'u' => {
                                src = src.sub(1);
                                if !scan_uni_esc(&mut src, &mut dst, msg) {
                                    return_err!(src, *msg);
                                }
                            }
                            _ => {
                                /* has_allow(EXT_ESCAPE) */
                                if inner_has_rflag(flg,
                                             YYJSON_READ_ALLOW_EXT_ESCAPE,
                                             true) {
                                    /* read extended escape (non-standard) */
                                    match *src {
                                        b'\'' => {
                                            *dst = b'\'';
                                            dst = dst.add(1);
                                            src = src.add(1);
                                        }
                                        b'a' => {
                                            *dst = 0x07;
                                            dst = dst.add(1);
                                            src = src.add(1);
                                        }
                                        b'v' => {
                                            *dst = 0x0B;
                                            dst = dst.add(1);
                                            src = src.add(1);
                                        }
                                        b'?' => {
                                            *dst = b'?';
                                            dst = dst.add(1);
                                            src = src.add(1);
                                        }
                                        b'e' => {
                                            *dst = 0x1B;
                                            dst = dst.add(1);
                                            src = src.add(1);
                                        }
                                        b'0' => {
                                            if !crate::mem::bytes::glyph_is_digit(
                                                *src.add(1)) {
                                                *dst = b'\0';
                                                dst = dst.add(1);
                                                src = src.add(1);
                                            } else {
                                                return_err!(src.sub(1),
                                                    c"octal escape is not allowed"
                                                        .as_ptr()
                                                        as *const c_char);
                                            }
                                        }
                                        b'1' | b'2' | b'3' | b'4' | b'5'
                                        | b'6' | b'7' | b'8' | b'9' => {
                                            return_err!(src.sub(1),
                                                c"invalid number escape"
                                                    .as_ptr()
                                                    as *const c_char);
                                        }
                                        b'x' => {
                                            let mut cx: u8 = 0;
                                            if base16_load_2(src.add(1),
                                                          &mut cx) {
                                                src = src.add(3);
                                                if cx <= 0x7F {
                                                    /* 1-byte ASCII */
                                                    *dst = cx;
                                                    dst = dst.add(1);
                                                } else {
                                                    /* 2-byte UTF-8 */
                                                    *dst = 0xC0 | (cx >> 6);
                                                    dst = dst.add(1);
                                                    *dst = 0x80 | (cx & 0x3F);
                                                    dst = dst.add(1);
                                                }
                                            } else {
                                                return_err!(src.sub(1),
                                                    c"invalid hex escape"
                                                        .as_ptr()
                                                        as *const c_char);
                                            }
                                        }
                                        b'\n' => {
                                            src = src.add(1);
                                        }
                                        b'\r' => {
                                            src = src.add(1);
                                            src = src.add(
                                                (*src == b'\n') as usize);
                                        }
                                        0xE2 => {
                                            /* Line terminator:
                                               U+2028, U+2029 */
                                            if (*src.add(1) == 0x80
                                                && *src.add(2) == 0xA8)
                                                || (*src.add(1) == 0x80
                                                    && *src.add(2) == 0xA9)
                                            {
                                                src = src.add(3);
                                            }
                                        }
                                        _ => {
                                            /* skip */
                                        }
                                    }
                                } else if quo == b'\'' && *src == b'\'' {
                                    *dst = b'\'';
                                    dst = dst.add(1);
                                    src = src.add(1);
                                } else {
                                    return_err!(src.sub(1),
                                        c"invalid escaped sequence in string"
                                            .as_ptr() as *const c_char);
                                }
                            }
                        }
                    } else if *src == quo {
                        (*val).tag = ((dst.offset_from(hdr) as u64)
                            << YYJSON_TAG_BIT)
                            | YYJSON_TYPE_STR as u64;
                        (*val).uni.str_ = hdr as *const c_char;
                        *dst = b'\0';
                        *end = src.add(1);
                        if !con.is_null() {
                            *con.add(1) = core::ptr::null_mut();
                            *con.add(0) = core::ptr::null_mut();
                        }
                        return true;
                    } else {
                        /* has_allow(INVALID_UNICODE) */
                        if !inner_has_rflag(flg, YYJSON_READ_ALLOW_INVALID_UNICODE,
                                      true) {
                            return_err!(src,
                                c"unexpected control character in string"
                                    .as_ptr() as *const c_char);
                        }
                        if src >= eof {
                            return_err!(src,
                                c"unclosed string".as_ptr() as *const c_char);
                        }
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                    }
                    state = RS_COPY_ASCII;
                    continue;
                }

                RS_COPY_ASCII => {
                    /* copy_ascii: 16-way unrolled copy */
                    loop {
                        let mut i: usize = 0;
                        if quo == b'"' {
                            while i < 16 && glyph_is_ascii_skip(*src.add(i)) {
                                i += 1;
                            }
                        } else {
                            while i < 16
                                && glyph_is_ascii_skip_sq(*src.add(i))
                            {
                                i += 1;
                            }
                        }
                        if i == 16 {
                            crate::mem::bytes::raw_move_16(dst as *mut c_void,
                                                      src as *const c_void);
                            dst = dst.add(16);
                            src = src.add(16);
                            continue;
                        }
                        /*
                         The memory is copied forward since `dst < src`.
                         So it's safe to move one extra byte to reduce
                         instruction count.
                         */
                        crate::mem::bytes::raw_move_forward(dst as *mut c_void,
                                                       src as *mut c_void, i);
                        dst = dst.add(i);
                        src = src.add(i);
                        break;
                    }
                    state = RS_COPY_UTF8;
                    continue;
                }

                _ => {
                    /* copy_utf8: */
                    if (*src & 0x80) != 0 {
                        /* non-ASCII character */
                        pos = src;
                        uni = crate::mem::bytes::raw_read_4(src as *const c_void);
                        while test_utf8_seq3(uni) {
                            raw_dup_4(dst as *mut c_void,
                                        &uni as *const u32 as *const c_void);
                            dst = dst.add(3);
                            src = src.add(3);
                            uni = crate::mem::bytes::raw_read_4(
                                src as *const c_void);
                        }
                        if test_utf8_seq1(uni) {
                            state = RS_COPY_ASCII;
                            continue;
                        }
                        while test_utf8_seq2(uni) {
                            crate::mem::bytes::raw_dup_2(
                                dst as *mut c_void,
                                &uni as *const u32 as *const c_void);
                            dst = dst.add(2);
                            src = src.add(2);
                            uni = crate::mem::bytes::raw_read_4(
                                src as *const c_void);
                        }
                        while test_utf8_seq4(uni) {
                            raw_dup_4(dst as *mut c_void,
                                        &uni as *const u32 as *const c_void);
                            dst = dst.add(4);
                            src = src.add(4);
                            uni = crate::mem::bytes::raw_read_4(
                                src as *const c_void);
                        }
                        if pos == src {
                            /* has_allow(INVALID_UNICODE) */
                            if !inner_has_rflag(flg,
                                          YYJSON_READ_ALLOW_INVALID_UNICODE,
                                          true) {
                                return_err!(src, MSG_ERR_UTF8);
                            }
                            /* goto copy_ascii_stop_1 */
                            crate::mem::bytes::raw_move_forward(
                                dst as *mut c_void, src as *mut c_void, 1);
                            dst = dst.add(1);
                            src = src.add(1);
                            state = RS_COPY_UTF8;
                            continue;
                        }
                        state = RS_COPY_ASCII;
                        continue;
                    }
                    state = RS_COPY_ESCAPE;
                    continue;
                }
            }
        }
    }
}

/** yyjson.c:5081 */
#[inline(always)]
pub(crate) unsafe fn scan_str(ptr: *mut *mut u8, eof: *mut u8,
                              flg: yyjson_read_flag, val: *mut yyjson_val,
                              msg: *mut *const c_char) -> bool {
    unsafe {
        scan_str_opt(b'"', ptr, eof, flg, val, msg, core::ptr::null_mut())
    }
}

/** yyjson.c:5086 */
#[inline(always)]
pub(crate) unsafe fn scan_str_con(ptr: *mut *mut u8, eof: *mut u8,
                                  flg: yyjson_read_flag, val: *mut yyjson_val,
                                  msg: *mut *const c_char,
                                  con: *mut *mut u8) -> bool {
    unsafe { scan_str_opt(b'"', ptr, eof, flg, val, msg, con) }
}

/** yyjson.c:5091 */
#[inline(never)]
pub(crate) unsafe fn scan_str_sq(ptr: *mut *mut u8, eof: *mut u8,
                                 flg: yyjson_read_flag, val: *mut yyjson_val,
                                 msg: *mut *const c_char) -> bool {
    unsafe {
        scan_str_opt(b'\'', ptr, eof, flg, val, msg, core::ptr::null_mut())
    }
}

/* States of the `scan_str_id` goto machine (yyjson.c:5097-5235). */
const RI_SKIP_ASCII: u32 = 0;
const RI_COPY_ESCAPE: u32 = 1;
const RI_COPY_ASCII: u32 = 2;
const RI_COPY_UTF8: u32 = 3;

/** yyjson.c:5097  Read unquoted key (identifier name). */
#[inline(never)]
pub(crate) unsafe fn scan_str_id(ptr: *mut *mut u8, _eof: *mut u8,
                                 flg: yyjson_read_flag, pre: *mut *mut u8,
                                 val: *mut yyjson_val,
                                 msg: *mut *const c_char) -> bool {
    unsafe {
        let hdr: *mut u8 = *ptr;
        let end: *mut *mut u8 = ptr;
        let mut src: *mut u8 = hdr;
        let mut dst: *mut u8 = core::ptr::null_mut();
        let mut uni: u32;

        /* yyjson.c:5099  return_err(_end, _msg) */
        macro_rules! return_err {
            ($end_:expr, $m:expr) => {{
                *msg = $m;
                *end = $end_;
                return false;
            }};
        }
        /* yyjson.c:5105  return_suc(_str_end, _cur_end) */
        macro_rules! return_suc {
            ($str_end:expr, $cur_end:expr) => {{
                let se_: *mut u8 = $str_end;
                (*val).tag = ((se_.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                    | YYJSON_TYPE_STR as u64;
                (*val).uni.str_ = hdr as *const c_char;
                *pre = se_;
                *end = $cur_end;
                return true;
            }};
        }

        /* add null-terminator for previous raw string */
        **pre = b'\0';

        let mut state: u32 = RI_SKIP_ASCII;
        loop {
            match state {
                RI_SKIP_ASCII => {
                    /* skip_ascii: */
                    while glyph_is_id_ascii(*src) {
                        src = src.add(1);
                    }

                    /* skip_ascii_end: */
                    if !glyph_is_id_next(*src) {
                        return_suc!(src, src);
                    }

                    /* skip_utf8: */
                    while *src >= 0x80 {
                        /* has_allow(EXT_WHITESPACE) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE,
                                     true) {
                            if glyph_is_space_ext(*src)
                                && inner_ext_space_len(src) != 0
                            {
                                return_suc!(src, src);
                            }
                        }
                        uni = crate::mem::bytes::raw_read_4(src as *const c_void);
                        if test_utf8_seq2(uni) {
                            src = src.add(2);
                        } else if test_utf8_seq3(uni) {
                            src = src.add(3);
                        } else if test_utf8_seq4(uni) {
                            src = src.add(4);
                        } else {
                            /* has_allow(INVALID_UNICODE) */
                            if !inner_has_rflag(flg,
                                          YYJSON_READ_ALLOW_INVALID_UNICODE,
                                          true) {
                                return_err!(src, MSG_ERR_UTF8);
                            }
                            src = src.add(1);
                        }
                    }
                    if glyph_is_id_ascii(*src) {
                        state = RI_SKIP_ASCII;
                        continue;
                    }

                    /* The escape character appears, we need to copy it. */
                    dst = src;
                    state = RI_COPY_ESCAPE;
                    continue;
                }

                RI_COPY_ESCAPE => {
                    /* copy_escape: */
                    if raw_match_2(src as *mut c_void,
                                    c"\\u".as_ptr() as *const u8) {
                        if !scan_uni_esc(&mut src, &mut dst, msg) {
                            return_err!(src, *msg);
                        }
                    } else {
                        if !glyph_is_id_next(*src) {
                            return_suc!(dst, src);
                        }
                        return_err!(src,
                            c"unexpected character in key".as_ptr()
                                as *const c_char);
                    }
                    state = RI_COPY_ASCII;
                    continue;
                }

                RI_COPY_ASCII => {
                    /* copy_ascii: */
                    loop {
                        let mut i: usize = 0;
                        while i < 16 && glyph_is_id_ascii(*src.add(i)) {
                            i += 1;
                        }
                        if i == 16 {
                            crate::mem::bytes::raw_move_16(dst as *mut c_void,
                                                      src as *const c_void);
                            dst = dst.add(16);
                            src = src.add(16);
                            continue;
                        }
                        crate::mem::bytes::raw_move_forward(dst as *mut c_void,
                                                       src as *mut c_void, i);
                        dst = dst.add(i);
                        src = src.add(i);
                        break;
                    }
                    state = RI_COPY_UTF8;
                    continue;
                }

                _ => {
                    /* copy_utf8: */
                    while *src >= 0x80 {
                        /* non-ASCII character */
                        /* has_allow(EXT_WHITESPACE) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE,
                                     true) {
                            if glyph_is_space_ext(*src)
                                && inner_ext_space_len(src) != 0
                            {
                                return_suc!(dst, src);
                            }
                        }
                        uni = crate::mem::bytes::raw_read_4(src as *const c_void);
                        if test_utf8_seq2(uni) {
                            crate::mem::bytes::raw_dup_2(
                                dst as *mut c_void,
                                &uni as *const u32 as *const c_void);
                            dst = dst.add(2);
                            src = src.add(2);
                        } else if test_utf8_seq3(uni) {
                            raw_dup_4(dst as *mut c_void,
                                        &uni as *const u32 as *const c_void);
                            dst = dst.add(3);
                            src = src.add(3);
                        } else if test_utf8_seq4(uni) {
                            raw_dup_4(dst as *mut c_void,
                                        &uni as *const u32 as *const c_void);
                            dst = dst.add(4);
                            src = src.add(4);
                        } else {
                            /* has_allow(INVALID_UNICODE) */
                            if !inner_has_rflag(flg,
                                          YYJSON_READ_ALLOW_INVALID_UNICODE,
                                          true) {
                                return_err!(src, MSG_ERR_UTF8);
                            }
                            *dst = *src;
                            dst = dst.add(1);
                            src = src.add(1);
                        }
                    }
                    if glyph_is_id_ascii(*src) {
                        state = RI_COPY_ASCII;
                        continue;
                    }
                    state = RI_COPY_ESCAPE;
                    continue;
                }
            }
        }
    }
}

/*==============================================================================
 * MARK: - JSON Reader Implementation (Private) (yyjson.c:5239-6251)
 *
 * The C builds a finite state machine out of `goto` statements, where the
 * state is the program counter. Here the state is an explicit `u32` and each
 * C label becomes one arm of a dispatch `loop`; `goto <label>` becomes
 * `state = ST_<LABEL>; continue;`. The `fail_*` labels are tail positions
 * that only expand `return_err`, so each `goto fail_*` expands `return_err!`
 * directly at the jump site.
 *
 * `YYJSON_READER_DEPTH_LIMIT` is 0 in this build, so `ctn_depth`, the depth
 * checks in `arr_begin` / `obj_begin` and the `fail_depth` label are not
 * compiled and are therefore absent here.
 *============================================================================*/

/* `sizeof(usize) < 8` is false on this target, so the `alc_max` guard inside
   `val_incr` is dead code in the C and is omitted. */

const ST_ARR_BEGIN: u32 = 0;
const ST_ARR_VAL_BEGIN: u32 = 1;
const ST_ARR_VAL_END: u32 = 2;
const ST_ARR_END: u32 = 3;
const ST_OBJ_BEGIN: u32 = 4;
const ST_OBJ_KEY_BEGIN: u32 = 5;
const ST_OBJ_KEY_END: u32 = 6;
const ST_OBJ_VAL_BEGIN: u32 = 7;
const ST_OBJ_VAL_END: u32 = 8;
const ST_OBJ_END: u32 = 9;

/** yyjson.c:5248  Read single value JSON document. */
#[inline(never)]
pub(crate) unsafe fn scan_root_single(hdr: *mut u8, mut cur: *mut u8,
                                      eof: *mut u8, alc: yyjson_alc,
                                      flg: yyjson_read_flag,
                                      err: *mut yyjson_read_err)
    -> *mut yyjson_doc {
    unsafe {
        let hdr_len: usize; /* value count used by doc */
        let alc_num: usize; /* value count capacity */
        let val_hdr: *mut yyjson_val; /* the head of allocated values */
        let val: *mut yyjson_val; /* current value */
        let doc: *mut yyjson_doc; /* the JSON document, equals to val_hdr */
        let mut msg: *const c_char = core::ptr::null(); /* error message */

        let mut raw_end: [u8; 1] = [0]; /* raw end for null-terminator */
        let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();
        /* previous raw end pointer */
        let pre: *mut *mut u8 = &mut raw_ptr;

        /* yyjson.c:5252  return_err(_pos, _code, _msg) */
        macro_rules! return_err {
            ($pos:expr, $code:expr, $m:expr) => {{
                let p_: *mut u8 = $pos;
                if is_truncated_end(hdr, p_, eof, $code, flg) {
                    (*err).pos = eof.offset_from(hdr) as usize;
                    (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                    (*err).msg = MSG_NOT_END;
                } else {
                    (*err).pos = p_.offset_from(hdr) as usize;
                    (*err).code = $code;
                    (*err).msg = $m;
                }
                if !val_hdr.is_null() {
                    (alc.free.unwrap_unchecked())(alc.ctx,
                                                  val_hdr as *mut c_void);
                }
                return core::ptr::null_mut();
            }};
        }

        hdr_len = core::mem::size_of::<yyjson_doc>()
            / core::mem::size_of::<yyjson_val>()
            + ((core::mem::size_of::<yyjson_doc>()
                % core::mem::size_of::<yyjson_val>()) > 0) as usize;
        alc_num = hdr_len + 1; /* single value */

        val_hdr = (alc.malloc.unwrap_unchecked())(
            alc.ctx, alc_num.wrapping_mul(core::mem::size_of::<yyjson_val>()))
            as *mut yyjson_val;
        if val_hdr.is_null() {
            /* fail_alloc */
            return_err!(cur, YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
        val = val_hdr.add(hdr_len);

        'doc_end: {
            if glyph_is_num(*cur) {
                if read_num(&mut cur, pre, flg, val, &mut msg) {
                    break 'doc_end;
                }
                /* fail_number */
                return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER, msg);
            }
            if *cur == b'"' {
                if scan_str(&mut cur, eof, flg, val, &mut msg) {
                    break 'doc_end;
                }
                /* fail_string */
                return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING, msg);
            }
            if *cur == b't' {
                if scan_true(&mut cur, val) {
                    break 'doc_end;
                }
                /* fail_literal_true */
                return_err!(cur, YYJSON_READ_ERROR_LITERAL, MSG_CHAR_T);
            }
            if *cur == b'f' {
                if scan_false(&mut cur, val) {
                    break 'doc_end;
                }
                /* fail_literal_false */
                return_err!(cur, YYJSON_READ_ERROR_LITERAL, MSG_CHAR_F);
            }
            if *cur == b'n' {
                if scan_null(&mut cur, val) {
                    break 'doc_end;
                }
                /* has_allow(INF_AND_NAN) */
                if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true) {
                    if scan_nan(&mut cur, pre, flg, val) {
                        break 'doc_end;
                    }
                }
                /* fail_literal_null */
                return_err!(cur, YYJSON_READ_ERROR_LITERAL, MSG_CHAR_N);
            }
            /* has_allow(INF_AND_NAN) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true) {
                if scan_inf_or_nan(&mut cur, pre, flg, val) {
                    break 'doc_end;
                }
            }
            /* has_allow(SINGLE_QUOTED_STR) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR, true)
                && *cur == b'\''
            {
                if scan_str_sq(&mut cur, eof, flg, val, &mut msg) {
                    break 'doc_end;
                }
                /* fail_string */
                return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING, msg);
            }
            /* fail_character */
            return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, MSG_CHAR);
        }

        /* doc_end: check invalid contents after json document */
        /* has_flg(STOP_WHEN_DONE) */
        if cur < eof && !inner_has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE, false) {
            while glyph_is_space(*cur) {
                cur = cur.add(1);
            }
            /* has_allow(TRIVIA) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                && glyph_is_trivia(*cur)
            {
                if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                    /* fail_comment */
                    return_err!(cur, YYJSON_READ_ERROR_INVALID_COMMENT,
                                MSG_COMMENT);
                }
            }
            if cur < eof {
                /* fail_garbage */
                return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CONTENT,
                            MSG_GARBAGE);
            }
        }

        **pre = b'\0';
        doc = val_hdr as *mut yyjson_doc;
        (*doc).root = val_hdr.add(hdr_len);
        (*doc).alc = alc;
        (*doc).dat_read = cur.offset_from(hdr) as usize;
        (*doc).val_read = 1;
        /* has_flg(INSITU) */
        (*doc).str_pool = if inner_has_rflag(flg, YYJSON_READ_INSITU, false) {
            core::ptr::null_mut()
        } else {
            hdr as *mut c_char
        };
        doc
    }
}

/** yyjson.c:5353  Read JSON document (accept all style, but optimized for
    minify). */
#[inline(always)]
pub(crate) unsafe fn scan_root_minify(hdr: *mut u8, mut cur: *mut u8,
                                      eof: *mut u8, alc: yyjson_alc,
                                      flg: yyjson_read_flag,
                                      err: *mut yyjson_read_err)
    -> *mut yyjson_doc {
    unsafe {
        let dat_len: usize; /* data length in bytes, hint for allocator */
        let hdr_len: usize; /* value count used by yyjson_doc */
        let mut alc_len: usize; /* value count allocated */
        let alc_max: usize; /* maximum value count for allocator */
        let mut ctn_len: usize; /* number of elements in current container */
        let mut val_hdr: *mut yyjson_val; /* head of allocated values */
        let mut val_end: *mut yyjson_val; /* end of allocated values */
        let mut val_tmp: *mut yyjson_val; /* temporary pointer for realloc */
        let mut val: *mut yyjson_val; /* current JSON value */
        let mut ctn: *mut yyjson_val; /* current container */
        let mut ctn_parent: *mut yyjson_val; /* parent of current container */
        let doc: *mut yyjson_doc; /* the JSON document, equals to val_hdr */
        let mut msg: *const c_char = core::ptr::null(); /* error message */

        let mut raw_end: [u8; 1] = [0]; /* raw end for null-terminator */
        let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();
        /* previous raw end pointer */
        let pre: *mut *mut u8 = &mut raw_ptr;

        /* yyjson.c:5357  return_err(_pos, _code, _msg) */
        macro_rules! return_err {
            ($pos:expr, $code:expr, $m:expr) => {{
                let p_: *mut u8 = $pos;
                if is_truncated_end(hdr, p_, eof, $code, flg) {
                    (*err).pos = eof.offset_from(hdr) as usize;
                    (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                    (*err).msg = MSG_NOT_END;
                } else {
                    (*err).pos = p_.offset_from(hdr) as usize;
                    (*err).code = $code;
                    (*err).msg = $m;
                }
                if !val_hdr.is_null() {
                    (alc.free.unwrap_unchecked())(alc.ctx,
                                                  val_hdr as *mut c_void);
                }
                return core::ptr::null_mut();
            }};
        }

        /* yyjson.c:5371  val_incr() */
        macro_rules! val_incr {
            () => {{
                val = val.add(1);
                if val >= val_end {
                    let alc_old: usize = alc_len;
                    let val_ofs: usize = val.offset_from(val_hdr) as usize;
                    let ctn_ofs: usize = ctn.offset_from(val_hdr) as usize;
                    alc_len = alc_len.wrapping_add(alc_len / 2);
                    val_tmp = (alc.realloc.unwrap_unchecked())(
                        alc.ctx, val_hdr as *mut c_void,
                        alc_old.wrapping_mul(
                            core::mem::size_of::<yyjson_val>()),
                        alc_len.wrapping_mul(
                            core::mem::size_of::<yyjson_val>()))
                        as *mut yyjson_val;
                    if val_tmp.is_null() {
                        /* fail_alloc */
                        return_err!(cur,
                                    YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                                    MSG_MALLOC);
                    }
                    val = val_tmp.add(val_ofs);
                    ctn = val_tmp.add(ctn_ofs);
                    val_hdr = val_tmp;
                    val_end = val_tmp.add(alc_len - 2);
                }
            }};
        }

        /* has_flg(STOP_WHEN_DONE) */
        dat_len = if inner_has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE, false) {
            256
        } else {
            eof.offset_from(cur) as usize
        };
        hdr_len = core::mem::size_of::<yyjson_doc>()
            / core::mem::size_of::<yyjson_val>()
            + ((core::mem::size_of::<yyjson_doc>()
                % core::mem::size_of::<yyjson_val>()) > 0) as usize;
        alc_max = crate::mem::bytes::USIZE_MAX / core::mem::size_of::<yyjson_val>();
        alc_len = hdr_len
            .wrapping_add(dat_len / YYJSON_READER_ESTIMATED_MINIFY_RATIO)
            .wrapping_add(4);
        alc_len = crate::mem::bytes::yyjson_min(alc_len, alc_max);

        val_hdr = (alc.malloc.unwrap_unchecked())(
            alc.ctx, alc_len.wrapping_mul(core::mem::size_of::<yyjson_val>()))
            as *mut yyjson_val;
        if val_hdr.is_null() {
            /* fail_alloc */
            return_err!(cur, YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
        /* padding for key-value pair reading */
        val_end = val_hdr.add(alc_len - 2);
        val = val_hdr.add(hdr_len);
        ctn = val;
        ctn_len = 0;

        let mut state: u32;
        let c0: u8 = *cur;
        cur = cur.add(1);
        if c0 == b'{' {
            (*ctn).tag = YYJSON_TYPE_OBJ as u64;
            (*ctn).uni.ofs = 0;
            state = ST_OBJ_KEY_BEGIN;
        } else {
            (*ctn).tag = YYJSON_TYPE_ARR as u64;
            (*ctn).uni.ofs = 0;
            state = ST_ARR_VAL_BEGIN;
        }

        'fsm: loop {
            match state {
                ST_ARR_BEGIN => {
                    /* save current container */
                    (*ctn).tag = ((ctn_len as u64).wrapping_add(1)
                        << YYJSON_TAG_BIT)
                        | ((*ctn).tag & YYJSON_TAG_MASK as u64);

                    /* create a new array value, save parent container
                       offset */
                    val_incr!();
                    (*val).tag = YYJSON_TYPE_ARR as u64;
                    (*val).uni.ofs =
                        (val as *mut u8).offset_from(ctn as *mut u8) as usize;

                    /* push the new array value as current container */
                    ctn = val;
                    ctn_len = 0;
                    state = ST_ARR_VAL_BEGIN;
                    continue 'fsm;
                }

                ST_ARR_VAL_BEGIN => {
                    if *cur == b'{' {
                        cur = cur.add(1);
                        state = ST_OBJ_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        state = ST_ARR_BEGIN;
                        continue 'fsm;
                    }
                    if glyph_is_num(*cur) {
                        val_incr!();
                        ctn_len += 1;
                        if read_num(&mut cur, pre, flg, val, &mut msg) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER,
                                    msg);
                    }
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if *cur == b't' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_true(&mut cur, val) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_T);
                    }
                    if *cur == b'f' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_false(&mut cur, val) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_F);
                    }
                    if *cur == b'n' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_null(&mut cur, val) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        /* has_allow(INF_AND_NAN) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        {
                            if scan_nan(&mut cur, pre, flg, val) {
                                state = ST_ARR_VAL_END;
                                continue 'fsm;
                            }
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_N);
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            state = ST_ARR_END;
                            continue 'fsm;
                        }
                        /* has_allow(TRAILING_COMMAS) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRAILING_COMMAS,
                                     true) {
                            state = ST_ARR_END;
                            continue 'fsm;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        return_err!(cur, YYJSON_READ_ERROR_JSON_STRUCTURE,
                                    MSG_COMMA);
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_ARR_VAL_BEGIN;
                        continue 'fsm;
                    }
                    /* has_allow(INF_AND_NAN) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        && (*cur == b'i' || *cur == b'I' || *cur == b'N')
                    {
                        val_incr!();
                        ctn_len += 1;
                        if scan_inf_or_nan(&mut cur, pre, flg, val) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur,
                                    YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                    MSG_CHAR);
                    }
                    /* has_allow(SINGLE_QUOTED_STR) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR,
                                 true) && *cur == b'\''
                    {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_ARR_VAL_BEGIN;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_CHAR);
                }

                ST_ARR_VAL_END => {
                    if *cur == b',' {
                        cur = cur.add(1);
                        state = ST_ARR_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        state = ST_ARR_END;
                        continue 'fsm;
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_ARR_VAL_END;
                        continue 'fsm;
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_ARR_END);
                }

                ST_ARR_END => {
                    /* get parent container */
                    ctn_parent = ((ctn as *mut u8).sub((*ctn).uni.ofs))
                        as *mut yyjson_val;

                    /* save the next sibling value offset */
                    (*ctn).uni.ofs =
                        ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                            .wrapping_add(core::mem::size_of::<yyjson_val>());
                    (*ctn).tag = ((ctn_len as u64) << YYJSON_TAG_BIT)
                        | YYJSON_TYPE_ARR as u64;
                    if ctn == ctn_parent {
                        break 'fsm;
                    }

                    /* pop parent as current container */
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    if ((*ctn).tag & YYJSON_TYPE_MASK as u64)
                        == YYJSON_TYPE_OBJ as u64
                    {
                        state = ST_OBJ_VAL_END;
                    } else {
                        state = ST_ARR_VAL_END;
                    }
                    continue 'fsm;
                }

                ST_OBJ_BEGIN => {
                    /* push container */
                    (*ctn).tag = ((ctn_len as u64).wrapping_add(1)
                        << YYJSON_TAG_BIT)
                        | ((*ctn).tag & YYJSON_TAG_MASK as u64);
                    val_incr!();
                    (*val).tag = YYJSON_TYPE_OBJ as u64;
                    /* offset to the parent */
                    (*val).uni.ofs =
                        (val as *mut u8).offset_from(ctn as *mut u8) as usize;
                    ctn = val;
                    ctn_len = 0;
                    state = ST_OBJ_KEY_BEGIN;
                    continue 'fsm;
                }

                ST_OBJ_KEY_BEGIN => {
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            state = ST_OBJ_END;
                            continue 'fsm;
                        }
                        /* has_allow(TRAILING_COMMAS) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRAILING_COMMAS,
                                     true) {
                            state = ST_OBJ_END;
                            continue 'fsm;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        return_err!(cur, YYJSON_READ_ERROR_JSON_STRUCTURE,
                                    MSG_COMMA);
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_OBJ_KEY_BEGIN;
                        continue 'fsm;
                    }
                    /* has_allow(SINGLE_QUOTED_STR) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR,
                                 true) && *cur == b'\''
                    {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    /* has_allow(UNQUOTED_KEY) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_UNQUOTED_KEY, true)
                        && glyph_is_id_start(*cur)
                    {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str_id(&mut cur, eof, flg, pre, val, &mut msg)
                        {
                            state = ST_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_OBJ_KEY_BEGIN;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_KEY);
                }

                ST_OBJ_KEY_END => {
                    if *cur == b':' {
                        cur = cur.add(1);
                        state = ST_OBJ_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_OBJ_KEY_END;
                        continue 'fsm;
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_SEP);
                }

                ST_OBJ_VAL_BEGIN => {
                    if *cur == b'"' {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_str(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if glyph_is_num(*cur) {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_num(&mut cur, pre, flg, val, &mut msg) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER,
                                    msg);
                    }
                    if *cur == b'{' {
                        cur = cur.add(1);
                        state = ST_OBJ_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        state = ST_ARR_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b't' {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_true(&mut cur, val) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_T);
                    }
                    if *cur == b'f' {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_false(&mut cur, val) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_F);
                    }
                    if *cur == b'n' {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_null(&mut cur, val) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        /* has_allow(INF_AND_NAN) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        {
                            if scan_nan(&mut cur, pre, flg, val) {
                                state = ST_OBJ_VAL_END;
                                continue 'fsm;
                            }
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_N);
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_OBJ_VAL_BEGIN;
                        continue 'fsm;
                    }
                    /* has_allow(INF_AND_NAN) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        && (*cur == b'i' || *cur == b'I' || *cur == b'N')
                    {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_inf_or_nan(&mut cur, pre, flg, val) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur,
                                    YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                    MSG_CHAR);
                    }
                    /* has_allow(SINGLE_QUOTED_STR) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR,
                                 true) && *cur == b'\''
                    {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_OBJ_VAL_BEGIN;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_CHAR);
                }

                ST_OBJ_VAL_END => {
                    if *cur == b',' {
                        cur = cur.add(1);
                        state = ST_OBJ_KEY_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        state = ST_OBJ_END;
                        continue 'fsm;
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_OBJ_VAL_END;
                        continue 'fsm;
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_END);
                }

                _ => {
                    /* obj_end: pop container */
                    ctn_parent = ((ctn as *mut u8).sub((*ctn).uni.ofs))
                        as *mut yyjson_val;
                    /* point to the next value */
                    (*ctn).uni.ofs =
                        ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                            .wrapping_add(core::mem::size_of::<yyjson_val>());
                    (*ctn).tag = ((ctn_len as u64) << (YYJSON_TAG_BIT - 1))
                        | YYJSON_TYPE_OBJ as u64;
                    if ctn == ctn_parent {
                        break 'fsm;
                    }
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    if ((*ctn).tag & YYJSON_TYPE_MASK as u64)
                        == YYJSON_TYPE_OBJ as u64
                    {
                        state = ST_OBJ_VAL_END;
                    } else {
                        state = ST_ARR_VAL_END;
                    }
                    continue 'fsm;
                }
            }
        }

        /* doc_end: check invalid contents after json document */
        /* has_flg(STOP_WHEN_DONE) */
        if cur < eof && !inner_has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE, false) {
            while glyph_is_space(*cur) {
                cur = cur.add(1);
            }
            /* has_allow(TRIVIA) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                && glyph_is_trivia(*cur)
            {
                if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                    return_err!(cur, YYJSON_READ_ERROR_INVALID_COMMENT,
                                MSG_COMMENT);
                }
            }
            if cur < eof {
                return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CONTENT,
                            MSG_GARBAGE);
            }
        }

        **pre = b'\0';
        doc = val_hdr as *mut yyjson_doc;
        (*doc).root = val_hdr.add(hdr_len);
        (*doc).alc = alc;
        (*doc).dat_read = cur.offset_from(hdr) as usize;
        (*doc).val_read = (val.offset_from((*doc).root) as usize)
            .wrapping_add(1);
        /* has_flg(INSITU) */
        (*doc).str_pool = if inner_has_rflag(flg, YYJSON_READ_INSITU, false) {
            core::ptr::null_mut()
        } else {
            hdr as *mut c_char
        };
        let _ = alc_max;
        let _ = val_tmp;
        doc
    }
}

/** yyjson.c:5780  Read JSON document (accept all style, but optimized for
    pretty). */
#[inline(always)]
pub(crate) unsafe fn scan_root_pretty(hdr: *mut u8, mut cur: *mut u8,
                                      eof: *mut u8, alc: yyjson_alc,
                                      flg: yyjson_read_flag,
                                      err: *mut yyjson_read_err)
    -> *mut yyjson_doc {
    unsafe {
        let dat_len: usize; /* data length in bytes, hint for allocator */
        let hdr_len: usize; /* value count used by yyjson_doc */
        let mut alc_len: usize; /* value count allocated */
        let alc_max: usize; /* maximum value count for allocator */
        let mut ctn_len: usize; /* number of elements in current container */
        let mut val_hdr: *mut yyjson_val; /* head of allocated values */
        let mut val_end: *mut yyjson_val; /* end of allocated values */
        let mut val_tmp: *mut yyjson_val; /* temporary pointer for realloc */
        let mut val: *mut yyjson_val; /* current JSON value */
        let mut ctn: *mut yyjson_val; /* current container */
        let mut ctn_parent: *mut yyjson_val; /* parent of current container */
        let doc: *mut yyjson_doc; /* the JSON document, equals to val_hdr */
        let mut msg: *const c_char = core::ptr::null(); /* error message */

        let mut raw_end: [u8; 1] = [0]; /* raw end for null-terminator */
        let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();
        /* previous raw end pointer */
        let pre: *mut *mut u8 = &mut raw_ptr;

        /* yyjson.c:5784  return_err(_pos, _code, _msg) */
        macro_rules! return_err {
            ($pos:expr, $code:expr, $m:expr) => {{
                let p_: *mut u8 = $pos;
                if is_truncated_end(hdr, p_, eof, $code, flg) {
                    (*err).pos = eof.offset_from(hdr) as usize;
                    (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                    (*err).msg = MSG_NOT_END;
                } else {
                    (*err).pos = p_.offset_from(hdr) as usize;
                    (*err).code = $code;
                    (*err).msg = $m;
                }
                if !val_hdr.is_null() {
                    (alc.free.unwrap_unchecked())(alc.ctx,
                                                  val_hdr as *mut c_void);
                }
                return core::ptr::null_mut();
            }};
        }

        /* yyjson.c:5798  val_incr() */
        macro_rules! val_incr {
            () => {{
                val = val.add(1);
                if val >= val_end {
                    let alc_old: usize = alc_len;
                    let val_ofs: usize = val.offset_from(val_hdr) as usize;
                    let ctn_ofs: usize = ctn.offset_from(val_hdr) as usize;
                    alc_len = alc_len.wrapping_add(alc_len / 2);
                    val_tmp = (alc.realloc.unwrap_unchecked())(
                        alc.ctx, val_hdr as *mut c_void,
                        alc_old.wrapping_mul(
                            core::mem::size_of::<yyjson_val>()),
                        alc_len.wrapping_mul(
                            core::mem::size_of::<yyjson_val>()))
                        as *mut yyjson_val;
                    if val_tmp.is_null() {
                        /* fail_alloc */
                        return_err!(cur,
                                    YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                                    MSG_MALLOC);
                    }
                    val = val_tmp.add(val_ofs);
                    ctn = val_tmp.add(ctn_ofs);
                    val_hdr = val_tmp;
                    val_end = val_tmp.add(alc_len - 2);
                }
            }};
        }

        /* has_flg(STOP_WHEN_DONE) */
        dat_len = if inner_has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE, false) {
            256
        } else {
            eof.offset_from(cur) as usize
        };
        hdr_len = core::mem::size_of::<yyjson_doc>()
            / core::mem::size_of::<yyjson_val>()
            + ((core::mem::size_of::<yyjson_doc>()
                % core::mem::size_of::<yyjson_val>()) > 0) as usize;
        alc_max = crate::mem::bytes::USIZE_MAX / core::mem::size_of::<yyjson_val>();
        alc_len = hdr_len
            .wrapping_add(dat_len / YYJSON_READER_ESTIMATED_PRETTY_RATIO)
            .wrapping_add(4);
        alc_len = crate::mem::bytes::yyjson_min(alc_len, alc_max);

        val_hdr = (alc.malloc.unwrap_unchecked())(
            alc.ctx, alc_len.wrapping_mul(core::mem::size_of::<yyjson_val>()))
            as *mut yyjson_val;
        if val_hdr.is_null() {
            /* fail_alloc */
            return_err!(cur, YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
        /* padding for key-value pair reading */
        val_end = val_hdr.add(alc_len - 2);
        val = val_hdr.add(hdr_len);
        ctn = val;
        ctn_len = 0;

        let mut state: u32;
        let c0: u8 = *cur;
        cur = cur.add(1);
        if c0 == b'{' {
            (*ctn).tag = YYJSON_TYPE_OBJ as u64;
            (*ctn).uni.ofs = 0;
            if *cur == b'\n' {
                cur = cur.add(1);
            }
            state = ST_OBJ_KEY_BEGIN;
        } else {
            (*ctn).tag = YYJSON_TYPE_ARR as u64;
            (*ctn).uni.ofs = 0;
            if *cur == b'\n' {
                cur = cur.add(1);
            }
            state = ST_ARR_VAL_BEGIN;
        }

        'fsm: loop {
            match state {
                ST_ARR_BEGIN => {
                    /* save current container */
                    (*ctn).tag = ((ctn_len as u64).wrapping_add(1)
                        << YYJSON_TAG_BIT)
                        | ((*ctn).tag & YYJSON_TAG_MASK as u64);

                    /* create a new array value, save parent container
                       offset */
                    val_incr!();
                    (*val).tag = YYJSON_TYPE_ARR as u64;
                    (*val).uni.ofs =
                        (val as *mut u8).offset_from(ctn as *mut u8) as usize;

                    /* push the new array value as current container */
                    ctn = val;
                    ctn_len = 0;
                    if *cur == b'\n' {
                        cur = cur.add(1);
                    }
                    state = ST_ARR_VAL_BEGIN;
                    continue 'fsm;
                }

                ST_ARR_VAL_BEGIN => {
                    while raw_match_2(cur as *mut c_void,
                                       c"  ".as_ptr() as *const u8) {
                        cur = cur.add(2);
                    }

                    if *cur == b'{' {
                        cur = cur.add(1);
                        state = ST_OBJ_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        state = ST_ARR_BEGIN;
                        continue 'fsm;
                    }
                    if glyph_is_num(*cur) {
                        val_incr!();
                        ctn_len += 1;
                        if read_num(&mut cur, pre, flg, val, &mut msg) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER,
                                    msg);
                    }
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if *cur == b't' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_true(&mut cur, val) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_T);
                    }
                    if *cur == b'f' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_false(&mut cur, val) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_F);
                    }
                    if *cur == b'n' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_null(&mut cur, val) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        /* has_allow(INF_AND_NAN) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        {
                            if scan_nan(&mut cur, pre, flg, val) {
                                state = ST_ARR_VAL_END;
                                continue 'fsm;
                            }
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_N);
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            state = ST_ARR_END;
                            continue 'fsm;
                        }
                        /* has_allow(TRAILING_COMMAS) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRAILING_COMMAS,
                                     true) {
                            state = ST_ARR_END;
                            continue 'fsm;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        return_err!(cur, YYJSON_READ_ERROR_JSON_STRUCTURE,
                                    MSG_COMMA);
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_ARR_VAL_BEGIN;
                        continue 'fsm;
                    }
                    /* has_allow(INF_AND_NAN) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        && (*cur == b'i' || *cur == b'I' || *cur == b'N')
                    {
                        val_incr!();
                        ctn_len += 1;
                        if scan_inf_or_nan(&mut cur, pre, flg, val) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur,
                                    YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                    MSG_CHAR);
                    }
                    /* has_allow(SINGLE_QUOTED_STR) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR,
                                 true) && *cur == b'\''
                    {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_ARR_VAL_BEGIN;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_CHAR);
                }

                ST_ARR_VAL_END => {
                    if raw_match_2(cur as *mut c_void,
                                    c",\n".as_ptr() as *const u8) {
                        cur = cur.add(2);
                        state = ST_ARR_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b',' {
                        cur = cur.add(1);
                        state = ST_ARR_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        state = ST_ARR_END;
                        continue 'fsm;
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_ARR_VAL_END;
                        continue 'fsm;
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_ARR_VAL_END;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_ARR_END);
                }

                ST_ARR_END => {
                    /* get parent container */
                    ctn_parent = ((ctn as *mut u8).sub((*ctn).uni.ofs))
                        as *mut yyjson_val;

                    /* save the next sibling value offset */
                    (*ctn).uni.ofs =
                        ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                            .wrapping_add(core::mem::size_of::<yyjson_val>());
                    (*ctn).tag = ((ctn_len as u64) << YYJSON_TAG_BIT)
                        | YYJSON_TYPE_ARR as u64;
                    if ctn == ctn_parent {
                        break 'fsm;
                    }

                    /* pop parent as current container */
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    if *cur == b'\n' {
                        cur = cur.add(1);
                    }
                    if ((*ctn).tag & YYJSON_TYPE_MASK as u64)
                        == YYJSON_TYPE_OBJ as u64
                    {
                        state = ST_OBJ_VAL_END;
                    } else {
                        state = ST_ARR_VAL_END;
                    }
                    continue 'fsm;
                }

                ST_OBJ_BEGIN => {
                    /* push container */
                    (*ctn).tag = ((ctn_len as u64).wrapping_add(1)
                        << YYJSON_TAG_BIT)
                        | ((*ctn).tag & YYJSON_TAG_MASK as u64);
                    val_incr!();
                    (*val).tag = YYJSON_TYPE_OBJ as u64;
                    /* offset to the parent */
                    (*val).uni.ofs =
                        (val as *mut u8).offset_from(ctn as *mut u8) as usize;
                    ctn = val;
                    ctn_len = 0;
                    if *cur == b'\n' {
                        cur = cur.add(1);
                    }
                    state = ST_OBJ_KEY_BEGIN;
                    continue 'fsm;
                }

                ST_OBJ_KEY_BEGIN => {
                    while raw_match_2(cur as *mut c_void,
                                       c"  ".as_ptr() as *const u8) {
                        cur = cur.add(2);
                    }
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            state = ST_OBJ_END;
                            continue 'fsm;
                        }
                        /* has_allow(TRAILING_COMMAS) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRAILING_COMMAS,
                                     true) {
                            state = ST_OBJ_END;
                            continue 'fsm;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        return_err!(cur, YYJSON_READ_ERROR_JSON_STRUCTURE,
                                    MSG_COMMA);
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_OBJ_KEY_BEGIN;
                        continue 'fsm;
                    }
                    /* has_allow(SINGLE_QUOTED_STR) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR,
                                 true) && *cur == b'\''
                    {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    /* has_allow(UNQUOTED_KEY) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_UNQUOTED_KEY, true)
                        && glyph_is_id_start(*cur)
                    {
                        val_incr!();
                        ctn_len += 1;
                        if scan_str_id(&mut cur, eof, flg, pre, val, &mut msg)
                        {
                            state = ST_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_OBJ_KEY_BEGIN;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_KEY);
                }

                ST_OBJ_KEY_END => {
                    if raw_match_2(cur as *mut c_void,
                                    c": ".as_ptr() as *const u8) {
                        cur = cur.add(2);
                        state = ST_OBJ_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b':' {
                        cur = cur.add(1);
                        state = ST_OBJ_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_OBJ_KEY_END;
                        continue 'fsm;
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_SEP);
                }

                ST_OBJ_VAL_BEGIN => {
                    if *cur == b'"' {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_str(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if glyph_is_num(*cur) {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_num(&mut cur, pre, flg, val, &mut msg) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER,
                                    msg);
                    }
                    if *cur == b'{' {
                        cur = cur.add(1);
                        state = ST_OBJ_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        state = ST_ARR_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b't' {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_true(&mut cur, val) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_T);
                    }
                    if *cur == b'f' {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_false(&mut cur, val) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_F);
                    }
                    if *cur == b'n' {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_null(&mut cur, val) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        /* has_allow(INF_AND_NAN) */
                        if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        {
                            if scan_nan(&mut cur, pre, flg, val) {
                                state = ST_OBJ_VAL_END;
                                continue 'fsm;
                            }
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_N);
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_OBJ_VAL_BEGIN;
                        continue 'fsm;
                    }
                    /* has_allow(INF_AND_NAN) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        && (*cur == b'i' || *cur == b'I' || *cur == b'N')
                    {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_inf_or_nan(&mut cur, pre, flg, val) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur,
                                    YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                    MSG_CHAR);
                    }
                    /* has_allow(SINGLE_QUOTED_STR) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR,
                                 true) && *cur == b'\''
                    {
                        val = val.add(1);
                        ctn_len += 1;
                        if scan_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_OBJ_VAL_BEGIN;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_CHAR);
                }

                ST_OBJ_VAL_END => {
                    if raw_match_2(cur as *mut c_void,
                                    c",\n".as_ptr() as *const u8) {
                        cur = cur.add(2);
                        state = ST_OBJ_KEY_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b',' {
                        cur = cur.add(1);
                        state = ST_OBJ_KEY_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        state = ST_OBJ_END;
                        continue 'fsm;
                    }
                    if glyph_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !glyph_is_space(*cur) {
                                break;
                            }
                        }
                        state = ST_OBJ_VAL_END;
                        continue 'fsm;
                    }
                    /* has_allow(TRIVIA) */
                    if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                        && glyph_is_trivia(*cur)
                    {
                        if skip_trivia(&mut cur, eof, flg) {
                            state = ST_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        if cur == eof {
                            return_err!(cur,
                                        YYJSON_READ_ERROR_INVALID_COMMENT,
                                        MSG_COMMENT);
                        }
                    }
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_END);
                }

                _ => {
                    /* obj_end: pop container */
                    ctn_parent = ((ctn as *mut u8).sub((*ctn).uni.ofs))
                        as *mut yyjson_val;
                    /* point to the next value */
                    (*ctn).uni.ofs =
                        ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                            .wrapping_add(core::mem::size_of::<yyjson_val>());
                    (*ctn).tag = ((ctn_len as u64) << (YYJSON_TAG_BIT - 1))
                        | YYJSON_TYPE_OBJ as u64;
                    if ctn == ctn_parent {
                        break 'fsm;
                    }
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    if *cur == b'\n' {
                        cur = cur.add(1);
                    }
                    if ((*ctn).tag & YYJSON_TYPE_MASK as u64)
                        == YYJSON_TYPE_OBJ as u64
                    {
                        state = ST_OBJ_VAL_END;
                    } else {
                        state = ST_ARR_VAL_END;
                    }
                    continue 'fsm;
                }
            }
        }

        /* doc_end: check invalid contents after json document */
        /* has_flg(STOP_WHEN_DONE) */
        if cur < eof && !inner_has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE, false) {
            while glyph_is_space(*cur) {
                cur = cur.add(1);
            }
            /* has_allow(TRIVIA) */
            if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                && glyph_is_trivia(*cur)
            {
                if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                    return_err!(cur, YYJSON_READ_ERROR_INVALID_COMMENT,
                                MSG_COMMENT);
                }
            }
            if cur < eof {
                return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CONTENT,
                            MSG_GARBAGE);
            }
        }

        **pre = b'\0';
        doc = val_hdr as *mut yyjson_doc;
        (*doc).root = val_hdr.add(hdr_len);
        (*doc).alc = alc;
        (*doc).dat_read = cur.offset_from(hdr) as usize;
        (*doc).val_read = (val.offset_from((*doc).root) as usize)
            .wrapping_add(1);
        /* has_flg(INSITU) */
        (*doc).str_pool = if inner_has_rflag(flg, YYJSON_READ_INSITU, false) {
            core::ptr::null_mut()
        } else {
            hdr as *mut c_char
        };
        let _ = alc_max;
        let _ = val_tmp;
        doc
    }
}

/*==============================================================================
 * MARK: - JSON Reader (Public) (yyjson.c:6251-6542)
 *============================================================================*/

/// `yyjson_doc *yyjson_read_opts(char *dat, size_t len, yyjson_read_flag flg,
///                               const yyjson_alc *alc, yyjson_read_err *err)`
/// yyjson.c:6255
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_read_opts(dat: *mut c_char, len: usize,
                                          flg: yyjson_read_flag,
                                          alc_ptr: *const yyjson_alc,
                                          mut err: *mut yyjson_read_err)
    -> *mut yyjson_doc {
    unsafe {
        let mut tmp_err: yyjson_read_err = yyjson_read_err {
            code: 0,
            msg: core::ptr::null(),
            pos: 0,
        };
        let alc: yyjson_alc = if !alc_ptr.is_null() {
            *alc_ptr
        } else {
            crate::mem::pools::YYJSON_DEFAULT_ALC
        };
        let doc: *mut yyjson_doc;
        let mut hdr: *mut u8 = core::ptr::null_mut();
        let eof: *mut u8;
        let mut cur: *mut u8;

        /* yyjson.c:6259  return_err(_pos, _code, _msg) */
        macro_rules! return_err {
            ($pos:expr, $code:expr, $m:expr) => {{
                (*err).pos = $pos;
                (*err).msg = $m;
                (*err).code = $code;
                /* has_flg(INSITU) */
                if !inner_has_rflag(flg, YYJSON_READ_INSITU, false)
                    && !hdr.is_null()
                {
                    (alc.free.unwrap_unchecked())(alc.ctx,
                                                  hdr as *mut c_void);
                }
                return core::ptr::null_mut();
            }};
        }

        /* validate input parameters */
        if err.is_null() {
            err = &mut tmp_err;
        }
        if dat.is_null() {
            return_err!(0, YYJSON_READ_ERROR_INVALID_PARAMETER,
                        c"input data is NULL".as_ptr() as *const c_char);
        }
        if len == 0 {
            return_err!(0, YYJSON_READ_ERROR_INVALID_PARAMETER,
                        c"input length is 0".as_ptr() as *const c_char);
        }

        /* add 4-byte zero padding for input data if necessary */
        /* has_flg(INSITU) */
        if inner_has_rflag(flg, YYJSON_READ_INSITU, false) {
            hdr = dat as *mut u8;
            eof = (dat as *mut u8).add(len);
            cur = dat as *mut u8;
        } else {
            if len >= crate::mem::bytes::USIZE_MAX - YYJSON_PADDING_SIZE {
                return_err!(0, YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                            MSG_MALLOC);
            }
            hdr = (alc.malloc.unwrap_unchecked())(
                alc.ctx, len.wrapping_add(YYJSON_PADDING_SIZE)) as *mut u8;
            if hdr.is_null() {
                return_err!(0, YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                            MSG_MALLOC);
            }
            eof = hdr.add(len);
            cur = hdr;
            core::ptr::copy_nonoverlapping(dat as *const u8, hdr, len);
        }
        core::ptr::write_bytes(eof, 0, YYJSON_PADDING_SIZE);

        /* has_allow(BOM) */
        if inner_has_rflag(flg, YYJSON_READ_ALLOW_BOM, true) {
            if len >= 3 && test_utf8_bom(cur) {
                cur = cur.add(3);
            }
        }

        /* skip empty contents before json document */
        if !glyph_is_ctn(*cur) {
            while glyph_is_space(*cur) {
                cur = cur.add(1);
            }
            if !glyph_is_ctn(*cur) {
                /* has_allow(TRIVIA) */
                if inner_has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA, true)
                    && glyph_is_trivia(*cur)
                {
                    if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                        return_err!(cur.offset_from(hdr) as usize,
                                    YYJSON_READ_ERROR_INVALID_COMMENT,
                                    MSG_COMMENT);
                    }
                }
            }
            if cur >= eof {
                return_err!(0, YYJSON_READ_ERROR_EMPTY_CONTENT,
                            c"input data is empty".as_ptr() as *const c_char);
            }
        }

        /* read json document */
        if glyph_is_ctn(*cur) {
            if glyph_is_space(*cur.add(1)) && glyph_is_space(*cur.add(2)) {
                doc = scan_root_pretty(hdr, cur, eof, alc, flg, err);
            } else {
                doc = scan_root_minify(hdr, cur, eof, alc, flg, err);
            }
        } else {
            doc = scan_root_single(hdr, cur, eof, alc, flg, err);
        }

        /* check result */
        if !doc.is_null() {
            core::ptr::write_bytes(err, 0, 1);
        } else {
            /* RFC 8259: JSON text MUST be encoded using UTF-8 */
            if (*err).pos == 0
                && (*err).code != YYJSON_READ_ERROR_MEMORY_ALLOCATION
            {
                if test_utf8_bom(hdr) {
                    (*err).msg = MSG_ERR_BOM;
                } else if len >= 4 && test_utf32_bom(hdr) {
                    (*err).msg = MSG_ERR_UTF32;
                } else if len >= 2 && test_utf16_bom(hdr) {
                    (*err).msg = MSG_ERR_UTF16;
                }
            }
            /* has_flg(INSITU) */
            if !inner_has_rflag(flg, YYJSON_READ_INSITU, false) {
                (alc.free.unwrap_unchecked())(alc.ctx, hdr as *mut c_void);
            }
        }
        doc
    }
}

/// `yyjson_doc *yyjson_read_file(const char *path, yyjson_read_flag flg,
///                               const yyjson_alc *alc, yyjson_read_err *err)`
/// yyjson.c:6345
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_read_file(path: *const c_char,
                                          flg: yyjson_read_flag,
                                          alc_ptr: *const yyjson_alc,
                                          mut err: *mut yyjson_read_err)
    -> *mut yyjson_doc {
    unsafe {
        let mut tmp_err: yyjson_read_err = yyjson_read_err {
            code: 0,
            msg: core::ptr::null(),
            pos: 0,
        };
        let doc: *mut yyjson_doc;
        let file: *mut FILE;

        /* yyjson.c:6349  return_err(_code, _msg) */
        macro_rules! return_err {
            ($code:expr, $m:expr) => {{
                (*err).pos = 0;
                (*err).msg = $m;
                (*err).code = $code;
                return core::ptr::null_mut();
            }};
        }

        if err.is_null() {
            err = &mut tmp_err;
        }
        if path.is_null() {
            return_err!(YYJSON_READ_ERROR_INVALID_PARAMETER,
                        c"input path is NULL".as_ptr() as *const c_char);
        }

        file = fopen_readonly(path);
        if file.is_null() {
            return_err!(YYJSON_READ_ERROR_FILE_OPEN, MSG_FREAD);
        }

        doc = yyjson_read_fp(file, flg, alc_ptr, err);
        ffi::fclose(file);
        doc
    }
}

/// `yyjson_doc *yyjson_read_fp(FILE *fp, yyjson_read_flag flg,
///                             const yyjson_alc *alc, yyjson_read_err *err)`
/// yyjson.c:6373
#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn yyjson_read_fp(file: *mut FILE,
                                        mut flg: yyjson_read_flag,
                                        alc_ptr: *const yyjson_alc,
                                        mut err: *mut yyjson_read_err)
    -> *mut yyjson_doc {
    unsafe {
        let mut tmp_err: yyjson_read_err = yyjson_read_err {
            code: 0,
            msg: core::ptr::null(),
            pos: 0,
        };
        let alc: yyjson_alc = if !alc_ptr.is_null() {
            *alc_ptr
        } else {
            crate::mem::pools::YYJSON_DEFAULT_ALC
        };
        let doc: *mut yyjson_doc;

        let mut file_size: c_long = 0;
        let file_pos: c_long;
        let mut buf: *mut c_void = core::ptr::null_mut();
        let mut buf_size: usize = 0;
        let mut dat_size: usize = 0;

        /* yyjson.c:6377  return_err(_code, _msg) */
        macro_rules! return_err {
            ($code:expr, $m:expr) => {{
                (*err).pos = 0;
                (*err).msg = $m;
                (*err).code = $code;
                if !buf.is_null() {
                    (alc.free.unwrap_unchecked())(alc.ctx, buf);
                }
                return core::ptr::null_mut();
            }};
        }

        /* validate input parameters */
        if err.is_null() {
            err = &mut tmp_err;
        }
        if file.is_null() {
            return_err!(YYJSON_READ_ERROR_INVALID_PARAMETER,
                        c"input file is NULL".as_ptr() as *const c_char);
        }

        /* get current position */
        file_pos = ffi::ftell(file);
        if file_pos != -1 {
            /* get total file size, may fail */
            if ffi::fseek(file, 0, ffi::SEEK_END) == 0 {
                file_size = ffi::ftell(file);
                if file_size == -1 {
                    file_size = 0;
                }
            }
            /* reset to original position, may fail */
            if ffi::fseek(file, file_pos, ffi::SEEK_SET) != 0 {
                file_size = 0;
            }
            /* get file size from current position to end */
            if file_size > 0 {
                file_size -= file_pos;
            }
        }

        /* read file */
        if file_size > 0 {
            /* read the entire file in one call */
            dat_size = file_size as usize;
            buf_size = dat_size.wrapping_add(YYJSON_PADDING_SIZE);
            buf = (alc.malloc.unwrap_unchecked())(alc.ctx, buf_size);
            if buf.is_null() {
                return_err!(YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
            }
            if fread_safe(buf, dat_size, file) != dat_size {
                return_err!(YYJSON_READ_ERROR_FILE_READ, MSG_FREAD);
            }
        } else {
            /* failed to get file size, read it as a stream */
            let chunk_min: usize = 64;
            let chunk_max: usize = 512 * 1024 * 1024;
            let mut chunk_now: usize = chunk_min;
            let mut read_size: usize;
            let mut tmp: *mut c_void;

            buf_size = YYJSON_PADDING_SIZE;
            loop {
                if buf_size.wrapping_add(chunk_now) < buf_size {
                    /* overflow */
                    return_err!(YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                                MSG_MALLOC);
                }
                buf_size = buf_size.wrapping_add(chunk_now);
                if buf.is_null() {
                    buf = (alc.malloc.unwrap_unchecked())(alc.ctx, buf_size);
                    if buf.is_null() {
                        return_err!(YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                                    MSG_MALLOC);
                    }
                } else {
                    tmp = (alc.realloc.unwrap_unchecked())(
                        alc.ctx, buf, buf_size.wrapping_sub(chunk_now),
                        buf_size);
                    if tmp.is_null() {
                        return_err!(YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                                    MSG_MALLOC);
                    }
                    buf = tmp;
                }
                tmp = (buf as *mut u8)
                    .add(buf_size)
                    .sub(YYJSON_PADDING_SIZE)
                    .sub(chunk_now) as *mut c_void;
                read_size = fread_safe(tmp, chunk_now, file);
                dat_size = dat_size.wrapping_add(read_size);
                if read_size != chunk_now {
                    if ffi::ferror(file) != 0 {
                        return_err!(YYJSON_READ_ERROR_FILE_READ, MSG_FREAD);
                    }
                    break;
                }

                chunk_now = chunk_now.wrapping_mul(2);
                if chunk_now > chunk_max {
                    chunk_now = chunk_max;
                }
            }
        }

        /* read JSON */
        core::ptr::write_bytes((buf as *mut u8).add(dat_size), 0,
                               YYJSON_PADDING_SIZE);
        flg |= YYJSON_READ_INSITU;
        doc = yyjson_read_opts(buf as *mut c_char, dat_size, flg, &alc, err);
        if !doc.is_null() {
            (*doc).str_pool = buf as *mut c_char;
            doc
        } else {
            (alc.free.unwrap_unchecked())(alc.ctx, buf);
            core::ptr::null_mut()
        }
    }
}

/// `const char *yyjson_read_number(const char *dat, yyjson_val *val,
///                                 yyjson_read_flag flg, const yyjson_alc *alc,
///                                 yyjson_read_err *err)`
/// yyjson.c:6476
///
/// `YYJSON_DISABLE_FAST_FP_CONV` is 0 in this build, so the `buf` / `dat_len`
/// locals and the copy-to-buffer path at yyjson.c:6495-6533 are not compiled
/// and are absent here; `alc` is therefore unused, exactly as in the C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_read_number(dat: *const c_char,
                                            val: *mut yyjson_val,
                                            flg: yyjson_read_flag,
                                            _alc: *const yyjson_alc,
                                            mut err: *mut yyjson_read_err)
    -> *const c_char {
    unsafe {
        let hdr: *mut u8 = dat as *mut u8;
        let mut cur: *mut u8 = hdr;
        let mut raw_end: [u8; 1] = [0]; /* raw end for null-terminator */
        let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();
        /* previous raw end pointer */
        let pre: *mut *mut u8 = &mut raw_ptr;
        let mut msg: *const c_char = core::ptr::null();
        let mut tmp_err: yyjson_read_err = yyjson_read_err {
            code: 0,
            msg: core::ptr::null(),
            pos: 0,
        };

        /* yyjson.c:6481  return_err(_pos, _code, _msg) */
        macro_rules! return_err {
            ($pos:expr, $code:expr, $m:expr) => {{
                let p_: *mut u8 = $pos;
                (*err).pos = if p_ > hdr {
                    p_.offset_from(hdr) as usize
                } else {
                    0
                };
                (*err).msg = $m;
                (*err).code = $code;
                return core::ptr::null();
            }};
        }

        if err.is_null() {
            err = &mut tmp_err;
        }
        if dat.is_null() {
            return_err!(cur, YYJSON_READ_ERROR_INVALID_PARAMETER,
                        c"input data is NULL".as_ptr() as *const c_char);
        }
        if val.is_null() {
            return_err!(cur, YYJSON_READ_ERROR_INVALID_PARAMETER,
                        c"output value is NULL".as_ptr() as *const c_char);
        }

        if !read_num(&mut cur, pre, flg, val, &mut msg) {
            return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER, msg);
        }
        cur as *const c_char
    }
}
