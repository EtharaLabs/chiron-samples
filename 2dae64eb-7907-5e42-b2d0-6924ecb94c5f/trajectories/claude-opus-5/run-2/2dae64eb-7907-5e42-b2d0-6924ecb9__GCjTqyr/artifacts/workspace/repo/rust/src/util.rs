//! Byte, character, size and number helpers.
//!
//! Direct transliterations of the "Load/Store Utils", "Character Utils",
//! "Hex Character Reader", "UTF8 Validation", "Number and Bit Utils" and
//! "Size Utils" sections of the original `src/yyjson.c`.

use crate::tables::*;

/* ===== error message strings (MSG_* in yyjson.c) ======================== */

macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const core::ffi::c_char
    };
}
pub(crate) use cstr;

pub const MSG_FOPEN: *const core::ffi::c_char = c"failed to open file".as_ptr();
pub const MSG_FREAD: *const core::ffi::c_char = c"failed to read file".as_ptr();
pub const MSG_FWRITE: *const core::ffi::c_char = c"failed to write file".as_ptr();
pub const MSG_FCLOSE: *const core::ffi::c_char = c"failed to close file".as_ptr();
pub const MSG_MALLOC: *const core::ffi::c_char = c"failed to allocate memory".as_ptr();
pub const MSG_CHAR_T: *const core::ffi::c_char = c"invalid literal, expected 'true'".as_ptr();
pub const MSG_CHAR_F: *const core::ffi::c_char = c"invalid literal, expected 'false'".as_ptr();
pub const MSG_CHAR_N: *const core::ffi::c_char = c"invalid literal, expected 'null'".as_ptr();
pub const MSG_CHAR: *const core::ffi::c_char = c"unexpected character, expected a JSON value".as_ptr();
pub const MSG_ARR_END: *const core::ffi::c_char = c"unexpected character, expected ',' or ']'".as_ptr();
pub const MSG_OBJ_KEY: *const core::ffi::c_char = c"unexpected character, expected a string key".as_ptr();
pub const MSG_OBJ_SEP: *const core::ffi::c_char = c"unexpected character, expected ':' after key".as_ptr();
pub const MSG_OBJ_END: *const core::ffi::c_char = c"unexpected character, expected ',' or '}'".as_ptr();
pub const MSG_GARBAGE: *const core::ffi::c_char = c"unexpected content after document".as_ptr();
pub const MSG_NOT_END: *const core::ffi::c_char = c"unexpected end of data".as_ptr();
pub const MSG_COMMENT: *const core::ffi::c_char = c"unclosed multiline comment".as_ptr();
pub const MSG_COMMA: *const core::ffi::c_char = c"trailing comma is not allowed".as_ptr();
pub const MSG_NAN_INF: *const core::ffi::c_char = c"nan or inf number is not allowed".as_ptr();
pub const MSG_ERR_TYPE: *const core::ffi::c_char = c"invalid JSON value type".as_ptr();
pub const MSG_ERR_BOM: *const core::ffi::c_char = c"UTF-8 byte order mark (BOM) is not supported".as_ptr();
pub const MSG_ERR_UTF8: *const core::ffi::c_char = c"invalid utf-8 encoding in string".as_ptr();
pub const MSG_ERR_UTF16: *const core::ffi::c_char = c"UTF-16 encoding is not supported".as_ptr();
pub const MSG_ERR_UTF32: *const core::ffi::c_char = c"UTF-32 encoding is not supported".as_ptr();
pub const MSG_DEPTH: *const core::ffi::c_char = c"depth limit exceeded".as_ptr();

/* ===== numeric constants ================================================ */

pub const U64_MAX: u64 = u64::MAX;
pub const USIZE_MAX: usize = usize::MAX;
pub const U64_SAFE_DIG: i32 = 19;
pub const U32_SAFE_DIG: i32 = 9;
pub const USIZE_SAFE_DIG: usize = if core::mem::size_of::<usize>() == 8 {
    U64_SAFE_DIG as usize
} else {
    U32_SAFE_DIG as usize
};

pub const F64_BITS_INF: u64 = 0x7FF0000000000000;
pub const F64_BITS_NAN: u64 = 0x7FF8000000000000;

