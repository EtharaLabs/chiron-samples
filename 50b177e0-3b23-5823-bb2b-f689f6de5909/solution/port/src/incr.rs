// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 * MARK: - incr.rs
 *
 * Owns yyjson.c 6547-7189 (incremental JSON reader).
 *
 * `struct yyjson_incr_state` is opaque in yyjson.h, so its real layout lives
 * here. The definition below is copied field-for-field from yyjson.c:6563 so
 * that `sizeof(*state)` - the size handed to `alc.malloc` in
 * `yyjson_incr_new` - is already correct.
 *
 * `YYJSON_READER_DEPTH_LIMIT` is 0 in this build, so the conditional
 * `ctn_depth` member, the `save_incr_depth()` macro body, the depth checks in
 * `arr_begin` / `obj_begin` and the `fail_depth` label are absent, exactly as
 * in the C translation unit.
 *
 * The `goto` state machine of `yyjson_incr_read` becomes an explicit
 * state-constant dispatch loop. `unexpected_end`, `return_err`,
 * `return_err_inv_param`, `save_incr_state` and
 * `check_maybe_truncated_number` all capture caller locals, so they are
 * function-local `macro_rules!`, exactly as the C `#define`s are scoped to
 * the function body by the trailing `#undef`s.
 *============================================================================*/

use crate::ffi::{c_char, c_void};
use crate::read_num::read_num;
use crate::reader::{
    is_truncated_end, read_false, read_null, read_str_con, read_true,
};
use crate::types::{
    YYJSON_PADDING_SIZE, YYJSON_READ_ALLOW_BOM,
    YYJSON_READ_ALLOW_INVALID_UNICODE, YYJSON_READ_ERROR_INVALID_NUMBER,
    YYJSON_READ_ERROR_INVALID_PARAMETER, YYJSON_READ_ERROR_INVALID_STRING,
    YYJSON_READ_ERROR_JSON_STRUCTURE, YYJSON_READ_ERROR_LITERAL,
    YYJSON_READ_ERROR_MEMORY_ALLOCATION, YYJSON_READ_ERROR_MORE,
    YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
    YYJSON_READ_ERROR_UNEXPECTED_CONTENT, YYJSON_READ_ERROR_UNEXPECTED_END,
    YYJSON_READ_INSITU, YYJSON_READ_JSON5, YYJSON_READ_STOP_WHEN_DONE,
    YYJSON_TAG_BIT, YYJSON_TAG_MASK, YYJSON_TYPE_ARR, YYJSON_TYPE_MASK,
    YYJSON_TYPE_OBJ, yyjson_alc, yyjson_doc, yyjson_read_err,
    yyjson_read_flag, yyjson_val,
};
use crate::util::{
    INCR_NUM_MAX_LEN, MSG_ARR_END, MSG_CHAR, MSG_CHAR_F, MSG_CHAR_N,
    MSG_CHAR_T, MSG_COMMA, MSG_ERR_BOM, MSG_ERR_UTF16, MSG_ERR_UTF32,
    MSG_GARBAGE, MSG_MALLOC, MSG_NOT_END, MSG_OBJ_END, MSG_OBJ_KEY,
    MSG_OBJ_SEP, USIZE_MAX, YYJSON_READER_ESTIMATED_MINIFY_RATIO, char_is_ctn,
    char_is_num, char_is_space, has_rflag, is_utf16_bom, is_utf32_bom,
    is_utf8_bom, yyjson_min,
};

/*==============================================================================
 * MARK: - Resume labels (yyjson.c:6552-6560)
 *============================================================================*/

pub(crate) const LABEL_doc_begin: u32 = 0;
pub(crate) const LABEL_arr_val_begin: u32 = 1;
pub(crate) const LABEL_arr_val_end: u32 = 2;
pub(crate) const LABEL_obj_key_begin: u32 = 3;
pub(crate) const LABEL_obj_key_end: u32 = 4;
pub(crate) const LABEL_obj_val_begin: u32 = 5;
pub(crate) const LABEL_obj_val_end: u32 = 6;
pub(crate) const LABEL_doc_end: u32 = 7;

