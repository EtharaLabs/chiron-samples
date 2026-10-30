//! The JSON writer: integer writer, floating-point writer (Schubfach), string
//! writer, the three root writers and the public `yyjson_*write*` entry
//! points.
//!
//! As in `read.rs`, the C `goto` state machines become `loop { match state }`
//! and the unrolled macro chains become counted loops. Every digit of the
//! number output comes from the same integer arithmetic as the C version, so
//! the output matches digit for digit.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::alc::{YYJSON_DEFAULT_ALC, YYJSON_NULL_ALC};
use crate::file;
use crate::mem;
use crate::pow10::*;
use crate::tables::*;
use crate::types::*;
use crate::util::*;

#[inline(always)]
fn has_flg(flg: u32, chk: u32) -> bool {
    (flg & chk) != 0
}

#[inline(always)]
unsafe fn cpy2(dst: *mut u8, src: *const u8) {
    mem::copy(dst, src, 2);
}

#[inline(always)]
unsafe fn cpy2_lit(dst: *mut u8, src: &[u8; 2]) {
    mem::copy(dst, src.as_ptr(), 2);
}

#[inline(always)]
fn dtbl(i: u32) -> *const u8 {
    unsafe { DIGIT_TABLE.as_ptr().add(i as usize) }
}

/* ----------------------------------------------------- integer writer */

/// `write_u32_len_8`
#[inline(always)]
unsafe fn write_u32_len_8(val: u32, buf: *mut u8) -> *mut u8 {
    let aabb = (((val as u64) * 109951163) >> 40) as u32; /* val / 10000 */
    let ccdd = val - aabb * 10000; /* val % 10000 */
    let aa = (aabb * 5243) >> 19; /* aabb / 100 */
    let cc = (ccdd * 5243) >> 19; /* ccdd / 100 */
    let bb = aabb - aa * 100;
    let dd = ccdd - cc * 100;
    cpy2(buf, dtbl(aa * 2));
    cpy2(buf.add(2), dtbl(bb * 2));
    cpy2(buf.add(4), dtbl(cc * 2));
    cpy2(buf.add(6), dtbl(dd * 2));
    buf.add(8)
}

/// `write_u32_len_4`
#[inline(always)]
unsafe fn write_u32_len_4(val: u32, buf: *mut u8) -> *mut u8 {
    let aa = (val * 5243) >> 19;
    let bb = val - aa * 100;
    cpy2(buf, dtbl(aa * 2));
    cpy2(buf.add(2), dtbl(bb * 2));
    buf.add(4)
}

/// `write_u32_len_1_to_8`
#[inline(always)]
unsafe fn write_u32_len_1_to_8(val: u32, buf: *mut u8) -> *mut u8 {
    let mut buf = buf;
    if val < 100 {
        /* 1-2 digits: aa */
        let lz = (val < 10) as usize;
        cpy2(buf, dtbl(val * 2 + lz as u32));
        buf = buf.sub(lz);
        buf.add(2)
    } else if val < 10000 {
        /* 3-4 digits: aabb */
        let aa = (val * 5243) >> 19;
        let bb = val - aa * 100;
        let lz = (aa < 10) as usize;
        cpy2(buf, dtbl(aa * 2 + lz as u32));
        buf = buf.sub(lz);
        cpy2(buf.add(2), dtbl(bb * 2));
        buf.add(4)
    } else if val < 1000000 {
        /* 5-6 digits: aabbcc */
        let aa = (((val as u64) * 429497) >> 32) as u32;
        let bbcc = val - aa * 10000;
        let bb = (bbcc * 5243) >> 19;
        let cc = bbcc - bb * 100;
        let lz = (aa < 10) as usize;
        cpy2(buf, dtbl(aa * 2 + lz as u32));
        buf = buf.sub(lz);
        cpy2(buf.add(2), dtbl(bb * 2));
        cpy2(buf.add(4), dtbl(cc * 2));
        buf.add(6)
    } else {
        /* 7-8 digits: aabbccdd */
        let aabb = (((val as u64) * 109951163) >> 40) as u32;
        let ccdd = val - aabb * 10000;
        let aa = (aabb * 5243) >> 19;
        let cc = (ccdd * 5243) >> 19;
        let bb = aabb - aa * 100;
        let dd = ccdd - cc * 100;
        let lz = (aa < 10) as usize;
        cpy2(buf, dtbl(aa * 2 + lz as u32));
        buf = buf.sub(lz);
        cpy2(buf.add(2), dtbl(bb * 2));
        cpy2(buf.add(4), dtbl(cc * 2));
        cpy2(buf.add(6), dtbl(dd * 2));
        buf.add(8)
    }
}

/// `write_u32_len_5_to_8`
#[inline(always)]
unsafe fn write_u32_len_5_to_8(val: u32, buf: *mut u8) -> *mut u8 {
    let mut buf = buf;
    if val < 1000000 {
        let aa = (((val as u64) * 429497) >> 32) as u32;
        let bbcc = val - aa * 10000;
        let bb = (bbcc * 5243) >> 19;
        let cc = bbcc - bb * 100;
        let lz = (aa < 10) as usize;
        cpy2(buf, dtbl(aa * 2 + lz as u32));
        buf = buf.sub(lz);
        cpy2(buf.add(2), dtbl(bb * 2));
        cpy2(buf.add(4), dtbl(cc * 2));
        buf.add(6)
    } else {
        let aabb = (((val as u64) * 109951163) >> 40) as u32;
        let ccdd = val - aabb * 10000;
        let aa = (aabb * 5243) >> 19;
        let cc = (ccdd * 5243) >> 19;
        let bb = aabb - aa * 100;
        let dd = ccdd - cc * 100;
        let lz = (aa < 10) as usize;
        cpy2(buf, dtbl(aa * 2 + lz as u32));
        buf = buf.sub(lz);
        cpy2(buf.add(2), dtbl(bb * 2));
        cpy2(buf.add(4), dtbl(cc * 2));
        cpy2(buf.add(6), dtbl(dd * 2));
        buf.add(8)
    }
}

/// `write_u64`
unsafe fn write_u64(val: u64, buf: *mut u8) -> *mut u8 {
    if val < 100000000 {
        /* 1-8 digits */
        write_u32_len_1_to_8(val as u32, buf)
    } else if val < 100000000u64 * 100000000 {
        /* 9-16 digits */
        let hgh = val / 100000000;
        let low = (val - hgh * 100000000) as u32;
        let buf = write_u32_len_1_to_8(hgh as u32, buf);
        write_u32_len_8(low, buf)
    } else {
        /* 17-20 digits */
        let tmp = val / 100000000;
        let low = (val - tmp * 100000000) as u32;
        let hgh = (tmp / 10000) as u32;
        let mid = (tmp - hgh as u64 * 10000) as u32;
        let buf = write_u32_len_5_to_8(hgh, buf);
        let buf = write_u32_len_4(mid, buf);
        write_u32_len_8(low, buf)
    }
}

/* ------------------------------------------------- floating-point writer */

/// `write_u64_len_1_to_16`
#[inline(always)]
unsafe fn write_u64_len_1_to_16(val: u64, buf: *mut u8) -> *mut u8 {
    if val < 100000000 {
        write_u32_len_1_to_8(val as u32, buf)
    } else {
        let hgh = val / 100000000;
        let low = (val - hgh * 100000000) as u32;
        let buf = write_u32_len_1_to_8(hgh as u32, buf);
        write_u32_len_8(low, buf)
    }
}

/// `write_u64_len_1_to_17`
#[inline(always)]
unsafe fn write_u64_len_1_to_17(val: u64, buf: *mut u8) -> *mut u8 {
    let mut buf = buf;
    if val >= 100000000u64 * 10000000 {
        /* len: 16 to 17 */
        let hgh = val / 100000000;
        let low = (val - hgh * 100000000) as u32;
        let one = (hgh / 100000000) as u32;
        let mid = (hgh - one as u64 * 100000000) as u32;
        *buf = one as u8 + b'0';
        buf = buf.add((one > 0) as usize);
        buf = write_u32_len_8(mid, buf);
        write_u32_len_8(low, buf)
    } else if val >= 100000000 {
        /* len: 9 to 15 */
        let hgh = val / 100000000;
        let low = (val - hgh * 100000000) as u32;
        let buf = write_u32_len_1_to_8(hgh as u32, buf);
        write_u32_len_8(low, buf)
    } else {
        /* len: 1 to 8 */
        write_u32_len_1_to_8(val as u32, buf)
    }
}

/// `write_u32_len_7_to_9_trim`
#[inline(always)]
unsafe fn write_u32_len_7_to_9_trim(val: u32, buf: *mut u8) -> *mut u8 {
    let mut buf = buf;
    let abbcc = val / 10000;
    let ddee = val - abbcc * 10000;
    let abb = (((abbcc as u64) * 167773) >> 24) as u32;
    let a = (abb * 41) >> 12;
    let bb = abb - a * 100;
    let cc = abbcc - abb * 100;

    *buf = a as u8 + b'0';
    buf = buf.add((a > 0) as usize);
    let lz = (bb < 10 && a == 0) as usize;
    cpy2(buf, dtbl(bb * 2 + lz as u32));
    buf = buf.sub(lz);
    cpy2(buf.add(2), dtbl(cc * 2));

    if ddee != 0 {
        let dd = (ddee * 5243) >> 19;
        let ee = ddee - dd * 100;
        cpy2(buf.add(4), dtbl(dd * 2));
        cpy2(buf.add(6), dtbl(ee * 2));
        let tz1 = DEC_TRAILING_ZERO_TABLE[dd as usize] as usize;
        let tz2 = DEC_TRAILING_ZERO_TABLE[ee as usize] as usize;
        let tz = if ee != 0 { tz2 } else { tz1 + 2 };
        buf.add(8 - tz)
    } else {
        let tz1 = DEC_TRAILING_ZERO_TABLE[bb as usize] as usize;
        let tz2 = DEC_TRAILING_ZERO_TABLE[cc as usize] as usize;
        let tz = if cc != 0 { tz2 } else { tz1 + tz2 };
        buf.add(4 - tz)
    }
}

