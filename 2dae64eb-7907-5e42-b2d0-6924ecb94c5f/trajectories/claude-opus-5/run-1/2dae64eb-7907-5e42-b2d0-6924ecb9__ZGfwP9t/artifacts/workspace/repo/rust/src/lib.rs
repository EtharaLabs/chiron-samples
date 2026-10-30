//! yyjson 0.13.0 — Rust implementation of the C library's public API.
//!
//! The crate is `no_std`: everything it needs from the C runtime (the
//! `malloc` family, `mem*`, and the stdio calls used by the file APIs) is
//! declared directly, so the shared library keeps exactly the same dependency
//! on `libc.so` the C build had and exports nothing but the yyjson symbols.
//!
//! Module map, mirroring the "MARK" sections of the original `src/yyjson.c`:
//!
//! | module      | C section                                               |
//! |-------------|---------------------------------------------------------|
//! | `types`     | header structs, tags, flags                             |
//! | `tables`    | the generated lookup tables                             |
//! | `pow10`     | the `pow10_sig_table`                                   |
//! | `util`      | character / size / bit / UTF-8 helpers                  |
//! | `mem`       | libc memory primitives                                  |
//! | `alc`       | null, default, pool and dynamic allocators              |
//! | `doc`       | JSON struct utils (copies, equality, pools)             |
//! | `read`      | JSON reader, number reader, string reader               |
//! | `incr`      | incremental JSON reader                                 |
//! | `write`     | JSON writer, number writer, string writer               |
//! | `ptr`       | JSON Pointer, JSON Patch, JSON Merge Patch              |
//! | `file`      | FILE* helpers shared by reader and writer               |

#![no_std]
// The C names are kept verbatim so that every item can be matched against the
// original source; `dead_code` is allowed because a few constants and helpers
// exist only to mirror the C file's set of definitions.
#![allow(dead_code)]
// The `incr_len!` macro of the writers updates the whole cursor set; its last
// use in the `DocEnd` state does not read some of them back.
#![allow(unused_assignments)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_unsafe)]

mod alc;
mod doc;
mod file;
mod incr;
mod mem;
mod pow10;
mod ptr;
mod read;
mod tables;
mod types;
mod util;
mod write;

/// The library is compiled with `panic = "abort"` (see `rust/Cargo.toml`), so
/// a panic can never unwind across the `extern "C"` boundary into a C caller;
/// it terminates the process through `abort()` instead.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { abort() }
}

extern "C" {
    fn abort() -> !;
}

/// `core` keeps a reference to the unwinder personality routine even when the
/// crate is built with `panic = "abort"`, so a definition has to exist for the
/// static archive to link. It is never called: nothing in this crate unwinds.
/// rustc puts every `#[no_mangle]` item into the export list it generates for
/// a `cdylib`, so the shared library is built with `--no-default-features`
/// (see `rust/Cargo.toml` and the `cargo` invocation in `CMakeLists.txt`) and
/// does not contain this definition at all; it does not need it, because the
/// reference is dropped by `--gc-sections` when the library is linked.
#[cfg(feature = "eh-personality")]
#[no_mangle]
extern "C" fn rust_eh_personality() {}

#[no_mangle]
pub extern "C" fn yyjson_version() -> u32 {
    /* YYJSON_VERSION_HEX */
    0x000D00
}
