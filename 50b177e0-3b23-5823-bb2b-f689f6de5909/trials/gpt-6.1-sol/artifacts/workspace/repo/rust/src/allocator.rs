use crate::abi::Alc;
use core::{ffi::c_void, mem::size_of, ptr::{copy_nonoverlapping, null_mut}};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

unsafe extern "C" fn default_malloc(_: *mut c_void, size: usize) -> *mut c_void {
    malloc(size)
}
unsafe extern "C" fn default_realloc(_: *mut c_void, ptr: *mut c_void, _: usize, size: usize) -> *mut c_void {
    realloc(ptr, size)
}
unsafe extern "C" fn default_free(_: *mut c_void, ptr: *mut c_void) {
    free(ptr)
}
unsafe extern "C" fn null_malloc(_: *mut c_void, _: usize) -> *mut c_void { null_mut() }
unsafe extern "C" fn null_realloc(_: *mut c_void, _: *mut c_void, _: usize, _: usize) -> *mut c_void { null_mut() }
unsafe extern "C" fn null_free(_: *mut c_void, _: *mut c_void) {}

pub(crate) const DEFAULT_ALC: Alc = Alc {
    malloc: Some(default_malloc), realloc: Some(default_realloc),
    free: Some(default_free), ctx: null_mut(),
};
const NULL_ALC: Alc = Alc {
    malloc: Some(null_malloc), realloc: Some(null_realloc),
    free: Some(null_free), ctx: null_mut(),
};

impl Alc {
    pub(crate) unsafe fn alloc(self, size: usize) -> *mut c_void {
        match self.malloc { Some(f) => f(self.ctx, size), None => null_mut() }
    }
    pub(crate) unsafe fn resize(self, ptr: *mut c_void, old: usize, size: usize) -> *mut c_void {
        match self.realloc { Some(f) => f(self.ctx, ptr, old, size), None => null_mut() }
    }
    pub(crate) unsafe fn release(self, ptr: *mut c_void) {
        if let Some(f) = self.free { f(self.ctx, ptr); }
    }
}

// Both allocator chunk headers have this layout. The dynamic allocator uses
// independent allocations, while the fixed pool uses offsets within its buffer.
#[repr(C)]
struct Chunk {
    size: usize,
    next: *mut Chunk,
}
#[repr(C)]
struct PoolCtx {
    size: usize,
    free_list: *mut Chunk,
}
#[repr(C)]
struct DynCtx {
    free_list: Chunk,
    used_list: Chunk,
}

fn align_up(size: usize, alignment: usize) -> usize {
    size.wrapping_add(alignment - 1) & !(alignment - 1)
}
fn pool_align(size: usize) -> usize {
    align_up(size, size_of::<Chunk>()).wrapping_add(size_of::<Chunk>())
}

unsafe extern "C" fn pool_malloc(ctx: *mut c_void, size: usize) -> *mut c_void {
    let ctx = ctx.cast::<PoolCtx>();
    if size >= (*ctx).size { return null_mut(); }
    let size = pool_align(size);
    let mut prev: *mut Chunk = null_mut();
    let mut cur = (*ctx).free_list;
    while !cur.is_null() {
        if (*cur).size < size {
            prev = cur;
            cur = (*cur).next;
            continue;
        }
        let next;
        if (*cur).size >= size.wrapping_add(size_of::<Chunk>() * 2) {
            next = cur.cast::<u8>().add(size).cast::<Chunk>();
            (*next).size = (*cur).size - size;
            (*next).next = (*cur).next;
            (*cur).size = size;
        } else {
            next = (*cur).next;
        }
        if prev.is_null() { (*ctx).free_list = next; }
        else { (*prev).next = next; }
        return cur.add(1).cast();
    }
    null_mut()
}

