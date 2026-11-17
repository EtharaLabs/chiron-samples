// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 * MARK: - alloc.rs
 *
 * Owns yyjson.c:
 *   55-60     Version (Public):            yyjson_version
 *   2189-2208 Null Memory Allocator
 *   2212-2246 Default Memory Allocator
 *   2250-2403 Pool Memory Allocator:       yyjson_alc_pool_init
 *   2407-2559 Dynamic Memory Allocator:    yyjson_alc_dyn_new/_free
 *
 * Allocation sizes, alignment arithmetic and the order of callback calls match
 * the C exactly; the private `pool_chunk`/`pool_ctx`/`dyn_chunk`/`dyn_ctx`
 * headers are `#[repr(C)]` with the same field order, because their sizes reach
 * the allocator callbacks.
 *============================================================================*/

use crate::abi::libc::{self as ffi, c_void};
use crate::repr::shape::{YYJSON_VERSION_HEX, yyjson_alc};
use crate::mem::bytes::{
    YYJSON_ALC_DYN_MIN_SIZE, mem_align_up, size_align_down, size_align_up,
};

use core::mem::size_of;

/*==============================================================================
 * MARK: - Version (Public) (yyjson.c:58-60)
 *============================================================================*/

/// `uint32_t yyjson_version(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_version() -> u32 {
    YYJSON_VERSION_HEX
}

/*==============================================================================
 * MARK: - Null Memory Allocator (Private) (yyjson.c:2189-2208)
 * This allocator is just a placeholder to ensure that the internal
 * malloc/realloc/free function pointers are not null.
 *============================================================================*/

unsafe extern "C" fn null_malloc(_ctx: *mut c_void, _size: usize)
    -> *mut c_void {
    core::ptr::null_mut()
}

unsafe extern "C" fn null_realloc(_ctx: *mut c_void, _ptr: *mut c_void,
                                  _old_size: usize, _size: usize)
    -> *mut c_void {
    core::ptr::null_mut()
}

unsafe extern "C" fn null_free(_ctx: *mut c_void, _ptr: *mut c_void) {}

/// `static const yyjson_alc YYJSON_NULL_ALC = {
///      null_malloc, null_realloc, null_free, NULL };`
///
/// A `const` rather than a `static` so that, like the C object with internal
/// linkage, it contributes no symbol; every use in C copies the whole struct.
pub(crate) const YYJSON_NULL_ALC: yyjson_alc = yyjson_alc {
    malloc: Some(null_malloc),
    realloc: Some(null_realloc),
    free: Some(null_free),
    ctx: core::ptr::null_mut(),
};

/*==============================================================================
 * MARK: - Default Memory Allocator (Private) (yyjson.c:2212-2246)
 * This is a simple libc memory allocator wrapper.
 * Neither YYJSON_CUSTOM_ALC nor YYJSON_FREESTANDING is set, so the libc
 * branch is the active one.
 *============================================================================*/

unsafe extern "C" fn default_malloc(_ctx: *mut c_void, size: usize)
    -> *mut c_void {
    unsafe { ffi::malloc(size) }
}

unsafe extern "C" fn default_realloc(_ctx: *mut c_void, ptr: *mut c_void,
                                     _old_size: usize, size: usize)
    -> *mut c_void {
    unsafe { ffi::realloc(ptr, size) }
}

unsafe extern "C" fn default_free(_ctx: *mut c_void, ptr: *mut c_void) {
    unsafe { ffi::free(ptr) }
}

/// `static const yyjson_alc YYJSON_DEFAULT_ALC = {
///      default_malloc, default_realloc, default_free, NULL };`
pub(crate) const YYJSON_DEFAULT_ALC: yyjson_alc = yyjson_alc {
    malloc: Some(default_malloc),
    realloc: Some(default_realloc),
    free: Some(default_free),
    ctx: core::ptr::null_mut(),
};

/*==============================================================================
 * MARK: - Pool Memory Allocator (Public) (yyjson.c:2250-2403)
 * This allocator is initialized with a fixed-size buffer.
 * The buffer is split into multiple memory chunks for memory allocation.
 *============================================================================*/

