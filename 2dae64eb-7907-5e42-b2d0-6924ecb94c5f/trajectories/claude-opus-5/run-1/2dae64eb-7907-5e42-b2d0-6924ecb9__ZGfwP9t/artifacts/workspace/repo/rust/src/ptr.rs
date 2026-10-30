//! JSON Pointer (RFC 6901), JSON Patch (RFC 6902) and JSON Merge Patch
//! (RFC 7386).
//!
//! The five exported `unsafe_yyjson_*ptr_*x` functions are the ones the
//! header's inline wrappers call. The wrappers themselves live in the header
//! and are compiled into the caller, but the patch code here needs them too,
//! so they are reproduced as the private `mut_ptr_*x` helpers below —
//! including their parameter checks, which are part of the observable
//! behaviour of `yyjson_patch`.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::doc::{mut_str_alc, mut_strncpy, mut_val, MutObjIter};
use crate::mem;
use crate::types::*;
use crate::util::*;

/* ------------------------------- header inline helpers, reimplemented */

/// `yyjson_mut_obj`
#[inline(always)]
unsafe fn new_mut_obj(doc: *mut yyjson_mut_doc) -> *mut yyjson_mut_val {
    if !doc.is_null() {
        let val = mut_val(doc, 1);
        if !val.is_null() {
            (*val).tag = (YYJSON_TYPE_OBJ | YYJSON_SUBTYPE_NONE) as u64;
            return val;
        }
    }
    ptr::null_mut()
}

/// `yyjson_mut_strn`
#[inline(always)]
unsafe fn new_mut_strn(
    doc: *mut yyjson_mut_doc,
    str_: *const c_char,
    len: usize,
) -> *mut yyjson_mut_val {
    if !doc.is_null() && !str_.is_null() {
        let val = mut_val(doc, 1);
        if !val.is_null() {
            unsafe_set_tag(
                val as *mut yyjson_val,
                YYJSON_TYPE_STR,
                YYJSON_SUBTYPE_NONE,
                len,
            );
            (*val).uni.str_ = str_;
            return val;
        }
    }
    ptr::null_mut()
}

/// `yyjson_mut_strncpy`
#[inline(always)]
unsafe fn new_mut_strncpy(
    doc: *mut yyjson_mut_doc,
    str_: *const c_char,
    len: usize,
) -> *mut yyjson_mut_val {
    if !doc.is_null() && !str_.is_null() {
        let val = mut_val(doc, 1);
        if !val.is_null() {
            let new_str = mut_strncpy(doc, str_, len);
            if new_str.is_null() {
                return ptr::null_mut();
            }
            unsafe_set_tag(
                val as *mut yyjson_val,
                YYJSON_TYPE_STR,
                YYJSON_SUBTYPE_NONE,
                len,
            );
            (*val).uni.str_ = new_str;
            return val;
        }
    }
    ptr::null_mut()
}

#[inline(always)]
unsafe fn mut_is_obj(val: *const yyjson_mut_val) -> bool {
    !val.is_null() && mut_get_type(val) == YYJSON_TYPE_OBJ
}

#[inline(always)]
unsafe fn mut_is_arr(val: *const yyjson_mut_val) -> bool {
    !val.is_null() && mut_get_type(val) == YYJSON_TYPE_ARR
}

#[inline(always)]
unsafe fn mut_is_str(val: *const yyjson_mut_val) -> bool {
    !val.is_null() && mut_get_type(val) == YYJSON_TYPE_STR
}

#[inline(always)]
unsafe fn is_obj(val: *const yyjson_val) -> bool {
    !val.is_null() && unsafe_get_type(val) == YYJSON_TYPE_OBJ
}

#[inline(always)]
unsafe fn is_arr(val: *const yyjson_val) -> bool {
    !val.is_null() && unsafe_get_type(val) == YYJSON_TYPE_ARR
}

#[inline(always)]
unsafe fn is_str(val: *const yyjson_val) -> bool {
    !val.is_null() && unsafe_get_type(val) == YYJSON_TYPE_STR
}

/// `unsafe_yyjson_arr_is_flat`
#[inline(always)]
unsafe fn arr_is_flat(val: *const yyjson_val) -> bool {
    let ofs = (*val).uni.ofs;
    let len = ((*val).tag >> YYJSON_TAG_BIT) as usize;
    len * size_of::<yyjson_val>() + size_of::<yyjson_val>() == ofs
}

/// `unsafe_yyjson_mut_obj_add`
#[inline(always)]
unsafe fn unsafe_mut_obj_add(
    obj: *mut yyjson_mut_val,
    key: *mut yyjson_mut_val,
    val: *mut yyjson_mut_val,
    len: usize,
) {
    if len != 0 {
        let prev_val = (*((*obj).uni.ptr as *mut yyjson_mut_val)).next;
        let next_key = (*prev_val).next;
        (*prev_val).next = key;
        (*val).next = next_key;
    } else {
        (*val).next = key;
    }
    (*key).next = val;
    (*obj).uni.ptr = key as *mut c_void;
    mut_set_len(obj, len + 1);
}

/// `yyjson_mut_obj_add`
#[inline(always)]
unsafe fn mut_obj_add(
    obj: *mut yyjson_mut_val,
    key: *mut yyjson_mut_val,
    val: *mut yyjson_mut_val,
) -> bool {
    if mut_is_obj(obj) && mut_is_str(key) && !val.is_null() {
        unsafe_mut_obj_add(obj, key, val, mut_get_len(obj));
        return true;
    }
    false
}

/// `yyjson_mut_obj_put`
unsafe fn mut_obj_put(
    obj: *mut yyjson_mut_val,
    key: *mut yyjson_mut_val,
    val: *mut yyjson_mut_val,
) -> bool {
    let mut replaced = false;
    if !mut_is_obj(obj) || !mut_is_str(key) {
        return false;
    }
    let key_len = mut_get_len(key);
    let mut iter = MutObjIter::new(obj);
    loop {
        let cur_key = iter.next();
        if cur_key.is_null() {
            break;
        }
        if unsafe_equals_strn(cur_key as *const yyjson_val, (*key).uni.str_, key_len) {
            if !replaced && !val.is_null() {
                replaced = true;
                (*val).next = (*(*cur_key).next).next;
                (*cur_key).next = val;
            } else {
                iter.remove();
            }
        }
    }
    if !replaced && !val.is_null() {
        unsafe_mut_obj_add(obj, key, val, iter.max);
    }
    true
}

