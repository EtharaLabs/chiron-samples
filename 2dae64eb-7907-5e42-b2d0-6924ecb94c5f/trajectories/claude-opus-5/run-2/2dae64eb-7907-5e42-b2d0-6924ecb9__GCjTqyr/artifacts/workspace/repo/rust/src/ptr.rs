//! JSON Pointer (RFC 6901), JSON Patch (RFC 6902) and JSON Merge Patch
//! (RFC 7386).
//!
//! Transliteration of the corresponding sections of the original
//! `src/yyjson.c`.

use core::ffi::{c_char, c_void};
use core::ptr;

use crate::hdr;
use crate::libc;
use crate::types::*;
use crate::util::*;

/* ===== pointer tokens =================================================== */

/// `ptr_next_token`
#[inline]
unsafe fn ptr_next_token(
    p: *mut *const c_char,
    end: *const c_char,
    len: *mut usize,
    esc: *mut usize,
) -> *const c_char {
    let hdr = (*p).add(1);
    let mut cur = hdr;
    /* skip unescaped characters */
    while cur < end && *cur != b'/' as c_char && *cur != b'~' as c_char {
        cur = cur.add(1);
    }
    if cur == end || *cur != b'~' as c_char {
        *p = cur;
        *len = cur.offset_from(hdr) as usize;
        *esc = 0;
        hdr
    } else {
        let mut esc_num = 0usize;
        while cur < end && *cur != b'/' as c_char {
            let c = *cur;
            cur = cur.add(1);
            if c == b'~' as c_char {
                if cur == end || (*cur != b'0' as c_char && *cur != b'1' as c_char) {
                    *p = cur.sub(1);
                    return ptr::null();
                }
                esc_num += 1;
            }
        }
        *p = cur;
        *len = (cur.offset_from(hdr) as usize) - esc_num;
        *esc = esc_num;
        hdr
    }
}

/// `ptr_token_to_idx`
#[inline]
unsafe fn ptr_token_to_idx(cur: *const c_char, len: usize, idx: *mut usize) -> bool {
    let end = cur.add(len);
    let mut cur = cur;
    let mut num = 0usize;
    if len == 0 || len > USIZE_SAFE_DIG {
        return false;
    }
    if *cur == b'0' as c_char {
        if len > 1 {
            return false;
        }
        *idx = 0;
        return true;
    }
    if *cur == b'-' as c_char {
        if len > 1 {
            return false;
        }
        *idx = USIZE_MAX;
        return true;
    }
    while cur < end {
        let add = (*cur as u8).wrapping_sub(b'0') as usize;
        if add > 9 {
            break;
        }
        num = num * 10 + add;
        cur = cur.add(1);
    }
    if num == 0 || cur < end {
        return false;
    }
    *idx = num;
    true
}

/// `ptr_token_eq`
#[inline]
unsafe fn ptr_token_eq(
    key: *mut c_void,
    token: *const c_char,
    len: usize,
    esc: usize,
) -> bool {
    let val = key as *mut YyjsonVal;
    if get_len(val) != len {
        return false;
    }
    if esc == 0 {
        libc::memcmp_eq((*val).uni.str_ as *const u8, token as *const u8, len)
    } else {
        let mut str_ = (*val).uni.str_;
        let mut token = token;
        let mut len = len;
        while len > 0 {
            len -= 1;
            if *token == b'~' as c_char {
                token = token.add(1);
                let want = if *token == b'0' as c_char { b'~' } else { b'/' };
                if *str_ != want as c_char {
                    return false;
                }
            } else if *str_ != *token {
                return false;
            }
            token = token.add(1);
            str_ = str_.add(1);
        }
        true
    }
}

/// `ptr_arr_get`
#[inline]
unsafe fn ptr_arr_get(
    arr: *const YyjsonVal,
    token: *const c_char,
    len: usize,
    _esc: usize,
) -> *mut YyjsonVal {
    let mut val = get_first(arr);
    let num = get_len(arr);
    let mut idx = 0usize;
    if num == 0 {
        return ptr::null_mut();
    }
    if !ptr_token_to_idx(token, len, &mut idx) {
        return ptr::null_mut();
    }
    if idx >= num {
        return ptr::null_mut();
    }
    if arr_is_flat(arr) {
        val.add(idx)
    } else {
        while idx > 0 {
            idx -= 1;
            val = get_next(val);
        }
        val
    }
}

/// `ptr_obj_get`
#[inline]
unsafe fn ptr_obj_get(
    obj: *const YyjsonVal,
    token: *const c_char,
    len: usize,
    esc: usize,
) -> *mut YyjsonVal {
    let mut key = get_first(obj);
    let mut num = get_len(obj);
    if num == 0 {
        return ptr::null_mut();
    }
    while num > 0 {
        if ptr_token_eq(key as *mut c_void, token, len, esc) {
            return key.add(1);
        }
        num -= 1;
        key = get_next(key.add(1));
    }
    ptr::null_mut()
}

