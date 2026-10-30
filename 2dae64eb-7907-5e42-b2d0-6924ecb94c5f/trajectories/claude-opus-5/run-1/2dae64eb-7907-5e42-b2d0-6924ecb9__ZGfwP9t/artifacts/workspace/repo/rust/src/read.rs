//! The JSON reader: literals, the number reader, the string reader, the
//! goto-based finite state machine of `read_root_minify` /
//! `read_root_pretty` / `read_root_single`, and the public `yyjson_read_*`
//! entry points.
//!
//! The C code builds its state machine out of `goto` statements; this port
//! keeps the same states as variants of an enum driven by one
//! `loop { match ... }`, so the transitions are a line-by-line match for the
//! original. The unrolled `repeat_in_1_18` / `repeat16_incr` macro chains
//! become ordinary counted loops with identical observable behaviour.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alc::YYJSON_DEFAULT_ALC;
use crate::file;
use crate::mem;
use crate::pow10::*;
use crate::tables::*;
use crate::types::*;
use crate::util::*;

pub const YYJSON_READ_ALLOW_TRIVIA: u32 =
    YYJSON_READ_ALLOW_COMMENTS | YYJSON_READ_ALLOW_EXT_WHITESPACE;

/// `has_rflag` / `has_flg` / `has_allow`.
#[inline(always)]
pub fn has_flg(flg: u32, chk: u32) -> bool {
    (flg & chk) != 0
}

/* ------------------------------------------------------------- literals */

/// `read_true`
#[inline(always)]
pub unsafe fn read_true(cur: &mut *mut u8, val: *mut yyjson_val) -> bool {
    let c = *cur;
    if byte_load_4(c) == byte_load_4(b"true".as_ptr()) {
        (*val).tag = (YYJSON_TYPE_BOOL | YYJSON_SUBTYPE_TRUE) as u64;
        *cur = c.add(4);
        return true;
    }
    false
}

/// `read_false`
#[inline(always)]
pub unsafe fn read_false(cur: &mut *mut u8, val: *mut yyjson_val) -> bool {
    let c = *cur;
    if byte_load_4(c.add(1)) == byte_load_4(b"alse".as_ptr()) {
        (*val).tag = (YYJSON_TYPE_BOOL | YYJSON_SUBTYPE_FALSE) as u64;
        *cur = c.add(5);
        return true;
    }
    false
}

/// `read_null`
#[inline(always)]
pub unsafe fn read_null(cur: &mut *mut u8, val: *mut yyjson_val) -> bool {
    let c = *cur;
    if byte_load_4(c) == byte_load_4(b"null".as_ptr()) {
        (*val).tag = YYJSON_TYPE_NULL as u64;
        *cur = c.add(4);
        return true;
    }
    false
}

/// `read_inf`
unsafe fn read_inf(ptr_: &mut *mut u8, pre: &mut *mut u8, flg: u32, val: *mut yyjson_val) -> bool {
    let hdr = *ptr_;
    let mut cur = *ptr_;
    let sign = *cur == b'-';
    if *cur == b'+' && !has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
        return false;
    }
    cur = cur.add(char_is_sign(*cur) as usize);
    if char_to_lower(*cur) == b'i'
        && char_to_lower(*cur.add(1)) == b'n'
        && char_to_lower(*cur.add(2)) == b'f'
    {
        if char_to_lower(*cur.add(3)) == b'i' {
            if char_to_lower(*cur.add(4)) == b'n'
                && char_to_lower(*cur.add(5)) == b'i'
                && char_to_lower(*cur.add(6)) == b't'
                && char_to_lower(*cur.add(7)) == b'y'
            {
                cur = cur.add(8);
            } else {
                return false;
            }
        } else {
            cur = cur.add(3);
        }
        *ptr_ = cur;
        if has_flg(flg, YYJSON_READ_NUMBER_AS_RAW) {
            **pre = 0;
            *pre = cur;
            (*val).tag = ((cur as u64 - hdr as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
            (*val).uni.str_ = hdr as *const c_char;
        } else {
            (*val).tag = (YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL) as u64;
            (*val).uni.u64_ = f64_bits_inf(sign);
        }
        return true;
    }
    false
}

/// `read_nan`
unsafe fn read_nan(ptr_: &mut *mut u8, pre: &mut *mut u8, flg: u32, val: *mut yyjson_val) -> bool {
    let hdr = *ptr_;
    let mut cur = *ptr_;
    let sign = *cur == b'-';
    if *cur == b'+' && !has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
        return false;
    }
    cur = cur.add(char_is_sign(*cur) as usize);
    if char_to_lower(*cur) == b'n'
        && char_to_lower(*cur.add(1)) == b'a'
        && char_to_lower(*cur.add(2)) == b'n'
    {
        cur = cur.add(3);
        *ptr_ = cur;
        if has_flg(flg, YYJSON_READ_NUMBER_AS_RAW) {
            **pre = 0;
            *pre = cur;
            (*val).tag = ((cur as u64 - hdr as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
            (*val).uni.str_ = hdr as *const c_char;
        } else {
            (*val).tag = (YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL) as u64;
            (*val).uni.u64_ = f64_bits_nan(sign);
        }
        return true;
    }
    false
}

/// `read_inf_or_nan`
#[inline(always)]
pub unsafe fn read_inf_or_nan(
    ptr_: &mut *mut u8,
    pre: &mut *mut u8,
    flg: u32,
    val: *mut yyjson_val,
) -> bool {
    if read_inf(ptr_, pre, flg, val) {
        return true;
    }
    if read_nan(ptr_, pre, flg, val) {
        return true;
    }
    false
}

/* ------------------------------------------------------- raw/hex numbers */

/// `read_num_raw`
unsafe fn read_num_raw(
    ptr_: &mut *mut u8,
    pre: &mut *mut u8,
    flg: u32,
    val: *mut yyjson_val,
    msg: &mut *const c_char,
) -> bool {
    let hdr = *ptr_;
    let mut cur = *ptr_;

    macro_rules! return_err {
        ($pos:expr, $m:expr) => {{
            *msg = $m.as_ptr() as *const c_char;
            *ptr_ = $pos;
            return false;
        }};
    }
    macro_rules! return_raw {
        () => {{
            (*val).tag = ((cur as u64 - hdr as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
            (*val).uni.str_ = hdr as *const c_char;
            **pre = 0;
            *pre = cur;
            *ptr_ = cur;
            return true;
        }};
    }

    /* skip sign */
    cur = cur.add((*cur == b'-') as usize);

    /* read first digit, check leading zero */
    let mut jump_to_double = false;
    while !char_is_digit(*cur) {
        if has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
            if *cur == b'+' && cur == hdr {
                cur = cur.add(1);
                continue;
            }
            if *cur == b'.' && char_is_digit(*cur.add(1)) {
                jump_to_double = true;
                break;
            }
        }
        if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN) && read_inf_or_nan(ptr_, pre, flg, val) {
            return true;
        }
        return_err!(cur, b"no digit after sign\0");
    }

    if !jump_to_double {
        /* read integral part */
        if *cur == b'0' {
            cur = cur.add(1);
            if char_is_digit(*cur) {
                return_err!(cur.sub(1), b"number with leading zero is not allowed\0");
            }
            if !char_is_fp(*cur) {
                if has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) && char_to_lower(*cur) == b'x' {
                    cur = cur.add(1);
                    if !char_is_hex(*cur) {
                        return_err!(cur, b"invalid hex number\0");
                    }
                    while char_is_hex(*cur) {
                        cur = cur.add(1);
                    }
                }
                return_raw!();
            }
        } else {
            while char_is_digit(*cur) {
                cur = cur.add(1);
            }
            if !char_is_fp(*cur) {
                return_raw!();
            }
        }
    }

    /* read fraction part */
    if *cur == b'.' {
        cur = cur.add(1);
        if !char_is_digit(*cur) {
            if has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
                if !char_is_exp(*cur) {
                    return_raw!();
                }
            } else {
                return_err!(cur, b"no digit after decimal point\0");
            }
        }
        while char_is_digit(*cur) {
            cur = cur.add(1);
        }
    }

    /* read exponent part */
    if char_is_exp(*cur) {
        cur = cur.add(1 + char_is_sign(*cur.add(1)) as usize);
        let c = *cur;
        cur = cur.add(1);
        if !char_is_digit(c) {
            return_err!(cur, b"no digit after exponent sign\0");
        }
        while char_is_digit(*cur) {
            cur = cur.add(1);
        }
    }

    return_raw!();
}

