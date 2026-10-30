//! Rust copies of the `static inline` functions in `src/yyjson.h`.
//!
//! These are *not* exported: the header still compiles its own copies into the
//! caller. They exist here because the exported functions in the original
//! `src/yyjson.c` call them, and the behaviour has to match exactly.

use core::ffi::{c_char, c_void};
use core::ptr;

use crate::libc;
use crate::types::*;

/* ===== pool allocation (header: unsafe_yyjson_mut_*) ==================== */

/// `unsafe_yyjson_mut_str_alc`
#[inline]
pub unsafe fn mut_str_alc(doc: *mut YyjsonMutDoc, len: usize) -> *mut c_char {
    let alc = &(*doc).alc as *const YyjsonAlc;
    let pool = &mut (*doc).str_pool as *mut YyjsonStrPool;
    if len == usize::MAX {
        return ptr::null_mut();
    }
    if ((*pool).end.offset_from((*pool).cur) as usize) <= len {
        if !crate::strct::unsafe_yyjson_str_pool_grow(pool, alc, len + 1) {
            return ptr::null_mut();
        }
    }
    let mem = (*pool).cur;
    (*pool).cur = mem.add(len + 1);
    mem
}

/// `unsafe_yyjson_mut_strncpy`
#[inline]
pub unsafe fn mut_strncpy(doc: *mut YyjsonMutDoc, str_: *const c_char, len: usize) -> *mut c_char {
    let mem = mut_str_alc(doc, len);
    if mem.is_null() {
        return ptr::null_mut();
    }
    libc::mem_copy(mem as *mut u8, str_ as *const u8, len);
    *mem.add(len) = 0;
    mem
}

/// `unsafe_yyjson_mut_val`
#[inline]
pub unsafe fn mut_val(doc: *mut YyjsonMutDoc, count: usize) -> *mut YyjsonMutVal {
    let alc = &(*doc).alc as *const YyjsonAlc;
    let pool = &mut (*doc).val_pool as *mut YyjsonValPool;
    if ((*pool).end.offset_from((*pool).cur) as usize) < count {
        if !crate::strct::unsafe_yyjson_val_pool_grow(pool, alc, count) {
            return ptr::null_mut();
        }
    }
    let val = (*pool).cur;
    (*pool).cur = val.add(count);
    val
}

/* ===== value creation =================================================== */

