use crate::{
    desktop,
    graphics::{Color, Draw, Point, Rectangle, Size},
    prelude::*,
    shadow_buffer::ShadowBuffer,
    sync::{
        mutex::{Mutex, MutexGuard},
        once_cell::OnceCell,
    },
};
use bootloader::boot_info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use core::convert::TryFrom;

static INFO: OnceCell<ScreenInfo> = OnceCell::uninit();
static DRAWER: OnceCell<Mutex<Drawer>> = OnceCell::uninit();

pub(crate) fn init(framebuffer: FrameBuffer) -> Result<()> {
    let original_info = framebuffer.info();
    let pixel_format = original_info.pixel_format;
    let pixel_drawer =
        select_pixel_drawer(pixel_format).ok_or(ErrorKind::UnsupportedPixelFormat(pixel_format))?;

    let info = ScreenInfo::new(&original_info)?;

    let mut drawer = Drawer {
        framebuffer,
        info,
        pixel_drawer,
    };

    drawer.fill_rect(info.area(), desktop::BG_COLOR);

    INFO.init_once(|| info);
    DRAWER.init_once(|| Mutex::new(drawer));
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
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) stride: i32,
    pub(crate) bytes_per_pixel: i32,
    pub(crate) pixel_format: PixelFormat,
}

impl ScreenInfo {
    fn new(info: &FrameBufferInfo) -> Result<Self> {
        fn usize_to_i32(name: &'static str, value: usize) -> Result<i32> {
            Ok(i32::try_from(value).map_err(|_e| ErrorKind::ParameterTooLarge(name, value))?)
        }

        Ok(Self {
            width: usize_to_i32("horizontal_resolution", info.horizontal_resolution)?,
            height: usize_to_i32("vertical_resolution", info.vertical_resolution)?,
            stride: usize_to_i32("stride", info.stride)?,
            bytes_per_pixel: usize_to_i32("byte_per_pixel", info.bytes_per_pixel)?,
            pixel_format: info.pixel_format,
        })
    }

    pub(crate) fn area(&self) -> Rectangle<i32> {
        Rectangle::new(Point::new(0, 0), self.size())
    }

    pub(crate) fn size(&self) -> Size<i32> {
        Size::new(self.width, self.height)
    }
}

pub(crate) struct Drawer {
    framebuffer: FrameBuffer,
    info: ScreenInfo,
    pixel_drawer: &'static (dyn PixelDraw + Send + Sync),
}

impl Draw for Drawer {
    fn size(&self) -> Size<i32> {
        Size::new(self.info.width, self.info.height)
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        if let Some(pixel_index) = self.pixel_index(p) {
            self.pixel_drawer
                .pixel_draw(self.framebuffer.buffer_mut(), pixel_index, c)
        }
    }
}

impl Drawer {
    pub(crate) fn copy(&mut self, pos: Point<i32>, src: &ShadowBuffer) {
        let dst_size = self.size();
        let src_size = src.size();

        let copy_start_dst_x = i32::max(pos.x, 0);
        let copy_start_dst_y = i32::max(pos.y, 0);
        let copy_end_dst_x = i32::min(pos.x + src_size.x, dst_size.x);
        let copy_end_dst_y = i32::min(pos.y + src_size.y, dst_size.y);

        let stride = self.info.stride;
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let bytes_per_copy_line = (bytes_per_pixel * (copy_end_dst_x - copy_start_dst_x)) as usize;

        let dst_start_idx =
            (bytes_per_pixel * (stride * copy_start_dst_y + copy_start_dst_x)) as usize;
        let dst_buf = &mut self.framebuffer.buffer_mut()[dst_start_idx..];
        let src_buf = src.data();

        for dy in 0..(copy_end_dst_y - copy_start_dst_y) {
            let dst =
                &mut dst_buf[(bytes_per_pixel * dy * stride) as usize..][..bytes_per_copy_line];
            let src =
                &src_buf[(bytes_per_pixel * dy * src_size.x) as usize..][..bytes_per_copy_line];
            dst.copy_from_slice(src);
        }
    }

    fn pixel_index(&self, p: Point<i32>) -> Option<usize> {
        if !self.area().contains(&p) {
            return None;
        }
        usize::try_from((p.y * self.info.stride + p.x) * self.info.bytes_per_pixel).ok()
    }
}

pub(crate) trait PixelDraw {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color);
}

pub(crate) fn select_pixel_drawer(
    pixel_format: PixelFormat,
) -> Option<&'static (dyn PixelDraw + Send + Sync)> {
    match pixel_format {
        PixelFormat::RGB => Some(&RGB_PIXEL_DRAWER as _),
        PixelFormat::BGR => Some(&BGR_PIXEL_DRAWER as _),
        PixelFormat::U8 => Some(&U8_PIXEL_DRAWER as _),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
struct RgbPixelDrawer;
static RGB_PIXEL_DRAWER: RgbPixelDrawer = RgbPixelDrawer;
impl PixelDraw for RgbPixelDrawer {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color) {
        buffer[pixel_index] = c.r;
        buffer[pixel_index + 1] = c.g;
        buffer[pixel_index + 2] = c.b;
    }
}

#[derive(Debug, Clone, Copy)]
struct BgrPixelDrawer;
static BGR_PIXEL_DRAWER: BgrPixelDrawer = BgrPixelDrawer;
impl PixelDraw for BgrPixelDrawer {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color) {
        buffer[pixel_index] = c.b;
        buffer[pixel_index + 1] = c.g;
        buffer[pixel_index + 2] = c.r;
    }
}

#[derive(Debug, Clone, Copy)]
struct U8PixelDrawer;
static U8_PIXEL_DRAWER: U8PixelDrawer = U8PixelDrawer;
impl PixelDraw for U8PixelDrawer {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color) {
        buffer[pixel_index] = c.to_grayscale();
    }
}
