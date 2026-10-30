//! "JSON Struct Utils": pool growth, mutable document creation/destruction,
//! the four copy functions, deep equality and `yyjson_locate_pos`.
//!
//! Also holds Rust twins of the handful of header `static inline` helpers the
//! implementation itself needs (`unsafe_yyjson_mut_val`,
//! `unsafe_yyjson_mut_strncpy`, the object iterators, ...). They are *copies*
//! of the header logic, not re-exports, because the header versions are
//! compiled into C callers only.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alc::alc_or_default;
use crate::mem;
use crate::types::*;
use crate::util::*;

/* -------------------------------------------- header inline reimplementation */

/// `unsafe_yyjson_mut_val`
#[inline(always)]
pub unsafe fn mut_val(doc: *mut yyjson_mut_doc, count: usize) -> *mut yyjson_mut_val {
    let alc = &(*doc).alc as *const yyjson_alc;
    let pool = &mut (*doc).val_pool as *mut yyjson_val_pool;
    let avail = ((*pool).end as usize).wrapping_sub((*pool).cur as usize)
        / size_of::<yyjson_mut_val>();
    if avail < count {
        if !unsafe_yyjson_val_pool_grow(pool, alc, count) {
            return ptr::null_mut();
        }
    }
    let val = (*pool).cur;
    (*pool).cur = (*pool).cur.add(count);
    val
}

/// `unsafe_yyjson_mut_str_alc`
#[inline(always)]
pub unsafe fn mut_str_alc(doc: *mut yyjson_mut_doc, len: usize) -> *mut c_char {
    let alc = &(*doc).alc as *const yyjson_alc;
    let pool = &mut (*doc).str_pool as *mut yyjson_str_pool;
    if len == usize::MAX {
        return ptr::null_mut();
    }
    let avail = ((*pool).end as usize).wrapping_sub((*pool).cur as usize);
    if avail <= len {
        if !unsafe_yyjson_str_pool_grow(pool, alc, len + 1) {
            return ptr::null_mut();
        }
    }
    let m = (*pool).cur;
    (*pool).cur = m.add(len + 1);
    m
}

/// `unsafe_yyjson_mut_strncpy`
#[inline(always)]
pub unsafe fn mut_strncpy(
    doc: *mut yyjson_mut_doc,
    str_: *const c_char,
    len: usize,
) -> *mut c_char {
    let m = mut_str_alc(doc, len);
    if m.is_null() {
        return ptr::null_mut();
    }
    mem::copy(m as *mut u8, str_ as *const u8, len);
    *m.add(len) = 0;
    m
}

/// `yyjson_doc_free` (inline in the header; needed internally too).
pub unsafe fn doc_free(doc: *mut yyjson_doc) {
    if !doc.is_null() {
        let alc = (*doc).alc;
        mem::set(&mut (*doc).alc as *mut yyjson_alc as *mut u8, 0, size_of::<yyjson_alc>());
        if !(*doc).str_pool.is_null() {
            alc.free((*doc).str_pool as *mut c_void);
        }
        alc.free(doc as *mut c_void);
    }
}

/* ------------------------------------------------------------ pool release */

unsafe fn str_pool_release(pool: *mut yyjson_str_pool, alc: *const yyjson_alc) {
    let mut chunk = (*pool).chunks;
    while !chunk.is_null() {
        let next = (*chunk).next;
        (*alc).free(chunk as *mut c_void);
        chunk = next;
    }
}

