use crate::{prelude::*, sync::OnceCell};
use bootloader::boot_info::{FrameBuffer, PixelFormat};

pub(crate) use self::{buffer_drawer::*, color::*, geometry::*, traits::*};

mod buffer_drawer;
mod color;
pub(crate) mod font;
pub(crate) mod frame_buffer;
mod geometry;
mod traits;

static SCREEN_INFO: OnceCell<ScreenInfo> = OnceCell::uninit();

pub(crate) fn init(frame_buffer: FrameBuffer) -> Result<()> {
    let screen_info = frame_buffer::init(frame_buffer)?;
    info!(
        "screen: size={}, bytes_per_pixel={}, pixel_format={:?}",
        screen_info.size, screen_info.bytes_per_pixel, screen_info.pixel_format,
    );

    SCREEN_INFO.init_once(|| screen_info);

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScreenInfo {
    pub(crate) size: Size<i32>,
    pub(crate) bytes_per_pixel: i32,
    pub(crate) pixel_format: PixelFormat,
}

impl ScreenInfo {
    pub(crate) fn get() -> ScreenInfo {
        *SCREEN_INFO.get()
    }

    pub(crate) fn area(&self) -> Rectangle<i32> {
        Rectangle::new(Point::new(0, 0), self.size)
    }
}
