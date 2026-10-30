//! Private helpers: character classification, size arithmetic, bit tricks and
//! the UTF-8 sequence validation masks. One-to-one with the "Character Utils",
//! "Size Utils", "Number and Bit Utils" and "UTF8 Validation" sections of the
//! original `src/yyjson.c`.

use crate::tables::{CHAR_TABLE1, CHAR_TABLE2, CHAR_TABLE3, HEX_CONV_TABLE};

/* char_table1 */
pub const CHAR_TYPE_ASCII: u8 = 1 << 0;
pub const CHAR_TYPE_ASCII_SQ: u8 = 1 << 1;
pub const CHAR_TYPE_SPACE: u8 = 1 << 2;
pub const CHAR_TYPE_SPACE_EXT: u8 = 1 << 3;
pub const CHAR_TYPE_NUM: u8 = 1 << 4;
pub const CHAR_TYPE_COMMENT: u8 = 1 << 5;

/* char_table2 */
pub const CHAR_TYPE_EOL: u8 = 1 << 0;
pub const CHAR_TYPE_EOL_EXT: u8 = 1 << 1;
pub const CHAR_TYPE_ID_START: u8 = 1 << 2;
pub const CHAR_TYPE_ID_NEXT: u8 = 1 << 3;
pub const CHAR_TYPE_ID_ASCII: u8 = 1 << 4;