/// `ptr_mut_arr_get`
#[inline]
unsafe fn ptr_mut_arr_get(
    arr: *const YyjsonMutVal,
    token: *const c_char,
    len: usize,
    _esc: usize,
    pre: *mut *mut YyjsonMutVal,
    last: *mut bool,
) -> *mut YyjsonMutVal {
    let mut val = (*arr).uni.ptr as *mut YyjsonMutVal; /* last (tail) */
    let num = get_len(mut_as_val(arr));
    let mut idx = 0usize;
    if !last.is_null() {
        *last = false;
    }
    if !pre.is_null() {
        *pre = ptr::null_mut();
    }
    if num == 0 {
        if !last.is_null() && len == 1 && (*token == b'0' as c_char || *token == b'-' as c_char)
        {
            *last = true;
        }
        return ptr::null_mut();
    }
    if !ptr_token_to_idx(token, len, &mut idx) {
        return ptr::null_mut();
    }
    if !last.is_null() {
        *last = idx == num || idx == USIZE_MAX;
    }
    if idx >= num {
        return ptr::null_mut();
    }
    while idx > 0 {
        idx -= 1;
        val = (*val).next;
    }
    if !pre.is_null() {
        *pre = val;
    }
    (*val).next
}

/// `ptr_mut_obj_get`
#[inline]
unsafe fn ptr_mut_obj_get(
    obj: *const YyjsonMutVal,
    token: *const c_char,
    len: usize,
    esc: usize,
    pre: *mut *mut YyjsonMutVal,
) -> *mut YyjsonMutVal {
    let mut pre_key = (*obj).uni.ptr as *mut YyjsonMutVal;
    let mut num = get_len(mut_as_val(obj));
    if !pre.is_null() {
        *pre = ptr::null_mut();
    }
    if num == 0 {
        return ptr::null_mut();
    }
    while num > 0 {
        let key = (*(*pre_key).next).next;
        if ptr_token_eq(key as *mut c_void, token, len, esc) {
            if !pre.is_null() {
                *pre = pre_key;
            }
            return (*key).next;
        }
        num -= 1;
        pre_key = key;
    }
    ptr::null_mut()
}

/// `ptr_new_key`
#[inline]
unsafe fn ptr_new_key(
    token: *const c_char,
    len: usize,
    esc: usize,
    doc: *mut YyjsonMutDoc,
) -> *mut YyjsonMutVal {
    let mut src = token;
    if esc == 0 {
        hdr::mut_strncpy_val(doc, src, len)
    } else {
        let end = src.add(len + esc);
        let mut dst = hdr::mut_str_alc(doc, len + esc);
        let str_ = dst;
        if dst.is_null() {
            return ptr::null_mut();
        }
        while src < end {
            if *src != b'~' as c_char {
                *dst = *src;
            } else {
                src = src.add(1);
                *dst = if *src == b'0' as c_char {
                    b'~' as c_char
                } else {
                    b'/' as c_char
                };
            }
            src = src.add(1);
            dst = dst.add(1);
        }
        *dst = 0;
        hdr::mut_strn(doc, str_, len)
    }
}

/* ===== pointer entry points ============================================= */

macro_rules! ptr_err {
    ($err:expr, $code:expr, $pos:expr, $msg:expr) => {
        if !$err.is_null() {
            (*$err).code = $code;
            (*$err).msg = cstr!($msg);
            (*$err).pos = $pos as usize;
        }
    };
}

