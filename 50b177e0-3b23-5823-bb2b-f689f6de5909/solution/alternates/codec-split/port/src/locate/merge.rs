// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 * MARK: - patch.rs
 *
 * Owns yyjson.c 11052-11480: "JSON Patch API (RFC 6902) (Public)" and
 * "JSON Merge-Patch API (RFC 7386) (Public)".
 *
 * `YYJSON_DISABLE_UTILS` is 0 in the default build, so the whole `#if` body is
 * ported.
 *
 * The caller-capturing macros of this section are hand-expanded (CONVENTIONS):
 *
 *     #define return_err(_code, _msg) do { \
 *         if (err->ptr.code == YYJSON_PTR_ERR_MEMORY_ALLOCATION) { \
 *             err->code = YYJSON_PATCH_ERROR_MEMORY_ALLOCATION; \
 *             err->msg = _msg; \
 *             memset(&err->ptr, 0, sizeof(yyjson_ptr_err)); \
 *         } else { \
 *             err->code = YYJSON_PATCH_ERROR_##_code; \
 *             err->msg = _msg; \
 *             err->idx = iter.idx ? iter.idx - 1 : 0; \
 *         } \
 *         return NULL; } while (false)
 *     #define return_err_copy() return_err(MEMORY_ALLOCATION,
 *                                          "failed to copy value")
 *     #define return_err_key(_key) return_err(MISSING_KEY, "missing key" _key)
 *     #define return_err_val(_key) return_err(INVALID_MEMBER,
 *                                             "invalid member " _key)
 *     #define ptr_get(_ptr)          yyjson_mut_ptr_getx(...)
 *     #define ptr_add(_ptr, _val)    yyjson_mut_ptr_addx(...)
 *     #define ptr_remove(_ptr)       yyjson_mut_ptr_removex(...)
 *     #define ptr_replace(_ptr,_val) yyjson_mut_ptr_replacex(...)
 *
 * `patch_set_err` below carries the side-effecting half of `return_err`; the
 * `return` statement stays at the use site, so the control flow is identical.
 * The `_msg` string-literal concatenation of `return_err_key` /
 * `return_err_val` is pre-concatenated into the message constants.
 *
 * The two `yyjson_*obj_foreach` macros (yyjson.h:2430, 3871) are also expanded
 * by hand. Their `yyjson_obj_size(obj)` / `yyjson_mut_obj_size(obj)` calls are
 * inlined as well; `crate::repr::inline` exports `yyjson_mut_obj_size` but not the
 * immutable `yyjson_obj_size`, so the latter is spelled out at the use site.
 *============================================================================*/

use crate::abi::libc::{self as ffi, c_char, c_void};
use crate::repr::shape::*;

use crate::repr::inline::{
    checked_yyjson_get_first, checked_yyjson_get_len, checked_yyjson_get_next,
    checked_yyjson_get_str, checked_yyjson_is_null, checked_yyjson_is_obj,
    yyjson_arr_iter_init, yyjson_arr_iter_next, yyjson_is_arr, yyjson_is_obj,
    yyjson_is_str, yyjson_mut_arr_iter_init, yyjson_mut_arr_iter_next,
    yyjson_mut_equals, yyjson_mut_is_arr, yyjson_mut_is_obj,
    yyjson_mut_is_str, yyjson_mut_obj, yyjson_mut_obj_add, yyjson_mut_obj_get,
    yyjson_mut_obj_getn, yyjson_mut_obj_size, yyjson_mut_ptr_addx,
    yyjson_mut_ptr_getx, yyjson_mut_ptr_removex, yyjson_mut_ptr_replacex,
    yyjson_obj_get, yyjson_obj_getn,
};

use core::mem::size_of;
use core::ptr::null_mut;

/*==============================================================================
 * MARK: - JSON Patch operation (yyjson.c:11056-11086)
 *============================================================================*/

/* JSON Patch operation (yyjson.c:11056) */
pub(crate) type patch_op = u32;

/** path, value */
pub(crate) const PATCH_OP_ADD: patch_op = 0;
/** path */
pub(crate) const PATCH_OP_REMOVE: patch_op = 1;
/** path, value */
pub(crate) const PATCH_OP_REPLACE: patch_op = 2;
/** from, path */
pub(crate) const PATCH_OP_MOVE: patch_op = 3;
/** from, path */
pub(crate) const PATCH_OP_COPY: patch_op = 4;
/** path, value */
pub(crate) const PATCH_OP_TEST: patch_op = 5;
/** invalid */
pub(crate) const PATCH_OP_NONE: patch_op = 6;