/* char_table3 */
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
pub fn char_is_sign(c: u8) -> bool {
    CHAR_TABLE3[c as usize] & CHAR_TYPE_SIGN != 0
}
#[inline(always)]
pub fn char_is_nonzero(c: u8) -> bool {
    CHAR_TABLE3[c as usize] & CHAR_TYPE_NONZERO != 0
}
#[inline(always)]
pub fn char_is_digit(c: u8) -> bool {
    CHAR_TABLE3[c as usize] & CHAR_TYPE_DIGIT != 0
}
#[inline(always)]
pub fn char_is_exp(c: u8) -> bool {
    CHAR_TABLE3[c as usize] & CHAR_TYPE_EXP != 0
}
#[inline(always)]
pub fn char_is_fp(c: u8) -> bool {
    CHAR_TABLE3[c as usize] & (CHAR_TYPE_DOT | CHAR_TYPE_EXP) != 0
}
#[inline(always)]
pub fn char_is_digit_or_fp(c: u8) -> bool {
    CHAR_TABLE3[c as usize] & (CHAR_TYPE_DIGIT | CHAR_TYPE_DOT | CHAR_TYPE_EXP) != 0
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
pub fn char_is_hex(c: u8) -> bool {
    HEX_CONV_TABLE[c as usize] != 0xF0
}

/// `hex_load_4`: four hex characters to a `u16`, true on valid input.
#[inline(always)]
pub unsafe fn hex_load_4(src: *const u8, dst: &mut u16) -> bool {
    let c0 = HEX_CONV_TABLE[*src as usize] as u16;
    let c1 = HEX_CONV_TABLE[*src.add(1) as usize] as u16;
    let c2 = HEX_CONV_TABLE[*src.add(2) as usize] as u16;
    let c3 = HEX_CONV_TABLE[*src.add(3) as usize] as u16;
    let t0 = (c0 << 8) | c2;
    let t1 = (c1 << 8) | c3;
    *dst = (t0 << 4) | t1;
    ((t0 | t1) & 0xF0F0) == 0
}

/// `hex_load_2`: two hex characters to a `u8`, true on valid input.
#[inline(always)]
pub unsafe fn hex_load_2(src: *const u8, dst: &mut u8) -> bool {
    let c0 = HEX_CONV_TABLE[*src as usize];
    let c1 = HEX_CONV_TABLE[*src.add(1) as usize];
    *dst = (c0 << 4) | c1;
    ((c0 | c1) & 0xF0) == 0
}

/* ------------------------------------------------------------ raw byte loads */

/// `byte_load_2`, as a little-endian `u16`.
#[inline(always)]
pub unsafe fn byte_load_2(src: *const u8) -> u16 {
    u16::from_le_bytes([*src, *src.add(1)])
}

/// `byte_load_3`, as a little-endian `u32` with a zero top byte.
#[inline(always)]
pub unsafe fn byte_load_3(src: *const u8) -> u32 {
    u32::from_le_bytes([*src, *src.add(1), *src.add(2), 0])
}

/// `byte_load_4`, as a little-endian `u32`.
#[inline(always)]
pub unsafe fn byte_load_4(src: *const u8) -> u32 {
    u32::from_le_bytes([*src, *src.add(1), *src.add(2), *src.add(3)])
}

/// `byte_move_forward`: `memmove` for `dst <= src` and `n <= 16`, rounding the
/// copied length up to an even number exactly as the C version does.
#[inline(always)]
pub unsafe fn byte_move_forward(dst: *mut u8, src: *const u8, n: usize) {
    let n = n + (n % 2);
    crate::mem::move_(dst, src, n);
}

/* -------------------------------------------------------------- UTF-8 masks */

/// Pack four bytes into a `u32` in the same order `byte_load_4` reads them.
#[inline(always)]
const fn seq(a: u32, b: u32, c: u32, d: u32) -> u32 {
    a | (b << 8) | (c << 16) | (d << 24)
}

const B1_MASK: u32 = seq(0x80, 0x00, 0x00, 0x00);
const B1_PATT: u32 = seq(0x00, 0x00, 0x00, 0x00);
const B2_MASK: u32 = seq(0xE0, 0xC0, 0x00, 0x00);
const B2_PATT: u32 = seq(0xC0, 0x80, 0x00, 0x00);
const B2_REQU: u32 = seq(0x1E, 0x00, 0x00, 0x00);
const B3_MASK: u32 = seq(0xF0, 0xC0, 0xC0, 0x00);
const B3_PATT: u32 = seq(0xE0, 0x80, 0x80, 0x00);
const B3_REQU: u32 = seq(0x0F, 0x20, 0x00, 0x00);
const B3_ERRO: u32 = seq(0x0D, 0x20, 0x00, 0x00);
const B4_MASK: u32 = seq(0xF8, 0xC0, 0xC0, 0xC0);
const B4_PATT: u32 = seq(0xF0, 0x80, 0x80, 0x80);
const B4_REQU: u32 = seq(0x07, 0x30, 0x00, 0x00);
const B4_REQ1: u32 = seq(0x04, 0x00, 0x00, 0x00);
const B4_REQ2: u32 = seq(0x03, 0x30, 0x00, 0x00);

#[inline(always)]
pub fn is_utf8_seq1(uni: u32) -> bool {
    (uni & B1_MASK) == B1_PATT
}
#[inline(always)]
pub fn is_utf8_seq2(uni: u32) -> bool {
    (uni & B2_MASK) == B2_PATT && (uni & B2_REQU) != 0
}
#[inline(always)]
pub fn is_utf8_seq3(uni: u32) -> bool {
    if (uni & B3_MASK) != B3_PATT {
        return false;
    }
    let tmp = uni & B3_REQU;
    tmp != 0 && tmp != B3_ERRO
}
#[inline(always)]
pub fn is_utf8_seq4(uni: u32) -> bool {
    if (uni & B4_MASK) != B4_PATT {
        return false;
    }
    let tmp = uni & B4_REQU;
    tmp != 0 && ((tmp & B4_REQ1) == 0 || (tmp & B4_REQ2) == 0)
}

#[inline(always)]
pub unsafe fn is_utf8_bom(cur: *const u8) -> bool {
    byte_load_3(cur) == seq(0xEF, 0xBB, 0xBF, 0x00)
}

#[inline(always)]
pub unsafe fn is_utf16_bom(cur: *const u8) -> bool {
    let u = byte_load_2(cur);
    u == u16::from_le_bytes([0xFE, 0xFF]) || u == u16::from_le_bytes([0xFF, 0xFE])
}

#[inline(always)]
pub unsafe fn is_utf32_bom(cur: *const u8) -> bool {
    let u = byte_load_4(cur);
    u == seq(0x00, 0x00, 0xFE, 0xFF) || u == seq(0xFF, 0xFE, 0x00, 0x00)
}

/// `ext_eol_len`: byte length of a JSON5 line terminator.
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

/// `ext_space_len`: byte length of a JSON5 whitespace character.
#[inline(always)]
pub unsafe fn ext_space_len(cur: *const u8) -> usize {
    if *cur < 0x80 {
        return 1;
    } else if byte_load_2(cur) == u16::from_le_bytes([0xC2, 0xA0]) {
        return 2;
    } else if byte_load_2(cur) == u16::from_le_bytes([0xE2, 0x80]) {
        let c2 = *cur.add(2);
        if c2 >= 0x80 && c2 <= 0x8A {
            return 3;
        }
        if c2 == 0xA8 || c2 == 0xA9 || c2 == 0xAF {
            return 3;
        }
    } else {
        let uni = byte_load_3(cur);
        if uni == seq(0xE1, 0x9A, 0x80, 0)
            || uni == seq(0xE2, 0x81, 0x9F, 0)
            || uni == seq(0xE3, 0x80, 0x80, 0)
            || uni == seq(0xEF, 0xBB, 0xBF, 0)
        {
            return 3;
        }
    }
    0
}

/* --------------------------------------------------------------- size utils */

pub const USIZE_MAX: usize = usize::MAX;
pub const U64_MAX: u64 = u64::MAX;
pub const I64_MAX: u64 = 0x7FFF_FFFF_FFFF_FFFF;
pub const U32_SAFE_DIG: u32 = 9;
pub const U64_SAFE_DIG: u32 = 19;
pub const USIZE_SAFE_DIG: u32 = if core::mem::size_of::<usize>() == 8 {
    U64_SAFE_DIG
} else {
    U32_SAFE_DIG
};

/// `size_add_is_overflow`.
#[inline(always)]
pub fn size_add_is_overflow(size: usize, add: usize) -> bool {
    size > size.wrapping_add(add)
}

/// `size_is_pow2` (size must not be 0).
#[inline(always)]
pub fn size_is_pow2(size: usize) -> bool {
    (size & size.wrapping_sub(1)) == 0
}

/// `size_align_up` (may overflow, wrapping as in C).
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

/// `size_align_down`.
#[inline(always)]
pub fn size_align_down(size: usize, align: usize) -> usize {
    if size_is_pow2(align) {
        size & !(align - 1)
    } else {
        size - (size % align)
    }
}

/// `mem_align_up`.
#[inline(always)]
pub fn mem_align_up(mem: *mut u8, align: usize) -> *mut u8 {
    size_align_up(mem as usize, align) as *mut u8
}

/* ---------------------------------------------------- number and bit utils */

pub const F64_BITS_INF: u64 = 0x7FF0_0000_0000_0000;
pub const F64_BITS_NAN: u64 = 0x7FF8_0000_0000_0000;
pub const F64_MAX_DEC_DIG: i32 = 768;
pub const F64_MAX_DEC_EXP: i32 = 308;
pub const F64_MIN_DEC_EXP: i32 = -324;
pub const F64_MAX_BIN_EXP: i32 = 1024;
pub const F64_MIN_BIN_EXP: i32 = -1021;
pub const F32_BITS: u32 = 32;
pub const F64_BITS: u32 = 64;
pub const F32_EXP_BITS: u32 = 8;
pub const F64_EXP_BITS: u32 = 11;
pub const F32_SIG_BITS: u32 = 23;
pub const F64_SIG_BITS: u32 = 52;
pub const F32_SIG_FULL_BITS: u32 = 24;
pub const F64_SIG_FULL_BITS: u32 = 53;
pub const F32_SIG_MASK: u32 = 0x007F_FFFF;
pub const F64_SIG_MASK: u64 = 0x000F_FFFF_FFFF_FFFF;
pub const F32_EXP_MASK: u32 = 0x7F80_0000;
pub const F64_EXP_MASK: u64 = 0x7FF0_0000_0000_0000;
pub const F32_EXP_BIAS: i32 = 127;
pub const F64_EXP_BIAS: i32 = 1023;
pub const F32_DEC_DIG: u32 = 9;
pub const F64_DEC_DIG: u32 = 17;
pub const FP_BUF_LEN: usize = 40;

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

/// `f64_to_f32`: narrowing cast, overflow allowed (saturates to infinity, the
/// same result the C cast produces on IEEE-754 hardware).
#[inline(always)]
pub fn f64_to_f32(val: f64) -> f32 {
    val as f32
}

/// `u64_lz_bits` (input must not be 0).
#[inline(always)]
pub fn u64_lz_bits(v: u64) -> u32 {
    v.leading_zeros()
}

/// `u64_tz_bits` (input must not be 0).
#[inline(always)]
pub fn u64_tz_bits(v: u64) -> u32 {
    v.trailing_zeros()
}

/// `u128_mul`: full 64x64 -> 128 bit product.
#[inline(always)]
pub fn u128_mul(a: u64, b: u64) -> (u64, u64) {
    let m = (a as u128) * (b as u128);
    ((m >> 64) as u64, m as u64)
}

/// `u128_mul_add`: `a * b + c` as a 128-bit result.
#[inline(always)]
pub fn u128_mul_add(a: u64, b: u64, c: u64) -> (u64, u64) {
    let m = (a as u128) * (b as u128) + (c as u128);
    ((m >> 64) as u64, m as u64)
}

/* --------------------------------------------------------------- estimates */

pub const YYJSON_READER_ESTIMATED_PRETTY_RATIO: usize = 16;
pub const YYJSON_READER_ESTIMATED_MINIFY_RATIO: usize = 6;
pub const YYJSON_WRITER_ESTIMATED_PRETTY_RATIO: usize = 32;
pub const YYJSON_WRITER_ESTIMATED_MINIFY_RATIO: usize = 18;

pub const YYJSON_MUT_DOC_STR_POOL_INIT_SIZE: usize = 0x100;
pub const YYJSON_MUT_DOC_STR_POOL_MAX_SIZE: usize = 0x1000_0000;
pub const YYJSON_MUT_DOC_VAL_POOL_INIT_SIZE: usize =
    0x10 * core::mem::size_of::<crate::types::yyjson_mut_val>();
pub const YYJSON_MUT_DOC_VAL_POOL_MAX_SIZE: usize =
    0x100_0000 * core::mem::size_of::<crate::types::yyjson_mut_val>();
pub const YYJSON_ALC_DYN_MIN_SIZE: usize = 0x1000;

pub const INCR_NUM_MAX_LEN: usize = 1024;

/* ---------------------------------------------------- common error messages */

pub const MSG_FOPEN: &[u8] = b"failed to open file\0";
pub const MSG_FREAD: &[u8] = b"failed to read file\0";
pub const MSG_FWRITE: &[u8] = b"failed to write file\0";
pub const MSG_FCLOSE: &[u8] = b"failed to close file\0";
pub const MSG_MALLOC: &[u8] = b"failed to allocate memory\0";
pub const MSG_CHAR_T: &[u8] = b"invalid literal, expected 'true'\0";
pub const MSG_CHAR_F: &[u8] = b"invalid literal, expected 'false'\0";
pub const MSG_CHAR_N: &[u8] = b"invalid literal, expected 'null'\0";
pub const MSG_CHAR: &[u8] = b"unexpected character, expected a JSON value\0";
pub const MSG_ARR_END: &[u8] = b"unexpected character, expected ',' or ']'\0";
pub const MSG_OBJ_KEY: &[u8] = b"unexpected character, expected a string key\0";
pub const MSG_OBJ_SEP: &[u8] = b"unexpected character, expected ':' after key\0";
pub const MSG_OBJ_END: &[u8] = b"unexpected character, expected ',' or '}'\0";
pub const MSG_GARBAGE: &[u8] = b"unexpected content after document\0";
pub const MSG_NOT_END: &[u8] = b"unexpected end of data\0";
pub const MSG_COMMENT: &[u8] = b"unclosed multiline comment\0";
pub const MSG_COMMA: &[u8] = b"trailing comma is not allowed\0";
pub const MSG_NAN_INF: &[u8] = b"nan or inf number is not allowed\0";
pub const MSG_ERR_TYPE: &[u8] = b"invalid JSON value type\0";
pub const MSG_ERR_BOM: &[u8] = b"UTF-8 byte order mark (BOM) is not supported\0";
pub const MSG_ERR_UTF8: &[u8] = b"invalid utf-8 encoding in string\0";
pub const MSG_ERR_UTF16: &[u8] = b"UTF-16 encoding is not supported\0";
pub const MSG_ERR_UTF32: &[u8] = b"UTF-32 encoding is not supported\0";
pub const MSG_DEPTH: &[u8] = b"depth limit exceeded\0";

/// A `const char *` pointing at one of the static messages above.
#[inline(always)]
pub fn msg(s: &'static [u8]) -> *const core::ffi::c_char {
    s.as_ptr() as *const core::ffi::c_char
}
