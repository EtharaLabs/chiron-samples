// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 * MARK: - pointer.rs
 *
 * Owns yyjson.c 10559-11051 ("JSON Pointer API (RFC 6901) (Public)").
 *
 * `YYJSON_DISABLE_UTILS` is 0 in the default build, so the whole `#if` body is
 * ported.
 *
 * The caller-capturing macros of this section are hand-expanded at every use
 * site (CONVENTIONS.md):
 *
 *     #define return_err(_ret, _code, _pos, _msg) do { \
 *         if (err) { err->code = YYJSON_PTR_ERR_##_code; \
 *                    err->msg = _msg; err->pos = (usize)(_pos); } \
 *         return _ret; } while (false)
 *     #define return_err_resolve(_ret, _pos) \
 *         return_err(_ret, RESOLVE, _pos, "JSON pointer cannot be resolved")
 *     #define return_err_syntax(_ret, _pos) \
 *         return_err(_ret, SYNTAX, _pos, "invalid escaped character")
 *     #define return_err_alloc(_ret) \
 *         return_err(_ret, MEMORY_ALLOCATION, 0, "failed to create value")
 *
 * The side-effecting half of each expansion lives in `ptr_set_err` below; the
 * `return` statement stays at the use site so the control flow is identical.
 *============================================================================*/

use crate::sys::abi::{self as ffi, c_char, c_void};
use crate::model::layout::*;

use crate::model::header::{
    raw_doc_arr_is_flat, raw_doc_get_first, raw_doc_get_span,
    raw_doc_get_next, raw_doc_get_type, raw_doc_mut_obj_add,
    raw_doc_mut_str_alc, raw_doc_set_span, yyjson_mut_arr_append,
    yyjson_mut_is_obj, yyjson_mut_obj, yyjson_mut_obj_add, yyjson_mut_obj_put,
    yyjson_mut_strn, yyjson_mut_strncpy, yyjson_ptr_ctx_remove,
    yyjson_ptr_ctx_replace,
};
use crate::support::octets::{USIZE_MAX, USIZE_SAFE_DIG};

use core::mem::size_of;
use core::ptr::null_mut;

/*==============================================================================
 * MARK: - `return_err` family, side-effect half
 *============================================================================*/

/** The body of `return_err(_ret, _code, _pos, _msg)` minus the `return`. */
#[inline(always)]
unsafe fn ptr_set_err(err: *mut yyjson_ptr_err, code: yyjson_ptr_code,
                      pos: usize, msg: *const c_char) {
    unsafe {
        if !err.is_null() {
            (*err).code = code;
            (*err).msg = msg;
            (*err).pos = pos;
        }
    }
}

/** `return_err_resolve` message. */
const MSG_PTR_RESOLVE: *const c_char =
    c"JSON pointer cannot be resolved".as_ptr();
/** `return_err_syntax` message. */
const MSG_PTR_SYNTAX: *const c_char = c"invalid escaped character".as_ptr();
/** `return_err_alloc` message. */
const MSG_PTR_ALLOC: *const c_char = c"failed to create value".as_ptr();

/*==============================================================================
 * MARK: - Token helpers (yyjson.c:10574-10784)
 *============================================================================*/

/**
 Get a token from JSON pointer string.
 yyjson.c:10574
 */
#[inline(always)]
pub(crate) unsafe fn ptr_next_token(ptr: *mut *mut u8, end: *mut u8,
                                    len: *mut usize, esc: *mut usize)
    -> *mut u8 {
    unsafe {
        let hdr: *mut u8 = (*ptr).add(1);
        let mut cur: *mut u8 = hdr;
        /* skip unescaped characters */
        while cur < end && *cur != b'/' && *cur != b'~' {
            cur = cur.add(1);
        }
        if cur == end || *cur != b'~' {
            /* no escaped characters, return */
            *ptr = cur;
            *len = cur.offset_from(hdr) as usize;
            *esc = 0;
            hdr
        } else {
            /* handle escaped characters */
            let mut esc_num: usize = 0;
            while cur < end && *cur != b'/' {
                let c = *cur;
                cur = cur.add(1);
                if c == b'~' {
                    if cur == end || (*cur != b'0' && *cur != b'1') {
                        *ptr = cur.sub(1);
                        return null_mut();
                    }
                    esc_num = esc_num.wrapping_add(1);
                }
            }
            *ptr = cur;
            *len = (cur.offset_from(hdr) as usize).wrapping_sub(esc_num);
            *esc = esc_num;
            hdr
        }
    }
}