/// `write_u64_len_16_to_17_trim`
#[inline(always)]
unsafe fn write_u64_len_16_to_17_trim(val: u64, buf: *mut u8) -> *mut u8 {
    let mut buf = buf;
    let abbccddee = (val / 100000000) as u32;
    let ffgghhii = (val - abbccddee as u64 * 100000000) as u32;
    let abbcc = abbccddee / 10000;
    let ddee = abbccddee - abbcc * 10000;
    let abb = (((abbcc as u64) * 167773) >> 24) as u32;
    let a = (abb * 41) >> 12;
    let bb = abb - a * 100;
    let cc = abbcc - abb * 100;
    *buf = a as u8 + b'0';
    buf = buf.add((a > 0) as usize);
    cpy2(buf, dtbl(bb * 2));
    cpy2(buf.add(2), dtbl(cc * 2));

    if ffgghhii != 0 {
        let dd = (ddee * 5243) >> 19;
        let ee = ddee - dd * 100;
        let ffgg = (((ffgghhii as u64) * 109951163) >> 40) as u32;
        let hhii = ffgghhii - ffgg * 10000;
        let ff = (ffgg * 5243) >> 19;
        let gg = ffgg - ff * 100;
        cpy2(buf.add(4), dtbl(dd * 2));
        cpy2(buf.add(6), dtbl(ee * 2));
        cpy2(buf.add(8), dtbl(ff * 2));
        cpy2(buf.add(10), dtbl(gg * 2));
        if hhii != 0 {
            let hh = (hhii * 5243) >> 19;
            let ii = hhii - hh * 100;
            cpy2(buf.add(12), dtbl(hh * 2));
            cpy2(buf.add(14), dtbl(ii * 2));
            let tz1 = DEC_TRAILING_ZERO_TABLE[hh as usize] as usize;
            let tz2 = DEC_TRAILING_ZERO_TABLE[ii as usize] as usize;
            let tz = if ii != 0 { tz2 } else { tz1 + 2 };
            buf.add(16 - tz)
        } else {
            let tz1 = DEC_TRAILING_ZERO_TABLE[ff as usize] as usize;
            let tz2 = DEC_TRAILING_ZERO_TABLE[gg as usize] as usize;
            let tz = if gg != 0 { tz2 } else { tz1 + 2 };
            buf.add(12 - tz)
        }
    } else if ddee != 0 {
        let dd = (ddee * 5243) >> 19;
        let ee = ddee - dd * 100;
        cpy2(buf.add(4), dtbl(dd * 2));
        cpy2(buf.add(6), dtbl(ee * 2));
        let tz1 = DEC_TRAILING_ZERO_TABLE[dd as usize] as usize;
        let tz2 = DEC_TRAILING_ZERO_TABLE[ee as usize] as usize;
        let tz = if ee != 0 { tz2 } else { tz1 + 2 };
        buf.add(8 - tz)
    } else {
        let tz1 = DEC_TRAILING_ZERO_TABLE[bb as usize] as usize;
        let tz2 = DEC_TRAILING_ZERO_TABLE[cc as usize] as usize;
        let tz = if cc != 0 { tz2 } else { tz1 + tz2 };
        buf.add(4 - tz)
    }
}

/// `write_f32_exp`: exponent part in range `e-45` to `e38`.
#[inline(always)]
unsafe fn write_f32_exp(exp: i32, buf: *mut u8) -> *mut u8 {
    cpy2_lit(buf, b"e-");
    let buf = buf.add(2 - (exp >= 0) as usize);
    let exp = if exp < 0 { -exp } else { exp };
    let lz = (exp < 10) as usize;
    cpy2(buf, dtbl(exp as u32 * 2 + lz as u32));
    buf.add(2 - lz)
}

/// `write_f64_exp`: exponent part in range `e-324` to `e308`.
#[inline(always)]
unsafe fn write_f64_exp(exp: i32, buf: *mut u8) -> *mut u8 {
    cpy2_lit(buf, b"e-");
    let buf = buf.add(2 - (exp >= 0) as usize);
    let exp = if exp < 0 { -exp } else { exp };
    if exp < 100 {
        let lz = (exp < 10) as usize;
        cpy2(buf, dtbl(exp as u32 * 2 + lz as u32));
        buf.add(2 - lz)
    } else {
        let hi = ((exp as u32) * 656) >> 16; /* exp / 100 */
        let lo = exp as u32 - hi * 100; /* exp % 100 */
        *buf = hi as u8 + b'0';
        cpy2(buf.add(1), dtbl(lo * 2));
        buf.add(3)
    }
}

/// `div_pow10`: returns `(div, mod, p10)`.
#[inline(always)]
fn div_pow10(num: u64, exp: u32) -> (u64, u64, u64) {
    let m = &DIV_POW10_TABLE[exp as usize];
    let (hi, _lo) = u128_mul(num >> m.shr1, m.mul);
    let div = hi >> m.shr2;
    let modv = num.wrapping_sub(div.wrapping_mul(m.p10));
    (div, modv, m.p10)
}

/// `u64_round_to_odd`
#[inline(always)]
fn u64_round_to_odd(u: u64, cp: u32) -> u32 {
    let (hi, lo) = u128_mul(cp as u64, u);
    let y_hi = hi as u32;
    let y_lo = (lo >> 32) as u32;
    y_hi | (y_lo > 1) as u32
}

/// `u128_round_to_odd`
#[inline(always)]
fn u128_round_to_odd(hi: u64, lo: u64, cp: u64) -> u64 {
    let (x_hi, _x_lo) = u128_mul(cp, lo);
    let (y_hi, y_lo) = u128_mul_add(cp, hi, x_hi);
    y_hi | (y_lo > 1) as u64
}

/// `pow10_table_get_sig`
#[inline(always)]
fn pow10_table_get_sig(exp10: i32) -> (u64, u64) {
    let idx = (exp10 - POW10_SIG_TABLE_MIN_EXP) as usize;
    (POW10_SIG_TABLE[idx * 2], POW10_SIG_TABLE[idx * 2 + 1])
}

/// `f32_bin_to_dec`
fn f32_bin_to_dec(sig_raw: u32, exp_raw: u32, sig_bin: u32, exp_bin: i32) -> (u32, i32) {
    /* Fast path, see `f64_bin_to_dec`. */
    if sig_raw != 0 {
        'fast: {
            let k = exp_bin.wrapping_mul(315653) >> 20;
            let h = exp_bin + ((-k).wrapping_mul(217707) >> 16);
            let (p10_hi, _p10_lo) = pow10_table_get_sig(-k);

            let cb = (sig_bin << (h + 1)) as u64;
            let (hi, lo) = u128_mul(cb, p10_hi);
            let s_hi = hi as u32;
            let s_lo = (lo >> 32) as u32;
            let modv = s_hi % 10;
            let dec = s_hi - modv;

            let c = (modv << (32 - 4)) | (s_lo >> 4);
            let half_ulp = (p10_hi >> (32 + 4 - h)) as u32;

            let w1_inside = s_lo >= (1u32 << 31);
            if s_lo == (1u32 << 31) {
                break 'fast;
            }
            let u0_inside = half_ulp >= c;
            if half_ulp == c {
                break 'fast;
            }
            let t0 = 10u32 << (32 - 4);
            let t1 = c.wrapping_add(half_ulp);
            let w0_inside = t1 >= t0;
            if t0.wrapping_sub(t1) <= 1 {
                break 'fast;
            }

            let trim = u0_inside | w0_inside;
            let add_10 = if w0_inside { 10 } else { 0 };
            let add_1 = modv + w1_inside as u32;
            return (dec + if trim { add_10 } else { add_1 }, k);
        }
    }

    /* Schubfach algorithm, see `f64_bin_to_dec`. */
    let irregular = sig_raw == 0 && exp_raw > 1;
    let is_even = (sig_bin & 1) == 0;
    let cbl = 4 * sig_bin - 2 + irregular as u32;
    let cb = 4 * sig_bin;
    let cbr = 4 * sig_bin + 2;

    let k = (exp_bin.wrapping_mul(315653) - if irregular { 131237 } else { 0 }) >> 20;
    let h = exp_bin + ((-k).wrapping_mul(217707) >> 16) + 1;
    let (mut p10_hi, _p10_lo) = pow10_table_get_sig(-k);
    p10_hi = p10_hi.wrapping_add(1);

    let vbl = u64_round_to_odd(p10_hi, cbl << h);
    let vb = u64_round_to_odd(p10_hi, cb << h);
    let vbr = u64_round_to_odd(p10_hi, cbr << h);
    let lower = vbl.wrapping_add(!is_even as u32);
    let upper = vbr.wrapping_sub(!is_even as u32);

    let s = vb / 4;
    if s >= 10 {
        let sp = s / 10;
        let u0_inside = lower <= 40 * sp;
        let w0_inside = upper >= 40 * sp + 40;
        if u0_inside != w0_inside {
            return (sp * 10 + if w0_inside { 10 } else { 0 }, k);
        }
    }
    let u1_inside = lower <= 4 * s;
    let w1_inside = upper >= 4 * s + 4;
    let mid = 4 * s + 2;
    let round_up = vb > mid || (vb == mid && (s & 1) != 0);
    let add = if u1_inside != w1_inside {
        w1_inside as u32
    } else {
        round_up as u32
    };
    (s + add, k)
}

/// `f64_bin_to_dec`
fn f64_bin_to_dec(sig_raw: u64, exp_raw: u32, sig_bin: u64, exp_bin: i32) -> (u64, i32) {
    /*
     Fast path: for a regularly spaced significand 'c' there are 4 candidates.
     If (c - 0.5ulp < u0), trim the last digit and round down;
     if (c + 0.5ulp > w0), trim the last digit and round up;
     if (c - 0.5ulp < u1), round down; if (c + 0.5ulp > w1), round up.
     */
    if sig_raw != 0 {
        'fast: {
            let k = exp_bin.wrapping_mul(315653) >> 20;
            let h = exp_bin + ((-k).wrapping_mul(217707) >> 16);
            let (p10_hi, p10_lo) = pow10_table_get_sig(-k);

            let cb = sig_bin << (h + 1);
            let (s_hi0, _s_lo0) = u128_mul(cb, p10_lo);
            let (s_hi, s_lo) = u128_mul_add(cb, p10_hi, s_hi0);
            let modv = s_hi % 10;
            let dec = s_hi - modv;

            let c = (modv << (64 - 4)) | (s_lo >> 4);
            let half_ulp = p10_hi >> (4 - h);

            let w1_inside = s_lo >= (1u64 << 63);
            if s_lo == (1u64 << 63) {
                break 'fast;
            }
            let u0_inside = half_ulp >= c;
            if half_ulp == c {
                break 'fast;
            }
            let t0 = 10u64 << (64 - 4);
            let t1 = c.wrapping_add(half_ulp);
            let w0_inside = t1 >= t0;
            if t0.wrapping_sub(t1) <= 1 {
                break 'fast;
            }

            let trim = u0_inside | w0_inside;
            let add_10 = if w0_inside { 10 } else { 0 };
            let add_1 = modv + w1_inside as u64;
            return (dec + if trim { add_10 } else { add_1 }, k);
        }
    }

    /*
     Schubfach algorithm:
     Raffaello Giulietti, The Schubfach way to render doubles, 2022.
     */
    let irregular = sig_raw == 0 && exp_raw > 1;
    let is_even = (sig_bin & 1) == 0;
    let cbl = 4 * sig_bin - 2 + irregular as u64;
    let cb = 4 * sig_bin;
    let cbr = 4 * sig_bin + 2;

    let k = (exp_bin.wrapping_mul(315653) - if irregular { 131237 } else { 0 }) >> 20;
    let h = exp_bin + ((-k).wrapping_mul(217707) >> 16) + 1;
    let (p10_hi, mut p10_lo) = pow10_table_get_sig(-k);
    p10_lo = p10_lo.wrapping_add(1);

    let vbl = u128_round_to_odd(p10_hi, p10_lo, cbl << h);
    let vb = u128_round_to_odd(p10_hi, p10_lo, cb << h);
    let vbr = u128_round_to_odd(p10_hi, p10_lo, cbr << h);
    let lower = vbl.wrapping_add(!is_even as u64);
    let upper = vbr.wrapping_sub(!is_even as u64);

    let s = vb / 4;
    if s >= 10 {
        let sp = s / 10;
        let u0_inside = lower <= 40 * sp;
        let w0_inside = upper >= 40 * sp + 40;
        if u0_inside != w0_inside {
            return (sp * 10 + if w0_inside { 10 } else { 0 }, k);
        }
    }
    let u1_inside = lower <= 4 * s;
    let w1_inside = upper >= 4 * s + 4;
    let mid = 4 * s + 2;
    let round_up = vb > mid || (vb == mid && (s & 1) != 0);
    let add = if u1_inside != w1_inside {
        w1_inside as u64
    } else {
        round_up as u64
    };
    (s + add, k)
}

