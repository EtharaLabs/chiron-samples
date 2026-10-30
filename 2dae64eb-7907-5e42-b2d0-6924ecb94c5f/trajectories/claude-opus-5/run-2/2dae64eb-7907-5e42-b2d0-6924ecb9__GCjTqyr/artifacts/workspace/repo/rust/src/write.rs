//! JSON writer.
//!
//! Transliteration of the "String Writer", "JSON Writer Utilities",
//! "JSON Writer Implementation", "JSON Writer (Public)", "Mutable JSON Writer
//! Implementation" and "Mutable JSON Writer (Public)" sections of the
//! original `src/yyjson.c`.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alc::{alc_or_default, YYJSON_NULL_ALC};
use crate::libc;
use crate::tables::*;
use crate::types::*;
use crate::util::*;
use crate::wnum::write_num;

/* ===== character encode types =========================================== */

const CHAR_ENC_CPY_1: u8 = 0;
const CHAR_ENC_ERR_1: u8 = 1;
const CHAR_ENC_ESC_A: u8 = 2;
const CHAR_ENC_ESC_1: u8 = 3;
const CHAR_ENC_CPY_2: u8 = 4;
const CHAR_ENC_ESC_2: u8 = 5;
const CHAR_ENC_CPY_3: u8 = 6;
const CHAR_ENC_ESC_3: u8 = 7;
const CHAR_ENC_CPY_4: u8 = 8;
const CHAR_ENC_ESC_4: u8 = 9;

#[inline(always)]
fn has_wflag(flg: u32, chk: u32) -> bool {
    (flg & chk) != 0
}

#[inline]
fn get_hex_table_with_flag(flg: u32) -> *const u8 {
    if has_wflag(flg, YYJSON_WRITE_LOWERCASE_HEX) {
        ESC_HEX_CHAR_TABLE_LOWER.as_ptr()
    } else {
        ESC_HEX_CHAR_TABLE.as_ptr()
    }
}

#[inline]
fn get_enc_table_with_flag(flg: u32) -> *const u8 {
    if has_wflag(flg, YYJSON_WRITE_ESCAPE_UNICODE) {
        if has_wflag(flg, YYJSON_WRITE_ESCAPE_SLASHES) {
            ENC_TABLE_ESC_SLASH.as_ptr()
        } else {
            ENC_TABLE_ESC.as_ptr()
        }
    } else if has_wflag(flg, YYJSON_WRITE_ESCAPE_SLASHES) {
        ENC_TABLE_CPY_SLASH.as_ptr()
    } else {
        ENC_TABLE_CPY.as_ptr()
    }
}

/* ===== string writer ==================================================== */

/// `write_raw`
#[inline]
unsafe fn write_raw(cur: *mut u8, raw: *const u8, raw_len: usize) -> *mut u8 {
    libc::mem_copy(cur, raw, raw_len);
    cur.add(raw_len)
}

/// `write_str_noesc`
unsafe fn write_str_noesc(cur: *mut u8, str_: *const u8, str_len: usize) -> *mut u8 {
    let mut cur = cur;
    let mut str_ = str_;
    let mut str_len = str_len;
    *cur = b'"';
    cur = cur.add(1);
    while str_len >= 16 {
        byte_copy_16(cur, str_);
        cur = cur.add(16);
        str_ = str_.add(16);
        str_len -= 16;
    }
    while str_len >= 4 {
        byte_copy_4(cur, str_);
        cur = cur.add(4);
        str_ = str_.add(4);
        str_len -= 4;
    }
    while str_len != 0 {
        *cur = *str_;
        cur = cur.add(1);
        str_ = str_.add(1);
        str_len -= 1;
    }
    *cur = b'"';
    cur.add(1)
}