macro_rules! err_resolve {
    ($err:expr, $pos:expr) => {
        ptr_err!(
            $err,
            YYJSON_PTR_ERR_RESOLVE,
            $pos,
            "JSON pointer cannot be resolved"
        )
    };
}
macro_rules! err_syntax {
    ($err:expr, $pos:expr) => {
        ptr_err!(
            $err,
            YYJSON_PTR_ERR_SYNTAX,
            $pos,
            "invalid escaped character"
        )
    };
}
macro_rules! err_alloc {
    ($err:expr) => {
        ptr_err!(
            $err,
            YYJSON_PTR_ERR_MEMORY_ALLOCATION,
            0,
            "failed to create value"
        )
    };
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_ptr_getx(
    val: *const YyjsonVal,
    p: *const c_char,
    ptr_len: usize,
    err: *mut YyjsonPtrErr,
) -> *mut YyjsonVal {
    let hdr_p = p;
    let mut p = p;
    let end = hdr_p.add(ptr_len);
    let mut val = val;
    let mut len: usize = 0;
    let mut esc: usize = 0;

    loop {
        let token = ptr_next_token(&mut p, end, &mut len, &mut esc);
        if token.is_null() {
            err_syntax!(err, p.offset_from(hdr_p));
            return ptr::null_mut();
        }
        let ty = get_type(val);
        val = if ty == YYJSON_TYPE_OBJ {
            ptr_obj_get(val, token, len, esc)
        } else if ty == YYJSON_TYPE_ARR {
            ptr_arr_get(val, token, len, esc)
        } else {
            ptr::null_mut()
        };
        if val.is_null() {
            err_resolve!(err, token.offset_from(hdr_p));
            return ptr::null_mut();
        }
        if p == end {
            return val as *mut YyjsonVal;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_getx(
    val: *const YyjsonMutVal,
    p: *const c_char,
    ptr_len: usize,
    ctx: *mut YyjsonPtrCtx,
    err: *mut YyjsonPtrErr,
) -> *mut YyjsonMutVal {
    let hdr_p = p;
    let mut p = p;
    let end = hdr_p.add(ptr_len);
    let mut val = val;
    let mut len: usize = 0;
    let mut esc: usize = 0;
    let mut pre: *mut YyjsonMutVal = ptr::null_mut();
    let mut idx_is_last = false;

    loop {
        let token = ptr_next_token(&mut p, end, &mut len, &mut esc);
        if token.is_null() {
            err_syntax!(err, p.offset_from(hdr_p));
            return ptr::null_mut();
        }
        let ctn = val as *mut YyjsonMutVal;
        let ty = get_type(mut_as_val(val));
        val = if ty == YYJSON_TYPE_OBJ {
            ptr_mut_obj_get(val, token, len, esc, &mut pre)
        } else if ty == YYJSON_TYPE_ARR {
            ptr_mut_arr_get(val, token, len, esc, &mut pre, &mut idx_is_last)
        } else {
            ptr::null_mut()
        };
        if !ctx.is_null() && p == end {
            if ty == YYJSON_TYPE_OBJ
                || (ty == YYJSON_TYPE_ARR && (!val.is_null() || idx_is_last))
            {
                (*ctx).ctn = ctn;
                (*ctx).pre = pre;
            }
        }
        if val.is_null() {
            err_resolve!(err, token.offset_from(hdr_p));
            return ptr::null_mut();
        }
        if p == end {
            return val as *mut YyjsonMutVal;
        }
    }
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_putx(
    val_in: *mut YyjsonMutVal,
    p: *const c_char,
    ptr_len: usize,
    new_val: *mut YyjsonMutVal,
    doc: *mut YyjsonMutDoc,
    create_parent: bool,
    insert_new: bool,
    ctx: *mut YyjsonPtrCtx,
    err: *mut YyjsonPtrErr,
) -> bool {
    let hdr_p = p;
    let mut p = p;
    let end = hdr_p.add(ptr_len);
    let mut val = val_in;
    let mut token_len: usize = 0;
    let mut esc: usize = 0;
    let mut pre: *mut YyjsonMutVal = ptr::null_mut();
    let mut sep_ctn: *mut YyjsonMutVal = ptr::null_mut();
    let mut sep_key: *mut YyjsonMutVal = ptr::null_mut();
    let mut sep_val: *mut YyjsonMutVal = ptr::null_mut();
    let mut idx_is_last = false;
    let mut ctn: *mut YyjsonMutVal;
    let mut ctn_type: u8;
    let mut token: *const c_char;
    let mut key: *mut YyjsonMutVal;

    /* skip existing parent nodes */
    loop {
        token = ptr_next_token(&mut p, end, &mut token_len, &mut esc);
        if token.is_null() {
            err_syntax!(err, p.offset_from(hdr_p));
            return false;
        }
        ctn = val;
        ctn_type = get_type(mut_as_val(ctn));
        if ctn_type == YYJSON_TYPE_OBJ {
            val = ptr_mut_obj_get(ctn, token, token_len, esc, &mut pre);
        } else if ctn_type == YYJSON_TYPE_ARR {
            val = ptr_mut_arr_get(ctn, token, token_len, esc, &mut pre, &mut idx_is_last);
        } else {
            err_resolve!(err, token.offset_from(hdr_p));
            return false;
        }
        if val.is_null() {
            break;
        }
        if p == end {
            break;
        }
    }

    /* create parent nodes if they do not exist */
    if p != end {
        if !create_parent {
            err_resolve!(err, token.offset_from(hdr_p));
            return false;
        }

        /* add value at the last index if the container is an array */
        if ctn_type == YYJSON_TYPE_ARR {
            if !idx_is_last || !insert_new {
                err_resolve!(err, token.offset_from(hdr_p));
                return false;
            }
            val = hdr::mut_obj(doc);
            if val.is_null() {
                err_alloc!(err);
                return false;
            }

            /* delay attaching until all operations are completed */
            sep_ctn = ctn;
            sep_key = ptr::null_mut();
            sep_val = val;

            ctn = val;
            val = ptr::null_mut();
            ctn_type = YYJSON_TYPE_OBJ;
            token = ptr_next_token(&mut p, end, &mut token_len, &mut esc);
            if token.is_null() {
                err_syntax!(err, p.offset_from(hdr_p));
                return false;
            }
        }

        /* container is object, create parent nodes */
        while p != end {
            key = ptr_new_key(token, token_len, esc, doc);
            if key.is_null() {
                err_alloc!(err);
                return false;
            }
            val = hdr::mut_obj(doc);
            if val.is_null() {
                err_alloc!(err);
                return false;
            }

            if sep_ctn.is_null() {
                sep_ctn = ctn;
                sep_key = key;
                sep_val = val;
            } else {
                hdr::mut_obj_add(ctn, key, val);
            }

            ctn = val;
            val = ptr::null_mut();
            token = ptr_next_token(&mut p, end, &mut token_len, &mut esc);
            if token.is_null() {
                err_syntax!(err, p.offset_from(hdr_p));
                return false;
            }
        }
    }

    /* JSON pointer is resolved, insert or replace the target value */
    let ctn_len = get_len(mut_as_val(ctn));
    if ctn_type == YYJSON_TYPE_OBJ {
        if !ctx.is_null() {
            (*ctx).ctn = ctn;
        }
        if val.is_null() || insert_new {
            /* insert new key-value pair */
            key = ptr_new_key(token, token_len, esc, doc);
            if key.is_null() {
                err_alloc!(err);
                return false;
            }
            if !ctx.is_null() {
                (*ctx).pre = if ctn_len != 0 {
                    (*ctn).uni.ptr as *mut YyjsonMutVal
                } else {
                    key
                };
            }
            hdr::unsafe_mut_obj_add(ctn, key, new_val, ctn_len);
        } else {
            /* replace existing value */
            key = (*(*pre).next).next;
            if !ctx.is_null() {
                (*ctx).pre = pre;
                (*ctx).old = val;
            }
            hdr::mut_obj_put(ctn, key, new_val);
        }
    } else {
        /* array */
        if !ctx.is_null() && (!val.is_null() || idx_is_last) {
            (*ctx).ctn = ctn;
        }
        if insert_new {
            /* append new value */
            if !val.is_null() {
                (*pre).next = new_val;
                (*new_val).next = val;
                if !ctx.is_null() {
                    (*ctx).pre = pre;
                }
                set_len(mut_as_val(ctn), ctn_len + 1);
            } else if idx_is_last {
                if !ctx.is_null() {
                    (*ctx).pre = if ctn_len != 0 {
                        (*ctn).uni.ptr as *mut YyjsonMutVal
                    } else {
                        new_val
                    };
                }
                hdr::mut_arr_append(ctn, new_val);
            } else {
                err_resolve!(err, token.offset_from(hdr_p));
                return false;
            }
        } else {
            /* replace existing value */
            if val.is_null() {
                err_resolve!(err, token.offset_from(hdr_p));
                return false;
            }
            if ctn_len > 1 {
                (*new_val).next = (*val).next;
                (*pre).next = new_val;
                if (*ctn).uni.ptr == val as *mut c_void {
                    (*ctn).uni.ptr = new_val as *mut c_void;
                }
            } else {
                (*new_val).next = new_val;
                (*ctn).uni.ptr = new_val as *mut c_void;
                pre = new_val;
            }
            if !ctx.is_null() {
                (*ctx).pre = pre;
                (*ctx).old = val;
            }
        }
    }

    /* all operations are completed, attach the new components to the target */
    if !sep_ctn.is_null() {
        if !sep_key.is_null() {
            hdr::mut_obj_add(sep_ctn, sep_key, sep_val);
        } else {
            hdr::mut_arr_append(sep_ctn, sep_val);
        }
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_replacex(
    val: *mut YyjsonMutVal,
    p: *const c_char,
    len: usize,
    new_val: *mut YyjsonMutVal,
    ctx_in: *mut YyjsonPtrCtx,
    err: *mut YyjsonPtrErr,
) -> *mut YyjsonMutVal {
    let mut cur_ctx = YyjsonPtrCtx {
        ctn: ptr::null_mut(),
        pre: ptr::null_mut(),
        old: ptr::null_mut(),
    };
    let ctx = if ctx_in.is_null() {
        &mut cur_ctx as *mut YyjsonPtrCtx
    } else {
        ctx_in
    };
    let cur_val = unsafe_yyjson_mut_ptr_getx(val, p, len, ctx, err);
    if cur_val.is_null() {
        return ptr::null_mut();
    }

    if !(*ctx).ctn.is_null() && get_type(mut_as_val((*ctx).ctn)) == YYJSON_TYPE_OBJ {
        let key = (*(*(*ctx).pre).next).next;
        hdr::mut_obj_put((*ctx).ctn, key, new_val);
    } else {
        hdr::ptr_ctx_replace(ctx, new_val);
    }
    (*ctx).old = cur_val;
    cur_val
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_removex(
    val: *mut YyjsonMutVal,
    p: *const c_char,
    len: usize,
    ctx_in: *mut YyjsonPtrCtx,
    err: *mut YyjsonPtrErr,
) -> *mut YyjsonMutVal {
    let mut cur_ctx = YyjsonPtrCtx {
        ctn: ptr::null_mut(),
        pre: ptr::null_mut(),
        old: ptr::null_mut(),
    };
    let ctx = if ctx_in.is_null() {
        &mut cur_ctx as *mut YyjsonPtrCtx
    } else {
        ctx_in
    };
    let cur_val = unsafe_yyjson_mut_ptr_getx(val, p, len, ctx, err);
    if !cur_val.is_null() {
        if !(*ctx).ctn.is_null() && get_type(mut_as_val((*ctx).ctn)) == YYJSON_TYPE_OBJ {
            let key = (*(*(*ctx).pre).next).next;
            hdr::mut_obj_put((*ctx).ctn, key, ptr::null_mut());
        } else {
            hdr::ptr_ctx_remove(ctx);
        }
        (*ctx).pre = ptr::null_mut();
        (*ctx).old = cur_val;
    }
    cur_val
}

/* ===== JSON Patch (RFC 6902) ============================================ */

#[derive(PartialEq, Clone, Copy)]
enum PatchOp {
    Add,
    Remove,
    Replace,
    Move,
    Copy,
    Test,
    None,
}

unsafe fn patch_op_get(op: *mut YyjsonVal) -> PatchOp {
    let s = (*op).uni.str_ as *const u8;
    match get_len(op) {
        3 => {
            if libc::memcmp_eq(s, b"add".as_ptr(), 3) {
                return PatchOp::Add;
            }
            PatchOp::None
        }
        4 => {
            if libc::memcmp_eq(s, b"move".as_ptr(), 4) {
                return PatchOp::Move;
            }
            if libc::memcmp_eq(s, b"copy".as_ptr(), 4) {
                return PatchOp::Copy;
            }
            if libc::memcmp_eq(s, b"test".as_ptr(), 4) {
                return PatchOp::Test;
            }
            PatchOp::None
        }
        6 => {
            if libc::memcmp_eq(s, b"remove".as_ptr(), 6) {
                return PatchOp::Remove;
            }
            PatchOp::None
        }
        7 => {
            if libc::memcmp_eq(s, b"replace".as_ptr(), 7) {
                return PatchOp::Replace;
            }
            PatchOp::None
        }
        _ => PatchOp::None,
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_patch(
    doc: *mut YyjsonMutDoc,
    orig: *const YyjsonVal,
    patch: *const YyjsonVal,
    err_in: *mut YyjsonPatchErr,
) -> *mut YyjsonMutVal {
    let mut err_tmp = core::mem::MaybeUninit::<YyjsonPatchErr>::uninit();
    let err = if err_in.is_null() {
        err_tmp.as_mut_ptr()
    } else {
        err_in
    };
    let mut iter = core::mem::MaybeUninit::<YyjsonArrIter>::uninit();
    let iter = iter.as_mut_ptr();
    libc::zero(err);
    libc::zero(iter);

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            if (*err).ptr.code == YYJSON_PTR_ERR_MEMORY_ALLOCATION {
                (*err).code = YYJSON_PATCH_ERROR_MEMORY_ALLOCATION;
                (*err).msg = cstr!($m);
                libc::zero(&mut (*err).ptr);
            } else {
                (*err).code = $code;
                (*err).msg = cstr!($m);
                (*err).idx = if (*iter).idx != 0 { (*iter).idx - 1 } else { 0 };
            }
            return ptr::null_mut();
        }};
    }

    if doc.is_null() || orig.is_null() || patch.is_null() {
        return_err!(
            YYJSON_PATCH_ERROR_INVALID_PARAMETER,
            "input parameter is NULL"
        );
    }
    if get_type(patch) != YYJSON_TYPE_ARR {
        return_err!(
            YYJSON_PATCH_ERROR_INVALID_PARAMETER,
            "input patch is not array"
        );
    }
    let mut root = crate::strct::yyjson_val_mut_copy(doc, orig);
    if root.is_null() {
        return_err!(YYJSON_PATCH_ERROR_MEMORY_ALLOCATION, "failed to copy value");
    }

    hdr::arr_iter_init(patch, iter);
    loop {
        let obj = hdr::arr_iter_next(iter);
        if obj.is_null() {
            break;
        }
        let mut val: *mut YyjsonMutVal = ptr::null_mut();
        let mut from_len: usize = 0;
        if get_type(obj) != YYJSON_TYPE_OBJ {
            return_err!(
                YYJSON_PATCH_ERROR_INVALID_OPERATION,
                "JSON patch operation is not object"
            );
        }

        /* get required member: op */
        let op = hdr::obj_get(obj, "op");
        if op.is_null() {
            return_err!(YYJSON_PATCH_ERROR_MISSING_KEY, "missing key `op`");
        }
        if get_type(op) != YYJSON_TYPE_STR {
            return_err!(YYJSON_PATCH_ERROR_INVALID_MEMBER, "invalid member `op`");
        }
        let op_enum = patch_op_get(op);

        /* get required member: path */
        let path = hdr::obj_get(obj, "path");
        if path.is_null() {
            return_err!(YYJSON_PATCH_ERROR_MISSING_KEY, "missing key `path`");
        }
        if get_type(path) != YYJSON_TYPE_STR {
            return_err!(YYJSON_PATCH_ERROR_INVALID_MEMBER, "invalid member `path`");
        }
        let path_len = get_len(path);
        let mut from: *mut YyjsonVal = ptr::null_mut();

        /* get required member: value, from */
        match op_enum {
            PatchOp::Add | PatchOp::Replace | PatchOp::Test => {
                let value = hdr::obj_get(obj, "value");
                if value.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_MISSING_KEY, "missing key `value`");
                }
                val = crate::strct::yyjson_val_mut_copy(doc, value);
                if val.is_null() {
                    return_err!(
                        YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                        "failed to copy value"
                    );
                }
            }
            PatchOp::Move | PatchOp::Copy => {
                from = hdr::obj_get(obj, "from");
                if from.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_MISSING_KEY, "missing key `from`");
                }
                if get_type(from) != YYJSON_TYPE_STR {
                    return_err!(YYJSON_PATCH_ERROR_INVALID_MEMBER, "invalid member `from`");
                }
                from_len = get_len(from);
            }
            _ => {}
        }

        macro_rules! ptr_get {
            ($v:expr, $l:expr) => {
                hdr::mut_ptr_getx(root, (*$v).uni.str_, $l, ptr::null_mut(), &mut (*err).ptr)
            };
        }
        macro_rules! ptr_add {
            ($v:expr, $l:expr, $nv:expr) => {
                hdr::mut_ptr_addx(
                    root,
                    (*$v).uni.str_,
                    $l,
                    $nv,
                    doc,
                    false,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                )
            };
        }
        macro_rules! ptr_remove {
            ($v:expr, $l:expr) => {
                hdr::mut_ptr_removex(root, (*$v).uni.str_, $l, ptr::null_mut(), &mut (*err).ptr)
            };
        }
        macro_rules! ptr_replace {
            ($v:expr, $l:expr, $nv:expr) => {
                hdr::mut_ptr_replacex(
                    root,
                    (*$v).uni.str_,
                    $l,
                    $nv,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                )
            };
        }

        /* perform an operation */
        match op_enum {
            PatchOp::Add => {
                if path_len == 0 {
                    root = val;
                } else if !ptr_add!(path, path_len, val) {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to add `path`");
                }
            }
            PatchOp::Remove => {
                if ptr_remove!(path, path_len).is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to remove `path`");
                }
            }
            PatchOp::Replace => {
                if path_len == 0 {
                    root = val;
                } else if ptr_replace!(path, path_len, val).is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to replace `path`");
                }
            }
            PatchOp::Move => {
                if from_len == 0 && path_len == 0 {
                    continue;
                }
                val = ptr_remove!(from, from_len);
                if val.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to remove `from`");
                }
                if path_len == 0 {
                    root = val;
                } else if !ptr_add!(path, path_len, val) {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to add `path`");
                }
            }
            PatchOp::Copy => {
                val = ptr_get!(from, from_len);
                if val.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to get `from`");
                }
                if path_len == 0 {
                    root = val;
                } else {
                    val = crate::strct::yyjson_mut_val_mut_copy(doc, val);
                    if val.is_null() {
                        return_err!(
                            YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                            "failed to copy value"
                        );
                    }
                    if !ptr_add!(path, path_len, val) {
                        return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to add `path`");
                    }
                }
            }
            PatchOp::Test => {
                let test = ptr_get!(path, path_len);
                if test.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to get `path`");
                }
                if val.is_null()
                    || test.is_null()
                    || !crate::strct::unsafe_yyjson_mut_equals(val, test)
                {
                    return_err!(YYJSON_PATCH_ERROR_EQUAL, "failed to test equal");
                }
            }
            PatchOp::None => {
                return_err!(YYJSON_PATCH_ERROR_INVALID_MEMBER, "unsupported `op`");
            }
        }
    }
    root
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_patch(
    doc: *mut YyjsonMutDoc,
    orig: *const YyjsonMutVal,
    patch: *const YyjsonMutVal,
    err_in: *mut YyjsonPatchErr,
) -> *mut YyjsonMutVal {
    let mut err_tmp = core::mem::MaybeUninit::<YyjsonPatchErr>::uninit();
    let err = if err_in.is_null() {
        err_tmp.as_mut_ptr()
    } else {
        err_in
    };
    let mut iter = core::mem::MaybeUninit::<YyjsonMutArrIter>::uninit();
    let iter = iter.as_mut_ptr();
    libc::zero(err);
    libc::zero(iter);

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            if (*err).ptr.code == YYJSON_PTR_ERR_MEMORY_ALLOCATION {
                (*err).code = YYJSON_PATCH_ERROR_MEMORY_ALLOCATION;
                (*err).msg = cstr!($m);
                libc::zero(&mut (*err).ptr);
            } else {
                (*err).code = $code;
                (*err).msg = cstr!($m);
                (*err).idx = if (*iter).idx != 0 { (*iter).idx - 1 } else { 0 };
            }
            return ptr::null_mut();
        }};
    }

    if doc.is_null() || orig.is_null() || patch.is_null() {
        return_err!(
            YYJSON_PATCH_ERROR_INVALID_PARAMETER,
            "input parameter is NULL"
        );
    }
    if get_type(mut_as_val(patch)) != YYJSON_TYPE_ARR {
        return_err!(
            YYJSON_PATCH_ERROR_INVALID_PARAMETER,
            "input patch is not array"
        );
    }
    let mut root = crate::strct::yyjson_mut_val_mut_copy(doc, orig);
    if root.is_null() {
        return_err!(YYJSON_PATCH_ERROR_MEMORY_ALLOCATION, "failed to copy value");
    }

    hdr::mut_arr_iter_init(patch as *mut YyjsonMutVal, iter);
    loop {
        let obj = hdr::mut_arr_iter_next(iter);
        if obj.is_null() {
            break;
        }
        let mut val: *mut YyjsonMutVal = ptr::null_mut();
        let mut from_len: usize = 0;
        if get_type(mut_as_val(obj)) != YYJSON_TYPE_OBJ {
            return_err!(
                YYJSON_PATCH_ERROR_INVALID_OPERATION,
                "JSON patch operation is not object"
            );
        }

        let op = hdr::mut_obj_get(obj, "op");
        if op.is_null() {
            return_err!(YYJSON_PATCH_ERROR_MISSING_KEY, "missing key `op`");
        }
        if get_type(mut_as_val(op)) != YYJSON_TYPE_STR {
            return_err!(YYJSON_PATCH_ERROR_INVALID_MEMBER, "invalid member `op`");
        }
        let op_enum = patch_op_get(mut_as_val(op));

        let path = hdr::mut_obj_get(obj, "path");
        if path.is_null() {
            return_err!(YYJSON_PATCH_ERROR_MISSING_KEY, "missing key `path`");
        }
        if get_type(mut_as_val(path)) != YYJSON_TYPE_STR {
            return_err!(YYJSON_PATCH_ERROR_INVALID_MEMBER, "invalid member `path`");
        }
        let path_len = get_len(mut_as_val(path));
        let mut from: *mut YyjsonMutVal = ptr::null_mut();

        match op_enum {
            PatchOp::Add | PatchOp::Replace | PatchOp::Test => {
                let value = hdr::mut_obj_get(obj, "value");
                if value.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_MISSING_KEY, "missing key `value`");
                }
                val = crate::strct::yyjson_mut_val_mut_copy(doc, value);
                if val.is_null() {
                    return_err!(
                        YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                        "failed to copy value"
                    );
                }
            }
            PatchOp::Move | PatchOp::Copy => {
                from = hdr::mut_obj_get(obj, "from");
                if from.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_MISSING_KEY, "missing key `from`");
                }
                if get_type(mut_as_val(from)) != YYJSON_TYPE_STR {
                    return_err!(YYJSON_PATCH_ERROR_INVALID_MEMBER, "invalid member `from`");
                }
                from_len = get_len(mut_as_val(from));
            }
            _ => {}
        }

        macro_rules! ptr_get {
            ($v:expr, $l:expr) => {
                hdr::mut_ptr_getx(root, (*$v).uni.str_, $l, ptr::null_mut(), &mut (*err).ptr)
            };
        }
        macro_rules! ptr_add {
            ($v:expr, $l:expr, $nv:expr) => {
                hdr::mut_ptr_addx(
                    root,
                    (*$v).uni.str_,
                    $l,
                    $nv,
                    doc,
                    false,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                )
            };
        }
        macro_rules! ptr_remove {
            ($v:expr, $l:expr) => {
                hdr::mut_ptr_removex(root, (*$v).uni.str_, $l, ptr::null_mut(), &mut (*err).ptr)
            };
        }
        macro_rules! ptr_replace {
            ($v:expr, $l:expr, $nv:expr) => {
                hdr::mut_ptr_replacex(
                    root,
                    (*$v).uni.str_,
                    $l,
                    $nv,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                )
            };
        }

        match op_enum {
            PatchOp::Add => {
                if path_len == 0 {
                    root = val;
                } else if !ptr_add!(path, path_len, val) {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to add `path`");
                }
            }
            PatchOp::Remove => {
                if ptr_remove!(path, path_len).is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to remove `path`");
                }
            }
            PatchOp::Replace => {
                if path_len == 0 {
                    root = val;
                } else if ptr_replace!(path, path_len, val).is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to replace `path`");
                }
            }
            PatchOp::Move => {
                if from_len == 0 && path_len == 0 {
                    continue;
                }
                val = ptr_remove!(from, from_len);
                if val.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to remove `from`");
                }
                if path_len == 0 {
                    root = val;
                } else if !ptr_add!(path, path_len, val) {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to add `path`");
                }
            }
            PatchOp::Copy => {
                val = ptr_get!(from, from_len);
                if val.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to get `from`");
                }
                if path_len == 0 {
                    root = val;
                } else {
                    val = crate::strct::yyjson_mut_val_mut_copy(doc, val);
                    if val.is_null() {
                        return_err!(
                            YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                            "failed to copy value"
                        );
                    }
                    if !ptr_add!(path, path_len, val) {
                        return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to add `path`");
                    }
                }
            }
            PatchOp::Test => {
                let test = ptr_get!(path, path_len);
                if test.is_null() {
                    return_err!(YYJSON_PATCH_ERROR_POINTER, "failed to get `path`");
                }
                if val.is_null()
                    || test.is_null()
                    || !crate::strct::unsafe_yyjson_mut_equals(val, test)
                {
                    return_err!(YYJSON_PATCH_ERROR_EQUAL, "failed to test equal");
                }
            }
            PatchOp::None => {
                return_err!(YYJSON_PATCH_ERROR_INVALID_MEMBER, "unsupported `op`");
            }
        }
    }
    root
}

