// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 * MARK: - value.rs
 *
 * Owns yyjson.c 2564-3119 ("JSON Struct Utils (Public)"): the string and value
 * memory pools, mutable document creation / freeing, every copy function, the
 * two equality functions and `yyjson_locate_pos`.
 *
 * Conventions (../port-spec/CONVENTIONS.md):
 *  - `static_inline` -> `#[inline(always)] pub(crate) unsafe fn`,
 *    plain `static` -> `#[inline] pub(crate) unsafe fn`;
 *  - every internal pointer parameter is `*mut`; the exported wrappers carry
 *    the exact C constness and cast at the boundary;
 *  - allocation sizes are reproduced bit for bit, because the allocator
 *    callbacks are observable.
 *============================================================================*/

use crate::abi::libc::{self as ffi, c_char, c_void};
use crate::repr::shape::*;

use crate::mem::pools::YYJSON_DEFAULT_ALC;
use crate::repr::inline::{
    checked_yyjson_get_first, checked_yyjson_get_len, checked_yyjson_get_next,
    checked_yyjson_get_str, checked_yyjson_get_subtype, checked_yyjson_get_type,
    checked_yyjson_mut_strncpy, checked_yyjson_mut_val, yyjson_mut_doc_set_root,
    yyjson_mut_obj_iter_getn, yyjson_mut_obj_iter_init, yyjson_obj_iter_getn,
    yyjson_obj_iter_init,
};
use crate::mem::bytes::{
    test_utf8_bom, size_align_up, yyjson_max, yyjson_min,
    USIZE_MAX, YYJSON_MUT_DOC_STR_POOL_INIT_SIZE,
    YYJSON_MUT_DOC_STR_POOL_MAX_SIZE, YYJSON_MUT_DOC_VAL_POOL_INIT_SIZE,
    YYJSON_MUT_DOC_VAL_POOL_MAX_SIZE,
};

use core::mem::size_of;
use core::ptr::null_mut;

/*==============================================================================
 * MARK: - Pool release (yyjson.c:2569-2587)
 *============================================================================*/

/** yyjson.c:2569 */
#[inline(always)]
pub(crate) unsafe fn checked_yyjson_str_pool_release(pool: *mut yyjson_str_pool,
                                                    alc: *mut yyjson_alc) {
    unsafe {
        let mut chunk: *mut yyjson_str_chunk = (*pool).chunks;
        let mut next: *mut yyjson_str_chunk;
        while !chunk.is_null() {
            next = (*chunk).next;
            ((*alc).free.unwrap_unchecked())((*alc).ctx, chunk as *mut c_void);
            chunk = next;
        }
    }
}

/** yyjson.c:2578 */
#[inline(always)]
pub(crate) unsafe fn checked_yyjson_val_pool_release(pool: *mut yyjson_val_pool,
                                                    alc: *mut yyjson_alc) {
    unsafe {
        let mut chunk: *mut yyjson_val_chunk = (*pool).chunks;
        let mut next: *mut yyjson_val_chunk;
        while !chunk.is_null() {
            next = (*chunk).next;
            ((*alc).free.unwrap_unchecked())((*alc).ctx, chunk as *mut c_void);
            chunk = next;
        }
    }
}

/*==============================================================================
 * MARK: - Pool growth (yyjson.c:2589-2640)
 *============================================================================*/

/// `bool unsafe_yyjson_str_pool_grow(yyjson_str_pool *pool,
///                                   const yyjson_alc *alc, size_t len)`
/// yyjson.c:2589
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_str_pool_grow(
    pool: *mut yyjson_str_pool,
    alc: *const yyjson_alc,
    len: usize,
) -> bool {
    unsafe {
        let chunk: *mut yyjson_str_chunk;
        let mut size: usize;
        let max_len: usize;

        /* create a new chunk */
        max_len = USIZE_MAX - size_of::<yyjson_str_chunk>();
        if len > max_len {
            return false;
        }
        size = len.wrapping_add(size_of::<yyjson_str_chunk>());
        size = yyjson_max((*pool).chunk_size, size);
        chunk = ((*alc).malloc.unwrap_unchecked())((*alc).ctx, size)
            as *mut yyjson_str_chunk;
        if chunk.is_null() {
            return false;
        }

        /* insert the new chunk as the head of the linked list */
        (*chunk).next = (*pool).chunks;
        (*chunk).chunk_size = size;
        (*pool).chunks = chunk;
        (*pool).cur = (chunk as *mut u8)
            .add(size_of::<yyjson_str_chunk>()) as *mut c_char;
        (*pool).end = (chunk as *mut u8).add(size) as *mut c_char;

        /* the next chunk is twice the size of the current one */
        size = yyjson_min((*pool).chunk_size.wrapping_mul(2),
                          (*pool).chunk_size_max);
        if size < (*pool).chunk_size {
            size = (*pool).chunk_size_max; /* overflow */
        }
        (*pool).chunk_size = size;
        true
    }
}

