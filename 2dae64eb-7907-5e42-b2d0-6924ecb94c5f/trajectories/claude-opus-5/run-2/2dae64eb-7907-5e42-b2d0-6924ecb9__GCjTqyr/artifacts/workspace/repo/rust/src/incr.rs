//! Incremental JSON reader.
//!
//! Transliteration of the "Incremental JSON Reader (Public)" section of the
//! original `src/yyjson.c`. The parser state lives in `YyjsonIncrState`, which
//! is opaque to the API; `save_incr_state!` records a resumable position and
//! `state.label` selects where the next call re-enters the machine.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alc::alc_or_default;
use crate::libc;
use crate::read::{is_truncated_end, Fail};
use crate::rnum::*;
use crate::rstr::*;
use crate::types::*;
use crate::util::*;

const LABEL_DOC_BEGIN: u32 = 0;
const LABEL_ARR_VAL_BEGIN: u32 = 1;
const LABEL_ARR_VAL_END: u32 = 2;
const LABEL_OBJ_KEY_BEGIN: u32 = 3;
const LABEL_OBJ_KEY_END: u32 = 4;
const LABEL_OBJ_VAL_BEGIN: u32 = 5;
const LABEL_OBJ_VAL_END: u32 = 6;
const LABEL_DOC_END: u32 = 7;

