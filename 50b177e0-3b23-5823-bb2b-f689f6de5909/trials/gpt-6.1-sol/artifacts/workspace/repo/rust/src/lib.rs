// Copyright (C) 2019 Yaoyuan <ibireme@gmail.com>.
// Rust port: released under the MIT license; see ../../LICENSE.
#![allow(clippy::missing_safety_doc)]

mod abi;
mod allocator;
mod document;
mod equality;

pub use abi::*;
pub use allocator::*;
pub use document::*;
pub use equality::*;

#[no_mangle]
pub extern "C" fn yyjson_version() -> u32 {
    0x000d00
}

/// Count bytes using yyjson's deliberately permissive character stepping.
/// In particular this is not a UTF-8 validation operation.
#[no_mangle]
pub unsafe extern "C" fn yyjson_locate_pos(
    str_: *const u8,
    len: usize,
    pos: usize,
    line: *mut usize,
    col: *mut usize,
    chr: *mut usize,
) -> bool {
    if str_.is_null() || pos > len {
        if !line.is_null() { *line = 0; }
        if !col.is_null() { *col = 0; }
        if !chr.is_null() { *chr = 0; }
        return false;
    }
    let mut cur = 0;
    let mut lines = 0;
    let mut line_pos = 0;
    let mut chars = 0;
    if pos >= 3 && *str_ == 0xef && *str_.add(1) == 0xbb && *str_.add(2) == 0xbf {
        cur = 3;
    }
    while cur < pos {
        let c = *str_.add(cur);
        chars += 1;
        cur += if c < 0x80 {
            if c == b'\n' {
                lines += 1;
                line_pos = chars;
            }
            1
        } else if c < 0xc0 { 1 }
        else if c < 0xe0 { 2 }
        else if c < 0xf0 { 3 }
        else if c < 0xf8 { 4 }
        else { 1 };
    }
    if !line.is_null() { *line = lines + 1; }
    if !col.is_null() { *col = chars - line_pos + 1; }
    if !chr.is_null() { *chr = chars; }
    true
}
