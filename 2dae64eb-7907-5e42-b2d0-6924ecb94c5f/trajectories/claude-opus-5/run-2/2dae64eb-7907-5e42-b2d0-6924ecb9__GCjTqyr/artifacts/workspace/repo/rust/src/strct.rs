//! JSON document/value creation, copying, releasing and comparison.
//!
//! Transliteration of the "JSON Struct Utils (Public)" section of the
//! original `src/yyjson.c`.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alc::{alc_or_default, YYJSON_DEFAULT_ALC};
use crate::hdr;
use crate::libc;
use crate::types::*;
use crate::util::*;

/* ===== memory pools ===================================================== */

unsafe fn str_pool_release(pool: *mut YyjsonStrPool, alc: *const YyjsonAlc) {
    let mut chunk = (*pool).chunks;
    while !chunk.is_null() {
        let next = (*chunk).next;
        (*alc).dealloc(chunk as *mut c_void);
        chunk = next;
    }
}

unsafe fn val_pool_release(pool: *mut YyjsonValPool, alc: *const YyjsonAlc) {
    let mut chunk = (*pool).chunks;
    while !chunk.is_null() {
        let next = (*chunk).next;
        (*alc).dealloc(chunk as *mut c_void);
        chunk = next;
    }
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_str_pool_grow(
    pool: *mut YyjsonStrPool,
    alc: *const YyjsonAlc,
    len: usize,
) -> bool {
    let max_len = USIZE_MAX - size_of::<YyjsonStrChunk>();
    if len > max_len {
        return false;
    }
    let mut size = len + size_of::<YyjsonStrChunk>();
    if (*pool).chunk_size > size {
        size = (*pool).chunk_size;
    }
    let chunk = (*alc).alloc(size) as *mut YyjsonStrChunk;
    if chunk.is_null() {
        return false;
    }

    (*chunk).next = (*pool).chunks;
    (*chunk).chunk_size = size;
    (*pool).chunks = chunk;
    (*pool).cur = (chunk as *mut u8).add(size_of::<YyjsonStrChunk>()) as *mut c_char;
    (*pool).end = (chunk as *mut u8).add(size) as *mut c_char;

    let mut size = (*pool).chunk_size.wrapping_mul(2);
    if size > (*pool).chunk_size_max {
        size = (*pool).chunk_size_max;
    }
    if size < (*pool).chunk_size {
        size = (*pool).chunk_size_max;
    }
    (*pool).chunk_size = size;
    true
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_val_pool_grow(
    pool: *mut YyjsonValPool,
    alc: *const YyjsonAlc,
    count: usize,
) -> bool {
    let max_count = USIZE_MAX / size_of::<YyjsonMutVal>() - 1;
    if count > max_count {
        return false;
    }
    let mut size = (count + 1) * size_of::<YyjsonMutVal>();
    if (*pool).chunk_size > size {
        size = (*pool).chunk_size;
    }
    let chunk = (*alc).alloc(size) as *mut YyjsonValChunk;
    if chunk.is_null() {
        return false;
    }

    (*chunk).next = (*pool).chunks;
    (*chunk).chunk_size = size;
    (*pool).chunks = chunk;
    (*pool).cur = (chunk as *mut YyjsonMutVal).add(1);
    (*pool).end = (chunk as *mut u8).add(size) as *mut YyjsonMutVal;

    let mut size = (*pool).chunk_size.wrapping_mul(2);
    if size > (*pool).chunk_size_max {
        size = (*pool).chunk_size_max;
    }
    if size < (*pool).chunk_size {
        size = (*pool).chunk_size_max;
    }
    (*pool).chunk_size = size;
    true
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_set_str_pool_size(
    doc: *mut YyjsonMutDoc,
    len: usize,
) -> bool {
    let max_size = USIZE_MAX - size_of::<YyjsonStrChunk>();
    if doc.is_null() || len == 0 || len > max_size {
        return false;
    }
    (*doc).str_pool.chunk_size = len + size_of::<YyjsonStrChunk>();
    true
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_set_val_pool_size(
    doc: *mut YyjsonMutDoc,
    count: usize,
) -> bool {
    let max_count = USIZE_MAX / size_of::<YyjsonMutVal>() - 1;
    if doc.is_null() || count == 0 || count > max_count {
        return false;
    }
    (*doc).val_pool.chunk_size = (count + 1) * size_of::<YyjsonMutVal>();
    true
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_free(doc: *mut YyjsonMutDoc) {
    if !doc.is_null() {
        let alc = (*doc).alc;
        libc::zero(&mut (*doc).alc);
        str_pool_release(&mut (*doc).str_pool, &alc);
        val_pool_release(&mut (*doc).val_pool, &alc);
        alc.dealloc(doc as *mut c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_new(alc_ptr: *const YyjsonAlc) -> *mut YyjsonMutDoc {
    let alc = if alc_ptr.is_null() {
        &YYJSON_DEFAULT_ALC as *const YyjsonAlc
    } else {
        alc_ptr
    };
    let doc = (*alc).alloc(size_of::<YyjsonMutDoc>()) as *mut YyjsonMutDoc;
    if doc.is_null() {
        return ptr::null_mut();
    }
    libc::zero(doc);

    (*doc).alc = *alc;
    (*doc).str_pool.chunk_size = YYJSON_MUT_DOC_STR_POOL_INIT_SIZE;
    (*doc).str_pool.chunk_size_max = YYJSON_MUT_DOC_STR_POOL_MAX_SIZE;
    (*doc).val_pool.chunk_size = YYJSON_MUT_DOC_VAL_POOL_INIT_SIZE;
    (*doc).val_pool.chunk_size_max = YYJSON_MUT_DOC_VAL_POOL_MAX_SIZE;
    doc
}

/* ===== copies =========================================================== */

#[no_mangle]
pub unsafe extern "C" fn yyjson_doc_mut_copy(
    doc: *const YyjsonDoc,
    alc: *const YyjsonAlc,
) -> *mut YyjsonMutDoc {
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
    doc: *const YyjsonMutDoc,
    alc: *const YyjsonAlc,
) -> *mut YyjsonMutDoc {
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
    m_doc: *mut YyjsonMutDoc,
    i_vals: *const YyjsonVal,
) -> *mut YyjsonMutVal {
    if m_doc.is_null() || i_vals.is_null() {
        return ptr::null_mut();
    }
    let i_end = get_next(i_vals);
    let i_vals_len = i_end.offset_from(i_vals as *mut YyjsonVal) as usize;
    let m_vals = hdr::mut_val(m_doc, i_vals_len);
    if m_vals.is_null() {
        return ptr::null_mut();
    }
    let mut i_val = i_vals as *mut YyjsonVal;
    let mut m_val = m_vals;

    while i_val < i_end {
        let ty = get_type(i_val);
        (*m_val).tag = (*i_val).tag;
        (*m_val).uni.u64_ = (*i_val).uni.u64_;
        if ty == YYJSON_TYPE_STR || ty == YYJSON_TYPE_RAW {
            let s = (*i_val).uni.str_;
            let str_len = get_len(i_val);
            (*m_val).uni.str_ = hdr::mut_strncpy(m_doc, s, str_len);
            if (*m_val).uni.str_.is_null() {
                return ptr::null_mut();
            }
        } else if ty == YYJSON_TYPE_ARR {
            let mut len = get_len(i_val);
            if len > 0 {
                let mut ii_val = i_val.add(1);
                let mut mm_val = m_val.add(1);
                let mm_ctn = m_val;
                while len > 1 {
                    len -= 1;
                    let ii_next = get_next(ii_val);
                    let mm_next = mm_val.offset(ii_next.offset_from(ii_val));
                    (*mm_val).next = mm_next;
                    ii_val = ii_next;
                    mm_val = mm_next;
                }
                (*mm_val).next = mm_ctn.add(1);
                (*mm_ctn).uni.ptr = mm_val as *mut c_void;
            }
        } else if ty == YYJSON_TYPE_OBJ {
            let mut len = get_len(i_val);
            if len > 0 {
                let mut ii_key = i_val.add(1);
                let mut mm_key = m_val.add(1);
                let mm_ctn = m_val;
                while len > 1 {
                    len -= 1;
                    let ii_nextkey = get_next(ii_key.add(1));
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

unsafe fn unsafe_mut_val_mut_copy(
    m_doc: *mut YyjsonMutDoc,
    m_vals: *const YyjsonMutVal,
) -> *mut YyjsonMutVal {
    let m_val = hdr::mut_val(m_doc, 1);
    if m_val.is_null() {
        return ptr::null_mut();
    }
    (*m_val).tag = (*m_vals).tag;

    match get_type(mut_as_val(m_vals)) {
        YYJSON_TYPE_OBJ | YYJSON_TYPE_ARR => {
            if get_len(mut_as_val(m_vals)) > 0 {
                let last = (*m_vals).uni.ptr as *mut YyjsonMutVal;
                let mut next = (*last).next;
                let mut prev = unsafe_mut_val_mut_copy(m_doc, last);
                if prev.is_null() {
                    return ptr::null_mut();
                }
                (*m_val).uni.ptr = prev as *mut c_void;
                while next != last {
                    (*prev).next = unsafe_mut_val_mut_copy(m_doc, next);
                    if (*prev).next.is_null() {
                        return ptr::null_mut();
                    }
                    prev = (*prev).next;
                    next = (*next).next;
                }
                (*prev).next = (*m_val).uni.ptr as *mut YyjsonMutVal;
            }
        }
        YYJSON_TYPE_RAW | YYJSON_TYPE_STR => {
            let s = (*m_vals).uni.str_;
            let str_len = get_len(mut_as_val(m_vals));
            (*m_val).uni.str_ = hdr::mut_strncpy(m_doc, s, str_len);
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
    doc: *mut YyjsonMutDoc,
    val: *const YyjsonMutVal,
) -> *mut YyjsonMutVal {
    if !doc.is_null() && !val.is_null() {
        return unsafe_mut_val_mut_copy(doc, val);
    }
    ptr::null_mut()
}

/// `yyjson_mut_stat`: count values and total string length.
unsafe fn mut_stat(val: *const YyjsonMutVal, val_sum: *mut usize, str_sum: *mut usize) {
    let ty = get_type(mut_as_val(val));
    *val_sum += 1;
    if ty == YYJSON_TYPE_ARR || ty == YYJSON_TYPE_OBJ {
        let mut child = (*val).uni.ptr as *mut YyjsonMutVal;
        let mut len = get_len(mut_as_val(val));
        len <<= (ty == YYJSON_TYPE_OBJ) as u8;
        *val_sum += len;
        for _ in 0..len {
            let stype = get_type(mut_as_val(child));
            if stype == YYJSON_TYPE_STR || stype == YYJSON_TYPE_RAW {
                *str_sum += get_len(mut_as_val(child)) + 1;
            } else if stype == YYJSON_TYPE_ARR || stype == YYJSON_TYPE_OBJ {
                mut_stat(child, val_sum, str_sum);
                *val_sum -= 1;
            }
            child = (*child).next;
        }
    } else if ty == YYJSON_TYPE_STR || ty == YYJSON_TYPE_RAW {
        *str_sum += get_len(mut_as_val(val)) + 1;
    }
}

/// `yyjson_imut_copy`: copy mutable values into an immutable value pool.
unsafe fn imut_copy(
    val_ptr: *mut *mut YyjsonVal,
    buf_ptr: *mut *mut c_char,
    mval: *const YyjsonMutVal,
) -> usize {
    let val = *val_ptr;
    let ty = get_type(mut_as_val(mval));
    if ty == YYJSON_TYPE_ARR || ty == YYJSON_TYPE_OBJ {
        let mut child = (*mval).uni.ptr as *mut YyjsonMutVal;
        let mut len = get_len(mut_as_val(mval));
        let mut val_sum = 1usize;
        if ty == YYJSON_TYPE_OBJ {
            if len != 0 {
                child = (*(*child).next).next;
            }
            len <<= 1;
        } else if len != 0 {
            child = (*child).next;
        }
        *val_ptr = val.add(1);
        for _ in 0..len {
            val_sum += imut_copy(val_ptr, buf_ptr, child);
            child = (*child).next;
        }
        (*val).tag = (*mval).tag;
        (*val).uni.ofs = val_sum * size_of::<YyjsonVal>();
        val_sum
    } else if ty == YYJSON_TYPE_STR || ty == YYJSON_TYPE_RAW {
        let buf = *buf_ptr;
        let len = get_len(mut_as_val(mval));
        libc::mem_copy(buf as *mut u8, (*mval).uni.str_ as *const u8, len);
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
    mdoc: *const YyjsonMutDoc,
    alc: *const YyjsonAlc,
) -> *mut YyjsonDoc {
    if mdoc.is_null() {
        return ptr::null_mut();
    }
    yyjson_mut_val_imut_copy((*mdoc).root, alc)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_imut_copy(
    mval: *const YyjsonMutVal,
    alc_ptr: *const YyjsonAlc,
) -> *mut YyjsonDoc {
    let mut val_num = 0usize;
    let mut str_sum = 0usize;

    // Mirrors the C: `str_hdr` starts out non-NULL only to quiet an analyzer
    // warning; it is never dereferenced unless `str_sum > 0`.
    let mut str_hdr: *mut c_char = &mut str_sum as *mut usize as *mut c_char;
    if mval.is_null() {
        return ptr::null_mut();
    }
    let alc = if alc_ptr.is_null() {
        &YYJSON_DEFAULT_ALC as *const YyjsonAlc
    } else {
        alc_ptr
    };

    mut_stat(mval, &mut val_num, &mut str_sum);

    let hdr_size = size_align_up(size_of::<YyjsonDoc>(), size_of::<YyjsonVal>());
    let buf_size = hdr_size + val_num * size_of::<YyjsonVal>();
    let doc = (*alc).alloc(buf_size) as *mut YyjsonDoc;
    if doc.is_null() {
        return ptr::null_mut();
    }
    libc::zero(doc);
    let mut val_hdr = (doc as *mut u8).add(hdr_size) as *mut YyjsonVal;
    (*doc).root = val_hdr;
    (*doc).alc = *alc;

    if str_sum > 0 {
        str_hdr = (*alc).alloc(str_sum) as *mut c_char;
        (*doc).str_pool = str_hdr;
        if str_hdr.is_null() {
            (*alc).dealloc(doc as *mut c_void);
            return ptr::null_mut();
        }
    }

    (*doc).val_read = imut_copy(&mut val_hdr, &mut str_hdr, mval);
    (*doc).dat_read = str_sum + 1;
    doc
}

/* ===== equality ========================================================= */

#[inline]
unsafe fn num_equals(lhs: *const YyjsonVal, rhs: *const YyjsonVal) -> bool {
    let luni = &(*lhs).uni;
    let runi = &(*rhs).uni;
    let lt = get_subtype(lhs);
    let rt = get_subtype(rhs);
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

#[inline]
unsafe fn str_equals(lhs: *const YyjsonVal, rhs: *const YyjsonVal) -> bool {
    let len = get_len(lhs);
    if len != get_len(rhs) {
        return false;
    }
    libc::memcmp_eq(get_str(lhs) as *const u8, get_str(rhs) as *const u8, len)
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_equals(
    lhs: *const YyjsonVal,
    rhs: *const YyjsonVal,
) -> bool {
    let mut lhs = lhs;
    let mut rhs = rhs;
    let ty = get_type(lhs);
    if ty != get_type(rhs) {
        return false;
    }

    match ty {
        YYJSON_TYPE_OBJ => {
            let mut len = get_len(lhs);
            if len != get_len(rhs) {
                return false;
            }
            if len > 0 {
                let mut iter = core::mem::MaybeUninit::<YyjsonObjIter>::uninit();
                let iter = iter.as_mut_ptr();
                hdr::obj_iter_init(rhs, iter);
                lhs = get_first(lhs);
                while len > 0 {
                    len -= 1;
                    rhs = hdr::obj_iter_getn(iter, (*lhs).uni.str_, get_len(lhs));
                    if rhs.is_null() {
                        return false;
                    }
                    if !unsafe_yyjson_equals(lhs.add(1), rhs) {
                        return false;
                    }
                    lhs = get_next((lhs as *mut YyjsonVal).add(1));
                }
            }
            true
        }
        YYJSON_TYPE_ARR => {
            let mut len = get_len(lhs);
            if len != get_len(rhs) {
                return false;
            }
            if len > 0 {
                lhs = get_first(lhs);
                rhs = get_first(rhs);
                while len > 0 {
                    len -= 1;
                    if !unsafe_yyjson_equals(lhs, rhs) {
                        return false;
                    }
                    lhs = get_next(lhs);
                    rhs = get_next(rhs);
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
    lhs: *const YyjsonMutVal,
    rhs: *const YyjsonMutVal,
) -> bool {
    let mut lhs = lhs;
    let mut rhs = rhs;
    let ty = get_type(mut_as_val(lhs));
    if ty != get_type(mut_as_val(rhs)) {
        return false;
    }

    match ty {
        YYJSON_TYPE_OBJ => {
            let mut len = get_len(mut_as_val(lhs));
            if len != get_len(mut_as_val(rhs)) {
                return false;
            }
            if len > 0 {
                let mut iter = core::mem::MaybeUninit::<YyjsonMutObjIter>::uninit();
                let iter = iter.as_mut_ptr();
                hdr::mut_obj_iter_init(rhs as *mut YyjsonMutVal, iter);
                lhs = (*lhs).uni.ptr as *mut YyjsonMutVal;
                while len > 0 {
                    len -= 1;
                    rhs = hdr::mut_obj_iter_getn(
                        iter,
                        (*lhs).uni.str_,
                        get_len(mut_as_val(lhs)),
                    );
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
            let mut len = get_len(mut_as_val(lhs));
            if len != get_len(mut_as_val(rhs)) {
                return false;
            }
            if len > 0 {
                lhs = (*lhs).uni.ptr as *mut YyjsonMutVal;
                rhs = (*rhs).uni.ptr as *mut YyjsonMutVal;
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
        YYJSON_TYPE_NUM => num_equals(mut_as_val(lhs), mut_as_val(rhs)),
        YYJSON_TYPE_RAW | YYJSON_TYPE_STR => str_equals(mut_as_val(lhs), mut_as_val(rhs)),
        YYJSON_TYPE_NULL | YYJSON_TYPE_BOOL => (*lhs).tag == (*rhs).tag,
        _ => false,
    }
}

/* ===== text locating ==================================================== */

#[no_mangle]
pub unsafe extern "C" fn yyjson_locate_pos(
    str_: *const c_char,
    len: usize,
    pos: usize,
    line: *mut usize,
    col: *mut usize,
    chr: *mut usize,
) -> bool {
    let mut line_sum = 0usize;
    let mut line_pos = 0usize;
    let mut chr_sum = 0usize;
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

/* ===== version ========================================================== */

/// `YYJSON_VERSION_HEX`
pub const YYJSON_VERSION_HEX: u32 = 0x000D00;

#[no_mangle]
pub extern "C" fn yyjson_version() -> u32 {
    YYJSON_VERSION_HEX
}

#[inline(always)]
pub unsafe fn alc_arg(alc_ptr: *const YyjsonAlc) -> YyjsonAlc {
    alc_or_default(alc_ptr)
}
