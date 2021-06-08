use crate::sync::{Mutex, OnceCell};
use core::slice;

extern "C" {
    static mut _binary_fs_fat_start: u8;
    static _binary_fs_fat_size: usize;
}

static FILESYSTEM: OnceCell<Mutex<&'static [u8]>> = OnceCell::uninit();

pub(crate) fn init() {
    let filesystem: &'static [u8] = unsafe {
        slice::from_raw_parts_mut(
            &mut _binary_fs_fat_start as *mut u8,
            &_binary_fs_fat_size as *const usize as usize,
        )
    };

    for (i, chunk) in filesystem.chunks(16).take(16).enumerate() {
        crate::print!("{:04x}", i);
        for (j, byte) in chunk.iter().enumerate() {
            crate::print!(" {:02x}", byte);
            if j % 8 == 7 {
                crate::print!(" ");
            }
        }
        crate::println!();
    }

    FILESYSTEM.init_once(|| Mutex::new(filesystem));
}