/// `f64_bin_to_dec_fast`: returns `(sig_dec, exp_dec, round_up)`.
fn f64_bin_to_dec_fast(sig_raw: u64, exp_raw: u32, sig_bin: u64, exp_bin: i32) -> (u64, i32, bool) {
    let irregular = sig_raw == 0 && exp_raw > 1;

    let k = (exp_bin.wrapping_mul(315653) - if irregular { 131237 } else { 0 }) >> 20;
    let h = exp_bin + ((-k).wrapping_mul(217707) >> 16);
    let (p10_hi, p10_lo) = pow10_table_get_sig(-k);

    let cb = sig_bin << (h + 1);
    let (s_hi0, _s_lo0) = u128_mul(cb, p10_lo);
    let (s_hi, s_lo) = u128_mul_add(cb, p10_hi, s_hi0);

    let u = s_lo >= if irregular { 0x5555555555555555 } else { 1u64 << 63 };
    (s_hi + u as u64, k, u)
}

/// `write_inf_or_nan`
#[inline(always)]
unsafe fn write_inf_or_nan(buf: *mut u8, flg: u32, sig_raw: u64, sign: bool) -> *mut u8 {
    if has_flg(flg, YYJSON_WRITE_INF_AND_NAN_AS_NULL) {
        mem::copy(buf, b"null".as_ptr(), 4);
        return buf.add(4);
    }
    if has_flg(flg, YYJSON_WRITE_ALLOW_INF_AND_NAN) {
        if sig_raw == 0 {
            *buf = b'-';
            let buf = buf.add(sign as usize);
            mem::copy(buf, b"Infinity".as_ptr(), 8);
            return buf.add(8);
        } else {
            mem::copy(buf, b"NaN\0".as_ptr(), 4);
            return buf.add(3);
        }
    }
    ptr::null_mut()
}

/// `write_f32_raw`: requires a 40 byte buffer.
unsafe fn write_f32_raw(buf: *mut u8, raw_f64: u64, flg: u32) -> *mut u8 {
    /* cast double to float */
    let raw = f32_to_bits(f64_to_f32(f64_from_bits(raw_f64)));

    let sign = (raw >> (F32_BITS - 1)) != 0;
    let sig_raw = raw & F32_SIG_MASK;
    let exp_raw = (raw & F32_EXP_MASK) >> F32_SIG_BITS;

    if exp_raw == (1u32 << F32_EXP_BITS) - 1 {
        return write_inf_or_nan(buf, flg, sig_raw as u64, sign);
    }

    /* add sign for all finite number */
    *buf = b'-';
    let buf = buf.add(sign as usize);

    if (raw << 1) == 0 {
        mem::copy(buf, b"0.0\0".as_ptr(), 4);
        return buf.add(3);
    }

    if exp_raw != 0 {
        /* normal number */
        let sig_bin = sig_raw | (1u32 << F32_SIG_BITS);
        let exp_bin = exp_raw as i32 - F32_EXP_BIAS - F32_SIG_BITS as i32;

        /* fast path for small integer number without fraction */
        if (-(F32_SIG_BITS as i32) <= exp_bin && exp_bin <= 0)
            && u64_tz_bits(sig_bin as u64) >= (-exp_bin) as u32
        {
            let sig_dec = sig_bin >> -exp_bin;
            let buf = write_u32_len_1_to_8(sig_dec, buf);
            cpy2_lit(buf, b".0");
            return buf.add(2);
        }

        let (sig_dec, mut exp_dec) = f32_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);

        /* the sig length is 7 or 9 */
        let sig_len = 7 + (sig_dec >= 10000000) as i32 + (sig_dec >= 100000000) as i32;

        /* the decimal point offset relative to the first digit */
        let dot_ofs = sig_len + exp_dec;

        if -6 < dot_ofs && dot_ofs <= 21 {
            mem::set(buf, b'0', 32);
            let no_pre_zero = dot_ofs > 0;
            let pre_ofs = if no_pre_zero { 0 } else { 2 - dot_ofs };
            let num_hdr = buf.offset(pre_ofs as isize);
            let mut num_end = write_u32_len_7_to_9_trim(sig_dec, num_hdr);

            /* separate these digits to leave a space for the dot */
            let num_sep_pos = if no_pre_zero { dot_ofs } else { 0 };
            let num_sep = num_hdr.offset(num_sep_pos as isize);
            mem::move_(num_sep.add(no_pre_zero as usize), num_sep, 8);
            num_end = num_end.add(no_pre_zero as usize);

            let dot_set_pos = if dot_ofs > 1 { dot_ofs } else { 1 };
            *buf.offset(dot_set_pos as isize) = b'.';

            let dot_end = buf.offset(dot_ofs as isize + 2);
            if dot_end > num_end {
                dot_end
            } else {
                num_end
            }
        } else {
            /* write with scientific notation, e.g. 1.234e56 */
            let mut end = write_u32_len_7_to_9_trim(sig_dec, buf.add(1));
            end = end.sub((end == buf.add(2)) as usize);
            exp_dec += sig_len - 1;
            *buf = *buf.add(1);
            *buf.add(1) = b'.';
            write_f32_exp(exp_dec, end)
        }
    } else {
        /* subnormal number */
        let sig_bin = sig_raw;
        let exp_bin = 1 - F32_EXP_BIAS - F32_SIG_BITS as i32;

        let (sig_dec, mut exp_dec) = f32_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);

        let mut end = write_u32_len_1_to_8(sig_dec, buf.add(1));
        *buf = *buf.add(1);
        *buf.add(1) = b'.';
        exp_dec += (end as usize - buf as usize) as i32 - 2;

        /* trim trailing zeros */
        end = end.sub((*end.sub(1) == b'0') as usize);
        end = end.sub((*end.sub(1) == b'0') as usize);
        while *end.sub(1) == b'0' {
            end = end.sub(1);
        }
        end = end.sub((*end.sub(1) == b'.') as usize);

        write_f32_exp(exp_dec, end)
    }
}

/// `write_f64_raw`: requires a 40 byte buffer.
unsafe fn write_f64_raw(buf: *mut u8, raw: u64, flg: u32) -> *mut u8 {
    let sign = (raw >> (F64_BITS - 1)) != 0;
    let sig_raw = raw & F64_SIG_MASK;
    let exp_raw = ((raw & F64_EXP_MASK) >> F64_SIG_BITS) as u32;

    if exp_raw == (1u32 << F64_EXP_BITS) - 1 {
        return write_inf_or_nan(buf, flg, sig_raw, sign);
    }

    *buf = b'-';
    let buf = buf.add(sign as usize);

    if (raw << 1) == 0 {
        mem::copy(buf, b"0.0\0".as_ptr(), 4);
        return buf.add(3);
    }

    if exp_raw != 0 {
        /* normal number */
        let sig_bin = sig_raw | (1u64 << F64_SIG_BITS);
        let exp_bin = exp_raw as i32 - F64_EXP_BIAS - F64_SIG_BITS as i32;

        /* fast path for small integer number without fraction */
        if (-(F64_SIG_BITS as i32) <= exp_bin && exp_bin <= 0)
            && u64_tz_bits(sig_bin) >= (-exp_bin) as u32
        {
            let sig_dec = sig_bin >> -exp_bin;
            let buf = write_u64_len_1_to_16(sig_dec, buf);
            cpy2_lit(buf, b".0");
            return buf.add(2);
        }

        let (sig_dec, mut exp_dec) = f64_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);

        /* the sig length is 16 or 17 */
        let sig_len = 16 + (sig_dec >= 100000000u64 * 100000000) as i32;

        let dot_ofs = sig_len + exp_dec;

        if -6 < dot_ofs && dot_ofs <= 21 {
            mem::set(buf, b'0', 32);
            let no_pre_zero = dot_ofs > 0;
            let pre_ofs = if no_pre_zero { 0 } else { 2 - dot_ofs };
            let num_hdr = buf.offset(pre_ofs as isize);
            let mut num_end = write_u64_len_16_to_17_trim(sig_dec, num_hdr);

            let num_sep_pos = if no_pre_zero { dot_ofs } else { 0 };
            let num_sep = num_hdr.offset(num_sep_pos as isize);
            mem::move_(num_sep.add(no_pre_zero as usize), num_sep, 16);
            num_end = num_end.add(no_pre_zero as usize);

            let dot_set_pos = if dot_ofs > 1 { dot_ofs } else { 1 };
            *buf.offset(dot_set_pos as isize) = b'.';

            let dot_end = buf.offset(dot_ofs as isize + 2);
            if dot_end > num_end {
                dot_end
            } else {
                num_end
            }
        } else {
            let mut end = write_u64_len_16_to_17_trim(sig_dec, buf.add(1));
            end = end.sub((end == buf.add(2)) as usize);
            exp_dec += sig_len - 1;
            *buf = *buf.add(1);
            *buf.add(1) = b'.';
            write_f64_exp(exp_dec, end)
        }
    } else {
        /* subnormal number */
        let sig_bin = sig_raw;
        let exp_bin = 1 - F64_EXP_BIAS - F64_SIG_BITS as i32;

        let (sig_dec, mut exp_dec) = f64_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);

        let mut end = write_u64_len_1_to_17(sig_dec, buf.add(1));
        *buf = *buf.add(1);
        *buf.add(1) = b'.';
        exp_dec += (end as usize - buf as usize) as i32 - 2;

        end = end.sub((*end.sub(1) == b'0') as usize);
        end = end.sub((*end.sub(1) == b'0') as usize);
        while *end.sub(1) == b'0' {
            end = end.sub(1);
        }
        end = end.sub((*end.sub(1) == b'.') as usize);

        write_f64_exp(exp_dec, end)
    }
}