/// `yyjson_mut_arr_append`
#[inline(always)]
unsafe fn mut_arr_append(arr: *mut yyjson_mut_val, val: *mut yyjson_mut_val) -> bool {
    if mut_is_arr(arr) && !val.is_null() {
        let len = mut_get_len(arr);
        mut_set_len(arr, len + 1);
        if len == 0 {
            (*val).next = val;
        } else {
            let prev = (*arr).uni.ptr as *mut yyjson_mut_val;
            let next = (*prev).next;
            (*prev).next = val;
            (*val).next = next;
        }
        (*arr).uni.ptr = val as *mut c_void;
        return true;
    }
    false
}

/// `yyjson_ptr_ctx_replace`
unsafe fn ptr_ctx_replace(ctx: *mut yyjson_ptr_ctx, val: *mut yyjson_mut_val) -> bool {
    if ctx.is_null() || (*ctx).ctn.is_null() || (*ctx).pre.is_null() || val.is_null() {
        return false;
    }
    let ctn = (*ctx).ctn;
    if mut_is_obj(ctn) {
        let pre_key = (*ctx).pre;
        let pre_val = (*pre_key).next;
        let cur_key = (*pre_val).next;
        let cur_val = (*cur_key).next;
        /* replace current value */
        (*cur_key).next = val;
        (*val).next = (*cur_val).next;
        (*ctx).old = cur_val;
    } else {
        let pre_val = (*ctx).pre;
        let cur_val = (*pre_val).next;
        /* replace current value */
        if pre_val != cur_val {
            (*val).next = (*cur_val).next;
            (*pre_val).next = val;
            if (*ctn).uni.ptr == cur_val as *mut c_void {
                (*ctn).uni.ptr = val as *mut c_void;
            }
        } else {
            (*val).next = val;
            (*ctn).uni.ptr = val as *mut c_void;
            (*ctx).pre = val;
        }
        (*ctx).old = cur_val;
    }
    true
}

/// `yyjson_ptr_ctx_remove`
unsafe fn ptr_ctx_remove(ctx: *mut yyjson_ptr_ctx) -> bool {
    if ctx.is_null() || (*ctx).ctn.is_null() || (*ctx).pre.is_null() {
        return false;
    }
    let ctn = (*ctx).ctn;
    if mut_is_obj(ctn) {
        let pre_key = (*ctx).pre;
        let pre_val = (*pre_key).next;
        let cur_key = (*pre_val).next;
        let cur_val = (*cur_key).next;
        (*pre_val).next = (*cur_val).next;
        if (*ctn).uni.ptr == cur_key as *mut c_void {
            (*ctn).uni.ptr = pre_key as *mut c_void;
        }
        (*ctx).pre = ptr::null_mut();
        (*ctx).old = cur_val;
    } else {
        let pre_val = (*ctx).pre;
        let cur_val = (*pre_val).next;
        (*pre_val).next = (*cur_val).next;
        if (*ctn).uni.ptr == cur_val as *mut c_void {
            (*ctn).uni.ptr = pre_val as *mut c_void;
        }
        (*ctx).pre = ptr::null_mut();
        (*ctx).old = cur_val;
    }
    let len = mut_get_len(ctn) - 1;
    if len == 0 {
        (*ctn).uni.ptr = ptr::null_mut();
    }
    mut_set_len(ctn, len);
    true
}

/// `yyjson_mut_obj_getn` (the object API, a plain linear scan)
#[inline(always)]
unsafe fn mut_obj_getn(
    obj: *const yyjson_mut_val,
    key: *const c_char,
    key_len: usize,
) -> *mut yyjson_mut_val {
    let mut len = if mut_is_obj(obj) { mut_get_len(obj) } else { 0 };
    if len != 0 && !key.is_null() {
        let mut k = (*(*((*obj).uni.ptr as *mut yyjson_mut_val)).next).next;
        while len > 0 {
            len -= 1;
            if unsafe_equals_strn(k as *const yyjson_val, key, key_len) {
                return (*k).next;
            }
            k = (*(*k).next).next;
        }
    }
    ptr::null_mut()
}

/// `yyjson_obj_getn`
#[inline(always)]
unsafe fn obj_getn(obj: *const yyjson_val, key: *const c_char, key_len: usize) -> *mut yyjson_val {
    if is_obj(obj) && !key.is_null() {
        let mut len = unsafe_get_len(obj);
        let mut k = unsafe_get_first(obj);
        while len > 0 {
            len -= 1;
            if unsafe_equals_strn(k, key, key_len) {
                return k.add(1);
            }
            k = unsafe_get_next(k.add(1));
        }
    }
    ptr::null_mut()
}

/// `yyjson_obj_get` for a NUL-terminated literal.
#[inline(always)]
unsafe fn obj_get_lit(obj: *const yyjson_val, key: &[u8]) -> *mut yyjson_val {
    obj_getn(obj, key.as_ptr() as *const c_char, key.len() - 1)
}

/// `yyjson_mut_obj_get` for a NUL-terminated literal.
#[inline(always)]
unsafe fn mut_obj_get_lit(obj: *const yyjson_mut_val, key: &[u8]) -> *mut yyjson_mut_val {
    mut_obj_getn(obj, key.as_ptr() as *const c_char, key.len() - 1)
}

/// `yyjson_mut_equals`
#[inline(always)]
unsafe fn mut_equals(lhs: *const yyjson_mut_val, rhs: *const yyjson_mut_val) -> bool {
    if lhs.is_null() || rhs.is_null() {
        return false;
    }
    crate::doc::unsafe_yyjson_mut_equals(lhs, rhs)
}

/* -------------------------------------------------------- token parsing */

/// `ptr_next_token`
#[inline(always)]
unsafe fn ptr_next_token(
    ptr_: &mut *const c_char,
    end: *const c_char,
    len: &mut usize,
    esc: &mut usize,
) -> *const c_char {
    let hdr = (*ptr_).add(1);
    let mut cur = hdr;
    /* skip unescaped characters */
    while cur < end && *cur != b'/' as c_char && *cur != b'~' as c_char {
        cur = cur.add(1);
    }
    if cur == end || *cur != b'~' as c_char {
        /* no escaped characters, return */
        *ptr_ = cur;
        *len = cur as usize - hdr as usize;
        *esc = 0;
        hdr
    } else {
        /* handle escaped characters */
        let mut esc_num: usize = 0;
        while cur < end && *cur != b'/' as c_char {
            let c = *cur;
            cur = cur.add(1);
            if c == b'~' as c_char {
                if cur == end || (*cur != b'0' as c_char && *cur != b'1' as c_char) {
                    *ptr_ = cur.sub(1);
                    return ptr::null();
                }
                esc_num += 1;
            }
        }
        *ptr_ = cur;
        *len = (cur as usize - hdr as usize) - esc_num;
        *esc = esc_num;
        hdr
    }
}