/*==============================================================================
 * MARK: - State for incremental JSON reader (yyjson.c:6563)
 *============================================================================*/

/** State for incremental JSON reader, opaque in the API. */
#[repr(C)]
pub struct yyjson_incr_state {
    /** current parser goto label */
    pub label: u32,
    /** allocator */
    pub alc: yyjson_alc,
    /** read flags */
    pub flg: yyjson_read_flag,
    /** JSON data header */
    pub hdr: *mut u8,
    /** current position in JSON data */
    pub cur: *mut u8,
    /** total buffer length (without padding) */
    pub buf_len: usize,
    /** value count used by yyjson_doc */
    pub hdr_len: usize,
    /** value count allocated */
    pub alc_len: usize,
    /** the number of elements in current container */
    pub ctn_len: usize,
    /* `usize ctn_depth;` only when YYJSON_READER_DEPTH_LIMIT != 0 */
    /** the head of allocated values */
    pub val_hdr: *mut yyjson_val,
    /** the end of allocated values */
    pub val_end: *mut yyjson_val,
    /** current JSON value */
    pub val: *mut yyjson_val,
    /** current container */
    pub ctn: *mut yyjson_val,
    /** string parser incremental state */
    pub str_con: [*mut u8; 2],
    /** pending position for a deferred raw null-terminator */
    pub raw_ptr: *mut u8,
    /** dummy target for the first deferred null-terminator */
    pub raw_end: [u8; 1],
}

/* The size reaches `alc.malloc`, so it is pinned here and checked against the
   C `sizeof(struct yyjson_incr_state)` by the foundation smoke test. */
const _: () = assert!(core::mem::size_of::<yyjson_incr_state>() == 160);
const _: () = assert!(core::mem::align_of::<yyjson_incr_state>() == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, label) == 0);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, alc) == 8);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, flg) == 40);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, hdr) == 48);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, cur) == 56);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, buf_len) == 64);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, hdr_len) == 72);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, alc_len) == 80);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, ctn_len) == 88);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, val_hdr) == 96);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, val_end) == 104);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, val) == 112);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, ctn) == 120);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, str_con) == 128);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, raw_ptr) == 144);
const _: () = assert!(core::mem::offset_of!(yyjson_incr_state, raw_end) == 152);

/*==============================================================================
 * MARK: - States of the `yyjson_incr_read` goto machine (yyjson.c:6780-7126)
 *============================================================================*/

const I_DOC_BEGIN: u32 = 0;
const I_ARR_BEGIN: u32 = 1;
const I_ARR_VAL_BEGIN: u32 = 2;
const I_ARR_VAL_CONTINUE: u32 = 3;
const I_ARR_VAL_MAYBE_END: u32 = 4;
const I_ARR_VAL_END: u32 = 5;
const I_ARR_END: u32 = 6;
const I_OBJ_BEGIN: u32 = 7;
const I_OBJ_KEY_BEGIN: u32 = 8;
const I_OBJ_KEY_CONTINUE: u32 = 9;
const I_OBJ_KEY_END: u32 = 10;
const I_OBJ_VAL_BEGIN: u32 = 11;
const I_OBJ_VAL_CONTINUE: u32 = 12;
const I_OBJ_VAL_MAYBE_END: u32 = 13;
const I_OBJ_VAL_END: u32 = 14;
const I_OBJ_END: u32 = 15;
const I_DOC_END: u32 = 16;

/*==============================================================================
 * MARK: - Exported functions
 *============================================================================*/