pub const F64_MAX_DEC_DIG: i32 = 768;
pub const F64_MAX_DEC_EXP: i32 = 308;
pub const F64_MIN_DEC_EXP: i32 = -324;
pub const F64_MAX_BIN_EXP: i32 = 1024;
pub const F64_MIN_BIN_EXP: i32 = -1021;

pub const F32_BITS: i32 = 32;
pub const F64_BITS: i32 = 64;
pub const F32_EXP_BITS: u32 = 8;
pub const F64_EXP_BITS: u32 = 11;
pub const F32_SIG_BITS: i32 = 23;
pub const F64_SIG_BITS: i32 = 52;
pub const F32_SIG_FULL_BITS: i32 = 24;
pub const F64_SIG_FULL_BITS: i32 = 53;
pub const F32_SIG_MASK: u32 = 0x007FFFFF;
pub const F64_SIG_MASK: u64 = 0x000FFFFFFFFFFFFF;
pub const F32_EXP_MASK: u32 = 0x7F800000;
pub const F64_EXP_MASK: u64 = 0x7FF0000000000000;
pub const F32_EXP_BIAS: i32 = 127;
pub const F64_EXP_BIAS: i32 = 1023;

pub const FP_BUF_LEN: usize = 40;
pub const INCR_NUM_MAX_LEN: usize = 1024;

pub const F64_POW10_MAX_EXACT_EXP: i32 = 22;
pub const U64_POW10_MAX_EXACT_EXP: i32 = 19;

pub const YYJSON_READER_ESTIMATED_PRETTY_RATIO: usize = 16;
pub const YYJSON_READER_ESTIMATED_MINIFY_RATIO: usize = 6;
pub const YYJSON_WRITER_ESTIMATED_PRETTY_RATIO: usize = 32;
pub const YYJSON_WRITER_ESTIMATED_MINIFY_RATIO: usize = 18;

pub const YYJSON_MUT_DOC_STR_POOL_INIT_SIZE: usize = 0x100;
pub const YYJSON_MUT_DOC_STR_POOL_MAX_SIZE: usize = 0x1000_0000;
pub const YYJSON_MUT_DOC_VAL_POOL_INIT_SIZE: usize =
    0x10 * core::mem::size_of::<crate::types::YyjsonMutVal>();
pub const YYJSON_MUT_DOC_VAL_POOL_MAX_SIZE: usize =
    0x100_0000 * core::mem::size_of::<crate::types::YyjsonMutVal>();

pub const YYJSON_ALC_DYN_MIN_SIZE: usize = 0x1000;

/* ===== load / store ===================================================== */

#[inline(always)]
pub unsafe fn byte_copy_2(dst: *mut u8, src: *const u8) {
    core::ptr::copy_nonoverlapping(src, dst, 2);
}
#[inline(always)]
pub unsafe fn byte_copy_4(dst: *mut u8, src: *const u8) {
    core::ptr::copy_nonoverlapping(src, dst, 4);
}
#[inline(always)]
pub unsafe fn byte_copy_8(dst: *mut u8, src: *const u8) {
    core::ptr::copy_nonoverlapping(src, dst, 8);
}
#[inline(always)]
pub unsafe fn byte_copy_16(dst: *mut u8, src: *const u8) {
    core::ptr::copy_nonoverlapping(src, dst, 16);
}
#[inline(always)]
pub unsafe fn byte_move_2(dst: *mut u8, src: *const u8) {
    let tmp = (src as *const [u8; 2]).read_unaligned();
    (dst as *mut [u8; 2]).write_unaligned(tmp);
}
#[inline(always)]
pub unsafe fn byte_move_4(dst: *mut u8, src: *const u8) {
    let tmp = (src as *const [u8; 4]).read_unaligned();
    (dst as *mut [u8; 4]).write_unaligned(tmp);
}
#[inline(always)]
pub unsafe fn byte_move_8(dst: *mut u8, src: *const u8) {
    let tmp = (src as *const [u8; 8]).read_unaligned();
    (dst as *mut [u8; 8]).write_unaligned(tmp);
}
#[inline(always)]
pub unsafe fn byte_move_16(dst: *mut u8, src: *const u8) {
    let t1 = (src as *const [u8; 8]).read_unaligned();
    let t2 = (src.add(8) as *const [u8; 8]).read_unaligned();
    (dst as *mut [u8; 8]).write_unaligned(t1);
    (dst.add(8) as *mut [u8; 8]).write_unaligned(t2);
}