/// `read_num_hex`
unsafe fn read_num_hex(
    ptr_: &mut *mut u8,
    pre: &mut *mut u8,
    flg: u32,
    val: *mut yyjson_val,
    msg: &mut *const c_char,
) -> bool {
    let hdr = *ptr_;
    let mut cur = *ptr_;
    let mut sig: u64 = 0;
    let mut i: usize = 0;

    /* skip sign and '0x' */
    let sign = *cur == b'-';
    cur = cur.add((*cur == b'-' || *cur == b'+') as usize + 2);

    /* read hex */
    while i < 16 {
        let c = HEX_CONV_TABLE[*cur.add(i) as usize];
        if c == 0xF0 {
            break;
        }
        sig <<= 4;
        sig |= c as u64;
        i += 1;
    }

    if i == 0 {
        *msg = b"invalid hex number\0".as_ptr() as *const c_char;
        return false;
    }

    /* check overflow */
    if i == 16 && (char_is_hex(*cur.add(16)) || (sign && sig > (1u64 << 63))) {
        if !has_flg(flg, YYJSON_READ_BIGNUM_AS_RAW) {
            *msg = b"hex number overflow\0".as_ptr() as *const c_char;
            return false;
        }
        cur = cur.add(16);
        while char_is_hex(*cur) {
            cur = cur.add(1);
        }
        **pre = 0;
        (*val).tag = ((cur as u64 - hdr as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
        (*val).uni.str_ = hdr as *const c_char;
        *pre = cur;
        *ptr_ = cur;
        return true;
    }

    (*val).tag = YYJSON_TYPE_NUM as u64 | (((sign as u8) << 3) as u64);
    (*val).uni.u64_ = if sign { (!sig).wrapping_add(1) } else { sig };
    *ptr_ = cur.add(i);
    true
}

/* ---------------------------------------------------------- skip trivia */

/// `skip_trivia`
pub unsafe fn skip_trivia(ptr_: &mut *mut u8, eof: *mut u8, flg: u32) -> bool {
    let hdr = *ptr_;
    let mut cur = *ptr_;

    while cur < eof {
        let loop_begin = cur;

        /* skip standard whitespace */
        while char_is_space(*cur) {
            cur = cur.add(1);
        }

        /* skip extended whitespace */
        if has_flg(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE) {
            while char_is_space_ext(*cur) {
                let len = ext_space_len(cur);
                cur = cur.add(len);
                if len == 0 {
                    break;
                }
            }
        }

        /* skip comment, do not validate encoding */
        if has_flg(flg, YYJSON_READ_ALLOW_COMMENTS) && *cur == b'/' {
            if *cur.add(1) == b'/' {
                cur = cur.add(2);
                if has_flg(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE) {
                    while cur < eof {
                        if char_is_eol_ext(*cur) {
                            let len = ext_eol_len(cur);
                            cur = cur.add(len);
                            if len != 0 {
                                break;
                            }
                        }
                        cur = cur.add(1);
                    }
                } else {
                    while cur < eof && !char_is_eol(*cur) {
                        cur = cur.add(1);
                    }
                }
            } else if *cur.add(1) == b'*' {
                cur = cur.add(2);
                while byte_load_2(cur) != byte_load_2(b"*/".as_ptr()) && cur < eof {
                    cur = cur.add(1);
                }
                if cur == eof {
                    *ptr_ = eof;
                    return false; /* unclosed comment */
                }
                cur = cur.add(2);
            }
        }
        if cur == loop_begin {
            break;
        }
    }
    *ptr_ = cur;
    cur > hdr
}

/* ------------------------------------------------- truncation detection */

/// `is_truncated_utf8`
unsafe fn is_truncated_utf8(cur: *mut u8, eof: *mut u8) -> bool {
    let len = eof as usize - cur as usize;
    if cur >= eof || len >= 4 {
        return false;
    }
    let c0 = *cur;
    let c1 = *cur.add(1);
    let c2 = *cur.add(2);
    if c0 < 0x80 {
        return false;
    }
    if len == 1 {
        if (c0 & 0xE0) == 0xC0 && (c0 & 0x1E) != 0x00 {
            return true;
        }
        if (c0 & 0xF0) == 0xE0 {
            return true;
        }
        if (c0 & 0xF8) == 0xF0 && (c0 & 0x07) <= 0x04 {
            return true;
        }
    } else if len == 2 {
        if (c0 & 0xF0) == 0xE0 && (c1 & 0xC0) == 0x80 {
            let t = ((c0 & 0x0F) << 1) | ((c1 & 0x20) >> 5);
            return 0x01 <= t && t != 0x1B;
        }
        if (c0 & 0xF8) == 0xF0 && (c1 & 0xC0) == 0x80 {
            let t = ((c0 & 0x07) << 2) | ((c1 & 0x30) >> 4);
            return 0x01 <= t && t <= 0x10;
        }
    } else if len == 3 {
        if (c0 & 0xF8) == 0xF0 && (c1 & 0xC0) == 0x80 && (c2 & 0xC0) == 0x80 {
            let t = ((c0 & 0x07) << 2) | ((c1 & 0x30) >> 4);
            return 0x01 <= t && t <= 0x10;
        }
    }
    false
}

/// `is_truncated_str`. `s` is a NUL-terminated lowercase ASCII literal.
unsafe fn is_truncated_str(cur: *mut u8, eof: *mut u8, s: &[u8], case_sensitive: bool) -> bool {
    let len = s.len() - 1;
    if cur.add(len) <= eof || eof <= cur {
        return false;
    }
    if case_sensitive {
        return mem::memcmp_raw(cur, s.as_ptr(), eof as usize - cur as usize) == 0;
    }
    let mut c = cur;
    let mut i = 0usize;
    while c < eof {
        if char_to_lower(*c) != s[i] {
            return false;
        }
        c = c.add(1);
        i += 1;
    }
    true
}

/// `is_truncated_end`
pub(crate) unsafe fn is_truncated_end(hdr: *mut u8, cur: *mut u8, eof: *mut u8, code: u32, flg: u32) -> bool {
    let mut cur = cur;
    if cur >= eof {
        return true;
    }
    if code == YYJSON_READ_ERROR_LITERAL
        && (is_truncated_str(cur, eof, b"true\0", true)
            || is_truncated_str(cur, eof, b"false\0", true)
            || is_truncated_str(cur, eof, b"null\0", true))
    {
        return true;
    }
    if (code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER
        || code == YYJSON_READ_ERROR_INVALID_NUMBER
        || code == YYJSON_READ_ERROR_LITERAL)
        && has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
    {
        if *cur == b'-' {
            cur = cur.add(1);
        }
        if is_truncated_str(cur, eof, b"infinity\0", false)
            || is_truncated_str(cur, eof, b"nan\0", false)
        {
            return true;
        }
    }
    if code == YYJSON_READ_ERROR_UNEXPECTED_CONTENT
        && has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
        && hdr.add(3) <= cur
        && is_truncated_str(cur.sub(3), eof, b"infinity\0", false)
    {
        /* e.g. `infin` would be read as `inf` + `in` */
        return true;
    }
    if code == YYJSON_READ_ERROR_INVALID_STRING {
        let len = eof as usize - cur as usize;

        /* unicode escape sequence */
        if *cur == b'\\' {
            if len == 1 {
                return true;
            }
            if len <= 5 {
                cur = cur.add(1);
                if *cur != b'u' {
                    return false;
                }
                cur = cur.add(1);
                while cur < eof {
                    if !char_is_hex(*cur) {
                        return false;
                    }
                    cur = cur.add(1);
                }
                return true;
            } else if len <= 11 {
                /* incomplete surrogate pair? */
                let mut hi: u16 = 0;
                cur = cur.add(1);
                if *cur != b'u' {
                    return false;
                }
                cur = cur.add(1);
                if !hex_load_4(cur, &mut hi) {
                    return false;
                }
                if (hi & 0xF800) != 0xD800 {
                    return false;
                }
                cur = cur.add(4);
                if cur >= eof {
                    return true;
                }
                if *cur != b'\\' {
                    return false;
                }
                cur = cur.add(1);
                if cur >= eof {
                    return true;
                }
                if *cur != b'u' {
                    return false;
                }
                cur = cur.add(1);
                if cur >= eof {
                    return true;
                }
                if *cur != b'd' && *cur != b'D' {
                    return false;
                }
                cur = cur.add(1);
                if cur >= eof {
                    return true;
                }
                if (*cur < b'c' || *cur > b'f') && (*cur < b'C' || *cur > b'F') {
                    return false;
                }
                cur = cur.add(1);
                if cur >= eof {
                    return true;
                }
                if !char_is_hex(*cur) {
                    return false;
                }
                return true;
            }
            return false;
        }

        /* 2 to 4 bytes UTF-8 */
        if is_truncated_utf8(cur, eof) {
            return true;
        }
    }
    if has_flg(flg, YYJSON_READ_ALLOW_COMMENTS) {
        if code == YYJSON_READ_ERROR_INVALID_COMMENT {
            return true;
        }
        if code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER && *cur == b'/' && cur.add(1) == eof {
            return true;
        }
    }
    if code == YYJSON_READ_ERROR_UNEXPECTED_CHARACTER && has_flg(flg, YYJSON_READ_ALLOW_BOM) {
        let len = eof as usize - cur as usize;
        if cur == hdr && len < 3 && mem::memcmp_raw(hdr, b"\xEF\xBB\xBF".as_ptr(), len) == 0 {
            return true;
        }
    }
    false
}

/* ------------------------------------------------------ bigint & diy_fp */

/// `bigint`: unsigned arbitrarily large integer.
struct BigInt {
    used: u32,
    bits: [u64; 64],
}

impl BigInt {
    const fn zeroed() -> BigInt {
        BigInt {
            used: 0,
            bits: [0; 64],
        }
    }

    /// `bigint_add_u64`
    fn add_u64(&mut self, val: u64) {
        let num = self.bits[0];
        let add = num.wrapping_add(val);
        self.bits[0] = add;
        if add >= num || add >= val {
            return;
        }
        let max = self.used;
        let mut idx = 1u32;
        while idx < max {
            if self.bits[idx as usize] != U64_MAX {
                self.bits[idx as usize] += 1;
                return;
            }
            self.bits[idx as usize] = 0;
            idx += 1;
        }
        self.bits[self.used as usize] = 1;
        self.used += 1;
    }

    /// `bigint_mul_u64`
    fn mul_u64(&mut self, val: u64) {
        let max = self.used;
        let mut idx = 0u32;
        let mut carry: u64 = 0;
        while idx < max {
            if self.bits[idx as usize] != 0 {
                break;
            }
            idx += 1;
        }
        while idx < max {
            let (hi, lo) = u128_mul_add(self.bits[idx as usize], val, carry);
            self.bits[idx as usize] = lo;
            carry = hi;
            idx += 1;
        }
        if carry != 0 {
            self.bits[self.used as usize] = carry;
            self.used += 1;
        }
    }

    /// `bigint_mul_pow2`
    fn mul_pow2(&mut self, exp: u32) {
        let shft = exp % 64;
        let mut move_ = exp / 64;
        let mut idx = self.used;
        if shft == 0 {
            while idx > 0 {
                self.bits[(idx + move_ - 1) as usize] = self.bits[(idx - 1) as usize];
                idx -= 1;
            }
            self.used += move_;
            while move_ != 0 {
                move_ -= 1;
                self.bits[move_ as usize] = 0;
            }
        } else {
            self.bits[idx as usize] = 0;
            while idx > 0 {
                let mut num = self.bits[idx as usize] << shft;
                num |= self.bits[(idx - 1) as usize] >> (64 - shft);
                self.bits[(idx + move_) as usize] = num;
                idx -= 1;
            }
            self.bits[move_ as usize] = self.bits[0] << shft;
            self.used += move_ + (self.bits[(self.used + move_) as usize] > 0) as u32;
            while move_ != 0 {
                move_ -= 1;
                self.bits[move_ as usize] = 0;
            }
        }
    }

    /// `bigint_mul_pow10`
    fn mul_pow10(&mut self, exp: i32) {
        let mut exp = exp;
        while exp >= U64_POW10_MAX_EXACT_EXP {
            self.mul_u64(U64_POW10_TABLE[U64_POW10_MAX_EXACT_EXP as usize]);
            exp -= U64_POW10_MAX_EXACT_EXP;
        }
        if exp != 0 {
            self.mul_u64(U64_POW10_TABLE[exp as usize]);
        }
    }

    /// `bigint_cmp`
    fn cmp(&self, b: &BigInt) -> i32 {
        if self.used < b.used {
            return -1;
        }
        if self.used > b.used {
            return 1;
        }
        let mut idx = self.used;
        while idx > 0 {
            idx -= 1;
            let av = self.bits[idx as usize];
            let bv = b.bits[idx as usize];
            if av < bv {
                return -1;
            }
            if av > bv {
                return 1;
            }
        }
        0
    }

    /// `bigint_set_u64`
    fn set_u64(&mut self, val: u64) {
        self.used = 1;
        self.bits[0] = val;
    }

    /// `bigint_set_buf`
    unsafe fn set_buf(
        &mut self,
        sig: u64,
        exp: &mut i32,
        sig_cut: *mut u8,
        sig_end: *mut u8,
        dot_pos: *mut u8,
    ) {
        if sig_cut.is_null() {
            /* no digit cut, set significant part only */
            self.set_u64(sig);
            return;
        }
        let mut sig = sig;
        let mut sig_end = sig_end;
        let hdr = sig_cut;
        let mut cur = hdr;
        let mut len: u32 = 0;
        let mut val: u64 = 0;
        let mut dig_big_cut = false;
        let has_dot = (hdr < dot_pos) & (dot_pos < sig_end);
        let mut dig_len_total =
            U64_SAFE_DIG as usize + (sig_end as usize - hdr as usize) - has_dot as usize;

        sig -= (*sig_cut >= b'5') as u64; /* sig was rounded before */
        if dig_len_total > F64_MAX_DEC_DIG as usize {
            dig_big_cut = true;
            sig_end = sig_end.sub(dig_len_total - (F64_MAX_DEC_DIG as usize + 1));
            sig_end = sig_end.sub((dot_pos.add(1) == sig_end) as usize);
            dig_len_total = F64_MAX_DEC_DIG as usize + 1;
        }
        *exp -= dig_len_total as i32 - U64_SAFE_DIG as i32;

        self.used = 1;
        self.bits[0] = sig;
        while cur < sig_end {
            if cur != dot_pos {
                val = val * 10 + (*cur - b'0') as u64;
                cur = cur.add(1);
                len += 1;
                if cur == sig_end && dig_big_cut {
                    /* The last digit must be non-zero;    */
                    /* set it to '1' for correct rounding. */
                    val = val - (val % 10) + 1;
                }
                if len == U64_SAFE_DIG || cur == sig_end {
                    self.mul_pow10(len as i32);
                    self.add_u64(val);
                    val = 0;
                    len = 0;
                }
            } else {
                cur = cur.add(1);
            }
        }
    }
}

/// `pow10_table_get_sig`
#[inline(always)]
fn pow10_table_get_sig(exp10: i32) -> (u64, u64) {
    let idx = (exp10 - POW10_SIG_TABLE_MIN_EXP) as usize;
    (POW10_SIG_TABLE[idx * 2], POW10_SIG_TABLE[idx * 2 + 1])
}

/// `pow10_table_get_exp`
#[inline(always)]
fn pow10_table_get_exp(exp10: i32) -> i32 {
    exp10.wrapping_mul(217706).wrapping_sub(4128768) >> 16
}

/// `diy_fp`
#[derive(Copy, Clone)]
struct DiyFp {
    sig: u64,
    exp: i32,
}

/// `diy_fp_get_cached_pow10`
#[inline(always)]
fn diy_fp_get_cached_pow10(exp10: i32) -> DiyFp {
    let (sig, sig_ext) = pow10_table_get_sig(exp10);
    DiyFp {
        sig: sig.wrapping_add(sig_ext >> 63),
        exp: pow10_table_get_exp(exp10),
    }
}

/// `diy_fp_mul`
#[inline(always)]
fn diy_fp_mul(fp: DiyFp, fp2: DiyFp) -> DiyFp {
    let (hi, lo) = u128_mul(fp.sig, fp2.sig);
    DiyFp {
        sig: hi.wrapping_add(lo >> 63),
        exp: fp.exp + fp2.exp + 64,
    }
}

/// `diy_fp_to_ieee_raw`
#[inline(always)]
fn diy_fp_to_ieee_raw(fp: DiyFp) -> u64 {
    let mut sig = fp.sig;
    let mut exp = fp.exp;
    if fp.sig == 0 {
        return 0;
    }
    let lz_bits = u64_lz_bits(sig);
    sig <<= lz_bits;
    sig >>= F64_BITS - F64_SIG_FULL_BITS;
    exp -= lz_bits as i32;
    exp += (F64_BITS - F64_SIG_FULL_BITS) as i32;
    exp += F64_SIG_BITS as i32;

    if exp >= F64_MAX_BIN_EXP {
        F64_BITS_INF
    } else if exp >= F64_MIN_BIN_EXP - 1 {
        exp += F64_EXP_BIAS;
        ((exp as u64) << F64_SIG_BITS) | (sig & F64_SIG_MASK)
    } else if exp >= F64_MIN_BIN_EXP - F64_SIG_FULL_BITS as i32 {
        sig >> (F64_MIN_BIN_EXP - exp - 1)
    } else {
        0
    }
}

/* ------------------------------------------------------- number reader */

/// The outcome of the unrolled fraction scanner.
enum FracOut {
    FracEnd,
    FracMore,
}

/// The `digi_frac_i` / `digi_stop_i` chain of `read_num`: entering at index
/// `i` runs `expr_frac(i) .. expr_frac(18)` and then either falls through
/// (`cur += 20`) or stops at the first non-digit (`digi_stop_j`).
#[inline(always)]
unsafe fn frac_scan(cur: &mut *mut u8, sig: &mut u64, i: usize) -> FracOut {
    let mut j = i;
    loop {
        if j > 18 {
            *cur = (*cur).add(20);
            if !char_is_digit(**cur) {
                return FracOut::FracEnd;
            }
            return FracOut::FracMore;
        }
        let num = (*(*cur).add(j + 1)).wrapping_sub(b'0') as u64;
        if num <= 9 {
            *sig = num.wrapping_add(sig.wrapping_mul(10));
            j += 1;
        } else {
            *cur = (*cur).add(j + 1);
            return FracOut::FracEnd;
        }
    }
}

/// The `digi_*` labels of `read_num` that survive the unrolled prologue.
#[derive(PartialEq, Copy, Clone)]
enum NumSt {
    DigiIntgMore,
    DigiFracMore,
    DigiFracEnd,
    DigiExpMore,
    DigiExpFinish,
    DigiFinish,
}

/// `read_num`
pub unsafe fn read_num(
    ptr_: &mut *mut u8,
    pre: &mut *mut u8,
    flg: u32,
    val: *mut yyjson_val,
    msg: &mut *const c_char,
) -> bool {
    let mut sig_cut: *mut u8 = ptr::null_mut(); /* significant part cut position */
    let mut sig_end: *mut u8 = ptr::null_mut(); /* significant part end position */
    let mut dot_pos: *mut u8 = ptr::null_mut(); /* decimal point position */

    let mut sig: u64 = 0;
    let mut exp: i32 = 0;

    let mut exp_sig: i64 = 0;
    let mut exp_lit: i64 = 0;
    let mut num: u64;
    let mut tmp: *mut u8;

    let hdr = *ptr_;
    let mut cur = *ptr_;

    /* read number as raw string if has `YYJSON_READ_NUMBER_AS_RAW` flag */
    if has_flg(flg, YYJSON_READ_NUMBER_AS_RAW) {
        return read_num_raw(ptr_, pre, flg, val, msg);
    }

    let sign = *hdr == b'-';
    cur = cur.add(sign as usize);

    macro_rules! return_err {
        ($pos:expr, $m:expr) => {{
            *msg = $m.as_ptr() as *const c_char;
            *ptr_ = $pos;
            return false;
        }};
    }
    macro_rules! return_0 {
        () => {{
            (*val).tag = YYJSON_TYPE_NUM as u64 | (((sign as u8) << 3) as u64);
            (*val).uni.u64_ = 0;
            *ptr_ = cur;
            return true;
        }};
    }
    macro_rules! return_i64 {
        ($v:expr) => {{
            let v: u64 = $v;
            (*val).tag = YYJSON_TYPE_NUM as u64 | (((sign as u8) << 3) as u64);
            (*val).uni.u64_ = if sign { (!v).wrapping_add(1) } else { v };
            *ptr_ = cur;
            return true;
        }};
    }
    macro_rules! return_f64 {
        ($v:expr) => {{
            let v: f64 = $v;
            (*val).tag = (YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL) as u64;
            (*val).uni.f64_ = if sign { -v } else { v };
            *ptr_ = cur;
            return true;
        }};
    }
    macro_rules! return_f64_bin {
        ($v:expr) => {{
            let v: u64 = $v;
            (*val).tag = (YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL) as u64;
            (*val).uni.u64_ = ((sign as u64) << 63) | v;
            *ptr_ = cur;
            return true;
        }};
    }
    macro_rules! return_raw {
        () => {{
            **pre = 0;
            (*val).tag = ((cur as u64 - hdr as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
            (*val).uni.str_ = hdr as *const c_char;
            *pre = cur;
            *ptr_ = cur;
            return true;
        }};
    }
    macro_rules! return_inf {
        () => {{
            if has_flg(flg, YYJSON_READ_BIGNUM_AS_RAW) {
                return_raw!();
            }
            if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN) {
                return_f64_bin!(F64_BITS_INF);
            } else {
                return_err!(hdr, b"number is infinity when parsed as double\0");
            }
        }};
    }

    let mut state: NumSt = 'lead: {
        /* The C source reads this part as
         *
         *     while (unlikely(!char_is_nonzero(*cur))) { ... }
         *
         * whose body always leaves through a `return` or a `goto`. Two flags
         * stand in for the two `goto`s that target the body itself:
         * `leading_dot` for `goto leading_dot` and `zero_tail` for falling
         * through to the three statements that close the body.
         */
        let mut leading_dot = false;
        let mut zero_tail = false;
        while !char_is_nonzero(*cur) {
            if *cur != b'0' {
                /* non-digit char */
                if has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
                    if *cur == b'+' && cur == hdr {
                        cur = cur.add(1);
                        continue;
                    }
                    if *cur == b'.' && char_is_digit(*cur.add(1)) {
                        /* e.g. '.123' */
                        leading_dot = true;
                        zero_tail = true;
                        break;
                    }
                }
                if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                    && read_inf_or_nan(ptr_, pre, flg, val)
                {
                    return true;
                }
                return_err!(cur, b"no digit after sign\0");
            }
            /* begin with 0 */
            cur = cur.add(1);
            if !char_is_digit_or_fp(*cur) {
                if has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) && char_to_lower(*cur) == b'x' {
                    /* hex */
                    return read_num_hex(ptr_, pre, flg, val, msg);
                }
                return_0!();
            }
            if *cur == b'.' {
                leading_dot = true;
            }
            zero_tail = true;
            break;
        }

        if leading_dot {
            /* `leading_dot` label */
            dot_pos = cur;
            cur = cur.add(1);
            if !char_is_digit(*cur) {
                if has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
                    if char_is_exp(*cur) {
                        break 'lead NumSt::DigiExpMore;
                    } else {
                        return_f64_bin!(0);
                    }
                }
                return_err!(cur, b"no digit after decimal point\0");
            }
            while *cur == b'0' {
                cur = cur.add(1);
            }
            if char_is_digit(*cur) {
                /* first non-zero digit after decimal point */
                sig = (*cur - b'0') as u64;
                cur = cur.sub(1);
                match frac_scan(&mut cur, &mut sig, 1) {
                    FracOut::FracEnd => break 'lead NumSt::DigiFracEnd,
                    FracOut::FracMore => break 'lead NumSt::DigiFracMore,
                }
            }
        }

        if zero_tail {
            /* the tail of the `while` body in the C source */
            if char_is_digit(*cur) {
                return_err!(cur.sub(1), b"number with leading zero is not allowed\0");
            }
            if char_is_exp(*cur) {
                /* 0 with any exponent is still 0 */
                cur = cur.add(1 + char_is_sign(*cur.add(1)) as usize);
                if !char_is_digit(*cur) {
                    return_err!(cur, b"no digit after exponent sign\0");
                }
                loop {
                    cur = cur.add(1);
                    if !char_is_digit(*cur) {
                        break;
                    }
                }
            }
            return_f64_bin!(0);
        }

        /* begin with non-zero digit */
        sig = (*cur - b'0') as u64;

        /* read integral part: `repeat_in_1_18(expr_intg)` */
        let mut i: usize = 1;
        loop {
            if i > 18 {
                cur = cur.add(19); /* skip continuous 19 digits */
                if !char_is_digit_or_fp(*cur) {
                    /* this number is an integer consisting of 19 digits */
                    if sign && sig > (1u64 << 63) {
                        if has_flg(flg, YYJSON_READ_BIGNUM_AS_RAW) {
                            return_raw!();
                        }
                        return_f64!(sig as f64);
                    }
                    return_i64!(sig);
                }
                break 'lead NumSt::DigiIntgMore;
            }
            num = (*cur.add(i)).wrapping_sub(b'0') as u64;
            if num <= 9 {
                sig = num.wrapping_add(sig.wrapping_mul(10));
                i += 1;
                continue;
            }
            /* `digi_sepr_i`: process first non-digit character */
            if !char_is_fp(*cur.add(i)) {
                cur = cur.add(i);
                return_i64!(sig);
            }
            dot_pos = cur.add(i);
            if *cur.add(i) == b'.' {
                match frac_scan(&mut cur, &mut sig, i) {
                    FracOut::FracEnd => break 'lead NumSt::DigiFracEnd,
                    FracOut::FracMore => break 'lead NumSt::DigiFracMore,
                }
            }
            cur = cur.add(i);
            sig_end = cur;
            break 'lead NumSt::DigiExpMore;
        }
    };

    loop {
        match state {
            /* read more digits in integral part */
            NumSt::DigiIntgMore => {
                if char_is_digit(*cur) && !char_is_digit_or_fp(*cur.add(1)) {
                    /* this number is an integer consisting of 20 digits */
                    num = (*cur - b'0') as u64;
                    if sig < (U64_MAX / 10) || (sig == (U64_MAX / 10) && num <= (U64_MAX % 10)) {
                        sig = num + sig * 10;
                        cur = cur.add(1);
                        /* convert to double if overflow */
                        if sign {
                            if has_flg(flg, YYJSON_READ_BIGNUM_AS_RAW) {
                                return_raw!();
                            }
                            return_f64!(sig as f64);
                        }
                        return_i64!(sig);
                    }
                }

                if char_is_exp(*cur) {
                    dot_pos = cur;
                    state = NumSt::DigiExpMore;
                    continue;
                }

                if *cur == b'.' {
                    dot_pos = cur;
                    cur = cur.add(1);
                    if !char_is_digit(*cur) {
                        if has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
                            state = NumSt::DigiFracEnd;
                            continue;
                        }
                        return_err!(cur, b"no digit after decimal point\0");
                    }
                }
                state = NumSt::DigiFracMore;
            }

            /* read more digits in fraction part */
            NumSt::DigiFracMore => {
                sig_cut = cur; /* too large to fit in u64, excess digits cut */
                sig += (*cur >= b'5') as u64; /* round */
                loop {
                    cur = cur.add(1);
                    if !char_is_digit(*cur) {
                        break;
                    }
                }
                if dot_pos.is_null() {
                    if !char_is_fp(*cur) && has_flg(flg, YYJSON_READ_BIGNUM_AS_RAW) {
                        return_raw!(); /* it's a large integer */
                    }
                    dot_pos = cur;
                    if *cur == b'.' {
                        cur = cur.add(1);
                        if !char_is_digit(*cur) && !has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
                            return_err!(cur, b"no digit after decimal point\0");
                        }
                        while char_is_digit(*cur) {
                            cur = cur.add(1);
                        }
                    }
                }
                exp_sig = (dot_pos as isize - sig_cut as isize) as i64;
                exp_sig += (dot_pos < sig_cut) as i64;

                /* ignore trailing zeros */
                tmp = cur.sub(1);
                while (*tmp == b'0' || *tmp == b'.') && tmp > hdr {
                    tmp = tmp.sub(1);
                }
                if tmp < sig_cut {
                    sig_cut = ptr::null_mut();
                } else {
                    sig_end = cur;
                }

                state = if char_is_exp(*cur) {
                    NumSt::DigiExpMore
                } else {
                    NumSt::DigiExpFinish
                };
            }

            /* fraction part end */
            NumSt::DigiFracEnd => {
                if dot_pos.add(1) == cur && !has_flg(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
                    return_err!(cur, b"no digit after decimal point\0");
                }
                sig_end = cur;
                exp_sig = -(((cur as u64 - dot_pos as u64).wrapping_sub(1)) as i64);
                if !char_is_exp(*cur) {
                    if exp_sig < (F64_MIN_DEC_EXP - 19) as i64 {
                        return_f64_bin!(0); /* underflow */
                    }
                    exp = exp_sig as i32;
                    state = NumSt::DigiFinish;
                } else {
                    state = NumSt::DigiExpMore;
                }
            }

            /* read exponent part */
            NumSt::DigiExpMore => {
                cur = cur.add(1);
                let exp_sign = *cur == b'-';
                cur = cur.add(char_is_sign(*cur) as usize);
                if !char_is_digit(*cur) {
                    return_err!(cur, b"no digit after exponent sign\0");
                }
                while *cur == b'0' {
                    cur = cur.add(1);
                }

                /* read exponent literal */
                tmp = cur;
                while char_is_digit(*cur) {
                    exp_lit =
                        ((*cur - b'0') as u64).wrapping_add((exp_lit as u64).wrapping_mul(10)) as i64;
                    cur = cur.add(1);
                }
                if (cur as usize - tmp as usize) >= U64_SAFE_DIG as usize {
                    if exp_sign {
                        return_f64_bin!(0); /* underflow */
                    } else {
                        return_inf!(); /* overflow */
                    }
                }
                exp_sig += if exp_sign { -exp_lit } else { exp_lit };
                state = NumSt::DigiExpFinish;
            }

            /* validate exponent value */
            NumSt::DigiExpFinish => {
                if exp_sig < (F64_MIN_DEC_EXP - 19) as i64 {
                    return_f64_bin!(0); /* underflow */
                }
                if exp_sig > F64_MAX_DEC_EXP as i64 {
                    return_inf!(); /* overflow */
                }
                exp = exp_sig as i32;
                state = NumSt::DigiFinish;
            }

            NumSt::DigiFinish => break,
        }
    }

    /* Fast path 1: exact double arithmetic */
    if sig < (1u64 << 53) && exp >= -F64_POW10_MAX_EXACT_EXP && exp <= F64_POW10_MAX_EXACT_EXP {
        let mut dbl = sig as f64;
        if exp < 0 {
            dbl /= F64_POW10_TABLE[(-exp) as usize];
        } else {
            dbl *= F64_POW10_TABLE[exp as usize];
        }
        return_f64!(dbl);
    }
    if sig == 0 {
        return_f64_bin!(0);
    }

    /* Fast path 2: 128-bit multiplication with the cached pow10 table */
    if sig_cut.is_null() && exp > -F64_MAX_DEC_EXP + 1 && exp < F64_MAX_DEC_EXP - 20 {
        let mut exact = false;
        let (sig2, sig2_ext) = pow10_table_get_sig(exp);
        let mut exp2 = pow10_table_get_exp(exp);

        let mut lz = u64_lz_bits(sig);
        let sig1 = sig << lz;
        exp2 -= lz as i32;
        let (mut hi, lo) = u128_mul(sig1, sig2);

        let bits = hi & ((1u64 << (64 - 54 - 1)) - 1);
        if bits.wrapping_sub(1) < ((1u64 << (64 - 54 - 1)) - 2) {
            exact = true;
        } else {
            let (hi2, _lo2) = u128_mul(sig1, sig2_ext);
            let add = lo.wrapping_add(hi2);
            if add.wrapping_add(1) > 1 {
                let carry = add < lo || add < hi2;
                hi = hi.wrapping_add(carry as u64);
                exact = true;
            }
        }

        if exact {
            /* normalize */
            lz = (hi < (1u64 << 63)) as u32;
            hi <<= lz;
            exp2 -= lz as i32;
            exp2 += 64;

            /* test the bit 54 and get rounding direction */
            let round_up = (hi & (1u64 << (64 - 54))) > 0;
            hi = hi.wrapping_add(if round_up { 1u64 << (64 - 54) } else { 0 });

            /* test overflow */
            if hi < (1u64 << (64 - 54)) {
                hi = 1u64 << 63;
                exp2 += 1;
            }

            hi >>= F64_BITS - F64_SIG_FULL_BITS;
            exp2 += (F64_BITS - F64_SIG_FULL_BITS + F64_SIG_BITS) as i32;
            exp2 += F64_EXP_BIAS;
            let raw = ((exp2 as u64) << F64_SIG_BITS) | (hi & F64_SIG_MASK);
            return_f64_bin!(raw);
        }
    }

    /* Slow path: diy_fp approximation, verified with bigint comparison */
    {
        const ERR_ULP_LOG: i32 = 3;
        const ERR_ULP: i32 = 1 << ERR_ULP_LOG;
        const ERR_CACHED_POW: i32 = ERR_ULP / 2;
        const ERR_MUL_FIXED: i32 = ERR_ULP / 2;
        const DIY_SIG_BITS: i32 = 64;
        const EXP_BIAS: i32 = F64_EXP_BIAS + F64_SIG_BITS as i32;
        const EXP_SUBNORMAL: i32 = -EXP_BIAS + 1;

        let mut fp = DiyFp { sig, exp: 0 };
        let mut fp_err: u64 = if !sig_cut.is_null() {
            (ERR_ULP / 2) as u64
        } else {
            0
        };

        /* normalize */
        let mut bits = u64_lz_bits(fp.sig);
        fp.sig <<= bits;
        fp.exp -= bits as i32;
        fp_err <<= bits;

        /* multiply and add error */
        fp = diy_fp_mul(fp, diy_fp_get_cached_pow10(exp));
        fp_err += ERR_CACHED_POW as u64 + (fp_err != 0) as u64 + ERR_MUL_FIXED as u64;

        /* normalize */
        bits = u64_lz_bits(fp.sig);
        fp.sig <<= bits;
        fp.exp -= bits as i32;
        fp_err <<= bits;

        /* effective significand */
        let order_of_magnitude = DIY_SIG_BITS + fp.exp;
        let effective_significand_size =
            if order_of_magnitude >= EXP_SUBNORMAL + F64_SIG_FULL_BITS as i32 {
                F64_SIG_FULL_BITS as i32
            } else if order_of_magnitude <= EXP_SUBNORMAL {
                0
            } else {
                order_of_magnitude - EXP_SUBNORMAL
            };

        /* precision digits count */
        let mut precision_digits_count = DIY_SIG_BITS - effective_significand_size;
        if precision_digits_count + ERR_ULP_LOG >= DIY_SIG_BITS {
            let shr = (precision_digits_count + ERR_ULP_LOG) - DIY_SIG_BITS + 1;
            fp.sig >>= shr;
            fp.exp += shr;
            fp_err = (fp_err >> shr) + 1 + ERR_ULP as u64;
            precision_digits_count -= shr;
        }

        /* half way */
        let mut precision_bits = fp.sig & ((1u64 << precision_digits_count) - 1);
        precision_bits = precision_bits.wrapping_mul(ERR_ULP as u64);
        let mut half_way = 1u64 << (precision_digits_count - 1);
        half_way = half_way.wrapping_mul(ERR_ULP as u64);

        /* rounding */
        fp.sig >>= precision_digits_count;
        fp.sig += (precision_bits >= half_way.wrapping_add(fp_err)) as u64;
        fp.exp += precision_digits_count;

        /* get IEEE double raw value */
        let mut raw = diy_fp_to_ieee_raw(fp);
        if raw == F64_BITS_INF {
            return_inf!();
        }
        if precision_bits <= half_way.wrapping_sub(fp_err)
            || precision_bits >= half_way.wrapping_add(fp_err)
        {
            return_f64_bin!(raw); /* number is accurate */
        }

        /* upper boundary */
        let mut fp_upper = DiyFp { sig: 0, exp: 0 };
        if raw & F64_EXP_MASK != 0 {
            fp_upper.sig = (raw & F64_SIG_MASK) + (1u64 << F64_SIG_BITS);
            fp_upper.exp = ((raw & F64_EXP_MASK) >> F64_SIG_BITS) as i32;
        } else {
            fp_upper.sig = raw & F64_SIG_MASK;
            fp_upper.exp = 1;
        }
        fp_upper.exp -= F64_EXP_BIAS + F64_SIG_BITS as i32;
        fp_upper.sig <<= 1;
        fp_upper.exp -= 1;
        fp_upper.sig += 1; /* add half ulp */

        /* compare with bigint */
        let mut big_full = BigInt::zeroed();
        let mut big_comp = BigInt::zeroed();
        big_full.set_buf(sig, &mut exp, sig_cut, sig_end, dot_pos);
        big_comp.set_u64(fp_upper.sig);
        if exp >= 0 {
            big_full.mul_pow10(exp);
        } else {
            big_comp.mul_pow10(-exp);
        }
        if fp_upper.exp > 0 {
            big_comp.mul_pow2(fp_upper.exp as u32);
        } else {
            big_full.mul_pow2((-fp_upper.exp) as u32);
        }
        let cmp = big_full.cmp(&big_comp);
        if cmp != 0 {
            /* round down or round up */
            raw += (cmp > 0) as u64;
        } else {
            /* falls midway, round to even */
            raw += raw & 1;
        }

        if raw == F64_BITS_INF {
            return_inf!();
        }
        return_f64_bin!(raw);
    }
}

