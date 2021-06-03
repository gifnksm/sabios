use crate::{
    desktop,
    graphics::{Draw, FrameBufferDrawer, ScreenInfo},
    prelude::*,
    sync::{Mutex, MutexGuard, OnceCell},
};
use bootloader::boot_info::FrameBuffer;

static DRAWER: OnceCell<Mutex<FrameBufferDrawer>> = OnceCell::uninit();

pub(super) fn init(frame_buffer: FrameBuffer) -> Result<ScreenInfo> {
    let mut drawer = FrameBufferDrawer::new_frame_buffer(frame_buffer)?;
    let info = drawer.info();
    drawer.fill_rect(info.area(), desktop::BG_COLOR);

    DRAWER.init_once(|| Mutex::new(drawer));

    Ok(info)
}

pub(crate) fn lock_drawer() -> MutexGuard<'static, FrameBufferDrawer> {
    DRAWER.get().lock()
}

pub(crate) unsafe fn emergency_lock_drawer() -> MutexGuard<'static, FrameBufferDrawer> {
    let drawer = DRAWER.get();
    if let Ok(drawer) = drawer.try_lock() {
        return drawer;
    }
    unsafe { drawer.force_unlock() };
    drawer.lock()
}
