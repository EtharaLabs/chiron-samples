//! Memory allocators: null, default (libc), pool and dynamic.
//!
//! Transliteration of the "Null/Default/Pool/Dynamic Memory Allocator"
//! sections of the original `src/yyjson.c`. The allocator contract is the C
//! one: `malloc(ctx, size)` is never called with `size == 0`, `realloc` is
//! only called to grow a live block and is given the old size, and `free` is
//! never called with `NULL`.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;

use crate::libc;
use crate::types::YyjsonAlc;
use crate::util::{mem_align_up, size_align_down, size_align_up, YYJSON_ALC_DYN_MIN_SIZE};

/* ===== null allocator =================================================== */

unsafe extern "C" fn null_malloc(_ctx: *mut c_void, _size: usize) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn null_realloc(
    _ctx: *mut c_void,
    _ptr: *mut c_void,
    _old: usize,
    _size: usize,
) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn null_free(_ctx: *mut c_void, _ptr: *mut c_void) {}

pub static YYJSON_NULL_ALC: YyjsonAlc = YyjsonAlc {
    malloc: Some(null_malloc),
    realloc: Some(null_realloc),
    free: Some(null_free),
    ctx: ptr::null_mut(),
};

/* ===== default allocator ================================================ */

unsafe extern "C" fn default_malloc(_ctx: *mut c_void, size: usize) -> *mut c_void {
    libc::malloc(size)
}

unsafe extern "C" fn default_realloc(
    _ctx: *mut c_void,
    ptr: *mut c_void,
    _old_size: usize,
    size: usize,
) -> *mut c_void {
    libc::realloc(ptr, size)
}

unsafe extern "C" fn default_free(_ctx: *mut c_void, ptr: *mut c_void) {
    libc::free(ptr)
}

pub static YYJSON_DEFAULT_ALC: YyjsonAlc = YyjsonAlc {
    malloc: Some(default_malloc),
    realloc: Some(default_realloc),
    free: Some(default_free),
    ctx: ptr::null_mut(),
};

/// `alc_ptr ? *alc_ptr : YYJSON_DEFAULT_ALC`
#[inline(always)]
pub unsafe fn alc_or_default(alc_ptr: *const YyjsonAlc) -> YyjsonAlc {
    if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    }
}

/* ===== pool allocator =================================================== */

#[repr(C)]
struct PoolChunk {
    size: usize,
    next: *mut PoolChunk,
}

#[repr(C)]
struct PoolCtx {
    size: usize,
    free_list: *mut PoolChunk,
}

#[inline(always)]
fn pool_size_align(size: usize) -> usize {
    size_align_up(size, size_of::<PoolChunk>()) + size_of::<PoolChunk>()
}

unsafe extern "C" fn pool_malloc(ctx_ptr: *mut c_void, size: usize) -> *mut c_void {
    let ctx = ctx_ptr as *mut PoolCtx;
    let mut prev: *mut PoolChunk = ptr::null_mut();
    let mut cur: *mut PoolChunk = (*ctx).free_list;

    if size >= (*ctx).size {
        return ptr::null_mut();
    }
    let size = pool_size_align(size);

    while !cur.is_null() {
        if (*cur).size < size {
            prev = cur;
            cur = (*cur).next;
            continue;
        }
        let next: *mut PoolChunk;
        if (*cur).size >= size + size_of::<PoolChunk>() * 2 {
            next = (cur as *mut u8).add(size) as *mut PoolChunk;
            (*next).size = (*cur).size - size;
            (*next).next = (*cur).next;
            (*cur).size = size;
        } else {
            next = (*cur).next;
        }
        if !prev.is_null() {
            (*prev).next = next;
        } else {
            (*ctx).free_list = next;
        }
        return cur.add(1) as *mut c_void;
    }
    ptr::null_mut()
}

unsafe extern "C" fn pool_free(ctx_ptr: *mut c_void, p: *mut c_void) {
    let ctx = ctx_ptr as *mut PoolCtx;
    let cur = (p as *mut PoolChunk).sub(1);
    let mut prev: *mut PoolChunk = ptr::null_mut();
    let mut next: *mut PoolChunk = (*ctx).free_list;

    while !next.is_null() && next < cur {
        prev = next;
        next = (*next).next;
    }
    if !prev.is_null() {
        (*prev).next = cur;
    } else {
        (*ctx).free_list = cur;
    }
    (*cur).next = next;

    if !next.is_null() && (cur as *mut u8).add((*cur).size) == next as *mut u8 {
        (*cur).size += (*next).size;
        (*cur).next = (*next).next;
    }
    if !prev.is_null() && (prev as *mut u8).add((*prev).size) == cur as *mut u8 {
        (*prev).size += (*cur).size;
        (*prev).next = (*cur).next;
    }
}

