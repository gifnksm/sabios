use crate::{
    buffer_drawer::FrameBufferDrawer,
    desktop,
    graphics::{Draw, Point, Rectangle, Size},
    prelude::*,
    sync::{Mutex, MutexGuard, OnceCell},
};
use bootloader::boot_info::{FrameBuffer, PixelFormat};

pub(crate) type Drawer = FrameBufferDrawer;

static INFO: OnceCell<ScreenInfo> = OnceCell::uninit();
static DRAWER: OnceCell<Mutex<Drawer>> = OnceCell::uninit();

pub(crate) fn init(framebuffer: FrameBuffer) -> Result<()> {
    let mut drawer = Drawer::new_framebuffer(framebuffer)?;
    let info = drawer.info();
    drawer.fill_rect(info.area(), desktop::BG_COLOR);

    INFO.init_once(|| info);
    DRAWER.init_once(|| Mutex::new(drawer));

    info!(
        "screen: size={}, bytes_per_pixel={}, pixel_format={:?}",
        info.size, info.bytes_per_pixel, info.pixel_format,
    );

    Ok(())
}

pub(crate) fn info() -> &'static ScreenInfo {
    INFO.get()
}

pub(crate) fn lock_drawer() -> MutexGuard<'static, Drawer> {
    DRAWER.get().lock()
}

pub(crate) unsafe fn emergency_lock_drawer() -> MutexGuard<'static, Drawer> {
    let drawer = DRAWER.get();
    if let Ok(drawer) = drawer.try_lock() {
        return drawer;
    }
    unsafe { drawer.force_unlock() };
    drawer.lock()
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScreenInfo {
    pub(crate) size: Size<i32>,
    pub(crate) bytes_per_pixel: i32,
    pub(crate) pixel_format: PixelFormat,
}

impl ScreenInfo {
    pub(crate) fn area(&self) -> Rectangle<i32> {
        Rectangle::new(Point::new(0, 0), self.size)
    }
}