/// `write_str` (requires a `len * 6 + 2` byte buffer)
unsafe fn write_str(
    cur: *mut u8,
    esc: bool,
    inv: bool,
    str_: *const u8,
    str_len: usize,
    enc_table: *const u8,
    hex_table: *const u8,
) -> *mut u8 {
    /* The `\u00` prefix of an escaped character. */
    const PRE: [u8; 4] = [b'\\', b'u', b'0', b'0'];

    let mut cur = cur;
    let mut src = str_;
    let end = str_.add(str_len);
    *cur = b'"';
    cur = cur.add(1);

    #[derive(PartialEq, Clone, Copy)]
    enum S {
        CopyAscii,
        CopyUtf8,
        ErrOne,
        ErrCpy,
        ErrEsc,
    }

    let mut state = S::CopyAscii;
    loop {
        match state {
            S::CopyAscii => {
                let stopped: usize = 'ascii: {
                    while (end as usize - src as usize) >= 16 {
                        for i in 0..16usize {
                            if *enc_table.add(*src.add(i) as usize) != 0 {
                                break 'ascii i;
                            }
                        }
                        byte_copy_16(cur, src);
                        cur = cur.add(16);
                        src = src.add(16);
                    }
                    while (end as usize - src as usize) >= 4 {
                        for i in 0..4usize {
                            if *enc_table.add(*src.add(i) as usize) != 0 {
                                break 'ascii i;
                            }
                        }
                        byte_copy_4(cur, src);
                        cur = cur.add(4);
                        src = src.add(4);
                    }
                    while end > src {
                        if *enc_table.add(*src as usize) != 0 {
                            break 'ascii 0;
                        }
                        *cur = *src;
                        cur = cur.add(1);
                        src = src.add(1);
                    }
                    *cur = b'"';
                    return cur.add(1);
                };
                let i = stopped;
                libc::mem_copy(cur, src, i);
                cur = cur.add(i);
                src = src.add(i);
                state = S::CopyUtf8;
            }

            S::CopyUtf8 => {
                if (end as usize - src as usize) < 4 {
                    if end == src {
                        /* copy_end */
                        *cur = b'"';
                        return cur.add(1);
                    }
                    if ((end as usize - src as usize) as u8)
                        < *enc_table.add(*src as usize) / 2
                    {
                        state = S::ErrOne;
                        continue;
                    }
                }
                match *enc_table.add(*src as usize) {
                    CHAR_ENC_CPY_1 => {
                        *cur = *src;
                        cur = cur.add(1);
                        src = src.add(1);
                        state = S::CopyAscii;
                    }
                    CHAR_ENC_CPY_2 => {
                        let mut uni: u32 = 0;
                        byte_copy_2(&mut uni as *mut u32 as *mut u8, src);
                        if !is_utf8_seq2(uni) {
                            state = S::ErrCpy;
                            continue;
                        }
                        byte_copy_2(cur, &uni as *const u32 as *const u8);
                        cur = cur.add(2);
                        src = src.add(2);
                    }
                    CHAR_ENC_CPY_3 => {
                        if (end as usize - src as usize) >= 4 {
                            let uni = byte_load_4(src);
                            if !is_utf8_seq3(uni) {
                                state = S::ErrCpy;
                                continue;
                            }
                            byte_copy_4(cur, src);
                        } else {
                            let uni = byte_load_3(src);
                            if !is_utf8_seq3(uni) {
                                state = S::ErrCpy;
                                continue;
                            }
                            byte_copy_4(cur, &uni as *const u32 as *const u8);
                        }
                        cur = cur.add(3);
                        src = src.add(3);
                    }
                    CHAR_ENC_CPY_4 => {
                        let uni = byte_load_4(src);
                        if !is_utf8_seq4(uni) {
                            state = S::ErrCpy;
                            continue;
                        }
                        byte_copy_4(cur, src);
                        cur = cur.add(4);
                        src = src.add(4);
                    }
                    CHAR_ENC_ESC_A => {
                        byte_copy_2(cur, ESC_SINGLE_CHAR_TABLE.as_ptr().add(*src as usize * 2));
                        cur = cur.add(2);
                        src = src.add(1);
                    }
                    CHAR_ENC_ESC_1 => {
                        byte_copy_4(cur, PRE.as_ptr());
                        byte_copy_2(cur.add(4), hex_table.add(*src as usize * 2));
                        cur = cur.add(6);
                        src = src.add(1);
                    }
                    CHAR_ENC_ESC_2 => {
                        let mut v4: u32 = 0;
                        let v2 = byte_load_2(src);
                        byte_copy_2(&mut v4 as *mut u32 as *mut u8, &v2 as *const u16 as *const u8);
                        if !is_utf8_seq2(v4) {
                            state = S::ErrEsc;
                            continue;
                        }
                        let u: u16 = (((*src & 0x1F) as u16) << 6)
                            | (((*src.add(1) & 0x3F) as u16) << 0);
                        byte_copy_2(cur, PRE.as_ptr());
                        byte_copy_2(cur.add(2), hex_table.add((u >> 8) as usize * 2));
                        byte_copy_2(cur.add(4), hex_table.add((u & 0xFF) as usize * 2));
                        cur = cur.add(6);
                        src = src.add(2);
                    }
                    CHAR_ENC_ESC_3 => {
                        let v = byte_load_3(src);
                        if !is_utf8_seq3(v) {
                            state = S::ErrEsc;
                            continue;
                        }
                        let u: u16 = (((*src & 0x0F) as u16) << 12)
                            | (((*src.add(1) & 0x3F) as u16) << 6)
                            | (((*src.add(2) & 0x3F) as u16) << 0);
                        byte_copy_2(cur, PRE.as_ptr());
                        byte_copy_2(cur.add(2), hex_table.add((u >> 8) as usize * 2));
                        byte_copy_2(cur.add(4), hex_table.add((u & 0xFF) as usize * 2));
                        cur = cur.add(6);
                        src = src.add(3);
                    }
                    CHAR_ENC_ESC_4 => {
                        let v = byte_load_4(src);
                        if !is_utf8_seq4(v) {
                            state = S::ErrEsc;
                            continue;
                        }
                        let mut u: u32 = (((*src & 0x07) as u32) << 18)
                            | (((*src.add(1) & 0x3F) as u32) << 12)
                            | (((*src.add(2) & 0x3F) as u32) << 6)
                            | (((*src.add(3) & 0x3F) as u32) << 0);
                        u -= 0x10000;
                        let hi = (u >> 10) + 0xD800;
                        let lo = (u & 0x3FF) + 0xDC00;
                        byte_copy_2(cur, PRE.as_ptr());
                        byte_copy_2(cur.add(2), hex_table.add((hi >> 8) as usize * 2));
                        byte_copy_2(cur.add(4), hex_table.add((hi & 0xFF) as usize * 2));
                        byte_copy_2(cur.add(6), PRE.as_ptr());
                        byte_copy_2(cur.add(8), hex_table.add((lo >> 8) as usize * 2));
                        byte_copy_2(cur.add(10), hex_table.add((lo & 0xFF) as usize * 2));
                        cur = cur.add(12);
                        src = src.add(4);
                    }
                    CHAR_ENC_ERR_1 => {
                        state = S::ErrOne;
                    }
                    _ => {}
                }
            }

            S::ErrOne => {
                state = if esc { S::ErrEsc } else { S::ErrCpy };
            }

            S::ErrCpy => {
                if !inv {
                    return ptr::null_mut();
                }
                *cur = *src;
                cur = cur.add(1);
                src = src.add(1);
                state = S::CopyUtf8;
            }

            S::ErrEsc => {
                if !inv {
                    return ptr::null_mut();
                }
                byte_copy_2(cur, PRE.as_ptr());
                /* U+FFFD, written as two pairs so that LOWERCASE_HEX works. */
                byte_copy_2(cur.add(2), hex_table.add(0xFF * 2));
                byte_copy_2(cur.add(4), hex_table.add(0xFD * 2));
                cur = cur.add(6);
                src = src.add(1);
                state = S::CopyUtf8;
            }
        }
    }
}

/* ===== writer utilities ================================================= */

/// `write_null` (requires an 8-byte buffer)
#[inline]
unsafe fn write_null(cur: *mut u8) -> *mut u8 {
    let v: [u8; 8] = [b'n', b'u', b'l', b'l', b',', b'\n', 0, 0];
    byte_copy_8(cur, v.as_ptr());
    cur.add(4)
}

/// `write_bool` (requires an 8-byte buffer)
#[inline]
unsafe fn write_bool(cur: *mut u8, val: bool) -> *mut u8 {
    let v0: [u8; 8] = [b'f', b'a', b'l', b's', b'e', b',', b'\n', 0];
    let v1: [u8; 8] = [b't', b'r', b'u', b'e', b',', b'\n', 0, 0];
    if val {
        byte_copy_8(cur, v1.as_ptr());
    } else {
        byte_copy_8(cur, v0.as_ptr());
    }
    cur.add(5 - val as usize)
}

/// `write_indent`
#[inline]
unsafe fn write_indent(cur: *mut u8, level: usize, spaces: usize) -> *mut u8 {
    let mut cur = cur;
    let mut level = level;
    while level > 0 {
        level -= 1;
        byte_copy_4(cur, b"    ".as_ptr());
        cur = cur.add(spaces);
    }
    cur
}

/// `write_dat_to_fp`
unsafe fn write_dat_to_fp(
    fp: *mut libc::File,
    dat: *mut u8,
    len: usize,
    err: *mut YyjsonWriteErr,
) -> bool {
    if libc::fwrite(dat as *const c_void, 1, len, fp) != len {
        (*err).msg = cstr!("file writing failed");
        (*err).code = YYJSON_WRITE_ERROR_FILE_WRITE;
        return false;
    }
    true
}