/** yyjson.c:11066 */
#[inline]
pub(crate) unsafe fn patch_op_get(op: *mut yyjson_val) -> patch_op {
    unsafe {
        let str_ = (*op).uni.str_ as *const c_void;
        match checked_yyjson_get_len(op as *mut c_void) {
            3 => {
                if ffi::memcmp(str_, c"add".as_ptr() as *const c_void, 3) == 0 {
                    return PATCH_OP_ADD;
                }
                PATCH_OP_NONE
            }
            4 => {
                if ffi::memcmp(str_, c"move".as_ptr() as *const c_void, 4) == 0 {
                    return PATCH_OP_MOVE;
                }
                if ffi::memcmp(str_, c"copy".as_ptr() as *const c_void, 4) == 0 {
                    return PATCH_OP_COPY;
                }
                if ffi::memcmp(str_, c"test".as_ptr() as *const c_void, 4) == 0 {
                    return PATCH_OP_TEST;
                }
                PATCH_OP_NONE
            }
            6 => {
                if ffi::memcmp(str_, c"remove".as_ptr() as *const c_void, 6) == 0
                {
                    return PATCH_OP_REMOVE;
                }
                PATCH_OP_NONE
            }
            7 => {
                if ffi::memcmp(str_, c"replace".as_ptr() as *const c_void, 7)
                    == 0
                {
                    return PATCH_OP_REPLACE;
                }
                PATCH_OP_NONE
            }
            _ => PATCH_OP_NONE,
        }
    }
}

/*==============================================================================
 * MARK: - `return_err` family, side-effect half
 *============================================================================*/

/** The body of `return_err(_code, _msg)` minus the `return NULL`. */
#[inline(always)]
unsafe fn patch_set_err(err: *mut yyjson_patch_err, iter_idx: usize,
                        code: yyjson_patch_code, msg: *const c_char) {
    unsafe {
        if (*err).ptr.code == YYJSON_PTR_ERR_MEMORY_ALLOCATION {
            (*err).code = YYJSON_PATCH_ERROR_MEMORY_ALLOCATION;
            (*err).msg = msg;
            ffi::memset(&raw mut (*err).ptr as *mut c_void, 0,
                        size_of::<yyjson_ptr_err>());
        } else {
            (*err).code = code;
            (*err).msg = msg;
            (*err).idx = if iter_idx != 0 {
                iter_idx.wrapping_sub(1)
            } else {
                0
            };
        }
    }
}

const MSG_PATCH_NULL: *const c_char = c"input parameter is NULL".as_ptr();
const MSG_PATCH_NOT_ARR: *const c_char = c"input patch is not array".as_ptr();
const MSG_PATCH_COPY: *const c_char = c"failed to copy value".as_ptr();
const MSG_PATCH_NOT_OBJ: *const c_char =
    c"JSON patch operation is not object".as_ptr();
const MSG_PATCH_KEY_OP: *const c_char = c"missing key `op`".as_ptr();
const MSG_PATCH_VAL_OP: *const c_char = c"invalid member `op`".as_ptr();
const MSG_PATCH_KEY_PATH: *const c_char = c"missing key `path`".as_ptr();
const MSG_PATCH_VAL_PATH: *const c_char = c"invalid member `path`".as_ptr();
const MSG_PATCH_KEY_VALUE: *const c_char = c"missing key `value`".as_ptr();
const MSG_PATCH_KEY_FROM: *const c_char = c"missing key `from`".as_ptr();
const MSG_PATCH_VAL_FROM: *const c_char = c"invalid member `from`".as_ptr();
const MSG_PATCH_ADD_PATH: *const c_char = c"failed to add `path`".as_ptr();
const MSG_PATCH_REMOVE_PATH: *const c_char =
    c"failed to remove `path`".as_ptr();
const MSG_PATCH_REPLACE_PATH: *const c_char =
    c"failed to replace `path`".as_ptr();
const MSG_PATCH_REMOVE_FROM: *const c_char =
    c"failed to remove `from`".as_ptr();
