//! JSON reader.
//!
//! Transliteration of the "JSON Reader Utils", "JSON Reader Implementation"
//! and "JSON Reader (Public)" sections of the original `src/yyjson.c`.
//! `read_root_minify()` and `read_root_pretty()` differ only in a handful of
//! fast-path checks, so they share one generic function here, specialised at
//! compile time on `PRETTY`.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alc::alc_or_default;
use crate::libc;
use crate::rnum::*;
use crate::rstr::*;
use crate::types::*;
use crate::util::*;

/* ===== trivia and truncation ============================================ */

/// `skip_trivia`
pub unsafe fn skip_trivia(p: *mut *mut u8, eof: *mut u8, flg: u32) -> bool {
    let hdr = *p;
    let mut cur = *p;

    while cur < eof {
        let loop_begin = cur;

        /* skip standard whitespace */
        while char_is_space(*cur) {
            cur = cur.add(1);
        }

        /* skip extended whitespace */
        if has_rflag(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE) {
            while char_is_space_ext(*cur) {
                let len = ext_space_len(cur);
                cur = cur.add(len);
                if len == 0 {
                    break;
                }
            }
        }

        /* skip comment, do not validate encoding */
        if has_rflag(flg, YYJSON_READ_ALLOW_COMMENTS) && *cur == b'/' {
            if *cur.add(1) == b'/' {
                /* single-line comment */
                cur = cur.add(2);
                if has_rflag(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE) {
                    while cur < eof {
                        if char_is_eol_ext(*cur) {
                            let len = ext_eol_len(cur);
                            cur = cur.add(len);
                            if len != 0 {
                                break;
                            }
                        }
                        cur = cur.add(1);
                    }
                } else {
                    while cur < eof && !char_is_eol(*cur) {
                        cur = cur.add(1);
                    }
                }
            } else if *cur.add(1) == b'*' {
                /* multi-line comment */
                cur = cur.add(2);
                while !byte_match_2(cur, b"*/") && cur < eof {
                    cur = cur.add(1);
                }
                if cur == eof {
                    *p = eof;
                    return false; /* unclosed comment */
                }
                cur = cur.add(2);
            }
        }
        if cur == loop_begin {
            break;
        }
    }
    *p = cur;
    cur > hdr
}

/// `is_truncated_utf8`
unsafe fn is_truncated_utf8(cur: *mut u8, eof: *mut u8) -> bool {
    let len = eof.offset_from(cur) as usize;
    if cur >= eof || len >= 4 {
        return false;
    }
    let c0 = *cur;
    let c1 = *cur.add(1);
    let c2 = *cur.add(2);
    if c0 < 0x80 {
        return false;
    }
    if len == 1 {
        if (c0 & 0xE0) == 0xC0 && (c0 & 0x1E) != 0x00 {
            return true;
        }
        if (c0 & 0xF0) == 0xE0 {
            return true;
        }
        if (c0 & 0xF8) == 0xF0 && (c0 & 0x07) <= 0x04 {
            return true;
        }
    } else if len == 2 {
        if (c0 & 0xF0) == 0xE0 && (c1 & 0xC0) == 0x80 {
            let t = ((c0 & 0x0F) << 1) | ((c1 & 0x20) >> 5);
            return t >= 0x01 && t != 0x1B;
        }
        if (c0 & 0xF8) == 0xF0 && (c1 & 0xC0) == 0x80 {
            let t = ((c0 & 0x07) << 2) | ((c1 & 0x30) >> 4);
            return (0x01..=0x10).contains(&t);
        }
    } else if len == 3
        && (c0 & 0xF8) == 0xF0
        && (c1 & 0xC0) == 0x80
        && (c2 & 0xC0) == 0x80
    {
        let t = ((c0 & 0x07) << 2) | ((c1 & 0x30) >> 4);
        return (0x01..=0x10).contains(&t);
    }
    false
}

/// `is_truncated_str`
unsafe fn is_truncated_str(cur: *mut u8, eof: *mut u8, s: &[u8], case_sensitive: bool) -> bool {
    let len = s.len();
    if (cur as usize + len) <= eof as usize || eof <= cur {
        return false;
    }
    if case_sensitive {
        return libc::memcmp_eq(cur, s.as_ptr(), eof.offset_from(cur) as usize);
    }
    let mut cur = cur;
    let mut i = 0usize;
    while cur < eof {
        if char_to_lower(*cur) != s[i] {
            return false;
        }
        cur = cur.add(1);
        i += 1;
    }
    true
}

