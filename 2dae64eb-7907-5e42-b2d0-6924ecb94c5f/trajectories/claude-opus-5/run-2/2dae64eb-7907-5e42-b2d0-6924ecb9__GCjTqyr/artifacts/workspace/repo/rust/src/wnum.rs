//! Integer and floating-point writers.
//!
//! Transliteration of the "Integer Writer" and "Number Writer" sections of
//! the original `src/yyjson.c` (the `!YYJSON_DISABLE_FAST_FP_CONV` branch).
//!
//! Several helpers temporarily move the cursor one byte *before* the buffer
//! (the `buf -= lz` leading-zero trick) and only ever add to it again, exactly
//! as the C does. `wrapping_add`/`wrapping_sub` are used for those cursors so
//! that the intermediate out-of-range pointer cannot be miscompiled.

use core::ffi::c_char;
use core::ptr;

use crate::tables::*;
use crate::types::*;
use crate::util::*;

#[inline(always)]
unsafe fn dt(idx: usize) -> *const u8 {
    DIGIT_TABLE.as_ptr().add(idx)
}

#[inline]
unsafe fn write_u32_len_8(val: u32, buf: *mut u8) -> *mut u8 {
    let aabb = (((val as u64) * 109951163) >> 40) as u32; /* val / 10000 */
    let ccdd = val - aabb * 10000; /* val % 10000 */
    let aa = (aabb * 5243) >> 19; /* aabb / 100 */
    let cc = (ccdd * 5243) >> 19; /* ccdd / 100 */
    let bb = aabb - aa * 100;
    let dd = ccdd - cc * 100;
    byte_copy_2(buf, dt(aa as usize * 2));
    byte_copy_2(buf.add(2), dt(bb as usize * 2));
    byte_copy_2(buf.add(4), dt(cc as usize * 2));
    byte_copy_2(buf.add(6), dt(dd as usize * 2));
    buf.add(8)
}

#[inline]
unsafe fn write_u32_len_4(val: u32, buf: *mut u8) -> *mut u8 {
    let aa = (val * 5243) >> 19;
    let bb = val - aa * 100;
    byte_copy_2(buf, dt(aa as usize * 2));
    byte_copy_2(buf.add(2), dt(bb as usize * 2));
    buf.add(4)
}

#[inline]
unsafe fn write_u32_len_1_to_8(val: u32, buf: *mut u8) -> *mut u8 {
    let mut buf = buf;
    if val < 100 {
        /* 1-2 digits: aa */
        let lz = (val < 10) as usize;
        byte_copy_2(buf, dt(val as usize * 2 + lz));
        buf = buf.wrapping_sub(lz);
        buf.wrapping_add(2)
    } else if val < 10000 {
        /* 3-4 digits: aabb */
        let aa = (val * 5243) >> 19;
        let bb = val - aa * 100;
        let lz = (aa < 10) as usize;
        byte_copy_2(buf, dt(aa as usize * 2 + lz));
        buf = buf.wrapping_sub(lz);
        byte_copy_2(buf.wrapping_add(2), dt(bb as usize * 2));
        buf.wrapping_add(4)
    } else if val < 1000000 {
        /* 5-6 digits: aabbcc */
        let aa = (((val as u64) * 429497) >> 32) as u32;
        let bbcc = val - aa * 10000;
        let bb = (bbcc * 5243) >> 19;
        let cc = bbcc - bb * 100;
        let lz = (aa < 10) as usize;
        byte_copy_2(buf, dt(aa as usize * 2 + lz));
        buf = buf.wrapping_sub(lz);
        byte_copy_2(buf.wrapping_add(2), dt(bb as usize * 2));
        byte_copy_2(buf.wrapping_add(4), dt(cc as usize * 2));
        buf.wrapping_add(6)
    } else {
        /* 7-8 digits: aabbccdd */
        let aabb = (((val as u64) * 109951163) >> 40) as u32;
        let ccdd = val - aabb * 10000;
        let aa = (aabb * 5243) >> 19;
        let cc = (ccdd * 5243) >> 19;
        let bb = aabb - aa * 100;
        let dd = ccdd - cc * 100;
        let lz = (aa < 10) as usize;
        byte_copy_2(buf, dt(aa as usize * 2 + lz));
        buf = buf.wrapping_sub(lz);
        byte_copy_2(buf.wrapping_add(2), dt(bb as usize * 2));
        byte_copy_2(buf.wrapping_add(4), dt(cc as usize * 2));
        byte_copy_2(buf.wrapping_add(6), dt(dd as usize * 2));
        buf.wrapping_add(8)
    }
}

