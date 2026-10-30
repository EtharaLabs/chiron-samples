// Gives the shared library the same SONAME the C build produced.
fn main() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libyyjson.so.0");
    // Record a DT_NEEDED on libc, matching the C build.
    println!("cargo:rustc-link-lib=dylib=c");
    println!("cargo:rerun-if-changed=build.rs");
}