/**
 Convert token string to index.
 yyjson.c:10611
 */
#[inline(always)]
pub(crate) unsafe fn ptr_token_to_idx(cur: *mut u8, len: usize,
                                      idx: *mut usize) -> bool {
    unsafe {
        let end: *mut u8 = cur.add(len);
        let mut cur = cur;
        let mut num: usize = 0;
        let mut add: usize;
        if len == 0 || len > USIZE_SAFE_DIG {
            return false;
        }
        if *cur == b'0' {
            if len > 1 {
                return false;
            }
            *idx = 0;
            return true;
        }
        if *cur == b'-' {
            if len > 1 {
                return false;
            }
            *idx = USIZE_MAX;
            return true;
        }
        /* C: `(usize)((u8)*cur - (u8)'0')` - the subtraction happens in `int`
           and the (possibly negative) result is cast to `usize`, so any
           non-digit yields a value far above 9 and stops the loop. */
        loop {
            if !(cur < end) {
                break;
            }
            add = (*cur as i32 - b'0' as i32) as usize;
            if add > 9 {
                break;
            }
            num = num.wrapping_mul(10).wrapping_add(add);
            cur = cur.add(1);
        }
        if num == 0 || cur < end {
            return false;
        }
        *idx = num;
        true
    }
}

/**
 Compare JSON key with token.
 yyjson.c:10637
 */
#[inline(always)]
pub(crate) unsafe fn ptr_token_eq(key: *mut c_void, token: *mut u8,
                                  len: usize, esc: usize) -> bool {
    unsafe {
        let val = key as *mut yyjson_val;
        if raw_doc_get_span(val as *mut c_void) != len {
            return false;
        }
        if esc == 0 {
            ffi::memcmp((*val).uni.str_ as *const c_void,
                        token as *const c_void, len) == 0
        } else {
            let mut str_: *mut u8 = (*val).uni.str_ as *mut u8;
            let mut token = token;
            let mut len = len;
            /* C: `for (; len-- > 0; token++, str++)` - post-decrement. */
            while len > 0 {
                len = len.wrapping_sub(1);
                if *token == b'~' {
                    token = token.add(1);
                    if *str_ != (if *token == b'0' { b'~' } else { b'/' }) {
                        return false;
                    }
                } else {
                    if *str_ != *token {
                        return false;
                    }
                }
                token = token.add(1);
                str_ = str_.add(1);
            }
            true
        }
    }
}

/**
 Get a value from array by token.
 yyjson.c:10663
 */
#[inline(always)]
pub(crate) unsafe fn ptr_arr_get(arr: *mut yyjson_val, token: *mut u8,
                                 len: usize, _esc: usize) -> *mut yyjson_val {
    unsafe {
        let mut val = raw_doc_get_first(arr);
        let num = raw_doc_get_span(arr as *mut c_void);
        let mut idx: usize = 0;
        if num == 0 {
            return null_mut();
        }
        if !ptr_token_to_idx(token, len, &raw mut idx) {
            return null_mut();
        }
        if idx >= num {
            return null_mut();
        }
        if raw_doc_arr_is_flat(arr) {
            val.add(idx)
        } else {
            /* C: `while (idx-- > 0)` - post-decrement. */
            while idx > 0 {
                idx = idx.wrapping_sub(1);
                val = raw_doc_get_next(val);
            }
            val
        }
    }
}

