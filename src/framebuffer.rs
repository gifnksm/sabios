use crate::{
    desktop,
    error::ConvertErr as _,
    graphics::{Color, Draw, Point, Rectangle, Size},
    prelude::*,
    sync::mutex::{Mutex, MutexGuard},
    util,
};
use bootloader::boot_info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use conquer_once::noblock::OnceCell;
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

    INFO.try_init_once(|| info)
        .convert_err("framebuffer::INFO")?;
    DRAWER
        .try_init_once(|| Mutex::new(drawer))
        .convert_err("framebuffer::DRAWER")?;
    Ok(())
}
pub(crate) fn info() -> Result<&'static ScreenInfo> {
    INFO.try_get().convert_err("framebuffer::INFO")
}

pub(crate) fn lock_drawer() -> Result<MutexGuard<'static, Drawer>> {
    util::try_get_and_lock(&DRAWER, "framebuffer::DRAWER")
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScreenInfo {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) stride: i32,
    pub(crate) bytes_per_pixel: i32,
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
    fn area(&self) -> Rectangle<i32> {
        Rectangle {
            pos: Point::new(0, 0),
            size: Point::new(self.info.width, self.info.height),
        }
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        if let Some(pixel_index) = self.pixel_index(p) {
            self.pixel_drawer
                .pixel_draw(self.framebuffer.buffer_mut(), pixel_index, c)
        }
    }
}

impl Drawer {
    fn pixel_index(&self, p: Point<i32>) -> Option<usize> {
        if !self.area().contains(&p) {
            return None;
        }
        usize::try_from((p.y * self.info.stride + p.x) * self.info.bytes_per_pixel).ok()
    }
}

trait PixelDraw {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color);
}

fn select_pixel_drawer(
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