#[inline]
unsafe fn write_u32_len_5_to_8(val: u32, buf: *mut u8) -> *mut u8 {
    let mut buf = buf;
    if val < 1000000 {
        let aa = (((val as u64) * 429497) >> 32) as u32;
        let bbcc = val - aa * 10000;
        let bb = (bbcc * 5243) >> 19;
        let cc = bbcc - bb * 100;
        let lz = (aa < 10) as usize;
        byte_copy_2(buf, dt(aa as usize * 2 + lz));
        buf = buf.wrapping_sub(lz);
        byte_copy_2(buf.wrapping_add(2), dt(bb as usize * 2));
        byte_copy_2(buf.wrapping_add(4), dt(cc as usize * 2));
        buf.wrapping_add(6)
    } else {
        let aabb = (((val as u64) * 109951163) >> 40) as u32;
        let ccdd = val - aabb * 10000;
        let aa = (aabb * 5243) >> 19;
        let cc = (ccdd * 5243) >> 19;
        let bb = aabb - aa * 100;
        let dd = ccdd - cc * 100;
        let lz = (aa < 10) as usize;
        byte_copy_2(buf, dt(aa as usize * 2 + lz));
        buf = buf.wrapping_sub(lz);
        byte_copy_2(buf.wrapping_add(2), dt(bb as usize * 2));
        byte_copy_2(buf.wrapping_add(4), dt(cc as usize * 2));
        byte_copy_2(buf.wrapping_add(6), dt(dd as usize * 2));
        buf.wrapping_add(8)
    }
}