/** memory chunk header */
#[repr(C)]
pub(crate) struct pool_chunk {
    /** chunk memory size, including chunk header */
    pub size: usize,
    /** linked list, nullable */
    pub next: *mut pool_chunk,
    /* char mem[]; flexible array member */
}

/** allocator ctx header */
#[repr(C)]
pub(crate) struct pool_ctx {
    /** total memory size, including ctx header */
    pub size: usize,
    /** linked list, nullable */
    pub free_list: *mut pool_chunk,
    /* pool_chunk chunks[]; flexible array member */
}

const _: () = assert!(size_of::<pool_chunk>() == 16);
const _: () = assert!(size_of::<pool_ctx>() == 16);
const _: () = assert!(core::mem::offset_of!(pool_chunk, size) == 0);
const _: () = assert!(core::mem::offset_of!(pool_chunk, next) == 8);
const _: () = assert!(core::mem::offset_of!(pool_ctx, size) == 0);
const _: () = assert!(core::mem::offset_of!(pool_ctx, free_list) == 8);

/** align up the input size to chunk size */
#[inline(always)]
unsafe fn pool_size_align(size: *mut usize) {
    unsafe {
        *size = size_align_up(*size, size_of::<pool_chunk>())
            .wrapping_add(size_of::<pool_chunk>());
    }
}

unsafe extern "C" fn pool_malloc(ctx_ptr: *mut c_void, size: usize)
    -> *mut c_void {
    unsafe {
        /* assert(size != 0) */
        let ctx = ctx_ptr as *mut pool_ctx;
        let next: *mut pool_chunk;
        let mut prev: *mut pool_chunk = core::ptr::null_mut();
        let mut cur: *mut pool_chunk = (*ctx).free_list;

        if size >= (*ctx).size {
            return core::ptr::null_mut();
        }
        let mut size = size;
        pool_size_align(&raw mut size);

        while !cur.is_null() {
            if (*cur).size < size {
                /* not enough space, try next chunk */
                prev = cur;
                cur = (*cur).next;
                continue;
            }
            if (*cur).size >= size.wrapping_add(size_of::<pool_chunk>() * 2) {
                /* too much space, split this chunk */
                next = (cur as *mut u8).add(size) as *mut pool_chunk;
                (*next).size = (*cur).size.wrapping_sub(size);
                (*next).next = (*cur).next;
                (*cur).size = size;
            } else {
                /* just enough space, use whole chunk */
                next = (*cur).next;
            }
            if !prev.is_null() {
                (*prev).next = next;
            } else {
                (*ctx).free_list = next;
            }
            return cur.add(1) as *mut c_void;
        }
        core::ptr::null_mut()
    }
}

unsafe extern "C" fn pool_free(ctx_ptr: *mut c_void, ptr: *mut c_void) {
    unsafe {
        /* assert(ptr != NULL) */
        let ctx = ctx_ptr as *mut pool_ctx;
        let cur = (ptr as *mut pool_chunk).sub(1);
        let mut prev: *mut pool_chunk = core::ptr::null_mut();
        let mut next: *mut pool_chunk = (*ctx).free_list;

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

        if !next.is_null()
            && (cur as *mut u8).add((*cur).size) == next as *mut u8
        {
            /* merge cur to higher chunk */
            (*cur).size = (*cur).size.wrapping_add((*next).size);
            (*cur).next = (*next).next;
        }
        if !prev.is_null()
            && (prev as *mut u8).add((*prev).size) == cur as *mut u8
        {
            /* merge cur to lower chunk */
            (*prev).size = (*prev).size.wrapping_add((*cur).size);
            (*prev).next = (*cur).next;
        }
    }
}