/// `byte_move_forward`: only valid when `dst <= src` and `n <= 16`; rounds the
/// length up to an even number, so it may copy one byte more than asked. The
/// callers rely on that (the extra byte is always re-copied afterwards).
#[inline(always)]
pub unsafe fn byte_move_forward(dst: *mut u8, src: *const u8, n: usize) {
    let mut n = n + (n % 2);
    let mut d = dst;
    let mut s = src;
    if n == 16 {
        byte_move_16(d, s);
        return;
    }
    if n >= 8 {
        byte_move_8(d, s);
        n -= 8;
        d = d.add(8);
        s = s.add(8);
    }
    if n >= 4 {
        byte_move_4(d, s);
        n -= 4;
        d = d.add(4);
        s = s.add(4);
    }
    if n >= 2 {
        byte_move_2(d, s);
    }
}

#[inline(always)]
pub unsafe fn byte_match_2(buf: *const u8, pat: &[u8; 2]) -> bool {
    (buf as *const [u8; 2]).read_unaligned() == *pat
}
#[inline(always)]
pub unsafe fn byte_match_4(buf: *const u8, pat: &[u8; 4]) -> bool {
    (buf as *const [u8; 4]).read_unaligned() == *pat
}

#[inline(always)]
pub unsafe fn byte_load_2(src: *const u8) -> u16 {
    u16::from_ne_bytes((src as *const [u8; 2]).read_unaligned())
}
#[inline(always)]
pub unsafe fn byte_load_3(src: *const u8) -> u32 {
    u32::from_ne_bytes([*src, *src.add(1), *src.add(2), 0])
}
#[inline(always)]
pub unsafe fn byte_load_4(src: *const u8) -> u32 {
    u32::from_ne_bytes((src as *const [u8; 4]).read_unaligned())
}

/* ===== character classification ========================================= */

pub const CHAR_TYPE_ASCII: u8 = 1 << 0;
pub const CHAR_TYPE_ASCII_SQ: u8 = 1 << 1;
pub const CHAR_TYPE_SPACE: u8 = 1 << 2;
pub const CHAR_TYPE_SPACE_EXT: u8 = 1 << 3;
pub const CHAR_TYPE_NUM: u8 = 1 << 4;
pub const CHAR_TYPE_COMMENT: u8 = 1 << 5;

pub const CHAR_TYPE_EOL: u8 = 1 << 0;
pub const CHAR_TYPE_EOL_EXT: u8 = 1 << 1;
pub const CHAR_TYPE_ID_START: u8 = 1 << 2;
pub const CHAR_TYPE_ID_NEXT: u8 = 1 << 3;
pub const CHAR_TYPE_ID_ASCII: u8 = 1 << 4;

pub const CHAR_TYPE_SIGN: u8 = 1 << 0;
pub const CHAR_TYPE_DIGIT: u8 = 1 << 1;
pub const CHAR_TYPE_NONZERO: u8 = 1 << 2;
pub const CHAR_TYPE_EXP: u8 = 1 << 3;
pub const CHAR_TYPE_DOT: u8 = 1 << 4;