/// `yyjson_mut_obj`
#[inline]
pub unsafe fn mut_obj(doc: *mut YyjsonMutDoc) -> *mut YyjsonMutVal {
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
#[inline]
pub unsafe fn mut_strn(
    doc: *mut YyjsonMutDoc,
    str_: *const c_char,
    len: usize,
) -> *mut YyjsonMutVal {
    if doc.is_null() || str_.is_null() {
        return ptr::null_mut();
    }
    let val = mut_val(doc, 1);
    if val.is_null() {
        return ptr::null_mut();
    }
    (*val).tag = ((len as u64) << YYJSON_TAG_BIT) | (YYJSON_TYPE_STR | YYJSON_SUBTYPE_NONE) as u64;
    (*val).uni.str_ = str_;
    val
}

/// `yyjson_mut_strncpy`
#[inline]
pub unsafe fn mut_strncpy_val(
    doc: *mut YyjsonMutDoc,
    str_: *const c_char,
    len: usize,
) -> *mut YyjsonMutVal {
    if doc.is_null() || str_.is_null() {
        return ptr::null_mut();
    }
    let val = mut_val(doc, 1);
    if val.is_null() {
        return ptr::null_mut();
    }
    let new_str = mut_strncpy(doc, str_, len);
    if new_str.is_null() {
        return ptr::null_mut();
    }
    (*val).tag = ((len as u64) << YYJSON_TAG_BIT) | (YYJSON_TYPE_STR | YYJSON_SUBTYPE_NONE) as u64;
    (*val).uni.str_ = new_str;
    val
}

/* ===== immutable iterators ============================================== */

/// `yyjson_arr_iter_init`
#[inline]
pub unsafe fn arr_iter_init(arr: *const YyjsonVal, iter: *mut YyjsonArrIter) -> bool {
    if !arr.is_null() && get_type(arr) == YYJSON_TYPE_ARR && !iter.is_null() {
        (*iter).idx = 0;
        (*iter).max = get_len(arr);
        (*iter).cur = get_first(arr);
        return true;
    }
    if !iter.is_null() {
        libc::zero(iter);
    }
    false
}

/// `yyjson_arr_iter_next`
#[inline]
pub unsafe fn arr_iter_next(iter: *mut YyjsonArrIter) -> *mut YyjsonVal {
    if !iter.is_null() && (*iter).idx < (*iter).max {
        let val = (*iter).cur;
        (*iter).cur = get_next(val);
        (*iter).idx += 1;
        return val;
    }
    ptr::null_mut()
}

/// `yyjson_obj_iter_init`
#[inline]
pub unsafe fn obj_iter_init(obj: *const YyjsonVal, iter: *mut YyjsonObjIter) -> bool {
    if !obj.is_null() && get_type(obj) == YYJSON_TYPE_OBJ && !iter.is_null() {
        (*iter).idx = 0;
        (*iter).max = get_len(obj);
        (*iter).cur = get_first(obj);
        (*iter).obj = obj as *mut YyjsonVal;
        return true;
    }
    if !iter.is_null() {
        libc::zero(iter);
    }
    false
}

/// `yyjson_obj_iter_getn`
#[inline]
pub unsafe fn obj_iter_getn(
    iter: *mut YyjsonObjIter,
    key: *const c_char,
    key_len: usize,
) -> *mut YyjsonVal {
    if !iter.is_null() && !key.is_null() {
        let mut idx = (*iter).idx;
        let mut max = (*iter).max;
        let mut cur = (*iter).cur;
        if idx == max {
            idx = 0;
            cur = get_first((*iter).obj);
        }
        while idx < max {
            idx += 1;
            let next = get_next(cur.add(1));
            if equals_strn(cur, key, key_len) {
                (*iter).idx = idx;
                (*iter).cur = next;
                return cur.add(1);
            }
            cur = next;
            if idx == (*iter).max && (*iter).idx < (*iter).max {
                idx = 0;
                max = (*iter).idx;
                cur = get_first((*iter).obj);
            }
        }
    }
    ptr::null_mut()
}

/// `yyjson_obj_getn`
#[inline]
pub unsafe fn obj_getn(
    obj: *const YyjsonVal,
    key_: *const c_char,
    key_len: usize,
) -> *mut YyjsonVal {
    if !obj.is_null() && get_type(obj) == YYJSON_TYPE_OBJ && !key_.is_null() {
        let mut len = get_len(obj);
        let mut key = get_first(obj);
        while len > 0 {
            len -= 1;
            if equals_strn(key, key_, key_len) {
                return key.add(1);
            }
            key = get_next(key.add(1));
        }
    }
    ptr::null_mut()
}

/// `yyjson_obj_get`
#[inline]
pub unsafe fn obj_get(obj: *const YyjsonVal, key: &str) -> *mut YyjsonVal {
    obj_getn(obj, key.as_ptr() as *const c_char, key.len())
}

/* ===== mutable iterators ================================================ */

/// `yyjson_mut_arr_iter_init`
#[inline]
pub unsafe fn mut_arr_iter_init(arr: *mut YyjsonMutVal, iter: *mut YyjsonMutArrIter) -> bool {
    if !arr.is_null() && get_type(mut_as_val(arr)) == YYJSON_TYPE_ARR && !iter.is_null() {
        (*iter).idx = 0;
        (*iter).max = get_len(mut_as_val(arr));
        (*iter).cur = if (*iter).max != 0 {
            (*arr).uni.ptr as *mut YyjsonMutVal
        } else {
            ptr::null_mut()
        };
        (*iter).pre = ptr::null_mut();
        (*iter).arr = arr;
        return true;
    }
    if !iter.is_null() {
        libc::zero(iter);
    }
    false
}

/// `yyjson_mut_arr_iter_next`
#[inline]
pub unsafe fn mut_arr_iter_next(iter: *mut YyjsonMutArrIter) -> *mut YyjsonMutVal {
    if !iter.is_null() && (*iter).idx < (*iter).max {
        let val = (*iter).cur;
        (*iter).pre = val;
        (*iter).cur = (*val).next;
        (*iter).idx += 1;
        return (*iter).cur;
    }
    ptr::null_mut()
}

/// `yyjson_mut_obj_iter_init`
#[inline]
pub unsafe fn mut_obj_iter_init(obj: *mut YyjsonMutVal, iter: *mut YyjsonMutObjIter) -> bool {
    if !obj.is_null() && get_type(mut_as_val(obj)) == YYJSON_TYPE_OBJ && !iter.is_null() {
        (*iter).idx = 0;
        (*iter).max = get_len(mut_as_val(obj));
        (*iter).cur = if (*iter).max != 0 {
            (*obj).uni.ptr as *mut YyjsonMutVal
        } else {
            ptr::null_mut()
        };
        (*iter).pre = ptr::null_mut();
        (*iter).obj = obj;
        return true;
    }
    if !iter.is_null() {
        libc::zero(iter);
    }
    false
}

/// `yyjson_mut_obj_iter_next`
#[inline]
pub unsafe fn mut_obj_iter_next(iter: *mut YyjsonMutObjIter) -> *mut YyjsonMutVal {
    if !iter.is_null() && (*iter).idx < (*iter).max {
        let key = (*iter).cur;
        (*iter).pre = key;
        (*iter).cur = (*(*key).next).next;
        (*iter).idx += 1;
        return (*iter).cur;
    }
    ptr::null_mut()
}

/// `yyjson_mut_obj_iter_remove`
#[inline]
pub unsafe fn mut_obj_iter_remove(iter: *mut YyjsonMutObjIter) -> *mut YyjsonMutVal {
    if !iter.is_null() && !(*iter).pre.is_null() && 0 < (*iter).idx && (*iter).idx <= (*iter).max {
        let prev = (*iter).pre;
        let cur = (*iter).cur;
        let next = (*(*cur).next).next;
        if (*iter).idx == (*iter).max {
            (*(*iter).obj).uni.ptr = prev as *mut c_void;
        }
        (*iter).idx -= 1;
        (*iter).max -= 1;
        set_len(mut_as_val((*iter).obj), (*iter).max);
        (*(*prev).next).next = next;
        (*iter).cur = prev;
        (*iter).pre = ptr::null_mut();
        return (*cur).next;
    }
    ptr::null_mut()
}

/// `yyjson_mut_obj_iter_getn`
#[inline]
pub unsafe fn mut_obj_iter_getn(
    iter: *mut YyjsonMutObjIter,
    key: *const c_char,
    key_len: usize,
) -> *mut YyjsonMutVal {
    if !iter.is_null() && !key.is_null() {
        let mut idx = 0usize;
        let max = (*iter).max;
        let mut cur = (*iter).cur;
        while idx < max {
            idx += 1;
            let pre = cur;
            cur = (*(*cur).next).next;
            if equals_strn(mut_as_val(cur), key, key_len) {
                (*iter).idx += idx;
                if (*iter).idx > max {
                    (*iter).idx -= max;
                }
                (*iter).pre = pre;
                (*iter).cur = cur;
                return (*cur).next;
            }
        }
    }
    ptr::null_mut()
}

/// `yyjson_mut_obj_size`
#[inline]
pub unsafe fn mut_obj_size(obj: *const YyjsonMutVal) -> usize {
    if !obj.is_null() && get_type(mut_as_val(obj)) == YYJSON_TYPE_OBJ {
        get_len(mut_as_val(obj))
    } else {
        0
    }
}

/// `yyjson_mut_obj_getn`
#[inline]
pub unsafe fn mut_obj_getn(
    obj: *const YyjsonMutVal,
    key_: *const c_char,
    key_len: usize,
) -> *mut YyjsonMutVal {
    let mut len = mut_obj_size(obj);
    if len != 0 && !key_.is_null() {
        let mut key = (*((*obj).uni.ptr as *mut YyjsonMutVal)).next;
        key = (*key).next;
        while len > 0 {
            len -= 1;
            if equals_strn(mut_as_val(key), key_, key_len) {
                return (*key).next;
            }
            key = (*(*key).next).next;
        }
    }
    ptr::null_mut()
}

/// `yyjson_mut_obj_get`
#[inline]
pub unsafe fn mut_obj_get(obj: *const YyjsonMutVal, key: &str) -> *mut YyjsonMutVal {
    mut_obj_getn(obj, key.as_ptr() as *const c_char, key.len())
}

/* ===== mutable modification ============================================= */

/// `unsafe_yyjson_mut_obj_add`
#[inline]
pub unsafe fn unsafe_mut_obj_add(
    obj: *mut YyjsonMutVal,
    key: *mut YyjsonMutVal,
    val: *mut YyjsonMutVal,
    len: usize,
) {
    if len != 0 {
        let prev_val = (*((*obj).uni.ptr as *mut YyjsonMutVal)).next;
        let next_key = (*prev_val).next;
        (*prev_val).next = key;
        (*val).next = next_key;
    } else {
        (*val).next = key;
    }
    (*key).next = val;
    (*obj).uni.ptr = key as *mut c_void;
    set_len(mut_as_val(obj), len + 1);
}

/// `yyjson_mut_obj_add`
#[inline]
pub unsafe fn mut_obj_add(
    obj: *mut YyjsonMutVal,
    key: *mut YyjsonMutVal,
    val: *mut YyjsonMutVal,
) -> bool {
    if !obj.is_null()
        && get_type(mut_as_val(obj)) == YYJSON_TYPE_OBJ
        && !key.is_null()
        && get_type(mut_as_val(key)) == YYJSON_TYPE_STR
        && !val.is_null()
    {
        unsafe_mut_obj_add(obj, key, val, get_len(mut_as_val(obj)));
        return true;
    }
    false
}

/// `yyjson_mut_obj_put`
#[inline]
pub unsafe fn mut_obj_put(
    obj: *mut YyjsonMutVal,
    key: *mut YyjsonMutVal,
    val: *mut YyjsonMutVal,
) -> bool {
    let mut replaced = false;
    if obj.is_null()
        || get_type(mut_as_val(obj)) != YYJSON_TYPE_OBJ
        || key.is_null()
        || get_type(mut_as_val(key)) != YYJSON_TYPE_STR
    {
        return false;
    }
    let key_len = get_len(mut_as_val(key));
    let mut iter = core::mem::MaybeUninit::<YyjsonMutObjIter>::uninit();
    let iter = iter.as_mut_ptr();
    mut_obj_iter_init(obj, iter);
    loop {
        let cur_key = mut_obj_iter_next(iter);
        if cur_key.is_null() {
            break;
        }
        if equals_strn(mut_as_val(cur_key), (*key).uni.str_, key_len) {
            if !replaced && !val.is_null() {
                replaced = true;
                (*val).next = (*(*cur_key).next).next;
                (*cur_key).next = val;
            } else {
                mut_obj_iter_remove(iter);
            }
        }
    }
    if !replaced && !val.is_null() {
        unsafe_mut_obj_add(obj, key, val, (*iter).max);
    }
    true
}

/// `yyjson_mut_arr_append`
#[inline]
pub unsafe fn mut_arr_append(arr: *mut YyjsonMutVal, val: *mut YyjsonMutVal) -> bool {
    if !arr.is_null() && get_type(mut_as_val(arr)) == YYJSON_TYPE_ARR && !val.is_null() {
        let len = get_len(mut_as_val(arr));
        set_len(mut_as_val(arr), len + 1);
        if len == 0 {
            (*val).next = val;
        } else {
            let prev = (*arr).uni.ptr as *mut YyjsonMutVal;
            let next = (*prev).next;
            (*prev).next = val;
            (*val).next = next;
        }
        (*arr).uni.ptr = val as *mut c_void;
        return true;
    }
    false
}

/* ===== JSON pointer context helpers ===================================== */

/// `yyjson_ptr_ctx_replace`
#[inline]
pub unsafe fn ptr_ctx_replace(ctx: *mut YyjsonPtrCtx, val: *mut YyjsonMutVal) -> bool {
    if ctx.is_null() || (*ctx).ctn.is_null() || (*ctx).pre.is_null() || val.is_null() {
        return false;
    }
    let ctn = (*ctx).ctn;
    if get_type(mut_as_val(ctn)) == YYJSON_TYPE_OBJ {
        let pre_key = (*ctx).pre;
        let pre_val = (*pre_key).next;
        let cur_key = (*pre_val).next;
        let cur_val = (*cur_key).next;
        (*cur_key).next = val;
        (*val).next = (*cur_val).next;
        (*ctx).old = cur_val;
    } else {
        let pre_val = (*ctx).pre;
        let cur_val = (*pre_val).next;
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
#[inline]
pub unsafe fn ptr_ctx_remove(ctx: *mut YyjsonPtrCtx) -> bool {
    if ctx.is_null() || (*ctx).ctn.is_null() || (*ctx).pre.is_null() {
        return false;
    }
    let ctn = (*ctx).ctn;
    if get_type(mut_as_val(ctn)) == YYJSON_TYPE_OBJ {
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
    let len = get_len(mut_as_val(ctn)) - 1;
    if len == 0 {
        (*ctn).uni.ptr = ptr::null_mut();
    }
    set_len(mut_as_val(ctn), len);
    true
}

/* ===== JSON pointer entry points (header wrappers) ====================== */

macro_rules! ptr_set_err {
    ($err:expr, $code:expr, $msg:expr) => {
        if !$err.is_null() {
            (*$err).code = $code;
            (*$err).msg = $msg;
            (*$err).pos = 0;
        }
    };
}

/// `yyjson_mut_ptr_getx`
#[inline]
pub unsafe fn mut_ptr_getx(
    val: *const YyjsonMutVal,
    p: *const c_char,
    len: usize,
    ctx: *mut YyjsonPtrCtx,
    err: *mut YyjsonPtrErr,
) -> *mut YyjsonMutVal {
    ptr_set_err!(err, YYJSON_PTR_ERR_NONE, ptr::null());
    if !ctx.is_null() {
        libc::zero(ctx);
    }
    if val.is_null() || p.is_null() {
        ptr_set_err!(
            err,
            YYJSON_PTR_ERR_PARAMETER,
            crate::util::cstr!("input parameter is NULL")
        );
        return ptr::null_mut();
    }
    if len == 0 {
        return val as *mut YyjsonMutVal;
    }
    if *p != b'/' as c_char {
        ptr_set_err!(err, YYJSON_PTR_ERR_SYNTAX, crate::util::cstr!("no prefix '/'"));
        return ptr::null_mut();
    }
    crate::ptr::unsafe_yyjson_mut_ptr_getx(val, p, len, ctx, err)
}

/// `yyjson_mut_ptr_addx`
#[inline]
#[allow(clippy::too_many_arguments)]
pub unsafe fn mut_ptr_addx(
    val: *mut YyjsonMutVal,
    p: *const c_char,
    len: usize,
    new_val: *mut YyjsonMutVal,
    doc: *mut YyjsonMutDoc,
    create_parent: bool,
    ctx: *mut YyjsonPtrCtx,
    err: *mut YyjsonPtrErr,
) -> bool {
    ptr_set_err!(err, YYJSON_PTR_ERR_NONE, ptr::null());
    if !ctx.is_null() {
        libc::zero(ctx);
    }
    if val.is_null() || p.is_null() || new_val.is_null() || doc.is_null() {
        ptr_set_err!(
            err,
            YYJSON_PTR_ERR_PARAMETER,
            crate::util::cstr!("input parameter is NULL")
        );
        return false;
    }
    if len == 0 {
        ptr_set_err!(err, YYJSON_PTR_ERR_SET_ROOT, crate::util::cstr!("cannot set root"));
        return false;
    }
    if *p != b'/' as c_char {
        ptr_set_err!(err, YYJSON_PTR_ERR_SYNTAX, crate::util::cstr!("no prefix '/'"));
        return false;
    }
    crate::ptr::unsafe_yyjson_mut_ptr_putx(val, p, len, new_val, doc, create_parent, true, ctx, err)
}

/// `yyjson_mut_ptr_replacex`
#[inline]
pub unsafe fn mut_ptr_replacex(
    val: *mut YyjsonMutVal,
    p: *const c_char,
    len: usize,
    new_val: *mut YyjsonMutVal,
    ctx: *mut YyjsonPtrCtx,
    err: *mut YyjsonPtrErr,
) -> *mut YyjsonMutVal {
    ptr_set_err!(err, YYJSON_PTR_ERR_NONE, ptr::null());
    if !ctx.is_null() {
        libc::zero(ctx);
    }
    if val.is_null() || p.is_null() || new_val.is_null() {
        ptr_set_err!(
            err,
            YYJSON_PTR_ERR_PARAMETER,
            crate::util::cstr!("input parameter is NULL")
        );
        return ptr::null_mut();
    }
    if len == 0 {
        ptr_set_err!(err, YYJSON_PTR_ERR_SET_ROOT, crate::util::cstr!("cannot set root"));
        return ptr::null_mut();
    }
    if *p != b'/' as c_char {
        ptr_set_err!(err, YYJSON_PTR_ERR_SYNTAX, crate::util::cstr!("no prefix '/'"));
        return ptr::null_mut();
    }
    crate::ptr::unsafe_yyjson_mut_ptr_replacex(val, p, len, new_val, ctx, err)
}

/// `yyjson_mut_ptr_removex`
#[inline]
pub unsafe fn mut_ptr_removex(
    val: *mut YyjsonMutVal,
    p: *const c_char,
    len: usize,
    ctx: *mut YyjsonPtrCtx,
    err: *mut YyjsonPtrErr,
) -> *mut YyjsonMutVal {
    ptr_set_err!(err, YYJSON_PTR_ERR_NONE, ptr::null());
    if !ctx.is_null() {
        libc::zero(ctx);
    }
    if val.is_null() || p.is_null() {
        ptr_set_err!(
            err,
            YYJSON_PTR_ERR_PARAMETER,
            crate::util::cstr!("input parameter is NULL")
        );
        return ptr::null_mut();
    }
    if len == 0 {
        ptr_set_err!(err, YYJSON_PTR_ERR_SET_ROOT, crate::util::cstr!("cannot set root"));
        return ptr::null_mut();
    }
    if *p != b'/' as c_char {
        ptr_set_err!(err, YYJSON_PTR_ERR_SYNTAX, crate::util::cstr!("no prefix '/'"));
        return ptr::null_mut();
    }
    crate::ptr::unsafe_yyjson_mut_ptr_removex(val, p, len, ctx, err)
}