/// `write_f64_raw_fixed`: requires a 40 byte buffer.
unsafe fn write_f64_raw_fixed(buf: *mut u8, raw: u64, flg: u32, prec: u32) -> *mut u8 {
    let sign = (raw >> (F64_BITS - 1)) != 0;
    let sig_raw = raw & F64_SIG_MASK;
    let exp_raw = ((raw & F64_EXP_MASK) >> F64_SIG_BITS) as u32;

    if exp_raw == (1u32 << F64_EXP_BITS) - 1 {
        return write_inf_or_nan(buf, flg, sig_raw, sign);
    }

    *buf = b'-';
    let mut buf = buf.add(sign as usize);

    if (raw << 1) == 0 {
        mem::copy(buf, b"0.0\0".as_ptr(), 4);
        return buf.add(3);
    }

    if exp_raw != 0 {
        let sig_bin = sig_raw | (1u64 << F64_SIG_BITS);
        let exp_bin = exp_raw as i32 - F64_EXP_BIAS - F64_SIG_BITS as i32;

        /* fast path for small integer number without fraction */
        if (-(F64_SIG_BITS as i32) <= exp_bin && exp_bin <= 0)
            && u64_tz_bits(sig_bin) >= (-exp_bin) as u32
        {
            let sig_dec = sig_bin >> -exp_bin;
            let buf = write_u64_len_1_to_16(sig_dec, buf);
            cpy2_lit(buf, b".0");
            return buf.add(2);
        }

        /* only `fabs(num) < 1e21` is processed here */
        if (raw << 1) < (0x444B1AE4D6E2EF50u64 << 1) {
            let (mut sig_dec, mut exp_dec, round_up) =
                f64_bin_to_dec_fast(sig_raw, exp_raw, sig_bin, exp_bin);

            let mut sig_len = 16 + (sig_dec >= 100000000u64 * 100000000) as i32;

            /* limit the number of digits after the decimal point */
            if exp_dec < -1 {
                let sig_len_cut = -exp_dec - prec as i32;
                if sig_len_cut > sig_len {
                    mem::copy(buf, b"0.0\0".as_ptr(), 4);
                    return buf.add(3);
                }
                if sig_len_cut > 0 {
                    /* remove round up */
                    sig_dec -= round_up as u64;
                    sig_len = 16 + (sig_dec >= 100000000u64 * 100000000) as i32;

                    /* cut off some digits */
                    let (div, modv, p10) = div_pow10(sig_dec, sig_len_cut as u32);

                    /* add round up */
                    sig_dec = div + (modv >= p10 / 2) as u64;

                    /* update exp and sig length */
                    exp_dec += sig_len_cut;
                    sig_len -= sig_len_cut;
                    sig_len += ((sig_len >= 0)
                        && (sig_dec >= DIV_POW10_TABLE[sig_len as usize].p10))
                        as i32;
                }
                if sig_len <= 0 {
                    mem::copy(buf, b"0.0\0".as_ptr(), 4);
                    return buf.add(3);
                }
            }

            mem::set(buf, b'0', 32);

            let dot_ofs = sig_len + exp_dec;
            let no_pre_zero = dot_ofs > 0;
            let pre_ofs = if no_pre_zero { 0 } else { 1 - dot_ofs };
            let num_hdr = buf.offset(pre_ofs as isize);
            let mut num_end = write_u64_len_1_to_17(sig_dec, num_hdr);

            let num_sep_pos = if no_pre_zero { dot_ofs } else { -dot_ofs };
            let num_sep = buf.offset(num_sep_pos as isize);
            mem::move_(num_sep.add(1), num_sep, 16);
            num_end = num_end.add((exp_dec < 0) as usize);

            let dot_set_pos = if dot_ofs > 1 { dot_ofs } else { 1 };
            *buf.offset(dot_set_pos as isize) = b'.';

            /* remove trailing zeros */
            buf = buf.offset(dot_set_pos as isize + 2);
            if buf < num_end {
                buf = num_end;
            }
            buf = buf.sub((*buf.sub(1) == b'0') as usize);
            buf = buf.sub((*buf.sub(1) == b'0') as usize);
            while *buf.sub(1) == b'0' {
                buf = buf.sub(1);
            }
            buf = buf.add((*buf.sub(1) == b'.') as usize); /* keep a zero */
            buf
        } else {
            let (sig_dec, mut exp_dec) = f64_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);
            let sig_len = 16 + (sig_dec >= 100000000u64 * 100000000) as i32;

            let mut end = write_u64_len_16_to_17_trim(sig_dec, buf.add(1));
            end = end.sub((end == buf.add(2)) as usize);
            exp_dec += sig_len - 1;
            *buf = *buf.add(1);
            *buf.add(1) = b'.';
            write_f64_exp(exp_dec, end)
        }
    } else {
        /* subnormal number */
        mem::copy(buf, b"0.0\0".as_ptr(), 4);
        buf.add(3)
    }
}

/// `write_num`: requires a 40 byte buffer.
#[inline(always)]
unsafe fn write_num(cur: *mut u8, val: *const yyjson_val, flg: u32) -> *mut u8 {
    if ((*val).tag & YYJSON_SUBTYPE_REAL as u64) == 0 {
        let pos = (*val).uni.u64_;
        let neg = (!pos).wrapping_add(1);
        let sign = (((*val).tag & YYJSON_SUBTYPE_SINT as u64) > 0) & ((pos as i64) < 0);
        *cur = b'-';
        write_u64(if sign { neg } else { pos }, cur.add(sign as usize))
    } else {
        let raw = (*val).uni.u64_;
        let val_fmt = ((*val).tag >> 32) as u32;
        let all_fmt = flg;
        let fmt = val_fmt | all_fmt;
        if (fmt >> (32 - YYJSON_WRITE_FP_FLAG_BITS)) == 0 {
            /* double to shortest */
            write_f64_raw(cur, raw, flg)
        } else if (fmt >> (32 - YYJSON_WRITE_FP_PREC_BITS)) != 0 {
            /* double to fixed */
            let val_prec = val_fmt >> (32 - YYJSON_WRITE_FP_PREC_BITS);
            let all_prec = all_fmt >> (32 - YYJSON_WRITE_FP_PREC_BITS);
            let prec = if val_prec != 0 { val_prec } else { all_prec };
            write_f64_raw_fixed(cur, raw, flg, prec)
        } else if (fmt & YYJSON_WRITE_FP_TO_FLOAT) != 0 {
            /* float to shortest */
            write_f32_raw(cur, raw, flg)
        } else {
            /* double to shortest */
            write_f64_raw(cur, raw, flg)
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_number(
    val: *const yyjson_val,
    buf: *mut c_char,
) -> *mut c_char {
    if val.is_null() || buf.is_null() {
        return ptr::null_mut();
    }
    let tag = ((*val).tag & YYJSON_TAG_MASK as u64) as u8;
    if tag == YYJSON_TYPE_NUM | YYJSON_SUBTYPE_UINT {
        let b = write_u64((*val).uni.u64_, buf as *mut u8);
        *b = 0;
        b as *mut c_char
    } else if tag == YYJSON_TYPE_NUM | YYJSON_SUBTYPE_SINT {
        let pos = (*val).uni.u64_;
        let neg = (!pos).wrapping_add(1);
        let sign = ((pos as i64) < 0) as usize;
        *(buf as *mut u8) = b'-';
        let b = write_u64(
            if sign != 0 { neg } else { pos },
            (buf as *mut u8).add(sign),
        );
        *b = 0;
        b as *mut c_char
    } else if tag == YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL {
        let raw = (*val).uni.u64_;
        let fmt = ((*val).tag >> 32) as u32;
        let flg = YYJSON_WRITE_ALLOW_INF_AND_NAN;
        let b = if (fmt >> (32 - YYJSON_WRITE_FP_FLAG_BITS)) == 0 {
            write_f64_raw(buf as *mut u8, raw, flg)
        } else if (fmt >> (32 - YYJSON_WRITE_FP_PREC_BITS)) != 0 {
            let prec = fmt >> (32 - YYJSON_WRITE_FP_PREC_BITS);
            write_f64_raw_fixed(buf as *mut u8, raw, flg, prec)
        } else if (fmt & YYJSON_WRITE_FP_TO_FLOAT) != 0 {
            write_f32_raw(buf as *mut u8, raw, flg)
        } else {
            write_f64_raw(buf as *mut u8, raw, flg)
        };
        if !b.is_null() {
            *b = 0;
        }
        b as *mut c_char
    } else {
        ptr::null_mut()
    }
}

/* ------------------------------------------------------- string writer */

const CHAR_ENC_CPY_1: u8 = 0;
const CHAR_ENC_ERR_1: u8 = 1;
const CHAR_ENC_ESC_A: u8 = 2;
const CHAR_ENC_ESC_1: u8 = 3;
const CHAR_ENC_CPY_2: u8 = 4;
const CHAR_ENC_ESC_2: u8 = 5;
const CHAR_ENC_CPY_3: u8 = 6;
const CHAR_ENC_ESC_3: u8 = 7;
const CHAR_ENC_CPY_4: u8 = 8;
const CHAR_ENC_ESC_4: u8 = 9;

/// `get_hex_table_with_flag`
#[inline(always)]
fn get_hex_table_with_flag(flg: u32) -> &'static [u8; 512] {
    if has_flg(flg, YYJSON_WRITE_LOWERCASE_HEX) {
        &ESC_HEX_CHAR_TABLE_LOWER
    } else {
        &ESC_HEX_CHAR_TABLE
    }
}

/// `get_enc_table_with_flag`
#[inline(always)]
fn get_enc_table_with_flag(flg: u32) -> &'static [u8; 256] {
    if has_flg(flg, YYJSON_WRITE_ESCAPE_UNICODE) {
        if has_flg(flg, YYJSON_WRITE_ESCAPE_SLASHES) {
            &ENC_TABLE_ESC_SLASH
        } else {
            &ENC_TABLE_ESC
        }
    } else if has_flg(flg, YYJSON_WRITE_ESCAPE_SLASHES) {
        &ENC_TABLE_CPY_SLASH
    } else {
        &ENC_TABLE_CPY
    }
}

/// `write_raw`
#[inline(always)]
unsafe fn write_raw(cur: *mut u8, raw: *const u8, raw_len: usize) -> *mut u8 {
    mem::copy(cur, raw, raw_len);
    cur.add(raw_len)
}

/// `write_str_noesc`
#[inline(always)]
unsafe fn write_str_noesc(cur: *mut u8, str_: *const u8, str_len: usize) -> *mut u8 {
    let mut cur = cur;
    let mut str_ = str_;
    let mut str_len = str_len;
    *cur = b'"';
    cur = cur.add(1);
    while str_len >= 16 {
        mem::copy(cur, str_, 16);
        cur = cur.add(16);
        str_ = str_.add(16);
        str_len -= 16;
    }
    while str_len >= 4 {
        mem::copy(cur, str_, 4);
        cur = cur.add(4);
        str_ = str_.add(4);
        str_len -= 4;
    }
    while str_len != 0 {
        *cur = *str_;
        cur = cur.add(1);
        str_ = str_.add(1);
        str_len -= 1;
    }
    *cur = b'"';
    cur.add(1)
}

/// `write_str`: requires a `len * 6 + 2` byte buffer; returns NULL on invalid
/// unicode when `inv` is false.
unsafe fn write_str(
    cur: *mut u8,
    esc: bool,
    inv: bool,
    str_: *const u8,
    str_len: usize,
    enc_table: &[u8; 256],
    hex_table: &[u8; 512],
) -> *mut u8 {
    /* the `\u00` prefix of an escape sequence */
    const PRE: [u8; 4] = [b'\\', b'u', b'0', b'0'];

    let mut cur = cur;
    let mut src = str_;
    let end = str_.add(str_len);
    *cur = b'"';
    cur = cur.add(1);

    enum S {
        CopyAscii,
        CopyUtf8,
    }
    let mut state = S::CopyAscii;

    macro_rules! err_cpy {
        () => {{
            if !inv {
                return ptr::null_mut();
            }
            *cur = *src;
            cur = cur.add(1);
            src = src.add(1);
            state = S::CopyUtf8;
            continue;
        }};
    }
    macro_rules! err_esc {
        () => {{
            if !inv {
                return ptr::null_mut();
            }
            /* U+FFFD, written through `hex_table` so that LOWERCASE_HEX
               produces "fffd" */
            mem::copy(cur, PRE.as_ptr(), 2);
            cpy2(cur.add(2), hex_table.as_ptr().add(0xFF * 2));
            cpy2(cur.add(4), hex_table.as_ptr().add(0xFD * 2));
            cur = cur.add(6);
            src = src.add(1);
            state = S::CopyUtf8;
            continue;
        }};
    }
    macro_rules! err_one {
        () => {{
            if esc {
                err_esc!();
            } else {
                err_cpy!();
            }
        }};
    }

    loop {
        match state {
            S::CopyAscii => {
                /* Copy continuous ASCII, loop unrolling as in the C source. */
                let mut stop: Option<usize> = None;
                'ascii: {
                    while (end as usize - src as usize) >= 16 {
                        let mut i = 0usize;
                        while i < 16 {
                            if enc_table[*src.add(i) as usize] != 0 {
                                stop = Some(i);
                                break 'ascii;
                            }
                            i += 1;
                        }
                        mem::copy(cur, src, 16);
                        cur = cur.add(16);
                        src = src.add(16);
                    }
                    while (end as usize - src as usize) >= 4 {
                        let mut i = 0usize;
                        while i < 4 {
                            if enc_table[*src.add(i) as usize] != 0 {
                                stop = Some(i);
                                break 'ascii;
                            }
                            i += 1;
                        }
                        mem::copy(cur, src, 4);
                        cur = cur.add(4);
                        src = src.add(4);
                    }
                    while end > src {
                        if enc_table[*src as usize] != 0 {
                            stop = Some(0);
                            break 'ascii;
                        }
                        *cur = *src;
                        cur = cur.add(1);
                        src = src.add(1);
                    }
                }
                match stop {
                    None => {
                        *cur = b'"';
                        return cur.add(1);
                    }
                    Some(i) => {
                        /* `stop_char_i` */
                        mem::copy(cur, src, i);
                        cur = cur.add(i);
                        src = src.add(i);
                        state = S::CopyUtf8;
                    }
                }
            }

            S::CopyUtf8 => {
                if src.add(4) > end {
                    if end == src {
                        /* `copy_end` */
                        *cur = b'"';
                        return cur.add(1);
                    }
                    if (end as usize - src as usize) < (enc_table[*src as usize] / 2) as usize {
                        err_one!();
                    }
                }
                match enc_table[*src as usize] {
                    CHAR_ENC_CPY_1 => {
                        *cur = *src;
                        cur = cur.add(1);
                        src = src.add(1);
                        state = S::CopyAscii;
                    }
                    CHAR_ENC_CPY_2 => {
                        let uni = u32::from_le_bytes([*src, *src.add(1), 0, 0]);
                        if !is_utf8_seq2(uni) {
                            err_cpy!();
                        }
                        mem::copy(cur, uni.to_le_bytes().as_ptr(), 2);
                        cur = cur.add(2);
                        src = src.add(2);
                    }
                    CHAR_ENC_CPY_3 => {
                        if src.add(4) <= end {
                            let uni = byte_load_4(src);
                            if !is_utf8_seq3(uni) {
                                err_cpy!();
                            }
                            mem::copy(cur, src, 4);
                        } else {
                            let uni = byte_load_3(src);
                            if !is_utf8_seq3(uni) {
                                err_cpy!();
                            }
                            mem::copy(cur, uni.to_le_bytes().as_ptr(), 4);
                        }
                        cur = cur.add(3);
                        src = src.add(3);
                    }
                    CHAR_ENC_CPY_4 => {
                        let uni = byte_load_4(src);
                        if !is_utf8_seq4(uni) {
                            err_cpy!();
                        }
                        mem::copy(cur, src, 4);
                        cur = cur.add(4);
                        src = src.add(4);
                    }
                    CHAR_ENC_ESC_A => {
                        cpy2(cur, ESC_SINGLE_CHAR_TABLE.as_ptr().add(*src as usize * 2));
                        cur = cur.add(2);
                        src = src.add(1);
                    }
                    CHAR_ENC_ESC_1 => {
                        mem::copy(cur, PRE.as_ptr(), 4);
                        cpy2(cur.add(4), hex_table.as_ptr().add(*src as usize * 2));
                        cur = cur.add(6);
                        src = src.add(1);
                    }
                    CHAR_ENC_ESC_2 => {
                        let v4 = u32::from_le_bytes([*src, *src.add(1), 0, 0]);
                        if !is_utf8_seq2(v4) {
                            err_esc!();
                        }
                        let u = (((*src & 0x1F) as u16) << 6) | ((*src.add(1) & 0x3F) as u16);
                        mem::copy(cur, PRE.as_ptr(), 2);
                        cpy2(cur.add(2), hex_table.as_ptr().add((u >> 8) as usize * 2));
                        cpy2(cur.add(4), hex_table.as_ptr().add((u & 0xFF) as usize * 2));
                        cur = cur.add(6);
                        src = src.add(2);
                    }
                    CHAR_ENC_ESC_3 => {
                        let v = byte_load_3(src);
                        if !is_utf8_seq3(v) {
                            err_esc!();
                        }
                        let u = (((*src & 0x0F) as u16) << 12)
                            | (((*src.add(1) & 0x3F) as u16) << 6)
                            | ((*src.add(2) & 0x3F) as u16);
                        mem::copy(cur, PRE.as_ptr(), 2);
                        cpy2(cur.add(2), hex_table.as_ptr().add((u >> 8) as usize * 2));
                        cpy2(cur.add(4), hex_table.as_ptr().add((u & 0xFF) as usize * 2));
                        cur = cur.add(6);
                        src = src.add(3);
                    }
                    CHAR_ENC_ESC_4 => {
                        let v = byte_load_4(src);
                        if !is_utf8_seq4(v) {
                            err_esc!();
                        }
                        let mut u = (((*src & 0x07) as u32) << 18)
                            | (((*src.add(1) & 0x3F) as u32) << 12)
                            | (((*src.add(2) & 0x3F) as u32) << 6)
                            | ((*src.add(3) & 0x3F) as u32);
                        u -= 0x10000;
                        let hi = (u >> 10) + 0xD800;
                        let lo = (u & 0x3FF) + 0xDC00;
                        mem::copy(cur, PRE.as_ptr(), 2);
                        cpy2(cur.add(2), hex_table.as_ptr().add((hi >> 8) as usize * 2));
                        cpy2(cur.add(4), hex_table.as_ptr().add((hi & 0xFF) as usize * 2));
                        mem::copy(cur.add(6), PRE.as_ptr(), 2);
                        cpy2(cur.add(8), hex_table.as_ptr().add((lo >> 8) as usize * 2));
                        cpy2(cur.add(10), hex_table.as_ptr().add((lo & 0xFF) as usize * 2));
                        cur = cur.add(12);
                        src = src.add(4);
                    }
                    CHAR_ENC_ERR_1 => {
                        err_one!();
                    }
                    _ => {}
                }
            }
        }
    }
}

