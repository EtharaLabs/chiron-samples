//! Build script: the shared-library link options.
//!
//! `rustc-link-arg-cdylib` only affects the `cdylib` artifact, so the static
//! archive is untouched by any of this.
//!
//! * `-soname libyyjson.so.0` reproduces the SONAME the CMake C build gave the
//!   library (`set_target_properties(yyjson PROPERTIES SOVERSION 0)`).
//! * `-lc` reproduces the `DT_NEEDED libc.so.6` entry of the C build. rustc
//!   links a `no_std` cdylib with `-nodefaultlibs`, so without this the
//!   library would reference `malloc`, `memcpy`, `fopen`, ... without
//!   recording a dependency on the library that provides them.
//!
//! The exported symbol set needs no extra flag: rustc emits its own
//! (anonymous) linker version script for a `cdylib` which marks every symbol
//! local except the `#[no_mangle]` items of the crate, and those are exactly
//! the 52 symbols the C library exported. A second version script cannot be
//! added on top of it ("anonymous version tag cannot be combined with other
//! version tags"), which is why `rust_eh_personality` sits behind the
//! `eh-personality` feature instead: the shared library is built with
//! `--no-default-features` so that definition is absent there, while the
//! static archive (built with the feature on) defines it for C programs that
//! link with a bare `-lyyjson`.

fn main() {
    println!("cargo:rustc-link-arg-cdylib=-Wl,-soname,libyyjson.so.0");
    println!("cargo:rustc-link-arg-cdylib=-lc");
}