/// `is_truncated_end`
pub(crate) unsafe fn is_truncated_end(
    hdr: *mut u8,
    cur: *mut u8,
    eof: *mut u8,
    code: u32,
    flg: u32,
) -> bool {
    let mut cur = cur;
    if cur >= eof {
        return true;
    }
    if code == YYJSON_READ_ERROR_LITERAL
        && (is_truncated_str(cur, eof, b"true", true)
            || is_truncated_str(cur, eof, b"false", true)
            || is_truncated_str(cur, eof, b"null", true))
    {
        return true;
    }
    if code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER
        || code == YYJSON_READ_ERROR_INVALID_NUMBER
        || code == YYJSON_READ_ERROR_LITERAL
    {
        if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN) {
            if *cur == b'-' {
                cur = cur.add(1);
            }
            if is_truncated_str(cur, eof, b"infinity", false)
                || is_truncated_str(cur, eof, b"nan", false)
            {
                return true;
            }
        }
    }
    if code == YYJSON_READ_ERROR_UNEXPECTED_CONTENT
        && has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
        && (hdr as usize + 3) <= cur as usize
        && is_truncated_str(cur.sub(3), eof, b"infinity", false)
    {
        /* e.g. `infin` would be read as `inf` + `in` */
        return true;
    }
    if code == YYJSON_READ_ERROR_INVALID_STRING {
        let len = eof.offset_from(cur) as usize;

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
                    if !char_is_hex(*cur) {
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
                if !hex_load_4(cur, &mut hi) {
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
                if (*cur < b'c' || *cur > b'f') && (*cur < b'C' || *cur > b'F') {
                    return false;
                }
                cur = cur.add(1);
                if cur >= eof {
                    return true;
                }
                if !char_is_hex(*cur) {
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
    if has_rflag(flg, YYJSON_READ_ALLOW_COMMENTS) {
        if code == YYJSON_READ_ERROR_INVALID_COMMENT {
            /* unclosed multiline comment */
            return true;
        }
        if code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER
            && *cur == b'/'
            && (cur as usize + 1) == eof as usize
        {
            /* truncated beginning of comment */
            return true;
        }
    }
    if code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER
        && has_rflag(flg, YYJSON_READ_ALLOW_BOM)
    {
        /* truncated UTF-8 BOM */
        let len = eof.offset_from(cur) as usize;
        if cur == hdr && len < 3 && libc::memcmp_eq(hdr, b"\xEF\xBB\xBF".as_ptr(), len) {
            return true;
        }
    }
    false
}

/* ===== reader failures ================================================== */

#[derive(Clone, Copy, PartialEq)]
pub enum Fail {
    Str,
    Num,
    Alloc,
    TrailingComma,
    LitTrue,
    LitFalse,
    LitNull,
    CharVal,
    /// `return_err(cur, UNEXPECTED_CHARACTER, msg)` with a caller-chosen msg
    /// (used by the incremental reader's root-value error).
    CharMsg,
    CharArrEnd,
    CharObjKey,
    CharObjSep,
    CharObjEnd,
    Comment,
    Garbage,
}

impl Fail {
    /// Maps a failure label to the `(code, msg)` pair of its `return_err`.
    pub fn code_msg(self, msg: *const c_char) -> (u32, *const c_char) {
        match self {
            Fail::Str => (YYJSON_READ_ERROR_INVALID_STRING, msg),
            Fail::Num => (YYJSON_READ_ERROR_INVALID_NUMBER, msg),
            Fail::Alloc => (YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC),
            Fail::TrailingComma => (YYJSON_READ_ERROR_JSON_STRUCTURE, MSG_COMMA),
            Fail::LitTrue => (YYJSON_READ_ERROR_LITERAL, MSG_CHAR_T),
            Fail::LitFalse => (YYJSON_READ_ERROR_LITERAL, MSG_CHAR_F),
            Fail::LitNull => (YYJSON_READ_ERROR_LITERAL, MSG_CHAR_N),
            Fail::CharVal => (YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, MSG_CHAR),
            Fail::CharMsg => (YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, msg),
            Fail::CharArrEnd => (YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, MSG_ARR_END),
            Fail::CharObjKey => (YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, MSG_OBJ_KEY),
            Fail::CharObjSep => (YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, MSG_OBJ_SEP),
            Fail::CharObjEnd => (YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, MSG_OBJ_END),
            Fail::Comment => (YYJSON_READ_ERROR_INVALID_COMMENT, MSG_COMMENT),
            Fail::Garbage => (YYJSON_READ_ERROR_UNEXPECTED_CONTENT, MSG_GARBAGE),
        }
    }
}

/* ===== single-value root ================================================ */

/// `read_root_single`
unsafe fn read_root_single(
    hdr: *mut u8,
    cur: *mut u8,
    eof: *mut u8,
    alc: YyjsonAlc,
    flg: u32,
    err: *mut YyjsonReadErr,
) -> *mut YyjsonDoc {
    let mut cur = cur;
    let mut msg: *const c_char = ptr::null();

    let mut raw_end: [u8; 1] = [0];
    let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();
    let pre: *mut *mut u8 = &mut raw_ptr;

    let mut hdr_len = size_of::<YyjsonDoc>() / size_of::<YyjsonVal>();
    hdr_len += ((size_of::<YyjsonDoc>() % size_of::<YyjsonVal>()) > 0) as usize;
    let alc_num = hdr_len + 1; /* single value */

    let val_hdr = alc.alloc(alc_num * size_of::<YyjsonVal>()) as *mut YyjsonVal;

    macro_rules! return_err {
        ($pos:expr, $code:expr, $m:expr) => {{
            if is_truncated_end(hdr, $pos, eof, $code, flg) {
                (*err).pos = eof.offset_from(hdr) as usize;
                (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                (*err).msg = MSG_NOT_END;
            } else {
                (*err).pos = $pos.offset_from(hdr) as usize;
                (*err).code = $code;
                (*err).msg = $m;
            }
            if !val_hdr.is_null() {
                alc.dealloc(val_hdr as *mut c_void);
            }
            return ptr::null_mut();
        }};
    }

    if val_hdr.is_null() {
        return_err!(
            cur,
            YYJSON_READ_ERROR_MEMORY_ALLOCATION,
            MSG_MALLOC
        );
    }
    let val = val_hdr.add(hdr_len);

    let fail: Option<Fail> = 'doc: {
        if char_is_num(*cur) {
            if read_num(&mut cur, pre, flg, val, &mut msg) {
                break 'doc None;
            }
            break 'doc Some(Fail::Num);
        }
        if *cur == b'"' {
            if read_str(&mut cur, eof, flg, val, &mut msg) {
                break 'doc None;
            }
            break 'doc Some(Fail::Str);
        }
        if *cur == b't' {
            if read_true(&mut cur, val) {
                break 'doc None;
            }
            break 'doc Some(Fail::LitTrue);
        }
        if *cur == b'f' {
            if read_false(&mut cur, val) {
                break 'doc None;
            }
            break 'doc Some(Fail::LitFalse);
        }
        if *cur == b'n' {
            if read_null(&mut cur, val) {
                break 'doc None;
            }
            if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                && read_nan(&mut cur, pre, flg, val)
            {
                break 'doc None;
            }
            break 'doc Some(Fail::LitNull);
        }
        if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
            && read_inf_or_nan(&mut cur, pre, flg, val)
        {
            break 'doc None;
        }
        if has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR) && *cur == b'\'' {
            if read_str_sq(&mut cur, eof, flg, val, &mut msg) {
                break 'doc None;
            }
            break 'doc Some(Fail::Str);
        }
        break 'doc Some(Fail::CharVal);
    };

    if let Some(f) = fail {
        let (code, m) = f.code_msg(msg);
        return_err!(cur, code, m);
    }

    /* doc_end: check invalid contents after the json document */
    if cur < eof && !has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE) {
        while char_is_space(*cur) {
            cur = cur.add(1);
        }
        if has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
            if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                return_err!(cur, YYJSON_READ_ERROR_INVALID_COMMENT, MSG_COMMENT);
            }
        }
        if cur < eof {
            return_err!(
                cur,
                YYJSON_READ_ERROR_UNEXPECTED_CONTENT,
                MSG_GARBAGE
            );
        }
    }

    **pre = 0;
    let doc = val_hdr as *mut YyjsonDoc;
    (*doc).root = val_hdr.add(hdr_len);
    (*doc).alc = alc;
    (*doc).dat_read = cur.offset_from(hdr) as usize;
    (*doc).val_read = 1;
    (*doc).str_pool = if has_rflag(flg, YYJSON_READ_INSITU) {
        ptr::null_mut()
    } else {
        hdr as *mut c_char
    };
    doc
}