/// `bool unsafe_yyjson_val_pool_grow(yyjson_val_pool *pool,
///                                   const yyjson_alc *alc, size_t count)`
/// yyjson.c:2616
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_val_pool_grow(
    pool: *mut yyjson_val_pool,
    alc: *const yyjson_alc,
    count: usize,
) -> bool {
    unsafe {
        let chunk: *mut yyjson_val_chunk;
        let mut size: usize;
        let max_count: usize;

        /* create a new chunk */
        max_count = USIZE_MAX / size_of::<yyjson_mut_val>() - 1;
        if count > max_count {
            return false;
        }
        size = count.wrapping_add(1).wrapping_mul(size_of::<yyjson_mut_val>());
        size = yyjson_max((*pool).chunk_size, size);
        chunk = ((*alc).malloc.unwrap_unchecked())((*alc).ctx, size)
            as *mut yyjson_val_chunk;
        if chunk.is_null() {
            return false;
        }

        /* insert the new chunk as the head of the linked list */
        (*chunk).next = (*pool).chunks;
        (*chunk).chunk_size = size;
        (*pool).chunks = chunk;
        /* C: `(yyjson_mut_val *)(void *)((u8 *)chunk) + 1` - the cast binds
           tighter than `+`, so this advances by one `yyjson_mut_val`. */
        (*pool).cur = (chunk as *mut yyjson_mut_val).add(1);
        (*pool).end = (chunk as *mut u8).add(size) as *mut yyjson_mut_val;

        /* the next chunk is twice the size of the current one */
        size = yyjson_min((*pool).chunk_size.wrapping_mul(2),
                          (*pool).chunk_size_max);
        if size < (*pool).chunk_size {
            size = (*pool).chunk_size_max; /* overflow */
        }
        (*pool).chunk_size = size;
        true
    }
}

/*==============================================================================
 * MARK: - Pool size setters (yyjson.c:2643-2655)
 *============================================================================*/

/// `bool yyjson_mut_doc_set_str_pool_size(yyjson_mut_doc *doc, size_t len)`
/// yyjson.c:2643
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_doc_set_str_pool_size(
    doc: *mut yyjson_mut_doc,
    len: usize,
) -> bool {
    unsafe {
        let max_size: usize = USIZE_MAX - size_of::<yyjson_str_chunk>();
        if doc.is_null() || len == 0 || len > max_size {
            return false;
        }
        (*doc).str_pool.chunk_size =
            len.wrapping_add(size_of::<yyjson_str_chunk>());
        true
    }
}

/// `bool yyjson_mut_doc_set_val_pool_size(yyjson_mut_doc *doc, size_t count)`
/// yyjson.c:2650
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_doc_set_val_pool_size(
    doc: *mut yyjson_mut_doc,
    count: usize,
) -> bool {
    unsafe {
        let max_count: usize = USIZE_MAX / size_of::<yyjson_mut_val>() - 1;
        if doc.is_null() || count == 0 || count > max_count {
            return false;
        }
        (*doc).val_pool.chunk_size =
            count.wrapping_add(1).wrapping_mul(size_of::<yyjson_mut_val>());
        true
    }
}

/*==============================================================================
 * MARK: - Mutable document (yyjson.c:2657-2679)
 *============================================================================*/

/// `void yyjson_mut_doc_free(yyjson_mut_doc *doc)`
/// yyjson.c:2657
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_doc_free(doc: *mut yyjson_mut_doc) {
    unsafe {
        if !doc.is_null() {
            let mut alc: yyjson_alc = (*doc).alc;
            ffi::memset(&raw mut (*doc).alc as *mut c_void, 0,
                        size_of::<yyjson_alc>());
            checked_yyjson_str_pool_release(&raw mut (*doc).str_pool,
                                           &raw mut alc);
            checked_yyjson_val_pool_release(&raw mut (*doc).val_pool,
                                           &raw mut alc);
            (alc.free.unwrap_unchecked())(alc.ctx, doc as *mut c_void);
        }
    }
}

