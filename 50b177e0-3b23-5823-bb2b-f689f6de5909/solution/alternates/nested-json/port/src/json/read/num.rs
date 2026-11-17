// CANARY-BEGIN
// CANARY-SLOT-1: d1e8ce267716be6676dd76053f33ff204621a43442e5fca09fcb35746f131ee6
// CANARY-SLOT-2: 1685d9fc5ca4a39af92cf6687d71d1e70dd35e98590a85f5aa9865a79c230fb6
// CANARY-SLOT-3: 0c80c2fb291dbf094f27204712e2783d95a637eabaf0119d542ca5866552fb17
// CANARY-SLOT-4: dc485d9f629ab8b2c87c0b9ad0bdb756aec8ab00156830793b436d0ed43a393f
// CANARY-END
/*==============================================================================
 * MARK: - read_num.rs
 *
 * Owns yyjson.c 3251-3394 (`decode_num_raw`, `return_raw`, `decode_num_hex`) and
 * 3603-4695 (bigint, diy_fp, number reader; the fast floating-point path only,
 * since `YYJSON_DISABLE_FAST_FP_CONV` is 0, so the `read_num` at yyjson.c:3855
 * is the active definition and the strtod fallback at 4490 is not built).
 *
 * No exported symbol belongs to this module. It owns one cross-module
 * interface, `read_num`, whose signature is fixed by CONVENTIONS.md and must
 * not change.
 *
 * The C macros `has_flg` / `has_allow` (yyjson.c:3122-3123) are expanded by
 * hand at every use site as `local_has_rflag(flg, YYJSON_READ_<X>, 0 | 1)`. The
 * caller-capturing `return_err` / `return_raw` / `return_0` / `return_i64` /
 * `return_f64` / `return_f64_bin` / `return_inf` macros become module-local
 * `macro_rules!` declared inside the function that owns their locals, exactly
 * as the C macros are `#define`d inside and `#undef`ed at the end of theirs.
 *
 * The `repeat_in_1_18(expr_intg/expr_sepr/expr_frac/expr_stop)` unrolled
 * `goto` chains of yyjson.c:3996-4047 become bounded loops, and the `digi_*`
 * label graph becomes an explicit state enum; see `decode_num_tail`.
 *============================================================================*/

use crate::base::abi::c_char;
use crate::base::inline::raw_yyjson_u64_to_f64;
use crate::json::read::doc::decode_inf_or_nan;
use crate::base::repr::{
    YYJSON_READ_ALLOW_EXT_NUMBER, YYJSON_READ_ALLOW_INF_AND_NAN,
    YYJSON_READ_BIGNUM_AS_RAW, YYJSON_READ_NUMBER_AS_RAW, YYJSON_SUBTYPE_REAL,
    YYJSON_TAG_BIT, YYJSON_TYPE_NUM, YYJSON_TYPE_RAW, yyjson_read_flag,
    yyjson_val,
};
use crate::base::octets::{
    F64_BITS, F64_BITS_INF, F64_EXP_BIAS, F64_EXP_MASK, F64_MAX_BIN_EXP,
    F64_MAX_DEC_DIG, F64_MAX_DEC_EXP, F64_MIN_BIN_EXP, F64_MIN_DEC_EXP,
    F64_POW10_MAX_EXACT_EXP, F64_SIG_BITS, F64_SIG_FULL_BITS, F64_SIG_MASK,
    U64_MAX, U64_POW10_MAX_EXACT_EXP, U64_SAFE_DIG, ch_digit,
    ch_digit_or_fp, ch_exp, ch_fp, ch_hex, ch_nonzero,
    ch_sign, char_to_lower, f64_pow10_table, local_has_rflag, hex_conv_table,
    pow10_table_get_exp, pow10_table_get_sig, u128_mul, u128_mul_add,
    u64_lz_bits, u64_pow10_table,
};

/*==============================================================================
 * MARK: - JSON Reader Utils (Private): raw and hex number readers
 * (yyjson.c:3248-3394)
 *============================================================================*/

/** yyjson.c:3249  Read a JSON number as raw string. */
#[inline(never)]
pub(crate) unsafe fn decode_num_raw(ptr: *mut *mut u8, pre: *mut *mut u8,
                                  flg: yyjson_read_flag, val: *mut yyjson_val,
                                  msg: *mut *const c_char) -> bool {
    unsafe {
        let hdr: *mut u8 = *ptr;
        let mut cur: *mut u8 = *ptr;
        let end: *mut *mut u8 = ptr;

        /* yyjson.c:3251  return_err(_pos, _msg) */
        macro_rules! return_err {
            ($pos:expr, $m:expr) => {{
                *msg = $m.as_ptr() as *const c_char;
                *end = $pos;
                return false;
            }};
        }
        /* yyjson.c:3255  return_raw() */
        macro_rules! return_raw {
            () => {{
                (*val).tag = ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                    | YYJSON_TYPE_RAW as u64;
                (*val).uni.str_ = hdr as *const c_char;
                **pre = b'\0';
                *pre = cur;
                *end = cur;
                return true;
            }};
        }

        /* skip sign */
        cur = cur.add((*cur == b'-') as usize);

        /* read first digit, check leading zero */
        let mut goto_read_double = false;
        while !ch_digit(*cur) {
            /* has_allow(EXT_NUMBER) */
            if local_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true) {
                if *cur == b'+' && cur == hdr {
                    /* leading `+` sign */
                    cur = cur.add(1);
                    continue;
                }
                if *cur == b'.' && ch_digit(*cur.add(1)) {
                    /* e.g. '.123' -> goto read_double */
                    goto_read_double = true;
                    break;
                }
            }
            /* has_allow(INF_AND_NAN) */
            if local_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true) {
                if decode_inf_or_nan(ptr, pre, flg, val) {
                    return true;
                }
            }
            return_err!(cur, c"no digit after sign");
        }

        if !goto_read_double {
            /* read integral part */
            if *cur == b'0' {
                cur = cur.add(1);
                if ch_digit(*cur) {
                    return_err!(cur.sub(1),
                                c"number with leading zero is not allowed");
                }
                if !ch_fp(*cur) {
                    /* has_allow(EXT_NUMBER) */
                    if local_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true)
                        && char_to_lower(*cur) == b'x'
                    {
                        /* hex */
                        cur = cur.add(1);
                        if !ch_hex(*cur) {
                            return_err!(cur, c"invalid hex number");
                        }
                        while ch_hex(*cur) {
                            cur = cur.add(1);
                        }
                    }
                    return_raw!();
                }
            } else {
                while ch_digit(*cur) {
                    cur = cur.add(1);
                }
                if !ch_fp(*cur) {
                    return_raw!();
                }
            }
        }

        /* read_double: */
        /* read fraction part */
        if *cur == b'.' {
            cur = cur.add(1);
            if !ch_digit(*cur) {
                /* has_allow(EXT_NUMBER) */
                if local_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true) {
                    if !ch_exp(*cur) {
                        return_raw!();
                    }
                } else {
                    return_err!(cur, c"no digit after decimal point");
                }
            }
            while ch_digit(*cur) {
                cur = cur.add(1);
            }
        }

        /* read exponent part */
        if ch_exp(*cur) {
            cur = cur.add(1 + ch_sign(*cur.add(1)) as usize);
            let c: u8 = *cur;
            cur = cur.add(1);
            if !ch_digit(c) {
                return_err!(cur, c"no digit after exponent sign");
            }
            while ch_digit(*cur) {
                cur = cur.add(1);
            }
        }

        return_raw!();
    }
}