/// `yyjson_incr_state *yyjson_incr_new(char *buf, size_t buf_len,
///                                     yyjson_read_flag flg,
///                                     const yyjson_alc *alc)`
/// yyjson.c:6585
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_incr_new(buf: *mut c_char, buf_len: usize,
                                         mut flg: yyjson_read_flag,
                                         alc_ptr: *const yyjson_alc)
    -> *mut yyjson_incr_state {
    unsafe {
        let state: *mut yyjson_incr_state;
        let alc: yyjson_alc = if !alc_ptr.is_null() {
            *alc_ptr
        } else {
            crate::alloc::YYJSON_DEFAULT_ALC
        };

        /* remove non-standard flags */
        flg &= !YYJSON_READ_JSON5;
        flg &= !YYJSON_READ_ALLOW_BOM;
        flg &= !YYJSON_READ_ALLOW_INVALID_UNICODE;

        if buf.is_null() {
            return core::ptr::null_mut();
        }
        if buf_len >= USIZE_MAX - YYJSON_PADDING_SIZE {
            return core::ptr::null_mut();
        }
        state = (alc.malloc.unwrap_unchecked())(
            alc.ctx, core::mem::size_of::<yyjson_incr_state>())
            as *mut yyjson_incr_state;
        if state.is_null() {
            return core::ptr::null_mut();
        }
        core::ptr::write_bytes(state as *mut u8, 0,
                               core::mem::size_of::<yyjson_incr_state>());
        (*state).alc = alc;
        (*state).flg = flg;
        (*state).buf_len = buf_len;

        /* add 4-byte zero padding for input data if necessary */
        /* has_flg(INSITU) */
        if has_rflag(flg, YYJSON_READ_INSITU, false) {
            (*state).hdr = buf as *mut u8;
        } else {
            (*state).hdr = (alc.malloc.unwrap_unchecked())(
                alc.ctx, buf_len.wrapping_add(YYJSON_PADDING_SIZE))
                as *mut u8;
            if (*state).hdr.is_null() {
                (alc.free.unwrap_unchecked())(alc.ctx, state as *mut c_void);
                return core::ptr::null_mut();
            }
            core::ptr::copy_nonoverlapping(buf as *const u8, (*state).hdr,
                                           buf_len);
        }
        core::ptr::write_bytes((*state).hdr.add(buf_len), 0,
                               YYJSON_PADDING_SIZE);
        (*state).cur = (*state).hdr;
        (*state).raw_ptr = (*state).raw_end.as_mut_ptr();
        (*state).label = LABEL_doc_begin;
        state
    }
}

/// `void yyjson_incr_free(yyjson_incr_state *state)`
/// yyjson.c:6623
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_incr_free(state: *mut yyjson_incr_state) {
    unsafe {
        if !state.is_null() {
            let alc: yyjson_alc = (*state).alc;
            core::ptr::write_bytes(&mut (*state).alc as *mut yyjson_alc
                                       as *mut u8,
                                   0, core::mem::size_of::<yyjson_alc>());
            if !(*state).val_hdr.is_null() {
                (alc.free.unwrap_unchecked())(alc.ctx,
                                              (*state).val_hdr
                                                  as *mut c_void);
            }
            if !(*state).hdr.is_null()
                && ((*state).flg & YYJSON_READ_INSITU) == 0
            {
                (alc.free.unwrap_unchecked())(alc.ctx,
                                              (*state).hdr as *mut c_void);
            }
            (alc.free.unwrap_unchecked())(alc.ctx, state as *mut c_void);
        }
    }
}

