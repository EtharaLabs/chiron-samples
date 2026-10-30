// FORGE-CANARY-BEGIN
// FORGE-CANARY-SLOT-1: 60ae0f0c079c366733615a2e6551dc4e64c530f1c56ffed9db295862b101ddc8
// FORGE-CANARY-SLOT-2: 87ab65e4ae3bd13b9cfb24e8b811c4ab101c8552a8b3d1f0d91c32bbd429958a
// FORGE-CANARY-SLOT-3: 76135f235d13f30c4446a51ada3f9a19b6fba2e250fa6b2ffc5a14d1b3f55ffb
// FORGE-CANARY-SLOT-4: e415e80ce167e9922758ef5d43162679b0fe51046883994e0fc30102c1e183f5
// FORGE-CANARY-END
/*==============================================================================
 * MARK: - hdr.rs
 *
 * The `yyjson_api_inline` functions of yyjson.h that yyjson.c calls, plus their
 * transitive inline callees. The seed list is port-spec's inline-used.txt (50
 * names); the transitive closure computed by port-tools/hdr_closure.py adds 10
 * more, for 60 functions in total. Each carries its yyjson.h line range.
 *
 * Conventions applied here:
 *  - same C names, same semantics;
 *  - every pointer parameter is `*mut` (the internal convention), so modules
 *    compose without casts; exported wrappers cast at the boundary;
 *  - where a header inline calls an exported function, the Rust exported
 *    function in its owning module is called (crate::value, crate::pointer).
 *
 * The C macros used by these functions are expanded by hand:
 *  - `yyjson_mut_val_one_str(func)`  (yyjson.h:6195)
 *  - `yyjson_ptr_set_err(code, msg)` (yyjson.h:7610)
 *  - `yyjson_constcast(type)`        (yyjson.h:362) -> a plain cast
 *============================================================================*/

use crate::ffi::{self, c_char, c_void};
use crate::types::*;

use core::mem::size_of;

/*==============================================================================
 * MARK: - Unsafe JSON Value API (yyjson.h:5056-5327)
 *============================================================================*/

/** yyjson.h:5056  `YYJSON_U64_TO_F64_NO_IMPL` is 0, so this is `(double)num`. */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_u64_to_f64(num: u64) -> f64 {
    num as f64
}

/** yyjson.h:5069 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_get_type(val: *mut c_void) -> yyjson_type {
    unsafe {
        let tag = (*(val as *mut yyjson_val)).tag as u8;
        tag & YYJSON_TYPE_MASK
    }
}

/** yyjson.h:5074 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_get_subtype(val: *mut c_void)
    -> yyjson_subtype {
    unsafe {
        let tag = (*(val as *mut yyjson_val)).tag as u8;
        tag & YYJSON_SUBTYPE_MASK
    }
}

/** yyjson.h:5079 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_get_tag(val: *mut c_void) -> u8 {
    unsafe {
        let tag = (*(val as *mut yyjson_val)).tag as u8;
        tag & YYJSON_TAG_MASK
    }
}

/** yyjson.h:5084 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_is_raw(val: *mut c_void) -> bool {
    unsafe { unsafe_yyjson_get_type(val) == YYJSON_TYPE_RAW }
}

/** yyjson.h:5088 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_is_null(val: *mut c_void) -> bool {
    unsafe { unsafe_yyjson_get_type(val) == YYJSON_TYPE_NULL }
}

/** yyjson.h:5100 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_is_str(val: *mut c_void) -> bool {
    unsafe { unsafe_yyjson_get_type(val) == YYJSON_TYPE_STR }
}

/** yyjson.h:5104 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_is_arr(val: *mut c_void) -> bool {
    unsafe { unsafe_yyjson_get_type(val) == YYJSON_TYPE_ARR }
}

/** yyjson.h:5108 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_is_obj(val: *mut c_void) -> bool {
    unsafe { unsafe_yyjson_get_type(val) == YYJSON_TYPE_OBJ }
}

/** yyjson.h:5112 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_is_ctn(val: *mut c_void) -> bool {
    unsafe {
        let mask = YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ;
        (unsafe_yyjson_get_tag(val) & mask) == mask
    }
}

/** yyjson.h:5148 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_arr_is_flat(val: *mut yyjson_val) -> bool {
    unsafe {
        let ofs = (*val).uni.ofs;
        let len = ((*val).tag >> YYJSON_TAG_BIT) as usize;
        len.wrapping_mul(size_of::<yyjson_val>())
            .wrapping_add(size_of::<yyjson_val>())
            == ofs
    }
}

/** yyjson.h:5158 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_get_bool(val: *mut c_void) -> bool {
    unsafe {
        let tag = unsafe_yyjson_get_tag(val);
        ((tag & YYJSON_SUBTYPE_MASK) >> YYJSON_TYPE_BIT) != 0
    }
}

/** yyjson.h:5191 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_get_str(val: *mut c_void) -> *const c_char {
    unsafe { (*(val as *mut yyjson_val)).uni.str_ }
}

/** yyjson.h:5195 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_get_len(val: *mut c_void) -> usize {
    unsafe { ((*(val as *mut yyjson_val)).tag >> YYJSON_TAG_BIT) as usize }
}

/** yyjson.h:5199  `yyjson_constcast(yyjson_val *)ctn + 1` */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_get_first(ctn: *mut yyjson_val)
    -> *mut yyjson_val {
    unsafe { ctn.add(1) }
}

