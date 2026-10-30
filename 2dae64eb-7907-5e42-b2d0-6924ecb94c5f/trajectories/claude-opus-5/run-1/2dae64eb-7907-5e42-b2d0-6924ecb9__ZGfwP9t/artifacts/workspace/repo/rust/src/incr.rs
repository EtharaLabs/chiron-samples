//! The incremental JSON reader (`yyjson_incr_new` / `yyjson_incr_read` /
//! `yyjson_incr_free`).
//!
//! `yyjson_incr_state` is opaque in the public header, so this port is free
//! to lay it out as it likes; it keeps exactly the fields the C struct has.
//!
//! How resuming works, exactly as in the C version: the parser only ever
//! suspends at one of eight "save points" (`LABEL_*`), each of which stores
//! the cursor, the current value, the container length, the pool bookkeeping
//! and the deferred raw-terminator position into the state, and then checks
//! whether the cursor has reached the end of the bytes supplied so far. If it
//! has, the call returns `YYJSON_READ_ERROR_MORE` and the next call re-enters
//! at that label. Strings are the one construct that can be suspended
//! *inside*: `read_str_opt` stores `(src, dst)` in `state.str_con` and
//! resumes the copy loop from there. Numbers are never suspended inside;
//! instead a number that ends exactly at the end of the supplied bytes is
//! treated as "maybe not finished yet" and the parser rewinds to the save
//! point that preceded it.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alc::YYJSON_DEFAULT_ALC;
use crate::mem;
use crate::read::*;
use crate::types::*;
use crate::util::*;

/* labels within `yyjson_incr_read` at which parsing can resume */
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
pub struct yyjson_incr_state {
    label: u32,
    alc: yyjson_alc,
    flg: yyjson_read_flag,
    hdr: *mut u8,
    cur: *mut u8,
    buf_len: usize,
    hdr_len: usize,
    alc_len: usize,
    ctn_len: usize,
    val_hdr: *mut yyjson_val,
    val_end: *mut yyjson_val,
    val: *mut yyjson_val,
    ctn: *mut yyjson_val,
    str_con: [*mut u8; 2],
    raw_ptr: *mut u8,
    raw_end: [u8; 1],
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_incr_new(
    buf: *mut c_char,
    buf_len: usize,
    flg: yyjson_read_flag,
    alc_ptr: *const yyjson_alc,
) -> *mut yyjson_incr_state {
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };

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
    let state =
        alc.malloc(size_of::<yyjson_incr_state>()) as *mut yyjson_incr_state;
    if state.is_null() {
        return ptr::null_mut();
    }
    mem::set(state as *mut u8, 0, size_of::<yyjson_incr_state>());
    (*state).alc = alc;
    (*state).flg = flg;
    (*state).buf_len = buf_len;