/// `yyjson_mut_doc *yyjson_mut_doc_new(const yyjson_alc *alc)`
/// yyjson.c:2667
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_doc_new(alc: *const yyjson_alc)
    -> *mut yyjson_mut_doc {
    unsafe {
        let doc: *mut yyjson_mut_doc;
        /* C takes the address of the file-static `YYJSON_DEFAULT_ALC`; the
           Rust const is materialised into a local with the same contents. */
        let def: yyjson_alc = YYJSON_DEFAULT_ALC;
        let mut alc = alc;
        if alc.is_null() {
            alc = &raw const def;
        }
        doc = ((*alc).malloc.unwrap_unchecked())(
            (*alc).ctx, size_of::<yyjson_mut_doc>()) as *mut yyjson_mut_doc;
        if doc.is_null() {
            return null_mut();
        }
        ffi::memset(doc as *mut c_void, 0, size_of::<yyjson_mut_doc>());

        (*doc).alc = *alc;
        (*doc).str_pool.chunk_size = YYJSON_MUT_DOC_STR_POOL_INIT_SIZE;
        (*doc).str_pool.chunk_size_max = YYJSON_MUT_DOC_STR_POOL_MAX_SIZE;
        (*doc).val_pool.chunk_size = YYJSON_MUT_DOC_VAL_POOL_INIT_SIZE;
        (*doc).val_pool.chunk_size_max = YYJSON_MUT_DOC_VAL_POOL_MAX_SIZE;
        doc
    }
}

/*==============================================================================
 * MARK: - Document copies (yyjson.c:2682-2715)
 *============================================================================*/

/// `yyjson_mut_doc *yyjson_doc_mut_copy(const yyjson_doc *doc,
///                                      const yyjson_alc *alc)`
/// yyjson.c:2682
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_doc_mut_copy(doc: *const yyjson_doc,
                                             alc: *const yyjson_alc)
    -> *mut yyjson_mut_doc {
    unsafe {
        let m_doc: *mut yyjson_mut_doc;
        let m_val: *mut yyjson_mut_val;

        if doc.is_null() || (*doc).root.is_null() {
            return null_mut();
        }
        m_doc = yyjson_mut_doc_new(alc);
        if m_doc.is_null() {
            return null_mut();
        }
        m_val = yyjson_val_mut_copy(m_doc, (*doc).root as *const yyjson_val);
        if m_val.is_null() {
            yyjson_mut_doc_free(m_doc);
            return null_mut();
        }
        yyjson_mut_doc_set_root(m_doc, m_val);
        m_doc
    }
}

/// `yyjson_mut_doc *yyjson_mut_doc_mut_copy(const yyjson_mut_doc *doc,
///                                          const yyjson_alc *alc)`
/// yyjson.c:2699
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_doc_mut_copy(doc: *const yyjson_mut_doc,
                                                 alc: *const yyjson_alc)
    -> *mut yyjson_mut_doc {
    unsafe {
        let m_doc: *mut yyjson_mut_doc;
        let m_val: *mut yyjson_mut_val;

        if doc.is_null() {
            return null_mut();
        }
        if (*doc).root.is_null() {
            return yyjson_mut_doc_new(alc);
        }

        m_doc = yyjson_mut_doc_new(alc);
        if m_doc.is_null() {
            return null_mut();
        }
        m_val = yyjson_mut_val_mut_copy(m_doc,
                                        (*doc).root as *const yyjson_mut_val);
        if m_val.is_null() {
            yyjson_mut_doc_free(m_doc);
            return null_mut();
        }
        yyjson_mut_doc_set_root(m_doc, m_val);
        m_doc
    }
}

/*==============================================================================
 * MARK: - Immutable -> mutable value copy (yyjson.c:2718-2827)
 *============================================================================*/