/** yyjson.c:3326  Read a hex number. */
#[inline(never)]
pub(crate) unsafe fn decode_num_hex(ptr: *mut *mut u8, pre: *mut *mut u8,
                                  flg: yyjson_read_flag, val: *mut yyjson_val,
                                  msg: *mut *const c_char) -> bool {
    unsafe {
        let hdr: *mut u8 = *ptr;
        let mut cur: *mut u8 = *ptr;
        let end: *mut *mut u8 = ptr;
        let mut sig: u64 = 0;
        let mut i: u64 = 0;
        let sign: bool;

        /* skip sign and '0x' */
        sign = *cur == b'-';
        cur = cur.add(((*cur == b'-' || *cur == b'+') as usize) + 2);

        /* read hex */
        while i < 16 {
            let c: u8 = *hex_conv_table.as_ptr()
                .add(*cur.add(i as usize) as usize);
            if c == 0xF0 {
                break;
            }
            sig <<= 4;
            sig |= c as u64;
            i += 1;
        }

        /* check error */
        if i == 0 {
            *msg = c"invalid hex number".as_ptr() as *const c_char;
            return false;
        }

        /* check overflow */
        if i == 16 {
            if ch_hex(*cur.add(16)) || (sign && sig > (1u64 << 63)) {
                /* has_flg(BIGNUM_AS_RAW) */
                if !local_has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW, false) {
                    *msg = c"hex number overflow".as_ptr() as *const c_char;
                    return false;
                }
                cur = cur.add(16);
                while ch_hex(*cur) {
                    cur = cur.add(1);
                }
                **pre = b'\0';
                (*val).tag = ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                    | YYJSON_TYPE_RAW as u64;
                (*val).uni.str_ = hdr as *const c_char;
                *pre = cur;
                *end = cur;
                return true;
            }
        }

        (*val).tag = YYJSON_TYPE_NUM as u64 | (((sign as u8) << 3) as u64);
        (*val).uni.u64_ = if sign { (!sig).wrapping_add(1) } else { sig };
        *end = cur.add(i as usize);
        true
    }
}

/*==============================================================================
 * MARK: - BigInt For Floating Point Number Reader (Private)
 * (yyjson.c:3602-3771)
 *
 * The bigint algorithm is used by the floating-point number reader to get a
 * correctly rounded result for numbers with lots of digits.
 *============================================================================*/

/** yyjson.c:3610  Unsigned arbitrarily large integer. */
#[repr(C)]
pub(crate) struct bigint {
    /** used chunks count, should not be 0 */
    pub used: u32,
    /** chunks (58 is enough here) */
    pub bits: [u64; 64],
}

const _: () = assert!(core::mem::size_of::<bigint>() == 8 + 64 * 8);

/** yyjson.c:3620  Evaluate 'big += val' (both can be 0). */
#[inline(always)]
pub(crate) unsafe fn wideint_add_u64(big: *mut bigint, val: u64) {
    unsafe {
        let mut idx: u32;
        let max: u32;
        let bits: *mut u64 = (*big).bits.as_mut_ptr();
        let num: u64 = *bits.add(0);
        let add: u64 = num.wrapping_add(val);
        *bits.add(0) = add;
        if (add >= num) || (add >= val) {
            return;
        }
        idx = 1;
        max = (*big).used;
        while idx < max {
            if *bits.add(idx as usize) != U64_MAX {
                *bits.add(idx as usize) =
                    (*bits.add(idx as usize)).wrapping_add(1);
                return;
            }
            *bits.add(idx as usize) = 0;
            idx = idx.wrapping_add(1);
        }
        *bits.add((*big).used as usize) = 1;
        (*big).used = (*big).used.wrapping_add(1);
    }
}

/** yyjson.c:3638  Evaluate 'big *= val' (val cannot be 0). */
#[inline(always)]
pub(crate) unsafe fn wideint_mul_u64(big: *mut bigint, val: u64) {
    unsafe {
        let mut idx: u32 = 0;
        let max: u32 = (*big).used;
        let mut hi: u64 = 0;
        let mut lo: u64 = 0;
        let mut carry: u64 = 0;
        let bits: *mut u64 = (*big).bits.as_mut_ptr();
        while idx < max {
            if *bits.add(idx as usize) != 0 {
                break;
            }
            idx = idx.wrapping_add(1);
        }
        while idx < max {
            u128_mul_add(*bits.add(idx as usize), val, carry, &mut hi,
                         &mut lo);
            *bits.add(idx as usize) = lo;
            carry = hi;
            idx = idx.wrapping_add(1);
        }
        if carry != 0 {
            *bits.add((*big).used as usize) = carry;
            (*big).used = (*big).used.wrapping_add(1);
        }
    }
}

/** yyjson.c:3657  Evaluate 'big *= 2^exp' (exp can be 0).
    The C local is named `move`, which is a Rust keyword; it is spelled `mov`
    here, the only renaming in this module. */
#[inline(always)]
pub(crate) unsafe fn wideint_mul_pow2(big: *mut bigint, exp: u32) {
    unsafe {
        let shft: u32 = exp % 64;
        let mut mov: u32 = exp / 64;
        let mut idx: u32 = (*big).used;
        let bits: *mut u64 = (*big).bits.as_mut_ptr();
        if shft == 0 {
            while idx > 0 {
                *bits.add(idx.wrapping_add(mov).wrapping_sub(1) as usize) =
                    *bits.add(idx.wrapping_sub(1) as usize);
                idx = idx.wrapping_sub(1);
            }
            (*big).used = (*big).used.wrapping_add(mov);
            while mov != 0 {
                mov = mov.wrapping_sub(1);
                *bits.add(mov as usize) = 0;
            }
        } else {
            *bits.add(idx as usize) = 0;
            while idx > 0 {
                let mut num: u64 = (*bits.add(idx as usize)) << shft;
                num |= (*bits.add(idx.wrapping_sub(1) as usize))
                    >> (64 - shft);
                *bits.add(idx.wrapping_add(mov) as usize) = num;
                idx = idx.wrapping_sub(1);
            }
            *bits.add(mov as usize) = (*bits.add(0)) << shft;
            (*big).used = (*big).used.wrapping_add(mov.wrapping_add(
                (*bits.add((*big).used.wrapping_add(mov) as usize) > 0)
                    as u32));
            while mov != 0 {
                mov = mov.wrapping_sub(1);
                *bits.add(mov as usize) = 0;
            }
        }
    }
}