const MSG_PATCH_GET_FROM: *const c_char = c"failed to get `from`".as_ptr();
const MSG_PATCH_GET_PATH: *const c_char = c"failed to get `path`".as_ptr();
const MSG_PATCH_EQUAL: *const c_char = c"failed to test equal".as_ptr();
const MSG_PATCH_OP: *const c_char = c"unsupported `op`".as_ptr();

/*==============================================================================
 * MARK: - JSON Patch (yyjson.c:11118-11364)
 *============================================================================*/

/// `yyjson_mut_val *yyjson_patch(yyjson_mut_doc *doc, const yyjson_val *orig,
///                               const yyjson_val *patch,
///                               yyjson_patch_err *err)`
/// yyjson.c:11118
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_patch(doc: *mut yyjson_mut_doc,
                                      orig: *const yyjson_val,
                                      patch: *const yyjson_val,
                                      err: *mut yyjson_patch_err)
    -> *mut yyjson_mut_val {
    unsafe {
        let mut root: *mut yyjson_mut_val;
        let mut obj: *mut yyjson_val;
        let mut iter: yyjson_arr_iter =
            core::mem::MaybeUninit::zeroed().assume_init();
        let mut err_tmp: yyjson_patch_err =
            core::mem::MaybeUninit::zeroed().assume_init();
        let mut err = err;
        if err.is_null() {
            err = &raw mut err_tmp;
        }
        ffi::memset(err as *mut c_void, 0, size_of::<yyjson_patch_err>());
        ffi::memset(&raw mut iter as *mut c_void, 0,
                    size_of::<yyjson_arr_iter>());

        if doc.is_null() || orig.is_null() || patch.is_null() {
            patch_set_err(err, iter.idx,
                          YYJSON_PATCH_ERROR_INVALID_PARAMETER,
                          MSG_PATCH_NULL);
            return null_mut();
        }
        if !yyjson_is_arr(patch as *mut yyjson_val) {
            patch_set_err(err, iter.idx,
                          YYJSON_PATCH_ERROR_INVALID_PARAMETER,
                          MSG_PATCH_NOT_ARR);
            return null_mut();
        }
        root = crate::repr::node::yyjson_val_mut_copy(doc, orig);
        if root.is_null() {
            patch_set_err(err, iter.idx,
                          YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                          MSG_PATCH_COPY);
            return null_mut();
        }

        /* iterate through the patch array */
        yyjson_arr_iter_init(patch as *mut yyjson_val, &raw mut iter);
        loop {
            obj = yyjson_arr_iter_next(&raw mut iter);
            if obj.is_null() {
                break;
            }
            let op_enum: patch_op;
            let op: *mut yyjson_val;
            let path: *mut yyjson_val;
            let mut from: *mut yyjson_val = null_mut();
            let value: *mut yyjson_val;
            let mut val: *mut yyjson_mut_val = null_mut();
            let test: *mut yyjson_mut_val;
            let path_len: usize;
            let mut from_len: usize = 0;
            if !checked_yyjson_is_obj(obj as *mut c_void) {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_INVALID_OPERATION,
                              MSG_PATCH_NOT_OBJ);
                return null_mut();
            }

            /* get required member: op */
            op = yyjson_obj_get(obj, c"op".as_ptr() as *mut u8);
            if op.is_null() {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_MISSING_KEY,
                              MSG_PATCH_KEY_OP);
                return null_mut();
            }
            if !yyjson_is_str(op) {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_INVALID_MEMBER,
                              MSG_PATCH_VAL_OP);
                return null_mut();
            }
            op_enum = patch_op_get(op);

            /* get required member: path */
            path = yyjson_obj_get(obj, c"path".as_ptr() as *mut u8);
            if path.is_null() {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_MISSING_KEY,
                              MSG_PATCH_KEY_PATH);
                return null_mut();
            }
            if !yyjson_is_str(path) {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_INVALID_MEMBER,
                              MSG_PATCH_VAL_PATH);
                return null_mut();
            }
            path_len = checked_yyjson_get_len(path as *mut c_void);

            /* get required member: value, from */
            match op_enum {
                PATCH_OP_ADD | PATCH_OP_REPLACE | PATCH_OP_TEST => {
                    value = yyjson_obj_get(obj, c"value".as_ptr() as *mut u8);
                    if value.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_MISSING_KEY,
                                      MSG_PATCH_KEY_VALUE);
                        return null_mut();
                    }
                    val = crate::repr::node::yyjson_val_mut_copy(
                        doc, value as *const yyjson_val);
                    if val.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                                      MSG_PATCH_COPY);
                        return null_mut();
                    }
                }
                PATCH_OP_MOVE | PATCH_OP_COPY => {
                    from = yyjson_obj_get(obj, c"from".as_ptr() as *mut u8);
                    if from.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_MISSING_KEY,
                                      MSG_PATCH_KEY_FROM);
                        return null_mut();
                    }
                    if !yyjson_is_str(from) {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_INVALID_MEMBER,
                                      MSG_PATCH_VAL_FROM);
                        return null_mut();
                    }
                    from_len = checked_yyjson_get_len(from as *mut c_void);
                }
                _ => {}
            }

            /* perform an operation */
            match op_enum {
                /* add(path, val) */
                PATCH_OP_ADD => {
                    if path_len == 0 {
                        root = val;
                    } else if !yyjson_mut_ptr_addx(
                        root, (*path).uni.str_ as *mut u8, path_len, val, doc,
                        false, null_mut(), &raw mut (*err).ptr)
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_ADD_PATH);
                        return null_mut();
                    }
                }
                /* remove(path) */
                PATCH_OP_REMOVE => {
                    if yyjson_mut_ptr_removex(
                        root, (*path).uni.str_ as *mut u8, path_len,
                        null_mut(), &raw mut (*err).ptr).is_null()
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_REMOVE_PATH);
                        return null_mut();
                    }
                }
                /* replace(path, val) */
                PATCH_OP_REPLACE => {
                    if path_len == 0 {
                        root = val;
                    } else if yyjson_mut_ptr_replacex(
                        root, (*path).uni.str_ as *mut u8, path_len, val,
                        null_mut(), &raw mut (*err).ptr).is_null()
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_REPLACE_PATH);
                        return null_mut();
                    }
                }
                /* val = remove(from), add(path, val) */
                PATCH_OP_MOVE => 'move_op: {
                    if from_len == 0 && path_len == 0 {
                        break 'move_op;
                    }
                    val = yyjson_mut_ptr_removex(
                        root, (*from).uni.str_ as *mut u8, from_len,
                        null_mut(), &raw mut (*err).ptr);
                    if val.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_REMOVE_FROM);
                        return null_mut();
                    }
                    if path_len == 0 {
                        root = val;
                        break 'move_op;
                    }
                    if !yyjson_mut_ptr_addx(
                        root, (*path).uni.str_ as *mut u8, path_len, val, doc,
                        false, null_mut(), &raw mut (*err).ptr)
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_ADD_PATH);
                        return null_mut();
                    }
                }
                /* val = get(from).copy, add(path, val) */
                PATCH_OP_COPY => 'copy_op: {
                    val = yyjson_mut_ptr_getx(
                        root, (*from).uni.str_ as *mut u8, from_len,
                        null_mut(), &raw mut (*err).ptr);
                    if val.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_GET_FROM);
                        return null_mut();
                    }
                    if path_len == 0 {
                        root = val;
                        break 'copy_op;
                    }
                    val = crate::repr::node::yyjson_mut_val_mut_copy(
                        doc, val as *const yyjson_mut_val);
                    if val.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                                      MSG_PATCH_COPY);
                        return null_mut();
                    }
                    if !yyjson_mut_ptr_addx(
                        root, (*path).uni.str_ as *mut u8, path_len, val, doc,
                        false, null_mut(), &raw mut (*err).ptr)
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_ADD_PATH);
                        return null_mut();
                    }
                }
                /* test = get(path), test.eq(val) */
                PATCH_OP_TEST => {
                    test = yyjson_mut_ptr_getx(
                        root, (*path).uni.str_ as *mut u8, path_len,
                        null_mut(), &raw mut (*err).ptr);
                    if test.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_GET_PATH);
                        return null_mut();
                    }
                    if !yyjson_mut_equals(val, test) {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_EQUAL,
                                      MSG_PATCH_EQUAL);
                        return null_mut();
                    }
                }
                _ => {
                    patch_set_err(err, iter.idx,
                                  YYJSON_PATCH_ERROR_INVALID_MEMBER,
                                  MSG_PATCH_OP);
                    return null_mut();
                }
            }
        }
        root
    }
}

