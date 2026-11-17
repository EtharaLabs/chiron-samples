use core::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Alc {
    pub malloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub realloc: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> *mut c_void>,
    pub free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub ctx: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ValUni {
    pub u64_: u64,
    pub i64_: i64,
    pub f64_: f64,
    pub str_: *const u8,
    pub ptr: *mut c_void,
    pub ofs: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Val {
    pub tag: u64,
    pub uni: ValUni,
}

#[repr(C)]
pub struct Doc {
    pub root: *mut Val,
    pub alc: Alc,
    pub dat_read: usize,
    pub val_read: usize,
    pub str_pool: *mut u8,
}

#[repr(C)]
pub struct MutVal {
    pub tag: u64,
    pub uni: ValUni,
    pub next: *mut MutVal,
}

#[repr(C)]
pub struct StrChunk {
    pub next: *mut StrChunk,
    pub chunk_size: usize,
}

#[repr(C)]
pub struct StrPool {
    pub cur: *mut u8,
    pub end: *mut u8,
    pub chunk_size: usize,
    pub chunk_size_max: usize,
    pub chunks: *mut StrChunk,
}

#[repr(C)]
pub struct ValChunk {
    pub next: *mut ValChunk,
    pub chunk_size: usize,
}

#[repr(C)]
pub struct ValPool {
    pub cur: *mut MutVal,
    pub end: *mut MutVal,
    pub chunk_size: usize,
    pub chunk_size_max: usize,
    pub chunks: *mut ValChunk,
}

#[repr(C)]
pub struct MutDoc {
    pub root: *mut MutVal,
    pub alc: Alc,
    pub str_pool: StrPool,
    pub val_pool: ValPool,
}
