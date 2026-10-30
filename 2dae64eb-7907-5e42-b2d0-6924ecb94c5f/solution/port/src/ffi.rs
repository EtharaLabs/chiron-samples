// FORGE-CANARY-BEGIN
// FORGE-CANARY-SLOT-1: 60ae0f0c079c366733615a2e6551dc4e64c530f1c56ffed9db295862b101ddc8
// FORGE-CANARY-SLOT-2: 87ab65e4ae3bd13b9cfb24e8b811c4ab101c8552a8b3d1f0d91c32bbd429958a
// FORGE-CANARY-SLOT-3: 76135f235d13f30c4446a51ada3f9a19b6fba2e250fa6b2ffc5a14d1b3f55ffb
// FORGE-CANARY-SLOT-4: e415e80ce167e9922758ef5d43162679b0fe51046883994e0fc30102c1e183f5
// FORGE-CANARY-END
/*==============================================================================
 * MARK: - libc declarations
 *
 * Exactly the libc functions yyjson.c calls, plus `abort` for the panic
 * handler. The set was taken from the undefined symbols of the State A shared
 * library built in the dev container:
 *
 *   fclose ferror fopen fread free fseek ftell fwrite
 *   malloc memcmp memcpy memmove memset realloc
 *
 * `abort` is the only addition; it is needed because a `#![no_std]` cdylib
 * must supply a `#[panic_handler]`, and aborting is the closest match to the C
 * library's behaviour (which has no unwinding at all).
 *============================================================================*/

/// `void` for pointer payloads (`void *` / `const void *`).
pub(crate) use core::ffi::c_void;

/// `char` in exported signatures and FFI only; `u8` is used internally.
/// On aarch64 this is `u8`, on x86_64 `i8`; both are 1 byte, so struct and
/// signature layouts are identical either way.
pub(crate) use core::ffi::c_char;

/// `int` as returned by `memcmp`/`ferror`.
pub(crate) use core::ffi::c_int;

/// `long` as used by `fseek`/`ftell`. LP64, so 64 bits.
pub(crate) use core::ffi::c_long;

/// Opaque `FILE`. yyjson.c never looks inside it; it only passes the pointer
/// to libc. Declared with a private zero-sized member so Rust treats it as an
/// opaque extern type with no accidental layout guarantees.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

/// `SEEK_SET` from glibc's `<stdio.h>`.
pub(crate) const SEEK_SET: c_int = 0;
/// `SEEK_END` from glibc's `<stdio.h>`.
pub(crate) const SEEK_END: c_int = 2;

#[link(name = "c")]
unsafe extern "C" {
    /* <stdlib.h> */
    pub(crate) fn malloc(size: usize) -> *mut c_void;
    pub(crate) fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub(crate) fn free(ptr: *mut c_void);
    pub(crate) fn abort() -> !;

    /* <string.h> */
    pub(crate) fn memcpy(dst: *mut c_void, src: *const c_void, n: usize)
        -> *mut c_void;
    pub(crate) fn memmove(dst: *mut c_void, src: *const c_void, n: usize)
        -> *mut c_void;
    pub(crate) fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub(crate) fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;

    /* <stdio.h> */
    pub(crate) fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub(crate) fn fclose(fp: *mut FILE) -> c_int;
    pub(crate) fn fread(buf: *mut c_void, size: usize, n: usize, fp: *mut FILE)
        -> usize;
    pub(crate) fn fwrite(buf: *const c_void, size: usize, n: usize,
                         fp: *mut FILE) -> usize;
    pub(crate) fn ferror(fp: *mut FILE) -> c_int;
    pub(crate) fn fseek(fp: *mut FILE, off: c_long, whence: c_int) -> c_int;
    pub(crate) fn ftell(fp: *mut FILE) -> c_long;
}