/* ===== JSON Merge Patch (RFC 7386) ====================================== */

#[no_mangle]
pub unsafe extern "C" fn yyjson_merge_patch(
    doc: *mut YyjsonMutDoc,
    orig: *const YyjsonVal,
    patch: *const YyjsonVal,
) -> *mut YyjsonMutVal {
    if patch.is_null() || get_type(patch) != YYJSON_TYPE_OBJ {
        return crate::strct::yyjson_val_mut_copy(doc, patch);
    }

    let builder = hdr::mut_obj(doc);
    if builder.is_null() {
        return ptr::null_mut();
    }

    let mut local_orig = YyjsonVal {
        tag: 0,
        uni: YyjsonValUni { u64_: 0 },
    };
    libc::zero(&mut local_orig);
    let mut orig = orig;
    let is_local;
    if orig.is_null() || get_type(orig) != YYJSON_TYPE_OBJ {
        local_orig.tag = (*builder).tag;
        local_orig.uni = (*builder).uni;
        orig = &local_orig;
        is_local = true;
    } else {
        is_local = false;
    }

    /* if orig is contributing, copy any items not modified by the patch */
    if !is_local {
        let mut idx = 0usize;
        let max = get_len(orig);
        let mut key = get_first(orig);
        while idx < max {
            let orig_val = key.add(1);
            let patch_val = hdr::obj_getn(patch, get_str(key), get_len(key));
            if patch_val.is_null() {
                let mut_key = crate::strct::yyjson_val_mut_copy(doc, key);
                let mut_val = crate::strct::yyjson_val_mut_copy(doc, orig_val);
                if !hdr::mut_obj_add(builder, mut_key, mut_val) {
                    return ptr::null_mut();
                }
            }
            idx += 1;
            key = get_next(key.add(1));
        }
    }

    /* merge items modified by the patch */
    let mut idx = 0usize;
    let max = get_len(patch);
    let mut key = get_first(patch);
    while idx < max {
        let patch_val = key.add(1);
        if get_type(patch_val) == YYJSON_TYPE_NULL {
            /* null indicates the field is removed */
            idx += 1;
            key = get_next(key.add(1));
            continue;
        }
        let mut_key = crate::strct::yyjson_val_mut_copy(doc, key);
        let orig_val = hdr::obj_getn(orig, get_str(key), get_len(key));
        let merged_val = yyjson_merge_patch(doc, orig_val, patch_val);
        if !hdr::mut_obj_add(builder, mut_key, merged_val) {
            return ptr::null_mut();
        }
        idx += 1;
        key = get_next(key.add(1));
    }

    builder
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_merge_patch(
    doc: *mut YyjsonMutDoc,
    orig: *const YyjsonMutVal,
    patch: *const YyjsonMutVal,
) -> *mut YyjsonMutVal {
    if patch.is_null() || get_type(mut_as_val(patch)) != YYJSON_TYPE_OBJ {
        return crate::strct::yyjson_mut_val_mut_copy(doc, patch);
    }

    let builder = hdr::mut_obj(doc);
    if builder.is_null() {
        return ptr::null_mut();
    }

    let mut local_orig = YyjsonMutVal {
        tag: 0,
        uni: YyjsonValUni { u64_: 0 },
        next: ptr::null_mut(),
    };
    libc::zero(&mut local_orig);
    let mut orig = orig;
    let is_local;
    if orig.is_null() || get_type(mut_as_val(orig)) != YYJSON_TYPE_OBJ {
        local_orig.tag = (*builder).tag;
        local_orig.uni = (*builder).uni;
        orig = &local_orig;
        is_local = true;
    } else {
        is_local = false;
    }

    if !is_local {
        /* `yyjson_mut_obj_foreach(orig, idx, max, key, val)` */
        let max = hdr::mut_obj_size(orig);
        let mut idx = 0usize;
        let mut key = if max != 0 {
            (*(*((*orig).uni.ptr as *mut YyjsonMutVal)).next).next
        } else {
            ptr::null_mut()
        };
        let mut orig_val = if !key.is_null() {
            (*key).next
        } else {
            ptr::null_mut()
        };
        while idx < max {
            let patch_val =
                hdr::mut_obj_getn(patch, (*key).uni.str_, get_len(mut_as_val(key)));
            if patch_val.is_null() {
                let mut_key = crate::strct::yyjson_mut_val_mut_copy(doc, key);
                let mut_val = crate::strct::yyjson_mut_val_mut_copy(doc, orig_val);
                if !hdr::mut_obj_add(builder, mut_key, mut_val) {
                    return ptr::null_mut();
                }
            }
            idx += 1;
            key = (*orig_val).next;
            orig_val = (*key).next;
        }
    }

    /* `yyjson_mut_obj_foreach(patch, idx, max, key, val)` */
    let max = hdr::mut_obj_size(patch);
    let mut idx = 0usize;
    let mut key = if max != 0 {
        (*(*((*patch).uni.ptr as *mut YyjsonMutVal)).next).next
    } else {
        ptr::null_mut()
    };
    let mut patch_val = if !key.is_null() {
        (*key).next
    } else {
        ptr::null_mut()
    };
    while idx < max {
        if get_type(mut_as_val(patch_val)) != YYJSON_TYPE_NULL {
            let mut_key = crate::strct::yyjson_mut_val_mut_copy(doc, key);
            let orig_val =
                hdr::mut_obj_getn(orig, (*key).uni.str_, get_len(mut_as_val(key)));
            let merged_val = yyjson_mut_merge_patch(doc, orig_val, patch_val);
            if !hdr::mut_obj_add(builder, mut_key, merged_val) {
                return ptr::null_mut();
            }
        }
        idx += 1;
        key = (*patch_val).next;
        patch_val = (*key).next;
    }

    builder
}