/// `yyjson_mut_val *yyjson_val_mut_copy(yyjson_mut_doc *doc,
///                                      const yyjson_val *val)`
/// yyjson.c:2718
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_val_mut_copy(doc: *mut yyjson_mut_doc,
                                             val: *const yyjson_val)
    -> *mut yyjson_mut_val {
    unsafe {
        /*
         The immutable object or array stores all sub-values in a contiguous
         memory, We copy them to another contiguous memory as mutable values,
         then reconnect the mutable values with the original relationship.
         */
        let m_doc = doc;
        let i_vals = val as *mut yyjson_val;
        let i_vals_len: usize;
        let m_vals: *mut yyjson_mut_val;
        let mut m_val: *mut yyjson_mut_val;
        let mut i_val: *mut yyjson_val;
        let i_end: *mut yyjson_val;

        if m_doc.is_null() || i_vals.is_null() {
            return null_mut();
        }
        i_end = checked_yyjson_get_next(i_vals);
        i_vals_len = checked_yyjson_get_next(i_vals).offset_from(i_vals) as usize;
        m_vals = checked_yyjson_mut_val(m_doc, i_vals_len);
        if m_vals.is_null() {
            return null_mut();
        }
        i_val = i_vals;
        m_val = m_vals;

        while i_val < i_end {
            let type_ = checked_yyjson_get_type(i_val as *mut c_void);
            (*m_val).tag = (*i_val).tag;
            (*m_val).uni.u64_ = (*i_val).uni.u64_;
            if type_ == YYJSON_TYPE_STR || type_ == YYJSON_TYPE_RAW {
                let str_ = (*i_val).uni.str_;
                let str_len = checked_yyjson_get_len(i_val as *mut c_void);
                (*m_val).uni.str_ =
                    checked_yyjson_mut_strncpy(m_doc, str_ as *mut u8, str_len);
                if (*m_val).uni.str_.is_null() {
                    return null_mut();
                }
            } else if type_ == YYJSON_TYPE_ARR {
                let mut len = checked_yyjson_get_len(i_val as *mut c_void);
                if len > 0 {
                    let mut ii_val: *mut yyjson_val = i_val.add(1);
                    let mut ii_next: *mut yyjson_val;
                    let mut mm_val: *mut yyjson_mut_val = m_val.add(1);
                    let mm_ctn: *mut yyjson_mut_val = m_val;
                    let mut mm_next: *mut yyjson_mut_val;
                    /* C: `while (len-- > 1)` - post-decrement. */
                    while len > 1 {
                        len = len.wrapping_sub(1);
                        ii_next = checked_yyjson_get_next(ii_val);
                        mm_next = mm_val.offset(ii_next.offset_from(ii_val));
                        (*mm_val).next = mm_next;
                        ii_val = ii_next;
                        mm_val = mm_next;
                    }
                    (*mm_val).next = mm_ctn.add(1);
                    (*mm_ctn).uni.ptr = mm_val as *mut c_void;
                }
            } else if type_ == YYJSON_TYPE_OBJ {
                let mut len = checked_yyjson_get_len(i_val as *mut c_void);
                if len > 0 {
                    let mut ii_key: *mut yyjson_val = i_val.add(1);
                    let mut ii_nextkey: *mut yyjson_val;
                    let mut mm_key: *mut yyjson_mut_val = m_val.add(1);
                    let mm_ctn: *mut yyjson_mut_val = m_val;
                    let mut mm_nextkey: *mut yyjson_mut_val;
                    while len > 1 {
                        len = len.wrapping_sub(1);
                        ii_nextkey = checked_yyjson_get_next(ii_key.add(1));
                        mm_nextkey =
                            mm_key.offset(ii_nextkey.offset_from(ii_key));
                        (*mm_key).next = mm_key.add(1);
                        (*(*mm_key).next).next = mm_nextkey;
                        ii_key = ii_nextkey;
                        mm_key = mm_nextkey;
                    }
                    (*mm_key).next = mm_key.add(1);
                    (*(*mm_key).next).next = mm_ctn.add(1);
                    (*mm_ctn).uni.ptr = mm_key as *mut c_void;
                }
            }
            i_val = i_val.add(1);
            m_val = m_val.add(1);
        }
        m_vals
    }
}

/*==============================================================================
 * MARK: - Mutable -> mutable value copy (yyjson.c:2795-2834)
 *============================================================================*/

/** yyjson.c:2795 */
#[inline]
pub(crate) unsafe fn checked_yyjson_mut_val_mut_copy(
    m_doc: *mut yyjson_mut_doc,
    m_vals: *mut yyjson_mut_val,
) -> *mut yyjson_mut_val {
    unsafe {
        /*
         The mutable object or array stores all sub-values in a circular linked
         list, so we can traverse them in the same loop. The traversal starts
         from the last item, continues with the first item in a list, and ends
         with the second to last item, which needs to be linked to the last
         item to close the circle.
         */
        let m_val = checked_yyjson_mut_val(m_doc, 1);
        if m_val.is_null() {
            return null_mut();
        }
        (*m_val).tag = (*m_vals).tag;

        match checked_yyjson_get_type(m_vals as *mut c_void) {
            YYJSON_TYPE_OBJ | YYJSON_TYPE_ARR => {
                if checked_yyjson_get_len(m_vals as *mut c_void) > 0 {
                    let last = (*m_vals).uni.ptr as *mut yyjson_mut_val;
                    let mut next = (*last).next;
                    let mut prev: *mut yyjson_mut_val;
                    prev = checked_yyjson_mut_val_mut_copy(m_doc, last);
                    if prev.is_null() {
                        return null_mut();
                    }
                    (*m_val).uni.ptr = prev as *mut c_void;
                    while next != last {
                        (*prev).next =
                            checked_yyjson_mut_val_mut_copy(m_doc, next);
                        if (*prev).next.is_null() {
                            return null_mut();
                        }
                        prev = (*prev).next;
                        next = (*next).next;
                    }
                    (*prev).next = (*m_val).uni.ptr as *mut yyjson_mut_val;
                }
            }
            YYJSON_TYPE_RAW | YYJSON_TYPE_STR => {
                let str_ = (*m_vals).uni.str_;
                let str_len = checked_yyjson_get_len(m_vals as *mut c_void);
                (*m_val).uni.str_ =
                    checked_yyjson_mut_strncpy(m_doc, str_ as *mut u8, str_len);
                if (*m_val).uni.str_.is_null() {
                    return null_mut();
                }
            }
            _ => {
                (*m_val).uni = (*m_vals).uni;
            }
        }
        m_val
    }
}

