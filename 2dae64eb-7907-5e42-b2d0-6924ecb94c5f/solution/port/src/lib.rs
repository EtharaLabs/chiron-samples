// FORGE-CANARY-BEGIN
// FORGE-CANARY-SLOT-1: 60ae0f0c079c366733615a2e6551dc4e64c530f1c56ffed9db295862b101ddc8
// FORGE-CANARY-SLOT-2: 87ab65e4ae3bd13b9cfb24e8b811c4ab101c8552a8b3d1f0d91c32bbd429958a
// FORGE-CANARY-SLOT-3: 76135f235d13f30c4446a51ada3f9a19b6fba2e250fa6b2ffc5a14d1b3f55ffb
// FORGE-CANARY-SLOT-4: e415e80ce167e9922758ef5d43162679b0fe51046883994e0fc30102c1e183f5
// FORGE-CANARY-END
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