/// `write_u64`
pub unsafe fn write_u64(val: u64, buf: *mut u8) -> *mut u8 {
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

/* ===== floating point =================================================== */

#[inline]
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

#[inline]
unsafe fn write_u64_len_1_to_17(val: u64, buf: *mut u8) -> *mut u8 {
    if val >= 100000000u64 * 10000000 {
        /* len: 16 to 17 */
        let hgh = val / 100000000;
        let low = (val - hgh * 100000000) as u32;
        let one = (hgh / 100000000) as u32;
        let mid = (hgh - (one as u64) * 100000000) as u32;
        *buf = one as u8 + b'0';
        let buf = buf.add((one > 0) as usize);
        let buf = write_u32_len_8(mid, buf);
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
#[inline]
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
    byte_copy_2(buf, dt(bb as usize * 2 + lz));
    buf = buf.wrapping_sub(lz);
    byte_copy_2(buf.wrapping_add(2), dt(cc as usize * 2));

    if ddee != 0 {
        let dd = (ddee * 5243) >> 19;
        let ee = ddee - dd * 100;
        byte_copy_2(buf.wrapping_add(4), dt(dd as usize * 2));
        byte_copy_2(buf.wrapping_add(6), dt(ee as usize * 2));
        let tz1 = DEC_TRAILING_ZERO_TABLE[dd as usize] as usize;
        let tz2 = DEC_TRAILING_ZERO_TABLE[ee as usize] as usize;
        let tz = if ee != 0 { tz2 } else { tz1 + 2 };
        buf.wrapping_add(8 - tz)
    } else {
        let tz1 = DEC_TRAILING_ZERO_TABLE[bb as usize] as usize;
        let tz2 = DEC_TRAILING_ZERO_TABLE[cc as usize] as usize;
        let tz = if cc != 0 { tz2 } else { tz1 + tz2 };
        buf.wrapping_add(4 - tz)
    }
}

/// `write_u64_len_16_to_17_trim`
#[inline]
unsafe fn write_u64_len_16_to_17_trim(val: u64, buf: *mut u8) -> *mut u8 {
    let mut buf = buf;
    let abbccddee = (val / 100000000) as u32;
    let ffgghhii = (val - (abbccddee as u64) * 100000000) as u32;
    let abbcc = abbccddee / 10000;
    let ddee = abbccddee - abbcc * 10000;
    let abb = (((abbcc as u64) * 167773) >> 24) as u32;
    let a = (abb * 41) >> 12;
    let bb = abb - a * 100;
    let cc = abbcc - abb * 100;
    *buf = a as u8 + b'0';
    buf = buf.add((a > 0) as usize);
    byte_copy_2(buf, dt(bb as usize * 2));
    byte_copy_2(buf.add(2), dt(cc as usize * 2));

    if ffgghhii != 0 {
        let dd = (ddee * 5243) >> 19;
        let ee = ddee - dd * 100;
        let ffgg = (((ffgghhii as u64) * 109951163) >> 40) as u32;
        let hhii = ffgghhii - ffgg * 10000;
        let ff = (ffgg * 5243) >> 19;
        let gg = ffgg - ff * 100;
        byte_copy_2(buf.add(4), dt(dd as usize * 2));
        byte_copy_2(buf.add(6), dt(ee as usize * 2));
        byte_copy_2(buf.add(8), dt(ff as usize * 2));
        byte_copy_2(buf.add(10), dt(gg as usize * 2));
        if hhii != 0 {
            let hh = (hhii * 5243) >> 19;
            let ii = hhii - hh * 100;
            byte_copy_2(buf.add(12), dt(hh as usize * 2));
            byte_copy_2(buf.add(14), dt(ii as usize * 2));
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
        byte_copy_2(buf.add(4), dt(dd as usize * 2));
        byte_copy_2(buf.add(6), dt(ee as usize * 2));
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
#[inline]
unsafe fn write_f32_exp(exp: i32, buf: *mut u8) -> *mut u8 {
    byte_copy_2(buf, b"e-".as_ptr());
    let buf = buf.add(2 - (exp >= 0) as usize);
    let exp = if exp < 0 { -exp } else { exp };
    let lz = (exp < 10) as usize;
    byte_copy_2(buf, dt(exp as usize * 2 + lz));
    buf.add(2 - lz)
}

/// `write_f64_exp`: exponent part in range `e-324` to `e308`.
#[inline]
unsafe fn write_f64_exp(exp: i32, buf: *mut u8) -> *mut u8 {
    byte_copy_2(buf, b"e-".as_ptr());
    let buf = buf.add(2 - (exp >= 0) as usize);
    let exp = if exp < 0 { -exp } else { exp };
    if exp < 100 {
        let lz = (exp < 10) as usize;
        byte_copy_2(buf, dt(exp as usize * 2 + lz));
        buf.add(2 - lz)
    } else {
        let hi = ((exp as u32) * 656) >> 16; /* exp / 100 */
        let lo = exp as u32 - hi * 100; /* exp % 100 */
        *buf = hi as u8 + b'0';
        byte_copy_2(buf.add(1), dt(lo as usize * 2));
        buf.add(3)
    }
}

/// `div_pow10`
#[inline]
fn div_pow10(num: u64, exp: u32) -> (u64, u64, u64) {
    let m = DIV_POW10_TABLE[exp as usize];
    let (hi, _lo) = u128_mul(num >> m.shr1, m.mul);
    let div = hi >> m.shr2;
    let modv = num - div * m.p10;
    (div, modv, m.p10)
}

/// `u64_round_to_odd`
#[inline]
fn u64_round_to_odd(u: u64, cp: u32) -> u32 {
    let (hi, lo) = u128_mul(cp as u64, u);
    let y_hi = hi as u32;
    let y_lo = (lo >> 32) as u32;
    y_hi | (y_lo > 1) as u32
}

/// `u128_round_to_odd`
#[inline]
fn u128_round_to_odd(hi: u64, lo: u64, cp: u64) -> u64 {
    let (x_hi, _x_lo) = u128_mul(cp, lo);
    let (y_hi, y_lo) = u128_mul_add(cp, hi, x_hi);
    y_hi | (y_lo > 1) as u64
}

/// `f32_bin_to_dec`
fn f32_bin_to_dec(sig_raw: u32, exp_raw: u32, sig_bin: u32, exp_bin: i32) -> (u32, i32) {
    /* Fast path, see `f64_bin_to_dec`. */
    if sig_raw != 0 {
        'fast: {
            let k = (exp_bin.wrapping_mul(315653)) >> 20;
            let h = exp_bin + ((-k * 217707) >> 16);
            let (p10_hi, _p10_lo) = pow10_table_get_sig(-k);

            let cb = sig_bin << (h + 1);
            let (hi, lo) = u128_mul(cb as u64, p10_hi);
            let s_hi = hi as u32;
            let s_lo = (lo >> 32) as u32;
            let m = s_hi % 10;
            let dec = s_hi - m;

            let c = (m << (32 - 4)) | (s_lo >> 4);
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
            let add_1 = m + w1_inside as u32;
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
    let h = exp_bin + ((-k * 217707) >> 16) + 1;
    let (mut p10_hi, _p10_lo) = pow10_table_get_sig(-k);
    p10_hi += 1;

    let vbl = u64_round_to_odd(p10_hi, cbl << h);
    let vb = u64_round_to_odd(p10_hi, cb << h);
    let vbr = u64_round_to_odd(p10_hi, cbr << h);
    let lower = vbl + !is_even as u32;
    let upper = vbr - !is_even as u32;

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
    let round_up = (vb > mid) || (vb == mid && (s & 1) != 0);
    (
        s + if u1_inside != w1_inside {
            w1_inside as u32
        } else {
            round_up as u32
        },
        k,
    )
}

/// `f64_bin_to_dec`
fn f64_bin_to_dec(sig_raw: u64, exp_raw: u32, sig_bin: u64, exp_bin: i32) -> (u64, i32) {
    /*
     Fast path: for a regularly spaced significand there are four candidates;
     see the long comment in the original `src/yyjson.c`.
     */
    if sig_raw != 0 {
        'fast: {
            let k = (exp_bin.wrapping_mul(315653)) >> 20;
            let h = exp_bin + ((-k * 217707) >> 16);
            let (p10_hi, p10_lo) = pow10_table_get_sig(-k);

            let cb = sig_bin << (h + 1);
            let (s_hi, _) = u128_mul(cb, p10_lo);
            let (s_hi, s_lo) = u128_mul_add(cb, p10_hi, s_hi);
            let m = s_hi % 10;
            let dec = s_hi - m;

            let c = (m << (64 - 4)) | (s_lo >> 4);
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
            let add_1 = m + w1_inside as u64;
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
    let h = exp_bin + ((-k * 217707) >> 16) + 1;
    let (p10_hi, mut p10_lo) = pow10_table_get_sig(-k);
    p10_lo += 1;

    let vbl = u128_round_to_odd(p10_hi, p10_lo, cbl << h);
    let vb = u128_round_to_odd(p10_hi, p10_lo, cb << h);
    let vbr = u128_round_to_odd(p10_hi, p10_lo, cbr << h);
    let lower = vbl + !is_even as u64;
    let upper = vbr - !is_even as u64;

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
    let round_up = (vb > mid) || (vb == mid && (s & 1) != 0);
    (
        s + if u1_inside != w1_inside {
            w1_inside as u64
        } else {
            round_up as u64
        },
        k,
    )
}

/// `f64_bin_to_dec_fast`
fn f64_bin_to_dec_fast(
    sig_raw: u64,
    exp_raw: u32,
    sig_bin: u64,
    exp_bin: i32,
) -> (u64, i32, bool) {
    let irregular = sig_raw == 0 && exp_raw > 1;

    let k = (exp_bin.wrapping_mul(315653) - if irregular { 131237 } else { 0 }) >> 20;
    let h = exp_bin + ((-k * 217707) >> 16);
    let (p10_hi, p10_lo) = pow10_table_get_sig(-k);

    let cb = sig_bin << (h + 1);
    let (s_hi, _) = u128_mul(cb, p10_lo);
    let (s_hi, s_lo) = u128_mul_add(cb, p10_hi, s_hi);

    let u = s_lo
        >= if irregular {
            0x5555555555555555
        } else {
            1u64 << 63
        };

    (s_hi + u as u64, k, u)
}

/// `write_inf_or_nan`
#[inline]
unsafe fn write_inf_or_nan(buf: *mut u8, flg: u32, sig_raw: u64, sign: bool) -> *mut u8 {
    if (flg & YYJSON_WRITE_INF_AND_NAN_AS_NULL) != 0 {
        byte_copy_4(buf, b"null".as_ptr());
        return buf.add(4);
    }
    if (flg & YYJSON_WRITE_ALLOW_INF_AND_NAN) != 0 {
        if sig_raw == 0 {
            *buf = b'-';
            let buf = buf.add(sign as usize);
            byte_copy_8(buf, b"Infinity".as_ptr());
            return buf.add(8);
        } else {
            byte_copy_4(buf, b"NaN\0".as_ptr());
            return buf.add(3);
        }
    }
    ptr::null_mut()
}

/// `write_f32_raw` (requires a 40-byte buffer)
unsafe fn write_f32_raw(buf: *mut u8, raw_f64: u64, flg: u32) -> *mut u8 {
    let mut buf = buf;

    /* cast double to float */
    let raw = f32_to_bits(f64_to_f32(f64_from_bits(raw_f64)));

    let sign = (raw >> (F32_BITS - 1)) != 0;
    let sig_raw = raw & F32_SIG_MASK;
    let exp_raw = (raw & F32_EXP_MASK) >> F32_SIG_BITS;

    if exp_raw == (1u32 << F32_EXP_BITS) - 1 {
        return write_inf_or_nan(buf, flg, sig_raw as u64, sign);
    }

    *buf = b'-';
    buf = buf.add(sign as usize);

    if (raw << 1) == 0 {
        byte_copy_4(buf, b"0.0\0".as_ptr());
        return buf.add(3);
    }

    if exp_raw != 0 {
        /* normal number */
        let sig_bin = sig_raw | (1u32 << F32_SIG_BITS);
        let exp_bin = exp_raw as i32 - F32_EXP_BIAS - F32_SIG_BITS;

        /* fast path for a small integer number without fraction */
        if (-F32_SIG_BITS..=0).contains(&exp_bin)
            && u64_tz_bits(sig_bin as u64) >= (-exp_bin) as u32
        {
            let sig_dec = sig_bin >> -exp_bin;
            let buf = write_u32_len_1_to_8(sig_dec, buf);
            byte_copy_2(buf, b".0".as_ptr());
            return buf.add(2);
        }

        let (sig_dec, mut exp_dec) = f32_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);

        /* the sig length is 7 or 9 */
        let sig_len =
            7 + (sig_dec >= 10000000) as i32 + (sig_dec >= 100000000) as i32;

        let dot_ofs = sig_len + exp_dec;

        if -6 < dot_ofs && dot_ofs <= 21 {
            crate::libc::mem_set(buf, b'0', 32);

            let no_pre_zero = dot_ofs > 0;
            let pre_ofs = if no_pre_zero { 0 } else { 2 - dot_ofs };
            let num_hdr = buf.offset(pre_ofs as isize);
            let mut num_end = write_u32_len_7_to_9_trim(sig_dec, num_hdr);

            let num_sep_pos = if no_pre_zero { dot_ofs } else { 0 };
            let num_sep = num_hdr.offset(num_sep_pos as isize);
            byte_move_8(num_sep.add(no_pre_zero as usize), num_sep);
            num_end = num_end.add(no_pre_zero as usize);

            let dot_set_pos = if dot_ofs > 1 { dot_ofs } else { 1 };
            *buf.offset(dot_set_pos as isize) = b'.';

            /* `dot_end` can land before `buf` when `dot_ofs` is negative;
               the `max` below always picks `num_end` in that case. */
            let dot_end = buf.wrapping_offset(dot_ofs as isize + 2);
            return if dot_end > num_end { dot_end } else { num_end };
        } else {
            /* write with scientific notation, e.g. 1.234e56 */
            let mut end = write_u32_len_7_to_9_trim(sig_dec, buf.add(1));
            end = end.wrapping_sub((end == buf.add(2)) as usize);
            exp_dec += sig_len - 1;
            *buf = *buf.add(1);
            *buf.add(1) = b'.';
            return write_f32_exp(exp_dec, end);
        }
    } else {
        /* subnormal number */
        let sig_bin = sig_raw;
        let exp_bin = 1 - F32_EXP_BIAS - F32_SIG_BITS;

        let (sig_dec, mut exp_dec) = f32_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);

        let mut end = write_u32_len_1_to_8(sig_dec, buf.add(1));
        *buf = *buf.add(1);
        *buf.add(1) = b'.';
        exp_dec += (end.offset_from(buf) as i32) - 2;

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

/// `write_f64_raw` (requires a 40-byte buffer)
pub unsafe fn write_f64_raw(buf: *mut u8, raw: u64, flg: u32) -> *mut u8 {
    let mut buf = buf;

    let sign = (raw >> (F64_BITS - 1)) != 0;
    let sig_raw = raw & F64_SIG_MASK;
    let exp_raw = ((raw & F64_EXP_MASK) >> F64_SIG_BITS) as u32;

    if exp_raw == (1u32 << F64_EXP_BITS) - 1 {
        return write_inf_or_nan(buf, flg, sig_raw, sign);
    }

    *buf = b'-';
    buf = buf.add(sign as usize);

    if (raw << 1) == 0 {
        byte_copy_4(buf, b"0.0\0".as_ptr());
        return buf.add(3);
    }

    if exp_raw != 0 {
        /* normal number */
        let sig_bin = sig_raw | (1u64 << F64_SIG_BITS);
        let exp_bin = exp_raw as i32 - F64_EXP_BIAS - F64_SIG_BITS;

        /* fast path for a small integer number without fraction */
        if (-F64_SIG_BITS..=0).contains(&exp_bin) && u64_tz_bits(sig_bin) >= (-exp_bin) as u32
        {
            let sig_dec = sig_bin >> -exp_bin;
            let buf = write_u64_len_1_to_16(sig_dec, buf);
            byte_copy_2(buf, b".0".as_ptr());
            return buf.add(2);
        }

        let (sig_dec, mut exp_dec) = f64_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);

        /* the sig length is 16 or 17 */
        let sig_len = 16 + (sig_dec >= 100000000u64 * 100000000) as i32;

        let dot_ofs = sig_len + exp_dec;

        if -6 < dot_ofs && dot_ofs <= 21 {
            crate::libc::mem_set(buf, b'0', 32);

            let no_pre_zero = dot_ofs > 0;
            let pre_ofs = if no_pre_zero { 0 } else { 2 - dot_ofs };
            let num_hdr = buf.offset(pre_ofs as isize);
            let mut num_end = write_u64_len_16_to_17_trim(sig_dec, num_hdr);

            let num_sep_pos = if no_pre_zero { dot_ofs } else { 0 };
            let num_sep = num_hdr.offset(num_sep_pos as isize);
            byte_move_16(num_sep.add(no_pre_zero as usize), num_sep);
            num_end = num_end.add(no_pre_zero as usize);

            let dot_set_pos = if dot_ofs > 1 { dot_ofs } else { 1 };
            *buf.offset(dot_set_pos as isize) = b'.';

            /* `dot_end` can land before `buf` when `dot_ofs` is negative;
               the `max` below always picks `num_end` in that case. */
            let dot_end = buf.wrapping_offset(dot_ofs as isize + 2);
            return if dot_end > num_end { dot_end } else { num_end };
        } else {
            let mut end = write_u64_len_16_to_17_trim(sig_dec, buf.add(1));
            end = end.wrapping_sub((end == buf.add(2)) as usize);
            exp_dec += sig_len - 1;
            *buf = *buf.add(1);
            *buf.add(1) = b'.';
            return write_f64_exp(exp_dec, end);
        }
    } else {
        /* subnormal number */
        let sig_bin = sig_raw;
        let exp_bin = 1 - F64_EXP_BIAS - F64_SIG_BITS;

        let (sig_dec, mut exp_dec) = f64_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);

        let mut end = write_u64_len_1_to_17(sig_dec, buf.add(1));
        *buf = *buf.add(1);
        *buf.add(1) = b'.';
        exp_dec += (end.offset_from(buf) as i32) - 2;

        /* trim trailing zeros */
        end = end.sub((*end.sub(1) == b'0') as usize);
        end = end.sub((*end.sub(1) == b'0') as usize);
        while *end.sub(1) == b'0' {
            end = end.sub(1);
        }
        end = end.sub((*end.sub(1) == b'.') as usize);

        write_f64_exp(exp_dec, end)
    }
}

/// `write_f64_raw_fixed` (requires a 40-byte buffer)
unsafe fn write_f64_raw_fixed(buf: *mut u8, raw: u64, flg: u32, prec: u32) -> *mut u8 {
    let mut buf = buf;

    let sign = (raw >> (F64_BITS - 1)) != 0;
    let sig_raw = raw & F64_SIG_MASK;
    let exp_raw = ((raw & F64_EXP_MASK) >> F64_SIG_BITS) as u32;

    if exp_raw == (1u32 << F64_EXP_BITS) - 1 {
        return write_inf_or_nan(buf, flg, sig_raw, sign);
    }

    *buf = b'-';
    buf = buf.add(sign as usize);

    if (raw << 1) == 0 {
        byte_copy_4(buf, b"0.0\0".as_ptr());
        return buf.add(3);
    }

    if exp_raw != 0 {
        /* normal number */
        let sig_bin = sig_raw | (1u64 << F64_SIG_BITS);
        let exp_bin = exp_raw as i32 - F64_EXP_BIAS - F64_SIG_BITS;

        /* fast path for a small integer number without fraction */
        if (-F64_SIG_BITS..=0).contains(&exp_bin) && u64_tz_bits(sig_bin) >= (-exp_bin) as u32
        {
            let sig_dec = sig_bin >> -exp_bin;
            let buf = write_u64_len_1_to_16(sig_dec, buf);
            byte_copy_2(buf, b".0".as_ptr());
            return buf.add(2);
        }

        /* only `fabs(num) < 1e21` is processed here */
        if (raw << 1) < (0x444B1AE4D6E2EF50u64 << 1) {
            let (mut sig_dec, mut exp_dec, round_up) =
                f64_bin_to_dec_fast(sig_raw, exp_raw, sig_bin, exp_bin);

            let mut sig_len = 16 + (sig_dec >= 100000000u64 * 100000000) as i32;

            /* limit the length of digits after the decimal point */
            if exp_dec < -1 {
                let sig_len_cut = -exp_dec - prec as i32;
                if sig_len_cut > sig_len {
                    byte_copy_4(buf, b"0.0\0".as_ptr());
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
                    byte_copy_4(buf, b"0.0\0".as_ptr());
                    return buf.add(3);
                }
            }

            crate::libc::mem_set(buf, b'0', 32);

            let dot_ofs = sig_len + exp_dec;
            let no_pre_zero = dot_ofs > 0;

            let pre_ofs = if no_pre_zero { 0 } else { 1 - dot_ofs };
            let num_hdr = buf.offset(pre_ofs as isize);
            let mut num_end = write_u64_len_1_to_17(sig_dec, num_hdr);

            let num_sep_pos = if no_pre_zero { dot_ofs } else { -dot_ofs };
            let num_sep = buf.offset(num_sep_pos as isize);
            byte_move_16(num_sep.add(1), num_sep);
            num_end = num_end.add((exp_dec < 0) as usize);

            let dot_set_pos = if dot_ofs > 1 { dot_ofs } else { 1 };
            *buf.offset(dot_set_pos as isize) = b'.';

            /* remove trailing zeros */
            let mut out = buf.offset(dot_set_pos as isize + 2);
            if num_end > out {
                out = num_end;
            }
            out = out.sub((*out.sub(1) == b'0') as usize);
            out = out.sub((*out.sub(1) == b'0') as usize);
            while *out.sub(1) == b'0' {
                out = out.sub(1);
            }
            out = out.add((*out.sub(1) == b'.') as usize);
            out
        } else {
            let (sig_dec, mut exp_dec) = f64_bin_to_dec(sig_raw, exp_raw, sig_bin, exp_bin);
            let sig_len = 16 + (sig_dec >= 100000000u64 * 100000000) as i32;

            let mut end = write_u64_len_16_to_17_trim(sig_dec, buf.add(1));
            end = end.wrapping_sub((end == buf.add(2)) as usize);
            exp_dec += sig_len - 1;
            *buf = *buf.add(1);
            *buf.add(1) = b'.';
            write_f64_exp(exp_dec, end)
        }
    } else {
        /* subnormal number */
        byte_copy_4(buf, b"0.0\0".as_ptr());
        buf.add(3)
    }
}

/// `write_num` (requires a 40-byte buffer)
pub unsafe fn write_num(cur: *mut u8, val: *mut YyjsonVal, flg: u32) -> *mut u8 {
    if ((*val).tag & YYJSON_SUBTYPE_REAL as u64) == 0 {
        let pos = (*val).uni.u64_;
        let neg = (!pos).wrapping_add(1);
        let sign = ((((*val).tag & YYJSON_SUBTYPE_SINT as u64) > 0) as usize)
            & (((pos as i64) < 0) as usize);
        *cur = b'-';
        write_u64(if sign != 0 { neg } else { pos }, cur.add(sign))
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
    val: *const YyjsonVal,
    buf: *mut c_char,
) -> *mut c_char {
    if val.is_null() || buf.is_null() {
        return ptr::null_mut();
    }
    match ((*val).tag & YYJSON_TAG_MASK as u64) as u8 {
        t if t == YYJSON_TYPE_NUM | YYJSON_SUBTYPE_UINT => {
            let buf = write_u64((*val).uni.u64_, buf as *mut u8) as *mut c_char;
            *buf = 0;
            buf
        }
        t if t == YYJSON_TYPE_NUM | YYJSON_SUBTYPE_SINT => {
            let pos = (*val).uni.u64_;
            let neg = (!pos).wrapping_add(1);
            let sign = ((pos as i64) < 0) as usize;
            *buf = b'-' as c_char;
            let buf = write_u64(
                if sign != 0 { neg } else { pos },
                (buf as *mut u8).add(sign),
            ) as *mut c_char;
            *buf = 0;
            buf
        }
        t if t == YYJSON_TYPE_NUM | YYJSON_SUBTYPE_REAL => {
            let raw = (*val).uni.u64_;
            let fmt = ((*val).tag >> 32) as u32;
            let flg = YYJSON_WRITE_ALLOW_INF_AND_NAN;
            let out = if (fmt >> (32 - YYJSON_WRITE_FP_FLAG_BITS)) == 0 {
                write_f64_raw(buf as *mut u8, raw, flg)
            } else if (fmt >> (32 - YYJSON_WRITE_FP_PREC_BITS)) != 0 {
                let prec = fmt >> (32 - YYJSON_WRITE_FP_PREC_BITS);
                write_f64_raw_fixed(buf as *mut u8, raw, flg, prec)
            } else if (fmt & YYJSON_WRITE_FP_TO_FLOAT) != 0 {
                write_f32_raw(buf as *mut u8, raw, flg)
            } else {
                write_f64_raw(buf as *mut u8, raw, flg)
            };
            if !out.is_null() {
                *out = 0;
            }
            out as *mut c_char
        }
        _ => ptr::null_mut(),
    }
}