#[inline(always)]
pub fn char_is_space(c: u8) -> bool {
    CHAR_TABLE1[c as usize] & CHAR_TYPE_SPACE != 0
}
#[inline(always)]
pub fn char_is_space_ext(c: u8) -> bool {
    CHAR_TABLE1[c as usize] & CHAR_TYPE_SPACE_EXT != 0
}
#[inline(always)]
pub fn char_is_num(c: u8) -> bool {
    CHAR_TABLE1[c as usize] & CHAR_TYPE_NUM != 0
}
#[inline(always)]
pub fn char_is_ascii_skip(c: u8) -> bool {
    CHAR_TABLE1[c as usize] & CHAR_TYPE_ASCII != 0
}
#[inline(always)]
pub fn char_is_ascii_skip_sq(c: u8) -> bool {
    CHAR_TABLE1[c as usize] & CHAR_TYPE_ASCII_SQ != 0
}
#[inline(always)]
pub fn char_is_trivia(c: u8) -> bool {
    CHAR_TABLE1[c as usize] & (CHAR_TYPE_SPACE_EXT | CHAR_TYPE_COMMENT) != 0
}
#[inline(always)]
pub fn char_is_eol(c: u8) -> bool {
    CHAR_TABLE2[c as usize] & CHAR_TYPE_EOL != 0
}
#[inline(always)]
pub fn char_is_eol_ext(c: u8) -> bool {
    CHAR_TABLE2[c as usize] & CHAR_TYPE_EOL_EXT != 0
}
#[inline(always)]
pub fn char_is_id_start(c: u8) -> bool {
    CHAR_TABLE2[c as usize] & CHAR_TYPE_ID_START != 0
}
#[inline(always)]
pub fn char_is_id_next(c: u8) -> bool {
    CHAR_TABLE2[c as usize] & CHAR_TYPE_ID_NEXT != 0
}
#[inline(always)]
pub fn char_is_id_ascii(c: u8) -> bool {
    CHAR_TABLE2[c as usize] & CHAR_TYPE_ID_ASCII != 0
}
#[inline(always)]
pub fn char_is_sign(d: u8) -> bool {
    CHAR_TABLE3[d as usize] & CHAR_TYPE_SIGN != 0
}
#[inline(always)]
pub fn char_is_nonzero(d: u8) -> bool {
    CHAR_TABLE3[d as usize] & CHAR_TYPE_NONZERO != 0
}
#[inline(always)]
pub fn char_is_digit(d: u8) -> bool {
    CHAR_TABLE3[d as usize] & CHAR_TYPE_DIGIT != 0
}
#[inline(always)]
pub fn char_is_exp(d: u8) -> bool {
    CHAR_TABLE3[d as usize] & CHAR_TYPE_EXP != 0
}
#[inline(always)]
pub fn char_is_fp(d: u8) -> bool {
    CHAR_TABLE3[d as usize] & (CHAR_TYPE_DOT | CHAR_TYPE_EXP) != 0
}
#[inline(always)]
pub fn char_is_digit_or_fp(d: u8) -> bool {
    CHAR_TABLE3[d as usize] & (CHAR_TYPE_DIGIT | CHAR_TYPE_DOT | CHAR_TYPE_EXP) != 0
}
#[inline(always)]
pub fn char_is_ctn(c: u8) -> bool {
    (c & 0xDF) == 0x5B
}
#[inline(always)]
pub fn char_to_lower(c: u8) -> u8 {
    c | 0x20
}

#[inline(always)]
pub unsafe fn is_utf8_bom(cur: *const u8) -> bool {
    byte_load_3(cur) == byte_load_3(b"\xEF\xBB\xBF".as_ptr())
}
#[inline(always)]
pub unsafe fn is_utf16_bom(cur: *const u8) -> bool {
    byte_load_2(cur) == byte_load_2(b"\xFE\xFF".as_ptr())
        || byte_load_2(cur) == byte_load_2(b"\xFF\xFE".as_ptr())
}
#[inline(always)]
pub unsafe fn is_utf32_bom(cur: *const u8) -> bool {
    byte_load_4(cur) == byte_load_4(b"\x00\x00\xFE\xFF".as_ptr())
        || byte_load_4(cur) == byte_load_4(b"\xFF\xFE\x00\x00".as_ptr())
}

/// `ext_eol_len`: length of a JSON5 line terminator starting at `cur`.
#[inline(always)]
pub unsafe fn ext_eol_len(cur: *const u8) -> usize {
    if *cur < 0x80 {
        return 1;
    }
    if *cur.add(1) == 0x80 && (*cur.add(2) == 0xA8 || *cur.add(2) == 0xA9) {
        return 3;
    }
    0
}