/* ------------------------------------------------------- string reader */

/// `read_uni_esc`
unsafe fn read_uni_esc(
    src_ptr: &mut *mut u8,
    dst_ptr: &mut *mut u8,
    msg: &mut *const c_char,
) -> bool {
    let mut src = *src_ptr;
    let mut dst = *dst_ptr;
    let mut hi: u16 = 0;
    let mut lo: u16 = 0;

    macro_rules! return_err {
        ($end:expr, $m:expr) => {{
            *msg = $m.as_ptr() as *const c_char;
            *src_ptr = $end;
            return false;
        }};
    }

    src = src.add(2); /* skip `\u` */
    if !hex_load_4(src, &mut hi) {
        return_err!(src.sub(2), b"invalid escaped sequence in string\0");
    }
    src = src.add(4); /* skip hex */
    if (hi & 0xF800) != 0xD800 {
        /* a BMP character */
        if hi >= 0x800 {
            *dst = 0xE0 | (hi >> 12) as u8;
            *dst.add(1) = 0x80 | ((hi >> 6) & 0x3F) as u8;
            *dst.add(2) = 0x80 | (hi & 0x3F) as u8;
            dst = dst.add(3);
        } else if hi >= 0x80 {
            *dst = 0xC0 | (hi >> 6) as u8;
            *dst.add(1) = 0x80 | (hi & 0x3F) as u8;
            dst = dst.add(2);
        } else {
            *dst = hi as u8;
            dst = dst.add(1);
        }
    } else {
        /* a non-BMP character, represented as a surrogate pair */
        if (hi & 0xFC00) != 0xD800 {
            return_err!(src.sub(6), b"invalid high surrogate in string\0");
        }
        if byte_load_2(src) != byte_load_2(b"\\u".as_ptr()) {
            return_err!(src.sub(6), b"no low surrogate in string\0");
        }
        if !hex_load_4(src.add(2), &mut lo) {
            return_err!(src.sub(6), b"invalid escape in string\0");
        }
        if (lo & 0xFC00) != 0xDC00 {
            return_err!(src.sub(6), b"invalid low surrogate in string\0");
        }
        let uni = ((((hi as u32) - 0xD800) << 10) | ((lo as u32) - 0xDC00)) + 0x10000;
        *dst = 0xF0 | (uni >> 18) as u8;
        *dst.add(1) = 0x80 | ((uni >> 12) & 0x3F) as u8;
        *dst.add(2) = 0x80 | ((uni >> 6) & 0x3F) as u8;
        *dst.add(3) = 0x80 | (uni & 0x3F) as u8;
        dst = dst.add(4);
        src = src.add(6);
    }
    *src_ptr = src;
    *dst_ptr = dst;
    true
}