/** yyjson.h:5203 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_get_next(val: *mut yyjson_val)
    -> *mut yyjson_val {
    unsafe {
        let is_ctn = unsafe_yyjson_is_ctn(val as *mut c_void);
        let ctn_ofs = (*val).uni.ofs;
        let ofs = if is_ctn { ctn_ofs } else { size_of::<yyjson_val>() };
        let ptr = val as *mut u8;
        ptr.add(ofs) as *mut yyjson_val
    }
}

/** yyjson.h:5211 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_equals_strn(val: *mut c_void,
                                               str_: *mut u8,
                                               len: usize) -> bool {
    unsafe {
        unsafe_yyjson_get_len(val) == len
            && ffi::memcmp((*(val as *mut yyjson_val)).uni.str_ as *const c_void,
                           str_ as *const c_void, len) == 0
    }
}

/** yyjson.h:5230 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_set_len(val: *mut c_void, len: usize) {
    unsafe {
        let mut tag =
            (*(val as *mut yyjson_val)).tag & (YYJSON_TAG_MASK as u64);
        tag |= (len as u64).wrapping_shl(YYJSON_TAG_BIT as u32);
        (*(val as *mut yyjson_val)).tag = tag;
    }
}

/** yyjson.h:5236 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_set_tag(val: *mut c_void,
                                           type_: yyjson_type,
                                           subtype: yyjson_subtype,
                                           len: usize) {
    unsafe {
        let mut tag = (len as u64).wrapping_shl(YYJSON_TAG_BIT as u32);
        tag |= (type_ | subtype) as u64;
        (*(val as *mut yyjson_val)).tag = tag;
    }
}

/** yyjson.h:5307 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_set_strn(val: *mut c_void,
                                            str_: *const c_char,
                                            len: usize) {
    unsafe {
        unsafe_yyjson_set_tag(val, YYJSON_TYPE_STR, YYJSON_SUBTYPE_NONE, len);
        (*(val as *mut yyjson_val)).uni.str_ = str_;
    }
}

/*==============================================================================
 * MARK: - JSON Value Type API (yyjson.h:5362-5412)
 *============================================================================*/

/** yyjson.h:5362 */
#[inline(always)]
pub(crate) unsafe fn yyjson_is_raw(val: *mut yyjson_val) -> bool {
    unsafe {
        if !val.is_null() { unsafe_yyjson_is_raw(val as *mut c_void) } else { false }
    }
}

/** yyjson.h:5402 */
#[inline(always)]
pub(crate) unsafe fn yyjson_is_str(val: *mut yyjson_val) -> bool {
    unsafe {
        if !val.is_null() { unsafe_yyjson_is_str(val as *mut c_void) } else { false }
    }
}

/** yyjson.h:5406 */
#[inline(always)]
pub(crate) unsafe fn yyjson_is_arr(val: *mut yyjson_val) -> bool {
    unsafe {
        if !val.is_null() { unsafe_yyjson_is_arr(val as *mut c_void) } else { false }
    }
}

/** yyjson.h:5410 */
#[inline(always)]
pub(crate) unsafe fn yyjson_is_obj(val: *mut yyjson_val) -> bool {
    unsafe {
        if !val.is_null() { unsafe_yyjson_is_obj(val as *mut c_void) } else { false }
    }
}

/*==============================================================================
 * MARK: - Array / Object Iterator API (yyjson.h:5662-5799)
 *============================================================================*/

/** yyjson.h:5662 */
#[inline(always)]
pub(crate) unsafe fn yyjson_arr_iter_init(arr: *mut yyjson_val,
                                          iter: *mut yyjson_arr_iter) -> bool {
    unsafe {
        if yyjson_is_arr(arr) && !iter.is_null() {
            (*iter).idx = 0;
            (*iter).max = unsafe_yyjson_get_len(arr as *mut c_void);
            (*iter).cur = unsafe_yyjson_get_first(arr);
            return true;
        }
        if !iter.is_null() {
            ffi::memset(iter as *mut c_void, 0, size_of::<yyjson_arr_iter>());
        }
        false
    }
}