/**
 Get a value from object by token.
 yyjson.c:10687
 */
#[inline(always)]
pub(crate) unsafe fn ptr_obj_get(obj: *mut yyjson_val, token: *mut u8,
                                 len: usize, esc: usize) -> *mut yyjson_val {
    unsafe {
        let mut key = raw_doc_get_first(obj);
        let mut num = raw_doc_get_span(obj as *mut c_void);
        if num == 0 {
            return null_mut();
        }
        while num > 0 {
            if ptr_token_eq(key as *mut c_void, token, len, esc) {
                return key.add(1);
            }
            num = num.wrapping_sub(1);
            key = raw_doc_get_next(key.add(1));
        }
        null_mut()
    }
}

/**
 Get a value from array by token (mutable).
 yyjson.c:10708
 */
#[inline(always)]
pub(crate) unsafe fn ptr_mut_arr_get(arr: *mut yyjson_mut_val, token: *mut u8,
                                     len: usize, _esc: usize,
                                     pre: *mut *mut yyjson_mut_val,
                                     last: *mut bool) -> *mut yyjson_mut_val {
    unsafe {
        /* last (tail) */
        let mut val = (*arr).uni.ptr as *mut yyjson_mut_val;
        let num = raw_doc_get_span(arr as *mut c_void);
        let mut idx: usize = 0;
        if !last.is_null() {
            *last = false;
        }
        if !pre.is_null() {
            *pre = null_mut();
        }
        if num == 0 {
            if !last.is_null() && len == 1
                && (*token == b'0' || *token == b'-')
            {
                *last = true;
            }
            return null_mut();
        }
        if !ptr_token_to_idx(token, len, &raw mut idx) {
            return null_mut();
        }
        if !last.is_null() {
            *last = idx == num || idx == USIZE_MAX;
        }
        if idx >= num {
            return null_mut();
        }
        /* C: `while (idx-- > 0)` - post-decrement. */
        while idx > 0 {
            idx = idx.wrapping_sub(1);
            val = (*val).next;
        }
        if !pre.is_null() {
            *pre = val;
        }
        (*val).next
    }
}

/**
 Get a value from object by token (mutable).
 yyjson.c:10740
 */
#[inline(always)]
pub(crate) unsafe fn ptr_mut_obj_get(obj: *mut yyjson_mut_val, token: *mut u8,
                                     len: usize, esc: usize,
                                     pre: *mut *mut yyjson_mut_val)
    -> *mut yyjson_mut_val {
    unsafe {
        let mut pre_key = (*obj).uni.ptr as *mut yyjson_mut_val;
        let mut key: *mut yyjson_mut_val;
        let mut num = raw_doc_get_span(obj as *mut c_void);
        if !pre.is_null() {
            *pre = null_mut();
        }
        if num == 0 {
            return null_mut();
        }
        while num > 0 {
            key = (*(*pre_key).next).next;
            if ptr_token_eq(key as *mut c_void, token, len, esc) {
                if !pre.is_null() {
                    *pre = pre_key;
                }
                return (*key).next;
            }
            num = num.wrapping_sub(1);
            pre_key = key;
        }
        null_mut()
    }
}

/**
 Create a string value with JSON pointer token.
 yyjson.c:10765
 */
#[inline(always)]
pub(crate) unsafe fn ptr_new_key(token: *mut u8, len: usize, esc: usize,
                                 doc: *mut yyjson_mut_doc)
    -> *mut yyjson_mut_val {
    unsafe {
        let mut src: *mut u8 = token;
        if esc == 0 {
            yyjson_mut_strncpy(doc, src, len)
        } else {
            let end: *mut u8 = src.add(len).add(esc);
            let mut dst: *mut c_char =
                raw_doc_mut_str_alc(doc, len.wrapping_add(esc));
            let str_: *mut c_char = dst;
            if dst.is_null() {
                return null_mut();
            }
            while src < end {
                if *src != b'~' {
                    *dst = *src as c_char;
                } else {
                    src = src.add(1);
                    *dst = (if *src == b'0' { b'~' } else { b'/' }) as c_char;
                }
                src = src.add(1);
                dst = dst.add(1);
            }
            *dst = 0;
            yyjson_mut_strn(doc, str_ as *mut u8, len)
        }
    }
}