/** yyjson.c:3683  Evaluate 'big *= 10^exp' (exp cannot be 0). */
#[inline(always)]
pub(crate) unsafe fn wideint_mul_pow10(big: *mut bigint, mut exp: i32) {
    unsafe {
        while exp >= U64_POW10_MAX_EXACT_EXP {
            wideint_mul_u64(big, *u64_pow10_table.as_ptr()
                .add(U64_POW10_MAX_EXACT_EXP as usize));
            exp = exp.wrapping_sub(U64_POW10_MAX_EXACT_EXP);
        }
        if exp != 0 {
            wideint_mul_u64(big, *u64_pow10_table.as_ptr().add(exp as usize));
        }
    }
}

/** yyjson.c:3696  Compare two bigint.
    @return -1 if 'a < b', +1 if 'a > b', 0 if 'a == b'. */
#[inline(always)]
pub(crate) unsafe fn wideint_cmp(a: *mut bigint, b: *mut bigint) -> i32 {
    unsafe {
        let mut idx: u32 = (*a).used;
        if (*a).used < (*b).used {
            return -1;
        }
        if (*a).used > (*b).used {
            return 1;
        }
        let abits: *const u64 = (*a).bits.as_ptr();
        let bbits: *const u64 = (*b).bits.as_ptr();
        while idx > 0 {
            idx = idx.wrapping_sub(1);
            let av: u64 = *abits.add(idx as usize);
            let bv: u64 = *bbits.add(idx as usize);
            if av < bv {
                return -1;
            }
            if av > bv {
                return 1;
            }
        }
        0
    }
}

/** yyjson.c:3715  Evaluate 'big = val'. */
#[inline(always)]
pub(crate) unsafe fn wideint_set_u64(big: *mut bigint, val: u64) {
    unsafe {
        (*big).used = 1;
        *(*big).bits.as_mut_ptr().add(0) = val;
    }
}

/** yyjson.c:3721  Set a bigint with a floating point number string. */
#[inline(never)]
pub(crate) unsafe fn wideint_set_buf(big: *mut bigint, mut sig: u64,
                                    exp: *mut i32, sig_cut: *mut u8,
                                    mut sig_end: *mut u8, dot_pos: *mut u8) {
    unsafe {
        if sig_cut.is_null() {
            /* no digit cut, set significant part only */
            wideint_set_u64(big, sig);
            return;
        } else {
            /* some digits were cut, read them from 'sig_cut' to 'sig_end' */
            let hdr: *mut u8 = sig_cut;
            let mut cur: *mut u8 = hdr;
            let mut len: u32 = 0;
            let mut val: u64 = 0;
            let mut dig_big_cut: bool = false;
            let has_dot: bool = (hdr < dot_pos) & (dot_pos < sig_end);
            let mut dig_len_total: usize = U64_SAFE_DIG
                .wrapping_add(sig_end.offset_from(hdr) as usize)
                .wrapping_sub(has_dot as usize);

            sig = sig.wrapping_sub((*sig_cut >= b'5') as u64); /* rounded */
            if dig_len_total > F64_MAX_DEC_DIG {
                dig_big_cut = true;
                sig_end = sig_end.sub(
                    dig_len_total.wrapping_sub(F64_MAX_DEC_DIG + 1));
                sig_end = sig_end.sub((dot_pos.add(1) == sig_end) as usize);
                dig_len_total = F64_MAX_DEC_DIG + 1;
            }
            *exp = (*exp).wrapping_sub(
                (dig_len_total as i32).wrapping_sub(U64_SAFE_DIG as i32));

            (*big).used = 1;
            *(*big).bits.as_mut_ptr().add(0) = sig;
            while cur < sig_end {
                if cur != dot_pos {
                    let d: u8 = *cur;
                    cur = cur.add(1);
                    val = val.wrapping_mul(10)
                        .wrapping_add(d.wrapping_sub(b'0') as u64);
                    len = len.wrapping_add(1);
                    if cur == sig_end && dig_big_cut {
                        /* The last digit must be non-zero,    */
                        /* set it to '1' for correct rounding. */
                        val = val.wrapping_sub(val % 10).wrapping_add(1);
                    }
                    if len as usize == U64_SAFE_DIG || cur == sig_end {
                        wideint_mul_pow10(big, len as i32);
                        wideint_add_u64(big, val);
                        val = 0;
                        len = 0;
                    }
                } else {
                    cur = cur.add(1);
                }
            }
        }
    }
}

/*==============================================================================
 * MARK: - Diy Floating Point (Private) (yyjson.c:3772-3845)
 *============================================================================*/

/** yyjson.c:3782  "Do It Yourself Floating Point" struct. */
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct diy_fp {
    /** significand */
    pub sig: u64,
    /** exponent, base 2 */
    pub exp: i32,
    /** padding, useless */
    pub pad: i32,
}

const _: () = assert!(core::mem::size_of::<diy_fp>() == 16);

/** yyjson.c:3790  Get cached rounded diy_fp for pow(10, e). The input value
    must be in `[POW10_SIG_TABLE_MIN_EXP, POW10_SIG_TABLE_MAX_EXP]`. */
#[inline(always)]
pub(crate) unsafe fn diy_fp_get_cached_pow10(exp10: i32) -> diy_fp {
    unsafe {
        let mut fp = diy_fp { sig: 0, exp: 0, pad: 0 };
        let mut sig_ext: u64 = 0;
        pow10_table_get_sig(exp10, &mut fp.sig, &mut sig_ext);
        pow10_table_get_exp(exp10, &mut fp.exp);
        fp.sig = fp.sig.wrapping_add(sig_ext >> 63);
        fp
    }
}

/** yyjson.c:3800  Returns fp * fp2. */
#[inline(always)]
pub(crate) unsafe fn diy_fp_mul(mut fp: diy_fp, fp2: diy_fp) -> diy_fp {
    unsafe {
        let mut hi: u64 = 0;
        let mut lo: u64 = 0;
        u128_mul(fp.sig, fp2.sig, &mut hi, &mut lo);
        fp.sig = hi.wrapping_add(lo >> 63);
        fp.exp = fp.exp.wrapping_add(fp2.exp).wrapping_add(64);
        fp
    }
}

