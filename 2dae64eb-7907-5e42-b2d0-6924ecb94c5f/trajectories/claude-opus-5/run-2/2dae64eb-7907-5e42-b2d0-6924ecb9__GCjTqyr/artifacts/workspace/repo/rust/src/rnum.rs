//! JSON number reader.
//!
//! Transliteration of the "BigInt", "Diy Floating Point" and "Number Reader"
//! sections of the original `src/yyjson.c` (the `!YYJSON_DISABLE_FAST_FP_CONV`
//! branch, which is what the default build compiles).

use core::ffi::c_char;
use core::ptr;

use crate::tables::U64_POW10_TABLE;
use crate::types::*;
use crate::util::*;

/* ===== bigint =========================================================== */

/// `bigint`: unsigned arbitrarily large integer (58 chunks are enough).
pub struct BigInt {
    pub used: u32,
    pub bits: [u64; 64],
}

impl BigInt {
    #[inline]
    pub fn new() -> BigInt {
        BigInt {
            used: 0,
            bits: [0u64; 64],
        }
    }
}

#[inline]
fn bigint_add_u64(big: &mut BigInt, val: u64) {
    let num = big.bits[0];
    let add = num.wrapping_add(val);
    big.bits[0] = add;
    if add >= num || add >= val {
        return;
    }
    let max = big.used;
    let mut idx = 1u32;
    while idx < max {
        if big.bits[idx as usize] != U64_MAX {
            big.bits[idx as usize] += 1;
            return;
        }
        big.bits[idx as usize] = 0;
        idx += 1;
    }
    big.bits[big.used as usize] = 1;
    big.used += 1;
}

#[inline]
fn bigint_mul_u64(big: &mut BigInt, val: u64) {
    let max = big.used;
    let mut idx = 0u32;
    let mut carry = 0u64;
    while idx < max {
        if big.bits[idx as usize] != 0 {
            break;
        }
        idx += 1;
    }
    while idx < max {
        let (hi, lo) = u128_mul_add(big.bits[idx as usize], val, carry);
        big.bits[idx as usize] = lo;
        carry = hi;
        idx += 1;
    }
    if carry != 0 {
        big.bits[big.used as usize] = carry;
        big.used += 1;
    }
}

#[inline]
fn bigint_mul_pow2(big: &mut BigInt, exp: u32) {
    let shft = exp % 64;
    let mut move_ = exp / 64;
    let mut idx = big.used;
    if shft == 0 {
        while idx > 0 {
            big.bits[(idx + move_ - 1) as usize] = big.bits[(idx - 1) as usize];
            idx -= 1;
        }
        big.used += move_;
        while move_ != 0 {
            move_ -= 1;
            big.bits[move_ as usize] = 0;
        }
    } else {
        big.bits[idx as usize] = 0;
        while idx > 0 {
            let mut num = big.bits[idx as usize] << shft;
            num |= big.bits[(idx - 1) as usize] >> (64 - shft);
            big.bits[(idx + move_) as usize] = num;
            idx -= 1;
        }
        big.bits[move_ as usize] = big.bits[0] << shft;
        big.used += move_ + (big.bits[(big.used + move_) as usize] > 0) as u32;
        while move_ != 0 {
            move_ -= 1;
            big.bits[move_ as usize] = 0;
        }
    }
}

#[inline]
fn bigint_mul_pow10(big: &mut BigInt, exp: i32) {
    let mut exp = exp;
    while exp >= U64_POW10_MAX_EXACT_EXP {
        bigint_mul_u64(big, U64_POW10_TABLE[U64_POW10_MAX_EXACT_EXP as usize]);
        exp -= U64_POW10_MAX_EXACT_EXP;
    }
    if exp != 0 {
        bigint_mul_u64(big, U64_POW10_TABLE[exp as usize]);
    }
}

