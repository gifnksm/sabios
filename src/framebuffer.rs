#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use crate::graphics::{Color, Draw, Point, Rectangle};
use bootloader::boot_info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use conquer_once::{spin::OnceCell, TryGetError, TryInitError};
use core::{convert::TryFrom, fmt};

static INFO: OnceCell<FrameBufferInfo> = OnceCell::uninit();
static DRAWER: OnceCell<spin::Mutex<Drawer>> = OnceCell::uninit();

#[derive(Debug)]
pub(crate) enum InitError {
    UnsupportedPixelFormat(PixelFormat),
    ParameterTooLarge(&'static str, usize),
    AlreadyInit,
    WouldBlock,
}

impl From<TryInitError> for InitError {
    fn from(err: TryInitError) -> Self {
        match err {
            TryInitError::AlreadyInit => InitError::AlreadyInit,
            TryInitError::WouldBlock => InitError::WouldBlock,
        }
    }
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPixelFormat(pixel_format) => {
                write!(f, "unsupported pixel format: {:?}", pixel_format)
            }
            Self::ParameterTooLarge(name, value) => write!(f, "too large {}: {}", name, value),
            Self::AlreadyInit => write!(f, "framebuffer has already been initialized"),
            Self::WouldBlock => write!(f, "framebuffer is currently being initialized"),
        }
    }
}

pub(crate) fn init(framebuffer: FrameBuffer) -> Result<(), InitError> {
    let info = framebuffer.info();
    let pixel_format = info.pixel_format;
    let pixel_drawer =
        select_pixel_drawer(pixel_format).ok_or(InitError::UnsupportedPixelFormat(pixel_format))?;

    fn usize_to_i32(name: &'static str, value: usize) -> Result<i32, InitError> {
        i32::try_from(value).map_err(|_e| InitError::ParameterTooLarge(name, value))
    }

    let drawer = Drawer {
        framebuffer,
        width: usize_to_i32("horizontal_resolution", info.horizontal_resolution)?,
        height: usize_to_i32("vertical_resolution", info.vertical_resolution)?,
        stride: usize_to_i32("stride", info.stride)?,
        bytes_per_pixel: usize_to_i32("byte_per_pixel", info.bytes_per_pixel)?,
        pixel_drawer,
    };

    INFO.try_init_once(|| info)?;
    DRAWER.try_init_once(|| spin::Mutex::new(drawer))?;
    Ok(())
}

#[derive(Debug)]
pub(crate) enum AccessError {
    Uninit,
    WouldBlock,
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessError::Uninit => write!(f, "framebuffer is uninitialized"),
            AccessError::WouldBlock => write!(f, "framebuffer is currently being initialized"),
        }
    }
}

impl From<TryGetError> for AccessError {
    fn from(err: TryGetError) -> Self {
        match err {
            TryGetError::Uninit => Self::Uninit,
            TryGetError::WouldBlock => Self::WouldBlock,
        }
    }
}

pub(crate) fn info() -> Result<&'static FrameBufferInfo, AccessError> {
    Ok(INFO.try_get()?)
}

pub(crate) fn lock_drawer() -> Result<spin::MutexGuard<'static, Drawer>, AccessError> {
    // TODO: consider interrupts
    Ok(DRAWER.try_get()?.lock())
}

pub(crate) struct Drawer {
    framebuffer: FrameBuffer,
    width: i32,
    height: i32,
    stride: i32,
    bytes_per_pixel: i32,
    pixel_drawer: &'static (dyn PixelDraw + Send + Sync),
}

impl Draw for Drawer {
    fn area(&self) -> crate::graphics::Rectangle<i32> {
        Rectangle {
            pos: Point::new(0i32, 0i32),
            size: Point::new(self.width, self.height),
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
        usize::try_from((p.y * self.stride + p.x) * self.bytes_per_pixel).ok()
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