/** yyjson.c:3809  Convert diy_fp to IEEE-754 raw value. */
#[inline(always)]
pub(crate) unsafe fn diy_fp_to_ieee_raw(fp: diy_fp) -> u64 {
    unsafe {
        let mut sig: u64 = fp.sig;
        let mut exp: i32 = fp.exp;
        let lz_bits: u32;
        if fp.sig == 0 {
            return 0;
        }

        lz_bits = u64_lz_bits(sig);
        sig <<= lz_bits;
        sig >>= F64_BITS - F64_SIG_FULL_BITS;
        exp = exp.wrapping_sub(lz_bits as i32);
        exp = exp.wrapping_add((F64_BITS - F64_SIG_FULL_BITS) as i32);
        exp = exp.wrapping_add(F64_SIG_BITS as i32);

        if exp >= F64_MAX_BIN_EXP {
            /* overflow */
            F64_BITS_INF
        } else if exp >= F64_MIN_BIN_EXP - 1 {
            /* normal */
            exp = exp.wrapping_add(F64_EXP_BIAS);
            ((exp as u64) << F64_SIG_BITS) | (sig & F64_SIG_MASK)
        } else if exp >= F64_MIN_BIN_EXP - F64_SIG_FULL_BITS as i32 {
            /* subnormal */
            sig >> F64_MIN_BIN_EXP.wrapping_sub(exp).wrapping_sub(1)
        } else {
            /* underflow */
            0
        }
    }
}

/*==============================================================================
 * MARK: - Number Reader (Private) (yyjson.c:3846-4489)
 *
 * `!YYJSON_DISABLE_FAST_FP_CONV` branch; the `read_num` at yyjson.c:3855.
 *============================================================================*/

/** The `digi_*` labels of yyjson.c:4014-4175. `DigiSepr(i)` and `DigiFrac(i)`
    stand for the `repeat_in_1_18`-generated `digi_sepr_##i` / `digi_frac_##i`
    labels; the `digi_stop_##i` chain is folded into `DigiFrac`. */
enum St {
    DigiSepr(u32),
    DigiFrac(u32),
    DigiIntgMore,
    DigiFracMore,
    DigiFracEnd,
    DigiExpMore,
    DigiExpFinish,
    DigiFinish,
}

/**
 yyjson.c:3855  Read a JSON number.

 1. This function assume that the floating-point number is in IEEE-754 format.
 2. This function support uint64/int64/double number. If an integer number
    cannot fit in uint64/int64, it will returns as a double number. If a double
    number is infinite, the return value is based on flag.

 Called by reader.rs and incr.rs.
 */
