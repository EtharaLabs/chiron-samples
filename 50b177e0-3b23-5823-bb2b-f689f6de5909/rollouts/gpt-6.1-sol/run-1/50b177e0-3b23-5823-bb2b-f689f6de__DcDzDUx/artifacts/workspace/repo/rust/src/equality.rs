use crate::{abi::*, document::imut_next};

unsafe fn str_equals(lhs: *const Val, rhs: *const Val) -> bool {
    let len = ((*lhs).tag >> 8) as usize;
    if len != ((*rhs).tag >> 8) as usize { return false; }
    let mut i = 0;
    while i < len {
        if *(*lhs).uni.str_.add(i) != *(*rhs).uni.str_.add(i) { return false; }
        i += 1;
    }
    true
}

unsafe fn scalar_equals(lhs: *const Val, rhs: *const Val, kind: u64) -> bool {
    match kind {
        4 => {
            let lt = (*lhs).tag & 0x18;
            let rt = (*rhs).tag & 0x18;
            if lt == rt { return (*lhs).uni.u64_ == (*rhs).uni.u64_; }
            if lt == 8 && rt == 0 {
                return (*lhs).uni.i64_ >= 0 && (*lhs).uni.u64_ == (*rhs).uni.u64_;
            }
            if lt == 0 && rt == 8 {
                return (*rhs).uni.i64_ >= 0 && (*lhs).uni.u64_ == (*rhs).uni.u64_;
            }
            false
        }
        1 | 5 => str_equals(lhs, rhs),
        2 | 3 => (*lhs).tag == (*rhs).tag,
        _ => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_equals(mut lhs: *const Val, mut rhs: *const Val) -> bool {
    let kind = (*lhs).tag & 7;
    if kind != (*rhs).tag & 7 { return false; }
    if kind != 6 && kind != 7 { return scalar_equals(lhs, rhs, kind); }
    let len = ((*lhs).tag >> 8) as usize;
    if len != ((*rhs).tag >> 8) as usize { return false; }
    if kind == 6 {
        lhs = lhs.add(1);
        rhs = rhs.add(1);
        for _ in 0..len {
            if !unsafe_yyjson_equals(lhs, rhs) { return false; }
            lhs = imut_next(lhs);
            rhs = imut_next(rhs);
        }
    } else if len > 0 {
        let obj = rhs;
        let mut cursor = obj.add(1);
        let mut iter_index = 0;
        lhs = lhs.add(1);
        for _ in 0..len {
            let mut index = iter_index;
            let mut max = len;
            let mut cur = cursor;
            if index == max { index = 0; cur = obj.add(1); }
            let mut found = false;
            while index < max {
                index += 1;
                let next = imut_next(cur.add(1));
                if str_equals(cur, lhs) {
                    iter_index = index;
                    cursor = next;
                    rhs = cur.add(1);
                    found = true;
                    break;
                }
                cur = next;
                if index == len && iter_index < len {
                    index = 0;
                    max = iter_index;
                    cur = obj.add(1);
                }
            }
            if !found || !unsafe_yyjson_equals(lhs.add(1), rhs) { return false; }
            lhs = imut_next(lhs.add(1));
        }
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn unsafe_yyjson_mut_equals(mut lhs: *const MutVal, mut rhs: *const MutVal) -> bool {
    let kind = (*lhs).tag & 7;
    if kind != (*rhs).tag & 7 { return false; }
    if kind != 6 && kind != 7 { return scalar_equals(lhs.cast(), rhs.cast(), kind); }
    let len = ((*lhs).tag >> 8) as usize;
    if len != ((*rhs).tag >> 8) as usize { return false; }
    if len == 0 { return true; }
    lhs = (*lhs).uni.ptr.cast();
    rhs = (*rhs).uni.ptr.cast();
    if kind == 6 {
        for _ in 0..len {
            if !unsafe_yyjson_mut_equals(lhs, rhs) { return false; }
            lhs = (*lhs).next;
            rhs = (*rhs).next;
        }
    } else {
        let mut cursor = rhs;
        for _ in 0..len {
            let mut cur = cursor;
            let mut found = false;
            for _ in 0..len {
                cur = (*(*cur).next).next;
                if str_equals(cur.cast(), lhs.cast()) {
                    cursor = cur;
                    rhs = (*cur).next;
                    found = true;
                    break;
                }
            }
            if !found || !unsafe_yyjson_mut_equals((*lhs).next, rhs) { return false; }
            lhs = (*(*lhs).next).next;
        }
    }
    true
}