/// `yyjson_mut_val *yyjson_mut_patch(yyjson_mut_doc *doc,
///                                   const yyjson_mut_val *orig,
///                                   const yyjson_mut_val *patch,
///                                   yyjson_patch_err *err)`
/// yyjson.c:11239
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_patch(doc: *mut yyjson_mut_doc,
                                          orig: *const yyjson_mut_val,
                                          patch: *const yyjson_mut_val,
                                          err: *mut yyjson_patch_err)
    -> *mut yyjson_mut_val {
    unsafe {
        let mut root: *mut yyjson_mut_val;
        let mut obj: *mut yyjson_mut_val;
        let mut iter: yyjson_mut_arr_iter =
            core::mem::MaybeUninit::zeroed().assume_init();
        let mut err_tmp: yyjson_patch_err =
            core::mem::MaybeUninit::zeroed().assume_init();
        let mut err = err;
        if err.is_null() {
            err = &raw mut err_tmp;
        }
        ffi::memset(err as *mut c_void, 0, size_of::<yyjson_patch_err>());
        ffi::memset(&raw mut iter as *mut c_void, 0,
                    size_of::<yyjson_mut_arr_iter>());

        if doc.is_null() || orig.is_null() || patch.is_null() {
            patch_set_err(err, iter.idx,
                          YYJSON_PATCH_ERROR_INVALID_PARAMETER,
                          MSG_PATCH_NULL);
            return null_mut();
        }
        if !yyjson_mut_is_arr(patch as *mut yyjson_mut_val) {
            patch_set_err(err, iter.idx,
                          YYJSON_PATCH_ERROR_INVALID_PARAMETER,
                          MSG_PATCH_NOT_ARR);
            return null_mut();
        }
        root = crate::repr::node::yyjson_mut_val_mut_copy(doc, orig);
        if root.is_null() {
            patch_set_err(err, iter.idx,
                          YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                          MSG_PATCH_COPY);
            return null_mut();
        }

        /* iterate through the patch array */
        yyjson_mut_arr_iter_init(patch as *mut yyjson_mut_val,
                                 &raw mut iter);
        loop {
            obj = yyjson_mut_arr_iter_next(&raw mut iter);
            if obj.is_null() {
                break;
            }
            let op_enum: patch_op;
            let op: *mut yyjson_mut_val;
            let path: *mut yyjson_mut_val;
            let mut from: *mut yyjson_mut_val = null_mut();
            let value: *mut yyjson_mut_val;
            let mut val: *mut yyjson_mut_val = null_mut();
            let test: *mut yyjson_mut_val;
            let path_len: usize;
            let mut from_len: usize = 0;
            if !checked_yyjson_is_obj(obj as *mut c_void) {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_INVALID_OPERATION,
                              MSG_PATCH_NOT_OBJ);
                return null_mut();
            }

            /* get required member: op */
            op = yyjson_mut_obj_get(obj, c"op".as_ptr() as *mut u8);
            if op.is_null() {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_MISSING_KEY,
                              MSG_PATCH_KEY_OP);
                return null_mut();
            }
            if !yyjson_mut_is_str(op) {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_INVALID_MEMBER,
                              MSG_PATCH_VAL_OP);
                return null_mut();
            }
            op_enum = patch_op_get(op as *mut yyjson_val);

            /* get required member: path */
            path = yyjson_mut_obj_get(obj, c"path".as_ptr() as *mut u8);
            if path.is_null() {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_MISSING_KEY,
                              MSG_PATCH_KEY_PATH);
                return null_mut();
            }
            if !yyjson_mut_is_str(path) {
                patch_set_err(err, iter.idx,
                              YYJSON_PATCH_ERROR_INVALID_MEMBER,
                              MSG_PATCH_VAL_PATH);
                return null_mut();
            }
            path_len = checked_yyjson_get_len(path as *mut c_void);

            /* get required member: value, from */
            match op_enum {
                PATCH_OP_ADD | PATCH_OP_REPLACE | PATCH_OP_TEST => {
                    value = yyjson_mut_obj_get(obj,
                                               c"value".as_ptr() as *mut u8);
                    if value.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_MISSING_KEY,
                                      MSG_PATCH_KEY_VALUE);
                        return null_mut();
                    }
                    val = crate::repr::node::yyjson_mut_val_mut_copy(
                        doc, value as *const yyjson_mut_val);
                    if val.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                                      MSG_PATCH_COPY);
                        return null_mut();
                    }
                }
                PATCH_OP_MOVE | PATCH_OP_COPY => {
                    from = yyjson_mut_obj_get(obj,
                                              c"from".as_ptr() as *mut u8);
                    if from.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_MISSING_KEY,
                                      MSG_PATCH_KEY_FROM);
                        return null_mut();
                    }
                    if !yyjson_mut_is_str(from) {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_INVALID_MEMBER,
                                      MSG_PATCH_VAL_FROM);
                        return null_mut();
                    }
                    from_len = checked_yyjson_get_len(from as *mut c_void);
                }
                _ => {}
            }

            /* perform an operation */
            match op_enum {
                /* add(path, val) */
                PATCH_OP_ADD => {
                    if path_len == 0 {
                        root = val;
                    } else if !yyjson_mut_ptr_addx(
                        root, (*path).uni.str_ as *mut u8, path_len, val, doc,
                        false, null_mut(), &raw mut (*err).ptr)
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_ADD_PATH);
                        return null_mut();
                    }
                }
                /* remove(path) */
                PATCH_OP_REMOVE => {
                    if yyjson_mut_ptr_removex(
                        root, (*path).uni.str_ as *mut u8, path_len,
                        null_mut(), &raw mut (*err).ptr).is_null()
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_REMOVE_PATH);
                        return null_mut();
                    }
                }
                /* replace(path, val) */
                PATCH_OP_REPLACE => {
                    if path_len == 0 {
                        root = val;
                    } else if yyjson_mut_ptr_replacex(
                        root, (*path).uni.str_ as *mut u8, path_len, val,
                        null_mut(), &raw mut (*err).ptr).is_null()
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_REPLACE_PATH);
                        return null_mut();
                    }
                }
                /* val = remove(from), add(path, val) */
                PATCH_OP_MOVE => 'move_op: {
                    if from_len == 0 && path_len == 0 {
                        break 'move_op;
                    }
                    val = yyjson_mut_ptr_removex(
                        root, (*from).uni.str_ as *mut u8, from_len,
                        null_mut(), &raw mut (*err).ptr);
                    if val.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_REMOVE_FROM);
                        return null_mut();
                    }
                    if path_len == 0 {
                        root = val;
                        break 'move_op;
                    }
                    if !yyjson_mut_ptr_addx(
                        root, (*path).uni.str_ as *mut u8, path_len, val, doc,
                        false, null_mut(), &raw mut (*err).ptr)
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_ADD_PATH);
                        return null_mut();
                    }
                }
                /* val = get(from).copy, add(path, val) */
                PATCH_OP_COPY => 'copy_op: {
                    val = yyjson_mut_ptr_getx(
                        root, (*from).uni.str_ as *mut u8, from_len,
                        null_mut(), &raw mut (*err).ptr);
                    if val.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_GET_FROM);
                        return null_mut();
                    }
                    if path_len == 0 {
                        root = val;
                        break 'copy_op;
                    }
                    val = crate::repr::node::yyjson_mut_val_mut_copy(
                        doc, val as *const yyjson_mut_val);
                    if val.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_MEMORY_ALLOCATION,
                                      MSG_PATCH_COPY);
                        return null_mut();
                    }
                    if !yyjson_mut_ptr_addx(
                        root, (*path).uni.str_ as *mut u8, path_len, val, doc,
                        false, null_mut(), &raw mut (*err).ptr)
                    {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_ADD_PATH);
                        return null_mut();
                    }
                }
                /* test = get(path), test.eq(val) */
                PATCH_OP_TEST => {
                    test = yyjson_mut_ptr_getx(
                        root, (*path).uni.str_ as *mut u8, path_len,
                        null_mut(), &raw mut (*err).ptr);
                    if test.is_null() {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_POINTER,
                                      MSG_PATCH_GET_PATH);
                        return null_mut();
                    }
                    if !yyjson_mut_equals(val, test) {
                        patch_set_err(err, iter.idx,
                                      YYJSON_PATCH_ERROR_EQUAL,
                                      MSG_PATCH_EQUAL);
                        return null_mut();
                    }
                }
                _ => {
                    patch_set_err(err, iter.idx,
                                  YYJSON_PATCH_ERROR_INVALID_MEMBER,
                                  MSG_PATCH_OP);
                    return null_mut();
                }
            }
        }
        root
    }
}