/// `read_str_opt`
pub unsafe fn read_str_opt(
    quo: u8,
    ptr_: &mut *mut u8,
    eof: *mut u8,
    flg: u32,
    val: *mut yyjson_val,
    msg: &mut *const c_char,
    con: *mut [*mut u8; 2],
) -> bool {
    let hdr = (*ptr_).add(1);
    let mut src = hdr;
    let mut dst: *mut u8 = ptr::null_mut();
    let mut pos: *mut u8;
    let mut uni: u32;

    macro_rules! return_err {
        ($end:expr, $m:expr) => {{
            *msg = $m.as_ptr() as *const c_char;
            *ptr_ = $end;
            if !con.is_null() {
                (*con)[0] = $end;
                (*con)[1] = dst;
            }
            return false;
        }};
    }
    macro_rules! return_err_keep_msg {
        ($end:expr) => {{
            *ptr_ = $end;
            if !con.is_null() {
                (*con)[0] = $end;
                (*con)[1] = dst;
            }
            return false;
        }};
    }
    macro_rules! skip_ok {
        ($c:expr) => {
            if quo == b'"' {
                char_is_ascii_skip($c)
            } else {
                char_is_ascii_skip_sq($c)
            }
        };
    }

    enum S {
        SkipAscii,
        SkipUtf8,
        CopyEscape,
        CopyAscii,
        CopyUtf8,
    }
    let mut state = S::SkipAscii;

    /* Resume incremental parsing. */
    if !con.is_null() && !(*con)[0].is_null() {
        src = (*con)[0];
        dst = (*con)[1];
        if !dst.is_null() {
            state = S::CopyAscii;
        }
    }

    loop {
        match state {
            S::SkipAscii => {
                /* `repeat16_incr(expr_jump)`: skip 16 bytes at a time */
                loop {
                    let mut i = 0usize;
                    while i < 16 && skip_ok!(*src.add(i)) {
                        i += 1;
                    }
                    if i == 16 {
                        src = src.add(16);
                        continue;
                    }
                    src = src.add(i);
                    break;
                }
                /* `skip_ascii_end` */
                if *src == quo {
                    (*val).tag = ((src as u64 - hdr as u64) << YYJSON_TAG_BIT)
                        | YYJSON_TYPE_STR as u64
                        | if quo == b'"' {
                            YYJSON_SUBTYPE_NOESC as u64
                        } else {
                            0
                        };
                    (*val).uni.str_ = hdr as *const c_char;
                    *src = 0;
                    *ptr_ = src.add(1);
                    if !con.is_null() {
                        (*con)[0] = ptr::null_mut();
                        (*con)[1] = ptr::null_mut();
                    }
                    return true;
                }
                state = S::SkipUtf8;
            }

            S::SkipUtf8 => {
                if *src & 0x80 != 0 {
                    /* non-ASCII character */
                    pos = src;
                    uni = byte_load_4(src);
                    while is_utf8_seq3(uni) {
                        src = src.add(3);
                        uni = byte_load_4(src);
                    }
                    if is_utf8_seq1(uni) {
                        state = S::SkipAscii;
                        continue;
                    }
                    while is_utf8_seq2(uni) {
                        src = src.add(2);
                        uni = byte_load_4(src);
                    }
                    while is_utf8_seq4(uni) {
                        src = src.add(4);
                        uni = byte_load_4(src);
                    }
                    if pos == src {
                        if has_flg(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                            src = src.add(1);
                        } else {
                            return_err!(src, b"invalid UTF-8 encoding in string\0");
                        }
                    }
                    state = S::SkipAscii;
                    continue;
                }
                /* The escape character appears, we need to copy it. */
                dst = src;
                state = S::CopyEscape;
            }

            S::CopyEscape => {
                if *src == b'\\' {
                    src = src.add(1);
                    let esc = *src;
                    match esc {
                        b'"' | b'\\' | b'/' => {
                            *dst = esc;
                            dst = dst.add(1);
                            src = src.add(1);
                        }
                        b'b' => {
                            *dst = 0x08;
                            dst = dst.add(1);
                            src = src.add(1);
                        }
                        b'f' => {
                            *dst = 0x0C;
                            dst = dst.add(1);
                            src = src.add(1);
                        }
                        b'n' => {
                            *dst = b'\n';
                            dst = dst.add(1);
                            src = src.add(1);
                        }
                        b'r' => {
                            *dst = b'\r';
                            dst = dst.add(1);
                            src = src.add(1);
                        }
                        b't' => {
                            *dst = b'\t';
                            dst = dst.add(1);
                            src = src.add(1);
                        }
                        b'u' => {
                            src = src.sub(1);
                            if !read_uni_esc(&mut src, &mut dst, msg) {
                                return_err_keep_msg!(src);
                            }
                        }
                        _ => {
                            if has_flg(flg, YYJSON_READ_ALLOW_EXT_ESCAPE) {
                                /* read extended escape (non-standard) */
                                match esc {
                                    b'\'' => {
                                        *dst = b'\'';
                                        dst = dst.add(1);
                                        src = src.add(1);
                                    }
                                    b'a' => {
                                        *dst = 0x07;
                                        dst = dst.add(1);
                                        src = src.add(1);
                                    }
                                    b'v' => {
                                        *dst = 0x0B;
                                        dst = dst.add(1);
                                        src = src.add(1);
                                    }
                                    b'?' => {
                                        *dst = b'?';
                                        dst = dst.add(1);
                                        src = src.add(1);
                                    }
                                    b'e' => {
                                        *dst = 0x1B;
                                        dst = dst.add(1);
                                        src = src.add(1);
                                    }
                                    b'0' => {
                                        if !char_is_digit(*src.add(1)) {
                                            *dst = 0;
                                            dst = dst.add(1);
                                            src = src.add(1);
                                        } else {
                                            return_err!(
                                                src.sub(1),
                                                b"octal escape is not allowed\0"
                                            );
                                        }
                                    }
                                    b'1'..=b'9' => {
                                        return_err!(src.sub(1), b"invalid number escape\0");
                                    }
                                    b'x' => {
                                        let mut c: u8 = 0;
                                        if hex_load_2(src.add(1), &mut c) {
                                            src = src.add(3);
                                            if c <= 0x7F {
                                                /* 1-byte ASCII */
                                                *dst = c;
                                                dst = dst.add(1);
                                            } else {
                                                /* 2-byte UTF-8 */
                                                *dst = 0xC0 | (c >> 6);
                                                *dst.add(1) = 0x80 | (c & 0x3F);
                                                dst = dst.add(2);
                                            }
                                        } else {
                                            return_err!(src.sub(1), b"invalid hex escape\0");
                                        }
                                    }
                                    b'\n' => {
                                        src = src.add(1);
                                    }
                                    b'\r' => {
                                        src = src.add(1);
                                        src = src.add((*src == b'\n') as usize);
                                    }
                                    0xE2 => {
                                        /* Line terminator: U+2028, U+2029 */
                                        if (*src.add(1) == 0x80 && *src.add(2) == 0xA8)
                                            || (*src.add(1) == 0x80 && *src.add(2) == 0xA9)
                                        {
                                            src = src.add(3);
                                        }
                                    }
                                    _ => {}
                                }
                            } else if quo == b'\'' && esc == b'\'' {
                                *dst = b'\'';
                                dst = dst.add(1);
                                src = src.add(1);
                            } else {
                                return_err!(
                                    src.sub(1),
                                    b"invalid escaped sequence in string\0"
                                );
                            }
                        }
                    }
                } else if *src == quo {
                    (*val).tag =
                        ((dst as u64 - hdr as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_STR as u64;
                    (*val).uni.str_ = hdr as *const c_char;
                    *dst = 0;
                    *ptr_ = src.add(1);
                    if !con.is_null() {
                        (*con)[0] = ptr::null_mut();
                        (*con)[1] = ptr::null_mut();
                    }
                    return true;
                } else {
                    if !has_flg(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                        return_err!(src, b"unexpected control character in string\0");
                    }
                    if src >= eof {
                        return_err!(src, b"unclosed string\0");
                    }
                    *dst = *src;
                    dst = dst.add(1);
                    src = src.add(1);
                }
                state = S::CopyAscii;
            }

            S::CopyAscii => {
                let stop_i;
                loop {
                    let mut i = 0usize;
                    while i < 16 && skip_ok!(*src.add(i)) {
                        i += 1;
                    }
                    if i == 16 {
                        mem::move_(dst, src, 16);
                        dst = dst.add(16);
                        src = src.add(16);
                        continue;
                    }
                    stop_i = i;
                    break;
                }
                /* `copy_ascii_stop_i`: one extra byte may be moved, which is
                   safe because `dst <= src` and the byte is rewritten later. */
                byte_move_forward(dst, src, stop_i);
                dst = dst.add(stop_i);
                src = src.add(stop_i);
                state = S::CopyUtf8;
            }

            S::CopyUtf8 => {
                if *src & 0x80 != 0 {
                    pos = src;
                    uni = byte_load_4(src);
                    while is_utf8_seq3(uni) {
                        mem::copy(dst, uni.to_le_bytes().as_ptr(), 4);
                        dst = dst.add(3);
                        src = src.add(3);
                        uni = byte_load_4(src);
                    }
                    if is_utf8_seq1(uni) {
                        state = S::CopyAscii;
                        continue;
                    }
                    while is_utf8_seq2(uni) {
                        mem::copy(dst, uni.to_le_bytes().as_ptr(), 2);
                        dst = dst.add(2);
                        src = src.add(2);
                        uni = byte_load_4(src);
                    }
                    while is_utf8_seq4(uni) {
                        mem::copy(dst, uni.to_le_bytes().as_ptr(), 4);
                        dst = dst.add(4);
                        src = src.add(4);
                        uni = byte_load_4(src);
                    }
                    if pos == src {
                        if !has_flg(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                            return_err!(src, MSG_ERR_UTF8);
                        }
                        /* `goto copy_ascii_stop_1` */
                        byte_move_forward(dst, src, 1);
                        dst = dst.add(1);
                        src = src.add(1);
                        continue;
                    }
                    state = S::CopyAscii;
                    continue;
                }
                state = S::CopyEscape;
            }
        }
    }
}

#[inline(always)]
pub unsafe fn read_str(
    ptr_: &mut *mut u8,
    eof: *mut u8,
    flg: u32,
    val: *mut yyjson_val,
    msg: &mut *const c_char,
) -> bool {
    read_str_opt(b'"', ptr_, eof, flg, val, msg, ptr::null_mut())
}

#[inline(always)]
pub unsafe fn read_str_con(
    ptr_: &mut *mut u8,
    eof: *mut u8,
    flg: u32,
    val: *mut yyjson_val,
    msg: &mut *const c_char,
    con: *mut [*mut u8; 2],
) -> bool {
    read_str_opt(b'"', ptr_, eof, flg, val, msg, con)
}

#[inline(always)]
pub unsafe fn read_str_sq(
    ptr_: &mut *mut u8,
    eof: *mut u8,
    flg: u32,
    val: *mut yyjson_val,
    msg: &mut *const c_char,
) -> bool {
    read_str_opt(b'\'', ptr_, eof, flg, val, msg, ptr::null_mut())
}

/// `read_str_id`: unquoted key (JSON5 identifier name).
pub unsafe fn read_str_id(
    ptr_: &mut *mut u8,
    _eof: *mut u8,
    flg: u32,
    pre: &mut *mut u8,
    val: *mut yyjson_val,
    msg: &mut *const c_char,
) -> bool {
    let hdr = *ptr_;
    let mut src = hdr;
    let mut dst: *mut u8 = ptr::null_mut();
    let mut uni: u32;

    macro_rules! return_err {
        ($end:expr, $m:expr) => {{
            *msg = $m.as_ptr() as *const c_char;
            *ptr_ = $end;
            return false;
        }};
    }
    macro_rules! return_suc {
        ($str_end:expr, $cur_end:expr) => {{
            (*val).tag =
                (($str_end as u64 - hdr as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_STR as u64;
            (*val).uni.str_ = hdr as *const c_char;
            *pre = $str_end;
            *ptr_ = $cur_end;
            return true;
        }};
    }

    /* add null-terminator for previous raw string */
    **pre = 0;

    enum S {
        SkipAscii,
        SkipUtf8,
        CopyEscape,
        CopyAscii,
        CopyUtf8,
    }
    let mut state = S::SkipAscii;

    loop {
        match state {
            S::SkipAscii => {
                loop {
                    let mut i = 0usize;
                    while i < 16 && char_is_id_ascii(*src.add(i)) {
                        i += 1;
                    }
                    if i == 16 {
                        src = src.add(16);
                        continue;
                    }
                    src = src.add(i);
                    break;
                }
                if !char_is_id_next(*src) {
                    return_suc!(src, src);
                }
                state = S::SkipUtf8;
            }

            S::SkipUtf8 => {
                while *src >= 0x80 {
                    if has_flg(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE)
                        && char_is_space_ext(*src)
                        && ext_space_len(src) != 0
                    {
                        return_suc!(src, src);
                    }
                    uni = byte_load_4(src);
                    if is_utf8_seq2(uni) {
                        src = src.add(2);
                    } else if is_utf8_seq3(uni) {
                        src = src.add(3);
                    } else if is_utf8_seq4(uni) {
                        src = src.add(4);
                    } else {
                        if !has_flg(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                            return_err!(src, MSG_ERR_UTF8);
                        }
                        src = src.add(1);
                    }
                }
                if char_is_id_ascii(*src) {
                    state = S::SkipAscii;
                    continue;
                }
                dst = src;
                state = S::CopyEscape;
            }

            S::CopyEscape => {
                if byte_load_2(src) == byte_load_2(b"\\u".as_ptr()) {
                    if !read_uni_esc(&mut src, &mut dst, msg) {
                        *ptr_ = src;
                        return false;
                    }
                } else {
                    if !char_is_id_next(*src) {
                        return_suc!(dst, src);
                    }
                    return_err!(src, b"unexpected character in key\0");
                }
                state = S::CopyAscii;
            }

            S::CopyAscii => {
                let stop_i;
                loop {
                    let mut i = 0usize;
                    while i < 16 && char_is_id_ascii(*src.add(i)) {
                        i += 1;
                    }
                    if i == 16 {
                        mem::move_(dst, src, 16);
                        dst = dst.add(16);
                        src = src.add(16);
                        continue;
                    }
                    stop_i = i;
                    break;
                }
                byte_move_forward(dst, src, stop_i);
                dst = dst.add(stop_i);
                src = src.add(stop_i);
                state = S::CopyUtf8;
            }

            S::CopyUtf8 => {
                while *src >= 0x80 {
                    if has_flg(flg, YYJSON_READ_ALLOW_EXT_WHITESPACE)
                        && char_is_space_ext(*src)
                        && ext_space_len(src) != 0
                    {
                        return_suc!(dst, src);
                    }
                    uni = byte_load_4(src);
                    if is_utf8_seq2(uni) {
                        mem::copy(dst, uni.to_le_bytes().as_ptr(), 2);
                        dst = dst.add(2);
                        src = src.add(2);
                    } else if is_utf8_seq3(uni) {
                        mem::copy(dst, uni.to_le_bytes().as_ptr(), 4);
                        dst = dst.add(3);
                        src = src.add(3);
                    } else if is_utf8_seq4(uni) {
                        mem::copy(dst, uni.to_le_bytes().as_ptr(), 4);
                        dst = dst.add(4);
                        src = src.add(4);
                    } else {
                        if !has_flg(flg, YYJSON_READ_ALLOW_INVALID_UNICODE) {
                            return_err!(src, MSG_ERR_UTF8);
                        }
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                    }
                }
                if char_is_id_ascii(*src) {
                    state = S::CopyAscii;
                    continue;
                }
                state = S::CopyEscape;
            }
        }
    }
}

/* ------------------------------------------------------------ FSM reader */

/// The `fail_*` labels of the root readers.
#[derive(Copy, Clone, PartialEq)]
enum Fail {
    String,
    Number,
    Alloc,
    TrailingComma,
    LiteralTrue,
    LiteralFalse,
    LiteralNull,
    CharacterVal,
    CharacterArrEnd,
    CharacterObjKey,
    CharacterObjSep,
    CharacterObjEnd,
    Comment,
    Garbage,
}

impl Fail {
    fn code(self) -> u32 {
        match self {
            Fail::String => YYJSON_READ_ERROR_INVALID_STRING,
            Fail::Number => YYJSON_READ_ERROR_INVALID_NUMBER,
            Fail::Alloc => YYJSON_READ_ERROR_MEMORY_ALLOCATION,
            Fail::TrailingComma => YYJSON_READ_ERROR_JSON_STRUCTURE,
            Fail::LiteralTrue | Fail::LiteralFalse | Fail::LiteralNull => {
                YYJSON_READ_ERROR_LITERAL
            }
            Fail::CharacterVal
            | Fail::CharacterArrEnd
            | Fail::CharacterObjKey
            | Fail::CharacterObjSep
            | Fail::CharacterObjEnd => YYJSON_READ_ERROR_UNEXPECTED_CHARACTER,
            Fail::Comment => YYJSON_READ_ERROR_INVALID_COMMENT,
            Fail::Garbage => YYJSON_READ_ERROR_UNEXPECTED_CONTENT,
        }
    }

    fn msg(self, msg: *const c_char) -> *const c_char {
        match self {
            Fail::String | Fail::Number => msg,
            Fail::Alloc => crate::util::msg(MSG_MALLOC),
            Fail::TrailingComma => crate::util::msg(MSG_COMMA),
            Fail::LiteralTrue => crate::util::msg(MSG_CHAR_T),
            Fail::LiteralFalse => crate::util::msg(MSG_CHAR_F),
            Fail::LiteralNull => crate::util::msg(MSG_CHAR_N),
            Fail::CharacterVal => crate::util::msg(MSG_CHAR),
            Fail::CharacterArrEnd => crate::util::msg(MSG_ARR_END),
            Fail::CharacterObjKey => crate::util::msg(MSG_OBJ_KEY),
            Fail::CharacterObjSep => crate::util::msg(MSG_OBJ_SEP),
            Fail::CharacterObjEnd => crate::util::msg(MSG_OBJ_END),
            Fail::Comment => crate::util::msg(MSG_COMMENT),
            Fail::Garbage => crate::util::msg(MSG_GARBAGE),
        }
    }
}

/// The body of the `return_err` macro shared by the three root readers.
#[inline(always)]
unsafe fn fill_err(
    err: *mut yyjson_read_err,
    hdr: *mut u8,
    pos: *mut u8,
    eof: *mut u8,
    code: u32,
    message: *const c_char,
    flg: u32,
) {
    if is_truncated_end(hdr, pos, eof, code, flg) {
        (*err).pos = eof as usize - hdr as usize;
        (*err).code = YYJSON_READ_ERROR_UNEXPECTED_END;
        (*err).msg = crate::util::msg(MSG_NOT_END);
    } else {
        (*err).pos = pos as usize - hdr as usize;
        (*err).code = code;
        (*err).msg = message;
    }
}

/// `sizeof(yyjson_doc)` rounded up to whole `yyjson_val`s.
pub const HDR_LEN: usize = {
    let n = size_of::<yyjson_doc>() / size_of::<yyjson_val>();
    n + ((size_of::<yyjson_doc>() % size_of::<yyjson_val>()) > 0) as usize
};

/// `read_root_single`
unsafe fn read_root_single(
    hdr: *mut u8,
    cur0: *mut u8,
    eof: *mut u8,
    alc: yyjson_alc,
    flg: u32,
    err: *mut yyjson_read_err,
) -> *mut yyjson_doc {
    let mut cur = cur0;
    let mut msg: *const c_char = ptr::null();
    let mut raw_end: [u8; 1] = [0];
    let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();

    let alc_num = HDR_LEN + 1; /* single value */
    let val_hdr = alc.malloc(alc_num * size_of::<yyjson_val>()) as *mut yyjson_val;

    let fail: Fail = 'fsm: {
        if val_hdr.is_null() {
            break 'fsm Fail::Alloc;
        }
        let val = val_hdr.add(HDR_LEN);

        'value: {
            if char_is_num(*cur) {
                if read_num(&mut cur, &mut raw_ptr, flg, val, &mut msg) {
                    break 'value;
                }
                break 'fsm Fail::Number;
            }
            if *cur == b'"' {
                if read_str(&mut cur, eof, flg, val, &mut msg) {
                    break 'value;
                }
                break 'fsm Fail::String;
            }
            if *cur == b't' {
                if read_true(&mut cur, val) {
                    break 'value;
                }
                break 'fsm Fail::LiteralTrue;
            }
            if *cur == b'f' {
                if read_false(&mut cur, val) {
                    break 'value;
                }
                break 'fsm Fail::LiteralFalse;
            }
            if *cur == b'n' {
                if read_null(&mut cur, val) {
                    break 'value;
                }
                if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                    && read_nan(&mut cur, &mut raw_ptr, flg, val)
                {
                    break 'value;
                }
                break 'fsm Fail::LiteralNull;
            }
            if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                && read_inf_or_nan(&mut cur, &mut raw_ptr, flg, val)
            {
                break 'value;
            }
            if has_flg(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR) && *cur == b'\'' {
                if read_str_sq(&mut cur, eof, flg, val, &mut msg) {
                    break 'value;
                }
                break 'fsm Fail::String;
            }
            break 'fsm Fail::CharacterVal;
        }

        /* `doc_end`: check invalid contents after json document */
        if cur < eof && !has_flg(flg, YYJSON_READ_STOP_WHEN_DONE) {
            while char_is_space(*cur) {
                cur = cur.add(1);
            }
            if has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                    break 'fsm Fail::Comment;
                }
            }
            if cur < eof {
                break 'fsm Fail::Garbage;
            }
        }

        *raw_ptr = 0;
        let doc = val_hdr as *mut yyjson_doc;
        (*doc).root = val_hdr.add(HDR_LEN);
        (*doc).alc = alc;
        (*doc).dat_read = cur as usize - hdr as usize;
        (*doc).val_read = 1;
        (*doc).str_pool = if has_flg(flg, YYJSON_READ_INSITU) {
            ptr::null_mut()
        } else {
            hdr as *mut c_char
        };
        return doc;
    };

    fill_err(err, hdr, cur, eof, fail.code(), fail.msg(msg), flg);
    if !val_hdr.is_null() {
        alc.free(val_hdr as *mut c_void);
    }
    ptr::null_mut()
}