/* ===== container root =================================================== */

#[derive(Clone, Copy, PartialEq)]
enum St {
    ArrBegin,
    ArrValBegin,
    ArrValEnd,
    ArrEnd,
    ObjBegin,
    ObjKeyBegin,
    ObjKeyEnd,
    ObjValBegin,
    ObjValEnd,
    ObjEnd,
    DocEnd,
}

/// `read_root_minify` / `read_root_pretty`
unsafe fn read_root_ctn<const PRETTY: bool>(
    hdr: *mut u8,
    cur_in: *mut u8,
    eof: *mut u8,
    alc: YyjsonAlc,
    flg: u32,
    err: *mut YyjsonReadErr,
) -> *mut YyjsonDoc {
    let mut cur = cur_in;
    let mut msg: *const c_char = ptr::null();

    let mut raw_end: [u8; 1] = [0];
    let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();
    let pre: *mut *mut u8 = &mut raw_ptr;

    let dat_len = if has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE) {
        256usize
    } else {
        eof.offset_from(cur) as usize
    };
    let mut hdr_len = size_of::<YyjsonDoc>() / size_of::<YyjsonVal>();
    hdr_len += ((size_of::<YyjsonDoc>() % size_of::<YyjsonVal>()) > 0) as usize;
    let alc_max = USIZE_MAX / size_of::<YyjsonVal>();
    let ratio = if PRETTY {
        YYJSON_READER_ESTIMATED_PRETTY_RATIO
    } else {
        YYJSON_READER_ESTIMATED_MINIFY_RATIO
    };
    let mut alc_len = hdr_len + (dat_len / ratio) + 4;
    if alc_len > alc_max {
        alc_len = alc_max;
    }

    let mut val_hdr = alc.alloc(alc_len * size_of::<YyjsonVal>()) as *mut YyjsonVal;

    macro_rules! return_err {
        ($pos:expr, $code:expr, $m:expr) => {{
            if is_truncated_end(hdr, $pos, eof, $code, flg) {
                (*err).pos = eof.offset_from(hdr) as usize;
                (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                (*err).msg = MSG_NOT_END;
            } else {
                (*err).pos = $pos.offset_from(hdr) as usize;
                (*err).code = $code;
                (*err).msg = $m;
            }
            if !val_hdr.is_null() {
                alc.dealloc(val_hdr as *mut c_void);
            }
            return ptr::null_mut();
        }};
    }

    if val_hdr.is_null() {
        return_err!(
            cur,
            YYJSON_READ_ERROR_MEMORY_ALLOCATION,
            MSG_MALLOC
        );
    }
    /* padding for key-value pair reading */
    let mut val_end = val_hdr.add(alc_len - 2);
    let mut val = val_hdr.add(hdr_len);
    let mut ctn = val;
    let mut ctn_len: usize = 0;

    let mut state: St;
    let c0 = *cur;
    cur = cur.add(1);
    if c0 == b'{' {
        (*ctn).tag = YYJSON_TYPE_OBJ as u64;
        (*ctn).uni.ofs = 0;
        if PRETTY && *cur == b'\n' {
            cur = cur.add(1);
        }
        state = St::ObjKeyBegin;
    } else {
        (*ctn).tag = YYJSON_TYPE_ARR as u64;
        (*ctn).uni.ofs = 0;
        if PRETTY && *cur == b'\n' {
            cur = cur.add(1);
        }
        state = St::ArrValBegin;
    }

    let fail: Fail = 'fsm: loop {
        macro_rules! val_incr {
            () => {{
                val = val.add(1);
                if val >= val_end {
                    let alc_old = alc_len;
                    let val_ofs = val.offset_from(val_hdr) as usize;
                    let ctn_ofs = ctn.offset_from(val_hdr) as usize;
                    alc_len += alc_len / 2;
                    let val_tmp = alc.realloc(
                        val_hdr as *mut c_void,
                        alc_old * size_of::<YyjsonVal>(),
                        alc_len * size_of::<YyjsonVal>(),
                    ) as *mut YyjsonVal;
                    if val_tmp.is_null() {
                        break 'fsm Fail::Alloc;
                    }
                    val = val_tmp.add(val_ofs);
                    ctn = val_tmp.add(ctn_ofs);
                    val_hdr = val_tmp;
                    val_end = val_tmp.add(alc_len - 2);
                }
            }};
        }

        match state {
            St::ArrBegin => {
                /* save current container */
                (*ctn).tag = (((ctn_len as u64) + 1) << YYJSON_TAG_BIT)
                    | ((*ctn).tag & YYJSON_TAG_MASK as u64);

                /* create a new array value, save parent container offset */
                val_incr!();
                (*val).tag = YYJSON_TYPE_ARR as u64;
                (*val).uni.ofs = (val as *mut u8).offset_from(ctn as *mut u8) as usize;

                /* push the new array value as current container */
                ctn = val;
                ctn_len = 0;
                if PRETTY && *cur == b'\n' {
                    cur = cur.add(1);
                }
                state = St::ArrValBegin;
            }

            St::ArrValBegin => {
                if PRETTY {
                    while byte_match_2(cur, b"  ") {
                        cur = cur.add(2);
                    }
                }
                if *cur == b'{' {
                    cur = cur.add(1);
                    state = St::ObjBegin;
                    continue;
                }
                if *cur == b'[' {
                    cur = cur.add(1);
                    state = St::ArrBegin;
                    continue;
                }
                if char_is_num(*cur) {
                    val_incr!();
                    ctn_len += 1;
                    if read_num(&mut cur, pre, flg, val, &mut msg) {
                        state = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Fail::Num;
                }
                if *cur == b'"' {
                    val_incr!();
                    ctn_len += 1;
                    if read_str(&mut cur, eof, flg, val, &mut msg) {
                        state = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Fail::Str;
                }
                if *cur == b't' {
                    val_incr!();
                    ctn_len += 1;
                    if read_true(&mut cur, val) {
                        state = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Fail::LitTrue;
                }
                if *cur == b'f' {
                    val_incr!();
                    ctn_len += 1;
                    if read_false(&mut cur, val) {
                        state = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Fail::LitFalse;
                }
                if *cur == b'n' {
                    val_incr!();
                    ctn_len += 1;
                    if read_null(&mut cur, val) {
                        state = St::ArrValEnd;
                        continue;
                    }
                    if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                        && read_nan(&mut cur, pre, flg, val)
                    {
                        state = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Fail::LitNull;
                }
                if *cur == b']' {
                    cur = cur.add(1);
                    if ctn_len == 0 {
                        state = St::ArrEnd;
                        continue;
                    }
                    if has_rflag(flg, YYJSON_READ_ALLOW_TRAILING_COMMAS) {
                        state = St::ArrEnd;
                        continue;
                    }
                    loop {
                        cur = cur.sub(1);
                        if *cur == b',' {
                            break;
                        }
                    }
                    break 'fsm Fail::TrailingComma;
                }
                if char_is_space(*cur) {
                    loop {
                        cur = cur.add(1);
                        if !char_is_space(*cur) {
                            break;
                        }
                    }
                    continue;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                    && (*cur == b'i' || *cur == b'I' || *cur == b'N')
                {
                    val_incr!();
                    ctn_len += 1;
                    if read_inf_or_nan(&mut cur, pre, flg, val) {
                        state = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Fail::CharVal;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR) && *cur == b'\'' {
                    val_incr!();
                    ctn_len += 1;
                    if read_str_sq(&mut cur, eof, flg, val, &mut msg) {
                        state = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Fail::Str;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                    if skip_trivia(&mut cur, eof, flg) {
                        continue;
                    }
                    if cur == eof {
                        break 'fsm Fail::Comment;
                    }
                }
                break 'fsm Fail::CharVal;
            }

            St::ArrValEnd => {
                if PRETTY && byte_match_2(cur, b",\n") {
                    cur = cur.add(2);
                    state = St::ArrValBegin;
                    continue;
                }
                if *cur == b',' {
                    cur = cur.add(1);
                    state = St::ArrValBegin;
                    continue;
                }
                if *cur == b']' {
                    cur = cur.add(1);
                    state = St::ArrEnd;
                    continue;
                }
                if char_is_space(*cur) {
                    loop {
                        cur = cur.add(1);
                        if !char_is_space(*cur) {
                            break;
                        }
                    }
                    continue;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                    if skip_trivia(&mut cur, eof, flg) {
                        continue;
                    }
                    if cur == eof {
                        break 'fsm Fail::Comment;
                    }
                }
                break 'fsm Fail::CharArrEnd;
            }

            St::ArrEnd => {
                /* get parent container */
                let ctn_parent =
                    (ctn as *mut u8).sub((*ctn).uni.ofs) as *mut YyjsonVal;

                /* save the next sibling value offset */
                (*ctn).uni.ofs = ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                    + size_of::<YyjsonVal>();
                (*ctn).tag = ((ctn_len as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_ARR as u64;
                if ctn == ctn_parent {
                    state = St::DocEnd;
                    continue;
                }

                /* pop parent as current container */
                ctn = ctn_parent;
                ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                if PRETTY && *cur == b'\n' {
                    cur = cur.add(1);
                }
                state = if ((*ctn).tag & YYJSON_TYPE_MASK as u64) == YYJSON_TYPE_OBJ as u64 {
                    St::ObjValEnd
                } else {
                    St::ArrValEnd
                };
            }

            St::ObjBegin => {
                /* push container */
                (*ctn).tag = (((ctn_len as u64) + 1) << YYJSON_TAG_BIT)
                    | ((*ctn).tag & YYJSON_TAG_MASK as u64);
                val_incr!();
                (*val).tag = YYJSON_TYPE_OBJ as u64;
                /* offset to the parent */
                (*val).uni.ofs = (val as *mut u8).offset_from(ctn as *mut u8) as usize;
                ctn = val;
                ctn_len = 0;
                if PRETTY && *cur == b'\n' {
                    cur = cur.add(1);
                }
                state = St::ObjKeyBegin;
            }

            St::ObjKeyBegin => {
                if PRETTY {
                    while byte_match_2(cur, b"  ") {
                        cur = cur.add(2);
                    }
                }
                if *cur == b'"' {
                    val_incr!();
                    ctn_len += 1;
                    if read_str(&mut cur, eof, flg, val, &mut msg) {
                        state = St::ObjKeyEnd;
                        continue;
                    }
                    break 'fsm Fail::Str;
                }
                if *cur == b'}' {
                    cur = cur.add(1);
                    if ctn_len == 0 {
                        state = St::ObjEnd;
                        continue;
                    }
                    if has_rflag(flg, YYJSON_READ_ALLOW_TRAILING_COMMAS) {
                        state = St::ObjEnd;
                        continue;
                    }
                    loop {
                        cur = cur.sub(1);
                        if *cur == b',' {
                            break;
                        }
                    }
                    break 'fsm Fail::TrailingComma;
                }
                if char_is_space(*cur) {
                    loop {
                        cur = cur.add(1);
                        if !char_is_space(*cur) {
                            break;
                        }
                    }
                    continue;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR) && *cur == b'\'' {
                    val_incr!();
                    ctn_len += 1;
                    if read_str_sq(&mut cur, eof, flg, val, &mut msg) {
                        state = St::ObjKeyEnd;
                        continue;
                    }
                    break 'fsm Fail::Str;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_UNQUOTED_KEY) && char_is_id_start(*cur) {
                    val_incr!();
                    ctn_len += 1;
                    if read_str_id(&mut cur, eof, flg, pre, val, &mut msg) {
                        state = St::ObjKeyEnd;
                        continue;
                    }
                    break 'fsm Fail::Str;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                    if skip_trivia(&mut cur, eof, flg) {
                        continue;
                    }
                    if cur == eof {
                        break 'fsm Fail::Comment;
                    }
                }
                break 'fsm Fail::CharObjKey;
            }

            St::ObjKeyEnd => {
                if PRETTY && byte_match_2(cur, b": ") {
                    cur = cur.add(2);
                    state = St::ObjValBegin;
                    continue;
                }
                if *cur == b':' {
                    cur = cur.add(1);
                    state = St::ObjValBegin;
                    continue;
                }
                if char_is_space(*cur) {
                    loop {
                        cur = cur.add(1);
                        if !char_is_space(*cur) {
                            break;
                        }
                    }
                    continue;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                    if skip_trivia(&mut cur, eof, flg) {
                        continue;
                    }
                    if cur == eof {
                        break 'fsm Fail::Comment;
                    }
                }
                break 'fsm Fail::CharObjSep;
            }

            St::ObjValBegin => {
                if *cur == b'"' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_str(&mut cur, eof, flg, val, &mut msg) {
                        state = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Fail::Str;
                }
                if char_is_num(*cur) {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_num(&mut cur, pre, flg, val, &mut msg) {
                        state = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Fail::Num;
                }
                if *cur == b'{' {
                    cur = cur.add(1);
                    state = St::ObjBegin;
                    continue;
                }
                if *cur == b'[' {
                    cur = cur.add(1);
                    state = St::ArrBegin;
                    continue;
                }
                if *cur == b't' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_true(&mut cur, val) {
                        state = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Fail::LitTrue;
                }
                if *cur == b'f' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_false(&mut cur, val) {
                        state = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Fail::LitFalse;
                }
                if *cur == b'n' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_null(&mut cur, val) {
                        state = St::ObjValEnd;
                        continue;
                    }
                    if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                        && read_nan(&mut cur, pre, flg, val)
                    {
                        state = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Fail::LitNull;
                }
                if char_is_space(*cur) {
                    loop {
                        cur = cur.add(1);
                        if !char_is_space(*cur) {
                            break;
                        }
                    }
                    continue;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                    && (*cur == b'i' || *cur == b'I' || *cur == b'N')
                {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_inf_or_nan(&mut cur, pre, flg, val) {
                        state = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Fail::CharVal;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR) && *cur == b'\'' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_str_sq(&mut cur, eof, flg, val, &mut msg) {
                        state = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Fail::Str;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                    if skip_trivia(&mut cur, eof, flg) {
                        continue;
                    }
                    if cur == eof {
                        break 'fsm Fail::Comment;
                    }
                }
                break 'fsm Fail::CharVal;
            }

            St::ObjValEnd => {
                if PRETTY && byte_match_2(cur, b",\n") {
                    cur = cur.add(2);
                    state = St::ObjKeyBegin;
                    continue;
                }
                if *cur == b',' {
                    cur = cur.add(1);
                    state = St::ObjKeyBegin;
                    continue;
                }
                if *cur == b'}' {
                    cur = cur.add(1);
                    state = St::ObjEnd;
                    continue;
                }
                if char_is_space(*cur) {
                    loop {
                        cur = cur.add(1);
                        if !char_is_space(*cur) {
                            break;
                        }
                    }
                    continue;
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                    if skip_trivia(&mut cur, eof, flg) {
                        continue;
                    }
                    if cur == eof {
                        break 'fsm Fail::Comment;
                    }
                }
                break 'fsm Fail::CharObjEnd;
            }

            St::ObjEnd => {
                /* pop container */
                let ctn_parent =
                    (ctn as *mut u8).sub((*ctn).uni.ofs) as *mut YyjsonVal;
                /* point to the next value */
                (*ctn).uni.ofs = ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                    + size_of::<YyjsonVal>();
                (*ctn).tag =
                    ((ctn_len as u64) << (YYJSON_TAG_BIT - 1)) | YYJSON_TYPE_OBJ as u64;
                if ctn == ctn_parent {
                    state = St::DocEnd;
                    continue;
                }
                ctn = ctn_parent;
                ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                if PRETTY && *cur == b'\n' {
                    cur = cur.add(1);
                }
                state = if ((*ctn).tag & YYJSON_TYPE_MASK as u64) == YYJSON_TYPE_OBJ as u64 {
                    St::ObjValEnd
                } else {
                    St::ArrValEnd
                };
            }

            St::DocEnd => {
                /* check invalid contents after the json document */
                if cur < eof && !has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE) {
                    while char_is_space(*cur) {
                        cur = cur.add(1);
                    }
                    if has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                        if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                            break 'fsm Fail::Comment;
                        }
                    }
                    if cur < eof {
                        break 'fsm Fail::Garbage;
                    }
                }

                **pre = 0;
                let doc = val_hdr as *mut YyjsonDoc;
                (*doc).root = val_hdr.add(hdr_len);
                (*doc).alc = alc;
                (*doc).dat_read = cur.offset_from(hdr) as usize;
                (*doc).val_read = (val.offset_from((*doc).root) + 1) as usize;
                (*doc).str_pool = if has_rflag(flg, YYJSON_READ_INSITU) {
                    ptr::null_mut()
                } else {
                    hdr as *mut c_char
                };
                return doc;
            }
        }
    };

    let (code, m) = fail.code_msg(msg);
    return_err!(cur, code, m);
}

/* ===== public reader ==================================================== */

#[no_mangle]
pub unsafe extern "C" fn yyjson_read_opts(
    dat: *mut c_char,
    len: usize,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err_in: *mut YyjsonReadErr,
) -> *mut YyjsonDoc {
    let mut tmp_err = YyjsonReadErr {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };
    let alc = alc_or_default(alc_ptr);
    let mut hdr: *mut u8 = ptr::null_mut();
    let eof: *mut u8;
    let mut cur: *mut u8;

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonReadErr
    } else {
        err_in
    };

    macro_rules! return_err {
        ($pos:expr, $code:expr, $m:expr) => {{
            (*err).pos = $pos;
            (*err).msg = $m;
            (*err).code = $code;
            if !has_rflag(flg, YYJSON_READ_INSITU) && !hdr.is_null() {
                alc.dealloc(hdr as *mut c_void);
            }
            return ptr::null_mut();
        }};
    }

    if dat.is_null() {
        return_err!(0, YYJSON_READ_ERROR_INVALID_PARAMETER, cstr!("input data is NULL"));
    }
    if len == 0 {
        return_err!(0, YYJSON_READ_ERROR_INVALID_PARAMETER, cstr!("input length is 0"));
    }

    /* add 4-byte zero padding for input data if necessary */
    if has_rflag(flg, YYJSON_READ_INSITU) {
        hdr = dat as *mut u8;
        eof = (dat as *mut u8).add(len);
        cur = dat as *mut u8;
    } else {
        if len >= USIZE_MAX - YYJSON_PADDING_SIZE {
            return_err!(0, YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
        hdr = alc.alloc(len + YYJSON_PADDING_SIZE) as *mut u8;
        if hdr.is_null() {
            return_err!(0, YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
        eof = hdr.add(len);
        cur = hdr;
        libc::mem_copy(hdr, dat as *const u8, len);
    }
    libc::mem_set(eof, 0, YYJSON_PADDING_SIZE);

    if has_rflag(flg, YYJSON_READ_ALLOW_BOM) && len >= 3 && is_utf8_bom(cur) {
        cur = cur.add(3);
    }

    /* skip empty contents before the json document */
    if !char_is_ctn(*cur) {
        while char_is_space(*cur) {
            cur = cur.add(1);
        }
        if !char_is_ctn(*cur)
            && has_rflag(flg, YYJSON_READ_ALLOW_TRIVIA)
            && char_is_trivia(*cur)
            && !skip_trivia(&mut cur, eof, flg)
            && cur == eof
        {
            return_err!(
                cur.offset_from(hdr) as usize,
                YYJSON_READ_ERROR_INVALID_COMMENT,
                MSG_COMMENT
            );
        }
        if cur >= eof {
            return_err!(0, YYJSON_READ_ERROR_EMPTY_CONTENT, cstr!("input data is empty"));
        }
    }

    /* read json document */
    let doc = if char_is_ctn(*cur) {
        if char_is_space(*cur.add(1)) && char_is_space(*cur.add(2)) {
            read_root_ctn::<true>(hdr, cur, eof, alc, flg, err)
        } else {
            read_root_ctn::<false>(hdr, cur, eof, alc, flg, err)
        }
    } else {
        read_root_single(hdr, cur, eof, alc, flg, err)
    };

    /* check result */
    if !doc.is_null() {
        libc::zero(err);
    } else {
        /* RFC 8259: JSON text MUST be encoded using UTF-8 */
        if (*err).pos == 0 && (*err).code != YYJSON_READ_ERROR_MEMORY_ALLOCATION {
            if is_utf8_bom(hdr) {
                (*err).msg = MSG_ERR_BOM;
            } else if len >= 4 && is_utf32_bom(hdr) {
                (*err).msg = MSG_ERR_UTF32;
            } else if len >= 2 && is_utf16_bom(hdr) {
                (*err).msg = MSG_ERR_UTF16;
            }
        }
        if !has_rflag(flg, YYJSON_READ_INSITU) {
            alc.dealloc(hdr as *mut c_void);
        }
    }
    doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_read_file(
    path: *const c_char,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err_in: *mut YyjsonReadErr,
) -> *mut YyjsonDoc {
    let mut tmp_err = YyjsonReadErr {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };
    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonReadErr
    } else {
        err_in
    };

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            (*err).pos = 0;
            (*err).msg = $m;
            (*err).code = $code;
            return ptr::null_mut();
        }};
    }

    if path.is_null() {
        return_err!(YYJSON_READ_ERROR_INVALID_PARAMETER, cstr!("input path is NULL"));
    }

    let file = libc::fopen(path, cstr!("rbe"));
    if file.is_null() {
        return_err!(YYJSON_READ_ERROR_FILE_OPEN, MSG_FREAD);
    }

    let doc = yyjson_read_fp(file, flg, alc_ptr, err);
    libc::fclose(file);
    doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_read_fp(
    file: *mut libc::File,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err_in: *mut YyjsonReadErr,
) -> *mut YyjsonDoc {
    let mut tmp_err = YyjsonReadErr {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };
    let alc = alc_or_default(alc_ptr);

    let mut file_size: i64 = 0;
    let mut buf: *mut c_void = ptr::null_mut();
    let mut buf_size: usize;
    let mut dat_size: usize = 0;

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonReadErr
    } else {
        err_in
    };

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            (*err).pos = 0;
            (*err).msg = $m;
            (*err).code = $code;
            if !buf.is_null() {
                alc.dealloc(buf);
            }
            return ptr::null_mut();
        }};
    }

    if file.is_null() {
        return_err!(YYJSON_READ_ERROR_INVALID_PARAMETER, cstr!("input file is NULL"));
    }

    /* get current position */
    let file_pos = libc::ftell(file);
    if file_pos != -1 {
        /* get total file size, may fail */
        if libc::fseek(file, 0, libc::SEEK_END) == 0 {
            file_size = libc::ftell(file) as i64;
            if file_size == -1 {
                file_size = 0;
            }
        }
        /* reset to original position, may fail */
        if libc::fseek(file, file_pos, libc::SEEK_SET) != 0 {
            file_size = 0;
        }
        /* get file size from current position to end */
        if file_size > 0 {
            file_size -= file_pos as i64;
        }
    }

    if file_size > 0 {
        /* read the entire file in one call */
        dat_size = file_size as usize;
        buf_size = dat_size + YYJSON_PADDING_SIZE;
        buf = alc.alloc(buf_size);
        if buf.is_null() {
            return_err!(YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
        if libc::fread(buf, 1, dat_size, file) != dat_size {
            return_err!(YYJSON_READ_ERROR_FILE_READ, MSG_FREAD);
        }
    } else {
        /* failed to get file size, read it as a stream */
        let chunk_min: usize = 64;
        let chunk_max: usize = 512 * 1024 * 1024;
        let mut chunk_now = chunk_min;

        buf_size = YYJSON_PADDING_SIZE;
        loop {
            if buf_size + chunk_now < buf_size {
                /* overflow */
                return_err!(YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
            }
            buf_size += chunk_now;
            if buf.is_null() {
                buf = alc.alloc(buf_size);
                if buf.is_null() {
                    return_err!(YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
                }
            } else {
                let tmp = alc.realloc(buf, buf_size - chunk_now, buf_size);
                if tmp.is_null() {
                    return_err!(YYJSON_READ_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
                }
                buf = tmp;
            }
            let tmp = (buf as *mut u8).add(buf_size - YYJSON_PADDING_SIZE - chunk_now);
            let read_size = libc::fread(tmp as *mut c_void, 1, chunk_now, file);
            dat_size += read_size;
            if read_size != chunk_now {
                if libc::ferror(file) != 0 {
                    return_err!(YYJSON_READ_ERROR_FILE_READ, MSG_FREAD);
                }
                break;
            }

            chunk_now *= 2;
            if chunk_now > chunk_max {
                chunk_now = chunk_max;
            }
        }
    }

    /* read JSON */
    libc::mem_set((buf as *mut u8).add(dat_size), 0, YYJSON_PADDING_SIZE);
    let flg = flg | YYJSON_READ_INSITU;
    let doc = yyjson_read_opts(buf as *mut c_char, dat_size, flg, &alc, err);
    if !doc.is_null() {
        (*doc).str_pool = buf as *mut c_char;
        doc
    } else {
        alc.dealloc(buf);
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_read_number(
    dat: *const c_char,
    val: *mut YyjsonVal,
    flg: u32,
    _alc: *const YyjsonAlc,
    err_in: *mut YyjsonReadErr,
) -> *const c_char {
    let hdr = dat as *mut u8;
    let mut cur = hdr;
    let mut raw_end: [u8; 1] = [0];
    let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();
    let pre: *mut *mut u8 = &mut raw_ptr;
    let mut msg: *const c_char = ptr::null();
    let mut tmp_err = YyjsonReadErr {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonReadErr
    } else {
        err_in
    };

    macro_rules! return_err {
        ($pos:expr, $code:expr, $m:expr) => {{
            (*err).pos = if $pos > hdr {
                $pos.offset_from(hdr) as usize
            } else {
                0
            };
            (*err).msg = $m;
            (*err).code = $code;
            return ptr::null();
        }};
    }

    if dat.is_null() {
        return_err!(
            cur,
            YYJSON_READ_ERROR_INVALID_PARAMETER,
            cstr!("input data is NULL")
        );
    }
    if val.is_null() {
        return_err!(
            cur,
            YYJSON_READ_ERROR_INVALID_PARAMETER,
            cstr!("output value is NULL")
        );
    }

    if !read_num(&mut cur, pre, flg, val, &mut msg) {
        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER, msg);
    }
    cur as *const c_char
}