/// `struct yyjson_incr_state`
#[repr(C)]
pub struct YyjsonIncrState {
    pub label: u32,
    pub alc: YyjsonAlc,
    pub flg: u32,
    pub hdr: *mut u8,
    pub cur: *mut u8,
    pub buf_len: usize,
    pub hdr_len: usize,
    pub alc_len: usize,
    pub ctn_len: usize,
    pub val_hdr: *mut YyjsonVal,
    pub val_end: *mut YyjsonVal,
    pub val: *mut YyjsonVal,
    pub ctn: *mut YyjsonVal,
    pub str_con: [*mut u8; 2],
    pub raw_ptr: *mut u8,
    pub raw_end: [u8; 1],
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_incr_new(
    buf: *mut c_char,
    buf_len: usize,
    flg: u32,
    alc_ptr: *const YyjsonAlc,
) -> *mut YyjsonIncrState {
    let alc = alc_or_default(alc_ptr);

    /* remove non-standard flags */
    let mut flg = flg;
    flg &= !YYJSON_READ_JSON5;
    flg &= !YYJSON_READ_ALLOW_BOM;
    flg &= !YYJSON_READ_ALLOW_INVALID_UNICODE;

    if buf.is_null() {
        return ptr::null_mut();
    }
    if buf_len >= USIZE_MAX - YYJSON_PADDING_SIZE {
        return ptr::null_mut();
    }
    let state = alc.alloc(size_of::<YyjsonIncrState>()) as *mut YyjsonIncrState;
    if state.is_null() {
        return ptr::null_mut();
    }
    libc::zero(state);
    (*state).alc = alc;
    (*state).flg = flg;
    (*state).buf_len = buf_len;

    /* add 4-byte zero padding for input data if necessary */
    if has_rflag(flg, YYJSON_READ_INSITU) {
        (*state).hdr = buf as *mut u8;
    } else {
        (*state).hdr = alc.alloc(buf_len + YYJSON_PADDING_SIZE) as *mut u8;
        if (*state).hdr.is_null() {
            alc.dealloc(state as *mut c_void);
            return ptr::null_mut();
        }
        libc::mem_copy((*state).hdr, buf as *const u8, buf_len);
    }
    libc::mem_set((*state).hdr.add(buf_len), 0, YYJSON_PADDING_SIZE);
    (*state).cur = (*state).hdr;
    (*state).raw_ptr = (*state).raw_end.as_mut_ptr();
    (*state).label = LABEL_DOC_BEGIN;
    state
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_incr_free(state: *mut YyjsonIncrState) {
    if !state.is_null() {
        let alc = (*state).alc;
        libc::zero(&mut (*state).alc);
        if !(*state).val_hdr.is_null() {
            alc.dealloc((*state).val_hdr as *mut c_void);
        }
        if !(*state).hdr.is_null() && ((*state).flg & YYJSON_READ_INSITU) == 0 {
            alc.dealloc((*state).hdr as *mut c_void);
        }
        alc.dealloc(state as *mut c_void);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum St {
    DocBegin,
    ArrBegin,
    ArrValBegin,
    ArrValContinue,
    ArrValMaybeEnd,
    ArrValEnd,
    ArrEnd,
    ObjBegin,
    ObjKeyBegin,
    ObjKeyContinue,
    ObjKeyEnd,
    ObjValBegin,
    ObjValContinue,
    ObjValMaybeEnd,
    ObjValEnd,
    ObjEnd,
    DocEnd,
}

/// Outcome of the incremental state machine.
enum Out {
    Fail(Fail),
    UnexpectedEnd,
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_incr_read(
    state: *mut YyjsonIncrState,
    len: usize,
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

    macro_rules! return_err_inv_param {
        ($m:expr) => {{
            (*err).pos = 0;
            (*err).msg = $m;
            (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
            return ptr::null_mut();
        }};
    }

    /* validate input parameters */
    if state.is_null() {
        return_err_inv_param!(cstr!("input state is NULL"));
    }
    if len == 0 {
        return_err_inv_param!(cstr!("input length is 0"));
    }
    if len > (*state).buf_len {
        return_err_inv_param!(cstr!("length is greater than total input length"));
    }

    /* restore state saved from the previous call */
    let hdr = (*state).hdr;
    let end = (*state).hdr.add(len);
    let mut cur = (*state).cur;
    let flg = (*state).flg;
    let alc = (*state).alc;
    let mut ctn_len = (*state).ctn_len;
    let mut hdr_len = (*state).hdr_len;
    let mut alc_len = (*state).alc_len;
    let mut val = (*state).val;
    let mut val_hdr = (*state).val_hdr;
    let mut val_end = (*state).val_end;
    let mut ctn = (*state).ctn;
    let con: *mut *mut u8 = (*state).str_con.as_mut_ptr();
    let mut raw_ptr = (*state).raw_ptr;
    let pre: *mut *mut u8 = &mut raw_ptr;
    let alc_max = USIZE_MAX / size_of::<YyjsonVal>();
    let mut msg: *const c_char = ptr::null();

    /* insert a null terminator so that we stop at the specified end, even if
       the data contains more valid JSON */
    let saved_end = *end;
    *end = 0;

    let mut state_lbl = match (*state).label {
        LABEL_DOC_BEGIN => St::DocBegin,
        LABEL_ARR_VAL_BEGIN => St::ArrValBegin,
        LABEL_ARR_VAL_END => St::ArrValEnd,
        LABEL_OBJ_KEY_BEGIN => St::ObjKeyBegin,
        LABEL_OBJ_KEY_END => St::ObjKeyEnd,
        LABEL_OBJ_VAL_BEGIN => St::ObjValBegin,
        LABEL_OBJ_VAL_END => St::ObjValEnd,
        LABEL_DOC_END => St::DocEnd,
        _ => {
            *end = saved_end;
            return_err_inv_param!(cstr!("invalid incremental state"));
        }
    };

    let out: Out = 'fsm: loop {
        macro_rules! val_incr {
            () => {{
                val = val.add(1);
                if val >= val_end {
                    let alc_old = alc_len;
                    alc_len += alc_len / 2;
                    let val_tmp = alc.realloc(
                        val_hdr as *mut c_void,
                        alc_old * size_of::<YyjsonVal>(),
                        alc_len * size_of::<YyjsonVal>(),
                    ) as *mut YyjsonVal;
                    if val_tmp.is_null() {
                        break 'fsm Out::Fail(Fail::Alloc);
                    }
                    val = val_tmp.add(val.offset_from(val_hdr) as usize);
                    ctn = val_tmp.add(ctn.offset_from(val_hdr) as usize);
                    (*state).val =
                        val_tmp.add((*state).val.offset_from(val_hdr) as usize);
                    val_hdr = val_tmp;
                    (*state).val_hdr = val_tmp;
                    val_end = val_tmp.add(alc_len - 2);
                    (*state).val_end = val_end;
                }
            }};
        }
        macro_rules! save_incr_state {
            ($label:expr) => {{
                (*state).label = $label;
                (*state).cur = cur;
                (*state).val = val;
                (*state).ctn_len = ctn_len;
                (*state).hdr_len = hdr_len;
                (*state).raw_ptr = raw_ptr;
                if cur >= end {
                    break 'fsm Out::UnexpectedEnd;
                }
            }};
        }
        macro_rules! check_maybe_truncated_number {
            () => {{
                if cur >= end {
                    if (cur as usize) > ((*state).cur as usize + INCR_NUM_MAX_LEN) {
                        msg = cstr!("number too long");
                        break 'fsm Out::Fail(Fail::Num);
                    }
                    break 'fsm Out::UnexpectedEnd;
                }
            }};
        }

        match state_lbl {
            St::DocBegin => {
                /* skip empty contents before the json document */
                if !char_is_ctn(*cur) {
                    while char_is_space(*cur) {
                        cur = cur.add(1);
                    }
                    if cur >= end {
                        /* input data is empty */
                        break 'fsm Out::UnexpectedEnd;
                    }
                }

                /* allocate memory for the document */
                if val_hdr.is_null() {
                    hdr_len = size_of::<YyjsonDoc>() / size_of::<YyjsonVal>();
                    hdr_len += ((size_of::<YyjsonDoc>() % size_of::<YyjsonVal>()) > 0) as usize;
                    if char_is_ctn(*cur) {
                        let dat_len = if has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE) {
                            256usize
                        } else {
                            (*state).buf_len
                        };
                        alc_len = hdr_len
                            + (dat_len / YYJSON_READER_ESTIMATED_MINIFY_RATIO)
                            + 4;
                        if alc_len > alc_max {
                            alc_len = alc_max;
                        }
                    } else {
                        alc_len = hdr_len + 1; /* single value */
                    }
                    val_hdr = alc.alloc(alc_len * size_of::<YyjsonVal>()) as *mut YyjsonVal;
                    if val_hdr.is_null() {
                        break 'fsm Out::Fail(Fail::Alloc);
                    }
                    val_end = val_hdr.add(alc_len - 2);
                    val = val_hdr.add(hdr_len);
                    ctn = val;
                    ctn_len = 0;
                    (*state).val_hdr = val_hdr;
                    (*state).val_end = val_end;
                    save_incr_state!(LABEL_DOC_BEGIN);
                }

                /* read json document */
                if *cur == b'{' {
                    cur = cur.add(1);
                    (*ctn).tag = YYJSON_TYPE_OBJ as u64;
                    (*ctn).uni.ofs = 0;
                    state_lbl = St::ObjKeyBegin;
                    continue;
                }
                if *cur == b'[' {
                    cur = cur.add(1);
                    (*ctn).tag = YYJSON_TYPE_ARR as u64;
                    (*ctn).uni.ofs = 0;
                    state_lbl = St::ArrValBegin;
                    continue;
                }
                if char_is_num(*cur) {
                    if read_num(&mut cur, pre, flg, val, &mut msg) {
                        /* a root number may continue in a later chunk */
                        if len < (*state).buf_len {
                            check_maybe_truncated_number!();
                        }
                        state_lbl = St::DocEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::Num);
                }
                if *cur == b'"' {
                    if read_str_con(&mut cur, end, flg, val, &mut msg, con) {
                        state_lbl = St::DocEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::Str);
                }
                if *cur == b't' {
                    if read_true(&mut cur, val) {
                        state_lbl = St::DocEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitTrue);
                }
                if *cur == b'f' {
                    if read_false(&mut cur, val) {
                        state_lbl = St::DocEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitFalse);
                }
                if *cur == b'n' {
                    if read_null(&mut cur, val) {
                        state_lbl = St::DocEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitNull);
                }

                msg = cstr!("unexpected character, expected a valid root value");
                if cur == hdr {
                    /* RFC 8259: JSON text MUST be encoded using UTF-8 */
                    if is_utf8_bom(hdr) {
                        msg = MSG_ERR_BOM;
                    } else if len >= 4 && is_utf32_bom(hdr) {
                        msg = MSG_ERR_UTF32;
                    } else if len >= 2 && is_utf16_bom(hdr) {
                        msg = MSG_ERR_UTF16;
                    }
                }
                break 'fsm Out::Fail(Fail::CharMsg);
            }

            St::ArrBegin => {
                /* save current container */
                (*ctn).tag = (((ctn_len as u64) + 1) << YYJSON_TAG_BIT)
                    | ((*ctn).tag & YYJSON_TAG_MASK as u64);

                /* create a new array value, save parent container offset */
                val_incr!();
                (*val).tag = YYJSON_TYPE_ARR as u64;
                (*val).uni.ofs = (val as *mut u8).offset_from(ctn as *mut u8) as usize;

                ctn = val;
                ctn_len = 0;
                state_lbl = St::ArrValBegin;
            }

            St::ArrValBegin => {
                save_incr_state!(LABEL_ARR_VAL_BEGIN);
                state_lbl = St::ArrValContinue;
            }

            St::ArrValContinue => {
                if *cur == b'{' {
                    cur = cur.add(1);
                    state_lbl = St::ObjBegin;
                    continue;
                }
                if *cur == b'[' {
                    cur = cur.add(1);
                    state_lbl = St::ArrBegin;
                    continue;
                }
                if char_is_num(*cur) {
                    val_incr!();
                    ctn_len += 1;
                    if read_num(&mut cur, pre, flg, val, &mut msg) {
                        state_lbl = St::ArrValMaybeEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::Num);
                }
                if *cur == b'"' {
                    val_incr!();
                    ctn_len += 1;
                    if read_str_con(&mut cur, end, flg, val, &mut msg, con) {
                        state_lbl = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::Str);
                }
                if *cur == b't' {
                    val_incr!();
                    ctn_len += 1;
                    if read_true(&mut cur, val) {
                        state_lbl = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitTrue);
                }
                if *cur == b'f' {
                    val_incr!();
                    ctn_len += 1;
                    if read_false(&mut cur, val) {
                        state_lbl = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitFalse);
                }
                if *cur == b'n' {
                    val_incr!();
                    ctn_len += 1;
                    if read_null(&mut cur, val) {
                        state_lbl = St::ArrValEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitNull);
                }
                if *cur == b']' {
                    cur = cur.add(1);
                    if ctn_len == 0 {
                        state_lbl = St::ArrEnd;
                        continue;
                    }
                    loop {
                        cur = cur.sub(1);
                        if *cur == b',' {
                            break;
                        }
                    }
                    break 'fsm Out::Fail(Fail::TrailingComma);
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
                break 'fsm Out::Fail(Fail::CharVal);
            }

            St::ArrValMaybeEnd => {
                /* if parsing stops in the middle of a number, it may continue
                   with more digits, so the value may not have ended yet */
                check_maybe_truncated_number!();
                state_lbl = St::ArrValEnd;
            }

            St::ArrValEnd => {
                save_incr_state!(LABEL_ARR_VAL_END);
                if *cur == b',' {
                    cur = cur.add(1);
                    state_lbl = St::ArrValBegin;
                    continue;
                }
                if *cur == b']' {
                    cur = cur.add(1);
                    state_lbl = St::ArrEnd;
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
                break 'fsm Out::Fail(Fail::CharArrEnd);
            }

            St::ArrEnd => {
                /* get parent container */
                let ctn_parent = (ctn as *mut u8).sub((*ctn).uni.ofs) as *mut YyjsonVal;

                /* save the next sibling value offset */
                (*ctn).uni.ofs = ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                    + size_of::<YyjsonVal>();
                (*ctn).tag = ((ctn_len as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_ARR as u64;
                if ctn == ctn_parent {
                    state_lbl = St::DocEnd;
                    continue;
                }

                ctn = ctn_parent;
                ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                state_lbl =
                    if ((*ctn).tag & YYJSON_TYPE_MASK as u64) == YYJSON_TYPE_OBJ as u64 {
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
                (*val).uni.ofs = (val as *mut u8).offset_from(ctn as *mut u8) as usize;
                ctn = val;
                ctn_len = 0;
                state_lbl = St::ObjKeyBegin;
            }

            St::ObjKeyBegin => {
                save_incr_state!(LABEL_OBJ_KEY_BEGIN);
                state_lbl = St::ObjKeyContinue;
            }

            St::ObjKeyContinue => {
                if *cur == b'"' {
                    val_incr!();
                    ctn_len += 1;
                    if read_str_con(&mut cur, end, flg, val, &mut msg, con) {
                        state_lbl = St::ObjKeyEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::Str);
                }
                if *cur == b'}' {
                    cur = cur.add(1);
                    if ctn_len == 0 {
                        state_lbl = St::ObjEnd;
                        continue;
                    }
                    loop {
                        cur = cur.sub(1);
                        if *cur == b',' {
                            break;
                        }
                    }
                    break 'fsm Out::Fail(Fail::TrailingComma);
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
                break 'fsm Out::Fail(Fail::CharObjKey);
            }

            St::ObjKeyEnd => {
                save_incr_state!(LABEL_OBJ_KEY_END);
                if *cur == b':' {
                    cur = cur.add(1);
                    state_lbl = St::ObjValBegin;
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
                break 'fsm Out::Fail(Fail::CharObjSep);
            }

            St::ObjValBegin => {
                save_incr_state!(LABEL_OBJ_VAL_BEGIN);
                state_lbl = St::ObjValContinue;
            }

            St::ObjValContinue => {
                if *cur == b'"' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_str_con(&mut cur, end, flg, val, &mut msg, con) {
                        state_lbl = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::Str);
                }
                if char_is_num(*cur) {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_num(&mut cur, pre, flg, val, &mut msg) {
                        state_lbl = St::ObjValMaybeEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::Num);
                }
                if *cur == b'{' {
                    cur = cur.add(1);
                    state_lbl = St::ObjBegin;
                    continue;
                }
                if *cur == b'[' {
                    cur = cur.add(1);
                    state_lbl = St::ArrBegin;
                    continue;
                }
                if *cur == b't' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_true(&mut cur, val) {
                        state_lbl = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitTrue);
                }
                if *cur == b'f' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_false(&mut cur, val) {
                        state_lbl = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitFalse);
                }
                if *cur == b'n' {
                    val = val.add(1);
                    ctn_len += 1;
                    if read_null(&mut cur, val) {
                        state_lbl = St::ObjValEnd;
                        continue;
                    }
                    break 'fsm Out::Fail(Fail::LitNull);
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
                break 'fsm Out::Fail(Fail::CharVal);
            }

            St::ObjValMaybeEnd => {
                check_maybe_truncated_number!();
                state_lbl = St::ObjValEnd;
            }

            St::ObjValEnd => {
                save_incr_state!(LABEL_OBJ_VAL_END);
                if *cur == b',' {
                    cur = cur.add(1);
                    state_lbl = St::ObjKeyBegin;
                    continue;
                }
                if *cur == b'}' {
                    cur = cur.add(1);
                    state_lbl = St::ObjEnd;
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
                break 'fsm Out::Fail(Fail::CharObjEnd);
            }

            St::ObjEnd => {
                /* pop container */
                let ctn_parent = (ctn as *mut u8).sub((*ctn).uni.ofs) as *mut YyjsonVal;
                (*ctn).uni.ofs = ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                    + size_of::<YyjsonVal>();
                (*ctn).tag =
                    ((ctn_len as u64) << (YYJSON_TAG_BIT - 1)) | YYJSON_TYPE_OBJ as u64;
                if ctn == ctn_parent {
                    state_lbl = St::DocEnd;
                    continue;
                }
                ctn = ctn_parent;
                ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                state_lbl =
                    if ((*ctn).tag & YYJSON_TYPE_MASK as u64) == YYJSON_TYPE_OBJ as u64 {
                        St::ObjValEnd
                    } else {
                        St::ArrValEnd
                    };
            }

            St::DocEnd => {
                /* check invalid contents after the json document */
                if (cur < end || len < (*state).buf_len)
                    && !has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE)
                {
                    save_incr_state!(LABEL_DOC_END);
                    while char_is_space(*cur) {
                        cur = cur.add(1);
                    }
                    if cur < end {
                        break 'fsm Out::Fail(Fail::Garbage);
                    }
                    /* the document is complete for the bytes seen so far, but
                       more input is pending and it may hold trailing content
                       that has to be rejected */
                    if len < (*state).buf_len {
                        break 'fsm Out::UnexpectedEnd;
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
                (*state).hdr = ptr::null_mut();
                (*state).val_hdr = ptr::null_mut();
                libc::zero(err);
                return doc;
            }
        }
    };

    match out {
        Out::UnexpectedEnd => {
            (*err).pos = len;
            /* if there is no more data, stop the incremental read */
            if len >= (*state).buf_len {
                (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                (*err).msg = MSG_NOT_END;
                return ptr::null_mut();
            }
            /* save parser state in addition to the last `save_incr_state` */
            (*err).code = YYJSON_READ_ERROR_MORE;
            (*err).msg = cstr!("need more data");
            (*state).val_end = val_end;
            (*state).ctn = ctn;
            (*state).alc_len = alc_len;
            /* restore the end where we inserted a null terminator */
            *end = saved_end;
            ptr::null_mut()
        }
        Out::Fail(f) => {
            let (code, m) = f.code_msg(msg);
            if is_truncated_end(hdr, cur, end, code, flg) {
                /* goto unexpected_end */
                (*err).pos = len;
                if len >= (*state).buf_len {
                    (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                    (*err).msg = MSG_NOT_END;
                    return ptr::null_mut();
                }
                (*err).code = YYJSON_READ_ERROR_MORE;
                (*err).msg = cstr!("need more data");
                (*state).val_end = val_end;
                (*state).ctn = ctn;
                (*state).alc_len = alc_len;
                *end = saved_end;
                return ptr::null_mut();
            }
            (*err).pos = cur.offset_from(hdr) as usize;
            (*err).code = code;
            (*err).msg = m;
            ptr::null_mut()
        }
    }
}