/// `ptr_token_to_idx`
#[inline(always)]
unsafe fn ptr_token_to_idx(cur: *const c_char, len: usize, idx: &mut usize) -> bool {
    let end = cur.add(len);
    let mut cur = cur;
    let mut num: usize = 0;
    if len == 0 || len > USIZE_SAFE_DIG as usize {
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
#[inline(always)]
unsafe fn ptr_token_eq(
    key: *const yyjson_val,
    token: *const c_char,
    len: usize,
    esc: usize,
) -> bool {
    if unsafe_get_len(key) != len {
        return false;
    }
    if esc == 0 {
        mem::memcmp_eq((*key).uni.str_ as *const u8, token as *const u8, len)
    } else {
        let mut str_ = (*key).uni.str_;
        let mut token = token;
        let mut len = len;
        while len > 0 {
            len -= 1;
            if *token == b'~' as c_char {
                token = token.add(1);
                let want = if *token == b'0' as c_char {
                    b'~' as c_char
                } else {
                    b'/' as c_char
                };
                if *str_ != want {
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
#[inline(always)]
unsafe fn ptr_arr_get(
    arr: *const yyjson_val,
    token: *const c_char,
    len: usize,
    _esc: usize,
) -> *mut yyjson_val {
    let mut val = unsafe_get_first(arr);
    let num = unsafe_get_len(arr);
    let mut idx: usize = 0;
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
            val = unsafe_get_next(val);
        }
        val
    }
}

/// `ptr_obj_get`
#[inline(always)]
unsafe fn ptr_obj_get(
    obj: *const yyjson_val,
    token: *const c_char,
    len: usize,
    esc: usize,
) -> *mut yyjson_val {
    let mut key = unsafe_get_first(obj);
    let mut num = unsafe_get_len(obj);
    if num == 0 {
        return ptr::null_mut();
    }
    while num > 0 {
        if ptr_token_eq(key, token, len, esc) {
            return key.add(1);
        }
        num -= 1;
        key = unsafe_get_next(key.add(1));
    }
    ptr::null_mut()
}

/// `ptr_mut_arr_get`
#[inline(always)]
unsafe fn ptr_mut_arr_get(
    arr: *const yyjson_mut_val,
    token: *const c_char,
    len: usize,
    _esc: usize,
    pre: *mut *mut yyjson_mut_val,
    last: *mut bool,
) -> *mut yyjson_mut_val {
    let mut val = (*arr).uni.ptr as *mut yyjson_mut_val; /* last (tail) */
    let num = mut_get_len(arr);
    let mut idx: usize = 0;
    if !last.is_null() {
        *last = false;
    }
    if !pre.is_null() {
        *pre = ptr::null_mut();
    }
    if num == 0 {
        if !last.is_null() && len == 1 && (*token == b'0' as c_char || *token == b'-' as c_char) {
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
#[inline(always)]
unsafe fn ptr_mut_obj_get(
    obj: *const yyjson_mut_val,
    token: *const c_char,
    len: usize,
    esc: usize,
    pre: *mut *mut yyjson_mut_val,
) -> *mut yyjson_mut_val {
    let mut pre_key = (*obj).uni.ptr as *mut yyjson_mut_val;
    let mut num = mut_get_len(obj);
    if !pre.is_null() {
        *pre = ptr::null_mut();
    }
    if num == 0 {
        return ptr::null_mut();
    }
    while num > 0 {
        let key = (*(*pre_key).next).next;
        if ptr_token_eq(key as *const yyjson_val, token, len, esc) {
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
#[inline(always)]
unsafe fn ptr_new_key(
    token: *const c_char,
    len: usize,
    esc: usize,
    doc: *mut yyjson_mut_doc,
) -> *mut yyjson_mut_val {
    let mut src = token;
    if esc == 0 {
        new_mut_strncpy(doc, src, len)
    } else {
        let end = src.add(len + esc);
        let mut dst = mut_str_alc(doc, len + esc);
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
        new_mut_strn(doc, str_, len)
    }
}

/* ------------------------------------------------------------- pointer */

macro_rules! set_err {
    ($err:expr, $code:expr, $pos:expr, $msg:expr) => {{
        if !$err.is_null() {
            (*$err).code = $code;
            (*$err).msg = $msg;
            (*$err).pos = $pos;
        }
    }};
}

const MSG_RESOLVE: &[u8] = b"JSON pointer cannot be resolved\0";
const MSG_SYNTAX: &[u8] = b"invalid escaped character\0";
const MSG_ALLOC: &[u8] = b"failed to create value\0";
const MSG_NO_PREFIX: &[u8] = b"no prefix '/'\0";
const MSG_PARAM: &[u8] = b"input parameter is NULL\0";
const MSG_SET_ROOT: &[u8] = b"cannot set root\0";

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_ptr_getx(
    val: *const yyjson_val,
    ptr_: *const c_char,
    ptr_len: usize,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_val {
    let hdr = ptr_;
    let end = ptr_.add(ptr_len);
    let mut p = ptr_;
    let mut val = val;
    let mut len: usize = 0;
    let mut esc: usize = 0;

    loop {
        let token = ptr_next_token(&mut p, end, &mut len, &mut esc);
        if token.is_null() {
            set_err!(
                err,
                YYJSON_PTR_ERR_SYNTAX,
                p as usize - hdr as usize,
                crate::util::msg(MSG_SYNTAX)
            );
            return ptr::null_mut();
        }
        let ty = unsafe_get_type(val);
        val = if ty == YYJSON_TYPE_OBJ {
            ptr_obj_get(val, token, len, esc)
        } else if ty == YYJSON_TYPE_ARR {
            ptr_arr_get(val, token, len, esc)
        } else {
            ptr::null_mut()
        };
        if val.is_null() {
            set_err!(
                err,
                YYJSON_PTR_ERR_RESOLVE,
                token as usize - hdr as usize,
                crate::util::msg(MSG_RESOLVE)
            );
            return ptr::null_mut();
        }
        if p == end {
            return val as *mut yyjson_val;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_getx(
    val: *const yyjson_mut_val,
    ptr_: *const c_char,
    ptr_len: usize,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    let hdr = ptr_;
    let end = ptr_.add(ptr_len);
    let mut p = ptr_;
    let mut val = val;
    let mut len: usize = 0;
    let mut esc: usize = 0;
    let mut pre: *mut yyjson_mut_val = ptr::null_mut();
    let mut idx_is_last = false;

    loop {
        let token = ptr_next_token(&mut p, end, &mut len, &mut esc);
        if token.is_null() {
            set_err!(
                err,
                YYJSON_PTR_ERR_SYNTAX,
                p as usize - hdr as usize,
                crate::util::msg(MSG_SYNTAX)
            );
            return ptr::null_mut();
        }
        let ctn = val as *mut yyjson_mut_val;
        let ty = mut_get_type(val);
        val = if ty == YYJSON_TYPE_OBJ {
            ptr_mut_obj_get(val, token, len, esc, &mut pre)
        } else if ty == YYJSON_TYPE_ARR {
            ptr_mut_arr_get(val, token, len, esc, &mut pre, &mut idx_is_last)
        } else {
            ptr::null_mut()
        };
        if !ctx.is_null()
            && p == end
            && (ty == YYJSON_TYPE_OBJ
                || (ty == YYJSON_TYPE_ARR && (!val.is_null() || idx_is_last)))
        {
            (*ctx).ctn = ctn;
            (*ctx).pre = pre;
        }
        if val.is_null() {
            set_err!(
                err,
                YYJSON_PTR_ERR_RESOLVE,
                token as usize - hdr as usize,
                crate::util::msg(MSG_RESOLVE)
            );
            return ptr::null_mut();
        }
        if p == end {
            return val as *mut yyjson_mut_val;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_putx(
    val: *mut yyjson_mut_val,
    ptr_: *const c_char,
    ptr_len: usize,
    new_val: *mut yyjson_mut_val,
    doc: *mut yyjson_mut_doc,
    create_parent: bool,
    insert_new: bool,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> bool {
    let hdr = ptr_;
    let end = ptr_.add(ptr_len);
    let mut p = ptr_;
    let mut val = val;
    let mut token_len: usize = 0;
    let mut esc: usize = 0;
    let mut ctn: *mut yyjson_mut_val;
    let mut key: *mut yyjson_mut_val;
    let mut pre: *mut yyjson_mut_val = ptr::null_mut();
    let mut sep_ctn: *mut yyjson_mut_val = ptr::null_mut();
    let mut sep_key: *mut yyjson_mut_val = ptr::null_mut();
    let mut sep_val: *mut yyjson_mut_val = ptr::null_mut();
    let mut ctn_type: u8;
    let mut idx_is_last = false;
    let mut token: *const c_char;

    macro_rules! err_resolve {
        ($pos:expr) => {{
            set_err!(
                err,
                YYJSON_PTR_ERR_RESOLVE,
                $pos,
                crate::util::msg(MSG_RESOLVE)
            );
            return false;
        }};
    }
    macro_rules! err_syntax {
        ($pos:expr) => {{
            set_err!(err, YYJSON_PTR_ERR_SYNTAX, $pos, crate::util::msg(MSG_SYNTAX));
            return false;
        }};
    }
    macro_rules! err_alloc {
        () => {{
            set_err!(
                err,
                YYJSON_PTR_ERR_MEMORY_ALLOCATION,
                0,
                crate::util::msg(MSG_ALLOC)
            );
            return false;
        }};
    }

    /* skip exist parent nodes */
    loop {
        token = ptr_next_token(&mut p, end, &mut token_len, &mut esc);
        if token.is_null() {
            err_syntax!(p as usize - hdr as usize);
        }
        ctn = val;
        ctn_type = mut_get_type(ctn);
        if ctn_type == YYJSON_TYPE_OBJ {
            val = ptr_mut_obj_get(ctn, token, token_len, esc, &mut pre);
        } else if ctn_type == YYJSON_TYPE_ARR {
            val = ptr_mut_arr_get(ctn, token, token_len, esc, &mut pre, &mut idx_is_last);
        } else {
            err_resolve!(token as usize - hdr as usize);
        }
        if val.is_null() {
            break;
        }
        if p == end {
            break; /* is last token */
        }
    }

    /* create parent nodes if not exist */
    if p != end {
        /* not last token */
        if !create_parent {
            err_resolve!(token as usize - hdr as usize);
        }

        /* add value at last index if container is array */
        if ctn_type == YYJSON_TYPE_ARR {
            if !idx_is_last || !insert_new {
                err_resolve!(token as usize - hdr as usize);
            }
            val = new_mut_obj(doc);
            if val.is_null() {
                err_alloc!();
            }

            /* delay attaching until all operations are completed */
            sep_ctn = ctn;
            sep_key = ptr::null_mut();
            sep_val = val;

            /* move to next token */
            ctn = val;
            val = ptr::null_mut();
            ctn_type = YYJSON_TYPE_OBJ;
            token = ptr_next_token(&mut p, end, &mut token_len, &mut esc);
            if token.is_null() {
                err_syntax!(p as usize - hdr as usize);
            }
        }

        /* container is object, create parent nodes */
        while p != end {
            key = ptr_new_key(token, token_len, esc, doc);
            if key.is_null() {
                err_alloc!();
            }
            val = new_mut_obj(doc);
            if val.is_null() {
                err_alloc!();
            }

            if sep_ctn.is_null() {
                sep_ctn = ctn;
                sep_key = key;
                sep_val = val;
            } else {
                mut_obj_add(ctn, key, val);
            }

            ctn = val;
            val = ptr::null_mut();
            token = ptr_next_token(&mut p, end, &mut token_len, &mut esc);
            if token.is_null() {
                err_syntax!(p as usize - hdr as usize);
            }
        }
    }

    /* JSON pointer is resolved, insert or replace target value */
    let ctn_len = mut_get_len(ctn);
    if ctn_type == YYJSON_TYPE_OBJ {
        if !ctx.is_null() {
            (*ctx).ctn = ctn;
        }
        if val.is_null() || insert_new {
            /* insert new key-value pair */
            key = ptr_new_key(token, token_len, esc, doc);
            if key.is_null() {
                err_alloc!();
            }
            if !ctx.is_null() {
                (*ctx).pre = if ctn_len != 0 {
                    (*ctn).uni.ptr as *mut yyjson_mut_val
                } else {
                    key
                };
            }
            unsafe_mut_obj_add(ctn, key, new_val, ctn_len);
        } else {
            /* replace exist value */
            key = (*(*pre).next).next;
            if !ctx.is_null() {
                (*ctx).pre = pre;
                (*ctx).old = val;
            }
            mut_obj_put(ctn, key, new_val);
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
                mut_set_len(ctn, ctn_len + 1);
            } else if idx_is_last {
                if !ctx.is_null() {
                    (*ctx).pre = if ctn_len != 0 {
                        (*ctn).uni.ptr as *mut yyjson_mut_val
                    } else {
                        new_val
                    };
                }
                mut_arr_append(ctn, new_val);
            } else {
                err_resolve!(token as usize - hdr as usize);
            }
        } else {
            /* replace exist value */
            if val.is_null() {
                err_resolve!(token as usize - hdr as usize);
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
            mut_obj_add(sep_ctn, sep_key, sep_val);
        } else {
            mut_arr_append(sep_ctn, sep_val);
        }
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_replacex(
    val: *mut yyjson_mut_val,
    ptr_: *const c_char,
    len: usize,
    new_val: *mut yyjson_mut_val,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    let mut cur_ctx = yyjson_ptr_ctx {
        ctn: ptr::null_mut(),
        pre: ptr::null_mut(),
        old: ptr::null_mut(),
    };
    let ctx = if ctx.is_null() {
        &mut cur_ctx as *mut yyjson_ptr_ctx
    } else {
        ctx
    };
    let cur_val = unsafe_yyjson_mut_ptr_getx(val, ptr_, len, ctx, err);
    if cur_val.is_null() {
        return ptr::null_mut();
    }

    if mut_is_obj((*ctx).ctn) {
        let key = (*(*(*ctx).pre).next).next;
        mut_obj_put((*ctx).ctn, key, new_val);
    } else {
        ptr_ctx_replace(ctx, new_val);
    }
    (*ctx).old = cur_val;
    cur_val
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_removex(
    val: *mut yyjson_mut_val,
    ptr_: *const c_char,
    len: usize,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    let mut cur_ctx = yyjson_ptr_ctx {
        ctn: ptr::null_mut(),
        pre: ptr::null_mut(),
        old: ptr::null_mut(),
    };
    let ctx = if ctx.is_null() {
        &mut cur_ctx as *mut yyjson_ptr_ctx
    } else {
        ctx
    };
    let cur_val = unsafe_yyjson_mut_ptr_getx(val, ptr_, len, ctx, err);
    if !cur_val.is_null() {
        if mut_is_obj((*ctx).ctn) {
            let key = (*(*(*ctx).pre).next).next;
            mut_obj_put((*ctx).ctn, key, ptr::null_mut());
        } else {
            ptr_ctx_remove(ctx);
        }
        (*ctx).pre = ptr::null_mut();
        (*ctx).old = cur_val;
    }
    cur_val
}

/* ------------- the header's inline ptr wrappers, for internal use ------- */

/// `yyjson_mut_ptr_getx`
unsafe fn mut_ptr_getx(
    val: *const yyjson_mut_val,
    p: *const c_char,
    len: usize,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    set_err!(err, YYJSON_PTR_ERR_NONE, 0, ptr::null());
    if !ctx.is_null() {
        mem::set(ctx as *mut u8, 0, size_of::<yyjson_ptr_ctx>());
    }
    if val.is_null() || p.is_null() {
        set_err!(err, YYJSON_PTR_ERR_PARAMETER, 0, crate::util::msg(MSG_PARAM));
        return ptr::null_mut();
    }
    if len == 0 {
        return val as *mut yyjson_mut_val;
    }
    if *p != b'/' as c_char {
        set_err!(err, YYJSON_PTR_ERR_SYNTAX, 0, crate::util::msg(MSG_NO_PREFIX));
        return ptr::null_mut();
    }
    unsafe_yyjson_mut_ptr_getx(val, p, len, ctx, err)
}

/// `yyjson_mut_ptr_addx`
unsafe fn mut_ptr_addx(
    val: *mut yyjson_mut_val,
    p: *const c_char,
    len: usize,
    new_val: *mut yyjson_mut_val,
    doc: *mut yyjson_mut_doc,
    create_parent: bool,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> bool {
    set_err!(err, YYJSON_PTR_ERR_NONE, 0, ptr::null());
    if !ctx.is_null() {
        mem::set(ctx as *mut u8, 0, size_of::<yyjson_ptr_ctx>());
    }
    if val.is_null() || p.is_null() || new_val.is_null() || doc.is_null() {
        set_err!(err, YYJSON_PTR_ERR_PARAMETER, 0, crate::util::msg(MSG_PARAM));
        return false;
    }
    if len == 0 {
        set_err!(err, YYJSON_PTR_ERR_SET_ROOT, 0, crate::util::msg(MSG_SET_ROOT));
        return false;
    }
    if *p != b'/' as c_char {
        set_err!(err, YYJSON_PTR_ERR_SYNTAX, 0, crate::util::msg(MSG_NO_PREFIX));
        return false;
    }
    unsafe_yyjson_mut_ptr_putx(val, p, len, new_val, doc, create_parent, true, ctx, err)
}

/// `yyjson_mut_ptr_removex`
unsafe fn mut_ptr_removex(
    val: *mut yyjson_mut_val,
    p: *const c_char,
    len: usize,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    set_err!(err, YYJSON_PTR_ERR_NONE, 0, ptr::null());
    if !ctx.is_null() {
        mem::set(ctx as *mut u8, 0, size_of::<yyjson_ptr_ctx>());
    }
    if val.is_null() || p.is_null() {
        set_err!(err, YYJSON_PTR_ERR_PARAMETER, 0, crate::util::msg(MSG_PARAM));
        return ptr::null_mut();
    }
    if len == 0 {
        set_err!(err, YYJSON_PTR_ERR_SET_ROOT, 0, crate::util::msg(MSG_SET_ROOT));
        return ptr::null_mut();
    }
    if *p != b'/' as c_char {
        set_err!(err, YYJSON_PTR_ERR_SYNTAX, 0, crate::util::msg(MSG_NO_PREFIX));
        return ptr::null_mut();
    }
    unsafe_yyjson_mut_ptr_removex(val, p, len, ctx, err)
}

/// `yyjson_mut_ptr_replacex`
unsafe fn mut_ptr_replacex(
    val: *mut yyjson_mut_val,
    p: *const c_char,
    len: usize,
    new_val: *mut yyjson_mut_val,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    set_err!(err, YYJSON_PTR_ERR_NONE, 0, ptr::null());
    if !ctx.is_null() {
        mem::set(ctx as *mut u8, 0, size_of::<yyjson_ptr_ctx>());
    }
    if val.is_null() || p.is_null() || new_val.is_null() {
        set_err!(err, YYJSON_PTR_ERR_PARAMETER, 0, crate::util::msg(MSG_PARAM));
        return ptr::null_mut();
    }
    if len == 0 {
        set_err!(err, YYJSON_PTR_ERR_SET_ROOT, 0, crate::util::msg(MSG_SET_ROOT));
        return ptr::null_mut();
    }
    if *p != b'/' as c_char {
        set_err!(err, YYJSON_PTR_ERR_SYNTAX, 0, crate::util::msg(MSG_NO_PREFIX));
        return ptr::null_mut();
    }
    unsafe_yyjson_mut_ptr_replacex(val, p, len, new_val, ctx, err)
}

/* ------------------------------------------------------------- patch */

/// `patch_op`
#[derive(Copy, Clone, PartialEq)]
enum PatchOp {
    Add,
    Remove,
    Replace,
    Move,
    Copy,
    Test,
    None,
}

/// `patch_op_get`
unsafe fn patch_op_get(op: *const yyjson_val) -> PatchOp {
    let s = (*op).uni.str_ as *const u8;
    match unsafe_get_len(op) {
        3 => {
            if mem::memcmp_eq(s, b"add".as_ptr(), 3) {
                PatchOp::Add
            } else {
                PatchOp::None
            }
        }
        4 => {
            if mem::memcmp_eq(s, b"move".as_ptr(), 4) {
                PatchOp::Move
            } else if mem::memcmp_eq(s, b"copy".as_ptr(), 4) {
                PatchOp::Copy
            } else if mem::memcmp_eq(s, b"test".as_ptr(), 4) {
                PatchOp::Test
            } else {
                PatchOp::None
            }
        }
        6 => {
            if mem::memcmp_eq(s, b"remove".as_ptr(), 6) {
                PatchOp::Remove
            } else {
                PatchOp::None
            }
        }
        7 => {
            if mem::memcmp_eq(s, b"replace".as_ptr(), 7) {
                PatchOp::Replace
            } else {
                PatchOp::None
            }
        }
        _ => PatchOp::None,
    }
}

/// The `return_err` macro of `yyjson_patch`: a pending pointer-level
/// allocation failure is reported as a patch-level allocation failure and
/// clears the nested pointer error.
#[inline(always)]
unsafe fn patch_set_err(
    err: *mut yyjson_patch_err,
    code: u32,
    msg: *const c_char,
    iter_idx: usize,
) {
    if (*err).ptr.code == YYJSON_PTR_ERR_MEMORY_ALLOCATION {
        (*err).code = YYJSON_PATCH_ERROR_MEMORY_ALLOCATION;
        (*err).msg = msg;
        mem::set(
            &mut (*err).ptr as *mut yyjson_ptr_err as *mut u8,
            0,
            size_of::<yyjson_ptr_err>(),
        );
    } else {
        (*err).code = code;
        (*err).msg = msg;
        (*err).idx = if iter_idx != 0 { iter_idx - 1 } else { 0 };
    }
}

const PATCH_ERR_TMP: yyjson_patch_err = yyjson_patch_err {
    code: 0,
    idx: 0,
    msg: ptr::null(),
    ptr: yyjson_ptr_err {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    },
};

#[no_mangle]
pub unsafe extern "C" fn yyjson_patch(
    doc: *mut yyjson_mut_doc,
    orig: *const yyjson_val,
    patch: *const yyjson_val,
    err: *mut yyjson_patch_err,
) -> *mut yyjson_mut_val {
    let mut err_tmp = PATCH_ERR_TMP;
    let err = if err.is_null() {
        &mut err_tmp as *mut yyjson_patch_err
    } else {
        err
    };
    mem::set(err as *mut u8, 0, size_of::<yyjson_patch_err>());

    let mut iter_idx: usize = 0;

    macro_rules! fail {
        ($code:expr, $m:expr) => {{
            patch_set_err(err, $code, $m.as_ptr() as *const c_char, iter_idx);
            return ptr::null_mut();
        }};
    }
    macro_rules! fail_copy {
        () => {
            fail!(
                YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                b"failed to copy value\0"
            )
        };
    }

    if doc.is_null() || orig.is_null() || patch.is_null() {
        fail!(
            YYJSON_PATCH_ERROR_INVALID_PARAMETER,
            b"input parameter is NULL\0"
        );
    }
    if !is_arr(patch) {
        fail!(
            YYJSON_PATCH_ERROR_INVALID_PARAMETER,
            b"input patch is not array\0"
        );
    }
    let mut root = crate::doc::yyjson_val_mut_copy(doc, orig);
    if root.is_null() {
        fail_copy!();
    }

    /* iterate through the patch array (`yyjson_arr_iter`) */
    let iter_max = unsafe_get_len(patch);
    let mut iter_cur = unsafe_get_first(patch);
    while iter_idx < iter_max {
        let obj = iter_cur;
        iter_cur = unsafe_get_next(obj);
        iter_idx += 1;

        if unsafe_get_type(obj) != YYJSON_TYPE_OBJ {
            fail!(
                YYJSON_PATCH_ERROR_INVALID_OPERATION,
                b"JSON patch operation is not object\0"
            );
        }

        /* get required member: op */
        let op = obj_get_lit(obj, b"op\0");
        if op.is_null() {
            fail!(YYJSON_PATCH_ERROR_MISSING_KEY, b"missing key `op`\0");
        }
        if !is_str(op) {
            fail!(YYJSON_PATCH_ERROR_INVALID_MEMBER, b"invalid member `op`\0");
        }
        let op_enum = patch_op_get(op);

        /* get required member: path */
        let path = obj_get_lit(obj, b"path\0");
        if path.is_null() {
            fail!(YYJSON_PATCH_ERROR_MISSING_KEY, b"missing key `path`\0");
        }
        if !is_str(path) {
            fail!(YYJSON_PATCH_ERROR_INVALID_MEMBER, b"invalid member `path`\0");
        }
        let path_len = unsafe_get_len(path);

        /* get required member: value, from */
        let mut val: *mut yyjson_mut_val = ptr::null_mut();
        let mut from: *mut yyjson_val = ptr::null_mut();
        let mut from_len: usize = 0;
        match op_enum {
            PatchOp::Add | PatchOp::Replace | PatchOp::Test => {
                let value = obj_get_lit(obj, b"value\0");
                if value.is_null() {
                    fail!(YYJSON_PATCH_ERROR_MISSING_KEY, b"missing key `value`\0");
                }
                val = crate::doc::yyjson_val_mut_copy(doc, value);
                if val.is_null() {
                    fail_copy!();
                }
            }
            PatchOp::Move | PatchOp::Copy => {
                from = obj_get_lit(obj, b"from\0");
                if from.is_null() {
                    fail!(YYJSON_PATCH_ERROR_MISSING_KEY, b"missing key `from`\0");
                }
                if !is_str(from) {
                    fail!(
                        YYJSON_PATCH_ERROR_INVALID_MEMBER,
                        b"invalid member `from`\0"
                    );
                }
                from_len = unsafe_get_len(from);
            }
            _ => {}
        }

        /* perform an operation */
        match op_enum {
            PatchOp::Add => {
                if path_len == 0 {
                    root = val;
                } else if !mut_ptr_addx(
                    root,
                    (*path).uni.str_,
                    path_len,
                    val,
                    doc,
                    false,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                ) {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to add `path`\0");
                }
            }
            PatchOp::Remove => {
                if mut_ptr_removex(
                    root,
                    (*path).uni.str_,
                    path_len,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                )
                .is_null()
                {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to remove `path`\0");
                }
            }
            PatchOp::Replace => {
                if path_len == 0 {
                    root = val;
                } else if mut_ptr_replacex(
                    root,
                    (*path).uni.str_,
                    path_len,
                    val,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                )
                .is_null()
                {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to replace `path`\0");
                }
            }
            PatchOp::Move => {
                if from_len == 0 && path_len == 0 {
                    continue;
                }
                val = mut_ptr_removex(
                    root,
                    (*from).uni.str_,
                    from_len,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                );
                if val.is_null() {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to remove `from`\0");
                }
                if path_len == 0 {
                    root = val;
                } else if !mut_ptr_addx(
                    root,
                    (*path).uni.str_,
                    path_len,
                    val,
                    doc,
                    false,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                ) {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to add `path`\0");
                }
            }
            PatchOp::Copy => {
                val = mut_ptr_getx(
                    root,
                    (*from).uni.str_,
                    from_len,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                );
                if val.is_null() {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to get `from`\0");
                }
                if path_len == 0 {
                    root = val;
                } else {
                    val = crate::doc::yyjson_mut_val_mut_copy(doc, val);
                    if val.is_null() {
                        fail_copy!();
                    }
                    if !mut_ptr_addx(
                        root,
                        (*path).uni.str_,
                        path_len,
                        val,
                        doc,
                        false,
                        ptr::null_mut(),
                        &mut (*err).ptr,
                    ) {
                        fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to add `path`\0");
                    }
                }
            }
            PatchOp::Test => {
                let test = mut_ptr_getx(
                    root,
                    (*path).uni.str_,
                    path_len,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                );
                if test.is_null() {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to get `path`\0");
                }
                if !mut_equals(val, test) {
                    fail!(YYJSON_PATCH_ERROR_EQUAL, b"failed to test equal\0");
                }
            }
            PatchOp::None => {
                fail!(YYJSON_PATCH_ERROR_INVALID_MEMBER, b"unsupported `op`\0");
            }
        }
    }
    root
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_patch(
    doc: *mut yyjson_mut_doc,
    orig: *const yyjson_mut_val,
    patch: *const yyjson_mut_val,
    err: *mut yyjson_patch_err,
) -> *mut yyjson_mut_val {
    let mut err_tmp = PATCH_ERR_TMP;
    let err = if err.is_null() {
        &mut err_tmp as *mut yyjson_patch_err
    } else {
        err
    };
    mem::set(err as *mut u8, 0, size_of::<yyjson_patch_err>());

    let mut iter_idx: usize = 0;

    macro_rules! fail {
        ($code:expr, $m:expr) => {{
            patch_set_err(err, $code, $m.as_ptr() as *const c_char, iter_idx);
            return ptr::null_mut();
        }};
    }
    macro_rules! fail_copy {
        () => {
            fail!(
                YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                b"failed to copy value\0"
            )
        };
    }

    if doc.is_null() || orig.is_null() || patch.is_null() {
        fail!(
            YYJSON_PATCH_ERROR_INVALID_PARAMETER,
            b"input parameter is NULL\0"
        );
    }
    if !mut_is_arr(patch) {
        fail!(
            YYJSON_PATCH_ERROR_INVALID_PARAMETER,
            b"input patch is not array\0"
        );
    }
    let mut root = crate::doc::yyjson_mut_val_mut_copy(doc, orig);
    if root.is_null() {
        fail_copy!();
    }

    /* iterate through the patch array (`yyjson_mut_arr_iter`, whose `next`
       returns the cursor *after* advancing it) */
    let iter_max = mut_get_len(patch);
    let mut iter_cur: *mut yyjson_mut_val = if iter_max != 0 {
        (*patch).uni.ptr as *mut yyjson_mut_val
    } else {
        ptr::null_mut()
    };
    while iter_idx < iter_max {
        iter_cur = (*iter_cur).next;
        iter_idx += 1;
        let obj = iter_cur;

        if mut_get_type(obj) != YYJSON_TYPE_OBJ {
            fail!(
                YYJSON_PATCH_ERROR_INVALID_OPERATION,
                b"JSON patch operation is not object\0"
            );
        }

        let op = mut_obj_get_lit(obj, b"op\0");
        if op.is_null() {
            fail!(YYJSON_PATCH_ERROR_MISSING_KEY, b"missing key `op`\0");
        }
        if !mut_is_str(op) {
            fail!(YYJSON_PATCH_ERROR_INVALID_MEMBER, b"invalid member `op`\0");
        }
        let op_enum = patch_op_get(op as *const yyjson_val);

        let path = mut_obj_get_lit(obj, b"path\0");
        if path.is_null() {
            fail!(YYJSON_PATCH_ERROR_MISSING_KEY, b"missing key `path`\0");
        }
        if !mut_is_str(path) {
            fail!(YYJSON_PATCH_ERROR_INVALID_MEMBER, b"invalid member `path`\0");
        }
        let path_len = mut_get_len(path);

        let mut val: *mut yyjson_mut_val = ptr::null_mut();
        let mut from: *mut yyjson_mut_val = ptr::null_mut();
        let mut from_len: usize = 0;
        match op_enum {
            PatchOp::Add | PatchOp::Replace | PatchOp::Test => {
                let value = mut_obj_get_lit(obj, b"value\0");
                if value.is_null() {
                    fail!(YYJSON_PATCH_ERROR_MISSING_KEY, b"missing key `value`\0");
                }
                val = crate::doc::yyjson_mut_val_mut_copy(doc, value);
                if val.is_null() {
                    fail_copy!();
                }
            }
            PatchOp::Move | PatchOp::Copy => {
                from = mut_obj_get_lit(obj, b"from\0");
                if from.is_null() {
                    fail!(YYJSON_PATCH_ERROR_MISSING_KEY, b"missing key `from`\0");
                }
                if !mut_is_str(from) {
                    fail!(
                        YYJSON_PATCH_ERROR_INVALID_MEMBER,
                        b"invalid member `from`\0"
                    );
                }
                from_len = mut_get_len(from);
            }
            _ => {}
        }

        match op_enum {
            PatchOp::Add => {
                if path_len == 0 {
                    root = val;
                } else if !mut_ptr_addx(
                    root,
                    (*path).uni.str_,
                    path_len,
                    val,
                    doc,
                    false,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                ) {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to add `path`\0");
                }
            }
            PatchOp::Remove => {
                if mut_ptr_removex(
                    root,
                    (*path).uni.str_,
                    path_len,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                )
                .is_null()
                {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to remove `path`\0");
                }
            }
            PatchOp::Replace => {
                if path_len == 0 {
                    root = val;
                } else if mut_ptr_replacex(
                    root,
                    (*path).uni.str_,
                    path_len,
                    val,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                )
                .is_null()
                {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to replace `path`\0");
                }
            }
            PatchOp::Move => {
                if from_len == 0 && path_len == 0 {
                    continue;
                }
                val = mut_ptr_removex(
                    root,
                    (*from).uni.str_,
                    from_len,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                );
                if val.is_null() {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to remove `from`\0");
                }
                if path_len == 0 {
                    root = val;
                } else if !mut_ptr_addx(
                    root,
                    (*path).uni.str_,
                    path_len,
                    val,
                    doc,
                    false,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                ) {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to add `path`\0");
                }
            }
            PatchOp::Copy => {
                val = mut_ptr_getx(
                    root,
                    (*from).uni.str_,
                    from_len,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                );
                if val.is_null() {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to get `from`\0");
                }
                if path_len == 0 {
                    root = val;
                } else {
                    val = crate::doc::yyjson_mut_val_mut_copy(doc, val);
                    if val.is_null() {
                        fail_copy!();
                    }
                    if !mut_ptr_addx(
                        root,
                        (*path).uni.str_,
                        path_len,
                        val,
                        doc,
                        false,
                        ptr::null_mut(),
                        &mut (*err).ptr,
                    ) {
                        fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to add `path`\0");
                    }
                }
            }
            PatchOp::Test => {
                let test = mut_ptr_getx(
                    root,
                    (*path).uni.str_,
                    path_len,
                    ptr::null_mut(),
                    &mut (*err).ptr,
                );
                if test.is_null() {
                    fail!(YYJSON_PATCH_ERROR_POINTER, b"failed to get `path`\0");
                }
                if !mut_equals(val, test) {
                    fail!(YYJSON_PATCH_ERROR_EQUAL, b"failed to test equal\0");
                }
            }
            PatchOp::None => {
                fail!(YYJSON_PATCH_ERROR_INVALID_MEMBER, b"unsupported `op`\0");
            }
        }
    }
    root
}

/* ------------------------------------------------------- merge patch */

#[no_mangle]
pub unsafe extern "C" fn yyjson_merge_patch(
    doc: *mut yyjson_mut_doc,
    orig: *const yyjson_val,
    patch: *const yyjson_val,
) -> *mut yyjson_mut_val {
    if !is_obj(patch) {
        return crate::doc::yyjson_val_mut_copy(doc, patch);
    }

    let builder = new_mut_obj(doc);
    if builder.is_null() {
        return ptr::null_mut();
    }

    /* `local_orig` stands in for a non-object `orig`: an empty object that
       makes every lookup miss. Its payload is a verbatim copy of the fresh
       builder's, exactly as in the C source. */
    let mut local_orig = yyjson_val {
        tag: 0,
        uni: yyjson_val_uni::ZERO,
    };
    let mut orig = orig;
    let local_is_used = !is_obj(orig);
    if local_is_used {
        local_orig.tag = (*builder).tag;
        local_orig.uni.u64_ = (*builder).uni.u64_;
        orig = &local_orig as *const yyjson_val;
    }

    /* if orig is contributing, copy any items not modified by the patch */
    if !local_is_used {
        let max = unsafe_get_len(orig);
        let mut key = unsafe_get_first(orig);
        let mut idx = 0usize;
        while idx < max {
            let orig_val = key.add(1);
            let patch_val = obj_getn(patch, (*key).uni.str_, unsafe_get_len(key));
            if patch_val.is_null() {
                let mut_key = crate::doc::yyjson_val_mut_copy(doc, key);
                let mut_val_ = crate::doc::yyjson_val_mut_copy(doc, orig_val);
                if !mut_obj_add(builder, mut_key, mut_val_) {
                    return ptr::null_mut();
                }
            }
            idx += 1;
            key = unsafe_get_next(orig_val);
        }
    }

    /* merge items modified by the patch */
    let max = unsafe_get_len(patch);
    let mut key = unsafe_get_first(patch);
    let mut idx = 0usize;
    while idx < max {
        let patch_val = key.add(1);
        /* null indicates the field is removed */
        if unsafe_get_type(patch_val) != YYJSON_TYPE_NULL {
            let mut_key = crate::doc::yyjson_val_mut_copy(doc, key);
            let orig_val = obj_getn(orig, (*key).uni.str_, unsafe_get_len(key));
            let merged_val = yyjson_merge_patch(doc, orig_val, patch_val);
            if !mut_obj_add(builder, mut_key, merged_val) {
                return ptr::null_mut();
            }
        }
        idx += 1;
        key = unsafe_get_next(patch_val);
    }

    builder
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_merge_patch(
    doc: *mut yyjson_mut_doc,
    orig: *const yyjson_mut_val,
    patch: *const yyjson_mut_val,
) -> *mut yyjson_mut_val {
    if !mut_is_obj(patch) {
        return crate::doc::yyjson_mut_val_mut_copy(doc, patch);
    }

    let builder = new_mut_obj(doc);
    if builder.is_null() {
        return ptr::null_mut();
    }

    let mut local_orig = yyjson_mut_val {
        tag: 0,
        uni: yyjson_val_uni::ZERO,
        next: ptr::null_mut(),
    };
    let mut orig = orig;
    let local_is_used = !mut_is_obj(orig);
    if local_is_used {
        local_orig.tag = (*builder).tag;
        local_orig.uni.u64_ = (*builder).uni.u64_;
        orig = &local_orig as *const yyjson_mut_val;
    }

    if !local_is_used {
        let max = mut_get_len(orig);
        let mut key: *mut yyjson_mut_val = if max != 0 {
            (*(*((*orig).uni.ptr as *mut yyjson_mut_val)).next).next
        } else {
            ptr::null_mut()
        };
        let mut idx = 0usize;
        while idx < max {
            let orig_val = (*key).next;
            let patch_val = mut_obj_getn(patch, (*key).uni.str_, mut_get_len(key));
            if patch_val.is_null() {
                let mut_key = crate::doc::yyjson_mut_val_mut_copy(doc, key);
                let mut_val_ = crate::doc::yyjson_mut_val_mut_copy(doc, orig_val);
                if !mut_obj_add(builder, mut_key, mut_val_) {
                    return ptr::null_mut();
                }
            }
            idx += 1;
            key = (*orig_val).next;
        }
    }

    let max = mut_get_len(patch);
    let mut key: *mut yyjson_mut_val = if max != 0 {
        (*(*((*patch).uni.ptr as *mut yyjson_mut_val)).next).next
    } else {
        ptr::null_mut()
    };
    let mut idx = 0usize;
    while idx < max {
        let patch_val = (*key).next;
        if mut_get_type(patch_val) != YYJSON_TYPE_NULL {
            let mut_key = crate::doc::yyjson_mut_val_mut_copy(doc, key);
            let orig_val = mut_obj_getn(orig, (*key).uni.str_, mut_get_len(key));
            let merged_val = yyjson_mut_merge_patch(doc, orig_val, patch_val);
            if !mut_obj_add(builder, mut_key, merged_val) {
                return ptr::null_mut();
            }
        }
        idx += 1;
        key = (*patch_val).next;
    }

    builder
}