/* ------------------------------------------------------ writer utilities */

/// `write_null`: requires an 8 byte buffer.
#[inline(always)]
unsafe fn write_null(cur: *mut u8) -> *mut u8 {
    mem::copy(cur, b"null,\n\0\0".as_ptr(), 8);
    cur.add(4)
}

/// `write_bool`: requires an 8 byte buffer.
#[inline(always)]
unsafe fn write_bool(cur: *mut u8, val: bool) -> *mut u8 {
    if val {
        mem::copy(cur, b"true,\n\0\0".as_ptr(), 8);
    } else {
        mem::copy(cur, b"false,\n\0".as_ptr(), 8);
    }
    cur.add(5 - val as usize)
}

/// `write_indent`: requires `level * 4` bytes; `spaces` must not exceed 4.
#[inline(always)]
unsafe fn write_indent(cur: *mut u8, level: usize, spaces: usize) -> *mut u8 {
    let mut cur = cur;
    let mut level = level;
    while level > 0 {
        level -= 1;
        mem::copy(cur, b"    ".as_ptr(), 4);
        cur = cur.add(spaces);
    }
    cur
}

/// `write_dat_to_fp`
unsafe fn write_dat_to_fp(
    fp: *mut FILE,
    dat: *mut u8,
    len: usize,
    err: *mut yyjson_write_err,
) -> bool {
    if fwrite(dat as *const c_void, 1, len, fp) != len {
        (*err).msg = b"file writing failed\0".as_ptr() as *const c_char;
        (*err).code = YYJSON_WRITE_ERROR_FILE_WRITE;
        return false;
    }
    true
}

/// `write_dat_to_file`
unsafe fn write_dat_to_file(
    path: *const c_char,
    dat: *mut u8,
    len: usize,
    err: *mut yyjson_write_err,
) -> bool {
    let fp = file::fopen_writeonly(path);
    if fp.is_null() {
        (*err).msg = crate::util::msg(MSG_FOPEN);
        (*err).code = YYJSON_WRITE_ERROR_FILE_OPEN;
        return false;
    }
    if fwrite(dat as *const c_void, 1, len, fp) != len {
        (*err).msg = crate::util::msg(MSG_FWRITE);
        (*err).code = YYJSON_WRITE_ERROR_FILE_WRITE;
        fclose(fp);
        return false;
    }
    if fclose(fp) != 0 {
        /* the C version clears its `file` local before `return_err` so that
           the error path does not close the stream a second time */
        (*err).msg = crate::util::msg(MSG_FCLOSE);
        (*err).code = YYJSON_WRITE_ERROR_FILE_WRITE;
        return false;
    }
    true
}

/* -------------------------------------------------- writer implementation */

/// `yyjson_write_ctx`
#[repr(C)]
#[derive(Copy, Clone)]
struct WriteCtx {
    tag: usize,
}

/// `yyjson_mut_write_ctx`
#[repr(C)]
#[derive(Copy, Clone)]
struct MutWriteCtx {
    tag: usize,
    ctn: *mut yyjson_mut_val,
}

/// The `fail_*` labels of the root writers.
#[derive(Copy, Clone)]
enum WFail {
    Alloc,
    Type,
    Num,
    Str,
}

impl WFail {
    unsafe fn apply(self, err: *mut yyjson_write_err) {
        let (code, msg) = match self {
            WFail::Alloc => (
                YYJSON_WRITE_ERROR_MEMORY_ALLOCATION,
                crate::util::msg(MSG_MALLOC),
            ),
            WFail::Type => (
                YYJSON_WRITE_ERROR_INVALID_VALUE_TYPE,
                crate::util::msg(MSG_ERR_TYPE),
            ),
            WFail::Num => (YYJSON_WRITE_ERROR_NAN_OR_INF, crate::util::msg(MSG_NAN_INF)),
            WFail::Str => (
                YYJSON_WRITE_ERROR_INVALID_STRING,
                crate::util::msg(MSG_ERR_UTF8),
            ),
        };
        (*err).code = code;
        (*err).msg = msg;
    }
}

/// `write_root_single`
unsafe fn write_root_single(
    val: *mut yyjson_val,
    flg: u32,
    alc: yyjson_alc,
    buf: *mut c_char,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut u8 {
    let enc_table = get_enc_table_with_flag(flg);
    let hex_table = get_hex_table_with_flag(flg);
    let cpy = ptr::eq(enc_table, &ENC_TABLE_CPY);
    let esc = has_flg(flg, YYJSON_WRITE_ESCAPE_UNICODE);
    let inv = has_flg(flg, YYJSON_WRITE_ALLOW_INVALID_UNICODE);
    let newline = has_flg(flg, YYJSON_WRITE_NEWLINE_AT_END);
    const END_LEN: usize = 2; /* '\n' and '\0' */

    let mut hdr: *mut u8 = ptr::null_mut();
    let mut cur: *mut u8;

    let fail: WFail = 'fsm: {
        macro_rules! incr_len {
            ($len:expr) => {{
                let l: usize = $len;
                if !buf.is_null() {
                    hdr = if *dat_len >= l {
                        buf as *mut u8
                    } else {
                        ptr::null_mut()
                    };
                } else {
                    hdr = alc.malloc(l) as *mut u8;
                }
                if hdr.is_null() {
                    break 'fsm WFail::Alloc;
                }
                cur = hdr;
            }};
        }
        macro_rules! check_str_len {
            ($len:expr) => {{
                if size_of::<usize>() < 8 && $len >= (USIZE_MAX - 16) / 6 {
                    break 'fsm WFail::Alloc;
                }
            }};
        }

        match unsafe_get_type(val) {
            YYJSON_TYPE_RAW => {
                let str_len = unsafe_get_len(val);
                let str_ptr = (*val).uni.str_ as *const u8;
                check_str_len!(str_len);
                incr_len!(str_len + END_LEN);
                cur = write_raw(cur, str_ptr, str_len);
            }
            YYJSON_TYPE_STR => {
                let str_len = unsafe_get_len(val);
                let str_ptr = (*val).uni.str_ as *const u8;
                check_str_len!(str_len);
                incr_len!(str_len * 6 + 2 + END_LEN);
                if cpy && unsafe_get_subtype(val) != 0 {
                    cur = write_str_noesc(cur, str_ptr, str_len);
                } else {
                    cur = write_str(cur, esc, inv, str_ptr, str_len, enc_table, hex_table);
                    if cur.is_null() {
                        break 'fsm WFail::Str;
                    }
                }
            }
            YYJSON_TYPE_NUM => {
                incr_len!(FP_BUF_LEN + END_LEN);
                cur = write_num(cur, val, flg);
                if cur.is_null() {
                    break 'fsm WFail::Num;
                }
            }
            YYJSON_TYPE_BOOL => {
                incr_len!(8);
                let b = (unsafe_get_tag(val) & YYJSON_SUBTYPE_MASK) >> YYJSON_TYPE_BIT != 0;
                cur = write_bool(cur, b);
            }
            YYJSON_TYPE_NULL => {
                incr_len!(8);
                cur = write_null(cur);
            }
            YYJSON_TYPE_ARR => {
                incr_len!(2 + END_LEN);
                cpy2_lit(cur, b"[]");
                cur = cur.add(2);
            }
            YYJSON_TYPE_OBJ => {
                incr_len!(2 + END_LEN);
                cpy2_lit(cur, b"{}");
                cur = cur.add(2);
            }
            _ => break 'fsm WFail::Type,
        }

        if newline {
            *cur = b'\n';
            cur = cur.add(1);
        }
        *cur = 0;
        *dat_len = cur as usize - hdr as usize;
        mem::set(err as *mut u8, 0, size_of::<yyjson_write_err>());
        return hdr;
    };

    if !hdr.is_null() {
        alc.free(hdr as *mut c_void);
    }
    *dat_len = 0;
    fail.apply(err);
    ptr::null_mut()
}