/// `yyjson_mut_val *yyjson_mut_val_mut_copy(yyjson_mut_doc *doc,
///                                          const yyjson_mut_val *val)`
/// yyjson.c:2830
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_val_mut_copy(doc: *mut yyjson_mut_doc,
                                                 val: *const yyjson_mut_val)
    -> *mut yyjson_mut_val {
    unsafe {
        if !doc.is_null() && !val.is_null() {
            return checked_yyjson_mut_val_mut_copy(doc,
                                                  val as *mut yyjson_mut_val);
        }
        null_mut()
    }
}

/*==============================================================================
 * MARK: - Mutable -> immutable copy (yyjson.c:2836-2946)
 *============================================================================*/

/** Count the number of values and the total length of the strings.
    yyjson.c:2837 */
#[inline]
pub(crate) unsafe fn yyjson_mut_stat(val: *mut yyjson_mut_val,
                                     val_sum: *mut usize,
                                     str_sum: *mut usize) {
    unsafe {
        let type_ = checked_yyjson_get_type(val as *mut c_void);
        *val_sum = (*val_sum).wrapping_add(1);
        if type_ == YYJSON_TYPE_ARR || type_ == YYJSON_TYPE_OBJ {
            let mut child = (*val).uni.ptr as *mut yyjson_mut_val;
            let mut len = checked_yyjson_get_len(val as *mut c_void);
            let mut i: usize;
            len = len.wrapping_shl((type_ == YYJSON_TYPE_OBJ) as u32);
            *val_sum = (*val_sum).wrapping_add(len);
            i = 0;
            while i < len {
                let stype = checked_yyjson_get_type(child as *mut c_void);
                if stype == YYJSON_TYPE_STR || stype == YYJSON_TYPE_RAW {
                    *str_sum = (*str_sum).wrapping_add(
                        checked_yyjson_get_len(child as *mut c_void)
                            .wrapping_add(1));
                } else if stype == YYJSON_TYPE_ARR || stype == YYJSON_TYPE_OBJ {
                    yyjson_mut_stat(child, val_sum, str_sum);
                    *val_sum = (*val_sum).wrapping_sub(1);
                }
                child = (*child).next;
                i = i.wrapping_add(1);
            }
        } else if type_ == YYJSON_TYPE_STR || type_ == YYJSON_TYPE_RAW {
            *str_sum = (*str_sum).wrapping_add(
                checked_yyjson_get_len(val as *mut c_void).wrapping_add(1));
        }
    }
}

/** Copy mutable values to immutable value pool.
    yyjson.c:2860 */
#[inline]
pub(crate) unsafe fn yyjson_imut_copy(val_ptr: *mut *mut yyjson_val,
                                      buf_ptr: *mut *mut c_char,
                                      mval: *mut yyjson_mut_val) -> usize {
    unsafe {
        let val: *mut yyjson_val = *val_ptr;
        let type_ = checked_yyjson_get_type(mval as *mut c_void);
        if type_ == YYJSON_TYPE_ARR || type_ == YYJSON_TYPE_OBJ {
            let mut child = (*mval).uni.ptr as *mut yyjson_mut_val;
            let mut len = checked_yyjson_get_len(mval as *mut c_void);
            let mut i: usize;
            let mut val_sum: usize = 1;
            if type_ == YYJSON_TYPE_OBJ {
                if len != 0 {
                    child = (*(*child).next).next;
                }
                len = len.wrapping_shl(1);
            } else {
                if len != 0 {
                    child = (*child).next;
                }
            }
            *val_ptr = val.add(1);
            i = 0;
            while i < len {
                val_sum = val_sum
                    .wrapping_add(yyjson_imut_copy(val_ptr, buf_ptr, child));
                child = (*child).next;
                i = i.wrapping_add(1);
            }
            (*val).tag = (*mval).tag;
            (*val).uni.ofs = val_sum.wrapping_mul(size_of::<yyjson_val>());
            val_sum
        } else if type_ == YYJSON_TYPE_STR || type_ == YYJSON_TYPE_RAW {
            let buf: *mut c_char = *buf_ptr;
            let len = checked_yyjson_get_len(mval as *mut c_void);
            ffi::memcpy(buf as *mut c_void,
                        (*mval).uni.str_ as *const c_void, len);
            *buf.add(len) = 0;
            (*val).tag = (*mval).tag;
            (*val).uni.str_ = buf;
            *val_ptr = val.add(1);
            *buf_ptr = buf.add(len).add(1);
            1
        } else {
            (*val).tag = (*mval).tag;
            (*val).uni = (*mval).uni;
            *val_ptr = val.add(1);
            1
        }
    }
}

