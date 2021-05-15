use crate::{log::Level, prelude::*};
use core::{ptr, slice, str};

#[no_mangle]
extern "C" fn sabios_log(level: i32, msg: *const u8, len: usize) -> i32 {
    let level = match level {
        3 => Level::Error,
        4 => Level::Warn,
        6 => Level::Info,
        7 => Level::Debug,
        _ => Level::Info,
    };

    unsafe {
        let s = slice::from_raw_parts(msg, len);
        let s = str::from_utf8_unchecked(s);
        log!(level, "{}", s.trim_end());
    }

    len as i32
}

extern "C" {
    fn __errno() -> *mut i32;
}

#[allow(non_camel_case_types)]
type pid_t = i32;
const EBADF: i32 = 9;
const ENOMEM: i32 = 12;
const EINVAL: i32 = 22;

#[no_mangle]
extern "C" fn sbrk(_increment: isize) -> *const u8 {
    ptr::null()
}

#[no_mangle]
extern "C" fn _exit() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
extern "C" fn kill(_pid: pid_t, _sig: i32) -> i32 {
    unsafe {
        *__errno() = EINVAL;
    }
    -1
}

#[no_mangle]
extern "C" fn getpid() -> pid_t {
    unsafe {
        *__errno() = EINVAL;
    }
    -1
}

#[no_mangle]
extern "C" fn close() -> i32 {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn read(_fd: i32, _buf: *mut u8, _count: usize) -> isize {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn write(_fd: i32, _buf: *const u8, _count: usize) -> isize {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn lseek(_fd: i32, _offset: isize, _whence: i32) -> isize {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn fstat(_fd: i32, _buf: *mut u8) -> i32 {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn isatty(_fd: i32) -> i32 {
    unsafe {
        *__errno() = EBADF;
    }
    -1
}

#[no_mangle]
extern "C" fn posix_memalign(_memptr: *mut *mut u8, _alignment: usize, _size: usize) -> i32 {
    ENOMEM
}
