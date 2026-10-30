//! yyjson — a Rust implementation of the yyjson C API.
//!
//! This crate is the implementation side of `src/yyjson.h`. The header is
//! unchanged, so every type here is laid out exactly like its C counterpart
//! and every exported function keeps the C name, signature and behaviour.
//!
//! The crate is `#![no_std]`: it uses only `core` plus a hand-declared set of
//! libc entry points (see [`libc`]). It never allocates through Rust's
//! allocator — all memory comes from the caller-supplied `yyjson_alc`, which
//! defaults to a thin `malloc`/`realloc`/`free` wrapper, exactly as in the C.

#![cfg_attr(not(test), no_std)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::identity_op)]

pub mod alc;
pub mod hdr;
pub mod incr;
pub mod libc;
pub mod pow10;
pub mod ptr;
pub mod read;
pub mod rnum;
pub mod rstr;
pub mod strct;
pub mod tables;
pub mod types;
pub mod util;
pub mod wnum;
pub mod write;

#[cfg(test)]
mod tests;

/// A Rust `panic` must never unwind into a C caller. The release profile sets
/// `panic = "abort"`, so no unwinding machinery exists at all; this handler is
/// the final backstop and terminates the process the same way `abort(3)` does.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { libc::abort() }
}