/** yyjson.h:5684 */
#[inline(always)]
pub(crate) unsafe fn yyjson_arr_iter_next(iter: *mut yyjson_arr_iter)
    -> *mut yyjson_val {
    unsafe {
        if !iter.is_null() && (*iter).idx < (*iter).max {
            let val = (*iter).cur;
            (*iter).cur = unsafe_yyjson_get_next(val);
            (*iter).idx = (*iter).idx.wrapping_add(1);
            return val;
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:5705  `strlen(key)` is open-coded; the C code calls libc's. */
#[inline(always)]
pub(crate) unsafe fn yyjson_obj_get(obj: *mut yyjson_val, key: *mut u8)
    -> *mut yyjson_val {
    unsafe {
        let len = if !key.is_null() { strlen(key) } else { 0 };
        yyjson_obj_getn(obj, key, len)
    }
}

/** yyjson.h:5710 */
#[inline(always)]
pub(crate) unsafe fn yyjson_obj_getn(obj: *mut yyjson_val, _key: *mut u8,
                                     key_len: usize) -> *mut yyjson_val {
    unsafe {
        if yyjson_is_obj(obj) && !_key.is_null() {
            let mut len = unsafe_yyjson_get_len(obj as *mut c_void);
            let mut key = unsafe_yyjson_get_first(obj);
            /* C: `while (len-- > 0)` - post-decrement, so the loop runs `len`
               times and `len` wraps to USIZE_MAX on the final test. */
            while len > 0 {
                len = len.wrapping_sub(1);
                if unsafe_yyjson_equals_strn(key as *mut c_void, _key, key_len) {
                    return key.add(1);
                }
                key = unsafe_yyjson_get_next(key.add(1));
            }
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:5730 */
#[inline(always)]
pub(crate) unsafe fn yyjson_obj_iter_init(obj: *mut yyjson_val,
                                          iter: *mut yyjson_obj_iter) -> bool {
    unsafe {
        if yyjson_is_obj(obj) && !iter.is_null() {
            (*iter).idx = 0;
            (*iter).max = unsafe_yyjson_get_len(obj as *mut c_void);
            (*iter).cur = unsafe_yyjson_get_first(obj);
            (*iter).obj = obj;
            return true;
        }
        if !iter.is_null() {
            ffi::memset(iter as *mut c_void, 0, size_of::<yyjson_obj_iter>());
        }
        false
    }
}

/** yyjson.h:5772 */
#[inline(always)]
pub(crate) unsafe fn yyjson_obj_iter_getn(iter: *mut yyjson_obj_iter,
                                          key: *mut u8,
                                          key_len: usize) -> *mut yyjson_val {
    unsafe {
        if !iter.is_null() && !key.is_null() {
            let mut idx = (*iter).idx;
            let mut max = (*iter).max;
            let mut cur = (*iter).cur;
            if idx == max {
                idx = 0;
                cur = unsafe_yyjson_get_first((*iter).obj);
            }
            /* C: `while (idx++ < max)` - the comparison uses the pre-increment
               value and `idx` is incremented regardless. */
            while idx < max {
                idx = idx.wrapping_add(1);
                let next = unsafe_yyjson_get_next(cur.add(1));
                if unsafe_yyjson_equals_strn(cur as *mut c_void, key, key_len) {
                    (*iter).idx = idx;
                    (*iter).cur = next;
                    return cur.add(1);
                }
                cur = next;
                if idx == (*iter).max && (*iter).idx < (*iter).max {
                    idx = 0;
                    max = (*iter).idx;
                    cur = unsafe_yyjson_get_first((*iter).obj);
                }
            }
        }
        core::ptr::null_mut()
    }
}

/*==============================================================================
 * MARK: - Mutable pools and value allocation (yyjson.h:5878-5919)
 *
 * These call the exported `unsafe_yyjson_str_pool_grow` /
 * `unsafe_yyjson_val_pool_grow`, which `crate::value` owns.
 *============================================================================*/

/** yyjson.h:5878  Allocate memory for string. */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_mut_str_alc(doc: *mut yyjson_mut_doc,
                                               len: usize) -> *mut c_char {
    unsafe {
        let mem: *mut c_char;
        let alc: *mut yyjson_alc = &raw mut (*doc).alc;
        let pool: *mut yyjson_str_pool = &raw mut (*doc).str_pool;
        /* `len + 1` is used below to reserve space for a null terminator;
           reject the value that would wrap it to 0. */
        if len == !0usize {
            return core::ptr::null_mut();
        }
        if ((*pool).end as usize).wrapping_sub((*pool).cur as usize) <= len {
            if !crate::value::unsafe_yyjson_str_pool_grow(
                pool, alc as *const yyjson_alc, len.wrapping_add(1))
            {
                return core::ptr::null_mut();
            }
        }
        mem = (*pool).cur;
        (*pool).cur = mem.add(len).add(1);
        mem
    }
}

/** yyjson.h:5897 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_mut_strncpy(doc: *mut yyjson_mut_doc,
                                               str_: *mut u8,
                                               len: usize) -> *mut c_char {
    unsafe {
        let mem = unsafe_yyjson_mut_str_alc(doc, len);
        if mem.is_null() {
            return core::ptr::null_mut();
        }
        ffi::memcpy(mem as *mut c_void, str_ as *const c_void, len);
        *mem.add(len) = 0;
        mem
    }
}

/** yyjson.h:5906 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_mut_val(doc: *mut yyjson_mut_doc,
                                           count: usize)
    -> *mut yyjson_mut_val {
    unsafe {
        let val: *mut yyjson_mut_val;
        let alc: *mut yyjson_alc = &raw mut (*doc).alc;
        let pool: *mut yyjson_val_pool = &raw mut (*doc).val_pool;
        /* C: `(usize)(pool->end - pool->cur) < count` - a pointer difference in
           units of yyjson_mut_val. */
        if (((*pool).end as usize).wrapping_sub((*pool).cur as usize)
            / size_of::<yyjson_mut_val>())
            < count
        {
            if !crate::value::unsafe_yyjson_val_pool_grow(
                pool, alc as *const yyjson_alc, count)
            {
                return core::ptr::null_mut();
            }
        }
        val = (*pool).cur;
        (*pool).cur = (*pool).cur.add(count);
        val
    }
}

/*==============================================================================
 * MARK: - Mutable JSON Document API (yyjson.h:5931)
 *============================================================================*/

/** yyjson.h:5931 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_doc_set_root(doc: *mut yyjson_mut_doc,
                                             root: *mut yyjson_mut_val) {
    unsafe {
        if !doc.is_null() {
            (*doc).root = root;
        }
    }
}

/*==============================================================================
 * MARK: - Mutable JSON Value Type API (yyjson.h:5982-5992)
 *============================================================================*/

/** yyjson.h:5982 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_is_str(val: *mut yyjson_mut_val) -> bool {
    unsafe {
        if !val.is_null() { unsafe_yyjson_is_str(val as *mut c_void) } else { false }
    }
}

/** yyjson.h:5986 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_is_arr(val: *mut yyjson_mut_val) -> bool {
    unsafe {
        if !val.is_null() { unsafe_yyjson_is_arr(val as *mut c_void) } else { false }
    }
}

/** yyjson.h:5990 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_is_obj(val: *mut yyjson_mut_val) -> bool {
    unsafe {
        if !val.is_null() { unsafe_yyjson_is_obj(val as *mut c_void) } else { false }
    }
}

/*==============================================================================
 * MARK: - Mutable value equality (yyjson.h:6071)
 *============================================================================*/

/** yyjson.h:6071  Calls the exported `unsafe_yyjson_mut_equals`
    (owned by crate::value). */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_equals(lhs: *mut yyjson_mut_val,
                                       rhs: *mut yyjson_mut_val) -> bool {
    unsafe {
        if lhs.is_null() || rhs.is_null() {
            return false;
        }
        crate::value::unsafe_yyjson_mut_equals(lhs, rhs)
    }
}

/*==============================================================================
 * MARK: - Mutable string creation (yyjson.h:6288-6315)
 *
 * `yyjson_mut_val_one_str(func)` (yyjson.h:6195) expanded by hand:
 *     if (doc && str) { val = unsafe_yyjson_mut_val(doc, 1);
 *                       if (val) { <func>; return val; } }
 *     return NULL;
 *============================================================================*/

/** yyjson.h:6288 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_strn(doc: *mut yyjson_mut_doc, str_: *mut u8,
                                     len: usize) -> *mut yyjson_mut_val {
    unsafe {
        if !doc.is_null() && !str_.is_null() {
            let val = unsafe_yyjson_mut_val(doc, 1);
            if !val.is_null() {
                unsafe_yyjson_set_strn(val as *mut c_void,
                                       str_ as *const c_char, len);
                return val;
            }
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:6307 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_strncpy(doc: *mut yyjson_mut_doc,
                                        str_: *mut u8,
                                        len: usize) -> *mut yyjson_mut_val {
    unsafe {
        if !doc.is_null() && !str_.is_null() {
            let val = unsafe_yyjson_mut_val(doc, 1);
            if !val.is_null() {
                let new_str = unsafe_yyjson_mut_strncpy(doc, str_, len);
                if new_str.is_null() {
                    return core::ptr::null_mut();
                }
                unsafe_yyjson_set_strn(val as *mut c_void, new_str, len);
                return val;
            }
        }
        core::ptr::null_mut()
    }
}

/*==============================================================================
 * MARK: - Mutable Array API (yyjson.h:6362-6652)
 *============================================================================*/

/** yyjson.h:6362 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_arr_iter_init(arr: *mut yyjson_mut_val,
                                              iter: *mut yyjson_mut_arr_iter)
    -> bool {
    unsafe {
        if yyjson_mut_is_arr(arr) && !iter.is_null() {
            (*iter).idx = 0;
            (*iter).max = unsafe_yyjson_get_len(arr as *mut c_void);
            (*iter).cur = if (*iter).max != 0 {
                (*arr).uni.ptr as *mut yyjson_mut_val
            } else {
                core::ptr::null_mut()
            };
            (*iter).pre = core::ptr::null_mut();
            (*iter).arr = arr;
            return true;
        }
        if !iter.is_null() {
            ffi::memset(iter as *mut c_void, 0,
                        size_of::<yyjson_mut_arr_iter>());
        }
        false
    }
}

/** yyjson.h:6387 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_arr_iter_next(iter: *mut yyjson_mut_arr_iter)
    -> *mut yyjson_mut_val {
    unsafe {
        if !iter.is_null() && (*iter).idx < (*iter).max {
            let val = (*iter).cur;
            (*iter).pre = val;
            (*iter).cur = (*val).next;
            (*iter).idx = (*iter).idx.wrapping_add(1);
            return (*iter).cur;
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:6635 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_arr_append(arr: *mut yyjson_mut_val,
                                           val: *mut yyjson_mut_val) -> bool {
    unsafe {
        if yyjson_mut_is_arr(arr) && !val.is_null() {
            let len = unsafe_yyjson_get_len(arr as *mut c_void);
            unsafe_yyjson_set_len(arr as *mut c_void, len.wrapping_add(1));
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
}

/*==============================================================================
 * MARK: - Mutable Object API (yyjson.h:6980-7299)
 *============================================================================*/

/** yyjson.h:6980 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_size(obj: *mut yyjson_mut_val) -> usize {
    unsafe {
        if yyjson_mut_is_obj(obj) {
            unsafe_yyjson_get_len(obj as *mut c_void)
        } else {
            0
        }
    }
}

/** yyjson.h:6984 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_get(obj: *mut yyjson_mut_val,
                                        key: *mut u8) -> *mut yyjson_mut_val {
    unsafe {
        let len = if !key.is_null() { strlen(key) } else { 0 };
        yyjson_mut_obj_getn(obj, key, len)
    }
}

/** yyjson.h:6989 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_getn(obj: *mut yyjson_mut_val,
                                         _key: *mut u8,
                                         key_len: usize)
    -> *mut yyjson_mut_val {
    unsafe {
        let mut len = yyjson_mut_obj_size(obj);
        if len != 0 && !_key.is_null() {
            let mut key =
                (*(*((*obj).uni.ptr as *mut yyjson_mut_val)).next).next;
            while len > 0 {
                len = len.wrapping_sub(1);
                if unsafe_yyjson_equals_strn(key as *mut c_void, _key, key_len) {
                    return (*key).next;
                }
                key = (*(*key).next).next;
            }
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:7009 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_iter_init(obj: *mut yyjson_mut_val,
                                              iter: *mut yyjson_mut_obj_iter)
    -> bool {
    unsafe {
        if yyjson_mut_is_obj(obj) && !iter.is_null() {
            (*iter).idx = 0;
            (*iter).max = unsafe_yyjson_get_len(obj as *mut c_void);
            (*iter).cur = if (*iter).max != 0 {
                (*obj).uni.ptr as *mut yyjson_mut_val
            } else {
                core::ptr::null_mut()
            };
            (*iter).pre = core::ptr::null_mut();
            (*iter).obj = obj;
            return true;
        }
        if !iter.is_null() {
            ffi::memset(iter as *mut c_void, 0,
                        size_of::<yyjson_mut_obj_iter>());
        }
        false
    }
}

/** yyjson.h:7034 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_iter_next(iter: *mut yyjson_mut_obj_iter)
    -> *mut yyjson_mut_val {
    unsafe {
        if !iter.is_null() && (*iter).idx < (*iter).max {
            let key = (*iter).cur;
            (*iter).pre = key;
            (*iter).cur = (*(*key).next).next;
            (*iter).idx = (*iter).idx.wrapping_add(1);
            return (*iter).cur;
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:7051 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_iter_remove(iter: *mut yyjson_mut_obj_iter)
    -> *mut yyjson_mut_val {
    unsafe {
        if !iter.is_null() && !(*iter).pre.is_null()
            && 0 < (*iter).idx && (*iter).idx <= (*iter).max
        {
            let prev = (*iter).pre;
            let cur = (*iter).cur;
            let next = (*(*cur).next).next;
            if (*iter).idx == (*iter).max {
                (*(*iter).obj).uni.ptr = prev as *mut c_void;
            }
            (*iter).idx = (*iter).idx.wrapping_sub(1);
            (*iter).max = (*iter).max.wrapping_sub(1);
            unsafe_yyjson_set_len((*iter).obj as *mut c_void, (*iter).max);
            (*(*prev).next).next = next;
            (*iter).cur = prev;
            (*iter).pre = core::ptr::null_mut();
            return (*cur).next;
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:7075 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_iter_getn(iter: *mut yyjson_mut_obj_iter,
                                              key: *mut u8,
                                              key_len: usize)
    -> *mut yyjson_mut_val {
    unsafe {
        if !iter.is_null() && !key.is_null() {
            let mut idx: usize = 0;
            let max = (*iter).max;
            let mut pre: *mut yyjson_mut_val;
            let mut cur = (*iter).cur;
            while idx < max {
                idx = idx.wrapping_add(1);
                pre = cur;
                cur = (*(*cur).next).next;
                if unsafe_yyjson_equals_strn(cur as *mut c_void, key, key_len) {
                    (*iter).idx = (*iter).idx.wrapping_add(idx);
                    if (*iter).idx > max {
                        (*iter).idx = (*iter).idx.wrapping_sub(max);
                    }
                    (*iter).pre = pre;
                    (*iter).cur = cur;
                    return (*cur).next;
                }
            }
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:7102 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj(doc: *mut yyjson_mut_doc)
    -> *mut yyjson_mut_val {
    unsafe {
        if !doc.is_null() {
            let val = unsafe_yyjson_mut_val(doc, 1);
            if !val.is_null() {
                (*val).tag = (YYJSON_TYPE_OBJ | YYJSON_SUBTYPE_NONE) as u64;
                return val;
            }
        }
        core::ptr::null_mut()
    }
}

/** yyjson.h:7190 */
#[inline(always)]
pub(crate) unsafe fn unsafe_yyjson_mut_obj_add(obj: *mut yyjson_mut_val,
                                               key: *mut yyjson_mut_val,
                                               val: *mut yyjson_mut_val,
                                               len: usize) {
    unsafe {
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
        unsafe_yyjson_set_len(obj as *mut c_void, len.wrapping_add(1));
    }
}

/** yyjson.h:7264 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_add(obj: *mut yyjson_mut_val,
                                        key: *mut yyjson_mut_val,
                                        val: *mut yyjson_mut_val) -> bool {
    unsafe {
        if yyjson_mut_is_obj(obj) && yyjson_mut_is_str(key) && !val.is_null() {
            unsafe_yyjson_mut_obj_add(obj, key, val,
                                      unsafe_yyjson_get_len(obj as *mut c_void));
            return true;
        }
        false
    }
}

/** yyjson.h:7275 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_obj_put(obj: *mut yyjson_mut_val,
                                        key: *mut yyjson_mut_val,
                                        val: *mut yyjson_mut_val) -> bool {
    unsafe {
        let mut replaced = false;
        let key_len: usize;
        let mut iter: yyjson_mut_obj_iter =
            core::mem::MaybeUninit::zeroed().assume_init();
        let mut cur_key: *mut yyjson_mut_val;
        if !yyjson_mut_is_obj(obj) || !yyjson_mut_is_str(key) {
            return false;
        }
        key_len = unsafe_yyjson_get_len(key as *mut c_void);
        yyjson_mut_obj_iter_init(obj, &raw mut iter);
        loop {
            cur_key = yyjson_mut_obj_iter_next(&raw mut iter);
            if cur_key.is_null() {
                break;
            }
            if unsafe_yyjson_equals_strn(cur_key as *mut c_void,
                                         (*key).uni.str_ as *mut u8, key_len) {
                if !replaced && !val.is_null() {
                    replaced = true;
                    (*val).next = (*(*cur_key).next).next;
                    (*cur_key).next = val;
                } else {
                    yyjson_mut_obj_iter_remove(&raw mut iter);
                }
            }
        }
        if !replaced && !val.is_null() {
            unsafe_yyjson_mut_obj_add(obj, key, val, iter.max);
        }
        true
    }
}

/*==============================================================================
 * MARK: - JSON Pointer API (yyjson.h:7760-8256)
 *
 * `yyjson_ptr_set_err(_code, _msg)` (yyjson.h:7610) expanded by hand:
 *     if (err) { err->code = YYJSON_PTR_ERR_##_code;
 *                err->msg = _msg; err->pos = 0; }
 *
 * The `unsafe_yyjson_mut_ptr_*` callees are exported functions owned by
 * crate::pointer.
 *============================================================================*/

/** Expansion of `yyjson_ptr_set_err`. */
#[inline(always)]
unsafe fn yyjson_ptr_set_err(err: *mut yyjson_ptr_err, code: yyjson_ptr_code,
                             msg: *const c_char) {
    unsafe {
        if !err.is_null() {
            (*err).code = code;
            (*err).msg = msg;
            (*err).pos = 0;
        }
    }
}

/** yyjson.h:7760 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_ptr_getx(val: *mut yyjson_mut_val,
                                         ptr: *mut u8, len: usize,
                                         ctx: *mut yyjson_ptr_ctx,
                                         err: *mut yyjson_ptr_err)
    -> *mut yyjson_mut_val {
    unsafe {
        yyjson_ptr_set_err(err, YYJSON_PTR_ERR_NONE, core::ptr::null());
        if !ctx.is_null() {
            ffi::memset(ctx as *mut c_void, 0, size_of::<yyjson_ptr_ctx>());
        }

        if val.is_null() || ptr.is_null() {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_PARAMETER,
                               c"input parameter is NULL".as_ptr());
            return core::ptr::null_mut();
        }
        if len == 0 {
            return val;
        }
        if *ptr != b'/' {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                               c"no prefix '/'".as_ptr());
            return core::ptr::null_mut();
        }
        crate::pointer::unsafe_yyjson_mut_ptr_getx(
            val as *const yyjson_mut_val, ptr as *const c_char, len, ctx, err)
    }
}

/** yyjson.h:7858 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_ptr_addx(val: *mut yyjson_mut_val,
                                         ptr: *mut u8, len: usize,
                                         new_val: *mut yyjson_mut_val,
                                         doc: *mut yyjson_mut_doc,
                                         create_parent: bool,
                                         ctx: *mut yyjson_ptr_ctx,
                                         err: *mut yyjson_ptr_err) -> bool {
    unsafe {
        yyjson_ptr_set_err(err, YYJSON_PTR_ERR_NONE, core::ptr::null());
        if !ctx.is_null() {
            ffi::memset(ctx as *mut c_void, 0, size_of::<yyjson_ptr_ctx>());
        }

        if val.is_null() || ptr.is_null() || new_val.is_null() || doc.is_null() {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_PARAMETER,
                               c"input parameter is NULL".as_ptr());
            return false;
        }
        if len == 0 {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_SET_ROOT,
                               c"cannot set root".as_ptr());
            return false;
        }
        if *ptr != b'/' {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                               c"no prefix '/'".as_ptr());
            return false;
        }
        crate::pointer::unsafe_yyjson_mut_ptr_putx(val, ptr as *const c_char,
                                                   len, new_val, doc,
                                                   create_parent, true, ctx,
                                                   err)
    }
}

/** yyjson.h:8046 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_ptr_replacex(val: *mut yyjson_mut_val,
                                             ptr: *mut u8, len: usize,
                                             new_val: *mut yyjson_mut_val,
                                             ctx: *mut yyjson_ptr_ctx,
                                             err: *mut yyjson_ptr_err)
    -> *mut yyjson_mut_val {
    unsafe {
        yyjson_ptr_set_err(err, YYJSON_PTR_ERR_NONE, core::ptr::null());
        if !ctx.is_null() {
            ffi::memset(ctx as *mut c_void, 0, size_of::<yyjson_ptr_ctx>());
        }

        if val.is_null() || ptr.is_null() || new_val.is_null() {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_PARAMETER,
                               c"input parameter is NULL".as_ptr());
            return core::ptr::null_mut();
        }
        if len == 0 {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_SET_ROOT,
                               c"cannot set root".as_ptr());
            return core::ptr::null_mut();
        }
        if *ptr != b'/' {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                               c"no prefix '/'".as_ptr());
            return core::ptr::null_mut();
        }
        crate::pointer::unsafe_yyjson_mut_ptr_replacex(
            val, ptr as *const c_char, len, new_val, ctx, err)
    }
}

/** yyjson.h:8119 */
#[inline(always)]
pub(crate) unsafe fn yyjson_mut_ptr_removex(val: *mut yyjson_mut_val,
                                            ptr: *mut u8, len: usize,
                                            ctx: *mut yyjson_ptr_ctx,
                                            err: *mut yyjson_ptr_err)
    -> *mut yyjson_mut_val {
    unsafe {
        yyjson_ptr_set_err(err, YYJSON_PTR_ERR_NONE, core::ptr::null());
        if !ctx.is_null() {
            ffi::memset(ctx as *mut c_void, 0, size_of::<yyjson_ptr_ctx>());
        }

        if val.is_null() || ptr.is_null() {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_PARAMETER,
                               c"input parameter is NULL".as_ptr());
            return core::ptr::null_mut();
        }
        if len == 0 {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_SET_ROOT,
                               c"cannot set root".as_ptr());
            return core::ptr::null_mut();
        }
        if *ptr != b'/' {
            yyjson_ptr_set_err(err, YYJSON_PTR_ERR_SYNTAX,
                               c"no prefix '/'".as_ptr());
            return core::ptr::null_mut();
        }
        crate::pointer::unsafe_yyjson_mut_ptr_removex(
            val, ptr as *const c_char, len, ctx, err)
    }
}

/** yyjson.h:8196 */
#[inline(always)]
pub(crate) unsafe fn yyjson_ptr_ctx_replace(ctx: *mut yyjson_ptr_ctx,
                                            val: *mut yyjson_mut_val) -> bool {
    unsafe {
        let ctn: *mut yyjson_mut_val;
        let pre_key: *mut yyjson_mut_val;
        let cur_key: *mut yyjson_mut_val;
        let pre_val: *mut yyjson_mut_val;
        let cur_val: *mut yyjson_mut_val;
        if ctx.is_null() || (*ctx).ctn.is_null() || (*ctx).pre.is_null()
            || val.is_null()
        {
            return false;
        }
        ctn = (*ctx).ctn;
        if yyjson_mut_is_obj(ctn) {
            pre_key = (*ctx).pre;
            pre_val = (*pre_key).next;
            cur_key = (*pre_val).next;
            cur_val = (*cur_key).next;
            /* replace current value */
            (*cur_key).next = val;
            (*val).next = (*cur_val).next;
            (*ctx).old = cur_val;
        } else {
            pre_val = (*ctx).pre;
            cur_val = (*pre_val).next;
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
}

/** yyjson.h:8228 */
#[inline(always)]
pub(crate) unsafe fn yyjson_ptr_ctx_remove(ctx: *mut yyjson_ptr_ctx) -> bool {
    unsafe {
        let ctn: *mut yyjson_mut_val;
        let pre_key: *mut yyjson_mut_val;
        let pre_val: *mut yyjson_mut_val;
        let cur_key: *mut yyjson_mut_val;
        let cur_val: *mut yyjson_mut_val;
        let len: usize;
        if ctx.is_null() || (*ctx).ctn.is_null() || (*ctx).pre.is_null() {
            return false;
        }
        ctn = (*ctx).ctn;
        if yyjson_mut_is_obj(ctn) {
            pre_key = (*ctx).pre;
            pre_val = (*pre_key).next;
            cur_key = (*pre_val).next;
            cur_val = (*cur_key).next;
            /* remove current key-value */
            (*pre_val).next = (*cur_val).next;
            if (*ctn).uni.ptr == cur_key as *mut c_void {
                (*ctn).uni.ptr = pre_key as *mut c_void;
            }
            (*ctx).pre = core::ptr::null_mut();
            (*ctx).old = cur_val;
        } else {
            pre_val = (*ctx).pre;
            cur_val = (*pre_val).next;
            /* remove current key-value */
            (*pre_val).next = (*cur_val).next;
            if (*ctn).uni.ptr == cur_val as *mut c_void {
                (*ctn).uni.ptr = pre_val as *mut c_void;
            }
            (*ctx).pre = core::ptr::null_mut();
            (*ctx).old = cur_val;
        }
        len = unsafe_yyjson_get_len(ctn as *mut c_void).wrapping_sub(1);
        if len == 0 {
            (*ctn).uni.ptr = core::ptr::null_mut();
        }
        unsafe_yyjson_set_len(ctn as *mut c_void, len);
        true
    }
}

/*==============================================================================
 * MARK: - strlen
 *
 * yyjson.h's `yyjson_obj_get` / `yyjson_mut_obj_get` call libc's `strlen`.
 * That symbol is not among the C library's imports (GCC constant-folds or
 * inlines every call in the State A build), so it is not declared in
 * crate::ffi; the loop below is yyjson.h's own `yyjson_strlen` fallback
 * (yyjson.h:431) and computes the identical value.
 *============================================================================*/

#[inline(always)]
pub(crate) unsafe fn strlen(s: *mut u8) -> usize {
    unsafe {
        let mut p = s;
        while *p != 0 {
            p = p.add(1);
        }
        p.offset_from(s) as usize
    }
}