/// `yyjson_doc *yyjson_incr_read(yyjson_incr_state *state, size_t len,
///                               yyjson_read_err *err)`
/// yyjson.c:6637
#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn yyjson_incr_read(state: *mut yyjson_incr_state,
                                          len: usize,
                                          mut err: *mut yyjson_read_err)
    -> *mut yyjson_doc {
    unsafe {
        let mut hdr: *mut u8 = core::ptr::null_mut();
        let mut end: *mut u8 = core::ptr::null_mut();
        let mut cur: *mut u8 = core::ptr::null_mut();
        let flg: yyjson_read_flag;
        let alc: yyjson_alc;
        let mut hdr_len: usize; /* value count used by yyjson_doc */
        let mut alc_len: usize; /* value count allocated */
        let alc_max: usize; /* maximum value count for allocator */
        let mut ctn_len: usize; /* elements in current container */
        let mut val_hdr: *mut yyjson_val; /* head of allocated values */
        let mut val_end: *mut yyjson_val; /* end of allocated values */
        let mut val_tmp: *mut yyjson_val; /* temporary pointer for realloc */
        let mut val: *mut yyjson_val; /* current JSON value */
        let mut ctn: *mut yyjson_val; /* current container */
        let mut ctn_parent: *mut yyjson_val; /* parent of current container */
        let doc: *mut yyjson_doc; /* the JSON document, equals to val_hdr */
        let mut msg: *const c_char = core::ptr::null(); /* error message */

        let mut tmp_err: yyjson_read_err = yyjson_read_err {
            code: 0,
            msg: core::ptr::null(),
            pos: 0,
        };
        /* deferred raw null-terminator position, committed at save.
           Every access goes through `pre`, which is `&raw mut raw_ptr`, so
           that `read_num`'s writes through `pre` and the reads in
           `save_incr_state` cannot be reordered around each other. */
        let mut raw_ptr: *mut u8 = core::ptr::null_mut();
        let pre: *mut *mut u8 = &raw mut raw_ptr;
        let mut con: *mut *mut u8 = core::ptr::null_mut();
        let mut saved_end: u8 = b'\0'; /* saved end char */

        /* yyjson.c:6639  return_err_inv_param(_msg) */
        macro_rules! return_err_inv_param {
            ($m:expr) => {{
                (*err).pos = 0;
                (*err).msg = $m;
                (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
                return core::ptr::null_mut();
            }};
        }

        /* validate input parameters */
        if err.is_null() {
            err = &mut tmp_err;
        }
        if state.is_null() {
            return_err_inv_param!(
                c"input state is NULL".as_ptr() as *const c_char);
        }
        if len == 0 {
            return_err_inv_param!(
                c"input length is 0".as_ptr() as *const c_char);
        }
        if len > (*state).buf_len {
            return_err_inv_param!(
                c"length is greater than total input length".as_ptr()
                    as *const c_char);
        }

        /* restore state saved from the previous call */
        hdr = (*state).hdr;
        end = (*state).hdr.add(len);
        cur = (*state).cur;
        flg = (*state).flg;
        alc = (*state).alc;
        ctn_len = (*state).ctn_len;
        hdr_len = (*state).hdr_len;
        alc_len = (*state).alc_len;
        val = (*state).val;
        val_hdr = (*state).val_hdr;
        val_end = (*state).val_end;
        ctn = (*state).ctn;
        con = (*state).str_con.as_mut_ptr();
        *pre = (*state).raw_ptr;
        alc_max = USIZE_MAX / core::mem::size_of::<yyjson_val>();

        /* insert null terminator to make us stop at the specified end, even
           if the data contains more valid JSON */
        saved_end = *end;
        *end = b'\0';

        /* yyjson.c:7140  unexpected_end: */
        macro_rules! goto_unexpected_end {
            () => {{
                (*err).pos = len;
                /* if no more data, stop the incr read */
                if len >= (*state).buf_len {
                    (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
                    (*err).msg = MSG_NOT_END;
                    return core::ptr::null_mut();
                }
                /* save parser state in extended error struct, in addition to
                   what was stored in the last save_incr_state */
                (*err).code = YYJSON_READ_ERROR_MORE;
                (*err).msg = c"need more data".as_ptr() as *const c_char;
                (*state).val_end = val_end;
                (*state).ctn = ctn;
                (*state).alc_len = alc_len;
                /* restore the end where we've inserted a null terminator */
                *end = saved_end;
                return core::ptr::null_mut();
            }};
        }

        /* yyjson.c:6646  return_err(_pos, _code, _msg) */
        macro_rules! return_err {
            ($pos:expr, $code:expr, $m:expr) => {{
                let p_: *mut u8 = $pos;
                if is_truncated_end(hdr, p_, end, $code, flg) {
                    goto_unexpected_end!();
                } else {
                    (*err).pos = p_.offset_from(hdr) as usize;
                    (*err).code = $code;
                    (*err).msg = $m;
                }
                return core::ptr::null_mut();
            }};
        }

        /* yyjson.c:6657  val_incr() */
        macro_rules! val_incr {
            () => {{
                val = val.add(1);
                if val >= val_end {
                    let alc_old: usize = alc_len;
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
                    val = val_tmp.add(val.offset_from(val_hdr) as usize);
                    ctn = val_tmp.add(ctn.offset_from(val_hdr) as usize);
                    (*state).val = val_tmp
                        .add((*state).val.offset_from(val_hdr) as usize);
                    val_hdr = val_tmp;
                    (*state).val_hdr = val_hdr;
                    val_end = val_tmp.add(alc_len - 2);
                    (*state).val_end = val_end;
                }
            }};
        }

        /* yyjson.c:6682  save_incr_state(_label)
           `save_incr_depth()` is a no-op because YYJSON_READER_DEPTH_LIMIT
           is 0. */
        macro_rules! save_incr_state {
            ($label:expr) => {{
                (*state).label = $label;
                (*state).cur = cur;
                (*state).val = val;
                (*state).ctn_len = ctn_len;
                (*state).hdr_len = hdr_len;
                (*state).raw_ptr = *pre;
                if cur >= end {
                    goto_unexpected_end!();
                }
            }};
        }

        /* yyjson.c:6693  check_maybe_truncated_number() */
        macro_rules! check_maybe_truncated_number {
            () => {{
                if cur >= end {
                    if cur > (*state).cur.add(INCR_NUM_MAX_LEN) {
                        msg = c"number too long".as_ptr() as *const c_char;
                        /* fail_number */
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER,
                                    msg);
                    }
                    goto_unexpected_end!();
                }
            }};
        }

        /* resume parsing from the last save point */
        let mut lbl: u32 = match (*state).label {
            LABEL_doc_begin => I_DOC_BEGIN,
            LABEL_arr_val_begin => I_ARR_VAL_BEGIN,
            LABEL_arr_val_end => I_ARR_VAL_END,
            LABEL_obj_key_begin => I_OBJ_KEY_BEGIN,
            LABEL_obj_key_end => I_OBJ_KEY_END,
            LABEL_obj_val_begin => I_OBJ_VAL_BEGIN,
            LABEL_obj_val_end => I_OBJ_VAL_END,
            LABEL_doc_end => I_DOC_END,
            _ => {
                return_err_inv_param!(
                    c"invalid incremental state".as_ptr() as *const c_char);
            }
        };

        'fsm: loop {
            match lbl {
                I_DOC_BEGIN => {
                    /* skip empty contents before json document */
                    if !char_is_ctn(*cur) {
                        while char_is_space(*cur) {
                            cur = cur.add(1);
                        }
                        if cur >= end {
                            /* input data is empty */
                            goto_unexpected_end!();
                        }
                    }

                    /* allocate memory for document */
                    if val_hdr.is_null() {
                        hdr_len = core::mem::size_of::<yyjson_doc>()
                            / core::mem::size_of::<yyjson_val>()
                            + ((core::mem::size_of::<yyjson_doc>()
                                % core::mem::size_of::<yyjson_val>()) > 0)
                                as usize;
                        if char_is_ctn(*cur) {
                            /* data length in bytes, hint for allocator */
                            /* has_flg(STOP_WHEN_DONE) */
                            let dat_len: usize = if has_rflag(
                                flg, YYJSON_READ_STOP_WHEN_DONE, false) {
                                256
                            } else {
                                (*state).buf_len
                            };
                            alc_len = hdr_len
                                .wrapping_add(dat_len
                                    / YYJSON_READER_ESTIMATED_MINIFY_RATIO)
                                .wrapping_add(4);
                            alc_len = yyjson_min(alc_len, alc_max);
                        } else {
                            alc_len = hdr_len + 1; /* single value */
                        }
                        val_hdr = (alc.malloc.unwrap_unchecked())(
                            alc.ctx,
                            alc_len.wrapping_mul(
                                core::mem::size_of::<yyjson_val>()))
                            as *mut yyjson_val;
                        if val_hdr.is_null() {
                            /* fail_alloc */
                            return_err!(cur,
                                        YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                                        MSG_MALLOC);
                        }
                        /* padding for kv pair reading */
                        val_end = val_hdr.add(alc_len - 2);
                        val = val_hdr.add(hdr_len);
                        ctn = val;
                        ctn_len = 0;
                        (*state).val_hdr = val_hdr;
                        (*state).val_end = val_end;
                        save_incr_state!(LABEL_doc_begin);
                    }

                    /* read json document */
                    if *cur == b'{' {
                        cur = cur.add(1);
                        (*ctn).tag = YYJSON_TYPE_OBJ as u64;
                        (*ctn).uni.ofs = 0;
                        lbl = I_OBJ_KEY_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        (*ctn).tag = YYJSON_TYPE_ARR as u64;
                        (*ctn).uni.ofs = 0;
                        lbl = I_ARR_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if char_is_num(*cur) {
                        if read_num(&mut cur, pre, flg, val, &mut msg) {
                            /* a root number may continue with more digits in
                               a later chunk */
                            if len < (*state).buf_len {
                                check_maybe_truncated_number!();
                            }
                            lbl = I_DOC_END;
                            continue 'fsm;
                        }
                        /* fail_number */
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER,
                                    msg);
                    }
                    if *cur == b'"' {
                        if read_str_con(&mut cur, end, flg, val, &mut msg,
                                        con) {
                            lbl = I_DOC_END;
                            continue 'fsm;
                        }
                        /* fail_string */
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if *cur == b't' {
                        if read_true(&mut cur, val) {
                            lbl = I_DOC_END;
                            continue 'fsm;
                        }
                        /* fail_literal_true */
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_T);
                    }
                    if *cur == b'f' {
                        if read_false(&mut cur, val) {
                            lbl = I_DOC_END;
                            continue 'fsm;
                        }
                        /* fail_literal_false */
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_F);
                    }
                    if *cur == b'n' {
                        if read_null(&mut cur, val) {
                            lbl = I_DOC_END;
                            continue 'fsm;
                        }
                        /* fail_literal_null */
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_N);
                    }

                    msg = c"unexpected character, expected a valid root value"
                        .as_ptr() as *const c_char;
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
                    return_err!(cur,
                                YYJSON_READ_ERROR_UNEXPECTED_CHARACTER, msg);
                }

                I_ARR_BEGIN => {
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
                    lbl = I_ARR_VAL_BEGIN;
                    continue 'fsm;
                }

                I_ARR_VAL_BEGIN => {
                    save_incr_state!(LABEL_arr_val_begin);
                    lbl = I_ARR_VAL_CONTINUE;
                    continue 'fsm;
                }

                I_ARR_VAL_CONTINUE => {
                    if *cur == b'{' {
                        cur = cur.add(1);
                        lbl = I_OBJ_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        lbl = I_ARR_BEGIN;
                        continue 'fsm;
                    }
                    if char_is_num(*cur) {
                        val_incr!();
                        ctn_len += 1;
                        if read_num(&mut cur, pre, flg, val, &mut msg) {
                            lbl = I_ARR_VAL_MAYBE_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER,
                                    msg);
                    }
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if read_str_con(&mut cur, end, flg, val, &mut msg,
                                        con) {
                            lbl = I_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if *cur == b't' {
                        val_incr!();
                        ctn_len += 1;
                        if read_true(&mut cur, val) {
                            lbl = I_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_T);
                    }
                    if *cur == b'f' {
                        val_incr!();
                        ctn_len += 1;
                        if read_false(&mut cur, val) {
                            lbl = I_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_F);
                    }
                    if *cur == b'n' {
                        val_incr!();
                        ctn_len += 1;
                        if read_null(&mut cur, val) {
                            lbl = I_ARR_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_N);
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            lbl = I_ARR_END;
                            continue 'fsm;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        /* fail_trailing_comma */
                        return_err!(cur, YYJSON_READ_ERROR_JSON_STRUCTURE,
                                    MSG_COMMA);
                    }
                    if char_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !char_is_space(*cur) {
                                break;
                            }
                        }
                        lbl = I_ARR_VAL_CONTINUE;
                        continue 'fsm;
                    }
                    /* fail_character_val */
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_CHAR);
                }

                I_ARR_VAL_MAYBE_END => {
                    /* if incremental parsing stops in the middle of a
                       number, it may continue with more digits, so arr val
                       maybe didn't end yet */
                    check_maybe_truncated_number!();
                    lbl = I_ARR_VAL_END;
                    continue 'fsm;
                }

                I_ARR_VAL_END => {
                    save_incr_state!(LABEL_arr_val_end);
                    if *cur == b',' {
                        cur = cur.add(1);
                        lbl = I_ARR_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        lbl = I_ARR_END;
                        continue 'fsm;
                    }
                    if char_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !char_is_space(*cur) {
                                break;
                            }
                        }
                        lbl = I_ARR_VAL_END;
                        continue 'fsm;
                    }
                    /* fail_character_arr_end */
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_ARR_END);
                }

                I_ARR_END => {
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
                        lbl = I_DOC_END;
                        continue 'fsm;
                    }

                    /* pop parent as current container */
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    if ((*ctn).tag & YYJSON_TYPE_MASK as u64)
                        == YYJSON_TYPE_OBJ as u64
                    {
                        lbl = I_OBJ_VAL_END;
                    } else {
                        lbl = I_ARR_VAL_END;
                    }
                    continue 'fsm;
                }

                I_OBJ_BEGIN => {
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
                    lbl = I_OBJ_KEY_BEGIN;
                    continue 'fsm;
                }

                I_OBJ_KEY_BEGIN => {
                    save_incr_state!(LABEL_obj_key_begin);
                    lbl = I_OBJ_KEY_CONTINUE;
                    continue 'fsm;
                }

                I_OBJ_KEY_CONTINUE => {
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if read_str_con(&mut cur, end, flg, val, &mut msg,
                                        con) {
                            lbl = I_OBJ_KEY_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            lbl = I_OBJ_END;
                            continue 'fsm;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        /* fail_trailing_comma */
                        return_err!(cur, YYJSON_READ_ERROR_JSON_STRUCTURE,
                                    MSG_COMMA);
                    }
                    if char_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !char_is_space(*cur) {
                                break;
                            }
                        }
                        lbl = I_OBJ_KEY_CONTINUE;
                        continue 'fsm;
                    }
                    /* fail_character_obj_key */
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_KEY);
                }

                I_OBJ_KEY_END => {
                    save_incr_state!(LABEL_obj_key_end);
                    if *cur == b':' {
                        cur = cur.add(1);
                        lbl = I_OBJ_VAL_BEGIN;
                        continue 'fsm;
                    }
                    if char_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !char_is_space(*cur) {
                                break;
                            }
                        }
                        lbl = I_OBJ_KEY_END;
                        continue 'fsm;
                    }
                    /* fail_character_obj_sep */
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_SEP);
                }

                I_OBJ_VAL_BEGIN => {
                    save_incr_state!(LABEL_obj_val_begin);
                    lbl = I_OBJ_VAL_CONTINUE;
                    continue 'fsm;
                }

                I_OBJ_VAL_CONTINUE => {
                    if *cur == b'"' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_str_con(&mut cur, end, flg, val, &mut msg,
                                        con) {
                            lbl = I_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_STRING,
                                    msg);
                    }
                    if char_is_num(*cur) {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_num(&mut cur, pre, flg, val, &mut msg) {
                            lbl = I_OBJ_VAL_MAYBE_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER,
                                    msg);
                    }
                    if *cur == b'{' {
                        cur = cur.add(1);
                        lbl = I_OBJ_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        lbl = I_ARR_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b't' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_true(&mut cur, val) {
                            lbl = I_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_T);
                    }
                    if *cur == b'f' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_false(&mut cur, val) {
                            lbl = I_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_F);
                    }
                    if *cur == b'n' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_null(&mut cur, val) {
                            lbl = I_OBJ_VAL_END;
                            continue 'fsm;
                        }
                        return_err!(cur, YYJSON_READ_ERROR_LITERAL,
                                    MSG_CHAR_N);
                    }
                    if char_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !char_is_space(*cur) {
                                break;
                            }
                        }
                        lbl = I_OBJ_VAL_CONTINUE;
                        continue 'fsm;
                    }
                    /* fail_character_val */
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_CHAR);
                }

                I_OBJ_VAL_MAYBE_END => {
                    /* if incremental parsing stops in the middle of a
                       number, it may continue with more digits, so obj val
                       maybe didn't end yet */
                    check_maybe_truncated_number!();
                    lbl = I_OBJ_VAL_END;
                    continue 'fsm;
                }

                I_OBJ_VAL_END => {
                    save_incr_state!(LABEL_obj_val_end);
                    if *cur == b',' {
                        cur = cur.add(1);
                        lbl = I_OBJ_KEY_BEGIN;
                        continue 'fsm;
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        lbl = I_OBJ_END;
                        continue 'fsm;
                    }
                    if char_is_space(*cur) {
                        loop {
                            cur = cur.add(1);
                            if !char_is_space(*cur) {
                                break;
                            }
                        }
                        lbl = I_OBJ_VAL_END;
                        continue 'fsm;
                    }
                    /* fail_character_obj_end */
                    return_err!(cur, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
                                MSG_OBJ_END);
                }

                I_OBJ_END => {
                    /* pop container */
                    ctn_parent = ((ctn as *mut u8).sub((*ctn).uni.ofs))
                        as *mut yyjson_val;
                    /* point to the next value */
                    (*ctn).uni.ofs =
                        ((val as *mut u8).offset_from(ctn as *mut u8) as usize)
                            .wrapping_add(core::mem::size_of::<yyjson_val>());
                    (*ctn).tag = ((ctn_len as u64) << (YYJSON_TAG_BIT - 1))
                        | YYJSON_TYPE_OBJ as u64;
                    if ctn == ctn_parent {
                        lbl = I_DOC_END;
                        continue 'fsm;
                    }
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    if ((*ctn).tag & YYJSON_TYPE_MASK as u64)
                        == YYJSON_TYPE_OBJ as u64
                    {
                        lbl = I_OBJ_VAL_END;
                    } else {
                        lbl = I_ARR_VAL_END;
                    }
                    continue 'fsm;
                }

                _ => {
                    /* doc_end: check invalid contents after json document */
                    /* has_flg(STOP_WHEN_DONE) */
                    if (cur < end || len < (*state).buf_len)
                        && !has_rflag(flg, YYJSON_READ_STOP_WHEN_DONE, false)
                    {
                        save_incr_state!(LABEL_doc_end);
                        while char_is_space(*cur) {
                            cur = cur.add(1);
                        }
                        if cur < end {
                            /* fail_garbage */
                            return_err!(
                                cur,
                                YYJSON_READ_ERROR_UNEXPECTED_CONTENT,
                                MSG_GARBAGE);
                        }
                        /* the document is complete for the bytes seen so
                           far, but more input is still pending; it may hold
                           trailing content that has to be rejected, so
                           request the remaining data before finalizing */
                        if len < (*state).buf_len {
                            goto_unexpected_end!();
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
                    (*doc).str_pool =
                        if has_rflag(flg, YYJSON_READ_INSITU, false) {
                            core::ptr::null_mut()
                        } else {
                            hdr as *mut c_char
                        };
                    (*state).hdr = core::ptr::null_mut();
                    (*state).val_hdr = core::ptr::null_mut();
                    core::ptr::write_bytes(err, 0, 1);
                    let _ = val_tmp;
                    return doc;
                }
            }
        }
    }
}