/// `ext_space_len`: length of a JSON5 whitespace character starting at `cur`.
#[inline(always)]
pub unsafe fn ext_space_len(cur: *const u8) -> usize {
    if *cur < 0x80 {
        return 1;
    } else if byte_load_2(cur) == byte_load_2(b"\xC2\xA0".as_ptr()) {
        return 2;
    } else if byte_load_2(cur) == byte_load_2(b"\xE2\x80".as_ptr()) {
        let c2 = *cur.add(2);
        if (0x80..=0x8A).contains(&c2) {
            return 3;
        }
        if c2 == 0xA8 || c2 == 0xA9 || c2 == 0xAF {
            return 3;
        }
    } else {
        let uni = byte_load_3(cur);
        if uni == byte_load_3(b"\xE1\x9A\x80".as_ptr())
            || uni == byte_load_3(b"\xE2\x81\x9F".as_ptr())
            || uni == byte_load_3(b"\xE3\x80\x80".as_ptr())
            || uni == byte_load_3(b"\xEF\xBB\xBF".as_ptr())
        {
            return 3;
        }
    }
    0
}

/* ===== hex ============================================================== */

#[inline(always)]
pub unsafe fn hex_load_4(src: *const u8, dst: *mut u16) -> bool {
    let c0 = HEX_CONV_TABLE[*src as usize] as u16;
    let c1 = HEX_CONV_TABLE[*src.add(1) as usize] as u16;
    let c2 = HEX_CONV_TABLE[*src.add(2) as usize] as u16;
    let c3 = HEX_CONV_TABLE[*src.add(3) as usize] as u16;
    let t0 = (c0 << 8) | c2;
    let t1 = (c1 << 8) | c3;
    *dst = (t0 << 4) | t1;
    ((t0 | t1) & 0xF0F0) == 0
}

#[inline(always)]
pub unsafe fn hex_load_2(src: *const u8, dst: *mut u8) -> bool {
    let c0 = HEX_CONV_TABLE[*src as usize];
    let c1 = HEX_CONV_TABLE[*src.add(1) as usize];
    *dst = (c0 << 4) | c1;
    ((c0 | c1) & 0xF0) == 0
}

#[inline(always)]
pub fn char_is_hex(c: u8) -> bool {
    HEX_CONV_TABLE[c as usize] != 0xF0
}

/* ===== UTF-8 validation ================================================= */

/// Four bytes in memory order packed into a native-endian `u32`, matching the
/// `utf8_seq_def` macro.
const fn useq(a: u8, b: u8, c: u8, d: u8) -> u32 {
    u32::from_ne_bytes([a, b, c, d])
}

pub const UTF8_B1_MASK: u32 = useq(0x80, 0x00, 0x00, 0x00);
pub const UTF8_B1_PATT: u32 = useq(0x00, 0x00, 0x00, 0x00);
pub const UTF8_B2_MASK: u32 = useq(0xE0, 0xC0, 0x00, 0x00);
pub const UTF8_B2_PATT: u32 = useq(0xC0, 0x80, 0x00, 0x00);
pub const UTF8_B2_REQU: u32 = useq(0x1E, 0x00, 0x00, 0x00);
pub const UTF8_B3_MASK: u32 = useq(0xF0, 0xC0, 0xC0, 0x00);
pub const UTF8_B3_PATT: u32 = useq(0xE0, 0x80, 0x80, 0x00);
pub const UTF8_B3_REQU: u32 = useq(0x0F, 0x20, 0x00, 0x00);
pub const UTF8_B3_ERRO: u32 = useq(0x0D, 0x20, 0x00, 0x00);
pub const UTF8_B4_MASK: u32 = useq(0xF8, 0xC0, 0xC0, 0xC0);
pub const UTF8_B4_PATT: u32 = useq(0xF0, 0x80, 0x80, 0x80);
pub const UTF8_B4_REQU: u32 = useq(0x07, 0x30, 0x00, 0x00);
pub const UTF8_B4_REQ1: u32 = useq(0x04, 0x00, 0x00, 0x00);
pub const UTF8_B4_REQ2: u32 = useq(0x03, 0x30, 0x00, 0x00);