#[inline(never)]
pub(crate) unsafe fn read_num(ptr: *mut *mut u8, pre: *mut *mut u8,
                              flg: yyjson_read_flag, val: *mut yyjson_val,
                              msg: *mut *const c_char) -> bool {
    unsafe {
        let mut sig: u64 = 0; /* significant part of the number */
        let mut dot_pos: *mut u8 = core::ptr::null_mut(); /* decimal point */

        let hdr: *mut u8 = *ptr;
        let mut cur: *mut u8 = *ptr;
        let end: *mut *mut u8 = ptr;
        let sign: bool;

        /* read number as raw string if has `YYJSON_READ_NUMBER_AS_RAW` flag */
        /* has_flg(NUMBER_AS_RAW) */
        if local_has_rflag(flg, YYJSON_READ_NUMBER_AS_RAW, false) {
            return decode_num_raw(ptr, pre, flg, val, msg);
        }

        /*--------------------------------------------------------------------
         The caller-capturing macros of yyjson.c:3856-3886. Only the ones this
         prologue reaches are declared here; `decode_num_tail` declares its own.
        --------------------------------------------------------------------*/
        macro_rules! return_err {
            ($pos:expr, $m:expr) => {{
                *msg = $m.as_ptr() as *const c_char;
                *end = $pos;
                return false;
            }};
        }
        macro_rules! return_0 {
            () => {{
                (*val).tag =
                    YYJSON_TYPE_NUM as u64 | (((sign as u8) << 3) as u64);
                (*val).uni.u64_ = 0;
                *end = cur;
                return true;
            }};
        }
        macro_rules! return_i64 {
            ($v:expr) => {{
                let v: u64 = $v;
                (*val).tag =
                    YYJSON_TYPE_NUM as u64 | (((sign as u8) << 3) as u64);
                (*val).uni.u64_ =
                    if sign { (!v).wrapping_add(1) } else { v };
                *end = cur;
                return true;
            }};
        }
        macro_rules! return_f64 {
            ($v:expr) => {{
                let v: f64 = $v;
                (*val).tag =
                    YYJSON_TYPE_NUM as u64 | YYJSON_SUBTYPE_REAL as u64;
                (*val).uni.f64_ = if sign { -v } else { v };
                *end = cur;
                return true;
            }};
        }
        macro_rules! return_f64_bin {
            ($v:expr) => {{
                let v: u64 = $v;
                (*val).tag =
                    YYJSON_TYPE_NUM as u64 | YYJSON_SUBTYPE_REAL as u64;
                (*val).uni.u64_ = ((sign as u64) << 63) | v;
                *end = cur;
                return true;
            }};
        }
        macro_rules! return_raw {
            () => {{
                /* add null-terminator for previous raw string */
                **pre = b'\0';
                (*val).tag = ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                    | YYJSON_TYPE_RAW as u64;
                (*val).uni.str_ = hdr as *const c_char;
                *pre = cur;
                *end = cur;
                return true;
            }};
        }

        sign = *hdr == b'-';
        cur = cur.add(sign as usize);

        /* begin with a leading zero or non-digit (yyjson.c:3899) */
        'nonzero: {
            'leading_dot: {
                while !ch_nonzero(*cur) {
                    /* 0 or non-digit char */
                    if *cur != b'0' {
                        /* non-digit char */
                        /* has_allow(EXT_NUMBER) */
                        if local_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true) {
                            if *cur == b'+' && cur == hdr {
                                /* leading `+` sign */
                                cur = cur.add(1);
                                continue;
                            }
                            if *cur == b'.' && ch_digit(*cur.add(1)) {
                                /* e.g. '.123' -> goto leading_dot */
                                break 'leading_dot;
                            }
                        }
                        /* has_allow(INF_AND_NAN) */
                        if local_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true)
                        {
                            if decode_inf_or_nan(ptr, pre, flg, val) {
                                return true;
                            }
                        }
                        return_err!(cur, c"no digit after sign");
                    }
                    /* begin with 0 */
                    cur = cur.add(1);
                    if !ch_digit_or_fp(*cur) {
                        /* has_allow(EXT_NUMBER) */
                        if local_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true)
                            && char_to_lower(*cur) == b'x'
                        {
                            /* hex */
                            return decode_num_hex(ptr, pre, flg, val, msg);
                        }
                        return_0!();
                    }
                    if *cur == b'.' {
                        break 'leading_dot;
                    }
                    if ch_digit(*cur) {
                        return_err!(
                            cur.sub(1),
                            c"number with leading zero is not allowed");
                    }
                    if ch_exp(*cur) {
                        /* 0 with any exponent is still 0 */
                        cur = cur.add(1 + ch_sign(*cur.add(1)) as usize);
                        if !ch_digit(*cur) {
                            return_err!(cur,
                                        c"no digit after exponent sign");
                        }
                        loop {
                            cur = cur.add(1);
                            if !ch_digit(*cur) {
                                break;
                            }
                        }
                    }
                    return_f64_bin!(0);
                }
                /* the `while` condition failed: begin with non-zero digit */
                break 'nonzero;
            }

            /* leading_dot: (yyjson.c:3916) */
            dot_pos = cur;
            cur = cur.add(1);
            if !ch_digit(*cur) {
                /* has_allow(EXT_NUMBER) */
                if local_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true) {
                    if ch_exp(*cur) {
                        /* goto digi_exp_more */
                        return decode_num_tail(ptr, pre, flg, val, msg,
                                             St::DigiExpMore, hdr, cur, sign,
                                             sig, core::ptr::null_mut(),
                                             core::ptr::null_mut(), dot_pos);
                    } else {
                        return_f64_bin!(0);
                    }
                }
                return_err!(cur, c"no digit after decimal point");
            }
            while *cur == b'0' {
                cur = cur.add(1);
            }
            if ch_digit(*cur) {
                /* first non-zero digit after decimal point */
                sig = (*cur).wrapping_sub(b'0') as u64; /* read first digit */
                cur = cur.sub(1);
                /* goto digi_frac_1: continue read fraction part */
                return decode_num_tail(ptr, pre, flg, val, msg, St::DigiFrac(1),
                                     hdr, cur, sign, sig,
                                     core::ptr::null_mut(),
                                     core::ptr::null_mut(), dot_pos);
            }
            if ch_digit(*cur) {
                return_err!(cur.sub(1),
                            c"number with leading zero is not allowed");
            }
            if ch_exp(*cur) {
                /* 0 with any exponent is still 0 */
                cur = cur.add(1 + ch_sign(*cur.add(1)) as usize);
                if !ch_digit(*cur) {
                    return_err!(cur, c"no digit after exponent sign");
                }
                loop {
                    cur = cur.add(1);
                    if !ch_digit(*cur) {
                        break;
                    }
                }
            }
            return_f64_bin!(0);
        }

        /* begin with non-zero digit (yyjson.c:3946) */
        sig = (*cur).wrapping_sub(b'0') as u64;

        /*
         Read integral part: `repeat_in_1_18(expr_intg)` of yyjson.c:3996,
         which the C comment documents as

             for (int i = 1; i <= 18; i++) {
                num = cur[i] - '0';
                if (num <= 9) sig = num + sig * 10;
                else goto digi_sepr_i;
             }
         */
        let state: St;
        {
            let mut i: u32 = 1;
            let mut sepr: u32 = 0;
            while i <= 18 {
                let num: u64 =
                    (*cur.add(i as usize)).wrapping_sub(b'0') as u64;
                if num <= 9 {
                    sig = num.wrapping_add(sig.wrapping_mul(10));
                    i = i.wrapping_add(1);
                } else {
                    sepr = i;
                    break;
                }
            }
            if sepr == 0 {
                cur = cur.add(19); /* skip continuous 19 digits */
                if !ch_digit_or_fp(*cur) {
                    /* this number is an integer consisting of 19 digits */
                    if sign && (sig > (1u64 << 63)) {
                        /* overflow */
                        /* has_flg(BIGNUM_AS_RAW) */
                        if local_has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW, false) {
                            return_raw!();
                        }
                        return_f64!(raw_yyjson_u64_to_f64(sig));
                    }
                    return_i64!(sig);
                }
                /* goto digi_intg_more: read more digits in integral part */
                state = St::DigiIntgMore;
            } else {
                state = St::DigiSepr(sepr);
            }
        }

        decode_num_tail(ptr, pre, flg, val, msg, state, hdr, cur, sign, sig,
                      core::ptr::null_mut(), core::ptr::null_mut(), dot_pos)
    }
}

/**
 The `digi_*` label graph of yyjson.c:4014-4484, driven by an explicit state
 machine. `read_num` enters it with the state matching the C `goto` it took;
 every local the labels read is passed in with its value at that point
 (`exp`, `exp_sig` and `exp_lit` are 0 at every entry edge, matching their C
 initializers, and the C code never reads them before writing on any path
 reachable from an entry that does not set them).
 */