/// `yyjson_doc *yyjson_mut_doc_imut_copy(const yyjson_mut_doc *doc,
///                                       const yyjson_alc *alc)`
/// yyjson.c:2902
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_doc_imut_copy(doc: *const yyjson_mut_doc,
                                                  alc: *const yyjson_alc)
    -> *mut yyjson_doc {
    unsafe {
        let mdoc = doc;
        if mdoc.is_null() {
            return null_mut();
        }
        yyjson_mut_val_imut_copy((*mdoc).root as *const yyjson_mut_val, alc)
    }
}

/// `yyjson_doc *yyjson_mut_val_imut_copy(const yyjson_mut_val *val,
///                                       const yyjson_alc *alc)`
/// yyjson.c:2908
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_mut_val_imut_copy(val: *const yyjson_mut_val,
                                                  alc: *const yyjson_alc)
    -> *mut yyjson_doc {
    unsafe {
        let mval = val as *mut yyjson_mut_val;
        let mut val_num: usize = 0;
        let mut str_sum: usize = 0;
        let hdr_size: usize;
        let buf_size: usize;
        let doc: *mut yyjson_doc;
        let mut val_hdr: *mut yyjson_val;

        /* This value should be NULL here. Setting a non-null value suppresses
           warning from the clang analyzer. */
        let mut str_hdr: *mut c_char = &raw mut str_sum as *mut c_char;
        if mval.is_null() {
            return null_mut();
        }
        let def: yyjson_alc = YYJSON_DEFAULT_ALC;
        let mut alc = alc;
        if alc.is_null() {
            alc = &raw const def;
        }

        /* traverse the input value to get pool size */
        yyjson_mut_stat(mval, &raw mut val_num, &raw mut str_sum);

        /* create doc and val pool */
        hdr_size = size_align_up(size_of::<yyjson_doc>(),
                                 size_of::<yyjson_val>());
        buf_size = hdr_size
            .wrapping_add(val_num.wrapping_mul(size_of::<yyjson_val>()));
        doc = ((*alc).malloc.unwrap_unchecked())((*alc).ctx, buf_size)
            as *mut yyjson_doc;
        if doc.is_null() {
            return null_mut();
        }
        ffi::memset(doc as *mut c_void, 0, size_of::<yyjson_doc>());
        val_hdr = (doc as *mut c_char).add(hdr_size) as *mut yyjson_val;
        (*doc).root = val_hdr;
        (*doc).alc = *alc;

        /* create str pool */
        if str_sum > 0 {
            str_hdr = ((*alc).malloc.unwrap_unchecked())((*alc).ctx, str_sum)
                as *mut c_char;
            (*doc).str_pool = str_hdr;
            if str_hdr.is_null() {
                ((*alc).free.unwrap_unchecked())((*alc).ctx,
                                                 doc as *mut c_void);
                return null_mut();
            }
        }

        /* copy vals and strs */
        (*doc).val_read = yyjson_imut_copy(&raw mut val_hdr,
                                           &raw mut str_hdr, mval);
        (*doc).dat_read = str_sum.wrapping_add(1);
        doc
    }
}

/*==============================================================================
 * MARK: - Equality (yyjson.c:2948-3080)
 *============================================================================*/

/** yyjson.c:2948 */
#[inline(always)]
pub(crate) unsafe fn checked_yyjson_num_equals(lhs: *mut c_void,
                                              rhs: *mut c_void) -> bool {
    unsafe {
        let luni: *mut yyjson_val_uni = &raw mut (*(lhs as *mut yyjson_val)).uni;
        let runi: *mut yyjson_val_uni = &raw mut (*(rhs as *mut yyjson_val)).uni;
        let lt = checked_yyjson_get_subtype(lhs);
        let rt = checked_yyjson_get_subtype(rhs);
        if lt == rt {
            return (*luni).u64_ == (*runi).u64_;
        }
        if lt == YYJSON_SUBTYPE_SINT && rt == YYJSON_SUBTYPE_UINT {
            return (*luni).i64_ >= 0 && (*luni).u64_ == (*runi).u64_;
        }
        if lt == YYJSON_SUBTYPE_UINT && rt == YYJSON_SUBTYPE_SINT {
            return (*runi).i64_ >= 0 && (*luni).u64_ == (*runi).u64_;
        }
        false
    }
}

