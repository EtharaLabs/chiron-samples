//! In-tree checks that do not need a C compiler.
//!
//! These are deliberately narrow: they pin the memory layout the header's
//! `static inline` functions assume, and they spot-check a handful of
//! reader/writer results. The real conformance evidence for this port is the
//! differential comparison against the original C library described in
//! `doc/rust-port.md`; these tests only guard against obvious regressions when
//! no C toolchain is available.

use core::ffi::{c_char, c_void};
use core::mem::{align_of, size_of};
use core::ptr;

use crate::types::*;

fn read(json: &str, flg: u32) -> *mut YyjsonDoc {
    let mut buf: Vec<u8> = json.as_bytes().to_vec();
    buf.push(0);
    unsafe {
        let doc = crate::read::yyjson_read_opts(
            buf.as_mut_ptr() as *mut c_char,
            json.len(),
            flg,
            ptr::null(),
            ptr::null_mut(),
        );
        doc
    }
}

fn write(doc: *mut YyjsonDoc, flg: u32) -> Option<String> {
    unsafe {
        let mut len: usize = 0;
        let out = crate::write::yyjson_write_opts(
            doc,
            flg,
            ptr::null(),
            &mut len,
            ptr::null_mut(),
        );
        if out.is_null() {
            return None;
        }
        let s = core::slice::from_raw_parts(out as *const u8, len).to_vec();
        crate::alc::YYJSON_DEFAULT_ALC.dealloc(out as *mut c_void);
        Some(String::from_utf8_lossy(&s).into_owned())
    }
}

fn roundtrip(json: &str) -> Option<String> {
    let doc = read(json, 0);
    if doc.is_null() {
        return None;
    }
    let s = write(doc, 0);
    unsafe {
        let alc = (*doc).alc;
        if !(*doc).str_pool.is_null() {
            alc.dealloc((*doc).str_pool as *mut c_void);
        }
        alc.dealloc(doc as *mut c_void);
    }
    s
}

#[test]
fn layout_matches_the_header() {
    // `yyjson.h` documents these sizes and the inline functions index values
    // by them; a mismatch would silently corrupt every document.
    assert_eq!(size_of::<YyjsonVal>(), 16);
    assert_eq!(align_of::<YyjsonVal>(), 8);
    assert_eq!(size_of::<YyjsonMutVal>(), 24);
    assert_eq!(size_of::<YyjsonValUni>(), 8);
    assert_eq!(size_of::<YyjsonAlc>(), 4 * size_of::<*mut c_void>());
    assert_eq!(
        size_of::<YyjsonDoc>(),
        size_of::<*mut c_void>() * 2 + size_of::<YyjsonAlc>() + size_of::<usize>() * 2
    );
    assert_eq!(
        size_of::<YyjsonMutDoc>(),
        size_of::<*mut c_void>()
            + size_of::<YyjsonAlc>()
            + size_of::<YyjsonStrPool>()
            + size_of::<YyjsonValPool>()
    );
    // `yyjson_val_chunk` must not be larger than one mutable value, because
    // `unsafe_yyjson_val_pool_grow` reserves exactly one value for the header.
    assert!(size_of::<YyjsonValChunk>() <= size_of::<YyjsonMutVal>());

    let v = YyjsonVal {
        tag: 0,
        uni: YyjsonValUni { u64_: 0 },
    };
    let base = &v as *const YyjsonVal as usize;
    assert_eq!(&v.tag as *const u64 as usize - base, 0);
    assert_eq!(&v.uni as *const YyjsonValUni as usize - base, 8);
}

#[test]
fn tag_packing() {
    unsafe {
        let mut v = YyjsonVal {
            tag: 0,
            uni: YyjsonValUni { u64_: 0 },
        };
        set_len(&mut v, 0x1234);
        v.tag |= (YYJSON_TYPE_STR | YYJSON_SUBTYPE_NOESC) as u64;
        assert_eq!(get_len(&v), 0x1234);
        assert_eq!(get_type(&v), YYJSON_TYPE_STR);
        assert_eq!(get_subtype(&v), YYJSON_SUBTYPE_NOESC);
        inc_len(&mut v);
        assert_eq!(get_len(&v), 0x1235);
        assert_eq!(get_type(&v), YYJSON_TYPE_STR);
    }
}

#[test]
fn reads_and_writes() {
    assert_eq!(roundtrip("{\"a\":[1,2,3]}").unwrap(), "{\"a\":[1,2,3]}");
    assert_eq!(roundtrip("[]").unwrap(), "[]");
    assert_eq!(roundtrip("{}").unwrap(), "{}");
    assert_eq!(roundtrip("  [ 1 , 2 ]  ").unwrap(), "[1,2]");
    assert_eq!(roundtrip("[\n  1,\n  2\n]").unwrap(), "[1,2]");
    assert_eq!(roundtrip("\"\\u0041\"").unwrap(), "\"A\"");
    assert_eq!(roundtrip("1e2").unwrap(), "100.0");
    // yyjson reads `-0` as the *integer* zero with the sint subtype,
    // so it is written back without the sign. Kept as the C does it.
    assert_eq!(roundtrip("-0").unwrap(), "0");
    assert_eq!(roundtrip("-0.0").unwrap(), "-0.0");
    assert_eq!(roundtrip("0.1").unwrap(), "0.1");
    assert_eq!(roundtrip("1e-7").unwrap(), "1e-7");
    assert_eq!(roundtrip("1e21").unwrap(), "1e21");
    assert_eq!(roundtrip("1e20").unwrap(), "100000000000000000000.0");
    assert_eq!(roundtrip("18446744073709551615").unwrap(), "18446744073709551615");
    assert_eq!(roundtrip("-9223372036854775808").unwrap(), "-9223372036854775808");
    assert!(roundtrip("[1,]").is_none());
    assert!(roundtrip("nope").is_none());
}

#[test]
fn error_positions() {
    unsafe {
        let json = b"[1,2,}]\0";
        let mut err = YyjsonReadErr {
            code: 0,
            msg: ptr::null(),
            pos: 0,
        };
        let mut buf = json.to_vec();
        let doc = crate::read::yyjson_read_opts(
            buf.as_mut_ptr() as *mut c_char,
            7,
            0,
            ptr::null(),
            &mut err,
        );
        assert!(doc.is_null());
        assert_eq!(err.code, YYJSON_READ_ERROR_UNEXPECTED_CHARACTER);
        assert_eq!(err.pos, 5);
    }
}

#[test]
fn shortest_double_output() {
    // Values whose shortest round-trippable form is easy to get wrong.
    for (json, want) in [
        ("0.3", "0.3"),
        ("2.2250738585072014e-308", "2.2250738585072014e-308"),
        ("5e-324", "5e-324"),
        ("1.7976931348623157e308", "1.7976931348623157e308"),
        ("9.999999999999999e22", "1e23"),
        ("123456789.12345679", "123456789.12345679"),
        ("1.1", "1.1"),
        ("4294967296", "4294967296"),
    ] {
        assert_eq!(roundtrip(json).as_deref(), Some(want), "input {json}");
    }
}

#[test]
fn version_is_unchanged() {
    assert_eq!(crate::strct::yyjson_version(), 0x000D00);
}