/*==============================================================================
 * MARK: - Public pointer API (yyjson.c:10806-11046)
 *============================================================================*/

/// `yyjson_val *unsafe_yyjson_ptr_getx(const yyjson_val *val,
///                                     const char *ptr, size_t len,
///                                     yyjson_ptr_err *err)`
/// require: val != NULL, *ptr == '/', len > 0
/// yyjson.c:10806
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_ptr_getx(val: *const yyjson_val,
                                                ptr: *const c_char,
                                                len: usize,
                                                err: *mut yyjson_ptr_err)
    -> *mut yyjson_val {
    unsafe {
        let mut val = val as *mut yyjson_val;
        let mut ptr = ptr as *mut u8;
        let ptr_len = len;
        let hdr: *mut u8 = ptr;
        let end: *mut u8 = ptr.add(ptr_len);
        let mut tok_len: usize = 0;
        let mut esc: usize = 0;

        loop {
            let token: *mut u8;
            let type_: yyjson_type;
            let token_ = ptr_next_token(&raw mut ptr, end,
                                        &raw mut tok_len, &raw mut esc);
            if token_.is_null() {
                ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                            ptr.offset_from(hdr) as usize, MSG_PTR_SYNTAX);
                return null_mut();
            }
            token = token_;
            type_ = raw_doc_get_type(val as *mut c_void);
            if type_ == YYJSON_TYPE_OBJ {
                val = ptr_obj_get(val, token, tok_len, esc);
            } else if type_ == YYJSON_TYPE_ARR {
                val = ptr_arr_get(val, token, tok_len, esc);
            } else {
                val = null_mut();
            }
            if val.is_null() {
                ptr_set_err(err, YYJSON_PTR_ERR_RESOLVE,
                            token.offset_from(hdr) as usize, MSG_PTR_RESOLVE);
                return null_mut();
            }
            if ptr == end {
                return val;
            }
        }
    }
}

/// `yyjson_mut_val *unsafe_yyjson_mut_ptr_getx(const yyjson_mut_val *val,
///                                             const char *ptr, size_t len,
///                                             yyjson_ptr_ctx *ctx,
///                                             yyjson_ptr_err *err)`
/// require: val != NULL, *ptr == '/', len > 0
/// yyjson.c:10830
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_getx(
    val: *const yyjson_mut_val,
    ptr: *const c_char,
    len: usize,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    unsafe {
        let mut val = val as *mut yyjson_mut_val;
        let mut ptr = ptr as *mut u8;
        let ptr_len = len;
        let hdr: *mut u8 = ptr;
        let end: *mut u8 = ptr.add(ptr_len);
        let mut tok_len: usize = 0;
        let mut esc: usize = 0;
        let mut pre: *mut yyjson_mut_val = null_mut();
        let mut idx_is_last: bool = false;

        loop {
            let token: *mut u8;
            let ctn: *mut yyjson_mut_val;
            let type_: yyjson_type;
            let token_ = ptr_next_token(&raw mut ptr, end,
                                        &raw mut tok_len, &raw mut esc);
            if token_.is_null() {
                ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                            ptr.offset_from(hdr) as usize, MSG_PTR_SYNTAX);
                return null_mut();
            }
            token = token_;
            ctn = val;
            type_ = raw_doc_get_type(val as *mut c_void);
            if type_ == YYJSON_TYPE_OBJ {
                val = ptr_mut_obj_get(val, token, tok_len, esc,
                                      &raw mut pre);
            } else if type_ == YYJSON_TYPE_ARR {
                val = ptr_mut_arr_get(val, token, tok_len, esc,
                                      &raw mut pre, &raw mut idx_is_last);
            } else {
                val = null_mut();
            }
            if !ctx.is_null() && ptr == end {
                if type_ == YYJSON_TYPE_OBJ
                    || (type_ == YYJSON_TYPE_ARR
                        && (!val.is_null() || idx_is_last))
                {
                    (*ctx).ctn = ctn;
                    (*ctx).pre = pre;
                }
            }
            if val.is_null() {
                ptr_set_err(err, YYJSON_PTR_ERR_RESOLVE,
                            token.offset_from(hdr) as usize, MSG_PTR_RESOLVE);
                return null_mut();
            }
            if ptr == end {
                return val;
            }
        }
    }
}