unsafe extern "C" fn pool_realloc(ctx_ptr: *mut c_void, ptr: *mut c_void,
                                  old_size: usize, size: usize)
    -> *mut c_void {
    unsafe {
        /* assert(ptr != NULL && size != 0 && old_size < size) */
        let ctx = ctx_ptr as *mut pool_ctx;
        let cur = (ptr as *mut pool_chunk).sub(1);
        let prev: *mut pool_chunk;
        let next: *mut pool_chunk;
        let tmp: *mut pool_chunk;

        /* check size */
        if size >= (*ctx).size {
            return core::ptr::null_mut();
        }
        let mut old_size = old_size;
        let mut size = size;
        pool_size_align(&raw mut old_size);
        pool_size_align(&raw mut size);
        if old_size == size {
            return ptr;
        }

        /* find next and prev chunk */
        let mut p: *mut pool_chunk = core::ptr::null_mut();
        let mut n: *mut pool_chunk = (*ctx).free_list;
        while !n.is_null() && n < cur {
            p = n;
            n = (*n).next;
        }
        prev = p;
        next = n;

        /* The C condition reads `next->size` only after establishing that
           `(u8 *)cur + cur->size == (u8 *)next`, which cannot hold for a NULL
           `next` (a chunk is never at address 0), so the dereference is as
           safe here as there; the short-circuit is kept explicit. */
        if (cur as *mut u8).add((*cur).size) == next as *mut u8
            && (*cur).size.wrapping_add((*next).size) >= size
        {
            /* merge to higher chunk if they are contiguous */
            let free_size =
                (*cur).size.wrapping_add((*next).size).wrapping_sub(size);
            if free_size > size_of::<pool_chunk>() * 2 {
                tmp = (cur as *mut u8).add(size) as *mut pool_chunk;
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
                (*cur).size = (*cur).size.wrapping_add((*next).size);
            }
            ptr
        } else {
            /* fallback to malloc and memcpy */
            let new_ptr = pool_malloc(ctx_ptr,
                                      size.wrapping_sub(
                                          size_of::<pool_chunk>()));
            if !new_ptr.is_null() {
                ffi::memcpy(new_ptr, ptr,
                            (*cur).size.wrapping_sub(
                                size_of::<pool_chunk>()));
                pool_free(ctx_ptr, ptr);
            }
            new_ptr
        }
    }
}

/// `bool yyjson_alc_pool_init(yyjson_alc *alc, void *buf, size_t size)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_alc_pool_init(alc: *mut yyjson_alc,
                                              buf: *mut c_void,
                                              size: usize) -> bool {
    unsafe {
        let chunk: *mut pool_chunk;
        let ctx: *mut pool_ctx;

        if alc.is_null() {
            return false;
        }
        *alc = YYJSON_NULL_ALC;
        if size < size_of::<pool_ctx>() * 4 {
            return false;
        }
        ctx = mem_align_up(buf, size_of::<pool_ctx>()) as *mut pool_ctx;
        if ctx.is_null() {
            return false;
        }
        let mut size = size;
        /* `(usize)((u8 *)ctx - (u8 *)buf)`: the alignment delta, computed on
           addresses so no provenance rule is involved. */
        size = size.wrapping_sub((ctx as usize).wrapping_sub(buf as usize));
        size = size_align_down(size, size_of::<pool_ctx>());

        chunk = ctx.add(1) as *mut pool_chunk;
        (*chunk).size = size.wrapping_sub(size_of::<pool_ctx>());
        (*chunk).next = core::ptr::null_mut();
        (*ctx).size = size;
        (*ctx).free_list = chunk;

        (*alc).malloc = Some(pool_malloc);
        (*alc).realloc = Some(pool_realloc);
        (*alc).free = Some(pool_free);
        (*alc).ctx = ctx as *mut c_void;
        true
    }
}

/*==============================================================================
 * MARK: - Dynamic Memory Allocator (Public) (yyjson.c:2407-2559)
 * This allocator allocates memory on demand and does not immediately release
 * unused memory. Instead, it places the unused memory into a freelist for
 * potential reuse in the future. It is only when the entire allocator is
 * destroyed that all previously allocated memory is released at once.
 *============================================================================*/

/** memory chunk header */
#[repr(C)]
pub(crate) struct dyn_chunk {
    /** chunk size, including header */
    pub size: usize,
    pub next: *mut dyn_chunk,
    /* char mem[]; flexible array member */
}