#[inline(always)]
pub fn is_utf8_seq1(uni: u32) -> bool {
    (uni & UTF8_B1_MASK) == UTF8_B1_PATT
}
#[inline(always)]
pub fn is_utf8_seq2(uni: u32) -> bool {
    (uni & UTF8_B2_MASK) == UTF8_B2_PATT && (uni & UTF8_B2_REQU) != 0
}
#[inline(always)]
pub fn is_utf8_seq3(uni: u32) -> bool {
    if (uni & UTF8_B3_MASK) != UTF8_B3_PATT {
        return false;
    }
    let tmp = uni & UTF8_B3_REQU;
    tmp != 0 && tmp != UTF8_B3_ERRO
}
#[inline(always)]
pub fn is_utf8_seq4(uni: u32) -> bool {
    if (uni & UTF8_B4_MASK) != UTF8_B4_PATT {
        return false;
    }
    let tmp = uni & UTF8_B4_REQU;
    tmp != 0 && ((tmp & UTF8_B4_REQ1) == 0 || (tmp & UTF8_B4_REQ2) == 0)
}

/* ===== number and bit utils ============================================= */

#[inline(always)]
pub fn f64_from_bits(u: u64) -> f64 {
    f64::from_bits(u)
}
#[inline(always)]
pub fn f64_to_bits(f: f64) -> u64 {
    f.to_bits()
}
#[inline(always)]
pub fn f32_to_bits(f: f32) -> u32 {
    f.to_bits()
}
#[inline(always)]
pub fn f64_bits_inf(sign: bool) -> u64 {
    F64_BITS_INF | ((sign as u64) << 63)
}
#[inline(always)]
pub fn f64_bits_nan(sign: bool) -> u64 {
    F64_BITS_NAN | ((sign as u64) << 63)
}
#[inline(always)]
pub fn f64_is_inf(val: f64) -> bool {
    (f64_to_bits(val) & F64_EXP_MASK) == F64_BITS_INF
}
/// Casting double to float, allowing overflow to infinity (as the C does).
#[inline(always)]
pub fn f64_to_f32(val: f64) -> f32 {
    val as f32
}
#[inline(always)]
pub fn u64_lz_bits(v: u64) -> u32 {
    v.leading_zeros()
}
#[inline(always)]
pub fn u64_tz_bits(v: u64) -> u32 {
    v.trailing_zeros()
}
#[inline(always)]
pub fn u128_mul(a: u64, b: u64) -> (u64, u64) {
    let m = (a as u128) * (b as u128);
    ((m >> 64) as u64, m as u64)
}
#[inline(always)]
pub fn u128_mul_add(a: u64, b: u64, c: u64) -> (u64, u64) {
    let m = (a as u128) * (b as u128) + (c as u128);
    ((m >> 64) as u64, m as u64)
}

/* ===== size utils ======================================================= */

#[inline(always)]
pub fn size_add_is_overflow(size: usize, add: usize) -> bool {
    size > size.wrapping_add(add)
}
#[inline(always)]
pub fn size_is_pow2(size: usize) -> bool {
    (size & size.wrapping_sub(1)) == 0
}
#[inline(always)]
pub fn size_align_up(size: usize, align: usize) -> usize {
    if size_is_pow2(align) {
        size.wrapping_add(align - 1) & !(align - 1)
    } else {
        size.wrapping_add(align)
            .wrapping_sub(size.wrapping_add(align).wrapping_sub(1) % align)
            .wrapping_sub(1)
    }
}
#[inline(always)]
pub fn size_align_down(size: usize, align: usize) -> usize {
    if size_is_pow2(align) {
        size & !(align - 1)
    } else {
        size - (size % align)
    }
}
#[inline(always)]
pub fn mem_align_up(mem: *mut u8, align: usize) -> *mut u8 {
    let size = size_align_up(mem as usize, align);
    // Matches the C, which round-trips the pointer through a `usize` with
    // `memcpy`; provenance is carried over explicitly here.
    mem.wrapping_add(size.wrapping_sub(mem as usize))
}

/* ===== pow10 ============================================================ */

#[inline(always)]
pub fn pow10_table_get_sig(exp10: i32) -> (u64, u64) {
    let idx = (exp10 - crate::pow10::POW10_SIG_TABLE_MIN_EXP) as usize;
    (
        crate::pow10::POW10_SIG_TABLE[idx * 2],
        crate::pow10::POW10_SIG_TABLE[idx * 2 + 1],
    )
}

#[inline(always)]
pub fn pow10_table_get_exp(exp10: i32) -> i32 {
    (exp10.wrapping_mul(217706).wrapping_sub(4128768)) >> 16
}