/// The `goto` labels of `read_root_minify` / `read_root_pretty`.
#[derive(Copy, Clone, PartialEq)]
enum RState {
    ArrBegin,
    ArrValBegin,
    ArrValEnd,
    ArrEnd,
    ObjBegin,
    ObjKeyBegin,
    ObjKeyEnd,
    ObjValBegin,
    ObjValEnd,
    ObjEnd,
    DocEnd,
}

/// `read_root_minify` (`PRETTY == false`) and `read_root_pretty`
/// (`PRETTY == true`).
///
/// The two C functions differ only in the memory-estimate ratio and in the
/// pretty-print fast paths (`if (*cur == '\n') cur++`, the two-space indent
/// skipping, and the `",\n"` / `": "` two-byte matches), so they share one
/// body here and are monomorphized into two specializations.
unsafe fn read_root_ctn<const PRETTY: bool>(
    hdr: *mut u8,
    cur0: *mut u8,
    eof: *mut u8,
    alc: yyjson_alc,
    flg: u32,
    err: *mut yyjson_read_err,
) -> *mut yyjson_doc {
    let mut cur = cur0;
    let mut msg: *const c_char = ptr::null();
    let mut raw_end: [u8; 1] = [0];
    let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();

    let dat_len = if has_flg(flg, YYJSON_READ_STOP_WHEN_DONE) {
        256
    } else {
        eof as usize - cur as usize
    };
    let alc_max = USIZE_MAX / size_of::<yyjson_val>();
    let ratio = if PRETTY {
        YYJSON_READER_ESTIMATED_PRETTY_RATIO
    } else {
        YYJSON_READER_ESTIMATED_MINIFY_RATIO
    };
    let mut alc_len = HDR_LEN + (dat_len / ratio) + 4;
    if alc_len > alc_max {
        alc_len = alc_max;
    }

    let mut val_hdr = alc.malloc(alc_len * size_of::<yyjson_val>()) as *mut yyjson_val;

    let fail: Fail = 'fsm: {
        if val_hdr.is_null() {
            break 'fsm Fail::Alloc;
        }
        /* padding for key-value pair reading */
        let mut val_end = val_hdr.add(alc_len - 2);
        let mut val = val_hdr.add(HDR_LEN);
        let mut ctn = val;
        let mut ctn_len: usize = 0;
        let mut ctn_parent: *mut yyjson_val;

        macro_rules! val_incr {
            () => {{
                val = val.add(1);
                if val >= val_end {
                    let alc_old = alc_len;
                    let val_ofs = val as usize - val_hdr as usize;
                    let ctn_ofs = ctn as usize - val_hdr as usize;
                    alc_len += alc_len / 2;
                    if size_of::<usize>() < 8 && alc_len >= alc_max {
                        break 'fsm Fail::Alloc;
                    }
                    let val_tmp = alc.realloc(
                        val_hdr as *mut c_void,
                        alc_old * size_of::<yyjson_val>(),
                        alc_len * size_of::<yyjson_val>(),
                    ) as *mut yyjson_val;
                    if val_tmp.is_null() {
                        break 'fsm Fail::Alloc;
                    }
                    val = (val_tmp as *mut u8).add(val_ofs) as *mut yyjson_val;
                    ctn = (val_tmp as *mut u8).add(ctn_ofs) as *mut yyjson_val;
                    val_hdr = val_tmp;
                    val_end = val_tmp.add(alc_len - 2);
                }
            }};
        }
        macro_rules! skip_spaces {
            () => {{
                loop {
                    cur = cur.add(1);
                    if !char_is_space(*cur) {
                        break;
                    }
                }
            }};
        }
        macro_rules! skip_indent {
            () => {{
                if PRETTY {
                    loop {
                        let mut i = 0;
                        while i < 16 && byte_load_2(cur) == byte_load_2(b"  ".as_ptr()) {
                            cur = cur.add(2);
                            i += 1;
                        }
                        if i != 16 {
                            break;
                        }
                    }
                }
            }};
        }

        let mut state: RState;
        let c0 = *cur;
        cur = cur.add(1);
        if c0 == b'{' {
            (*ctn).tag = YYJSON_TYPE_OBJ as u64;
            (*ctn).uni.ofs = 0;
            if PRETTY && *cur == b'\n' {
                cur = cur.add(1);
            }
            state = RState::ObjKeyBegin;
        } else {
            (*ctn).tag = YYJSON_TYPE_ARR as u64;
            (*ctn).uni.ofs = 0;
            if PRETTY && *cur == b'\n' {
                cur = cur.add(1);
            }
            state = RState::ArrValBegin;
        }

        loop {
            match state {
                RState::ArrBegin => {
                    /* save current container */
                    (*ctn).tag = (((ctn_len as u64) + 1) << YYJSON_TAG_BIT)
                        | ((*ctn).tag & YYJSON_TAG_MASK as u64);
                    /* create a new array value, save parent container offset */
                    val_incr!();
                    (*val).tag = YYJSON_TYPE_ARR as u64;
                    (*val).uni.ofs = val as usize - ctn as usize;
                    /* push the new array value as current container */
                    ctn = val;
                    ctn_len = 0;
                    if PRETTY && *cur == b'\n' {
                        cur = cur.add(1);
                    }
                    state = RState::ArrValBegin;
                }

                RState::ArrValBegin => {
                    skip_indent!();
                    if *cur == b'{' {
                        cur = cur.add(1);
                        state = RState::ObjBegin;
                        continue;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        state = RState::ArrBegin;
                        continue;
                    }
                    if char_is_num(*cur) {
                        val_incr!();
                        ctn_len += 1;
                        if read_num(&mut cur, &mut raw_ptr, flg, val, &mut msg) {
                            state = RState::ArrValEnd;
                            continue;
                        }
                        break 'fsm Fail::Number;
                    }
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if read_str(&mut cur, eof, flg, val, &mut msg) {
                            state = RState::ArrValEnd;
                            continue;
                        }
                        break 'fsm Fail::String;
                    }
                    if *cur == b't' {
                        val_incr!();
                        ctn_len += 1;
                        if read_true(&mut cur, val) {
                            state = RState::ArrValEnd;
                            continue;
                        }
                        break 'fsm Fail::LiteralTrue;
                    }
                    if *cur == b'f' {
                        val_incr!();
                        ctn_len += 1;
                        if read_false(&mut cur, val) {
                            state = RState::ArrValEnd;
                            continue;
                        }
                        break 'fsm Fail::LiteralFalse;
                    }
                    if *cur == b'n' {
                        val_incr!();
                        ctn_len += 1;
                        if read_null(&mut cur, val) {
                            state = RState::ArrValEnd;
                            continue;
                        }
                        if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                            && read_nan(&mut cur, &mut raw_ptr, flg, val)
                        {
                            state = RState::ArrValEnd;
                            continue;
                        }
                        break 'fsm Fail::LiteralNull;
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            state = RState::ArrEnd;
                            continue;
                        }
                        if has_flg(flg, YYJSON_READ_ALLOW_TRAILING_COMMAS) {
                            state = RState::ArrEnd;
                            continue;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        break 'fsm Fail::TrailingComma;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                        && (*cur == b'i' || *cur == b'I' || *cur == b'N')
                    {
                        val_incr!();
                        ctn_len += 1;
                        if read_inf_or_nan(&mut cur, &mut raw_ptr, flg, val) {
                            state = RState::ArrValEnd;
                            continue;
                        }
                        break 'fsm Fail::CharacterVal;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR) && *cur == b'\'' {
                        val_incr!();
                        ctn_len += 1;
                        if read_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = RState::ArrValEnd;
                            continue;
                        }
                        break 'fsm Fail::String;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                        if skip_trivia(&mut cur, eof, flg) {
                            continue;
                        }
                        if cur == eof {
                            break 'fsm Fail::Comment;
                        }
                    }
                    break 'fsm Fail::CharacterVal;
                }

                RState::ArrValEnd => {
                    if PRETTY && byte_load_2(cur) == byte_load_2(b",\n".as_ptr()) {
                        cur = cur.add(2);
                        state = RState::ArrValBegin;
                        continue;
                    }
                    if *cur == b',' {
                        cur = cur.add(1);
                        state = RState::ArrValBegin;
                        continue;
                    }
                    if *cur == b']' {
                        cur = cur.add(1);
                        state = RState::ArrEnd;
                        continue;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                        if skip_trivia(&mut cur, eof, flg) {
                            continue;
                        }
                        if cur == eof {
                            break 'fsm Fail::Comment;
                        }
                    }
                    break 'fsm Fail::CharacterArrEnd;
                }

                RState::ArrEnd => {
                    /* get parent container */
                    ctn_parent = (ctn as *mut u8).sub((*ctn).uni.ofs) as *mut yyjson_val;
                    /* save the next sibling value offset */
                    (*ctn).uni.ofs = (val as usize - ctn as usize) + size_of::<yyjson_val>();
                    (*ctn).tag = ((ctn_len as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_ARR as u64;
                    if ctn == ctn_parent {
                        state = RState::DocEnd;
                        continue;
                    }
                    /* pop parent as current container */
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    if PRETTY && *cur == b'\n' {
                        cur = cur.add(1);
                    }
                    state = if ((*ctn).tag & YYJSON_TYPE_MASK as u64) == YYJSON_TYPE_OBJ as u64 {
                        RState::ObjValEnd
                    } else {
                        RState::ArrValEnd
                    };
                }

                RState::ObjBegin => {
                    /* push container */
                    (*ctn).tag = (((ctn_len as u64) + 1) << YYJSON_TAG_BIT)
                        | ((*ctn).tag & YYJSON_TAG_MASK as u64);
                    val_incr!();
                    (*val).tag = YYJSON_TYPE_OBJ as u64;
                    /* offset to the parent */
                    (*val).uni.ofs = val as usize - ctn as usize;
                    ctn = val;
                    ctn_len = 0;
                    if PRETTY && *cur == b'\n' {
                        cur = cur.add(1);
                    }
                    state = RState::ObjKeyBegin;
                }

                RState::ObjKeyBegin => {
                    skip_indent!();
                    if *cur == b'"' {
                        val_incr!();
                        ctn_len += 1;
                        if read_str(&mut cur, eof, flg, val, &mut msg) {
                            state = RState::ObjKeyEnd;
                            continue;
                        }
                        break 'fsm Fail::String;
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        if ctn_len == 0 {
                            state = RState::ObjEnd;
                            continue;
                        }
                        if has_flg(flg, YYJSON_READ_ALLOW_TRAILING_COMMAS) {
                            state = RState::ObjEnd;
                            continue;
                        }
                        loop {
                            cur = cur.sub(1);
                            if *cur == b',' {
                                break;
                            }
                        }
                        break 'fsm Fail::TrailingComma;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR) && *cur == b'\'' {
                        val_incr!();
                        ctn_len += 1;
                        if read_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = RState::ObjKeyEnd;
                            continue;
                        }
                        break 'fsm Fail::String;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_UNQUOTED_KEY) && char_is_id_start(*cur) {
                        val_incr!();
                        ctn_len += 1;
                        if read_str_id(&mut cur, eof, flg, &mut raw_ptr, val, &mut msg) {
                            state = RState::ObjKeyEnd;
                            continue;
                        }
                        break 'fsm Fail::String;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                        if skip_trivia(&mut cur, eof, flg) {
                            continue;
                        }
                        if cur == eof {
                            break 'fsm Fail::Comment;
                        }
                    }
                    break 'fsm Fail::CharacterObjKey;
                }

                RState::ObjKeyEnd => {
                    if PRETTY && byte_load_2(cur) == byte_load_2(b": ".as_ptr()) {
                        cur = cur.add(2);
                        state = RState::ObjValBegin;
                        continue;
                    }
                    if *cur == b':' {
                        cur = cur.add(1);
                        state = RState::ObjValBegin;
                        continue;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                        if skip_trivia(&mut cur, eof, flg) {
                            continue;
                        }
                        if cur == eof {
                            break 'fsm Fail::Comment;
                        }
                    }
                    break 'fsm Fail::CharacterObjSep;
                }

                RState::ObjValBegin => {
                    if *cur == b'"' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_str(&mut cur, eof, flg, val, &mut msg) {
                            state = RState::ObjValEnd;
                            continue;
                        }
                        break 'fsm Fail::String;
                    }
                    if char_is_num(*cur) {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_num(&mut cur, &mut raw_ptr, flg, val, &mut msg) {
                            state = RState::ObjValEnd;
                            continue;
                        }
                        break 'fsm Fail::Number;
                    }
                    if *cur == b'{' {
                        cur = cur.add(1);
                        state = RState::ObjBegin;
                        continue;
                    }
                    if *cur == b'[' {
                        cur = cur.add(1);
                        state = RState::ArrBegin;
                        continue;
                    }
                    if *cur == b't' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_true(&mut cur, val) {
                            state = RState::ObjValEnd;
                            continue;
                        }
                        break 'fsm Fail::LiteralTrue;
                    }
                    if *cur == b'f' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_false(&mut cur, val) {
                            state = RState::ObjValEnd;
                            continue;
                        }
                        break 'fsm Fail::LiteralFalse;
                    }
                    if *cur == b'n' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_null(&mut cur, val) {
                            state = RState::ObjValEnd;
                            continue;
                        }
                        if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                            && read_nan(&mut cur, &mut raw_ptr, flg, val)
                        {
                            state = RState::ObjValEnd;
                            continue;
                        }
                        break 'fsm Fail::LiteralNull;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                        && (*cur == b'i' || *cur == b'I' || *cur == b'N')
                    {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_inf_or_nan(&mut cur, &mut raw_ptr, flg, val) {
                            state = RState::ObjValEnd;
                            continue;
                        }
                        break 'fsm Fail::CharacterVal;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_SINGLE_QUOTED_STR) && *cur == b'\'' {
                        val = val.add(1);
                        ctn_len += 1;
                        if read_str_sq(&mut cur, eof, flg, val, &mut msg) {
                            state = RState::ObjValEnd;
                            continue;
                        }
                        break 'fsm Fail::String;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                        if skip_trivia(&mut cur, eof, flg) {
                            continue;
                        }
                        if cur == eof {
                            break 'fsm Fail::Comment;
                        }
                    }
                    break 'fsm Fail::CharacterVal;
                }

                RState::ObjValEnd => {
                    if PRETTY && byte_load_2(cur) == byte_load_2(b",\n".as_ptr()) {
                        cur = cur.add(2);
                        state = RState::ObjKeyBegin;
                        continue;
                    }
                    if *cur == b',' {
                        cur = cur.add(1);
                        state = RState::ObjKeyBegin;
                        continue;
                    }
                    if *cur == b'}' {
                        cur = cur.add(1);
                        state = RState::ObjEnd;
                        continue;
                    }
                    if char_is_space(*cur) {
                        skip_spaces!();
                        continue;
                    }
                    if has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                        if skip_trivia(&mut cur, eof, flg) {
                            continue;
                        }
                        if cur == eof {
                            break 'fsm Fail::Comment;
                        }
                    }
                    break 'fsm Fail::CharacterObjEnd;
                }

                RState::ObjEnd => {
                    /* pop container */
                    ctn_parent = (ctn as *mut u8).sub((*ctn).uni.ofs) as *mut yyjson_val;
                    /* point to the next value */
                    (*ctn).uni.ofs = (val as usize - ctn as usize) + size_of::<yyjson_val>();
                    /* `ctn_len` counts keys *and* values, hence the shift by
                       `YYJSON_TAG_BIT - 1` instead of `YYJSON_TAG_BIT`. */
                    (*ctn).tag =
                        ((ctn_len as u64) << (YYJSON_TAG_BIT - 1)) | YYJSON_TYPE_OBJ as u64;
                    if ctn == ctn_parent {
                        state = RState::DocEnd;
                        continue;
                    }
                    ctn = ctn_parent;
                    ctn_len = ((*ctn).tag >> YYJSON_TAG_BIT) as usize;
                    if PRETTY && *cur == b'\n' {
                        cur = cur.add(1);
                    }
                    state = if ((*ctn).tag & YYJSON_TYPE_MASK as u64) == YYJSON_TYPE_OBJ as u64 {
                        RState::ObjValEnd
                    } else {
                        RState::ArrValEnd
                    };
                }

                RState::DocEnd => {
                    /* check invalid contents after json document */
                    if cur < eof && !has_flg(flg, YYJSON_READ_STOP_WHEN_DONE) {
                        while char_is_space(*cur) {
                            cur = cur.add(1);
                        }
                        if has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
                            if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                                break 'fsm Fail::Comment;
                            }
                        }
                        if cur < eof {
                            break 'fsm Fail::Garbage;
                        }
                    }

                    *raw_ptr = 0;
                    let doc = val_hdr as *mut yyjson_doc;
                    (*doc).root = val_hdr.add(HDR_LEN);
                    (*doc).alc = alc;
                    (*doc).dat_read = cur as usize - hdr as usize;
                    (*doc).val_read =
                        ((val as usize - (*doc).root as usize) / size_of::<yyjson_val>()) + 1;
                    (*doc).str_pool = if has_flg(flg, YYJSON_READ_INSITU) {
                        ptr::null_mut()
                    } else {
                        hdr as *mut c_char
                    };
                    return doc;
                }
            }
        }
    };

    fill_err(err, hdr, cur, eof, fail.code(), fail.msg(msg), flg);
    if !val_hdr.is_null() {
        alc.free(val_hdr as *mut c_void);
    }
    ptr::null_mut()
}