/// The `goto` labels of the container writers.
#[derive(Copy, Clone, PartialEq)]
enum WState {
    ValBegin,
    ValEnd,
    CtnEnd,
    DocEnd,
}

/// `write_root_minify` (`PRETTY == false`) and `write_root_pretty`
/// (`PRETTY == true`). The root must be a non-empty container.
unsafe fn write_root_ctn<const PRETTY: bool>(
    root: *const yyjson_val,
    flg: u32,
    alc: yyjson_alc,
    buf: *mut c_char,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut u8 {
    let enc_table = get_enc_table_with_flag(flg);
    let hex_table = get_hex_table_with_flag(flg);
    let cpy = ptr::eq(enc_table, &ENC_TABLE_CPY);
    let esc = has_flg(flg, YYJSON_WRITE_ESCAPE_UNICODE);
    let inv = has_flg(flg, YYJSON_WRITE_ALLOW_INVALID_UNICODE);
    let spaces = if has_flg(flg, YYJSON_WRITE_PRETTY_TWO_SPACES) {
        2
    } else {
        4
    };
    let newline = has_flg(flg, YYJSON_WRITE_NEWLINE_AT_END);

    let mut hdr: *mut u8 = ptr::null_mut();
    let mut alc_len: usize;

    let fail: WFail = 'fsm: {
        if !buf.is_null() {
            hdr = buf as *mut u8;
            alc_len = *dat_len;
            alc_len = size_align_down(alc_len, size_of::<WriteCtx>());
            if alc_len <= size_of::<WriteCtx>() {
                break 'fsm WFail::Alloc;
            }
        } else {
            alc_len = (*root).uni.ofs / size_of::<yyjson_val>();
            alc_len = alc_len
                * if PRETTY {
                    YYJSON_WRITER_ESTIMATED_PRETTY_RATIO
                } else {
                    YYJSON_WRITER_ESTIMATED_MINIFY_RATIO
                }
                + 64;
            alc_len = size_align_up(alc_len, size_of::<WriteCtx>());
            hdr = alc.malloc(alc_len) as *mut u8;
            if hdr.is_null() {
                break 'fsm WFail::Alloc;
            }
        }
        let mut cur = hdr;
        let mut end = hdr.add(alc_len);
        let mut ctx = end as *mut WriteCtx;

        macro_rules! incr_len {
            ($len:expr) => {{
                let ext_len: usize = $len;
                if cur.add(ext_len) >= ctx as *mut u8 {
                    let ctx_pos = ctx as usize - hdr as usize;
                    let cur_pos = cur as usize - hdr as usize;
                    let ctx_len = end as usize - ctx as usize;
                    let mut alc_inc = if alc_len / 2 > ext_len {
                        alc_len / 2
                    } else {
                        ext_len
                    };
                    alc_inc = size_align_up(alc_inc, size_of::<WriteCtx>());
                    if size_of::<usize>() < 8 && size_add_is_overflow(alc_len, alc_inc) {
                        break 'fsm WFail::Alloc;
                    }
                    alc_len += alc_inc;
                    let tmp = alc.realloc(hdr as *mut c_void, alc_len - alc_inc, alc_len) as *mut u8;
                    if tmp.is_null() {
                        break 'fsm WFail::Alloc;
                    }
                    let ctx_tmp = tmp.add(alc_len - ctx_len) as *mut WriteCtx;
                    mem::move_(ctx_tmp as *mut u8, tmp.add(ctx_pos), ctx_len);
                    ctx = ctx_tmp;
                    cur = tmp.add(cur_pos);
                    end = tmp.add(alc_len);
                    hdr = tmp;
                }
            }};
        }
        macro_rules! check_str_len {
            ($len:expr) => {{
                if size_of::<usize>() < 8 && $len >= (USIZE_MAX - 16) / 6 {
                    break 'fsm WFail::Alloc;
                }
            }};
        }

        /* doc_begin */
        let mut val = root as *mut yyjson_val;
        let mut val_type = unsafe_get_type(val);
        let mut ctn_obj = val_type == YYJSON_TYPE_OBJ;
        let mut ctn_len = unsafe_get_len(val) << ctn_obj as u32;
        *cur = b'[' | ((ctn_obj as u8) << 5);
        cur = cur.add(1);
        if PRETTY {
            *cur = b'\n';
            cur = cur.add(1);
        }
        val = val.add(1);
        let mut level: usize = 1;

        let mut state = WState::ValBegin;
        loop {
            match state {
                WState::ValBegin => {
                    val_type = unsafe_get_type(val);
                    let no_indent = (ctn_obj as u8 & ctn_len as u8) != 0;
                    if val_type == YYJSON_TYPE_STR {
                        let is_key = (ctn_obj as u8 & !(ctn_len as u8)) != 0;
                        let str_len = unsafe_get_len(val);
                        let str_ptr = (*val).uni.str_ as *const u8;
                        check_str_len!(str_len);
                        if PRETTY {
                            incr_len!(str_len * 6 + 16 + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(str_len * 6 + 16);
                        }
                        if cpy && unsafe_get_subtype(val) != 0 {
                            cur = write_str_noesc(cur, str_ptr, str_len);
                        } else {
                            cur = write_str(cur, esc, inv, str_ptr, str_len, enc_table, hex_table);
                            if cur.is_null() {
                                break 'fsm WFail::Str;
                            }
                        }
                        *cur = if is_key { b':' } else { b',' };
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = if is_key { b' ' } else { b'\n' };
                            cur = cur.add(1);
                        }
                        state = WState::ValEnd;
                        continue;
                    }
                    if val_type == YYJSON_TYPE_NUM {
                        if PRETTY {
                            incr_len!(FP_BUF_LEN + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(FP_BUF_LEN);
                        }
                        cur = write_num(cur, val, flg);
                        if cur.is_null() {
                            break 'fsm WFail::Num;
                        }
                        *cur = b',';
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        state = WState::ValEnd;
                        continue;
                    }
                    if (val_type & (YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ))
                        == (YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ)
                    {
                        let ctn_len_tmp = unsafe_get_len(val);
                        let ctn_obj_tmp = val_type == YYJSON_TYPE_OBJ;
                        if PRETTY {
                            incr_len!(
                                2 * size_of::<WriteCtx>() + if no_indent { 0 } else { level * 4 }
                            );
                        } else {
                            incr_len!(2 * size_of::<WriteCtx>());
                        }
                        if ctn_len_tmp == 0 {
                            /* write empty container */
                            if PRETTY {
                                cur =
                                    write_indent(cur, if no_indent { 0 } else { level }, spaces);
                            }
                            *cur = b'[' | ((ctn_obj_tmp as u8) << 5);
                            *cur.add(1) = b']' | ((ctn_obj_tmp as u8) << 5);
                            *cur.add(2) = b',';
                            cur = cur.add(3);
                            if PRETTY {
                                *cur = b'\n';
                                cur = cur.add(1);
                            }
                            state = WState::ValEnd;
                            continue;
                        } else {
                            /* push context, setup new container */
                            ctx = ctx.sub(1);
                            (*ctx).tag = (ctn_len << 1) | ctn_obj as usize;
                            ctn_len = ctn_len_tmp << ctn_obj_tmp as u32;
                            ctn_obj = ctn_obj_tmp;
                            if PRETTY {
                                cur =
                                    write_indent(cur, if no_indent { 0 } else { level }, spaces);
                                level += 1;
                            }
                            *cur = b'[' | ((ctn_obj as u8) << 5);
                            cur = cur.add(1);
                            if PRETTY {
                                *cur = b'\n';
                                cur = cur.add(1);
                            }
                            val = val.add(1);
                            continue;
                        }
                    }
                    if val_type == YYJSON_TYPE_BOOL {
                        if PRETTY {
                            incr_len!(16 + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(16);
                        }
                        let b = (unsafe_get_tag(val) & YYJSON_SUBTYPE_MASK) >> YYJSON_TYPE_BIT != 0;
                        cur = write_bool(cur, b);
                        cur = cur.add(if PRETTY { 2 } else { 1 });
                        state = WState::ValEnd;
                        continue;
                    }
                    if val_type == YYJSON_TYPE_NULL {
                        if PRETTY {
                            incr_len!(16 + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(16);
                        }
                        cur = write_null(cur);
                        cur = cur.add(if PRETTY { 2 } else { 1 });
                        state = WState::ValEnd;
                        continue;
                    }
                    if val_type == YYJSON_TYPE_RAW {
                        let str_len = unsafe_get_len(val);
                        let str_ptr = (*val).uni.str_ as *const u8;
                        check_str_len!(str_len);
                        if PRETTY {
                            incr_len!(str_len + 3 + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(str_len + 2);
                        }
                        cur = write_raw(cur, str_ptr, str_len);
                        *cur = b',';
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        state = WState::ValEnd;
                        continue;
                    }
                    break 'fsm WFail::Type;
                }

                WState::ValEnd => {
                    val = val.add(1);
                    ctn_len -= 1;
                    state = if ctn_len == 0 {
                        WState::CtnEnd
                    } else {
                        WState::ValBegin
                    };
                }

                WState::CtnEnd => {
                    if PRETTY {
                        cur = cur.sub(2);
                        *cur = b'\n';
                        cur = cur.add(1);
                        incr_len!(level * 4);
                        level -= 1;
                        cur = write_indent(cur, level, spaces);
                        *cur = b']' | ((ctn_obj as u8) << 5);
                        cur = cur.add(1);
                        if ctx as *mut u8 >= end {
                            state = WState::DocEnd;
                            continue;
                        }
                        let tag = (*ctx).tag;
                        ctx = ctx.add(1);
                        ctn_len = tag >> 1;
                        ctn_obj = (tag & 1) != 0;
                        ctn_len -= 1;
                        *cur = b',';
                        *cur.add(1) = b'\n';
                        cur = cur.add(2);
                    } else {
                        cur = cur.sub(1);
                        *cur = b']' | ((ctn_obj as u8) << 5);
                        *cur.add(1) = b',';
                        cur = cur.add(2);
                        if ctx as *mut u8 >= end {
                            state = WState::DocEnd;
                            continue;
                        }
                        let tag = (*ctx).tag;
                        ctx = ctx.add(1);
                        ctn_len = tag >> 1;
                        ctn_obj = (tag & 1) != 0;
                        ctn_len -= 1;
                    }
                    state = if ctn_len > 0 {
                        WState::ValBegin
                    } else {
                        WState::CtnEnd
                    };
                }

                WState::DocEnd => {
                    if PRETTY {
                        if newline {
                            incr_len!(2);
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        *cur = 0;
                    } else {
                        if newline {
                            incr_len!(2);
                            *cur.sub(1) = b'\n';
                            cur = cur.add(1);
                        }
                        cur = cur.sub(1);
                        *cur = 0;
                    }
                    *dat_len = cur as usize - hdr as usize;
                    mem::set(err as *mut u8, 0, size_of::<yyjson_write_err>());
                    return hdr;
                }
            }
        }
    };

    *dat_len = 0;
    fail.apply(err);
    if !hdr.is_null() {
        alc.free(hdr as *mut c_void);
    }
    ptr::null_mut()
}

/// `write_root`
unsafe fn write_root(
    val: *const yyjson_val,
    flg: u32,
    alc_ptr: *const yyjson_alc,
    buf: *mut c_char,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut c_char {
    let mut tmp_err = yyjson_write_err {
        code: 0,
        msg: ptr::null(),
    };
    let mut tmp_dat_len: usize = 0;
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };
    let root = val as *mut yyjson_val;

    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_write_err
    } else {
        err
    };
    let dat_len = if dat_len.is_null() {
        &mut tmp_dat_len as *mut usize
    } else {
        dat_len
    };

    if root.is_null() {
        *dat_len = 0;
        (*err).msg = b"input JSON is NULL\0".as_ptr() as *const c_char;
        /* the C source uses the read-side constant here; both are 1 */
        (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
        return ptr::null_mut();
    }

    if !unsafe_is_ctn(root) || unsafe_get_len(root) == 0 {
        write_root_single(root, flg, alc, buf, dat_len, err) as *mut c_char
    } else if (flg & (YYJSON_WRITE_PRETTY | YYJSON_WRITE_PRETTY_TWO_SPACES)) != 0 {
        write_root_ctn::<true>(root, flg, alc, buf, dat_len, err) as *mut c_char
    } else {
        write_root_ctn::<false>(root, flg, alc, buf, dat_len, err) as *mut c_char
    }
}

/* ------------------------------------------------------- public writer */

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_write_opts(
    val: *const yyjson_val,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut c_char {
    write_root(val, flg, alc_ptr, ptr::null_mut(), dat_len, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_opts(
    doc: *const yyjson_doc,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut c_char {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    write_root(root, flg, alc_ptr, ptr::null_mut(), dat_len, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_write_file(
    path: *const c_char,
    val: *const yyjson_val,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_write_err,
) -> bool {
    let mut tmp_err = yyjson_write_err {
        code: 0,
        msg: ptr::null(),
    };
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };
    let mut dat_len: usize = 0;
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_write_err
    } else {
        err
    };

    if path.is_null() || *path == 0 {
        (*err).msg = b"input path is invalid\0".as_ptr() as *const c_char;
        (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
        return false;
    }

    let dat = write_root(val, flg, &alc, ptr::null_mut(), &mut dat_len, err) as *mut u8;
    if dat.is_null() {
        return false;
    }
    let suc = write_dat_to_file(path, dat, dat_len, err);
    alc.free(dat as *mut c_void);
    suc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_write_fp(
    fp: *mut FILE,
    val: *const yyjson_val,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_write_err,
) -> bool {
    let mut tmp_err = yyjson_write_err {
        code: 0,
        msg: ptr::null(),
    };
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };
    let mut dat_len: usize = 0;
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_write_err
    } else {
        err
    };

    if fp.is_null() {
        (*err).msg = b"input fp is invalid\0".as_ptr() as *const c_char;
        (*err).code = YYJSON_READ_ERROR_INVALID_PARAMETER;
        return false;
    }

    let dat = write_root(val, flg, &alc, ptr::null_mut(), &mut dat_len, err) as *mut u8;
    if dat.is_null() {
        return false;
    }
    let suc = write_dat_to_fp(fp, dat, dat_len, err);
    alc.free(dat as *mut c_void);
    suc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_file(
    path: *const c_char,
    doc: *const yyjson_doc,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_write_err,
) -> bool {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_val_write_file(path, root, flg, alc_ptr, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_fp(
    fp: *mut FILE,
    doc: *const yyjson_doc,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_write_err,
) -> bool {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_val_write_fp(fp, root, flg, alc_ptr, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_val_write_buf(
    buf: *mut c_char,
    buf_len: usize,
    val: *const yyjson_val,
    flg: yyjson_write_flag,
    err: *mut yyjson_write_err,
) -> usize {
    if buf.is_null() || buf_len == 0 {
        if !err.is_null() {
            (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
            (*err).msg = b"input buf or buf_len is invalid\0".as_ptr() as *const c_char;
        }
        0
    } else {
        let mut len = buf_len;
        write_root(val, flg, &YYJSON_NULL_ALC, buf, &mut len, err);
        len
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_write_buf(
    buf: *mut c_char,
    buf_len: usize,
    doc: *const yyjson_doc,
    flg: yyjson_write_flag,
    err: *mut yyjson_write_err,
) -> usize {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_val_write_buf(buf, buf_len, root, flg, err)
}

/* ------------------------------------------- mutable writer implementation */

/// `yyjson_mut_doc_estimated_val_num`
unsafe fn mut_doc_estimated_val_num(doc: *const yyjson_mut_doc) -> usize {
    let mut sum: usize = 0;
    let mut chunk = (*doc).val_pool.chunks;
    while !chunk.is_null() {
        sum += (*chunk).chunk_size / size_of::<yyjson_mut_val>() - 1;
        if chunk == (*doc).val_pool.chunks {
            sum -= ((*doc).val_pool.end as usize - (*doc).val_pool.cur as usize)
                / size_of::<yyjson_mut_val>();
        }
        chunk = (*chunk).next;
    }
    sum
}

/// `mut_write_root_minify` (`PRETTY == false`) and `mut_write_root_pretty`
/// (`PRETTY == true`). The root must be a non-empty container.
unsafe fn mut_write_root_ctn<const PRETTY: bool>(
    root: *const yyjson_mut_val,
    estimated_val_num: usize,
    flg: u32,
    alc: yyjson_alc,
    buf: *mut c_char,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut u8 {
    let enc_table = get_enc_table_with_flag(flg);
    let hex_table = get_hex_table_with_flag(flg);
    let cpy = ptr::eq(enc_table, &ENC_TABLE_CPY);
    let esc = has_flg(flg, YYJSON_WRITE_ESCAPE_UNICODE);
    let inv = has_flg(flg, YYJSON_WRITE_ALLOW_INVALID_UNICODE);
    let spaces = if has_flg(flg, YYJSON_WRITE_PRETTY_TWO_SPACES) {
        2
    } else {
        4
    };
    let newline = has_flg(flg, YYJSON_WRITE_NEWLINE_AT_END);

    let mut hdr: *mut u8 = ptr::null_mut();
    let mut alc_len: usize;

    let fail: WFail = 'fsm: {
        if !buf.is_null() {
            hdr = buf as *mut u8;
            alc_len = *dat_len;
            alc_len = size_align_down(alc_len, size_of::<MutWriteCtx>());
            if alc_len <= size_of::<MutWriteCtx>() {
                break 'fsm WFail::Alloc;
            }
        } else {
            alc_len = estimated_val_num
                * if PRETTY {
                    YYJSON_WRITER_ESTIMATED_PRETTY_RATIO
                } else {
                    YYJSON_WRITER_ESTIMATED_MINIFY_RATIO
                }
                + 64;
            alc_len = size_align_up(alc_len, size_of::<MutWriteCtx>());
            hdr = alc.malloc(alc_len) as *mut u8;
            if hdr.is_null() {
                break 'fsm WFail::Alloc;
            }
        }
        let mut cur = hdr;
        let mut end = hdr.add(alc_len);
        let mut ctx = end as *mut MutWriteCtx;

        macro_rules! incr_len {
            ($len:expr) => {{
                let ext_len: usize = $len;
                if cur.add(ext_len) >= ctx as *mut u8 {
                    let ctx_pos = ctx as usize - hdr as usize;
                    let cur_pos = cur as usize - hdr as usize;
                    let ctx_len = end as usize - ctx as usize;
                    let mut alc_inc = if alc_len / 2 > ext_len {
                        alc_len / 2
                    } else {
                        ext_len
                    };
                    alc_inc = size_align_up(alc_inc, size_of::<MutWriteCtx>());
                    if size_of::<usize>() < 8 && size_add_is_overflow(alc_len, alc_inc) {
                        break 'fsm WFail::Alloc;
                    }
                    alc_len += alc_inc;
                    let tmp = alc.realloc(hdr as *mut c_void, alc_len - alc_inc, alc_len) as *mut u8;
                    if tmp.is_null() {
                        break 'fsm WFail::Alloc;
                    }
                    let ctx_tmp = tmp.add(alc_len - ctx_len) as *mut MutWriteCtx;
                    mem::move_(ctx_tmp as *mut u8, tmp.add(ctx_pos), ctx_len);
                    ctx = ctx_tmp;
                    cur = tmp.add(cur_pos);
                    end = tmp.add(alc_len);
                    hdr = tmp;
                }
            }};
        }
        macro_rules! check_str_len {
            ($len:expr) => {{
                if size_of::<usize>() < 8 && $len >= (USIZE_MAX - 16) / 6 {
                    break 'fsm WFail::Alloc;
                }
            }};
        }

        /* doc_begin */
        let mut val = root as *mut yyjson_mut_val;
        let mut val_type = mut_get_type(val);
        let mut ctn_obj = val_type == YYJSON_TYPE_OBJ;
        let mut ctn_len = mut_get_len(val) << ctn_obj as u32;
        *cur = b'[' | ((ctn_obj as u8) << 5);
        cur = cur.add(1);
        if PRETTY {
            *cur = b'\n';
            cur = cur.add(1);
        }
        let mut ctn = val;
        val = (*val).uni.ptr as *mut yyjson_mut_val; /* tail */
        val = if ctn_obj {
            (*(*val).next).next
        } else {
            (*val).next
        };
        let mut level: usize = 1;

        let mut state = WState::ValBegin;
        loop {
            match state {
                WState::ValBegin => {
                    val_type = mut_get_type(val);
                    let no_indent = (ctn_obj as u8 & ctn_len as u8) != 0;
                    if val_type == YYJSON_TYPE_STR {
                        let is_key = (ctn_obj as u8 & !(ctn_len as u8)) != 0;
                        let str_len = mut_get_len(val);
                        let str_ptr = (*val).uni.str_ as *const u8;
                        check_str_len!(str_len);
                        if PRETTY {
                            incr_len!(str_len * 6 + 16 + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(str_len * 6 + 16);
                        }
                        if cpy && mut_get_subtype(val) != 0 {
                            cur = write_str_noesc(cur, str_ptr, str_len);
                        } else {
                            cur = write_str(cur, esc, inv, str_ptr, str_len, enc_table, hex_table);
                            if cur.is_null() {
                                break 'fsm WFail::Str;
                            }
                        }
                        *cur = if is_key { b':' } else { b',' };
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = if is_key { b' ' } else { b'\n' };
                            cur = cur.add(1);
                        }
                        state = WState::ValEnd;
                        continue;
                    }
                    if val_type == YYJSON_TYPE_NUM {
                        if PRETTY {
                            incr_len!(FP_BUF_LEN + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(FP_BUF_LEN);
                        }
                        cur = write_num(cur, val as *const yyjson_val, flg);
                        if cur.is_null() {
                            break 'fsm WFail::Num;
                        }
                        *cur = b',';
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        state = WState::ValEnd;
                        continue;
                    }
                    if (val_type & (YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ))
                        == (YYJSON_TYPE_ARR & YYJSON_TYPE_OBJ)
                    {
                        let ctn_len_tmp = mut_get_len(val);
                        let ctn_obj_tmp = val_type == YYJSON_TYPE_OBJ;
                        if PRETTY {
                            incr_len!(
                                2 * size_of::<MutWriteCtx>()
                                    + if no_indent { 0 } else { level * 4 }
                            );
                        } else {
                            incr_len!(2 * size_of::<MutWriteCtx>());
                        }
                        if ctn_len_tmp == 0 {
                            if PRETTY {
                                cur =
                                    write_indent(cur, if no_indent { 0 } else { level }, spaces);
                            }
                            *cur = b'[' | ((ctn_obj_tmp as u8) << 5);
                            *cur.add(1) = b']' | ((ctn_obj_tmp as u8) << 5);
                            *cur.add(2) = b',';
                            cur = cur.add(3);
                            if PRETTY {
                                *cur = b'\n';
                                cur = cur.add(1);
                            }
                            state = WState::ValEnd;
                            continue;
                        } else {
                            ctx = ctx.sub(1);
                            (*ctx).tag = (ctn_len << 1) | ctn_obj as usize;
                            (*ctx).ctn = ctn;
                            ctn_len = ctn_len_tmp << ctn_obj_tmp as u32;
                            ctn_obj = ctn_obj_tmp;
                            if PRETTY {
                                cur =
                                    write_indent(cur, if no_indent { 0 } else { level }, spaces);
                                level += 1;
                            }
                            *cur = b'[' | ((ctn_obj as u8) << 5);
                            cur = cur.add(1);
                            if PRETTY {
                                *cur = b'\n';
                                cur = cur.add(1);
                            }
                            ctn = val;
                            val = (*ctn).uni.ptr as *mut yyjson_mut_val; /* tail */
                            val = if ctn_obj {
                                (*(*val).next).next
                            } else {
                                (*val).next
                            };
                            continue;
                        }
                    }
                    if val_type == YYJSON_TYPE_BOOL {
                        if PRETTY {
                            incr_len!(16 + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(16);
                        }
                        let b = (mut_get_tag(val) & YYJSON_SUBTYPE_MASK) >> YYJSON_TYPE_BIT != 0;
                        cur = write_bool(cur, b);
                        cur = cur.add(if PRETTY { 2 } else { 1 });
                        state = WState::ValEnd;
                        continue;
                    }
                    if val_type == YYJSON_TYPE_NULL {
                        if PRETTY {
                            incr_len!(16 + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(16);
                        }
                        cur = write_null(cur);
                        cur = cur.add(if PRETTY { 2 } else { 1 });
                        state = WState::ValEnd;
                        continue;
                    }
                    if val_type == YYJSON_TYPE_RAW {
                        let str_len = mut_get_len(val);
                        let str_ptr = (*val).uni.str_ as *const u8;
                        check_str_len!(str_len);
                        if PRETTY {
                            incr_len!(str_len + 3 + if no_indent { 0 } else { level * 4 });
                            cur = write_indent(cur, if no_indent { 0 } else { level }, spaces);
                        } else {
                            incr_len!(str_len + 2);
                        }
                        cur = write_raw(cur, str_ptr, str_len);
                        *cur = b',';
                        cur = cur.add(1);
                        if PRETTY {
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        state = WState::ValEnd;
                        continue;
                    }
                    break 'fsm WFail::Type;
                }

                WState::ValEnd => {
                    ctn_len -= 1;
                    if ctn_len == 0 {
                        state = WState::CtnEnd;
                    } else {
                        val = (*val).next;
                        state = WState::ValBegin;
                    }
                }

                WState::CtnEnd => {
                    if PRETTY {
                        cur = cur.sub(2);
                        *cur = b'\n';
                        cur = cur.add(1);
                        incr_len!(level * 4);
                        level -= 1;
                        cur = write_indent(cur, level, spaces);
                        *cur = b']' | ((ctn_obj as u8) << 5);
                        cur = cur.add(1);
                        if ctx as *mut u8 >= end {
                            state = WState::DocEnd;
                            continue;
                        }
                        val = (*ctn).next;
                        let tag = (*ctx).tag;
                        ctn = (*ctx).ctn;
                        ctx = ctx.add(1);
                        ctn_len = tag >> 1;
                        ctn_obj = (tag & 1) != 0;
                        ctn_len -= 1;
                        *cur = b',';
                        *cur.add(1) = b'\n';
                        cur = cur.add(2);
                    } else {
                        cur = cur.sub(1);
                        *cur = b']' | ((ctn_obj as u8) << 5);
                        *cur.add(1) = b',';
                        cur = cur.add(2);
                        if ctx as *mut u8 >= end {
                            state = WState::DocEnd;
                            continue;
                        }
                        val = (*ctn).next;
                        let tag = (*ctx).tag;
                        ctn = (*ctx).ctn;
                        ctx = ctx.add(1);
                        ctn_len = tag >> 1;
                        ctn_obj = (tag & 1) != 0;
                        ctn_len -= 1;
                    }
                    state = if ctn_len > 0 {
                        WState::ValBegin
                    } else {
                        WState::CtnEnd
                    };
                }

                WState::DocEnd => {
                    if PRETTY {
                        if newline {
                            incr_len!(2);
                            *cur = b'\n';
                            cur = cur.add(1);
                        }
                        *cur = 0;
                    } else {
                        if newline {
                            incr_len!(2);
                            *cur.sub(1) = b'\n';
                            cur = cur.add(1);
                        }
                        cur = cur.sub(1);
                        *cur = 0;
                    }
                    *dat_len = cur as usize - hdr as usize;
                    (*err).code = YYJSON_WRITE_SUCCESS;
                    (*err).msg = ptr::null();
                    return hdr;
                }
            }
        }
    };

    *dat_len = 0;
    fail.apply(err);
    if !hdr.is_null() {
        alc.free(hdr as *mut c_void);
    }
    ptr::null_mut()
}

/// `mut_write_root`
unsafe fn mut_write_root(
    val: *const yyjson_mut_val,
    estimated_val_num: usize,
    flg: u32,
    alc_ptr: *const yyjson_alc,
    buf: *mut c_char,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut c_char {
    let mut tmp_err = yyjson_write_err {
        code: 0,
        msg: ptr::null(),
    };
    let mut tmp_dat_len: usize = 0;
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };
    let root = val as *mut yyjson_mut_val;

    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_write_err
    } else {
        err
    };
    let dat_len = if dat_len.is_null() {
        &mut tmp_dat_len as *mut usize
    } else {
        dat_len
    };

    if root.is_null() {
        *dat_len = 0;
        (*err).msg = b"input JSON is NULL\0".as_ptr() as *const c_char;
        (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
        return ptr::null_mut();
    }

    if !unsafe_is_ctn(root as *const yyjson_val) || mut_get_len(root) == 0 {
        write_root_single(root as *mut yyjson_val, flg, alc, buf, dat_len, err) as *mut c_char
    } else if (flg & (YYJSON_WRITE_PRETTY | YYJSON_WRITE_PRETTY_TWO_SPACES)) != 0 {
        mut_write_root_ctn::<true>(root, estimated_val_num, flg, alc, buf, dat_len, err)
            as *mut c_char
    } else {
        mut_write_root_ctn::<false>(root, estimated_val_num, flg, alc, buf, dat_len, err)
            as *mut c_char
    }
}

/* --------------------------------------------------- public mutable writer */

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_write_opts(
    val: *const yyjson_mut_val,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut c_char {
    mut_write_root(val, 0, flg, alc_ptr, ptr::null_mut(), dat_len, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_write_opts(
    doc: *const yyjson_mut_doc,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    dat_len: *mut usize,
    err: *mut yyjson_write_err,
) -> *mut c_char {
    let (root, estimated_val_num) = if !doc.is_null() {
        ((*doc).root, mut_doc_estimated_val_num(doc))
    } else {
        (ptr::null_mut(), 0)
    };
    mut_write_root(
        root,
        estimated_val_num,
        flg,
        alc_ptr,
        ptr::null_mut(),
        dat_len,
        err,
    )
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_write_buf(
    buf: *mut c_char,
    buf_len: usize,
    val: *const yyjson_mut_val,
    flg: yyjson_write_flag,
    err: *mut yyjson_write_err,
) -> usize {
    if buf.is_null() || buf_len == 0 {
        if !err.is_null() {
            (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
            (*err).msg = b"input buf or buf_len is invalid\0".as_ptr() as *const c_char;
        }
        0
    } else {
        let mut len = buf_len;
        mut_write_root(val, 0, flg, &YYJSON_NULL_ALC, buf, &mut len, err);
        len
    }
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_write_buf(
    buf: *mut c_char,
    buf_len: usize,
    doc: *const yyjson_mut_doc,
    flg: yyjson_write_flag,
    err: *mut yyjson_write_err,
) -> usize {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_mut_val_write_buf(buf, buf_len, root, flg, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_write_file(
    path: *const c_char,
    val: *const yyjson_mut_val,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_write_err,
) -> bool {
    let mut tmp_err = yyjson_write_err {
        code: 0,
        msg: ptr::null(),
    };
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };
    let mut dat_len: usize = 0;
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_write_err
    } else {
        err
    };

    if path.is_null() || *path == 0 {
        (*err).msg = b"input path is invalid\0".as_ptr() as *const c_char;
        (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
        return false;
    }

    let dat = yyjson_mut_val_write_opts(val, flg, &alc, &mut dat_len, err) as *mut u8;
    if dat.is_null() {
        return false;
    }
    let suc = write_dat_to_file(path, dat, dat_len, err);
    alc.free(dat as *mut c_void);
    suc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_val_write_fp(
    fp: *mut FILE,
    val: *const yyjson_mut_val,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_write_err,
) -> bool {
    let mut tmp_err = yyjson_write_err {
        code: 0,
        msg: ptr::null(),
    };
    let alc = if alc_ptr.is_null() {
        YYJSON_DEFAULT_ALC
    } else {
        *alc_ptr
    };
    let mut dat_len: usize = 0;
    let err = if err.is_null() {
        &mut tmp_err as *mut yyjson_write_err
    } else {
        err
    };

    if fp.is_null() {
        (*err).msg = b"input fp is invalid\0".as_ptr() as *const c_char;
        (*err).code = YYJSON_WRITE_ERROR_INVALID_PARAMETER;
        return false;
    }

    let dat = yyjson_mut_val_write_opts(val, flg, &alc, &mut dat_len, err) as *mut u8;
    if dat.is_null() {
        return false;
    }
    let suc = write_dat_to_fp(fp, dat, dat_len, err);
    alc.free(dat as *mut c_void);
    suc
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_write_file(
    path: *const c_char,
    doc: *const yyjson_mut_doc,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_write_err,
) -> bool {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_mut_val_write_file(path, root, flg, alc_ptr, err)
}

#[no_mangle]
pub unsafe extern "C" fn yyjson_mut_write_fp(
    fp: *mut FILE,
    doc: *const yyjson_mut_doc,
    flg: yyjson_write_flag,
    alc_ptr: *const yyjson_alc,
    err: *mut yyjson_write_err,
) -> bool {
    let root = if doc.is_null() {
        ptr::null_mut()
    } else {
        (*doc).root
    };
    yyjson_mut_val_write_fp(fp, root, flg, alc_ptr, err)
}