/** yyjson.c:2962 */
#[inline(always)]
pub(crate) unsafe fn checked_yyjson_str_equals(lhs: *mut c_void,
                                              rhs: *mut c_void) -> bool {
    unsafe {
        let len = checked_yyjson_get_len(lhs);
        if len != checked_yyjson_get_len(rhs) {
            return false;
        }
        ffi::memcmp(checked_yyjson_get_str(lhs) as *const c_void,
                    checked_yyjson_get_str(rhs) as *const c_void, len) == 0
    }
}

/// `bool unsafe_yyjson_equals(const yyjson_val *lhs, const yyjson_val *rhs)`
/// yyjson.c:2971
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_equals(lhs: *const yyjson_val,
                                              rhs: *const yyjson_val) -> bool {
    unsafe {
        let mut lhs = lhs as *mut yyjson_val;
        let mut rhs = rhs as *mut yyjson_val;
        let type_ = checked_yyjson_get_type(lhs as *mut c_void);
        if type_ != checked_yyjson_get_type(rhs as *mut c_void) {
            return false;
        }

        match type_ {
            YYJSON_TYPE_OBJ => {
                let mut len = checked_yyjson_get_len(lhs as *mut c_void);
                if len != checked_yyjson_get_len(rhs as *mut c_void) {
                    return false;
                }
                if len > 0 {
                    let mut iter: yyjson_obj_iter =
                        core::mem::MaybeUninit::zeroed().assume_init();
                    yyjson_obj_iter_init(rhs, &raw mut iter);
                    lhs = checked_yyjson_get_first(lhs);
                    /* C: `while (len-- > 0)` - post-decrement. */
                    while len > 0 {
                        len = len.wrapping_sub(1);
                        rhs = yyjson_obj_iter_getn(
                            &raw mut iter,
                            (*lhs).uni.str_ as *mut u8,
                            checked_yyjson_get_len(lhs as *mut c_void));
                        if rhs.is_null() {
                            return false;
                        }
                        if !unsafe_yyjson_equals(lhs.add(1) as *const yyjson_val,
                                                 rhs as *const yyjson_val) {
                            return false;
                        }
                        lhs = checked_yyjson_get_next(lhs.add(1));
                    }
                }
                /* yyjson allows duplicate keys, so the check may be
                   inaccurate */
                true
            }

            YYJSON_TYPE_ARR => {
                let mut len = checked_yyjson_get_len(lhs as *mut c_void);
                if len != checked_yyjson_get_len(rhs as *mut c_void) {
                    return false;
                }
                if len > 0 {
                    lhs = checked_yyjson_get_first(lhs);
                    rhs = checked_yyjson_get_first(rhs);
                    while len > 0 {
                        len = len.wrapping_sub(1);
                        if !unsafe_yyjson_equals(lhs as *const yyjson_val,
                                                 rhs as *const yyjson_val) {
                            return false;
                        }
                        lhs = checked_yyjson_get_next(lhs);
                        rhs = checked_yyjson_get_next(rhs);
                    }
                }
                true
            }

            YYJSON_TYPE_NUM => checked_yyjson_num_equals(lhs as *mut c_void,
                                                        rhs as *mut c_void),

            YYJSON_TYPE_RAW | YYJSON_TYPE_STR =>
                checked_yyjson_str_equals(lhs as *mut c_void,
                                         rhs as *mut c_void),

            YYJSON_TYPE_NULL | YYJSON_TYPE_BOOL => (*lhs).tag == (*rhs).tag,

            _ => false,
        }
    }
}

