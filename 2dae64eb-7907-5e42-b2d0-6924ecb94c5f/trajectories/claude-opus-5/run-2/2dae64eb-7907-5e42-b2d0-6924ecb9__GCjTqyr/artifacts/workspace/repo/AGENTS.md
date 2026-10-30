# Repository notes

yyjson's implementation is **Rust**, in `rust/` (crate `yyjson`, `#![no_std]`,
no dependencies). `src/yyjson.h` is the unchanged C header and is the ABI
contract: its `static inline` functions are compiled into user code and read
`yyjson_val` / `yyjson_mut_val` / `yyjson_doc` / `yyjson_mut_doc` fields
directly, so the `#[repr(C)]` types in `rust/src/types.rs` must not change
shape. There is no C, C++, Objective-C or assembly source in the tree, and the
build must never enable a compiled language — `CMakeLists.txt` uses
`project(... LANGUAGES NONE)` and drives cargo through `add_custom_command`.

Build and install, exactly as before:

```
cmake -S . -B build -DBUILD_SHARED_LIBS=ON -DCMAKE_INSTALL_PREFIX=<prefix>
cmake --build build
cmake --install build
```

(and the same with `-DBUILD_SHARED_LIBS=OFF`). The build must keep working
with no compiler on `PATH`; check with `PATH=/opt/graded/bin:$PATH`.

Invariants worth re-checking after any change:

* the shared object's SONAME is `libyyjson.so.0` (set in `rust/build.rs`),
  it exports exactly 52 symbols and needs only `libc.so.6`;
* the installed tree is `include/yyjson.h`, `lib/libyyjson.so{,.0,.0.13.0}`
  or `lib/libyyjson.a`, `lib/cmake/yyjson/yyjson-config{,-release}.cmake`,
  `lib/pkgconfig/yyjson.pc`;
* behaviour is byte-for-byte that of the original C, including its quirks.

`doc/rust-port.md` is the design note: layout, allocator contract, unsafe
inventory, preserved quirks, number formatting, the incremental reader, the
build, and how the port was verified against the C. Read it before changing
anything subtle. `cargo test` in `rust/` runs the in-tree checks.