/*==============================================================================
 * MARK: - JSON Merge-Patch API (RFC 7386) (yyjson.c:11372-11476)
 *============================================================================*/

/// `yyjson_mut_val *yyjson_merge_patch(yyjson_mut_doc *doc,
///                                     const yyjson_val *orig,
///                                     const yyjson_val *patch)`
/// yyjson.c:11376
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_merge_patch(doc: *mut yyjson_mut_doc,
                                            orig: *const yyjson_val,
                                            patch: *const yyjson_val)
    -> *mut yyjson_mut_val {
    unsafe {
        let mut idx: usize;
        let max: usize;
        let mut key: *mut yyjson_val;
        let mut orig_val: *mut yyjson_val;
        let mut patch_val: *mut yyjson_val;
        let mut local_orig: yyjson_val =
            core::mem::MaybeUninit::zeroed().assume_init();
        let builder: *mut yyjson_mut_val;
        let mut mut_key: *mut yyjson_mut_val;
        let mut mut_val: *mut yyjson_mut_val;
        let mut merged_val: *mut yyjson_mut_val;

        let mut orig = orig as *mut yyjson_val;
        let patch = patch as *mut yyjson_val;

        if !yyjson_is_obj(patch) {
            return crate::repr::node::yyjson_val_mut_copy(
                doc, patch as *const yyjson_val);
        }

        builder = yyjson_mut_obj(doc);
        if builder.is_null() {
            return null_mut();
        }

        ffi::memset(&raw mut local_orig as *mut c_void, 0,
                    size_of::<yyjson_val>());
        if !yyjson_is_obj(orig) {
            local_orig.tag = (*builder).tag;
            local_orig.uni = (*builder).uni;
            orig = &raw mut local_orig;
        }

        /* If orig is contributing, copy any items not modified by the patch */
        if orig != &raw mut local_orig {
            /* yyjson_obj_foreach(orig, idx, max, key, orig_val) */
            idx = 0;
            let o_max = if yyjson_is_obj(orig) {
                checked_yyjson_get_len(orig as *mut c_void)
            } else {
                0
            };
            key = if !orig.is_null() {
                checked_yyjson_get_first(orig)
            } else {
                null_mut()
            };
            orig_val = key.wrapping_add(1);
            while idx < o_max {
                patch_val = yyjson_obj_getn(
                    patch,
                    checked_yyjson_get_str(key as *mut c_void) as *mut u8,
                    checked_yyjson_get_len(key as *mut c_void));
                if patch_val.is_null() {
                    mut_key = crate::repr::node::yyjson_val_mut_copy(
                        doc, key as *const yyjson_val);
                    mut_val = crate::repr::node::yyjson_val_mut_copy(
                        doc, orig_val as *const yyjson_val);
                    if !yyjson_mut_obj_add(builder, mut_key, mut_val) {
                        return null_mut();
                    }
                }
                idx = idx.wrapping_add(1);
                key = checked_yyjson_get_next(orig_val);
                orig_val = key.wrapping_add(1);
            }
        }

        /* Merge items modified by the patch. */
        /* yyjson_obj_foreach(patch, idx, max, key, patch_val) */
        idx = 0;
        max = if yyjson_is_obj(patch) {
            checked_yyjson_get_len(patch as *mut c_void)
        } else {
            0
        };
        key = if !patch.is_null() {
            checked_yyjson_get_first(patch)
        } else {
            null_mut()
        };
        patch_val = key.wrapping_add(1);
        while idx < max {
            /* null indicates the field is removed. */
            if !checked_yyjson_is_null(patch_val as *mut c_void) {
                mut_key = crate::repr::node::yyjson_val_mut_copy(
                    doc, key as *const yyjson_val);
                orig_val = yyjson_obj_getn(
                    orig,
                    checked_yyjson_get_str(key as *mut c_void) as *mut u8,
                    checked_yyjson_get_len(key as *mut c_void));
                merged_val = yyjson_merge_patch(doc,
                                                orig_val as *const yyjson_val,
                                                patch_val as *const yyjson_val);
                if !yyjson_mut_obj_add(builder, mut_key, merged_val) {
                    return null_mut();
                }
            }
            idx = idx.wrapping_add(1);
            key = checked_yyjson_get_next(patch_val);
            patch_val = key.wrapping_add(1);
        }

        builder
    }
}