/// `bool unsafe_yyjson_mut_equals(const yyjson_mut_val *lhs,
///                                const yyjson_mut_val *rhs)`
/// yyjson.c:3026
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unsafe_yyjson_mut_equals(lhs: *const yyjson_mut_val,
                                                  rhs: *const yyjson_mut_val)
    -> bool {
    unsafe {
        let mut lhs = lhs as *mut yyjson_mut_val;
        let mut rhs = rhs as *mut yyjson_mut_val;
        let type_ = checked_yyjson_get_type(lhs as *mut c_void);
        if type_ != checked_yyjson_get_type(rhs as *mut c_void) {
            return false;
        }

        match type_ {
            YYJSON_TYPE_OBJ => {
                let mut len = checked_yyjson_get_len(lhs as *mut c_void);
                if len != checked_yyjson_get_len(rhs as *mut c_void) {
                    return false;
                }
                if len > 0 {
                    let mut iter: yyjson_mut_obj_iter =
                        core::mem::MaybeUninit::zeroed().assume_init();
                    yyjson_mut_obj_iter_init(rhs, &raw mut iter);
                    lhs = (*lhs).uni.ptr as *mut yyjson_mut_val;
                    while len > 0 {
                        len = len.wrapping_sub(1);
                        rhs = yyjson_mut_obj_iter_getn(
                            &raw mut iter,
                            (*lhs).uni.str_ as *mut u8,
                            checked_yyjson_get_len(lhs as *mut c_void));
                        if rhs.is_null() {
                            return false;
                        }
                        if !unsafe_yyjson_mut_equals(
                            (*lhs).next as *const yyjson_mut_val,
                            rhs as *const yyjson_mut_val)
                        {
                            return false;
                        }
                        lhs = (*(*lhs).next).next;
                    }
                }
                /* yyjson allows duplicate keys, so the check may be
                   inaccurate */
                true
            }

            YYJSON_TYPE_ARR => {
                let mut len = checked_yyjson_get_len(lhs as *mut c_void);
                if len != checked_yyjson_get_len(rhs as *mut c_void) {
                    return false;
                }
                if len > 0 {
                    lhs = (*lhs).uni.ptr as *mut yyjson_mut_val;
                    rhs = (*rhs).uni.ptr as *mut yyjson_mut_val;
                    while len > 0 {
                        len = len.wrapping_sub(1);
                        if !unsafe_yyjson_mut_equals(
                            lhs as *const yyjson_mut_val,
                            rhs as *const yyjson_mut_val)
                        {
                            return false;
                        }
                        lhs = (*lhs).next;
                        rhs = (*rhs).next;
                    }
                }
                true
            }

            YYJSON_TYPE_NUM => checked_yyjson_num_equals(lhs as *mut c_void,
                                                        rhs as *mut c_void),

            YYJSON_TYPE_RAW | YYJSON_TYPE_STR =>
                checked_yyjson_str_equals(lhs as *mut c_void,
                                         rhs as *mut c_void),

            YYJSON_TYPE_NULL | YYJSON_TYPE_BOOL => (*lhs).tag == (*rhs).tag,

            _ => false,
        }
    }
}

/*==============================================================================
 * MARK: - Position location (yyjson.c:3082-3119)
 *============================================================================*/

/// `bool yyjson_locate_pos(const char *str, size_t len, size_t pos,
///                         size_t *line, size_t *col, size_t *chr)`
/// yyjson.c:3082
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_locate_pos(str: *const c_char, len: usize,
                                           pos: usize, line: *mut usize,
                                           col: *mut usize,
                                           chr: *mut usize) -> bool {
    unsafe {
        let mut line_sum: usize = 0;
        let mut line_pos: usize = 0;
        let mut chr_sum: usize = 0;
        let mut cur: *mut u8 = str as *mut u8;
        let end: *mut u8 = cur.wrapping_add(pos);

        if str.is_null() || pos > len {
            if !line.is_null() {
                *line = 0;
            }
            if !col.is_null() {
                *col = 0;
            }
            if !chr.is_null() {
                *chr = 0;
            }
            return false;
        }

        if pos >= 3 && test_utf8_bom(cur) {
            cur = cur.add(3); /* don't count BOM */
        }
        while cur < end {
            let c = *cur;
            chr_sum = chr_sum.wrapping_add(1);
            if c < 0x80 {
                /* 0xxxxxxx (0x00-0x7F) ASCII */
                if c == b'\n' {
                    line_sum = line_sum.wrapping_add(1);
                    line_pos = chr_sum;
                }
                cur = cur.add(1);
            } else if c < 0xC0 {
                cur = cur.add(1); /* 10xxxxxx (0x80-0xBF) Invalid */
            } else if c < 0xE0 {
                cur = cur.add(2); /* 110xxxxx (0xC0-0xDF) 2-byte UTF-8 */
            } else if c < 0xF0 {
                cur = cur.add(3); /* 1110xxxx (0xE0-0xEF) 3-byte UTF-8 */
            } else if c < 0xF8 {
                cur = cur.add(4); /* 11110xxx (0xF0-0xF7) 4-byte UTF-8 */
            } else {
                cur = cur.add(1); /* 11111xxx (0xF8-0xFF) Invalid */
            }
        }
        if !line.is_null() {
            *line = line_sum.wrapping_add(1);
        }
        if !col.is_null() {
            *col = chr_sum.wrapping_sub(line_pos).wrapping_add(1);
        }
        if !chr.is_null() {
            *chr = chr_sum;
        }
        true
    }
}