    /* add 4-byte zero padding for input data if necessary */
    if has_flg(flg, YYJSON_READ_INSITU) {
        (*state).hdr = buf as *mut u8;
    } else {
        (*state).hdr = alc.malloc(buf_len + YYJSON_PADDING_SIZE) as *mut u8;
        if (*state).hdr.is_null() {
            alc.free(state as *mut c_void);
            return ptr::null_mut();
        }
        mem::copy((*state).hdr, buf as *const u8, buf_len);
    }
    mem::set((*state).hdr.add(buf_len), 0, YYJSON_PADDING_SIZE);
    (*state).cur = (*state).hdr;
    (*state).raw_ptr = (*state).raw_end.as_mut_ptr();
    (*state).label = LABEL_DOC_BEGIN;
    state
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_incr_free(state: *mut yyjson_incr_state) {
    if !state.is_null() {
        let alc = (*state).alc;
        mem::set(
            &mut (*state).alc as *mut yyjson_alc as *mut u8,
            0,
            size_of::<yyjson_alc>(),
        );
        if !(*state).val_hdr.is_null() {
            alc.free((*state).val_hdr as *mut c_void);
        }
        if !(*state).hdr.is_null() && ((*state).flg & YYJSON_READ_INSITU) == 0 {
            alc.free((*state).hdr as *mut c_void);
        }
        alc.free(state as *mut c_void);
    }
}

/// The `goto` labels of `yyjson_incr_read`.
#[derive(Copy, Clone, PartialEq)]
enum IState {
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

/// The outcome of the incremental state machine.
enum IOut {
    Doc(*mut yyjson_doc),
    /// `goto unexpected_end`
    NeedMore,
    /// `return_err(pos, code, msg)`
    Fail(*mut u8, u32, *const c_char),
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_incr_read(
    state: *mut yyjson_incr_state,
    len: usize,
    err: *mut yyjson_read_err,
) -> *mut yyjson_doc {
    let mut tmp_err = yyjson_read_err {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_read_err
    } else {
        err
    };

    macro_rules! return_err_inv_param {
        ($m:expr) => {{
            (*err).pos = 0;
            (*err).msg = $m.as_ptr() as *const c_char;
            (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
            return ptr::null_mut();
        }};
    }

    /* validate input parameters */
    if state.is_null() {
        return_err_inv_param!(b"input state is NULL\0");
    }
    if len == 0 {
        return_err_inv_param!(b"input length is 0\0");
    }
    if len > (*state).buf_len {
        return_err_inv_param!(b"length is greater than total input length\0");
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
    let con: *mut [*mut u8; 2] = &mut (*state).str_con;
    let mut raw_ptr = (*state).raw_ptr;
    let alc_max = USIZE_MAX / size_of::<yyjson_val>();
    let mut msg: *const c_char = ptr::null();

    let first_label = (*state).label;
    if first_label > LABEL_DOC_END {
        return_err_inv_param!(b"invalid incremental state\0");
    }

    /* insert null terminator to make us stop at the specified end, even if
       the data contains more valid JSON */
    let saved_end = *end;
    *end = 0;

    let mut state_lbl = match first_label {
        LABEL_DOC_BEGIN => IState::DocBegin,
        LABEL_ARR_VAL_BEGIN => IState::ArrValBegin,
        LABEL_ARR_VAL_END => IState::ArrValEnd,
        LABEL_OBJ_KEY_BEGIN => IState::ObjKeyBegin,
        LABEL_OBJ_KEY_END => IState::ObjKeyEnd,
        LABEL_OBJ_VAL_BEGIN => IState::ObjValBegin,
        LABEL_OBJ_VAL_END => IState::ObjValEnd,
        _ => IState::DocEnd,
    };

    let out: IOut = 'fsm: {
        macro_rules! val_incr {
            () => {{
                val = val.add(1);
                if val >= val_end {
                    let alc_old = alc_len;
                    alc_len += alc_len / 2;
                    if size_of::<usize>() < 8 && alc_len >= alc_max {
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                            crate::util::msg(MSG_MALLOC),
                        );
                    }
                    let val_tmp = alc.realloc(
                        val_hdr as *mut c_void,
                        alc_old * size_of::<yyjson_val>(),
                        alc_len * size_of::<yyjson_val>(),
                    ) as *mut yyjson_val;
                    if val_tmp.is_null() {
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                            crate::util::msg(MSG_MALLOC),
                        );
                    }
                    let val_ofs = val.offset_from(val_hdr);
                    let ctn_ofs = ctn.offset_from(val_hdr);
                    let st_ofs = (*state).val.offset_from(val_hdr);
                    val = val_tmp.offset(val_ofs);
                    ctn = val_tmp.offset(ctn_ofs);
                    (*state).val = val_tmp.offset(st_ofs);
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
                    break 'fsm IOut::NeedMore;
                }
            }};
        }
        macro_rules! check_maybe_truncated_number {
            () => {{
                if cur >= end {
                    if cur > (*state).cur.add(INCR_NUM_MAX_LEN) {
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_INVALID_NUMBER,
                            b"number too long\0".as_ptr() as *const c_char,
                        );
                    }
                    break 'fsm IOut::NeedMore;
                }
            }};
        }
        macro_rules! skip_spaces {
            () => {{
                loop {
                    cur = cur.add(1);
                    if !char_is_space(*cur) {
                        break;
                    }
                }
            }};
        }
        loop {
            match state_lbl {
                IState::DocBegin => {
                    /* skip empty contents before json document */
                    if !char_is_ctn(*cur) {
                        while char_is_space(*cur) {
                            cur = cur.add(1);
                        }
                        if cur >= end {
                            break 'fsm IOut::NeedMore;
                        }
                    }

                    /* allocate memory for document */
                    if val_hdr.is_null() {
                        hdr_len = HDR_LEN;
                        if char_is_ctn(*cur) {
                            let dat_len = if has_flg(flg, YYJSON_READ_STOP_WHEN_DONE) {
                                256
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
                        val_hdr =
                            alc.malloc(alc_len * size_of::<yyjson_val>()) as *mut yyjson_val;
                        if val_hdr.is_null() {
                            break 'fsm IOut::Fail(
                                cur,
                                YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                                crate::util::msg(MSG_MALLOC),
                            );
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
                        state_lbl = IState::ObjKeyBegin;
                        continue;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        (*ctn).tag = YYJSON_TYPE_ARR as u64;
                        (*ctn).uni.ofs = 0;
                        state_lbl = IState::ArrValBegin;
                        continue;
                    }
                    if char_is_num(*cur) {
                        if read_num(&mut cur, &mut raw_ptr, flg, val, &mut msg) {
                            /* a root number may continue with more digits */
                            if len < (*state).buf_len {
                                check_maybe_truncated_number!();
                            }
                            state_lbl = IState::DocEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(cur, YYJSON_READ_ERROR_INVALID_NUMBER, msg);
                    }
                    if *cur == b'"' {
                        if read_str_con(&mut cur, end, flg, val, &mut msg, con) {
                            state_lbl = IState::DocEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(cur, YYJSON_READ_ERROR_INVALID_STRING, msg);
                    }
                    if *cur == b't' {
                        if read_true(&mut cur, val) {
                            state_lbl = IState::DocEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_T),
                        );
                    }
                    if *cur == b'f' {
                        if read_false(&mut cur, val) {
                            state_lbl = IState::DocEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_F),
                        );
                    }
                    if *cur == b'n' {
                        if read_null(&mut cur, val) {
                            state_lbl = IState::DocEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_N),
                        );
                    }

                    let mut m: *const c_char =
                        b"unexpected character, expected a valid root value\0".as_ptr()
                            as *const c_char;
                    if cur == hdr {
                        /* RFC 8259: JSON text MUST be encoded using UTF-8 */
                        if is_utf8_bom(hdr) {
                            m = crate::util::msg(MSG_ERR_BOM);
                        } else if len >= 4 && is_utf32_bom(hdr) {
                            m = crate::util::msg(MSG_ERR_UTF32);
                        } else if len >= 2 && is_utf16_bom(hdr) {
                            m = crate::util::msg(MSG_ERR_UTF16);
                        }
                    }
                    break 'fsm IOut::Fail(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, m);
                }

                IState::ArrBegin => {
                    /* save current container */
                    (*ctn).tag = (((ctn_len as u64) + 1) << YYJSON_TAG_BIT)
                        | ((*ctn).tag & YYJSON_TAG_MASK as u64);
                    /* create a new array value, save parent container offset */
                    val_incr!();
                    (*val).tag = YYJSON_TYPE_ARR as u64;
                    (*val).uni.ofs = val as usize - ctn as usize;
                    /* push the new array value as current container */
                    ctn = val;
                    ctn_len = 0;
                    state_lbl = IState::ArrValBegin;
                }

                IState::ArrValBegin => {
                    save_incr_state!(LABEL_ARR_VAL_BEGIN);
                    state_lbl = IState::ArrValContinue;
                }

                IState::ArrValContinue => {
                    if *cur == b'{' {
                        cur = cur.add(1);
                        state_lbl = IState::ObjBegin;
                        continue;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        state_lbl = IState::ArrBegin;
                        continue;
                    }
                    if char_is_num(*cur) {
                        val_incr!();
                        ctn_len += 1;
                        if read_num(&mut cur, &mut raw_ptr, flg, val, &mut msg) {
                            state_lbl = IState::ArrValMaybeEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(cur, YYJSON_READ_ERROR_INVALID_NUMBER, msg);
                    }
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if read_str_con(&mut cur, end, flg, val, &mut msg, con) {
                            state_lbl = IState::ArrValEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(cur, YYJSON_READ_ERROR_INVALID_STRING, msg);
                    }
                    if *cur == b't' {
                        val_incr!();
                        ctn_len += 1;
                        if read_true(&mut cur, val) {
                            state_lbl = IState::ArrValEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_T),
                        );
                    }
                    if *cur == b'f' {
                        val_incr!();
                        ctn_len += 1;
                        if read_false(&mut cur, val) {
                            state_lbl = IState::ArrValEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_F),
                        );
                    }
                    if *cur == b'n' {
                        val_incr!();
                        ctn_len += 1;
                        if read_null(&mut cur, val) {
                            state_lbl = IState::ArrValEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_N),
                        );
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            state_lbl = IState::ArrEnd;
                            continue;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_JSON_STRUCTURE,
                            crate::util::msg(MSG_COMMA),
                        );
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    break 'fsm IOut::Fail(
                        cur,
                        YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                        crate::util::msg(MSG_CHAR),
                    );
                }