/// `bool unsafe_yyjson_mut_ptr_putx(yyjson_mut_val *val, const char *ptr,
///                                  size_t len, yyjson_mut_val *new_val,
///                                  yyjson_mut_doc *doc, bool create_parent,
///                                  bool insert_new, yyjson_ptr_ctx *ctx,
///                                  yyjson_ptr_err *err)`
/// require: val != NULL, *ptr == '/', len > 0
/// yyjson.c:10864
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_putx(
    val: *mut yyjson_mut_val,
    ptr: *const c_char,
    len: usize,
    new_val: *mut yyjson_mut_val,
    doc: *mut yyjson_mut_doc,
    create_parent: bool,
    insert_new: bool,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> bool {
    unsafe {
        let mut val = val;
        let mut ptr = ptr as *mut u8;
        let ptr_len = len;
        let hdr: *mut u8 = ptr;
        let end: *mut u8 = ptr.add(ptr_len);
        let mut token: *mut u8;
        let mut token_len: usize = 0;
        let mut esc: usize = 0;
        let ctn_len: usize;
        let mut ctn: *mut yyjson_mut_val;
        let mut key: *mut yyjson_mut_val;
        let mut pre: *mut yyjson_mut_val = null_mut();
        let mut sep_ctn: *mut yyjson_mut_val = null_mut();
        let mut sep_key: *mut yyjson_mut_val = null_mut();
        let mut sep_val: *mut yyjson_mut_val = null_mut();
        let mut ctn_type: yyjson_type;
        let mut idx_is_last: bool = false;

        /* skip exist parent nodes */
        loop {
            let token_ = ptr_next_token(&raw mut ptr, end,
                                        &raw mut token_len, &raw mut esc);
            if token_.is_null() {
                ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                            ptr.offset_from(hdr) as usize, MSG_PTR_SYNTAX);
                return false;
            }
            token = token_;
            ctn = val;
            ctn_type = raw_doc_get_type(ctn as *mut c_void);
            if ctn_type == YYJSON_TYPE_OBJ {
                val = ptr_mut_obj_get(ctn, token, token_len, esc,
                                      &raw mut pre);
            } else if ctn_type == YYJSON_TYPE_ARR {
                val = ptr_mut_arr_get(ctn, token, token_len, esc,
                                      &raw mut pre, &raw mut idx_is_last);
            } else {
                ptr_set_err(err, YYJSON_PTR_ERR_RESOLVE,
                            token.offset_from(hdr) as usize, MSG_PTR_RESOLVE);
                return false;
            }
            if val.is_null() {
                break;
            }
            if ptr == end {
                break; /* is last token */
            }
        }

        /* create parent nodes if not exist */
        if ptr != end {
            /* not last token */
            if !create_parent {
                ptr_set_err(err, YYJSON_PTR_ERR_RESOLVE,
                            token.offset_from(hdr) as usize, MSG_PTR_RESOLVE);
                return false;
            }

            /* add value at last index if container is array */
            if ctn_type == YYJSON_TYPE_ARR {
                if !idx_is_last || !insert_new {
                    ptr_set_err(err, YYJSON_PTR_ERR_RESOLVE,
                                token.offset_from(hdr) as usize,
                                MSG_PTR_RESOLVE);
                    return false;
                }
                val = yyjson_mut_obj(doc);
                if val.is_null() {
                    ptr_set_err(err, YYJSON_PTR_ERR_MEMORY_ALLOCATION, 0,
                                MSG_PTR_ALLOC);
                    return false;
                }

                /* delay attaching until all operations are completed */
                sep_ctn = ctn;
                sep_key = null_mut();
                sep_val = val;

                /* move to next token */
                ctn = val;
                val = null_mut();
                ctn_type = YYJSON_TYPE_OBJ;
                let token_ = ptr_next_token(&raw mut ptr, end,
                                            &raw mut token_len, &raw mut esc);
                if token_.is_null() {
                    ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                                ptr.offset_from(hdr) as usize, MSG_PTR_SYNTAX);
                    return false;
                }
                token = token_;
            }

            /* container is object, create parent nodes */
            while ptr != end {
                /* not last token */
                key = ptr_new_key(token, token_len, esc, doc);
                if key.is_null() {
                    ptr_set_err(err, YYJSON_PTR_ERR_MEMORY_ALLOCATION, 0,
                                MSG_PTR_ALLOC);
                    return false;
                }
                val = yyjson_mut_obj(doc);
                if val.is_null() {
                    ptr_set_err(err, YYJSON_PTR_ERR_MEMORY_ALLOCATION, 0,
                                MSG_PTR_ALLOC);
                    return false;
                }

                /* delay attaching until all operations are completed */
                if sep_ctn.is_null() {
                    sep_ctn = ctn;
                    sep_key = key;
                    sep_val = val;
                } else {
                    yyjson_mut_obj_add(ctn, key, val);
                }

                /* move to next token */
                ctn = val;
                val = null_mut();
                let token_ = ptr_next_token(&raw mut ptr, end,
                                            &raw mut token_len, &raw mut esc);
                if token_.is_null() {
                    ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                                ptr.offset_from(hdr) as usize, MSG_PTR_SYNTAX);
                    return false;
                }
                token = token_;
            }
        }

        /* JSON pointer is resolved, insert or replace target value */
        ctn_len = raw_doc_get_span(ctn as *mut c_void);
        if ctn_type == YYJSON_TYPE_OBJ {
            if !ctx.is_null() {
                (*ctx).ctn = ctn;
            }
            if val.is_null() || insert_new {
                /* insert new key-value pair */
                key = ptr_new_key(token, token_len, esc, doc);
                if key.is_null() {
                    ptr_set_err(err, YYJSON_PTR_ERR_MEMORY_ALLOCATION, 0,
                                MSG_PTR_ALLOC);
                    return false;
                }
                if !ctx.is_null() {
                    (*ctx).pre = if ctn_len != 0 {
                        (*ctn).uni.ptr as *mut yyjson_mut_val
                    } else {
                        key
                    };
                }
                raw_doc_mut_obj_add(ctn, key, new_val, ctn_len);
            } else {
                /* replace exist value */
                key = (*(*pre).next).next;
                if !ctx.is_null() {
                    (*ctx).pre = pre;
                }
                if !ctx.is_null() {
                    (*ctx).old = val;
                }
                yyjson_mut_obj_put(ctn, key, new_val);
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
                    raw_doc_set_span(ctn as *mut c_void,
                                          ctn_len.wrapping_add(1));
                } else if idx_is_last {
                    if !ctx.is_null() {
                        (*ctx).pre = if ctn_len != 0 {
                            (*ctn).uni.ptr as *mut yyjson_mut_val
                        } else {
                            new_val
                        };
                    }
                    yyjson_mut_arr_append(ctn, new_val);
                } else {
                    ptr_set_err(err, YYJSON_PTR_ERR_RESOLVE,
                                token.offset_from(hdr) as usize,
                                MSG_PTR_RESOLVE);
                    return false;
                }
            } else {
                /* replace exist value */
                if val.is_null() {
                    ptr_set_err(err, YYJSON_PTR_ERR_RESOLVE,
                                token.offset_from(hdr) as usize,
                                MSG_PTR_RESOLVE);
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
                }
                if !ctx.is_null() {
                    (*ctx).old = val;
                }
            }
        }

        /* all operations are completed, attach the new components to the
           target */
        if !sep_ctn.is_null() {
            if !sep_key.is_null() {
                yyjson_mut_obj_add(sep_ctn, sep_key, sep_val);
            } else {
                yyjson_mut_arr_append(sep_ctn, sep_val);
            }
        }
        true
    }
}

