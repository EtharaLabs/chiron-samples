//! Thin wrappers over the libc memory primitives the C implementation uses.
//!
//! Keeping these in one place documents exactly which libc symbols the shared
//! library depends on: `memcpy`, `memmove`, `memset`, `memcmp` (plus the
//! `malloc` family and the stdio calls declared in `types.rs`).

use core::ffi::c_void;

extern "C" {
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32;
}

/// `memcpy(dst, src, n)`, no overlap.
#[inline(always)]
pub unsafe fn copy(dst: *mut u8, src: *const u8, n: usize) {
    core::ptr::copy_nonoverlapping(src, dst, n);
}

/// `memmove(dst, src, n)`, overlap allowed.
#[inline(always)]
pub unsafe fn move_(dst: *mut u8, src: *const u8, n: usize) {
    core::ptr::copy(src, dst, n);
}

/// `memset(dst, v, n)`.
#[inline(always)]
pub unsafe fn set(dst: *mut u8, v: u8, n: usize) {
    core::ptr::write_bytes(dst, v, n);
}

/// `memcmp(a, b, n) == 0`.
#[inline(always)]
pub unsafe fn memcmp_eq(a: *const u8, b: *const u8, n: usize) -> bool {
    n == 0 || memcmp(a as *const c_void, b as *const c_void, n) == 0
}

/// `memcmp(a, b, n)`.
#[inline(always)]
pub unsafe fn memcmp_raw(a: *const u8, b: *const u8, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    memcmp(a as *const c_void, b as *const c_void, n)
}

/// `strlen(s)`.
#[inline(always)]
pub unsafe fn strlen(s: *const u8) -> usize {
    let mut p = s;
    while *p != 0 {
        p = p.add(1);
    }
    p.offset_from(s) as usize
}