/* ------------------------------------------------------- public reader */

#[no_mangle]
pub unsafe extern "C" fn yyjson_read_opts(
    dat: *mut c_char,
    len: usize,
    flg: yyjson_read_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_read_err,
) -> *mut yyjson_doc {
    let mut tmp_err = yyjson_read_err {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_read_err
    } else {
        err
    };
    let mut hdr: *mut u8 = ptr::null_mut();
    let eof: *mut u8;
    let mut cur: *mut u8;

    macro_rules! return_err {
        ($pos:expr, $code:expr, $m:expr) => {{
            (*err).pos = $pos;
            (*err).msg = $m;
            (*err).code = $code;
            if !has_flg(flg, YYJSON_READ_INSITU) && !hdr.is_null() {
                alc.free(hdr as *mut c_void);
            }
            return ptr::null_mut();
        }};
    }

    /* validate input parameters */
    if dat.is_null() {
        return_err!(
            0,
            YYJSON_READ_ERROR_INVALID_PARAMETER,
            b"input data is NULL\0".as_ptr() as *const c_char
        );
    }
    if len == 0 {
        return_err!(
            0,
            YYJSON_READ_ERROR_INVALID_PARAMETER,
            b"input length is 0\0".as_ptr() as *const c_char
        );
    }

    /* add 4-byte zero padding for input data if necessary */
    if has_flg(flg, YYJSON_READ_INSITU) {
        hdr = dat as *mut u8;
        eof = (dat as *mut u8).add(len);
        cur = dat as *mut u8;
    } else {
        if len >= USIZE_MAX - YYJSON_PADDING_SIZE {
            return_err!(
                0,
                YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                crate::util::msg(MSG_MALLOC)
            );
        }
        hdr = alc.malloc(len + YYJSON_PADDING_SIZE) as *mut u8;
        if hdr.is_null() {
            return_err!(
                0,
                YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                crate::util::msg(MSG_MALLOC)
            );
        }
        eof = hdr.add(len);
        cur = hdr;
        mem::copy(hdr, dat as *const u8, len);
    }
    mem::set(eof, 0, YYJSON_PADDING_SIZE);

    if has_flg(flg, YYJSON_READ_ALLOW_BOM) && len >= 3 && is_utf8_bom(cur) {
        cur = cur.add(3);
    }

    /* skip empty contents before json document */
    if !char_is_ctn(*cur) {
        while char_is_space(*cur) {
            cur = cur.add(1);
        }
        if !char_is_ctn(*cur) && has_flg(flg, YYJSON_READ_ALLOW_TRIVIA) && char_is_trivia(*cur) {
            if !skip_trivia(&mut cur, eof, flg) && cur == eof {
                return_err!(
                    cur as usize - hdr as usize,
                    YYJSON_READ_ERROR_INVALID_COMMENT,
                    crate::util::msg(MSG_COMMENT)
                );
            }
        }
        if cur >= eof {
            return_err!(
                0,
                YYJSON_READ_ERROR_EMPTY_CONTENT,
                b"input data is empty\0".as_ptr() as *const c_char
            );
        }
    }

    /* read json document */
    let doc = if char_is_ctn(*cur) {
        if char_is_space(*cur.add(1)) && char_is_space(*cur.add(2)) {
            read_root_ctn::<true>(hdr, cur, eof, alc, flg, err)
        } else {
            read_root_ctn::<false>(hdr, cur, eof, alc, flg, err)
        }
    } else {
        read_root_single(hdr, cur, eof, alc, flg, err)
    };

    /* check result */
    if !doc.is_null() {
        mem::set(err as *mut u8, 0, size_of::<yyjson_read_err>());
    } else {
        /* RFC 8259: JSON text MUST be encoded using UTF-8 */
        if (*err).pos == 0 && (*err).code != YYJSON_READ_ERROR_MEMORY_ALLOCATION {
            if is_utf8_bom(hdr) {
                (*err).msg = crate::util::msg(MSG_ERR_BOM);
            } else if len >= 4 && is_utf32_bom(hdr) {
                (*err).msg = crate::util::msg(MSG_ERR_UTF32);
            } else if len >= 2 && is_utf16_bom(hdr) {
                (*err).msg = crate::util::msg(MSG_ERR_UTF16);
            }
        }
        if !has_flg(flg, YYJSON_READ_INSITU) {
            alc.free(hdr as *mut c_void);
        }
    }
    doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_read_file(
    path: *const c_char,
    flg: yyjson_read_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_read_err,
) -> *mut yyjson_doc {
    let mut tmp_err = yyjson_read_err {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_read_err
    } else {
        err
    };

    if path.is_null() {
        (*err).pos = 0;
        (*err).msg = b"input path is NULL\0".as_ptr() as *const c_char;
        (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
        return ptr::null_mut();
    }

    let fp = file::fopen_readonly(path);
    if fp.is_null() {
        (*err).pos = 0;
        (*err).msg = crate::util::msg(MSG_FREAD);
        (*err).code = YYJSON_READ_ERROR_FILE_OPEN;
        return ptr::null_mut();
    }

    let doc = yyjson_read_fp(fp, flg, alc_ptr, err);
    fclose(fp);
    doc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_read_fp(
    fp: *mut FILE,
    flg: yyjson_read_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_read_err,
) -> *mut yyjson_doc {
    let mut tmp_err = yyjson_read_err {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_read_err
    } else {
        err
    };

    let mut buf: *mut c_void = ptr::null_mut();
    let mut buf_size: usize;
    let mut dat_size: usize = 0;

    macro_rules! return_err {
        ($code:expr, $m:expr) => {{
            (*err).pos = 0;
            (*err).msg = $m;
            (*err).code = $code;
            if !buf.is_null() {
                alc.free(buf);
            }
            return ptr::null_mut();
        }};
    }

    if fp.is_null() {
        return_err!(
            YYJSON_READ_ERROR_INVALID_PARAMETER,
            b"input file is NULL\0".as_ptr() as *const c_char
        );
    }

    /* get current position */
    let mut file_size: core::ffi::c_long = 0;
    let file_pos = ftell(fp);
    if file_pos != -1 {
        /* get total file size, may fail */
        if fseek(fp, 0, SEEK_END) == 0 {
            file_size = ftell(fp);
            if file_size == -1 {
                file_size = 0;
            }
        }
        /* reset to original position, may fail */
        if fseek(fp, file_pos, SEEK_SET) != 0 {
            file_size = 0;
        }
        /* get file size from current position to end */
        if file_size > 0 {
            file_size -= file_pos;
        }
    }

    if file_size > 0 {
        /* read the entire file in one call */
        dat_size = file_size as usize;
        buf_size = dat_size + YYJSON_PADDING_SIZE;
        buf = alc.malloc(buf_size);
        if buf.is_null() {
            return_err!(
                YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                crate::util::msg(MSG_MALLOC)
            );
        }
        if file::fread_safe(buf, dat_size, fp) != dat_size {
            return_err!(YYJSON_READ_ERROR_FILE_READ, crate::util::msg(MSG_FREAD));
        }
    } else {
        /* failed to get file size, read it as a stream */
        let chunk_min: usize = 64;
        let chunk_max: usize = 512 * 1024 * 1024;
        let mut chunk_now = chunk_min;
        buf_size = YYJSON_PADDING_SIZE;
        loop {
            if buf_size + chunk_now < buf_size {
                return_err!(
                    YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                    crate::util::msg(MSG_MALLOC)
                );
            }
            buf_size += chunk_now;
            if buf.is_null() {
                buf = alc.malloc(buf_size);
                if buf.is_null() {
                    return_err!(
                        YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                        crate::util::msg(MSG_MALLOC)
                    );
                }
            } else {
                let tmp = alc.realloc(buf, buf_size - chunk_now, buf_size);
                if tmp.is_null() {
                    return_err!(
                        YYJSON_READ_ERROR_MEMORY_ALLOCATION,
                        crate::util::msg(MSG_MALLOC)
                    );
                }
                buf = tmp;
            }
            let tmp = (buf as *mut u8).add(buf_size - YYJSON_PADDING_SIZE - chunk_now);
            let read_size = file::fread_safe(tmp as *mut c_void, chunk_now, fp);
            dat_size += read_size;
            if read_size != chunk_now {
                if ferror(fp) != 0 {
                    return_err!(YYJSON_READ_ERROR_FILE_READ, crate::util::msg(MSG_FREAD));
                }
                break;
            }
            chunk_now *= 2;
            if chunk_now > chunk_max {
                chunk_now = chunk_max;
            }
        }
    }

    /* read JSON */
    mem::set((buf as *mut u8).add(dat_size), 0, YYJSON_PADDING_SIZE);
    let flg = flg | YYJSON_READ_INSITU;
    let doc = yyjson_read_opts(buf as *mut c_char, dat_size, flg, &alc, err);
    if !doc.is_null() {
        (*doc).str_pool = buf as *mut c_char;
        doc
    } else {
        alc.free(buf);
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_read_number(
    dat: *const c_char,
    val: *mut yyjson_val,
    flg: yyjson_read_flag,
    _alc: *const yyjson_alc,
    err: *mut yyjson_read_err,
) -> *const c_char {
    let mut tmp_err = yyjson_read_err {
        code: 0,
        msg: ptr::null(),
        pos: 0,
    };
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_read_err
    } else {
        err
    };
    let hdr = dat as *mut u8;
    let mut cur = hdr;
    let mut raw_end: [u8; 1] = [0];
    let mut raw_ptr: *mut u8 = raw_end.as_mut_ptr();
    let mut msg: *const c_char = ptr::null();

    macro_rules! return_err {
        ($pos:expr, $code:expr, $m:expr) => {{
            (*err).pos = if $pos > hdr {
                $pos as usize - hdr as usize
            } else {
                0
            };
            (*err).msg = $m;
            (*err).code = $code;
            return ptr::null();
        }};
    }

    if dat.is_null() {
        return_err!(
            cur,
            YYJSON_READ_ERROR_INVALID_PARAMETER,
            b"input data is NULL\0".as_ptr() as *const c_char
        );
    }
    if val.is_null() {
        return_err!(
            cur,
            YYJSON_READ_ERROR_INVALID_PARAMETER,
            b"output value is NULL\0".as_ptr() as *const c_char
        );
    }

    if !read_num(&mut cur, &mut raw_ptr, flg, val, &mut msg) {
        return_err!(cur, YYJSON_READ_ERROR_INVALID_NUMBER, msg);
    }
    cur as *const c_char
}
