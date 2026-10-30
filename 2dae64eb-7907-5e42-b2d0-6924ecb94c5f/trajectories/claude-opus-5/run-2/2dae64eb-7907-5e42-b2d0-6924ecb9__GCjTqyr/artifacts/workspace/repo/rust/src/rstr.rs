//! JSON string reader.
//!
//! Transliteration of the "String Reader (Private)" section of the original
//! `src/yyjson.c`. The `goto` web of `read_str_opt()` is expressed as an
//! explicit state machine; the state names match the C labels one to one.

use core::ffi::c_char;
use core::ptr;

use crate::rnum::has_rflag;
use crate::types::*;
use crate::util::*;

/// `read_uni_esc`
#[inline]
unsafe fn read_uni_esc(
    src_ptr: *mut *mut u8,
    dst_ptr: *mut *mut u8,
    msg: *mut *const c_char,
) -> bool {
    macro_rules! return_err {
        ($end:expr, $m:expr) => {{
            *msg = cstr!($m);
            *src_ptr = $end;
            return false;
        }};
    }

    let mut src = *src_ptr;
    let mut dst = *dst_ptr;
    let mut hi: u16 = 0;
    let mut lo: u16 = 0;

    src = src.add(2); /* skip `\u` */
    if !hex_load_4(src, &mut hi) {
        return_err!(src.sub(2), "invalid escaped sequence in string");
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
            return_err!(src.sub(6), "invalid high surrogate in string");
        }
        if !byte_match_2(src, b"\\u") {
            return_err!(src.sub(6), "no low surrogate in string");
        }
        if !hex_load_4(src.add(2), &mut lo) {
            return_err!(src.sub(6), "invalid escape in string");
        }
        if (lo & 0xFC00) != 0xDC00 {
            return_err!(src.sub(6), "invalid low surrogate in string");
        }
        let uni = ((((hi as u32) - 0xD800) << 10) | ((lo as u32) - 0xDC00)) + 0x10000;
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

#[derive(PartialEq, Clone, Copy)]
enum S {
    SkipAscii,
    SkipUtf8,
    CopyEscape,
    CopyAscii,
    CopyUtf8,
    CopyAsciiStop1,
}

/// `read_str_opt`
pub unsafe fn read_str_opt(
    quo: u8,
    p: *mut *mut u8,
    eof: *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
    msg: *mut *const c_char,
    con: *mut *mut u8,
) -> bool {
    let hdr = (*p).add(1);
    let mut src = hdr;
    let mut dst: *mut u8 = ptr::null_mut();

    macro_rules! return_err {
        ($end:expr, $m:expr) => {{
            *msg = $m;
            *p = $end;
            if !con.is_null() {
                *con = $end;
                *con.add(1) = dst;
            }
            return false;
        }};
    }
    macro_rules! return_err_s {
        ($end:expr, $m:expr) => {
            return_err!($end, cstr!($m))
        };
    }

    let skipf: fn(u8) -> bool = if quo == b'"' {
        char_is_ascii_skip
    } else {
        char_is_ascii_skip_sq
    };

    let mut state = S::SkipAscii;

    /* Resume incremental parsing. */
    if !con.is_null() && !(*con).is_null() {
        src = *con;
        dst = *con.add(1);
        if !dst.is_null() {
            state = S::CopyAscii;
        }
    }

    loop {
        match state {
            S::SkipAscii => {
                /*
                 Most strings have no escaped characters, so we can jump over
                 them quickly; the C unrolls this loop 16 times.
                 */
                loop {
                    let mut stop = 16usize;
                    for i in 0..16usize {
                        if !skipf(*src.add(i)) {
                            stop = i;
                            break;
                        }
                    }
                    if stop < 16 {
                        src = src.add(stop);
                        break;
                    }
                    src = src.add(16);
                }

                /* skip_ascii_end */
                if *src == quo {
                    (*val).tag = ((src.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                        | YYJSON_TYPE_STR as u64
                        | if quo == b'"' {
                            YYJSON_SUBTYPE_NOESC as u64
                        } else {
                            0
                        };
                    (*val).uni.str_ = hdr as *const c_char;
                    *src = 0;
                    *p = src.add(1);
                    if !con.is_null() {
                        *con = ptr::null_mut();
                        *con.add(1) = ptr::null_mut();
                    }
                    return true;
                }
                state = S::SkipUtf8;
            }

            S::SkipUtf8 => {
                if *src & 0x80 != 0 {
                    /* non-ASCII character */
                    let pos = src;
                    let mut uni = byte_load_4(src);
                    while is_utf8_seq3(uni) {
                        src = src.add(3);
                        uni = byte_load_4(src);
                    }
                    if is_utf8_seq1(uni) {
                        state = S::SkipAscii;
                        continue;
                    }
                    while is_utf8_seq2(uni) {
                        src = src.add(2);
                        uni = byte_load_4(src);
                    }
                    while is_utf8_seq4(uni) {
                        src = src.add(4);
                        uni = byte_load_4(src);
                    }
                    if pos == src {
                        if has_rflag(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                            src = src.add(1);
                        } else {
                            return_err_s!(src, "invalid UTF-8 encoding in string");
                        }
                    }
                    state = S::SkipAscii;
                    continue;
                }
                /* The escape character appears, we need to copy it. */
                dst = src;
                state = S::CopyEscape;
            }

            S::CopyEscape => {
                if *src == b'\\' {
                    src = src.add(1);
                    let c = *src;
                    match c {
                        b'"' | b'\\' | b'/' => {
                            *dst = c;
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
                            if !read_uni_esc(&mut src, &mut dst, msg) {
                                return_err!(src, *msg);
                            }
                        }
                        _ => {
                            if has_rflag(flg, YYJSON_READ_ALLOW_EXT_ESCAPE) {
                                /* read extended escape (non-standard) */
                                match c {
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
                                        if !char_is_digit(*src.add(1)) {
                                            *dst = 0;
                                            dst = dst.add(1);
                                            src = src.add(1);
                                        } else {
                                            return_err_s!(
                                                src.sub(1),
                                                "octal escape is not allowed"
                                            );
                                        }
                                    }
                                    b'1'..=b'9' => {
                                        return_err_s!(src.sub(1), "invalid number escape");
                                    }
                                    b'x' => {
                                        let mut cc: u8 = 0;
                                        if hex_load_2(src.add(1), &mut cc) {
                                            src = src.add(3);
                                            if cc <= 0x7F {
                                                /* 1-byte ASCII */
                                                *dst = cc;
                                                dst = dst.add(1);
                                            } else {
                                                /* 2-byte UTF-8 */
                                                *dst = 0xC0 | (cc >> 6);
                                                dst = dst.add(1);
                                                *dst = 0x80 | (cc & 0x3F);
                                                dst = dst.add(1);
                                            }
                                        } else {
                                            return_err_s!(src.sub(1), "invalid hex escape");
                                        }
                                    }
                                    b'\n' => {
                                        src = src.add(1);
                                    }
                                    b'\r' => {
                                        src = src.add(1);
                                        src = src.add((*src == b'\n') as usize);
                                    }
                                    0xE2 => {
                                        /* Line terminator: U+2028, U+2029 */
                                        if (*src.add(1) == 0x80 && *src.add(2) == 0xA8)
                                            || (*src.add(1) == 0x80 && *src.add(2) == 0xA9)
                                        {
                                            src = src.add(3);
                                        }
                                    }
                                    _ => {}
                                }
                            } else if quo == b'\'' && c == b'\'' {
                                *dst = b'\'';
                                dst = dst.add(1);
                                src = src.add(1);
                            } else {
                                return_err_s!(src.sub(1), "invalid escaped sequence in string");
                            }
                        }
                    }
                } else if *src == quo {
                    (*val).tag = ((dst.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                        | YYJSON_TYPE_STR as u64;
                    (*val).uni.str_ = hdr as *const c_char;
                    *dst = 0;
                    *p = src.add(1);
                    if !con.is_null() {
                        *con = ptr::null_mut();
                        *con.add(1) = ptr::null_mut();
                    }
                    return true;
                } else {
                    if !has_rflag(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                        return_err_s!(src, "unexpected control character in string");
                    }
                    if src >= eof {
                        return_err_s!(src, "unclosed string");
                    }
                    *dst = *src;
                    dst = dst.add(1);
                    src = src.add(1);
                }
                state = S::CopyAscii;
            }

            S::CopyAscii => {
                let stopped = loop {
                    let mut stop = 16usize;
                    for i in 0..16usize {
                        if !skipf(*src.add(i)) {
                            stop = i;
                            break;
                        }
                    }
                    if stop < 16 {
                        break stop;
                    }
                    byte_move_16(dst, src);
                    dst = dst.add(16);
                    src = src.add(16);
                };
                /*
                 The memory is copied forward since `dst < src`, so it is safe
                 to move one extra byte to reduce the instruction count.
                 */
                byte_move_forward(dst, src, stopped);
                dst = dst.add(stopped);
                src = src.add(stopped);
                state = S::CopyUtf8;
            }

            S::CopyAsciiStop1 => {
                byte_move_forward(dst, src, 1);
                dst = dst.add(1);
                src = src.add(1);
                state = S::CopyUtf8;
            }

            S::CopyUtf8 => {
                if *src & 0x80 != 0 {
                    /* non-ASCII character */
                    let pos = src;
                    let mut uni = byte_load_4(src);
                    while is_utf8_seq3(uni) {
                        byte_copy_4(dst, &uni as *const u32 as *const u8);
                        dst = dst.add(3);
                        src = src.add(3);
                        uni = byte_load_4(src);
                    }
                    if is_utf8_seq1(uni) {
                        state = S::CopyAscii;
                        continue;
                    }
                    while is_utf8_seq2(uni) {
                        byte_copy_2(dst, &uni as *const u32 as *const u8);
                        dst = dst.add(2);
                        src = src.add(2);
                        uni = byte_load_4(src);
                    }
                    while is_utf8_seq4(uni) {
                        byte_copy_4(dst, &uni as *const u32 as *const u8);
                        dst = dst.add(4);
                        src = src.add(4);
                        uni = byte_load_4(src);
                    }
                    if pos == src {
                        if !has_rflag(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                            return_err!(src, cstr!("invalid utf-8 encoding in string"));
                        }
                        state = S::CopyAsciiStop1;
                        continue;
                    }
                    state = S::CopyAscii;
                    continue;
                }
                state = S::CopyEscape;
            }
        }
    }
}

/// `read_str`
#[inline]
pub unsafe fn read_str(
    p: *mut *mut u8,
    eof: *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
    msg: *mut *const c_char,
) -> bool {
    read_str_opt(b'"', p, eof, flg, val, msg, ptr::null_mut())
}

/// `read_str_con`
#[inline]
pub unsafe fn read_str_con(
    p: *mut *mut u8,
    eof: *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
    msg: *mut *const c_char,
    con: *mut *mut u8,
) -> bool {
    read_str_opt(b'"', p, eof, flg, val, msg, con)
}

/// `read_str_sq`
#[inline]
pub unsafe fn read_str_sq(
    p: *mut *mut u8,
    eof: *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
    msg: *mut *const c_char,
) -> bool {
    read_str_opt(b'\'', p, eof, flg, val, msg, ptr::null_mut())
}

#[derive(PartialEq, Clone, Copy)]
enum SId {
    SkipAscii,
    SkipUtf8,
    CopyEscape,
    CopyAscii,
    CopyUtf8,
}

/// `read_str_id`: read an unquoted key (identifier name).
pub unsafe fn read_str_id(
    p: *mut *mut u8,
    _eof: *mut u8,
    flg: u32,
    pre: *mut *mut u8,
    val: *mut YyjsonVal,
    msg: *mut *const c_char,
) -> bool {
    let hdr = *p;
    let mut src = hdr;
    let mut dst: *mut u8 = ptr::null_mut();

    macro_rules! return_err {
        ($end:expr, $m:expr) => {{
            *msg = $m;
            *p = $end;
            return false;
        }};
    }
    macro_rules! return_suc {
        ($str_end:expr, $cur_end:expr) => {{
            (*val).tag = (($str_end.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                | YYJSON_TYPE_STR as u64;
            (*val).uni.str_ = hdr as *const c_char;
            *pre = $str_end;
            *p = $cur_end;
            return true;
        }};
    }

    /* add null-terminator for previous raw string */
    **pre = 0;

    let mut state = SId::SkipAscii;
    loop {
        match state {
            SId::SkipAscii => {
                loop {
                    let mut stop = 16usize;
                    for i in 0..16usize {
                        if !char_is_id_ascii(*src.add(i)) {
                            stop = i;
                            break;
                        }
                    }
                    if stop < 16 {
                        src = src.add(stop);
                        break;
                    }
                    src = src.add(16);
                }
                /* skip_ascii_end */
                if !char_is_id_next(*src) {
                    return_suc!(src, src);
                }
                state = SId::SkipUtf8;
            }

            SId::SkipUtf8 => {
                while *src >= 0x80 {
                    if has_rflag(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE)
                        && char_is_space_ext(*src)
                        && ext_space_len(src) != 0
                    {
                        return_suc!(src, src);
                    }
                    let uni = byte_load_4(src);
                    if is_utf8_seq2(uni) {
                        src = src.add(2);
                    } else if is_utf8_seq3(uni) {
                        src = src.add(3);
                    } else if is_utf8_seq4(uni) {
                        src = src.add(4);
                    } else {
                        if !has_rflag(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                            return_err!(src, cstr!("invalid utf-8 encoding in string"));
                        }
                        src = src.add(1);
                    }
                }
                if char_is_id_ascii(*src) {
                    state = SId::SkipAscii;
                    continue;
                }
                /* The escape character appears, we need to copy it. */
                dst = src;
                state = SId::CopyEscape;
            }

            SId::CopyEscape => {
                if byte_match_2(src, b"\\u") {
                    if !read_uni_esc(&mut src, &mut dst, msg) {
                        return_err!(src, *msg);
                    }
                } else {
                    if !char_is_id_next(*src) {
                        return_suc!(dst, src);
                    }
                    return_err!(src, cstr!("unexpected character in key"));
                }
                state = SId::CopyAscii;
            }

            SId::CopyAscii => {
                let stopped = loop {
                    let mut stop = 16usize;
                    for i in 0..16usize {
                        if !char_is_id_ascii(*src.add(i)) {
                            stop = i;
                            break;
                        }
                    }
                    if stop < 16 {
                        break stop;
                    }
                    byte_move_16(dst, src);
                    dst = dst.add(16);
                    src = src.add(16);
                };
                byte_move_forward(dst, src, stopped);
                dst = dst.add(stopped);
                src = src.add(stopped);
                state = SId::CopyUtf8;
            }

            SId::CopyUtf8 => {
                while *src >= 0x80 {
                    /* non-ASCII character */
                    if has_rflag(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE)
                        && char_is_space_ext(*src)
                        && ext_space_len(src) != 0
                    {
                        return_suc!(dst, src);
                    }
                    let uni = byte_load_4(src);
                    if is_utf8_seq2(uni) {
                        byte_copy_2(dst, &uni as *const u32 as *const u8);
                        dst = dst.add(2);
                        src = src.add(2);
                    } else if is_utf8_seq3(uni) {
                        byte_copy_4(dst, &uni as *const u32 as *const u8);
                        dst = dst.add(3);
                        src = src.add(3);
                    } else if is_utf8_seq4(uni) {
                        byte_copy_4(dst, &uni as *const u32 as *const u8);
                        dst = dst.add(4);
                        src = src.add(4);
                    } else {
                        if !has_rflag(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                            return_err!(src, cstr!("invalid utf-8 encoding in string"));
                        }
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                    }
                }
                if char_is_id_ascii(*src) {
                    state = SId::CopyAscii;
                    continue;
                }
                state = SId::CopyEscape;
            }
        }
    }
}