/// `write_dat_to_file`
unsafe fn write_dat_to_file(
    path: *const c_char,
    dat: *mut u8,
    len: usize,
    err: *mut YyjsonWriteErr,
) -> bool {
    let mut file = libc::fopen(path, cstr!("wbe"));

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            (*err).msg = $m;
            (*err).code = $code;
            if !file.is_null() {
                libc::fclose(file);
            }
            return false;
        }};
    }

    if file.is_null() {
        return_err!(YYJSON_WRITE_ERROR_FILE_OPEN, MSG_FOPEN);
    }
    if libc::fwrite(dat as *const c_void, 1, len, file) != len {
        return_err!(YYJSON_WRITE_ERROR_FILE_WRITE, MSG_FWRITE);
    }
    if libc::fclose(file) != 0 {
        file = ptr::null_mut();
        return_err!(YYJSON_WRITE_ERROR_FILE_WRITE, MSG_FCLOSE);
    }
    true
}

/* ===== write failures =================================================== */

#[derive(Clone, Copy, PartialEq)]
enum WFail {
    Alloc,
    Type,
    Num,
    Str,
}

impl WFail {
    fn code_msg(self) -> (u32, *const c_char) {
        match self {
            WFail::Alloc => (YYJSON_WRITE_ERROR_MEMORY_ALLOCATION, MSG_MALLOC),
            WFail::Type => (YYJSON_WRITE_ERROR_INVALID_VALUE_TYPE, MSG_ERR_TYPE),
            WFail::Num => (YYJSON_WRITE_ERROR_NAN_OR_INF, MSG_NAN_INF),
            WFail::Str => (YYJSON_WRITE_ERROR_INVALID_STRING, MSG_ERR_UTF8),
        }
    }
}

/* ===== single value root ================================================ */

/// `write_root_single`
unsafe fn write_root_single(
    val: *mut YyjsonVal,
    flg: u32,
    alc: YyjsonAlc,
    buf: *mut c_char,
    dat_len: *mut usize,
    err: *mut YyjsonWriteErr,
) -> *mut u8 {
    let mut hdr: *mut u8 = ptr::null_mut();
    let mut cur: *mut u8;
    let enc_table = get_enc_table_with_flag(flg);
    let hex_table = get_hex_table_with_flag(flg);
    let cpy = enc_table == ENC_TABLE_CPY.as_ptr();
    let esc = has_wflag(flg, YYJSON_WRITE_ESCAPE_UNICODE);
    let inv = has_wflag(flg, YYJSON_WRITE_ALLOW_INVALID_UNICODE);
    let newline = has_wflag(flg, YYJSON_WRITE_NEWLINE_AT_END);
    let end_len: usize = 2; /* '\n' and '\0' */

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            if !hdr.is_null() {
                alc.dealloc(hdr as *mut c_void);
            }
            *dat_len = 0;
            (*err).code = $code;
            (*err).msg = $m;
            return ptr::null_mut();
        }};
    }
    macro_rules! incr_len {
        ($len:expr) => {{
            let l: usize = $len;
            if !buf.is_null() {
                hdr = if *dat_len >= l {
                    buf as *mut u8
                } else {
                    ptr::null_mut()
                };
            } else {
                hdr = alc.alloc(l) as *mut u8;
            }
            if hdr.is_null() {
                return_err!(YYJSON_WRITE_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
            }
            cur = hdr;
        }};
    }

    let fail: Option<WFail> = 'single: {
        match get_type(val) {
            YYJSON_TYPE_RAW => {
                let str_len = get_len(val);
                let str_ptr = get_str(val) as *const u8;
                incr_len!(str_len + end_len);
                cur = write_raw(cur, str_ptr, str_len);
            }
            YYJSON_TYPE_STR => {
                let str_len = get_len(val);
                let str_ptr = get_str(val) as *const u8;
                incr_len!(str_len * 6 + 2 + end_len);
                if cpy && get_subtype(val) != 0 {
                    cur = write_str_noesc(cur, str_ptr, str_len);
                } else {
                    cur = write_str(cur, esc, inv, str_ptr, str_len, enc_table, hex_table);
                    if cur.is_null() {
                        break 'single Some(WFail::Str);
                    }
                }
            }
            YYJSON_TYPE_NUM => {
                incr_len!(FP_BUF_LEN + end_len);
                cur = write_num(cur, val, flg);
                if cur.is_null() {
                    break 'single Some(WFail::Num);
                }
            }
            YYJSON_TYPE_BOOL => {
                incr_len!(8);
                cur = write_bool(cur, get_bool(val));
            }
            YYJSON_TYPE_NULL => {
                incr_len!(8);
                cur = write_null(cur);
            }
            YYJSON_TYPE_ARR => {
                incr_len!(2 + end_len);
                byte_copy_2(cur, b"[]".as_ptr());
                cur = cur.add(2);
            }
            YYJSON_TYPE_OBJ => {
                incr_len!(2 + end_len);
                byte_copy_2(cur, b"{}".as_ptr());
                cur = cur.add(2);
            }
            _ => break 'single Some(WFail::Type),
        }

        if newline {
            *cur = b'\n';
            cur = cur.add(1);
        }
        *cur = 0;
        *dat_len = cur.offset_from(hdr) as usize;
        libc::zero(err);
        return hdr;
    };

    let (code, m) = fail.unwrap_unchecked().code_msg();
    if !hdr.is_null() {
        alc.dealloc(hdr as *mut c_void);
    }
    *dat_len = 0;
    (*err).code = code;
    (*err).msg = m;
    ptr::null_mut()
}

/* ===== immutable container root ========================================= */

#[repr(C)]
#[derive(Clone, Copy)]
struct WriteCtx {
    tag: usize,
}

#[inline]
unsafe fn write_ctx_set(ctx: *mut WriteCtx, size: usize, is_obj: bool) {
    (*ctx).tag = (size << 1) | is_obj as usize;
}

#[inline]
unsafe fn write_ctx_get(ctx: *mut WriteCtx) -> (usize, bool) {
    let tag = (*ctx).tag;
    (tag >> 1, (tag & 1) != 0)
}

#[derive(Clone, Copy, PartialEq)]
enum WSt {
    ValBegin,
    ValEnd,
    CtnEnd,
    DocEnd,
}