#[inline]
fn bigint_cmp(a: &BigInt, b: &BigInt) -> i32 {
    if a.used < b.used {
        return -1;
    }
    if a.used > b.used {
        return 1;
    }
    let mut idx = a.used;
    while idx > 0 {
        idx -= 1;
        let av = a.bits[idx as usize];
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

#[inline]
fn bigint_set_u64(big: &mut BigInt, val: u64) {
    big.used = 1;
    big.bits[0] = val;
}

unsafe fn bigint_set_buf(
    big: &mut BigInt,
    sig: u64,
    exp: *mut i32,
    sig_cut: *mut u8,
    sig_end: *mut u8,
    dot_pos: *mut u8,
) {
    if sig_cut.is_null() {
        bigint_set_u64(big, sig);
        return;
    }

    let hdr = sig_cut;
    let mut cur = hdr;
    let mut len = 0u32;
    let mut val = 0u64;
    let mut dig_big_cut = false;
    let mut sig_end = sig_end;
    let has_dot = (hdr < dot_pos) & (dot_pos < sig_end);
    let mut dig_len_total =
        U64_SAFE_DIG as usize + (sig_end.offset_from(hdr) as usize) - has_dot as usize;

    let mut sig = sig;
    sig -= (*sig_cut >= b'5') as u64;
    if dig_len_total > F64_MAX_DEC_DIG as usize {
        dig_big_cut = true;
        sig_end = sig_end.sub(dig_len_total - (F64_MAX_DEC_DIG as usize + 1));
        sig_end = sig_end.sub((dot_pos.add(1) == sig_end) as usize);
        dig_len_total = F64_MAX_DEC_DIG as usize + 1;
    }
    *exp -= dig_len_total as i32 - U64_SAFE_DIG;

    big.used = 1;
    big.bits[0] = sig;
    while cur < sig_end {
        if cur != dot_pos {
            val = val * 10 + (*cur - b'0') as u64;
            cur = cur.add(1);
            len += 1;
            if cur == sig_end && dig_big_cut {
                /* The last digit must be non-zero; set it to '1' so that
                   rounding stays correct. */
                val = val - (val % 10) + 1;
            }
            if len == U64_SAFE_DIG as u32 || cur == sig_end {
                bigint_mul_pow10(big, len as i32);
                bigint_add_u64(big, val);
                val = 0;
                len = 0;
            }
        } else {
            cur = cur.add(1);
        }
    }
}

/* ===== diy floating point =============================================== */

#[derive(Clone, Copy)]
struct DiyFp {
    sig: u64,
    exp: i32,
}

#[inline]
fn diy_fp_get_cached_pow10(exp10: i32) -> DiyFp {
    let (sig, sig_ext) = pow10_table_get_sig(exp10);
    let exp = pow10_table_get_exp(exp10);
    DiyFp {
        sig: sig.wrapping_add(sig_ext >> 63),
        exp,
    }
}

#[inline]
fn diy_fp_mul(fp: DiyFp, fp2: DiyFp) -> DiyFp {
    let (hi, lo) = u128_mul(fp.sig, fp2.sig);
    DiyFp {
        sig: hi.wrapping_add(lo >> 63),
        exp: fp.exp + fp2.exp + 64,
    }
}

#[inline]
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
    exp += F64_BITS - F64_SIG_FULL_BITS;
    exp += F64_SIG_BITS;

    if exp >= F64_MAX_BIN_EXP {
        F64_BITS_INF
    } else if exp >= F64_MIN_BIN_EXP - 1 {
        exp += F64_EXP_BIAS;
        ((exp as u64) << F64_SIG_BITS) | (sig & F64_SIG_MASK)
    } else if exp >= F64_MIN_BIN_EXP - F64_SIG_FULL_BITS {
        sig >> (F64_MIN_BIN_EXP - exp - 1)
    } else {
        0
    }
}

/* ===== flag helpers ===================================================== */

#[inline(always)]
pub fn has_rflag(flg: u32, chk: u32) -> bool {
    (flg & chk) != 0
}

/* ===== literals ========================================================= */

/// `read_true`
#[inline]
pub unsafe fn read_true(p: *mut *mut u8, val: *mut YyjsonVal) -> bool {
    let cur = *p;
    if byte_match_4(cur, b"true") {
        (*val).tag = (YYJSON_TYPE_BOOL | YYJSON_SUBTYPE_TRUE) as u64;
        *p = cur.add(4);
        return true;
    }
    false
}

/// `read_false`
#[inline]
pub unsafe fn read_false(p: *mut *mut u8, val: *mut YyjsonVal) -> bool {
    let cur = *p;
    if byte_match_4(cur.add(1), b"alse") {
        (*val).tag = (YYJSON_TYPE_BOOL | YYJSON_SUBTYPE_FALSE) as u64;
        *p = cur.add(5);
        return true;
    }
    false
}

/// `read_null`
#[inline]
pub unsafe fn read_null(p: *mut *mut u8, val: *mut YyjsonVal) -> bool {
    let cur = *p;
    if byte_match_4(cur, b"null") {
        (*val).tag = YYJSON_TYPE_NULL as u64;
        *p = cur.add(4);
        return true;
    }
    false
}

/// `read_inf`
pub unsafe fn read_inf(
    p: *mut *mut u8,
    pre: *mut *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
) -> bool {
    let hdr = *p;
    let mut cur = *p;
    let sign = *cur == b'-';
    if *cur == b'+' && !has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
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
        *p = cur;
        if has_rflag(flg, YYJSON_READ_NUMBER_AS_RAW) {
            **pre = 0;
            *pre = cur;
            (*val).tag =
                ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
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
pub unsafe fn read_nan(
    p: *mut *mut u8,
    pre: *mut *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
) -> bool {
    let hdr = *p;
    let mut cur = *p;
    let sign = *cur == b'-';
    if *cur == b'+' && !has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
        return false;
    }
    cur = cur.add(char_is_sign(*cur) as usize);
    if char_to_lower(*cur) == b'n'
        && char_to_lower(*cur.add(1)) == b'a'
        && char_to_lower(*cur.add(2)) == b'n'
    {
        cur = cur.add(3);
        *p = cur;
        if has_rflag(flg, YYJSON_READ_NUMBER_AS_RAW) {
            **pre = 0;
            *pre = cur;
            (*val).tag =
                ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
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
#[inline]
pub unsafe fn read_inf_or_nan(
    p: *mut *mut u8,
    pre: *mut *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
) -> bool {
    if read_inf(p, pre, flg, val) {
        return true;
    }
    if read_nan(p, pre, flg, val) {
        return true;
    }
    false
}

/* ===== raw / hex readers ================================================ */

/// `read_num_raw`
unsafe fn read_num_raw(
    p: *mut *mut u8,
    pre: *mut *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
    msg: *mut *const c_char,
) -> bool {
    let hdr = *p;
    let mut cur = *p;

    macro_rules! return_err {
        ($pos:expr, $m:expr) => {{
            *msg = cstr!($m);
            *p = $pos;
            return false;
        }};
    }
    macro_rules! return_raw {
        () => {{
            (*val).tag =
                ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
            (*val).uni.str_ = hdr as *const c_char;
            **pre = 0;
            *pre = cur;
            *p = cur;
            return true;
        }};
    }

    cur = cur.add((*cur == b'-') as usize);

    let mut goto_read_double = false;
    while !char_is_digit(*cur) {
        if has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
            if *cur == b'+' && cur == hdr {
                cur = cur.add(1);
                continue;
            }
            if *cur == b'.' && char_is_digit(*cur.add(1)) {
                goto_read_double = true;
                break;
            }
        }
        if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN) && read_inf_or_nan(p, pre, flg, val) {
            return true;
        }
        return_err!(cur, "no digit after sign");
    }

    if !goto_read_double {
        if *cur == b'0' {
            cur = cur.add(1);
            if char_is_digit(*cur) {
                return_err!(cur.sub(1), "number with leading zero is not allowed");
            }
            if !char_is_fp(*cur) {
                if has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER) && char_to_lower(*cur) == b'x' {
                    cur = cur.add(1);
                    if !char_is_hex(*cur) {
                        return_err!(cur, "invalid hex number");
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

    /* read_double: */
    if *cur == b'.' {
        cur = cur.add(1);
        if !char_is_digit(*cur) {
            if has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER) {
                if !char_is_exp(*cur) {
                    return_raw!();
                }
            } else {
                return_err!(cur, "no digit after decimal point");
            }
        }
        while char_is_digit(*cur) {
            cur = cur.add(1);
        }
    }

    if char_is_exp(*cur) {
        cur = cur.add(1 + char_is_sign(*cur.add(1)) as usize);
        let c = *cur;
        cur = cur.add(1);
        if !char_is_digit(c) {
            return_err!(cur, "no digit after exponent sign");
        }
        while char_is_digit(*cur) {
            cur = cur.add(1);
        }
    }

    return_raw!();
}

/// `read_num_hex`
unsafe fn read_num_hex(
    p: *mut *mut u8,
    pre: *mut *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
    msg: *mut *const c_char,
) -> bool {
    let hdr = *p;
    let mut cur = *p;
    let mut sig = 0u64;
    let mut i = 0usize;

    let sign = *cur == b'-';
    cur = cur.add((*cur == b'-' || *cur == b'+') as usize + 2);

    while i < 16 {
        let c = crate::tables::HEX_CONV_TABLE[*cur.add(i) as usize];
        if c == 0xF0 {
            break;
        }
        sig <<= 4;
        sig |= c as u64;
        i += 1;
    }

    if i == 0 {
        *msg = cstr!("invalid hex number");
        return false;
    }

    if i == 16 && (char_is_hex(*cur.add(16)) || (sign && sig > (1u64 << 63))) {
        if !has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW) {
            *msg = cstr!("hex number overflow");
            return false;
        }
        cur = cur.add(16);
        while char_is_hex(*cur) {
            cur = cur.add(1);
        }
        **pre = 0;
        (*val).tag = ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
        (*val).uni.str_ = hdr as *const c_char;
        *pre = cur;
        *p = cur;
        return true;
    }

    (*val).tag = YYJSON_TYPE_NUM as u64 | ((sign as u64) << 3);
    (*val).uni.u64_ = if sign { (!sig).wrapping_add(1) } else { sig };
    *p = cur.add(i);
    true
}

/* ===== number reader ==================================================== */

/// States of the hand-written FSM that the C builds out of `goto` labels.
#[derive(PartialEq, Clone, Copy)]
enum St {
    DigiIntgMore,
    DigiFracMore,
    DigiFracEnd,
    DigiExpMore,
    DigiExpFinish,
    DigiFinish,
}

enum FracOut {
    /// `digi_stop_i` was reached at index `i`.
    Stop(usize),
    /// All unrolled steps consumed digits (`digi_frac_18` fell through).
    Through,
}

/// The `digi_frac_i` chain: reads `cur[i+1] ..= cur[19]` as fraction digits.
#[inline]
unsafe fn digi_frac(cur: *mut u8, sig: &mut u64, start: usize) -> FracOut {
    for i in start..=18usize {
        let num = (*cur.add(i + 1) as u64).wrapping_sub(b'0' as u64);
        if num <= 9 {
            *sig = num + *sig * 10;
        } else {
            return FracOut::Stop(i);
        }
    }
    FracOut::Through
}

/// `read_num`
pub unsafe fn read_num(
    p: *mut *mut u8,
    pre: *mut *mut u8,
    flg: u32,
    val: *mut YyjsonVal,
    msg: *mut *const c_char,
) -> bool {
    let hdr = *p;
    let mut cur = *p;

    let mut sig_cut: *mut u8 = ptr::null_mut();
    let mut sig_end: *mut u8 = ptr::null_mut();
    let mut dot_pos: *mut u8 = ptr::null_mut();

    let mut sig: u64 = 0;
    let mut exp: i32 = 0;

    let mut exp_sig: i64 = 0;
    let mut exp_lit: i64 = 0;
    let mut num: u64;
    let mut tmp: *mut u8;

    macro_rules! return_err {
        ($pos:expr, $m:expr) => {{
            *msg = cstr!($m);
            *p = $pos;
            return false;
        }};
    }

    if has_rflag(flg, YYJSON_READ_NUMBER_AS_RAW) {
        return read_num_raw(p, pre, flg, val, msg);
    }

    let sign: bool = *hdr == b'-';
    cur = cur.add(sign as usize);

    macro_rules! return_0 {
        () => {{
            (*val).tag = YYJSON_TYPE_NUM as u64 | ((sign as u64) << 3);
            (*val).uni.u64_ = 0;
            *p = cur;
            return true;
        }};
    }
    macro_rules! return_i64 {
        ($v:expr) => {{
            let v: u64 = $v;
            (*val).tag = YYJSON_TYPE_NUM as u64 | ((sign as u64) << 3);
            (*val).uni.u64_ = if sign { (!v).wrapping_add(1) } else { v };
            *p = cur;
            return true;
        }};
    }
    macro_rules! return_f64 {
        ($v:expr) => {{
            let v: f64 = $v;
            (*val).tag = (YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL) as u64;
            (*val).uni.f64_ = if sign { -v } else { v };
            *p = cur;
            return true;
        }};
    }
    macro_rules! return_f64_bin {
        ($v:expr) => {{
            let v: u64 = $v;
            (*val).tag = (YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL) as u64;
            (*val).uni.u64_ = ((sign as u64) << 63) | v;
            *p = cur;
            return true;
        }};
    }
    macro_rules! return_raw {
        () => {{
            **pre = 0;
            (*val).tag =
                ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT) | YYJSON_TYPE_RAW as u64;
            (*val).uni.str_ = hdr as *const c_char;
            *pre = cur;
            *p = cur;
            return true;
        }};
    }
    macro_rules! return_inf {
        () => {{
            if has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW) {
                return_raw!();
            }
            if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN) {
                return_f64_bin!(F64_BITS_INF);
            } else {
                return_err!(hdr, "number is infinity when parsed as double");
            }
        }};
    }
    macro_rules! frac_from {
        ($i:expr) => {
            match digi_frac(cur, &mut sig, $i) {
                FracOut::Stop(j) => {
                    cur = cur.add(j + 1);
                    St::DigiFracEnd
                }
                FracOut::Through => {
                    cur = cur.add(20);
                    if !char_is_digit(*cur) {
                        St::DigiFracEnd
                    } else {
                        St::DigiFracMore
                    }
                }
            }
        };
    }

    let ext = has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER);

    let mut state: St = 'lead: {
        /* begin with a leading zero or non-digit */
        let mut in_zero_body = false;
        let mut leading_dot = false;
        loop {
            if char_is_nonzero(*cur) {
                break;
            }
            if *cur != b'0' {
                /* non-digit char */
                if ext {
                    if *cur == b'+' && cur == hdr {
                        /* leading `+` sign */
                        cur = cur.add(1);
                        continue;
                    }
                    if *cur == b'.' && char_is_digit(*cur.add(1)) {
                        /* e.g. '.123' */
                        in_zero_body = true;
                        leading_dot = true;
                        break;
                    }
                }
                if has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN)
                    && read_inf_or_nan(p, pre, flg, val)
                {
                    return true;
                }
                return_err!(cur, "no digit after sign");
            }
            /* begin with 0 */
            cur = cur.add(1);
            if !char_is_digit_or_fp(*cur) {
                if ext && char_to_lower(*cur) == b'x' {
                    /* hex */
                    return read_num_hex(p, pre, flg, val, msg);
                }
                return_0!();
            }
            in_zero_body = true;
            leading_dot = *cur == b'.';
            break;
        }

        if in_zero_body {
            if leading_dot {
                dot_pos = cur;
                cur = cur.add(1);
                if !char_is_digit(*cur) {
                    if ext {
                        if char_is_exp(*cur) {
                            break 'lead St::DigiExpMore;
                        } else {
                            return_f64_bin!(0);
                        }
                    }
                    return_err!(cur, "no digit after decimal point");
                }
                while *cur == b'0' {
                    cur = cur.add(1);
                }
                if char_is_digit(*cur) {
                    /* first non-zero digit after decimal point */
                    sig = (*cur - b'0') as u64;
                    cur = cur.sub(1);
                    break 'lead frac_from!(1);
                }
            }
            if char_is_digit(*cur) {
                return_err!(cur.sub(1), "number with leading zero is not allowed");
            }
            if char_is_exp(*cur) {
                /* 0 with any exponent is still 0 */
                cur = cur.add(1 + char_is_sign(*cur.add(1)) as usize);
                if !char_is_digit(*cur) {
                    return_err!(cur, "no digit after exponent sign");
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

        let mut sepr_i = 0usize;
        for i in 1..=18usize {
            num = (*cur.add(i) as u64).wrapping_sub(b'0' as u64);
            if num <= 9 {
                sig = num + sig * 10;
            } else {
                sepr_i = i;
                break;
            }
        }

        if sepr_i == 0 {
            cur = cur.add(19); /* skip continuous 19 digits */
            if !char_is_digit_or_fp(*cur) {
                /* this number is an integer consisting of 19 digits */
                if sign && (sig > (1u64 << 63)) {
                    /* overflow */
                    if has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW) {
                        return_raw!();
                    }
                    return_f64!(sig as f64);
                }
                return_i64!(sig);
            }
            break 'lead St::DigiIntgMore;
        }

        /* digi_sepr_i: process the first non-digit character */
        let i = sepr_i;
        if !char_is_fp(*cur.add(i)) {
            cur = cur.add(i);
            return_i64!(sig);
        }
        dot_pos = cur.add(i);
        if *cur.add(i) == b'.' {
            break 'lead frac_from!(i);
        }
        cur = cur.add(i);
        sig_end = cur;
        St::DigiExpMore
    };

    /* digi_intg_more: read more digits in the integral part */
    if state == St::DigiIntgMore {
        if char_is_digit(*cur) && !char_is_digit_or_fp(*cur.add(1)) {
            /* this number is an integer consisting of 20 digits */
            num = (*cur - b'0') as u64;
            if (sig < (U64_MAX / 10)) || (sig == (U64_MAX / 10) && num <= (U64_MAX % 10)) {
                sig = num + sig * 10;
                cur = cur.add(1);
                /* convert to double if overflow */
                if sign {
                    if has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW) {
                        return_raw!();
                    }
                    return_f64!(sig as f64);
                }
                return_i64!(sig);
            }
        }

        if char_is_exp(*cur) {
            dot_pos = cur;
            state = St::DigiExpMore;
        } else {
            state = St::DigiFracMore;
            if *cur == b'.' {
                dot_pos = cur;
                cur = cur.add(1);
                if !char_is_digit(*cur) {
                    if ext {
                        state = St::DigiFracEnd;
                    } else {
                        return_err!(cur, "no digit after decimal point");
                    }
                }
            }
        }
    }

    /* digi_frac_more: read more digits in the fraction part */
    if state == St::DigiFracMore {
        sig_cut = cur; /* too large to fit in u64, excess digits need to be cut */
        sig += (*cur >= b'5') as u64; /* round */
        loop {
            cur = cur.add(1);
            if !char_is_digit(*cur) {
                break;
            }
        }
        if dot_pos.is_null() {
            if !char_is_fp(*cur) && has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW) {
                return_raw!(); /* it's a large integer */
            }
            dot_pos = cur;
            if *cur == b'.' {
                cur = cur.add(1);
                if !char_is_digit(*cur) && !ext {
                    return_err!(cur, "no digit after decimal point");
                }
                while char_is_digit(*cur) {
                    cur = cur.add(1);
                }
            }
        }
        exp_sig = dot_pos.offset_from(sig_cut) as i64;
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
            St::DigiExpMore
        } else {
            St::DigiExpFinish
        };
    }

    /* digi_frac_end: fraction part end */
    if state == St::DigiFracEnd {
        if dot_pos.add(1) == cur && !ext {
            return_err!(cur, "no digit after decimal point");
        }
        sig_end = cur;
        exp_sig = -(((cur.offset_from(dot_pos) as usize) - 1) as i64);
        if !char_is_exp(*cur) {
            if exp_sig < (F64_MIN_DEC_EXP - 19) as i64 {
                return_f64_bin!(0); /* underflow */
            }
            exp = exp_sig as i32;
            state = St::DigiFinish;
        } else {
            state = St::DigiExpMore;
        }
    }

    /* digi_exp_more: read the exponent part */
    if state == St::DigiExpMore {
        cur = cur.add(1);
        let exp_sign = *cur == b'-';
        cur = cur.add(char_is_sign(*cur) as usize);
        if !char_is_digit(*cur) {
            return_err!(cur, "no digit after exponent sign");
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
        if cur.offset_from(tmp) >= U64_SAFE_DIG as isize {
            if exp_sign {
                return_f64_bin!(0); /* underflow */
            } else {
                return_inf!(); /* overflow */
            }
        }
        exp_sig += if exp_sign { -exp_lit } else { exp_lit };
        state = St::DigiExpFinish;
    }

    /* digi_exp_finish: validate the exponent value */
    if state == St::DigiExpFinish {
        if exp_sig < (F64_MIN_DEC_EXP - 19) as i64 {
            return_f64_bin!(0); /* underflow */
        }
        if exp_sig > F64_MAX_DEC_EXP as i64 {
            return_inf!(); /* overflow */
        }
        exp = exp_sig as i32;
        state = St::DigiFinish;
    }
    let _ = state;

    /* digi_finish */

    /*
     Fast path 1: the whole computation is exact in double arithmetic, so the
     hardware does the rounding for us.
     */
    if sig < (1u64 << 53) && exp >= -F64_POW10_MAX_EXACT_EXP && exp <= F64_POW10_MAX_EXACT_EXP {
        let mut dbl = sig as f64;
        if exp < 0 {
            dbl /= crate::tables::F64_POW10_TABLE[(-exp) as usize];
        } else {
            dbl *= crate::tables::F64_POW10_TABLE[exp as usize];
        }
        return_f64!(dbl);
    }
    if sig == 0 {
        return_f64_bin!(0);
    }

    /* Fast path 2: 128-bit multiplication against the cached pow10. */
    if sig_cut.is_null() && exp > -F64_MAX_DEC_EXP + 1 && exp < F64_MAX_DEC_EXP - 20 {
        let mut exact = false;
        let (sig2, sig2_ext) = pow10_table_get_sig(exp);
        let mut exp2 = pow10_table_get_exp(exp);

        let lz = u64_lz_bits(sig);
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
            let lz = (hi < (1u64 << 63)) as u32;
            hi <<= lz;
            exp2 -= lz as i32;
            exp2 += 64;

            /* test the bit 54 and get the rounding direction */
            let round_up = (hi & (1u64 << (64 - 54))) > 0;
            hi = hi.wrapping_add(if round_up { 1u64 << (64 - 54) } else { 0 });

            /* test overflow */
            if hi < (1u64 << (64 - 54)) {
                hi = 1u64 << 63;
                exp2 += 1;
            }

            hi >>= F64_BITS - F64_SIG_FULL_BITS;
            exp2 += F64_BITS - F64_SIG_FULL_BITS + F64_SIG_BITS;
            exp2 += F64_EXP_BIAS;
            let raw = ((exp2 as u64) << F64_SIG_BITS) | (hi & F64_SIG_MASK);
            return_f64_bin!(raw);
        }
    }

    /* Slow path: diyfp approximation, verified with bigints when needed. */
    {
        const ERR_ULP_LOG: i32 = 3;
        const ERR_ULP: i32 = 1 << ERR_ULP_LOG;
        const ERR_CACHED_POW: i32 = ERR_ULP / 2;
        const ERR_MUL_FIXED: i32 = ERR_ULP / 2;
        const DIY_SIG_BITS: i32 = 64;
        const EXP_BIAS: i32 = F64_EXP_BIAS + F64_SIG_BITS;
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
        let effective_significand_size = if order_of_magnitude >= EXP_SUBNORMAL + F64_SIG_FULL_BITS
        {
            F64_SIG_FULL_BITS
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
        /* now the number is the correct value, or the next lower value */

        /* upper boundary */
        let mut fp_upper = DiyFp { sig: 0, exp: 0 };
        if raw & F64_EXP_MASK != 0 {
            fp_upper.sig = (raw & F64_SIG_MASK) + (1u64 << F64_SIG_BITS);
            fp_upper.exp = ((raw & F64_EXP_MASK) >> F64_SIG_BITS) as i32;
        } else {
            fp_upper.sig = raw & F64_SIG_MASK;
            fp_upper.exp = 1;
        }
        fp_upper.exp -= F64_EXP_BIAS + F64_SIG_BITS;
        fp_upper.sig <<= 1;
        fp_upper.exp -= 1;
        fp_upper.sig += 1; /* add half ulp */

        /* compare with bigint */
        let mut big_full = BigInt::new();
        let mut big_comp = BigInt::new();
        bigint_set_buf(&mut big_full, sig, &mut exp, sig_cut, sig_end, dot_pos);
        bigint_set_u64(&mut big_comp, fp_upper.sig);
        if exp >= 0 {
            bigint_mul_pow10(&mut big_full, exp);
        } else {
            bigint_mul_pow10(&mut big_comp, -exp);
        }
        if fp_upper.exp > 0 {
            bigint_mul_pow2(&mut big_comp, fp_upper.exp as u32);
        } else {
            bigint_mul_pow2(&mut big_full, (-fp_upper.exp) as u32);
        }
        let cmp = bigint_cmp(&big_full, &big_comp);
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