/// `yyjson_mut_val *unsafe_yyjson_mut_ptr_replacex(
///      yyjson_mut_val *val, const char *ptr, size_t len,
///      yyjson_mut_val *new_val, yyjson_ptr_ctx *ctx, yyjson_ptr_err *err)`
/// require: val != NULL, *ptr == '/', len > 0
/// yyjson.c:11000
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_replacex(
    val: *mut yyjson_mut_val,
    ptr: *const c_char,
    len: usize,
    new_val: *mut yyjson_mut_val,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    unsafe {
        let cur_val: *mut yyjson_mut_val;
        let mut cur_ctx: yyjson_ptr_ctx =
            core::mem::MaybeUninit::zeroed().assume_init();
        let mut ctx = ctx;
        ffi::memset(&raw mut cur_ctx as *mut c_void, 0,
                    size_of::<yyjson_ptr_ctx>());
        if ctx.is_null() {
            ctx = &raw mut cur_ctx;
        }
        cur_val = unsafe_yyjson_mut_ptr_getx(val as *const yyjson_mut_val,
                                             ptr, len, ctx, err);
        if cur_val.is_null() {
            return null_mut();
        }

        if yyjson_mut_is_obj((*ctx).ctn) {
            let key = (*(*(*ctx).pre).next).next;
            yyjson_mut_obj_put((*ctx).ctn, key, new_val);
        } else {
            yyjson_ptr_ctx_replace(ctx, new_val);
        }
        (*ctx).old = cur_val;
        cur_val
    }
}