/** allocator ctx header */
#[repr(C)]
pub(crate) struct dyn_ctx {
    /** dummy header, sorted from small to large */
    pub free_list: dyn_chunk,
    /** dummy header */
    pub used_list: dyn_chunk,
}

const _: () = assert!(size_of::<dyn_chunk>() == 16);
const _: () = assert!(size_of::<dyn_ctx>() == 32);
const _: () = assert!(core::mem::offset_of!(dyn_chunk, size) == 0);
const _: () = assert!(core::mem::offset_of!(dyn_chunk, next) == 8);
const _: () = assert!(core::mem::offset_of!(dyn_ctx, free_list) == 0);
const _: () = assert!(core::mem::offset_of!(dyn_ctx, used_list) == 16);

/** align up the input size to chunk size */
#[inline(always)]
unsafe fn dyn_size_align(size: *mut usize) -> bool {
    unsafe {
        let mut alc_size = (*size).wrapping_add(size_of::<dyn_chunk>());
        alc_size = size_align_up(alc_size, YYJSON_ALC_DYN_MIN_SIZE);
        if alc_size < *size {
            return false; /* overflow */
        }
        *size = alc_size;
        true
    }
}

/** remove a chunk from list (the chunk must already be in the list) */
#[inline(always)]
unsafe fn dyn_chunk_list_remove(list: *mut dyn_chunk, chunk: *mut dyn_chunk) {
    unsafe {
        let mut prev: *mut dyn_chunk = list;
        let mut cur: *mut dyn_chunk = (*prev).next;
        while !cur.is_null() {
            if cur == chunk {
                (*prev).next = (*cur).next;
                (*cur).next = core::ptr::null_mut();
                return;
            }
            prev = cur;
            cur = (*cur).next;
        }
    }
}

/** add a chunk to list header (the chunk must not be in the list) */
#[inline(always)]
unsafe fn dyn_chunk_list_add(list: *mut dyn_chunk, chunk: *mut dyn_chunk) {
    unsafe {
        (*chunk).next = (*list).next;
        (*list).next = chunk;
    }
}

unsafe extern "C" fn dyn_malloc(ctx_ptr: *mut c_void, size: usize)
    -> *mut c_void {
    unsafe {
        /* assert(size != 0) */
        let def: yyjson_alc = YYJSON_DEFAULT_ALC;
        let ctx = ctx_ptr as *mut dyn_ctx;
        let mut chunk: *mut dyn_chunk;
        let mut prev: *mut dyn_chunk;
        let mut size = size;
        if !dyn_size_align(&raw mut size) {
            return core::ptr::null_mut();
        }

        /* freelist is empty, create new chunk */
        if (*ctx).free_list.next.is_null() {
            chunk = (def.malloc.unwrap_unchecked())(def.ctx, size)
                as *mut dyn_chunk;
            if chunk.is_null() {
                return core::ptr::null_mut();
            }
            (*chunk).size = size;
            (*chunk).next = core::ptr::null_mut();
            dyn_chunk_list_add(&raw mut (*ctx).used_list, chunk);
            return chunk.add(1) as *mut c_void;
        }

        /* find a large enough chunk, or resize the largest chunk */
        prev = &raw mut (*ctx).free_list;
        loop {
            chunk = (*prev).next;
            if (*chunk).size >= size {
                /* enough size, reuse this chunk */
                (*prev).next = (*chunk).next;
                dyn_chunk_list_add(&raw mut (*ctx).used_list, chunk);
                return chunk.add(1) as *mut c_void;
            }
            if (*chunk).next.is_null() {
                /* resize the largest chunk */
                chunk = (def.realloc.unwrap_unchecked())(
                    def.ctx, chunk as *mut c_void, (*chunk).size, size)
                    as *mut dyn_chunk;
                if chunk.is_null() {
                    return core::ptr::null_mut();
                }
                (*prev).next = core::ptr::null_mut();
                (*chunk).size = size;
                dyn_chunk_list_add(&raw mut (*ctx).used_list, chunk);
                return chunk.add(1) as *mut c_void;
            }
            prev = chunk;
        }
    }
}

