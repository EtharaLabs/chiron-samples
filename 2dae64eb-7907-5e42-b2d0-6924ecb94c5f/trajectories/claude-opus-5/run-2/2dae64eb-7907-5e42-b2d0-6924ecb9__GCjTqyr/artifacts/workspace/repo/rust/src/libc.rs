//! The small slice of libc the port needs.
//!
//! The original `src/yyjson.c` uses `malloc`/`realloc`/`free` for the default
//! allocator, `memcpy`/`memmove`/`memset`/`memcmp`/`strlen` for byte work, and
//! `fopen`/`fread`/`fwrite`/`fseek`/`ftell`/`fclose`/`ferror` for the file
//! APIs. We declare exactly those, so the shared library's `NEEDED` list stays
//! `libc.so.6` and nothing else.

use core::ffi::{c_char, c_int, c_long, c_void};

/// Opaque stand-in for C's `FILE`.
pub enum File {}

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);

    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut File;
    pub fn fclose(file: *mut File) -> c_int;
    pub fn fread(buf: *mut c_void, size: usize, n: usize, file: *mut File) -> usize;
    pub fn fwrite(buf: *const c_void, size: usize, n: usize, file: *mut File) -> usize;
    pub fn fseek(file: *mut File, off: c_long, whence: c_int) -> c_int;
    pub fn ftell(file: *mut File) -> c_long;
    pub fn ferror(file: *mut File) -> c_int;

    pub fn abort() -> !;
}

pub const SEEK_SET: c_int = 0;
pub const SEEK_END: c_int = 2;

#[inline(always)]
pub unsafe fn mem_copy(dst: *mut u8, src: *const u8, n: usize) {
    memcpy(dst as *mut c_void, src as *const c_void, n);
}

#[inline(always)]
pub unsafe fn mem_move(dst: *mut u8, src: *const u8, n: usize) {
    memmove(dst as *mut c_void, src as *const c_void, n);
}

#[inline(always)]
pub unsafe fn mem_set(dst: *mut u8, c: u8, n: usize) {
    memset(dst as *mut c_void, c as c_int, n);
}

#[inline(always)]
pub unsafe fn memcmp_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    memcmp(a as *const c_void, b as *const c_void, n) == 0
}

/// Zero a `T` in place, matching the C code's `memset(&x, 0, sizeof(x))`.
#[inline(always)]
pub unsafe fn zero<T>(p: *mut T) {
    mem_set(p as *mut u8, 0, core::mem::size_of::<T>());
}
