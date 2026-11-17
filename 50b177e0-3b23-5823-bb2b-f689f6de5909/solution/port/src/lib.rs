// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 Faithful Rust port of yyjson 0.13.0.

 Source of truth: ../state-a/src/yyjson.c and ../state-a/src/yyjson.h.
 Conventions: ../port-spec/CONVENTIONS.md (binding).

 Target configuration (the DEFAULT build of State A):
   - 64-bit little-endian Linux, LP64, GCC -O3
   - all YYJSON_DISABLE_* are 0
   - YYJSON_READER_DEPTH_LIMIT == 0, YYJSON_WRITER_DEPTH_LIMIT == 0
   - fast floating-point conversion ON (!YYJSON_DISABLE_FAST_FP_CONV)
   - unaligned memory access allowed (YYJSON_DISABLE_UNALIGNED_MEMORY_ACCESS 0)
   - YYJSON_HAS_INT128 1, YYJSON_HAS_IEEE_754 1, YYJSON_DOUBLE_MATH_CORRECT 1
 Only the active branch of every `#if` is ported.
 *============================================================================*/

#![no_std]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

mod ffi;
mod types;
mod hdr;
mod util;
mod alloc;
mod value;
mod reader;
mod incr;
mod read_num;
mod write_num;
mod writer;
mod pointer;
mod patch;

/// The C library has no unwinding and no panic runtime; `panic = "abort"` is
/// set in both profiles, and no code path in the port is allowed to panic.
/// Should one ever be reached, abort the way a C assertion failure would.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { crate::ffi::abort() }
}