unsafe extern "C" fn pool_free(ctx: *mut c_void, ptr: *mut c_void) {
    let ctx = ctx.cast::<PoolCtx>();
    let cur = ptr.cast::<Chunk>().sub(1);
    let mut prev: *mut Chunk = null_mut();
    let mut next = (*ctx).free_list;
    while !next.is_null() && (next as usize) < cur as usize {
        prev = next;
        next = (*next).next;
    }
    if prev.is_null() { (*ctx).free_list = cur; }
    else { (*prev).next = cur; }
    (*cur).next = next;
    if !next.is_null() && cur.cast::<u8>().add((*cur).size) == next.cast() {
        (*cur).size += (*next).size;
        (*cur).next = (*next).next;
    }
    if !prev.is_null() && prev.cast::<u8>().add((*prev).size) == cur.cast() {
        (*prev).size += (*cur).size;
        (*prev).next = (*cur).next;
    }
}

unsafe extern "C" fn pool_realloc(ctx: *mut c_void, ptr: *mut c_void, old: usize, size: usize) -> *mut c_void {
    let pool = ctx.cast::<PoolCtx>();
    let cur = ptr.cast::<Chunk>().sub(1);
    if size >= (*pool).size { return null_mut(); }
    let size = pool_align(size);
    if pool_align(old) == size { return ptr; }
    let mut prev: *mut Chunk = null_mut();
    let mut next = (*pool).free_list;
    while !next.is_null() && (next as usize) < cur as usize {
        prev = next;
        next = (*next).next;
    }
    if cur.cast::<u8>().add((*cur).size) == next.cast() && (*cur).size + (*next).size >= size {
        let free_size = (*cur).size + (*next).size - size;
        if free_size > size_of::<Chunk>() * 2 {
            let tmp = cur.cast::<u8>().add(size).cast::<Chunk>();
            if prev.is_null() { (*pool).free_list = tmp; }
            else { (*prev).next = tmp; }
            (*tmp).next = (*next).next;
            (*tmp).size = free_size;
            (*cur).size = size;
        } else {
            if prev.is_null() { (*pool).free_list = (*next).next; }
            else { (*prev).next = (*next).next; }
            (*cur).size += (*next).size;
        }
        ptr
    } else {
        let new_ptr = pool_malloc(ctx, size - size_of::<Chunk>());
        if !new_ptr.is_null() {
            copy_nonoverlapping(ptr.cast::<u8>(), new_ptr.cast(), (*cur).size - size_of::<Chunk>());
            pool_free(ctx, ptr);
        }
        new_ptr
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_alc_pool_init(alc: *mut Alc, buf: *mut c_void, size: usize) -> bool {
    if alc.is_null() { return false; }
    *alc = NULL_ALC;
    if size < size_of::<PoolCtx>() * 4 { return false; }
    let address = align_up(buf as usize, size_of::<PoolCtx>());
    if address == 0 { return false; }
    let size = size.wrapping_sub(address.wrapping_sub(buf as usize)) & !(size_of::<PoolCtx>() - 1);
    let ctx = address as *mut PoolCtx;
    let chunk = ctx.add(1).cast::<Chunk>();
    (*chunk).size = size - size_of::<PoolCtx>();
    (*chunk).next = null_mut();
    (*ctx).size = size;
    (*ctx).free_list = chunk;
    *alc = Alc { malloc: Some(pool_malloc), realloc: Some(pool_realloc), free: Some(pool_free), ctx: ctx.cast() };
    true
}

fn dyn_align(size: usize) -> Option<usize> {
    let result = align_up(size.wrapping_add(size_of::<Chunk>()), 0x1000);
    if result < size { None } else { Some(result) }
}

unsafe fn list_remove(list: *mut Chunk, chunk: *mut Chunk) {
    let mut prev = list;
    let mut cur = (*prev).next;
    while !cur.is_null() {
        if cur == chunk {
            (*prev).next = (*cur).next;
            (*cur).next = null_mut();
            return;
        }
        prev = cur;
        cur = (*cur).next;
    }
}
unsafe fn list_add(list: *mut Chunk, chunk: *mut Chunk) {
    (*chunk).next = (*list).next;
    (*list).next = chunk;
}

unsafe extern "C" fn dyn_malloc(ctx: *mut c_void, size: usize) -> *mut c_void {
    let size = match dyn_align(size) { Some(n) => n, None => return null_mut() };
    let ctx = ctx.cast::<DynCtx>();
    let used = core::ptr::addr_of_mut!((*ctx).used_list);
    if (*ctx).free_list.next.is_null() {
        let chunk = DEFAULT_ALC.alloc(size).cast::<Chunk>();
        if chunk.is_null() { return null_mut(); }
        (*chunk).size = size;
        (*chunk).next = null_mut();
        list_add(used, chunk);
        return chunk.add(1).cast();
    }
    let mut prev = core::ptr::addr_of_mut!((*ctx).free_list);
    loop {
        let chunk = (*prev).next;
        if (*chunk).size >= size {
            (*prev).next = (*chunk).next;
            list_add(used, chunk);
            return chunk.add(1).cast();
        }
        if (*chunk).next.is_null() {
            let new = DEFAULT_ALC.resize(chunk.cast(), (*chunk).size, size).cast::<Chunk>();
            if new.is_null() { return null_mut(); }
            (*prev).next = null_mut();
            (*new).size = size;
            list_add(used, new);
            return new.add(1).cast();
        }
        prev = chunk;
    }
}

unsafe extern "C" fn dyn_realloc(ctx: *mut c_void, ptr: *mut c_void, _: usize, size: usize) -> *mut c_void {
    let size = match dyn_align(size) { Some(n) => n, None => return null_mut() };
    let ctx = ctx.cast::<DynCtx>();
    let chunk = ptr.cast::<Chunk>().sub(1);
    if (*chunk).size >= size { return ptr; }
    let used = core::ptr::addr_of_mut!((*ctx).used_list);
    list_remove(used, chunk);
    let new = DEFAULT_ALC.resize(chunk.cast(), (*chunk).size, size).cast::<Chunk>();
    if new.is_null() {
        list_add(used, chunk);
        null_mut()
    } else {
        (*new).size = size;
        list_add(used, new);
        new.add(1).cast()
    }
}

unsafe extern "C" fn dyn_free(ctx: *mut c_void, ptr: *mut c_void) {
    let ctx = ctx.cast::<DynCtx>();
    let chunk = ptr.cast::<Chunk>().sub(1);
    list_remove(core::ptr::addr_of_mut!((*ctx).used_list), chunk);
    let mut prev = core::ptr::addr_of_mut!((*ctx).free_list);
    while !prev.is_null() {
        let next = (*prev).next;
        if next.is_null() || (*next).size >= (*chunk).size {
            (*chunk).next = next;
            (*prev).next = chunk;
            break;
        }
        prev = next;
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_alc_dyn_new() -> *mut Alc {
    let alc = DEFAULT_ALC.alloc(size_of::<Alc>() + size_of::<DynCtx>()).cast::<Alc>();
    if alc.is_null() { return null_mut(); }
    let ctx = alc.add(1).cast::<DynCtx>();
    core::ptr::write_bytes(ctx, 0, 1);
    *alc = Alc { malloc: Some(dyn_malloc), realloc: Some(dyn_realloc), free: Some(dyn_free), ctx: ctx.cast() };
    alc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_alc_dyn_free(alc: *mut Alc) {
    if alc.is_null() { return; }
    let ctx = alc.add(1).cast::<DynCtx>();
    let mut chunk = (*ctx).free_list.next;
    while !chunk.is_null() {
        let next = (*chunk).next;
        DEFAULT_ALC.release(chunk.cast());
        chunk = next;
    }
    chunk = (*ctx).used_list.next;
    while !chunk.is_null() {
        let next = (*chunk).next;
        DEFAULT_ALC.release(chunk.cast());
        chunk = next;
    }
    DEFAULT_ALC.release(alc.cast());
}
