use crate::{
    prelude::*,
    sync::{Mutex, MutexGuard, OnceCell},
};

pub(crate) use self::{bpb::*, directory_entry::*};

mod bpb;
mod directory_entry;

#[derive(Debug)]
pub(crate) enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

extern "C" {
    static mut _binary_fs_fat_start: u8;
    static _binary_fs_fat_size: usize;
}

static FILESYSTEM: OnceCell<Mutex<&'static mut dyn BiosParameterBlock>> = OnceCell::uninit();

pub(crate) fn init() {
    let filesystem = unsafe { bpb::get(&mut _binary_fs_fat_start) };
    info!("file system type: {:?}", filesystem.fat_type());
    info!("{:?}", filesystem);
    FILESYSTEM.init_once(move || Mutex::new(filesystem));
}

pub(crate) fn lock() -> MutexGuard<'static, &'static mut dyn BiosParameterBlock> {
    FILESYSTEM.get().lock()
}