unsafe fn val_pool_release(pool: *mut yyjson_val_pool, alc: *const yyjson_alc) {
    let mut chunk = (*pool).chunks;
    while !chunk.is_null() {
        let next = (*chunk).next;
        (*alc).free(chunk as *mut c_void);
        chunk = next;
    }
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_str_pool_grow(
    pool: *mut yyjson_str_pool,
    alc: *const yyjson_alc,
    len: usize,
) -> bool {
    let max_len = USIZE_MAX - size_of::<yyjson_str_chunk>();
    if len > max_len {
        return false;
    }
    let mut size = len + size_of::<yyjson_str_chunk>();
    if (*pool).chunk_size > size {
        size = (*pool).chunk_size;
    }
    let chunk = (*alc).malloc(size) as *mut yyjson_str_chunk;
    if chunk.is_null() {
        return false;
    }

    (*chunk).next = (*pool).chunks;
    (*chunk).chunk_size = size;
    (*pool).chunks = chunk;
    (*pool).cur = (chunk as *mut u8).add(size_of::<yyjson_str_chunk>()) as *mut c_char;
    (*pool).end = (chunk as *mut u8).add(size) as *mut c_char;

    let doubled = (*pool).chunk_size.wrapping_mul(2);
    let mut size = if doubled < (*pool).chunk_size_max {
        doubled
    } else {
        (*pool).chunk_size_max
    };
    if size < (*pool).chunk_size {
        size = (*pool).chunk_size_max;
    }
    (*pool).chunk_size = size;
    true
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_val_pool_grow(
    pool: *mut yyjson_val_pool,
    alc: *const yyjson_alc,
    count: usize,
) -> bool {
    let max_count = USIZE_MAX / size_of::<yyjson_mut_val>() - 1;
    if count > max_count {
        return false;
    }
    let mut size = (count + 1) * size_of::<yyjson_mut_val>();
    if (*pool).chunk_size > size {
        size = (*pool).chunk_size;
    }
    let chunk = (*alc).malloc(size) as *mut yyjson_val_chunk;
    if chunk.is_null() {
        return false;
    }

    (*chunk).next = (*pool).chunks;
    (*chunk).chunk_size = size;
    (*pool).chunks = chunk;
    (*pool).cur = (chunk as *mut yyjson_mut_val).add(1);
    (*pool).end = (chunk as *mut u8).add(size) as *mut yyjson_mut_val;

    let doubled = (*pool).chunk_size.wrapping_mul(2);
    let mut size = if doubled < (*pool).chunk_size_max {
        doubled
    } else {
        (*pool).chunk_size_max
    };
    if size < (*pool).chunk_size {
        size = (*pool).chunk_size_max;
    }
    (*pool).chunk_size = size;
    true
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_set_str_pool_size(
    doc: *mut yyjson_mut_doc,
    len: usize,
) -> bool {
    let max_size = USIZE_MAX - size_of::<yyjson_str_chunk>();
    if doc.is_null() || len == 0 || len > max_size {
        return false;
    }
    (*doc).str_pool.chunk_size = len + size_of::<yyjson_str_chunk>();
    true
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_set_val_pool_size(
    doc: *mut yyjson_mut_doc,
    count: usize,
) -> bool {
    let max_count = USIZE_MAX / size_of::<yyjson_mut_val>() - 1;
    if doc.is_null() || count == 0 || count > max_count {
        return false;
    }
    (*doc).val_pool.chunk_size = (count + 1) * size_of::<yyjson_mut_val>();
    true
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_free(doc: *mut yyjson_mut_doc) {
    if !doc.is_null() {
        let alc = (*doc).alc;
        mem::set(
            &mut (*doc).alc as *mut yyjson_alc as *mut u8,
            0,
            size_of::<yyjson_alc>(),
        );
        str_pool_release(&mut (*doc).str_pool, &alc);
        val_pool_release(&mut (*doc).val_pool, &alc);
        alc.free(doc as *mut c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_new(alc: *const yyjson_alc) -> *mut yyjson_mut_doc {
    let alc = alc_or_default(alc);
    let doc = (*alc).malloc(size_of::<yyjson_mut_doc>()) as *mut yyjson_mut_doc;
    if doc.is_null() {
        return ptr::null_mut();
    }
    mem::set(doc as *mut u8, 0, size_of::<yyjson_mut_doc>());

    (*doc).alc = *alc;
    (*doc).str_pool.chunk_size = YYJSON_MUT_DOC_STR_POOL_INIT_SIZE;
    (*doc).str_pool.chunk_size_max = YYJSON_MUT_DOC_STR_POOL_MAX_SIZE;
    (*doc).val_pool.chunk_size = YYJSON_MUT_DOC_VAL_POOL_INIT_SIZE;
    (*doc).val_pool.chunk_size_max = YYJSON_MUT_DOC_VAL_POOL_MAX_SIZE;
    doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_doc_mut_copy(
    doc: *const yyjson_doc,
    alc: *const yyjson_alc,
) -> *mut yyjson_mut_doc {
    if doc.is_null() || (*doc).root.is_null() {
        return ptr::null_mut();
    }
    let m_doc = yyjson_mut_doc_new(alc);
    if m_doc.is_null() {
        return ptr::null_mut();
    }
    let m_val = yyjson_val_mut_copy(m_doc, (*doc).root);
    if m_val.is_null() {
        yyjson_mut_doc_free(m_doc);
        return ptr::null_mut();
    }
    (*m_doc).root = m_val;
    m_doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_mut_copy(
    doc: *const yyjson_mut_doc,
    alc: *const yyjson_alc,
) -> *mut yyjson_mut_doc {
    if doc.is_null() {
        return ptr::null_mut();
    }
    if (*doc).root.is_null() {
        return yyjson_mut_doc_new(alc);
    }
    let m_doc = yyjson_mut_doc_new(alc);
    if m_doc.is_null() {
        return ptr::null_mut();
    }
    let m_val = yyjson_mut_val_mut_copy(m_doc, (*doc).root);
    if m_val.is_null() {
        yyjson_mut_doc_free(m_doc);
        return ptr::null_mut();
    }
    (*m_doc).root = m_val;
    m_doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_mut_copy(
    m_doc: *mut yyjson_mut_doc,
    i_vals: *const yyjson_val,
) -> *mut yyjson_mut_val {
    if m_doc.is_null() || i_vals.is_null() {
        return ptr::null_mut();
    }
    let i_end = unsafe_get_next(i_vals);
    let i_vals_len = i_end.offset_from(i_vals) as usize;
    let m_vals = mut_val(m_doc, i_vals_len);
    if m_vals.is_null() {
        return ptr::null_mut();
    }
    let mut i_val = i_vals as *mut yyjson_val;
    let mut m_val = m_vals;

    while i_val < i_end {
        let ty = unsafe_get_type(i_val);
        (*m_val).tag = (*i_val).tag;
        (*m_val).uni.u64_ = (*i_val).uni.u64_;
        if ty == YYJSON_TYPE_STR || ty == YYJSON_TYPE_RAW {
            let str_ = (*i_val).uni.str_;
            let str_len = unsafe_get_len(i_val);
            (*m_val).uni.str_ = mut_strncpy(m_doc, str_, str_len);
            if (*m_val).uni.str_.is_null() {
                return ptr::null_mut();
            }
        } else if ty == YYJSON_TYPE_ARR {
            let mut len = unsafe_get_len(i_val);
            if len > 0 {
                let mut ii_val = i_val.add(1);
                let mut mm_val = m_val.add(1);
                let mm_ctn = m_val;
                while len > 1 {
                    len -= 1;
                    let ii_next = unsafe_get_next(ii_val);
                    let mm_next = mm_val.offset(ii_next.offset_from(ii_val));
                    (*mm_val).next = mm_next;
                    ii_val = ii_next;
                    mm_val = mm_next;
                }
                (*mm_val).next = mm_ctn.add(1);
                (*mm_ctn).uni.ptr = mm_val as *mut c_void;
            }
        } else if ty == YYJSON_TYPE_OBJ {
            let mut len = unsafe_get_len(i_val);
            if len > 0 {
                let mut ii_key = i_val.add(1);
                let mut mm_key = m_val.add(1);
                let mm_ctn = m_val;
                while len > 1 {
                    len -= 1;
                    let ii_nextkey = unsafe_get_next(ii_key.add(1));
                    let mm_nextkey = mm_key.offset(ii_nextkey.offset_from(ii_key));
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

unsafe fn mut_val_mut_copy(
    m_doc: *mut yyjson_mut_doc,
    m_vals: *const yyjson_mut_val,
) -> *mut yyjson_mut_val {
    let m_val = mut_val(m_doc, 1);
    if m_val.is_null() {
        return ptr::null_mut();
    }
    (*m_val).tag = (*m_vals).tag;

    match mut_get_type(m_vals) {
        YYJSON_TYPE_OBJ | YYJSON_TYPE_ARR => {
            if mut_get_len(m_vals) > 0 {
                let last = (*m_vals).uni.ptr as *mut yyjson_mut_val;
                let mut next = (*last).next;
                let mut prev = mut_val_mut_copy(m_doc, last);
                if prev.is_null() {
                    return ptr::null_mut();
                }
                (*m_val).uni.ptr = prev as *mut c_void;
                while next != last {
                    (*prev).next = mut_val_mut_copy(m_doc, next);
                    if (*prev).next.is_null() {
                        return ptr::null_mut();
                    }
                    prev = (*prev).next;
                    next = (*next).next;
                }
                (*prev).next = (*m_val).uni.ptr as *mut yyjson_mut_val;
            }
        }
        YYJSON_TYPE_RAW | YYJSON_TYPE_STR => {
            let str_ = (*m_vals).uni.str_;
            let str_len = mut_get_len(m_vals);
            (*m_val).uni.str_ = mut_strncpy(m_doc, str_, str_len);
            if (*m_val).uni.str_.is_null() {
                return ptr::null_mut();
            }
        }
        _ => {
            (*m_val).uni = (*m_vals).uni;
        }
    }
    m_val
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_mut_copy(
    doc: *mut yyjson_mut_doc,
    val: *const yyjson_mut_val,
) -> *mut yyjson_mut_val {
    if !doc.is_null() && !val.is_null() {
        return mut_val_mut_copy(doc, val);
    }
    ptr::null_mut()
}

/// `yyjson_mut_stat`: count values and total string length.
unsafe fn mut_stat(val: *const yyjson_mut_val, val_sum: &mut usize, str_sum: &mut usize) {
    let ty = mut_get_type(val);
    *val_sum += 1;
    if ty == YYJSON_TYPE_ARR || ty == YYJSON_TYPE_OBJ {
        let mut child = (*val).uni.ptr as *mut yyjson_mut_val;
        let mut len = mut_get_len(val);
        len <<= (ty == YYJSON_TYPE_OBJ) as u32;
        *val_sum += len;
        let mut i = 0;
        while i < len {
            let stype = mut_get_type(child);
            if stype == YYJSON_TYPE_STR || stype == YYJSON_TYPE_RAW {
                *str_sum += mut_get_len(child) + 1;
            } else if stype == YYJSON_TYPE_ARR || stype == YYJSON_TYPE_OBJ {
                mut_stat(child, val_sum, str_sum);
                *val_sum -= 1;
            }
            child = (*child).next;
            i += 1;
        }
    } else if ty == YYJSON_TYPE_STR || ty == YYJSON_TYPE_RAW {
        *str_sum += mut_get_len(val) + 1;
    }
}

/// `yyjson_imut_copy`: copy mutable values into the immutable value pool.
unsafe fn imut_copy(
    val_ptr: &mut *mut yyjson_val,
    buf_ptr: &mut *mut c_char,
    mval: *const yyjson_mut_val,
) -> usize {
    let val = *val_ptr;
    let ty = mut_get_type(mval);
    if ty == YYJSON_TYPE_ARR || ty == YYJSON_TYPE_OBJ {
        let mut child = (*mval).uni.ptr as *mut yyjson_mut_val;
        let mut len = mut_get_len(mval);
        let mut val_sum = 1usize;
        if ty == YYJSON_TYPE_OBJ {
            if len != 0 {
                child = (*(*child).next).next;
            }
            len <<= 1;
        } else {
            if len != 0 {
                child = (*child).next;
            }
        }
        *val_ptr = val.add(1);
        let mut i = 0;
        while i < len {
            val_sum += imut_copy(val_ptr, buf_ptr, child);
            child = (*child).next;
            i += 1;
        }
        (*val).tag = (*mval).tag;
        (*val).uni.ofs = val_sum * size_of::<yyjson_val>();
        val_sum
    } else if ty == YYJSON_TYPE_STR || ty == YYJSON_TYPE_RAW {
        let buf = *buf_ptr;
        let len = mut_get_len(mval);
        mem::copy(buf as *mut u8, (*mval).uni.str_ as *const u8, len);
        *buf.add(len) = 0;
        (*val).tag = (*mval).tag;
        (*val).uni.str_ = buf;
        *val_ptr = val.add(1);
        *buf_ptr = buf.add(len + 1);
        1
    } else {
        (*val).tag = (*mval).tag;
        (*val).uni = (*mval).uni;
        *val_ptr = val.add(1);
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_imut_copy(
    mdoc: *const yyjson_mut_doc,
    alc: *const yyjson_alc,
) -> *mut yyjson_doc {
    if mdoc.is_null() {
        return ptr::null_mut();
    }
    yyjson_mut_val_imut_copy((*mdoc).root, alc)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_imut_copy(
    mval: *const yyjson_mut_val,
    alc: *const yyjson_alc,
) -> *mut yyjson_doc {
    let mut val_num: usize = 0;
    let mut str_sum: usize = 0;

    if mval.is_null() {
        return ptr::null_mut();
    }
    let alc = alc_or_default(alc);

    mut_stat(mval, &mut val_num, &mut str_sum);

    let hdr_size = size_align_up(size_of::<yyjson_doc>(), size_of::<yyjson_val>());
    let buf_size = hdr_size + val_num * size_of::<yyjson_val>();
    let doc = (*alc).malloc(buf_size) as *mut yyjson_doc;
    if doc.is_null() {
        return ptr::null_mut();
    }
    mem::set(doc as *mut u8, 0, size_of::<yyjson_doc>());
    let mut val_hdr = (doc as *mut u8).add(hdr_size) as *mut yyjson_val;
    (*doc).root = val_hdr;
    (*doc).alc = *alc;

    /* Matches the C: a non-null placeholder that is never dereferenced when
       `str_sum == 0`. */
    let mut str_hdr = &mut str_sum as *mut usize as *mut c_char;
    if str_sum > 0 {
        str_hdr = (*alc).malloc(str_sum) as *mut c_char;
        (*doc).str_pool = str_hdr;
        if str_hdr.is_null() {
            (*alc).free(doc as *mut c_void);
            return ptr::null_mut();
        }
    }

    (*doc).val_read = imut_copy(&mut val_hdr, &mut str_hdr, mval);
    (*doc).dat_read = str_sum + 1;
    doc
}

/* -------------------------------------------------------------- equality */

#[inline(always)]
unsafe fn num_equals(lhs: *const yyjson_val, rhs: *const yyjson_val) -> bool {
    let luni = &(*lhs).uni;
    let runi = &(*rhs).uni;
    let lt = unsafe_get_subtype(lhs);
    let rt = unsafe_get_subtype(rhs);
    if lt == rt {
        return luni.u64_ == runi.u64_;
    }
    if lt == YYJSON_SUBTYPE_SINT && rt == YYJSON_SUBTYPE_UINT {
        return luni.i64_ >= 0 && luni.u64_ == runi.u64_;
    }
    if lt == YYJSON_SUBTYPE_UINT && rt == YYJSON_SUBTYPE_SINT {
        return runi.i64_ >= 0 && luni.u64_ == runi.u64_;
    }
    false
}

#[inline(always)]
unsafe fn str_equals(lhs: *const yyjson_val, rhs: *const yyjson_val) -> bool {
    let len = unsafe_get_len(lhs);
    if len != unsafe_get_len(rhs) {
        return false;
    }
    mem::memcmp_eq(
        (*lhs).uni.str_ as *const u8,
        (*rhs).uni.str_ as *const u8,
        len,
    )
}

/* `yyjson_obj_iter` logic, copied from the header. */

pub struct ObjIter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut yyjson_val,
    pub obj: *mut yyjson_val,
}

impl ObjIter {
    pub unsafe fn new(obj: *const yyjson_val) -> ObjIter {
        if !obj.is_null() && unsafe_get_type(obj) == YYJSON_TYPE_OBJ {
            ObjIter {
                idx: 0,
                max: unsafe_get_len(obj),
                cur: unsafe_get_first(obj),
                obj: obj as *mut yyjson_val,
            }
        } else {
            ObjIter {
                idx: 0,
                max: 0,
                cur: ptr::null_mut(),
                obj: ptr::null_mut(),
            }
        }
    }

    /// `yyjson_obj_iter_getn`
    pub unsafe fn getn(&mut self, key: *const c_char, key_len: usize) -> *mut yyjson_val {
        if !key.is_null() {
            let mut idx = self.idx;
            let mut max = self.max;
            let mut cur = self.cur;
            if idx == max {
                idx = 0;
                cur = unsafe_get_first(self.obj);
            }
            while idx < max {
                idx += 1;
                let next = unsafe_get_next(cur.add(1));
                if unsafe_equals_strn(cur, key, key_len) {
                    self.idx = idx;
                    self.cur = next;
                    return cur.add(1);
                }
                cur = next;
                if idx == self.max && self.idx < self.max {
                    idx = 0;
                    max = self.idx;
                    cur = unsafe_get_first(self.obj);
                }
            }
        }
        ptr::null_mut()
    }
}

/* `yyjson_mut_obj_iter` logic, copied from the header. */

pub struct MutObjIter {
    pub idx: usize,
    pub max: usize,
    pub cur: *mut yyjson_mut_val,
    pub pre: *mut yyjson_mut_val,
    pub obj: *mut yyjson_mut_val,
}

impl MutObjIter {
    pub unsafe fn new(obj: *mut yyjson_mut_val) -> MutObjIter {
        if !obj.is_null() && mut_get_type(obj) == YYJSON_TYPE_OBJ {
            let max = mut_get_len(obj);
            MutObjIter {
                idx: 0,
                max,
                cur: if max != 0 {
                    (*obj).uni.ptr as *mut yyjson_mut_val
                } else {
                    ptr::null_mut()
                },
                pre: ptr::null_mut(),
                obj,
            }
        } else {
            MutObjIter {
                idx: 0,
                max: 0,
                cur: ptr::null_mut(),
                pre: ptr::null_mut(),
                obj: ptr::null_mut(),
            }
        }
    }

    /// `yyjson_mut_obj_iter_getn`
    pub unsafe fn getn(&mut self, key: *const c_char, key_len: usize) -> *mut yyjson_mut_val {
        if !key.is_null() {
            let mut idx = 0usize;
            let max = self.max;
            let mut cur = self.cur;
            while idx < max {
                idx += 1;
                let pre = cur;
                cur = (*(*cur).next).next;
                if unsafe_equals_strn(cur as *const yyjson_val, key, key_len) {
                    self.idx += idx;
                    if self.idx > max {
                        self.idx -= max;
                    }
                    self.pre = pre;
                    self.cur = cur;
                    return (*cur).next;
                }
            }
        }
        ptr::null_mut()
    }

    /// `yyjson_mut_obj_iter_next`
    pub unsafe fn next(&mut self) -> *mut yyjson_mut_val {
        if self.idx < self.max {
            let key = self.cur;
            self.pre = key;
            self.cur = (*(*key).next).next;
            self.idx += 1;
            return self.cur;
        }
        ptr::null_mut()
    }

    /// `yyjson_mut_obj_iter_remove`
    pub unsafe fn remove(&mut self) -> *mut yyjson_mut_val {
        if !self.pre.is_null() && 0 < self.idx && self.idx <= self.max {
            let prev = self.pre;
            let cur = self.cur;
            let next = (*(*cur).next).next;
            if self.idx == self.max {
                (*self.obj).uni.ptr = prev as *mut c_void;
            }
            self.idx -= 1;
            self.max -= 1;
            mut_set_len(self.obj, self.max);
            (*(*prev).next).next = next;
            self.cur = prev;
            self.pre = ptr::null_mut();
            return (*cur).next;
        }
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_equals(
    lhs: *const yyjson_val,
    rhs: *const yyjson_val,
) -> bool {
    let mut lhs = lhs;
    let mut rhs = rhs;
    let ty = unsafe_get_type(lhs);
    if ty != unsafe_get_type(rhs) {
        return false;
    }

    match ty {
        YYJSON_TYPE_OBJ => {
            let mut len = unsafe_get_len(lhs);
            if len != unsafe_get_len(rhs) {
                return false;
            }
            if len > 0 {
                let mut iter = ObjIter::new(rhs);
                lhs = unsafe_get_first(lhs);
                while len > 0 {
                    len -= 1;
                    rhs = iter.getn((*lhs).uni.str_, unsafe_get_len(lhs));
                    if rhs.is_null() {
                        return false;
                    }
                    if !unsafe_yyjson_equals(lhs.add(1), rhs) {
                        return false;
                    }
                    lhs = unsafe_get_next(lhs.add(1));
                }
            }
            true
        }
        YYJSON_TYPE_ARR => {
            let mut len = unsafe_get_len(lhs);
            if len != unsafe_get_len(rhs) {
                return false;
            }
            if len > 0 {
                lhs = unsafe_get_first(lhs);
                rhs = unsafe_get_first(rhs);
                while len > 0 {
                    len -= 1;
                    if !unsafe_yyjson_equals(lhs, rhs) {
                        return false;
                    }
                    lhs = unsafe_get_next(lhs);
                    rhs = unsafe_get_next(rhs);
                }
            }
            true
        }
        YYJSON_TYPE_NUM => num_equals(lhs, rhs),
        YYJSON_TYPE_RAW | YYJSON_TYPE_STR => str_equals(lhs, rhs),
        YYJSON_TYPE_NULL | YYJSON_TYPE_BOOL => (*lhs).tag == (*rhs).tag,
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_equals(
    lhs: *const yyjson_mut_val,
    rhs: *const yyjson_mut_val,
) -> bool {
    let mut lhs = lhs;
    let mut rhs = rhs;
    let ty = mut_get_type(lhs);
    if ty != mut_get_type(rhs) {
        return false;
    }

    match ty {
        YYJSON_TYPE_OBJ => {
            let mut len = mut_get_len(lhs);
            if len != mut_get_len(rhs) {
                return false;
            }
            if len > 0 {
                let mut iter = MutObjIter::new(rhs as *mut yyjson_mut_val);
                lhs = (*lhs).uni.ptr as *mut yyjson_mut_val;
                while len > 0 {
                    len -= 1;
                    rhs = iter.getn((*lhs).uni.str_, mut_get_len(lhs));
                    if rhs.is_null() {
                        return false;
                    }
                    if !unsafe_yyjson_mut_equals((*lhs).next, rhs) {
                        return false;
                    }
                    lhs = (*(*lhs).next).next;
                }
            }
            true
        }
        YYJSON_TYPE_ARR => {
            let mut len = mut_get_len(lhs);
            if len != mut_get_len(rhs) {
                return false;
            }
            if len > 0 {
                lhs = (*lhs).uni.ptr as *mut yyjson_mut_val;
                rhs = (*rhs).uni.ptr as *mut yyjson_mut_val;
                while len > 0 {
                    len -= 1;
                    if !unsafe_yyjson_mut_equals(lhs, rhs) {
                        return false;
                    }
                    lhs = (*lhs).next;
                    rhs = (*rhs).next;
                }
            }
            true
        }
        YYJSON_TYPE_NUM => num_equals(lhs as *const yyjson_val, rhs as *const yyjson_val),
        YYJSON_TYPE_RAW | YYJSON_TYPE_STR => {
            str_equals(lhs as *const yyjson_val, rhs as *const yyjson_val)
        }
        YYJSON_TYPE_NULL | YYJSON_TYPE_BOOL => (*lhs).tag == (*rhs).tag,
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_locate_pos(
    str_: *const c_char,
    len: usize,
    pos: usize,
    line: *mut usize,
    col: *mut usize,
    chr: *mut usize,
) -> bool {
    let mut line_sum: usize = 0;
    let mut line_pos: usize = 0;
    let mut chr_sum: usize = 0;
    let mut cur = str_ as *const u8;

    if str_.is_null() || pos > len {
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
    let end = cur.add(pos);

    if pos >= 3 && is_utf8_bom(cur) {
        cur = cur.add(3);
    }
    while cur < end {
        let c = *cur;
        chr_sum += 1;
        if c < 0x80 {
            if c == b'\n' {
                line_sum += 1;
                line_pos = chr_sum;
            }
            cur = cur.add(1);
        } else if c < 0xC0 {
            cur = cur.add(1);
        } else if c < 0xE0 {
            cur = cur.add(2);
        } else if c < 0xF0 {
            cur = cur.add(3);
        } else if c < 0xF8 {
            cur = cur.add(4);
        } else {
            cur = cur.add(1);
        }
    }
    if !line.is_null() {
        *line = line_sum + 1;
    }
    if !col.is_null() {
        *col = chr_sum - line_pos + 1;
    }
    if !chr.is_null() {
        *chr = chr_sum;
    }
    true
}
