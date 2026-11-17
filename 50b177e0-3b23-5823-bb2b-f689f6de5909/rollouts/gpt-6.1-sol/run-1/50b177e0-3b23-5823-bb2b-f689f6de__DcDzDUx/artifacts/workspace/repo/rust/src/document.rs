use crate::{abi::*, allocator::DEFAULT_ALC};
use core::{mem::size_of, ptr::{copy_nonoverlapping, null_mut, write_bytes}};

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_str_pool_grow(pool: *mut StrPool, alc: *const Alc, len: usize) -> bool {
    if len > usize::MAX - size_of::<StrChunk>() { return false; }
    let size = core::cmp::max((*pool).chunk_size, len + size_of::<StrChunk>());
    let chunk = (*alc).alloc(size).cast::<StrChunk>();
    if chunk.is_null() { return false; }
    (*chunk).next = (*pool).chunks;
    (*chunk).chunk_size = size;
    (*pool).chunks = chunk;
    (*pool).cur = chunk.add(1).cast();
    (*pool).end = chunk.cast::<u8>().add(size);
    (*pool).chunk_size = next_size((*pool).chunk_size, (*pool).chunk_size_max);
    true
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_val_pool_grow(pool: *mut ValPool, alc: *const Alc, count: usize) -> bool {
    if count > usize::MAX / size_of::<MutVal>() - 1 { return false; }
    let size = core::cmp::max((*pool).chunk_size, (count + 1) * size_of::<MutVal>());
    let chunk = (*alc).alloc(size).cast::<ValChunk>();
    if chunk.is_null() { return false; }
    (*chunk).next = (*pool).chunks;
    (*chunk).chunk_size = size;
    (*pool).chunks = chunk;
    (*pool).cur = chunk.cast::<MutVal>().add(1);
    (*pool).end = chunk.cast::<u8>().add(size).cast();
    (*pool).chunk_size = next_size((*pool).chunk_size, (*pool).chunk_size_max);
    true
}

fn next_size(current: usize, max: usize) -> usize {
    let size = core::cmp::min(current.wrapping_mul(2), max);
    if size < current { max } else { size }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_set_str_pool_size(doc: *mut MutDoc, len: usize) -> bool {
    if doc.is_null() || len == 0 || len > usize::MAX - size_of::<StrChunk>() { return false; }
    (*doc).str_pool.chunk_size = len + size_of::<StrChunk>();
    true
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_set_val_pool_size(doc: *mut MutDoc, count: usize) -> bool {
    if doc.is_null() || count == 0 || count > usize::MAX / size_of::<MutVal>() - 1 { return false; }
    (*doc).val_pool.chunk_size = (count + 1) * size_of::<MutVal>();
    true
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_new(alc: *const Alc) -> *mut MutDoc {
    let fallback = DEFAULT_ALC;
    let alc = if alc.is_null() { core::ptr::addr_of!(fallback) } else { alc };
    let doc = (*alc).alloc(size_of::<MutDoc>()).cast::<MutDoc>();
    if doc.is_null() { return null_mut(); }
    write_bytes(doc, 0, 1);
    (*doc).alc = *alc;
    (*doc).str_pool.chunk_size = 0x100;
    (*doc).str_pool.chunk_size_max = 0x10000000;
    (*doc).val_pool.chunk_size = 0x10 * size_of::<MutVal>();
    (*doc).val_pool.chunk_size_max = 0x1000000 * size_of::<MutVal>();
    doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_free(doc: *mut MutDoc) {
    if doc.is_null() { return; }
    let alc = (*doc).alc;
    write_bytes(core::ptr::addr_of_mut!((*doc).alc), 0, 1);
    let mut str_chunk = (*doc).str_pool.chunks;
    while !str_chunk.is_null() {
        let next = (*str_chunk).next;
        alc.release(str_chunk.cast());
        str_chunk = next;
    }
    let mut val_chunk = (*doc).val_pool.chunks;
    while !val_chunk.is_null() {
        let next = (*val_chunk).next;
        alc.release(val_chunk.cast());
        val_chunk = next;
    }
    alc.release(doc.cast());
}

unsafe fn mut_values(doc: *mut MutDoc, count: usize) -> *mut MutVal {
    let pool = core::ptr::addr_of_mut!((*doc).val_pool);
    let available = ((*pool).end as usize).wrapping_sub((*pool).cur as usize) / size_of::<MutVal>();
    if available < count && !unsafe_yyjson_val_pool_grow(pool, core::ptr::addr_of!((*doc).alc), count) {
        return null_mut();
    }
    let val = (*pool).cur;
    (*pool).cur = val.add(count);
    val
}

unsafe fn mut_string(doc: *mut MutDoc, str_: *const u8, len: usize) -> *mut u8 {
    if len == usize::MAX { return null_mut(); }
    let pool = core::ptr::addr_of_mut!((*doc).str_pool);
    let available = ((*pool).end as usize).wrapping_sub((*pool).cur as usize);
    if available <= len && !unsafe_yyjson_str_pool_grow(pool, core::ptr::addr_of!((*doc).alc), len + 1) {
        return null_mut();
    }
    let mem = (*pool).cur;
    (*pool).cur = mem.add(len + 1);
    if len != 0 { copy_nonoverlapping(str_, mem, len); }
    *mem.add(len) = 0;
    mem
}

pub(crate) unsafe fn imut_next(val: *const Val) -> *const Val {
    if (*val).tag & 6 == 6 { val.cast::<u8>().add((*val).uni.ofs).cast() }
    else { val.add(1) }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_mut_copy(doc: *mut MutDoc, vals: *const Val) -> *mut MutVal {
    if doc.is_null() || vals.is_null() { return null_mut(); }
    let end = imut_next(vals);
    let count = (end as usize - vals as usize) / size_of::<Val>();
    let output = mut_values(doc, count);
    if output.is_null() { return null_mut(); }
    let mut index = 0;
    while index < count {
        let src = vals.add(index);
        let dst = output.add(index);
        let kind = (*src).tag & 7;
        (*dst).tag = (*src).tag;
        (*dst).uni = (*src).uni;
        if kind == 1 || kind == 5 {
            let str_ = mut_string(doc, (*src).uni.str_, ((*src).tag >> 8) as usize);
            if str_.is_null() { return null_mut(); }
            (*dst).uni.str_ = str_;
        } else if kind == 6 || kind == 7 {
            let mut len = ((*src).tag >> 8) as usize;
            if len != 0 {
                let mut si = src.add(1);
                let mut di = dst.add(1);
                while len > 1 {
                    let snext = if kind == 7 { imut_next(si.add(1)) } else { imut_next(si) };
                    let step = (snext as usize - si as usize) / size_of::<Val>();
                    let dnext = di.add(step);
                    if kind == 7 {
                        (*di).next = di.add(1);
                        (*di.add(1)).next = dnext;
                    } else { (*di).next = dnext; }
                    si = snext;
                    di = dnext;
                    len -= 1;
                }
                if kind == 7 {
                    (*di).next = di.add(1);
                    (*di.add(1)).next = dst.add(1);
                } else { (*di).next = dst.add(1); }
                (*dst).uni.ptr = di.cast();
            }
        }
        index += 1;
    }
    output
}

unsafe fn copy_mut(doc: *mut MutDoc, src: *const MutVal) -> *mut MutVal {
    let dst = mut_values(doc, 1);
    if dst.is_null() { return null_mut(); }
    (*dst).tag = (*src).tag;
    match (*src).tag & 7 {
        6 | 7 => {
            if (*src).tag >> 8 != 0 {
                let last = (*src).uni.ptr.cast::<MutVal>();
                let mut next = (*last).next;
                let mut prev = copy_mut(doc, last);
                if prev.is_null() { return null_mut(); }
                (*dst).uni.ptr = prev.cast();
                while next != last {
                    (*prev).next = copy_mut(doc, next);
                    if (*prev).next.is_null() { return null_mut(); }
                    prev = (*prev).next;
                    next = (*next).next;
                }
                (*prev).next = (*dst).uni.ptr.cast();
            }
        }
        1 | 5 => {
            let str_ = mut_string(doc, (*src).uni.str_, ((*src).tag >> 8) as usize);
            if str_.is_null() { return null_mut(); }
            (*dst).uni.str_ = str_;
        }
        _ => { (*dst).uni = (*src).uni; }
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_mut_copy(doc: *mut MutDoc, val: *const MutVal) -> *mut MutVal {
    if doc.is_null() || val.is_null() { return null_mut(); }
    copy_mut(doc, val)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_doc_mut_copy(doc: *const Doc, alc: *const Alc) -> *mut MutDoc {
    if doc.is_null() || (*doc).root.is_null() { return null_mut(); }
    let dst = yyjson_mut_doc_new(alc);
    if dst.is_null() { return null_mut(); }
    let val = yyjson_val_mut_copy(dst, (*doc).root);
    if val.is_null() { yyjson_mut_doc_free(dst); return null_mut(); }
    (*dst).root = val;
    dst
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_mut_copy(doc: *const MutDoc, alc: *const Alc) -> *mut MutDoc {
    if doc.is_null() { return null_mut(); }
    if (*doc).root.is_null() { return yyjson_mut_doc_new(alc); }
    let dst = yyjson_mut_doc_new(alc);
    if dst.is_null() { return null_mut(); }
    let val = copy_mut(dst, (*doc).root);
    if val.is_null() { yyjson_mut_doc_free(dst); return null_mut(); }
    (*dst).root = val;
    dst
}

unsafe fn mutable_stats(val: *const MutVal, values: &mut usize, strings: &mut usize) {
    *values = values.wrapping_add(1);
    match (*val).tag & 7 {
        kind @ (6 | 7) => {
            let len = (((*val).tag >> 8) as usize) << usize::from(kind == 7);
            *values = values.wrapping_add(len);
            let mut child = (*val).uni.ptr.cast::<MutVal>();
            for _ in 0..len {
                match (*child).tag & 7 {
                    1 | 5 => { *strings = strings.wrapping_add(((*child).tag >> 8) as usize).wrapping_add(1); }
                    6 | 7 => {
                        mutable_stats(child, values, strings);
                        *values = values.wrapping_sub(1);
                    }
                    _ => {}
                }
                child = (*child).next;
            }
        }
        1 | 5 => { *strings = strings.wrapping_add(((*val).tag >> 8) as usize).wrapping_add(1); }
        _ => {}
    }
}

unsafe fn copy_to_imut(cursor: &mut *mut Val, strings: &mut *mut u8, src: *const MutVal) -> usize {
    let val = *cursor;
    match (*src).tag & 7 {
        kind @ (6 | 7) => {
            let mut child = (*src).uni.ptr.cast::<MutVal>();
            let mut len = ((*src).tag >> 8) as usize;
            if len != 0 {
                child = (*child).next;
                if kind == 7 { child = (*child).next; }
            }
            if kind == 7 { len <<= 1; }
            *cursor = val.add(1);
            let mut count: usize = 1;
            for _ in 0..len {
                count = count.wrapping_add(copy_to_imut(cursor, strings, child));
                child = (*child).next;
            }
            (*val).tag = (*src).tag;
            (*val).uni.ofs = count.wrapping_mul(size_of::<Val>());
            count
        }
        1 | 5 => {
            let buf = *strings;
            let len = ((*src).tag >> 8) as usize;
            if len != 0 { copy_nonoverlapping((*src).uni.str_, buf, len); }
            *buf.add(len) = 0;
            (*val).tag = (*src).tag;
            (*val).uni.str_ = buf;
            *cursor = val.add(1);
            *strings = buf.add(len + 1);
            1
        }
        _ => {
            (*val).tag = (*src).tag;
            (*val).uni = (*src).uni;
            *cursor = val.add(1);
            1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_imut_copy(val: *const MutVal, alc: *const Alc) -> *mut Doc {
    if val.is_null() { return null_mut(); }
    let fallback = DEFAULT_ALC;
    let alc = if alc.is_null() { core::ptr::addr_of!(fallback) } else { alc };
    let mut values = 0;
    let mut strings = 0;
    mutable_stats(val, &mut values, &mut strings);
    let header_size = (size_of::<Doc>() + size_of::<Val>() - 1) & !(size_of::<Val>() - 1);
    let size = header_size.wrapping_add(values.wrapping_mul(size_of::<Val>()));
    let doc = (*alc).alloc(size).cast::<Doc>();
    if doc.is_null() { return null_mut(); }
    write_bytes(doc, 0, 1);
    let mut cursor = doc.cast::<u8>().add(header_size).cast::<Val>();
    (*doc).root = cursor;
    (*doc).alc = *alc;
    let mut buf = null_mut();
    if strings > 0 {
        buf = (*alc).alloc(strings).cast::<u8>();
        (*doc).str_pool = buf;
        if buf.is_null() { (*alc).release(doc.cast()); return null_mut(); }
    }
    (*doc).val_read = copy_to_imut(&mut cursor, &mut buf, val);
    (*doc).dat_read = strings.wrapping_add(1);
    doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_doc_imut_copy(doc: *const MutDoc, alc: *const Alc) -> *mut Doc {
    if doc.is_null() { return null_mut(); }
    yyjson_mut_val_imut_copy((*doc).root, alc)
}