#[inline(always)]
unsafe fn decode_num_tail(ptr: *mut *mut u8, pre: *mut *mut u8,
                        flg: yyjson_read_flag, val: *mut yyjson_val,
                        msg: *mut *const c_char, state0: St, hdr: *mut u8,
                        cur0: *mut u8, sign: bool, sig0: u64,
                        sig_cut0: *mut u8, sig_end0: *mut u8,
                        dot_pos0: *mut u8) -> bool {
    unsafe {
        let mut sig_cut: *mut u8 = sig_cut0;
        let mut sig_end: *mut u8 = sig_end0;
        let mut dot_pos: *mut u8 = dot_pos0;
        let mut sig: u64 = sig0;
        let mut exp: i32 = 0;
        let mut exp_sign: bool = false;
        let _ = exp_sign;
        let mut exp_sig: i64 = 0;
        let mut exp_lit: i64 = 0;
        let mut cur: *mut u8 = cur0;
        let end: *mut *mut u8 = ptr;
        let mut state: St = state0;

        macro_rules! return_err {
            ($pos:expr, $m:expr) => {{
                *msg = $m.as_ptr() as *const c_char;
                *end = $pos;
                return false;
            }};
        }
        macro_rules! return_i64 {
            ($v:expr) => {{
                let v: u64 = $v;
                (*val).tag =
                    YYJSON_TYPE_NUM as u64 | (((sign as u8) << 3) as u64);
                (*val).uni.u64_ =
                    if sign { (!v).wrapping_add(1) } else { v };
                *end = cur;
                return true;
            }};
        }
        macro_rules! return_f64 {
            ($v:expr) => {{
                let v: f64 = $v;
                (*val).tag =
                    YYJSON_TYPE_NUM as u64 | YYJSON_SUBTYPE_REAL as u64;
                (*val).uni.f64_ = if sign { -v } else { v };
                *end = cur;
                return true;
            }};
        }
        macro_rules! return_f64_bin {
            ($v:expr) => {{
                let v: u64 = $v;
                (*val).tag =
                    YYJSON_TYPE_NUM as u64 | YYJSON_SUBTYPE_REAL as u64;
                (*val).uni.u64_ = ((sign as u64) << 63) | v;
                *end = cur;
                return true;
            }};
        }
        macro_rules! return_raw {
            () => {{
                **pre = b'\0';
                (*val).tag = ((cur.offset_from(hdr) as u64) << YYJSON_TAG_BIT)
                    | YYJSON_TYPE_RAW as u64;
                (*val).uni.str_ = hdr as *const c_char;
                *pre = cur;
                *end = cur;
                return true;
            }};
        }
        macro_rules! return_inf {
            () => {{
                /* has_flg(BIGNUM_AS_RAW) */
                if local_has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW, false) {
                    return_raw!();
                }
                /* has_allow(INF_AND_NAN) */
                if local_has_rflag(flg, YYJSON_READ_ALLOW_INF_AND_NAN, true) {
                    return_f64_bin!(F64_BITS_INF);
                } else {
                    return_err!(hdr,
                                c"number is infinity when parsed as double");
                }
            }};
        }

        loop {
            match state {
                /* `repeat_in_1_18(expr_sepr)` of yyjson.c:4017:
                   process first non-digit character */
                St::DigiSepr(i) => {
                    if !ch_fp(*cur.add(i as usize)) {
                        cur = cur.add(i as usize);
                        return_i64!(sig);
                    }
                    dot_pos = cur.add(i as usize);
                    if *cur.add(i as usize) == b'.' {
                        state = St::DigiFrac(i);
                        continue;
                    }
                    cur = cur.add(i as usize);
                    sig_end = cur;
                    state = St::DigiExpMore;
                    continue;
                }

                /* `repeat_in_1_18(expr_frac)` of yyjson.c:4027 followed by
                   the `repeat_in_1_18(expr_stop)` chain of yyjson.c:4042:
                   read fraction part */
                St::DigiFrac(i0) => {
                    let mut i: u32 = i0;
                    let mut stop: u32 = 0;
                    while i <= 18 {
                        let num: u64 = (*cur.add(i.wrapping_add(1) as usize))
                            .wrapping_sub(b'0') as u64;
                        if num <= 9 {
                            sig = num.wrapping_add(sig.wrapping_mul(10));
                            i = i.wrapping_add(1);
                        } else {
                            stop = i;
                            break;
                        }
                    }
                    if stop == 0 {
                        /* skip 19 digits and 1 decimal point */
                        cur = cur.add(20);
                        if !ch_digit(*cur) {
                            /* goto digi_frac_end */
                            state = St::DigiFracEnd;
                        } else {
                            /* goto digi_frac_more */
                            state = St::DigiFracMore;
                        }
                    } else {
                        /* digi_stop_##i: cur += i + 1; goto digi_frac_end */
                        cur = cur.add(stop.wrapping_add(1) as usize);
                        state = St::DigiFracEnd;
                    }
                    continue;
                }

                /* yyjson.c:4050  read more digits in integral part */
                St::DigiIntgMore => {
                    if ch_digit(*cur) {
                        if !ch_digit_or_fp(*cur.add(1)) {
                            /* an integer consisting of 20 digits */
                            let num: u64 = (*cur).wrapping_sub(b'0') as u64;
                            if (sig < (U64_MAX / 10))
                                || (sig == (U64_MAX / 10)
                                    && num <= (U64_MAX % 10))
                            {
                                sig = num.wrapping_add(sig.wrapping_mul(10));
                                cur = cur.add(1);
                                /* convert to double if overflow */
                                if sign {
                                    /* has_flg(BIGNUM_AS_RAW) */
                                    if local_has_rflag(flg,
                                                 YYJSON_READ_BIGNUM_AS_RAW,
                                                 false)
                                    {
                                        return_raw!();
                                    }
                                    return_f64!(
                                        raw_yyjson_u64_to_f64(sig));
                                }
                                return_i64!(sig);
                            }
                        }
                    }

                    if ch_exp(*cur) {
                        dot_pos = cur;
                        state = St::DigiExpMore;
                        continue;
                    }

                    if *cur == b'.' {
                        dot_pos = cur;
                        cur = cur.add(1);
                        if !ch_digit(*cur) {
                            /* has_allow(EXT_NUMBER) */
                            if local_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER,
                                         true)
                            {
                                state = St::DigiFracEnd;
                                continue;
                            }
                            return_err!(cur, c"no digit after decimal point");
                        }
                    }

                    /* fall through to digi_frac_more */
                    state = St::DigiFracMore;
                    continue;
                }

                /* yyjson.c:4084  read more digits in fraction part */
                St::DigiFracMore => {
                    /* too large to fit in u64, excess digits need to be cut */
                    sig_cut = cur;
                    sig = sig.wrapping_add((*cur >= b'5') as u64); /* round */
                    loop {
                        cur = cur.add(1);
                        if !ch_digit(*cur) {
                            break;
                        }
                    }
                    if dot_pos.is_null() {
                        /* has_flg(BIGNUM_AS_RAW) */
                        if !ch_fp(*cur)
                            && local_has_rflag(flg, YYJSON_READ_BIGNUM_AS_RAW,
                                         false)
                        {
                            /* it's a large integer */
                            return_raw!();
                        }
                        dot_pos = cur;
                        if *cur == b'.' {
                            cur = cur.add(1);
                            if !ch_digit(*cur) {
                                /* has_allow(EXT_NUMBER) */
                                if !local_has_rflag(flg,
                                              YYJSON_READ_ALLOW_EXT_NUMBER,
                                              true)
                                {
                                    return_err!(
                                        cur,
                                        c"no digit after decimal point");
                                }
                            }
                            while ch_digit(*cur) {
                                cur = cur.add(1);
                            }
                        }
                    }
                    exp_sig = dot_pos.offset_from(sig_cut) as i64;
                    exp_sig =
                        exp_sig.wrapping_add((dot_pos < sig_cut) as i64);

                    /* ignore trailing zeros */
                    let mut tmp: *mut u8 = cur.sub(1);
                    while (*tmp == b'0' || *tmp == b'.') && tmp > hdr {
                        tmp = tmp.sub(1);
                    }
                    if tmp < sig_cut {
                        sig_cut = core::ptr::null_mut();
                    } else {
                        sig_end = cur;
                    }

                    if ch_exp(*cur) {
                        state = St::DigiExpMore;
                    } else {
                        state = St::DigiExpFinish;
                    }
                    continue;
                }

                /* yyjson.c:4120  fraction part end */
                St::DigiFracEnd => {
                    if dot_pos.add(1) == cur {
                        /* has_allow(EXT_NUMBER) */
                        if !local_has_rflag(flg, YYJSON_READ_ALLOW_EXT_NUMBER, true)
                        {
                            return_err!(cur, c"no digit after decimal point");
                        }
                    }
                    sig_end = cur;
                    exp_sig = -(((cur.offset_from(dot_pos) as u64)
                        .wrapping_sub(1)) as i64);
                    if !ch_exp(*cur) {
                        if exp_sig < (F64_MIN_DEC_EXP - 19) as i64 {
                            return_f64_bin!(0); /* underflow */
                        }
                        exp = exp_sig as i32;
                        state = St::DigiFinish;
                        continue;
                    } else {
                        state = St::DigiExpMore;
                        continue;
                    }
                }

                /* yyjson.c:4136  read exponent part */
                St::DigiExpMore => {
                    cur = cur.add(1);
                    exp_sign = *cur == b'-';
                    cur = cur.add(ch_sign(*cur) as usize);
                    if !ch_digit(*cur) {
                        return_err!(cur, c"no digit after exponent sign");
                    }
                    while *cur == b'0' {
                        cur = cur.add(1);
                    }

                    /* read exponent literal */
                    let tmp: *mut u8 = cur;
                    while ch_digit(*cur) {
                        let d: u8 = *cur;
                        cur = cur.add(1);
                        exp_lit = (d.wrapping_sub(b'0') as u64)
                            .wrapping_add((exp_lit as u64).wrapping_mul(10))
                            as i64;
                    }
                    if (cur.offset_from(tmp) as i64) >= U64_SAFE_DIG as i64 {
                        if exp_sign {
                            return_f64_bin!(0); /* underflow */
                        } else {
                            return_inf!(); /* overflow */
                        }
                    }
                    exp_sig = exp_sig.wrapping_add(if exp_sign {
                        exp_lit.wrapping_neg()
                    } else {
                        exp_lit
                    });

                    /* fall through to digi_exp_finish */
                    state = St::DigiExpFinish;
                    continue;
                }

                /* yyjson.c:4161  validate exponent value */
                St::DigiExpFinish => {
                    if exp_sig < (F64_MIN_DEC_EXP - 19) as i64 {
                        return_f64_bin!(0); /* underflow */
                    }
                    if exp_sig > F64_MAX_DEC_EXP as i64 {
                        return_inf!(); /* overflow */
                    }
                    exp = exp_sig as i32;

                    /* fall through to digi_finish */
                    state = St::DigiFinish;
                    continue;
                }

                /* yyjson.c:4171  all digit read finished */
                St::DigiFinish => break,
            }
        }

        /*
         Fast path 1 (yyjson.c:4186, `YYJSON_DOUBLE_MATH_CORRECT` is 1):

         1. The floating-point number calculation should be accurate.
         2. Correct rounding should be performed (fegetround() == FE_TONEAREST).
         3. The input to floating-point calculations does not lose precision,
            which means: 64 - leading_zero(input) - trailing_zero(input) < 53.
         */
        if sig < (1u64 << 53)
            && exp >= -F64_POW10_MAX_EXACT_EXP
            && exp <= F64_POW10_MAX_EXACT_EXP
        {
            let mut dbl: f64 = sig as f64;
            if exp < 0 {
                dbl /= *f64_pow10_table.as_ptr().add(exp.wrapping_neg() as usize);
            } else {
                dbl *= *f64_pow10_table.as_ptr().add(exp as usize);
            }
            return_f64!(dbl);
        }
        if sig == 0 {
            return_f64_bin!(0);
        }

        /*
         Fast path 2 (yyjson.c:4200):
         To keep it simple, we only accept normal number here,
         let the slow path handle subnormal and infinite numbers.
         */
        if sig_cut.is_null()
            && exp > -F64_MAX_DEC_EXP + 1
            && exp < F64_MAX_DEC_EXP - 20
        {
            let raw: u64;
            let sig1: u64;
            let mut sig2: u64 = 0;
            let mut sig2_ext: u64 = 0;
            let mut hi: u64 = 0;
            let mut lo: u64 = 0;
            let mut hi2: u64 = 0;
            let mut lo2: u64 = 0;
            let add: u64;
            let bits: u64;
            let mut exp2: i32 = 0;
            let mut lz: u32;
            let mut exact: bool = false;
            let carry: bool;
            let round_up: bool;

            /* convert (10^exp) to (sig2 * 2^exp2) */
            pow10_table_get_sig(exp, &mut sig2, &mut sig2_ext);
            pow10_table_get_exp(exp, &mut exp2);

            /* normalize and multiply */
            lz = u64_lz_bits(sig);
            sig1 = sig << lz;
            exp2 = exp2.wrapping_sub(lz as i32);
            u128_mul(sig1, sig2, &mut hi, &mut lo);

            /*
             The highest significant 53 bits is used by IEEE-754 double
             number, and the bit 54 is used to detect rounding direction.
             The lowest (64 - 54 - 1) bits is used to check whether it
             contains 0.
             */
            bits = hi & ((1u64 << (64 - 54 - 1)) - 1);
            if bits.wrapping_sub(1) < ((1u64 << (64 - 54 - 1)) - 2) {
                /*
                 (bits != 0 && bits != 0x1FF) => (bits - 1 < 0x1FF - 1)
                 The `bits` is not zero, so we don't need to check
                 `round to even`.
                 */
                exact = true;
            } else {
                /*
                 (bits == 0 || bits == 0x1FF)
                 The `bits` is filled with all `0` or all `1`, so we need to
                 check lower bits with another 64-bit multiplication.
                 */
                u128_mul(sig1, sig2_ext, &mut hi2, &mut lo2);

                add = lo.wrapping_add(hi2);
                if add.wrapping_add(1) > 1u64 {
                    /* (add != 0 && add != U64_MAX) => (add + 1 > 1) */
                    carry = add < lo || add < hi2;
                    hi = hi.wrapping_add(carry as u64);
                    exact = true;
                }
            }

            if exact {
                /* normalize */
                lz = (hi < (1u64 << 63)) as u32;
                hi <<= lz;
                exp2 = exp2.wrapping_sub(lz as i32);
                exp2 = exp2.wrapping_add(64);

                /* test the bit 54 and get rounding direction */
                round_up = (hi & (1u64 << (64 - 54))) > 0u64;
                hi = hi.wrapping_add(if round_up {
                    1u64 << (64 - 54)
                } else {
                    0u64
                });

                /* test overflow */
                if hi < (1u64 << (64 - 54)) {
                    hi = 1u64 << 63;
                    exp2 = exp2.wrapping_add(1);
                }

                /* This is a normal number, convert it to IEEE-754 format. */
                hi >>= F64_BITS - F64_SIG_FULL_BITS;
                exp2 = exp2.wrapping_add(
                    (F64_BITS - F64_SIG_FULL_BITS + F64_SIG_BITS) as i32);
                exp2 = exp2.wrapping_add(F64_EXP_BIAS);
                raw = ((exp2 as u64) << F64_SIG_BITS) | (hi & F64_SIG_MASK);
                return_f64_bin!(raw);
            }
        }

        /*
         Slow path (yyjson.c:4323): read double number exactly with diyfp.
         1. Use cached diyfp to get an approximation value.
         2. Use bigcomp to check the approximation value if needed.

         This algorithm refers to google's double-conversion project.
         */
        {
            const ERR_ULP_LOG: i32 = 3;
            const ERR_ULP: i32 = 1 << ERR_ULP_LOG;
            const ERR_CACHED_POW: i32 = ERR_ULP / 2;
            const ERR_MUL_FIXED: i32 = ERR_ULP / 2;
            const DIY_SIG_BITS: i32 = 64;
            const EXP_BIAS: i32 = F64_EXP_BIAS + F64_SIG_BITS as i32;
            const EXP_SUBNORMAL: i32 = -EXP_BIAS + 1;

            let mut fp_err: u64;
            let mut bits: u32;
            let order_of_magnitude: i32;
            let effective_significand_size: i32;
            let mut precision_digits_count: i32;
            let mut precision_bits: u64;
            let mut half_way: u64;

            let mut raw: u64;
            let mut fp: diy_fp;
            let mut fp_upper: diy_fp;
            let mut big_full: bigint = core::mem::zeroed();
            let mut big_comp: bigint = core::mem::zeroed();
            let cmp: i32;

            fp = diy_fp { sig: sig, exp: 0, pad: 0 };
            fp_err = if !sig_cut.is_null() {
                (ERR_ULP / 2) as u64
            } else {
                0u64
            };

            /* normalize */
            bits = u64_lz_bits(fp.sig);
            fp.sig <<= bits;
            fp.exp = fp.exp.wrapping_sub(bits as i32);
            fp_err <<= bits;

            /* multiply and add error */
            fp = diy_fp_mul(fp, diy_fp_get_cached_pow10(exp));
            fp_err = fp_err
                .wrapping_add(ERR_CACHED_POW as u64)
                .wrapping_add((fp_err != 0) as u64)
                .wrapping_add(ERR_MUL_FIXED as u64);

            /* normalize */
            bits = u64_lz_bits(fp.sig);
            fp.sig <<= bits;
            fp.exp = fp.exp.wrapping_sub(bits as i32);
            fp_err <<= bits;

            /* effective significand */
            order_of_magnitude = DIY_SIG_BITS.wrapping_add(fp.exp);
            if order_of_magnitude >= EXP_SUBNORMAL + F64_SIG_FULL_BITS as i32
            {
                effective_significand_size = F64_SIG_FULL_BITS as i32;
            } else if order_of_magnitude <= EXP_SUBNORMAL {
                effective_significand_size = 0;
            } else {
                effective_significand_size =
                    order_of_magnitude.wrapping_sub(EXP_SUBNORMAL);
            }

            /* precision digits count */
            precision_digits_count =
                DIY_SIG_BITS.wrapping_sub(effective_significand_size);
            if precision_digits_count.wrapping_add(ERR_ULP_LOG)
                >= DIY_SIG_BITS
            {
                let shr: i32 = precision_digits_count
                    .wrapping_add(ERR_ULP_LOG)
                    .wrapping_sub(DIY_SIG_BITS).wrapping_add(1);
                fp.sig >>= shr;
                fp.exp = fp.exp.wrapping_add(shr);
                fp_err = (fp_err >> shr)
                    .wrapping_add(1)
                    .wrapping_add(ERR_ULP as u32 as u64);
                precision_digits_count =
                    precision_digits_count.wrapping_sub(shr);
            }

            /* half way */
            precision_bits = fp.sig & ((1u64 << precision_digits_count) - 1);
            precision_bits =
                precision_bits.wrapping_mul(ERR_ULP as u32 as u64);
            half_way = 1u64 << (precision_digits_count - 1);
            half_way = half_way.wrapping_mul(ERR_ULP as u32 as u64);

            /* rounding */
            fp.sig >>= precision_digits_count;
            fp.sig = fp.sig.wrapping_add(
                (precision_bits >= half_way.wrapping_add(fp_err)) as u64);
            fp.exp = fp.exp.wrapping_add(precision_digits_count);

            /* get IEEE double raw value */
            raw = diy_fp_to_ieee_raw(fp);
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
            fp_upper = diy_fp { sig: 0, exp: 0, pad: 0 };
            if (raw & F64_EXP_MASK) != 0 {
                fp_upper.sig = (raw & F64_SIG_MASK)
                    .wrapping_add(1u64 << F64_SIG_BITS);
                fp_upper.exp = ((raw & F64_EXP_MASK) >> F64_SIG_BITS) as i32;
            } else {
                fp_upper.sig = raw & F64_SIG_MASK;
                fp_upper.exp = 1;
            }
            fp_upper.exp = fp_upper.exp
                .wrapping_sub(F64_EXP_BIAS + F64_SIG_BITS as i32);
            fp_upper.sig <<= 1;
            fp_upper.exp = fp_upper.exp.wrapping_sub(1);
            fp_upper.sig = fp_upper.sig.wrapping_add(1); /* add half ulp */

            /* compare with bigint */
            wideint_set_buf(&mut big_full, sig, &mut exp, sig_cut, sig_end,
                           dot_pos);
            wideint_set_u64(&mut big_comp, fp_upper.sig);
            if exp >= 0 {
                wideint_mul_pow10(&mut big_full, exp);
            } else {
                wideint_mul_pow10(&mut big_comp, exp.wrapping_neg());
            }
            if fp_upper.exp > 0 {
                wideint_mul_pow2(&mut big_comp, fp_upper.exp as u32);
            } else {
                wideint_mul_pow2(&mut big_full,
                                fp_upper.exp.wrapping_neg() as u32);
            }
            cmp = wideint_cmp(&mut big_full, &mut big_comp);
            if cmp != 0 {
                /* round down or round up */
                raw = raw.wrapping_add((cmp > 0) as u64);
            } else {
                /* falls midway, round to even */
                raw = raw.wrapping_add(raw & 1);
            }

            if raw == F64_BITS_INF {
                return_inf!();
            }
            return_f64_bin!(raw);
        }
    }
}