                IState::ArrValMaybeEnd => {
                    check_maybe_truncated_number!();
                    state_lbl = IState::ArrValEnd;
                }

                IState::ArrValEnd => {
                    save_incr_state!(LABEL_ARR_VAL_END);
                    if *cur == b',' {
                        cur = cur.add(1);
                        state_lbl = IState::ArrValBegin;
                        continue;
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        state_lbl = IState::ArrEnd;
                        continue;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    break 'fsm IOut::Fail(
                        cur,
                        YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                        crate::util::msg(MSG_ARR_END),
                    );
                }

                IState::ArrEnd => {
                    /* get parent container */
                    let ctn_parent = (ctn as *mut u8).sub((*ctn).uni.ofs) as *mut yyjson_val;
                    /* save the next sibling value offset */
                    (*ctn).uni.ofs = (val as usize - ctn as usize) + size_of::<yyjson_val>();
                    (*ctn).tag = ((ctn_len as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_ARR as u64;
                    if ctn == ctn_parent {
                        state_lbl = IState::DocEnd;
                        continue;
                    }
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    state_lbl = if ((*ctn).tag & YYJSON_TYPE_MASK as u64)
                        == YYJSON_TYPE_OBJ as u64
                    {
                        IState::ObjValEnd
                    } else {
                        IState::ArrValEnd
                    };
                }

                IState::ObjBegin => {
                    /* push container */
                    (*ctn).tag = (((ctn_len as u64) + 1) << YYJSON_TAG_BIT)
                        | ((*ctn).tag & YYJSON_TAG_MASK as u64);
                    val_incr!();
                    (*val).tag = YYJSON_TYPE_OBJ as u64;
                    (*val).uni.ofs = val as usize - ctn as usize;
                    ctn = val;
                    ctn_len = 0;
                    state_lbl = IState::ObjKeyBegin;
                }

                IState::ObjKeyBegin => {
                    save_incr_state!(LABEL_OBJ_KEY_BEGIN);
                    state_lbl = IState::ObjKeyContinue;
                }

                IState::ObjKeyContinue => {
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if read_str_con(&mut cur, end, flg, val, &mut msg, con) {
                            state_lbl = IState::ObjKeyEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(cur, YYJSON_READ_ERROR_INVALID_STRING, msg);
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            state_lbl = IState::ObjEnd;
                            continue;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_JSON_STRUCTURE,
                            crate::util::msg(MSG_COMMA),
                        );
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    break 'fsm IOut::Fail(
                        cur,
                        YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                        crate::util::msg(MSG_OBJ_KEY),
                    );
                }

                IState::ObjKeyEnd => {
                    save_incr_state!(LABEL_OBJ_KEY_END);
                    if *cur == b':' {
                        cur = cur.add(1);
                        state_lbl = IState::ObjValBegin;
                        continue;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    break 'fsm IOut::Fail(
                        cur,
                        YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                        crate::util::msg(MSG_OBJ_SEP),
                    );
                }

                IState::ObjValBegin => {
                    save_incr_state!(LABEL_OBJ_VAL_BEGIN);
                    state_lbl = IState::ObjValContinue;
                }

                IState::ObjValContinue => {
                    if *cur == b'"' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_str_con(&mut cur, end, flg, val, &mut msg, con) {
                            state_lbl = IState::ObjValEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(cur, YYJSON_READ_ERROR_INVALID_STRING, msg);
                    }
                    if char_is_num(*cur) {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_num(&mut cur, &mut raw_ptr, flg, val, &mut msg) {
                            state_lbl = IState::ObjValMaybeEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(cur, YYJSON_READ_ERROR_INVALID_NUMBER, msg);
                    }
                    if *cur == b'{' {
                        cur = cur.add(1);
                        state_lbl = IState::ObjBegin;
                        continue;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        state_lbl = IState::ArrBegin;
                        continue;
                    }
                    if *cur == b't' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_true(&mut cur, val) {
                            state_lbl = IState::ObjValEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_T),
                        );
                    }
                    if *cur == b'f' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_false(&mut cur, val) {
                            state_lbl = IState::ObjValEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_F),
                        );
                    }
                    if *cur == b'n' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_null(&mut cur, val) {
                            state_lbl = IState::ObjValEnd;
                            continue;
                        }
                        break 'fsm IOut::Fail(
                            cur,
                            YYJSON_READ_ERROR_LITERAL,
                            crate::util::msg(MSG_CHAR_N),
                        );
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    break 'fsm IOut::Fail(
                        cur,
                        YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                        crate::util::msg(MSG_CHAR),
                    );
                }

                IState::ObjValMaybeEnd => {
                    check_maybe_truncated_number!();
                    state_lbl = IState::ObjValEnd;
                }

                IState::ObjValEnd => {
                    save_incr_state!(LABEL_OBJ_VAL_END);
                    if *cur == b',' {
                        cur = cur.add(1);
                        state_lbl = IState::ObjKeyBegin;
                        continue;
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        state_lbl = IState::ObjEnd;
                        continue;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    break 'fsm IOut::Fail(
                        cur,
                        YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                        crate::util::msg(MSG_OBJ_END),
                    );
                }

                IState::ObjEnd => {
                    let ctn_parent = (ctn as *mut u8).sub((*ctn).uni.ofs) as *mut yyjson_val;
                    (*ctn).uni.ofs = (val as usize - ctn as usize) + size_of::<yyjson_val>();
                    (*ctn).tag =
                        ((ctn_len as u64) << (YYJSON_TAG_BIT - 1)) | YYJSON_TYPE_OBJ as u64;
                    if ctn == ctn_parent {
                        state_lbl = IState::DocEnd;
                        continue;
                    }
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    state_lbl = if ((*ctn).tag & YYJSON_TYPE_MASK as u64)
                        == YYJSON_TYPE_OBJ as u64
                    {
                        IState::ObjValEnd
                    } else {
                        IState::ArrValEnd
                    };
                }

                IState::DocEnd => {
                    /* check invalid contents after json document */
                    if (cur < end || len < (*state).buf_len)
                        && !has_flg(flg, YYJSON_READ_STOP_WHEN_DONE)
                    {
                        save_incr_state!(LABEL_DOC_END);
                        while char_is_space(*cur) {
                            cur = cur.add(1);
                        }
                        if cur < end {
                            break 'fsm IOut::Fail(
                                cur,
                                YYJSON_READ_ERROR_UNEXPECTED_CONTENT,
                                crate::util::msg(MSG_GARBAGE),
                            );
                        }
                        /* the document is complete for the bytes seen so far,
                           but more input is still pending; it may hold
                           trailing content that has to be rejected */
                        if len < (*state).buf_len {
                            break 'fsm IOut::NeedMore;
                        }
                    }

                    *raw_ptr = 0;
                    let doc = val_hdr as *mut yyjson_doc;
                    (*doc).root = val_hdr.add(hdr_len);
                    (*doc).alc = alc;
                    (*doc).dat_read = cur as usize - hdr as usize;
                    (*doc).val_read =
                        ((val as usize - (*doc).root as usize) / size_of::<yyjson_val>()) + 1;
                    (*doc).str_pool = if has_flg(flg, YYJSON_READ_INSITU) {
                        ptr::null_mut()
                    } else {
                        hdr as *mut c_char
                    };
                    (*state).hdr = ptr::null_mut();
                    (*state).val_hdr = ptr::null_mut();
                    break 'fsm IOut::Doc(doc);
                }
            }
        }
    };

    /* `unexpected_end` */
    macro_rules! unexpected_end {
        () => {{
            (*err).pos = len;
            if len >= (*state).buf_len {
                (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                (*err).msg = crate::util::msg(MSG_NOT_END);
                return ptr::null_mut();
            }
            (*err).code = YYJSON_READ_ERROR_MORE;
            (*err).msg = b"need more data\0".as_ptr() as *const c_char;
            (*state).val_end = val_end;
            (*state).ctn = ctn;
            (*state).alc_len = alc_len;
            /* restore the end where we've inserted a null terminator */
            *end = saved_end;
            return ptr::null_mut();
        }};
    }

    match out {
        IOut::Doc(doc) => {
            mem::set(err as *mut u8, 0, size_of::<yyjson_read_err>());
            doc
        }
        IOut::NeedMore => unexpected_end!(),
        IOut::Fail(pos, code, message) => {
            if is_truncated_end(hdr, pos, end, code, flg) {
                unexpected_end!()
            } else {
                (*err).pos = pos as usize - hdr as usize;
                (*err).code = code;
                (*err).msg = message;
                ptr::null_mut()
            }
        }
    }
}