/// `yyjson_mut_val *yyjson_mut_merge_patch(yyjson_mut_doc *doc,
///                                         const yyjson_mut_val *orig,
///                                         const yyjson_mut_val *patch)`
/// yyjson.c:11428
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_merge_patch(doc: *mut yyjson_mut_doc,
                                                orig: *const yyjson_mut_val,
                                                patch: *const yyjson_mut_val)
    -> *mut yyjson_mut_val {
    unsafe {
        let mut idx: usize;
        let max: usize;
        let mut key: *mut yyjson_mut_val;
        let mut orig_val: *mut yyjson_mut_val;
        let mut patch_val: *mut yyjson_mut_val;
        let mut local_orig: yyjson_mut_val =
            core::mem::MaybeUninit::zeroed().assume_init();
        let builder: *mut yyjson_mut_val;
        let mut mut_key: *mut yyjson_mut_val;
        let mut mut_val: *mut yyjson_mut_val;
        let mut merged_val: *mut yyjson_mut_val;

        let mut orig = orig as *mut yyjson_mut_val;
        let patch = patch as *mut yyjson_mut_val;

        if !yyjson_mut_is_obj(patch) {
            return crate::repr::node::yyjson_mut_val_mut_copy(
                doc, patch as *const yyjson_mut_val);
        }

        builder = yyjson_mut_obj(doc);
        if builder.is_null() {
            return null_mut();
        }

        ffi::memset(&raw mut local_orig as *mut c_void, 0,
                    size_of::<yyjson_mut_val>());
        if !yyjson_mut_is_obj(orig) {
            local_orig.tag = (*builder).tag;
            local_orig.uni = (*builder).uni;
            orig = &raw mut local_orig;
        }

        /* If orig is contributing, copy any items not modified by the patch */
        if orig != &raw mut local_orig {
            /* yyjson_mut_obj_foreach(orig, idx, max, key, orig_val) */
            idx = 0;
            let o_max = yyjson_mut_obj_size(orig);
            key = if o_max != 0 {
                (*(*((*orig).uni.ptr as *mut yyjson_mut_val)).next).next
            } else {
                null_mut()
            };
            orig_val = if !key.is_null() { (*key).next } else { null_mut() };
            while idx < o_max {
                patch_val = yyjson_mut_obj_getn(
                    patch,
                    checked_yyjson_get_str(key as *mut c_void) as *mut u8,
                    checked_yyjson_get_len(key as *mut c_void));
                if patch_val.is_null() {
                    mut_key = crate::repr::node::yyjson_mut_val_mut_copy(
                        doc, key as *const yyjson_mut_val);
                    mut_val = crate::repr::node::yyjson_mut_val_mut_copy(
                        doc, orig_val as *const yyjson_mut_val);
                    if !yyjson_mut_obj_add(builder, mut_key, mut_val) {
                        return null_mut();
                    }
                }
                idx = idx.wrapping_add(1);
                key = (*orig_val).next;
                orig_val = (*key).next;
            }
        }

        /* Merge items modified by the patch. */
        /* yyjson_mut_obj_foreach(patch, idx, max, key, patch_val) */
        idx = 0;
        max = yyjson_mut_obj_size(patch);
        key = if max != 0 {
            (*(*((*patch).uni.ptr as *mut yyjson_mut_val)).next).next
        } else {
            null_mut()
        };
        patch_val = if !key.is_null() { (*key).next } else { null_mut() };
        while idx < max {
            /* null indicates the field is removed. */
            if !checked_yyjson_is_null(patch_val as *mut c_void) {
                mut_key = crate::repr::node::yyjson_mut_val_mut_copy(
                    doc, key as *const yyjson_mut_val);
                orig_val = yyjson_mut_obj_getn(
                    orig,
                    checked_yyjson_get_str(key as *mut c_void) as *mut u8,
                    checked_yyjson_get_len(key as *mut c_void));
                merged_val = yyjson_mut_merge_patch(
                    doc, orig_val as *const yyjson_mut_val,
                    patch_val as *const yyjson_mut_val);
                if !yyjson_mut_obj_add(builder, mut_key, merged_val) {
                    return null_mut();
                }
            }
            idx = idx.wrapping_add(1);
            key = (*patch_val).next;
            patch_val = (*key).next;
        }

        builder
    }
}