unsafe extern "C" fn dyn_realloc(ctx_ptr: *mut c_void, ptr: *mut c_void,
                                 _old_size: usize, size: usize)
    -> *mut c_void {
    unsafe {
        /* assert(ptr != NULL && size != 0 && old_size < size) */
        let def: yyjson_alc = YYJSON_DEFAULT_ALC;
        let ctx = ctx_ptr as *mut dyn_ctx;
        let new_chunk: *mut dyn_chunk;
        let mut chunk = (ptr as *mut dyn_chunk).sub(1);
        let mut size = size;
        if !dyn_size_align(&raw mut size) {
            return core::ptr::null_mut();
        }
        if (*chunk).size >= size {
            return ptr;
        }

        dyn_chunk_list_remove(&raw mut (*ctx).used_list, chunk);
        new_chunk = (def.realloc.unwrap_unchecked())(
            def.ctx, chunk as *mut c_void, (*chunk).size, size)
            as *mut dyn_chunk;
        if !new_chunk.is_null() {
            (*new_chunk).size = size;
            chunk = new_chunk;
        }
        dyn_chunk_list_add(&raw mut (*ctx).used_list, chunk);
        if !new_chunk.is_null() {
            new_chunk.add(1) as *mut c_void
        } else {
            core::ptr::null_mut()
        }
    }
}

unsafe extern "C" fn dyn_free(ctx_ptr: *mut c_void, ptr: *mut c_void) {
    unsafe {
        /* assert(ptr != NULL) */
        let ctx = ctx_ptr as *mut dyn_ctx;
        let chunk = (ptr as *mut dyn_chunk).sub(1);
        let mut prev: *mut dyn_chunk;

        dyn_chunk_list_remove(&raw mut (*ctx).used_list, chunk);
        prev = &raw mut (*ctx).free_list;
        while !prev.is_null() {
            if (*prev).next.is_null()
                || (*(*prev).next).size >= (*chunk).size
            {
                (*chunk).next = (*prev).next;
                (*prev).next = chunk;
                break;
            }
            prev = (*prev).next;
        }
    }
}

/// `yyjson_alc *yyjson_alc_dyn_new(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_alc_dyn_new() -> *mut yyjson_alc {
    unsafe {
        let def: yyjson_alc = YYJSON_DEFAULT_ALC;
        let hdr_len = size_of::<yyjson_alc>() + size_of::<dyn_ctx>();
        let alc: *mut yyjson_alc;
        let ctx: *mut dyn_ctx;
        alc = (def.malloc.unwrap_unchecked())(def.ctx, hdr_len)
            as *mut yyjson_alc;
        if alc.is_null() {
            return core::ptr::null_mut();
        }
        ctx = alc.add(1) as *mut dyn_ctx;
        (*alc).malloc = Some(dyn_malloc);
        (*alc).realloc = Some(dyn_realloc);
        (*alc).free = Some(dyn_free);
        (*alc).ctx = alc.add(1) as *mut c_void;
        ffi::memset(ctx as *mut c_void, 0, size_of::<dyn_ctx>());
        alc
    }
}

/// `void yyjson_alc_dyn_free(yyjson_alc *alc)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yyjson_alc_dyn_free(alc: *mut yyjson_alc) {
    unsafe {
        let def: yyjson_alc = YYJSON_DEFAULT_ALC;
        let ctx: *mut dyn_ctx;
        let mut chunk: *mut dyn_chunk;
        let mut next: *mut dyn_chunk;
        if alc.is_null() {
            return;
        }
        ctx = alc.add(1) as *mut dyn_ctx;
        chunk = (*ctx).free_list.next;
        while !chunk.is_null() {
            next = (*chunk).next;
            (def.free.unwrap_unchecked())(def.ctx, chunk as *mut c_void);
            chunk = next;
        }
        chunk = (*ctx).used_list.next;
        while !chunk.is_null() {
            next = (*chunk).next;
            (def.free.unwrap_unchecked())(def.ctx, chunk as *mut c_void);
            chunk = next;
        }
        (def.free.unwrap_unchecked())(def.ctx, alc as *mut c_void);
    }
}