/// `yyjson_mut_val *unsafe_yyjson_mut_ptr_removex(yyjson_mut_val *val,
///                                                const char *ptr, size_t len,
///                                                yyjson_ptr_ctx *ctx,
///                                                yyjson_ptr_err *err)`
/// require: val != NULL, *ptr == '/', len > 0
/// yyjson.c:11021
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_mut_ptr_removex(
    val: *mut yyjson_mut_val,
    ptr: *const c_char,
    len: usize,
    ctx: *mut yyjson_ptr_ctx,
    err: *mut yyjson_ptr_err,
) -> *mut yyjson_mut_val {
    unsafe {
        let cur_val: *mut yyjson_mut_val;
        let mut cur_ctx: yyjson_ptr_ctx =
            core::mem::MaybeUninit::zeroed().assume_init();
        let mut ctx = ctx;
        ffi::memset(&raw mut cur_ctx as *mut c_void, 0,
                    size_of::<yyjson_ptr_ctx>());
        if ctx.is_null() {
            ctx = &raw mut cur_ctx;
        }
        cur_val = unsafe_yyjson_mut_ptr_getx(val as *const yyjson_mut_val,
                                             ptr, len, ctx, err);
        if !cur_val.is_null() {
            if yyjson_mut_is_obj((*ctx).ctn) {
                let key = (*(*(*ctx).pre).next).next;
                yyjson_mut_obj_put((*ctx).ctn, key, null_mut());
            } else {
                yyjson_ptr_ctx_remove(ctx);
            }
            (*ctx).pre = null_mut();
            (*ctx).old = cur_val;
        }
        cur_val
    }
}