/// `write_root_minify` / `write_root_pretty`
// The final `incr_len!` in `DocEnd` refreshes `ctx`/`end` that nothing reads
// afterwards; that mirrors the C macro and is intentional.
#[allow(unused_assignments)]
unsafe fn write_root_ctn<const PRETTY: bool>(
    root: *const YyjsonVal,
    flg: u32,
    alc: YyjsonAlc,
    buf: *mut c_char,
    dat_len: *mut usize,
    err: *mut YyjsonWriteErr,
) -> *mut u8 {
    let mut hdr: *mut u8;
    let mut cur: *mut u8;
    let mut end: *mut u8;
    let mut ctx: *mut WriteCtx;
    let mut alc_len: usize;

    let enc_table = get_enc_table_with_flag(flg);
    let hex_table = get_hex_table_with_flag(flg);
    let cpy = enc_table == ENC_TABLE_CPY.as_ptr();
    let esc = has_wflag(flg, YYJSON_WRITE_ESCAPE_UNICODE);
    let inv = has_wflag(flg, YYJSON_WRITE_ALLOW_INVALID_UNICODE);
    let spaces = if has_wflag(flg, YYJSON_WRITE_PRETTY_TWO_SPACES) {
        2usize
    } else {
        4usize
    };
    let newline = has_wflag(flg, YYJSON_WRITE_NEWLINE_AT_END);

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            *dat_len = 0;
            (*err).code = $code;
            (*err).msg = $m;
            if !hdr.is_null() {
                alc.dealloc(hdr as *mut c_void);
            }
            return ptr::null_mut();
        }};
    }

    if !buf.is_null() {
        hdr = buf as *mut u8;
        alc_len = *dat_len;
        alc_len = size_align_down(alc_len, size_of::<WriteCtx>());
        if alc_len <= size_of::<WriteCtx>() {
            hdr = ptr::null_mut();
            return_err!(YYJSON_WRITE_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
    } else {
        alc_len = (*root).uni.ofs / size_of::<YyjsonVal>();
        let ratio = if PRETTY {
            YYJSON_WRITER_ESTIMATED_PRETTY_RATIO
        } else {
            YYJSON_WRITER_ESTIMATED_MINIFY_RATIO
        };
        alc_len = alc_len * ratio + 64;
        alc_len = size_align_up(alc_len, size_of::<WriteCtx>());
        hdr = alc.alloc(alc_len) as *mut u8;
        if hdr.is_null() {
            return_err!(YYJSON_WRITE_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
    }
    cur = hdr;
    end = hdr.add(alc_len);
    ctx = end as *mut WriteCtx;

    /* doc_begin */
    let mut val = root as *mut YyjsonVal;
    let mut val_type = get_type(val);
    let mut ctn_obj = val_type == YYJSON_TYPE_OBJ;
    let mut ctn_len = get_len(val) << ctn_obj as u8;
    *cur = b'[' | ((ctn_obj as u8) << 5);
    cur = cur.add(1);
    let mut level: usize = 1;
    if PRETTY {
        *cur = b'\n';
        cur = cur.add(1);
    }
    val = val.add(1);

    let mut state = WSt::ValBegin;

    let fail: WFail = 'fsm: loop {
        macro_rules! incr_len {
            ($len:expr) => {{
                let ext_len: usize = $len;
                if (cur as usize + ext_len) >= ctx as usize {
                    let ctx_pos = (ctx as *mut u8).offset_from(hdr) as usize;
                    let cur_pos = cur.offset_from(hdr) as usize;
                    let ctx_len = end.offset_from(ctx as *mut u8) as usize;
                    let mut alc_inc = if alc_len / 2 > ext_len {
                        alc_len / 2
                    } else {
                        ext_len
                    };
                    alc_inc = size_align_up(alc_inc, size_of::<WriteCtx>());
                    alc_len += alc_inc;
                    let tmp =
                        alc.realloc(hdr as *mut c_void, alc_len - alc_inc, alc_len) as *mut u8;
                    if tmp.is_null() {
                        break 'fsm WFail::Alloc;
                    }
                    let ctx_tmp = tmp.add(alc_len - ctx_len) as *mut WriteCtx;
                    libc::mem_move(ctx_tmp as *mut u8, tmp.add(ctx_pos), ctx_len);
                    ctx = ctx_tmp;
                    cur = tmp.add(cur_pos);
                    end = tmp.add(alc_len);
                    hdr = tmp;
                }
            }};
        }

        match state {
            WSt::ValBegin => {
                val_type = get_type(val);
                if val_type == YYJSON_TYPE_STR {
                    let is_key = (ctn_obj as u8) & !(ctn_len as u8) != 0;
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    let str_len = get_len(val);
                    let str_ptr = get_str(val) as *const u8;
                    if PRETTY {
                        incr_len!(
                            str_len * 6 + 16 + if no_indent { 0 } else { level * 4 }
                        );
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                    } else {
                        incr_len!(str_len * 6 + 16);
                    }
                    if cpy && get_subtype(val) != 0 {
                        cur = write_str_noesc(cur, str_ptr, str_len);
                    } else {
                        cur = write_str(cur, esc, inv, str_ptr, str_len, enc_table, hex_table);
                        if cur.is_null() {
                            break 'fsm WFail::Str;
                        }
                    }
                    *cur = if is_key { b':' } else { b',' };
                    cur = cur.add(1);
                    if PRETTY {
                        *cur = if is_key { b' ' } else { b'\n' };
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                if val_type == YYJSON_TYPE_NUM {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    if PRETTY {
                        incr_len!(FP_BUF_LEN + if no_indent { 0 } else { level * 4 });
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                    } else {
                        incr_len!(FP_BUF_LEN);
                    }
                    cur = write_num(cur, val, flg);
                    if cur.is_null() {
                        break 'fsm WFail::Num;
                    }
                    *cur = b',';
                    cur = cur.add(1);
                    if PRETTY {
                        *cur = b'\n';
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                if (val_type & (YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ))
                    == (YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ)
                {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    let ctn_len_tmp = get_len(val);
                    let ctn_obj_tmp = val_type == YYJSON_TYPE_OBJ;
                    if PRETTY {
                        incr_len!(
                            2 * size_of::<WriteCtx>()
                                + if no_indent { 0 } else { level * 4 }
                        );
                    } else {
                        incr_len!(2 * size_of::<WriteCtx>());
                    }
                    if ctn_len_tmp == 0 {
                        /* write empty container */
                        if PRETTY {
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        }
                        *cur = b'[' | ((ctn_obj_tmp as u8) << 5);
                        cur = cur.add(1);
                        *cur = b']' | ((ctn_obj_tmp as u8) << 5);
                        cur = cur.add(1);
                        *cur = b',';
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        state = WSt::ValEnd;
                        continue;
                    } else {
                        /* push context, setup new container */
                        ctx = ctx.sub(1);
                        write_ctx_set(ctx, ctn_len, ctn_obj);
                        ctn_len = ctn_len_tmp << ctn_obj_tmp as u8;
                        ctn_obj = ctn_obj_tmp;
                        if PRETTY {
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                            level += 1;
                        }
                        *cur = b'[' | ((ctn_obj as u8) << 5);
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        val = val.add(1);
                        continue;
                    }
                }
                if val_type == YYJSON_TYPE_BOOL {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    if PRETTY {
                        incr_len!(16 + if no_indent { 0 } else { level * 4 });
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        cur = write_bool(cur, get_bool(val));
                        cur = cur.add(2);
                    } else {
                        incr_len!(16);
                        cur = write_bool(cur, get_bool(val));
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                if val_type == YYJSON_TYPE_NULL {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    if PRETTY {
                        incr_len!(16 + if no_indent { 0 } else { level * 4 });
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        cur = write_null(cur);
                        cur = cur.add(2);
                    } else {
                        incr_len!(16);
                        cur = write_null(cur);
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                if val_type == YYJSON_TYPE_RAW {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    let str_len = get_len(val);
                    let str_ptr = get_str(val) as *const u8;
                    if PRETTY {
                        incr_len!(str_len + 3 + if no_indent { 0 } else { level * 4 });
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                    } else {
                        incr_len!(str_len + 2);
                    }
                    cur = write_raw(cur, str_ptr, str_len);
                    *cur = b',';
                    cur = cur.add(1);
                    if PRETTY {
                        *cur = b'\n';
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                break 'fsm WFail::Type;
            }

            WSt::ValEnd => {
                val = val.add(1);
                ctn_len -= 1;
                if ctn_len == 0 {
                    state = WSt::CtnEnd;
                } else {
                    state = WSt::ValBegin;
                }
            }

            WSt::CtnEnd => {
                if PRETTY {
                    cur = cur.sub(2);
                    *cur = b'\n';
                    cur = cur.add(1);
                    incr_len!(level * 4);
                    level -= 1;
                    cur = write_indent(cur, level, spaces);
                    *cur = b']' | ((ctn_obj as u8) << 5);
                    cur = cur.add(1);
                    if (ctx as *mut u8) >= end {
                        state = WSt::DocEnd;
                        continue;
                    }
                    let (l, o) = write_ctx_get(ctx);
                    ctx = ctx.add(1);
                    ctn_len = l;
                    ctn_obj = o;
                    ctn_len -= 1;
                    *cur = b',';
                    cur = cur.add(1);
                    *cur = b'\n';
                    cur = cur.add(1);
                } else {
                    cur = cur.sub(1);
                    *cur = b']' | ((ctn_obj as u8) << 5);
                    cur = cur.add(1);
                    *cur = b',';
                    cur = cur.add(1);
                    if (ctx as *mut u8) >= end {
                        state = WSt::DocEnd;
                        continue;
                    }
                    let (l, o) = write_ctx_get(ctx);
                    ctx = ctx.add(1);
                    ctn_len = l;
                    ctn_obj = o;
                    ctn_len -= 1;
                }
                if ctn_len > 0 {
                    state = WSt::ValBegin;
                } else {
                    state = WSt::CtnEnd;
                }
            }

            WSt::DocEnd => {
                if PRETTY {
                    if newline {
                        incr_len!(2);
                        *cur = b'\n';
                        cur = cur.add(1);
                    }
                    *cur = 0;
                } else {
                    if newline {
                        incr_len!(2);
                        *cur.sub(1) = b'\n';
                        cur = cur.add(1);
                    }
                    cur = cur.sub(1);
                    *cur = 0;
                }
                *dat_len = cur.offset_from(hdr) as usize;
                libc::zero(err);
                return hdr;
            }
        }
    };

    let (code, m) = fail.code_msg();
    return_err!(code, m);
}

/// `write_root`
unsafe fn write_root(
    val: *const YyjsonVal,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    buf: *mut c_char,
    dat_len_in: *mut usize,
    err_in: *mut YyjsonWriteErr,
) -> *mut c_char {
    let mut tmp_err = YyjsonWriteErr {
        code: 0,
        msg: ptr::null(),
    };
    let mut tmp_dat_len: usize = 0;
    let alc = alc_or_default(alc_ptr);
    let root = val as *mut YyjsonVal;

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonWriteErr
    } else {
        err_in
    };
    let dat_len = if dat_len_in.is_null() {
        &mut tmp_dat_len as *mut usize
    } else {
        dat_len_in
    };

    if root.is_null() {
        *dat_len = 0;
        (*err).msg = cstr!("input JSON is NULL");
        (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
        return ptr::null_mut();
    }

    if !is_ctn(root) || get_len(root) == 0 {
        write_root_single(root, flg, alc, buf, dat_len, err) as *mut c_char
    } else if (flg & (YYJSON_WRITE_PRETTY | YYJSON_WRITE_PRETTY_TWO_SPACES)) != 0 {
        write_root_ctn::<true>(root, flg, alc, buf, dat_len, err) as *mut c_char
    } else {
        write_root_ctn::<false>(root, flg, alc, buf, dat_len, err) as *mut c_char
    }
}

/* ===== public immutable writer ========================================== */

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_write_opts(
    val: *const YyjsonVal,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    dat_len: *mut usize,
    err: *mut YyjsonWriteErr,
) -> *mut c_char {
    write_root(val, flg, alc_ptr, ptr::null_mut(), dat_len, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_opts(
    doc: *const YyjsonDoc,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    dat_len: *mut usize,
    err: *mut YyjsonWriteErr,
) -> *mut c_char {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    write_root(root, flg, alc_ptr, ptr::null_mut(), dat_len, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_write_file(
    path: *const c_char,
    val: *const YyjsonVal,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err_in: *mut YyjsonWriteErr,
) -> bool {
    let mut tmp_err = YyjsonWriteErr {
        code: 0,
        msg: ptr::null(),
    };
    let alc = alc_or_default(alc_ptr);
    let mut dat_len: usize = 0;
    let root = val as *mut YyjsonVal;

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonWriteErr
    } else {
        err_in
    };
    if path.is_null() || *path == 0 {
        (*err).msg = cstr!("input path is invalid");
        (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
        return false;
    }

    let dat = write_root(root, flg, &alc, ptr::null_mut(), &mut dat_len, err) as *mut u8;
    if dat.is_null() {
        return false;
    }
    let suc = write_dat_to_file(path, dat, dat_len, err);
    alc.dealloc(dat as *mut c_void);
    suc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_write_fp(
    fp: *mut libc::File,
    val: *const YyjsonVal,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err_in: *mut YyjsonWriteErr,
) -> bool {
    let mut tmp_err = YyjsonWriteErr {
        code: 0,
        msg: ptr::null(),
    };
    let alc = alc_or_default(alc_ptr);
    let mut dat_len: usize = 0;
    let root = val as *mut YyjsonVal;

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonWriteErr
    } else {
        err_in
    };
    if fp.is_null() {
        (*err).msg = cstr!("input fp is invalid");
        (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
        return false;
    }

    let dat = write_root(root, flg, &alc, ptr::null_mut(), &mut dat_len, err) as *mut u8;
    if dat.is_null() {
        return false;
    }
    let suc = write_dat_to_fp(fp, dat, dat_len, err);
    alc.dealloc(dat as *mut c_void);
    suc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_file(
    path: *const c_char,
    doc: *const YyjsonDoc,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err: *mut YyjsonWriteErr,
) -> bool {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_val_write_file(path, root, flg, alc_ptr, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_fp(
    fp: *mut libc::File,
    doc: *const YyjsonDoc,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err: *mut YyjsonWriteErr,
) -> bool {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_val_write_fp(fp, root, flg, alc_ptr, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_write_buf(
    buf: *mut c_char,
    buf_len: usize,
    val: *const YyjsonVal,
    flg: u32,
    err: *mut YyjsonWriteErr,
) -> usize {
    if buf.is_null() || buf_len == 0 {
        if !err.is_null() {
            (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
            (*err).msg = cstr!("input buf or buf_len is invalid");
        }
        0
    } else {
        let mut buf_len = buf_len;
        write_root(val, flg, &YYJSON_NULL_ALC, buf, &mut buf_len, err);
        buf_len
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_buf(
    buf: *mut c_char,
    buf_len: usize,
    doc: *const YyjsonDoc,
    flg: u32,
    err: *mut YyjsonWriteErr,
) -> usize {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_val_write_buf(buf, buf_len, root, flg, err)
}

/* ===== mutable writer =================================================== */

#[repr(C)]
#[derive(Clone, Copy)]
struct MutWriteCtx {
    tag: usize,
    ctn: *mut YyjsonMutVal,
}

#[inline]
unsafe fn mut_write_ctx_set(
    ctx: *mut MutWriteCtx,
    ctn: *mut YyjsonMutVal,
    size: usize,
    is_obj: bool,
) {
    (*ctx).tag = (size << 1) | is_obj as usize;
    (*ctx).ctn = ctn;
}

#[inline]
unsafe fn mut_write_ctx_get(ctx: *mut MutWriteCtx) -> (*mut YyjsonMutVal, usize, bool) {
    let tag = (*ctx).tag;
    ((*ctx).ctn, tag >> 1, (tag & 1) != 0)
}

/// `yyjson_mut_doc_estimated_val_num`
unsafe fn mut_doc_estimated_val_num(doc: *const YyjsonMutDoc) -> usize {
    let mut sum = 0usize;
    let mut chunk = (*doc).val_pool.chunks;
    while !chunk.is_null() {
        sum += (*chunk).chunk_size / size_of::<YyjsonMutVal>() - 1;
        if chunk == (*doc).val_pool.chunks {
            sum -= (*doc).val_pool.end.offset_from((*doc).val_pool.cur) as usize;
        }
        chunk = (*chunk).next;
    }
    sum
}

/// `mut_write_root_minify` / `mut_write_root_pretty`
#[allow(unused_assignments)]
unsafe fn mut_write_root_ctn<const PRETTY: bool>(
    root: *const YyjsonMutVal,
    estimated_val_num: usize,
    flg: u32,
    alc: YyjsonAlc,
    buf: *mut c_char,
    dat_len: *mut usize,
    err: *mut YyjsonWriteErr,
) -> *mut u8 {
    let mut hdr: *mut u8;
    let mut cur: *mut u8;
    let mut end: *mut u8;
    let mut ctx: *mut MutWriteCtx;
    let mut alc_len: usize;

    let enc_table = get_enc_table_with_flag(flg);
    let hex_table = get_hex_table_with_flag(flg);
    let cpy = enc_table == ENC_TABLE_CPY.as_ptr();
    let esc = has_wflag(flg, YYJSON_WRITE_ESCAPE_UNICODE);
    let inv = has_wflag(flg, YYJSON_WRITE_ALLOW_INVALID_UNICODE);
    let spaces = if has_wflag(flg, YYJSON_WRITE_PRETTY_TWO_SPACES) {
        2usize
    } else {
        4usize
    };
    let newline = has_wflag(flg, YYJSON_WRITE_NEWLINE_AT_END);

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            *dat_len = 0;
            (*err).code = $code;
            (*err).msg = $m;
            if !hdr.is_null() {
                alc.dealloc(hdr as *mut c_void);
            }
            return ptr::null_mut();
        }};
    }

    if !buf.is_null() {
        hdr = buf as *mut u8;
        alc_len = *dat_len;
        alc_len = size_align_down(alc_len, size_of::<MutWriteCtx>());
        if alc_len <= size_of::<MutWriteCtx>() {
            hdr = ptr::null_mut();
            return_err!(YYJSON_WRITE_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
    } else {
        let ratio = if PRETTY {
            YYJSON_WRITER_ESTIMATED_PRETTY_RATIO
        } else {
            YYJSON_WRITER_ESTIMATED_MINIFY_RATIO
        };
        alc_len = estimated_val_num * ratio + 64;
        alc_len = size_align_up(alc_len, size_of::<MutWriteCtx>());
        hdr = alc.alloc(alc_len) as *mut u8;
        if hdr.is_null() {
            return_err!(YYJSON_WRITE_ERROR_MEMORY_ALLOCATION, MSG_MALLOC);
        }
    }
    cur = hdr;
    end = hdr.add(alc_len);
    ctx = end as *mut MutWriteCtx;

    /* doc_begin */
    let mut val = root as *mut YyjsonMutVal;
    let mut val_type = get_type(mut_as_val(val));
    let mut ctn_obj = val_type == YYJSON_TYPE_OBJ;
    let mut ctn_len = get_len(mut_as_val(val)) << ctn_obj as u8;
    *cur = b'[' | ((ctn_obj as u8) << 5);
    cur = cur.add(1);
    if PRETTY {
        *cur = b'\n';
        cur = cur.add(1);
    }
    let mut ctn = val;
    val = (*val).uni.ptr as *mut YyjsonMutVal; /* tail */
    val = if ctn_obj {
        (*(*val).next).next
    } else {
        (*val).next
    };
    let mut level: usize = 1;

    let mut state = WSt::ValBegin;

    let fail: WFail = 'fsm: loop {
        macro_rules! incr_len {
            ($len:expr) => {{
                let ext_len: usize = $len;
                if (cur as usize + ext_len) >= ctx as usize {
                    let ctx_pos = (ctx as *mut u8).offset_from(hdr) as usize;
                    let cur_pos = cur.offset_from(hdr) as usize;
                    let ctx_len = end.offset_from(ctx as *mut u8) as usize;
                    let mut alc_inc = if alc_len / 2 > ext_len {
                        alc_len / 2
                    } else {
                        ext_len
                    };
                    alc_inc = size_align_up(alc_inc, size_of::<MutWriteCtx>());
                    alc_len += alc_inc;
                    let tmp =
                        alc.realloc(hdr as *mut c_void, alc_len - alc_inc, alc_len) as *mut u8;
                    if tmp.is_null() {
                        break 'fsm WFail::Alloc;
                    }
                    let ctx_tmp = tmp.add(alc_len - ctx_len) as *mut MutWriteCtx;
                    libc::mem_move(ctx_tmp as *mut u8, tmp.add(ctx_pos), ctx_len);
                    ctx = ctx_tmp;
                    cur = tmp.add(cur_pos);
                    end = tmp.add(alc_len);
                    hdr = tmp;
                }
            }};
        }

        match state {
            WSt::ValBegin => {
                val_type = get_type(mut_as_val(val));
                if val_type == YYJSON_TYPE_STR {
                    let is_key = (ctn_obj as u8) & !(ctn_len as u8) != 0;
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    let str_len = get_len(mut_as_val(val));
                    let str_ptr = (*val).uni.str_ as *const u8;
                    if PRETTY {
                        incr_len!(
                            str_len * 6 + 16 + if no_indent { 0 } else { level * 4 }
                        );
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                    } else {
                        incr_len!(str_len * 6 + 16);
                    }
                    if cpy && get_subtype(mut_as_val(val)) != 0 {
                        cur = write_str_noesc(cur, str_ptr, str_len);
                    } else {
                        cur = write_str(cur, esc, inv, str_ptr, str_len, enc_table, hex_table);
                        if cur.is_null() {
                            break 'fsm WFail::Str;
                        }
                    }
                    *cur = if is_key { b':' } else { b',' };
                    cur = cur.add(1);
                    if PRETTY {
                        *cur = if is_key { b' ' } else { b'\n' };
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                if val_type == YYJSON_TYPE_NUM {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    if PRETTY {
                        incr_len!(FP_BUF_LEN + if no_indent { 0 } else { level * 4 });
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                    } else {
                        incr_len!(FP_BUF_LEN);
                    }
                    cur = write_num(cur, mut_as_val(val), flg);
                    if cur.is_null() {
                        break 'fsm WFail::Num;
                    }
                    *cur = b',';
                    cur = cur.add(1);
                    if PRETTY {
                        *cur = b'\n';
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                if (val_type & (YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ))
                    == (YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ)
                {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    let ctn_len_tmp = get_len(mut_as_val(val));
                    let ctn_obj_tmp = val_type == YYJSON_TYPE_OBJ;
                    if PRETTY {
                        incr_len!(
                            2 * size_of::<MutWriteCtx>()
                                + if no_indent { 0 } else { level * 4 }
                        );
                    } else {
                        incr_len!(2 * size_of::<MutWriteCtx>());
                    }
                    if ctn_len_tmp == 0 {
                        if PRETTY {
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        }
                        *cur = b'[' | ((ctn_obj_tmp as u8) << 5);
                        cur = cur.add(1);
                        *cur = b']' | ((ctn_obj_tmp as u8) << 5);
                        cur = cur.add(1);
                        *cur = b',';
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        state = WSt::ValEnd;
                        continue;
                    } else {
                        ctx = ctx.sub(1);
                        mut_write_ctx_set(ctx, ctn, ctn_len, ctn_obj);
                        ctn_len = ctn_len_tmp << ctn_obj_tmp as u8;
                        ctn_obj = ctn_obj_tmp;
                        if PRETTY {
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                            level += 1;
                        }
                        *cur = b'[' | ((ctn_obj as u8) << 5);
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        ctn = val;
                        val = (*ctn).uni.ptr as *mut YyjsonMutVal; /* tail */
                        val = if ctn_obj {
                            (*(*val).next).next
                        } else {
                            (*val).next
                        };
                        continue;
                    }
                }
                if val_type == YYJSON_TYPE_BOOL {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    if PRETTY {
                        incr_len!(16 + if no_indent { 0 } else { level * 4 });
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        cur = write_bool(cur, get_bool(mut_as_val(val)));
                        cur = cur.add(2);
                    } else {
                        incr_len!(16);
                        cur = write_bool(cur, get_bool(mut_as_val(val)));
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                if val_type == YYJSON_TYPE_NULL {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    if PRETTY {
                        incr_len!(16 + if no_indent { 0 } else { level * 4 });
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        cur = write_null(cur);
                        cur = cur.add(2);
                    } else {
                        incr_len!(16);
                        cur = write_null(cur);
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                if val_type == YYJSON_TYPE_RAW {
                    let no_indent = (ctn_obj as u8) & (ctn_len as u8) != 0;
                    let str_len = get_len(mut_as_val(val));
                    let str_ptr = (*val).uni.str_ as *const u8;
                    if PRETTY {
                        incr_len!(str_len + 3 + if no_indent { 0 } else { level * 4 });
                        cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                    } else {
                        incr_len!(str_len + 2);
                    }
                    cur = write_raw(cur, str_ptr, str_len);
                    *cur = b',';
                    cur = cur.add(1);
                    if PRETTY {
                        *cur = b'\n';
                        cur = cur.add(1);
                    }
                    state = WSt::ValEnd;
                    continue;
                }
                break 'fsm WFail::Type;
            }

            WSt::ValEnd => {
                ctn_len -= 1;
                if ctn_len == 0 {
                    state = WSt::CtnEnd;
                } else {
                    val = (*val).next;
                    state = WSt::ValBegin;
                }
            }

            WSt::CtnEnd => {
                if PRETTY {
                    cur = cur.sub(2);
                    *cur = b'\n';
                    cur = cur.add(1);
                    incr_len!(level * 4);
                    level -= 1;
                    cur = write_indent(cur, level, spaces);
                    *cur = b']' | ((ctn_obj as u8) << 5);
                    cur = cur.add(1);
                    if (ctx as *mut u8) >= end {
                        state = WSt::DocEnd;
                        continue;
                    }
                    val = (*ctn).next;
                    let (c, l, o) = mut_write_ctx_get(ctx);
                    ctx = ctx.add(1);
                    ctn = c;
                    ctn_len = l;
                    ctn_obj = o;
                    ctn_len -= 1;
                    *cur = b',';
                    cur = cur.add(1);
                    *cur = b'\n';
                    cur = cur.add(1);
                } else {
                    cur = cur.sub(1);
                    *cur = b']' | ((ctn_obj as u8) << 5);
                    cur = cur.add(1);
                    *cur = b',';
                    cur = cur.add(1);
                    if (ctx as *mut u8) >= end {
                        state = WSt::DocEnd;
                        continue;
                    }
                    val = (*ctn).next;
                    let (c, l, o) = mut_write_ctx_get(ctx);
                    ctx = ctx.add(1);
                    ctn = c;
                    ctn_len = l;
                    ctn_obj = o;
                    ctn_len -= 1;
                }
                if ctn_len > 0 {
                    state = WSt::ValBegin;
                } else {
                    state = WSt::CtnEnd;
                }
            }

            WSt::DocEnd => {
                if PRETTY {
                    if newline {
                        incr_len!(2);
                        *cur = b'\n';
                        cur = cur.add(1);
                    }
                    *cur = 0;
                } else {
                    if newline {
                        incr_len!(2);
                        *cur.sub(1) = b'\n';
                        cur = cur.add(1);
                    }
                    cur = cur.sub(1);
                    *cur = 0;
                }
                *dat_len = cur.offset_from(hdr) as usize;
                (*err).code = YYJSON_WRITE_SUCCESS;
                (*err).msg = ptr::null();
                return hdr;
            }
        }
    };

    let (code, m) = fail.code_msg();
    return_err!(code, m);
}

/// `mut_write_root`
unsafe fn mut_write_root(
    val: *const YyjsonMutVal,
    estimated_val_num: usize,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    buf: *mut c_char,
    dat_len_in: *mut usize,
    err_in: *mut YyjsonWriteErr,
) -> *mut c_char {
    let mut tmp_err = YyjsonWriteErr {
        code: 0,
        msg: ptr::null(),
    };
    let mut tmp_dat_len: usize = 0;
    let alc = alc_or_default(alc_ptr);
    let root = val as *mut YyjsonMutVal;

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonWriteErr
    } else {
        err_in
    };
    let dat_len = if dat_len_in.is_null() {
        &mut tmp_dat_len as *mut usize
    } else {
        dat_len_in
    };

    if root.is_null() {
        *dat_len = 0;
        (*err).msg = cstr!("input JSON is NULL");
        (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
        return ptr::null_mut();
    }

    if !is_ctn(mut_as_val(root)) || get_len(mut_as_val(root)) == 0 {
        write_root_single(mut_as_val(root), flg, alc, buf, dat_len, err) as *mut c_char
    } else if (flg & (YYJSON_WRITE_PRETTY | YYJSON_WRITE_PRETTY_TWO_SPACES)) != 0 {
        mut_write_root_ctn::<true>(root, estimated_val_num, flg, alc, buf, dat_len, err)
            as *mut c_char
    } else {
        mut_write_root_ctn::<false>(root, estimated_val_num, flg, alc, buf, dat_len, err)
            as *mut c_char
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_write_opts(
    val: *const YyjsonMutVal,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    dat_len: *mut usize,
    err: *mut YyjsonWriteErr,
) -> *mut c_char {
    mut_write_root(val, 0, flg, alc_ptr, ptr::null_mut(), dat_len, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_write_opts(
    doc: *const YyjsonMutDoc,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    dat_len: *mut usize,
    err: *mut YyjsonWriteErr,
) -> *mut c_char {
    let (root, estimated_val_num) = if !doc.is_null() {
        ((*doc).root, mut_doc_estimated_val_num(doc))
    } else {
        (ptr::null_mut(), 0)
    };
    mut_write_root(
        root,
        estimated_val_num,
        flg,
        alc_ptr,
        ptr::null_mut(),
        dat_len,
        err,
    )
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_write_buf(
    buf: *mut c_char,
    buf_len: usize,
    val: *const YyjsonMutVal,
    flg: u32,
    err: *mut YyjsonWriteErr,
) -> usize {
    if buf.is_null() || buf_len == 0 {
        if !err.is_null() {
            (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
            (*err).msg = cstr!("input buf or buf_len is invalid");
        }
        0
    } else {
        let mut buf_len = buf_len;
        mut_write_root(val, 0, flg, &YYJSON_NULL_ALC, buf, &mut buf_len, err);
        buf_len
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_write_buf(
    buf: *mut c_char,
    buf_len: usize,
    doc: *const YyjsonMutDoc,
    flg: u32,
    err: *mut YyjsonWriteErr,
) -> usize {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_mut_val_write_buf(buf, buf_len, root, flg, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_write_file(
    path: *const c_char,
    val: *const YyjsonMutVal,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err_in: *mut YyjsonWriteErr,
) -> bool {
    let mut tmp_err = YyjsonWriteErr {
        code: 0,
        msg: ptr::null(),
    };
    let alc = alc_or_default(alc_ptr);
    let mut dat_len: usize = 0;
    let root = val as *mut YyjsonMutVal;

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonWriteErr
    } else {
        err_in
    };
    if path.is_null() || *path == 0 {
        (*err).msg = cstr!("input path is invalid");
        (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
        return false;
    }

    let dat = yyjson_mut_val_write_opts(root, flg, &alc, &mut dat_len, err) as *mut u8;
    if dat.is_null() {
        return false;
    }
    let suc = write_dat_to_file(path, dat, dat_len, err);
    alc.dealloc(dat as *mut c_void);
    suc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_write_fp(
    fp: *mut libc::File,
    val: *const YyjsonMutVal,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err_in: *mut YyjsonWriteErr,
) -> bool {
    let mut tmp_err = YyjsonWriteErr {
        code: 0,
        msg: ptr::null(),
    };
    let alc = alc_or_default(alc_ptr);
    let mut dat_len: usize = 0;
    let root = val as *mut YyjsonMutVal;

    let err = if err_in.is_null() {
        &mut tmp_err as *mut YyjsonWriteErr
    } else {
        err_in
    };
    if fp.is_null() {
        (*err).msg = cstr!("input fp is invalid");
        (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
        return false;
    }

    let dat = yyjson_mut_val_write_opts(root, flg, &alc, &mut dat_len, err) as *mut u8;
    if dat.is_null() {
        return false;
    }
    let suc = write_dat_to_fp(fp, dat, dat_len, err);
    alc.dealloc(dat as *mut c_void);
    suc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_write_file(
    path: *const c_char,
    doc: *const YyjsonMutDoc,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err: *mut YyjsonWriteErr,
) -> bool {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_mut_val_write_file(path, root, flg, alc_ptr, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_write_fp(
    fp: *mut libc::File,
    doc: *const YyjsonMutDoc,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
    err: *mut YyjsonWriteErr,
) -> bool {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_mut_val_write_fp(fp, root, flg, alc_ptr, err)
}
