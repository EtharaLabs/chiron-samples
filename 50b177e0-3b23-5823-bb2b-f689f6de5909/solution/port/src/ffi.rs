// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
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
