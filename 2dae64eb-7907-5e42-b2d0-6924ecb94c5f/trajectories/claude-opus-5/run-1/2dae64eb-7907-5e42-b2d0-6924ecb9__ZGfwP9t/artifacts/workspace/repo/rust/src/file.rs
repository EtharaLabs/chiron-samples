//! "File Utils": the `FILE *` helpers shared by the reader and writer.
//!
//! The C code appends glibc's `"e"` (`O_CLOEXEC`) extension to the `fopen`
//! mode string when `__GLIBC_PREREQ(2, 7)` holds, so the same mode strings
//! are used here.

use core::ffi::{c_char, c_void};

use crate::types::{fopen, fread, FILE};

#[cfg(target_env = "gnu")]
const MODE_READ: &[u8] = b"rbe\0";
#[cfg(target_env = "gnu")]
const MODE_WRITE: &[u8] = b"wbe\0";

#[cfg(not(target_env = "gnu"))]
const MODE_READ: &[u8] = b"rb\0";
#[cfg(not(target_env = "gnu"))]
const MODE_WRITE: &[u8] = b"wb\0";

/// `fopen_readonly`
#[inline(always)]
pub unsafe fn fopen_readonly(path: *const c_char) -> *mut FILE {
    fopen(path, MODE_READ.as_ptr() as *const c_char)
}

/// `fopen_writeonly`
#[inline(always)]
pub unsafe fn fopen_writeonly(path: *const c_char) -> *mut FILE {
    fopen(path, MODE_WRITE.as_ptr() as *const c_char)
}

/// `fread_safe`: `fread(buf, 1, size, file)`
#[inline(always)]
pub unsafe fn fread_safe(buf: *mut c_void, size: usize, fp: *mut FILE) -> usize {
    fread(buf, 1, size, fp)
}