unsafe extern "C" fn pool_realloc(
    ctx_ptr: *mut c_void,
    p: *mut c_void,
    old_size: usize,
    size: usize,
) -> *mut c_void {
    let ctx = ctx_ptr as *mut PoolCtx;
    let cur = (p as *mut PoolChunk).sub(1);

    if size >= (*ctx).size {
        return ptr::null_mut();
    }
    let old_size = pool_size_align(old_size);
    let size = pool_size_align(size);
    if old_size == size {
        return p;
    }

    let mut prev: *mut PoolChunk = ptr::null_mut();
    let mut next: *mut PoolChunk = (*ctx).free_list;
    while !next.is_null() && next < cur {
        prev = next;
        next = (*next).next;
    }

    if (cur as *mut u8).add((*cur).size) == next as *mut u8
        && (*cur).size + (*next).size >= size
    {
        let free_size = (*cur).size + (*next).size - size;
        if free_size > size_of::<PoolChunk>() * 2 {
            let tmp = (cur as *mut u8).add(size) as *mut PoolChunk;
            if !prev.is_null() {
                (*prev).next = tmp;
            } else {
                (*ctx).free_list = tmp;
            }
            (*tmp).next = (*next).next;
            (*tmp).size = free_size;
            (*cur).size = size;
        } else {
            if !prev.is_null() {
                (*prev).next = (*next).next;
            } else {
                (*ctx).free_list = (*next).next;
            }
            (*cur).size += (*next).size;
        }
        p
    } else {
        let new_ptr = pool_malloc(ctx_ptr, size - size_of::<PoolChunk>());
        if !new_ptr.is_null() {
            libc::mem_copy(
                new_ptr as *mut u8,
                p as *const u8,
                (*cur).size - size_of::<PoolChunk>(),
            );
            pool_free(ctx_ptr, p);
        }
        new_ptr
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_alc_pool_init(
    alc: *mut YyjsonAlc,
    buf: *mut c_void,
    size: usize,
) -> bool {
    if alc.is_null() {
        return false;
    }
    *alc = YYJSON_NULL_ALC;
    if size < size_of::<PoolCtx>() * 4 {
        return false;
    }
    let ctx = mem_align_up(buf as *mut u8, size_of::<PoolCtx>()) as *mut PoolCtx;
    if ctx.is_null() {
        return false;
    }
    let mut size = size - ((ctx as *mut u8).offset_from(buf as *mut u8) as usize);
    size = size_align_down(size, size_of::<PoolCtx>());

    let chunk = ctx.add(1) as *mut PoolChunk;
    (*chunk).size = size - size_of::<PoolCtx>();
    (*chunk).next = ptr::null_mut();
    (*ctx).size = size;
    (*ctx).free_list = chunk;

    (*alc).malloc = Some(pool_malloc);
    (*alc).realloc = Some(pool_realloc);
    (*alc).free = Some(pool_free);
    (*alc).ctx = ctx as *mut c_void;
    true
}

/* ===== dynamic allocator ================================================ */

#[repr(C)]
#[derive(Clone, Copy)]
struct DynChunk {
    size: usize,
    next: *mut DynChunk,
}

#[repr(C)]
struct DynCtx {
    free_list: DynChunk,
    used_list: DynChunk,
}

#[inline(always)]
unsafe fn dyn_size_align(size: *mut usize) -> bool {
    let mut alc_size = (*size).wrapping_add(size_of::<DynChunk>());
    alc_size = size_align_up(alc_size, YYJSON_ALC_DYN_MIN_SIZE);
    if alc_size < *size {
        return false;
    }
    *size = alc_size;
    true
}

unsafe fn dyn_chunk_list_remove(list: *mut DynChunk, chunk: *mut DynChunk) {
    let mut prev = list;
    let mut cur = (*prev).next;
    while !cur.is_null() {
        if cur == chunk {
            (*prev).next = (*cur).next;
            (*cur).next = ptr::null_mut();
            return;
        }
        prev = cur;
        cur = (*cur).next;
    }
}

unsafe fn dyn_chunk_list_add(list: *mut DynChunk, chunk: *mut DynChunk) {
    (*chunk).next = (*list).next;
    (*list).next = chunk;
}

unsafe extern "C" fn dyn_malloc(ctx_ptr: *mut c_void, size: usize) -> *mut c_void {
    let def = YYJSON_DEFAULT_ALC;
    let ctx = ctx_ptr as *mut DynCtx;
    let mut size = size;
    if !dyn_size_align(&mut size) {
        return ptr::null_mut();
    }

    if (*ctx).free_list.next.is_null() {
        let chunk = def.alloc(size) as *mut DynChunk;
        if chunk.is_null() {
            return ptr::null_mut();
        }
        (*chunk).size = size;
        (*chunk).next = ptr::null_mut();
        dyn_chunk_list_add(&mut (*ctx).used_list, chunk);
        return chunk.add(1) as *mut c_void;
    }

    let mut prev: *mut DynChunk = &mut (*ctx).free_list;
    loop {
        let mut chunk = (*prev).next;
        if (*chunk).size >= size {
            (*prev).next = (*chunk).next;
            dyn_chunk_list_add(&mut (*ctx).used_list, chunk);
            return chunk.add(1) as *mut c_void;
        }
        if (*chunk).next.is_null() {
            chunk = def.realloc(chunk as *mut c_void, (*chunk).size, size) as *mut DynChunk;
            if chunk.is_null() {
                return ptr::null_mut();
            }
            (*prev).next = ptr::null_mut();
            (*chunk).size = size;
            dyn_chunk_list_add(&mut (*ctx).used_list, chunk);
            return chunk.add(1) as *mut c_void;
        }
        prev = chunk;
    }
}

unsafe extern "C" fn dyn_realloc(
    ctx_ptr: *mut c_void,
    p: *mut c_void,
    _old_size: usize,
    size: usize,
) -> *mut c_void {
    let def = YYJSON_DEFAULT_ALC;
    let ctx = ctx_ptr as *mut DynCtx;
    let mut chunk = (p as *mut DynChunk).sub(1);
    let mut size = size;
    if !dyn_size_align(&mut size) {
        return ptr::null_mut();
    }
    if (*chunk).size >= size {
        return p;
    }

    dyn_chunk_list_remove(&mut (*ctx).used_list, chunk);
    let new_chunk = def.realloc(chunk as *mut c_void, (*chunk).size, size) as *mut DynChunk;
    if !new_chunk.is_null() {
        (*new_chunk).size = size;
        chunk = new_chunk;
    }
    dyn_chunk_list_add(&mut (*ctx).used_list, chunk);
    if new_chunk.is_null() {
        ptr::null_mut()
    } else {
        new_chunk.add(1) as *mut c_void
    }
}

unsafe extern "C" fn dyn_free(ctx_ptr: *mut c_void, p: *mut c_void) {
    let ctx = ctx_ptr as *mut DynCtx;
    let chunk = (p as *mut DynChunk).sub(1);

    dyn_chunk_list_remove(&mut (*ctx).used_list, chunk);
    let mut prev: *mut DynChunk = &mut (*ctx).free_list;
    while !prev.is_null() {
        if (*prev).next.is_null() || (*(*prev).next).size >= (*chunk).size {
            (*chunk).next = (*prev).next;
            (*prev).next = chunk;
            break;
        }
        prev = (*prev).next;
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_alc_dyn_new() -> *mut YyjsonAlc {
    let def = YYJSON_DEFAULT_ALC;
    let hdr_len = size_of::<YyjsonAlc>() + size_of::<DynCtx>();
    let alc = def.alloc(hdr_len) as *mut YyjsonAlc;
    if alc.is_null() {
        return ptr::null_mut();
    }
    let ctx = alc.add(1) as *mut DynCtx;
    (*alc).malloc = Some(dyn_malloc);
    (*alc).realloc = Some(dyn_realloc);
    (*alc).free = Some(dyn_free);
    (*alc).ctx = ctx as *mut c_void;
    libc::zero(ctx);
    alc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_alc_dyn_free(alc: *mut YyjsonAlc) {
    let def = YYJSON_DEFAULT_ALC;
    if alc.is_null() {
        return;
    }
    let ctx = alc.add(1) as *mut DynCtx;
    let mut chunk = (*ctx).free_list.next;
    while !chunk.is_null() {
        let next = (*chunk).next;
        def.dealloc(chunk as *mut c_void);
        chunk = next;
    }
    let mut chunk = (*ctx).used_list.next;
    while !chunk.is_null() {
        let next = (*chunk).next;
        def.dealloc(chunk as *mut c_void);
        chunk = next;
    }
    def.dealloc(alc as *mut c_void);
}
