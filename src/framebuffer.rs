#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use crate::graphics::{Color, Point};
use bootloader::boot_info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use conquer_once::{spin::OnceCell, TryGetError, TryInitError};
use core::{convert::TryInto, fmt, ops::Range};

static INFO: OnceCell<FrameBufferInfo> = OnceCell::uninit();
static DRAWER: OnceCell<spin::Mutex<Drawer>> = OnceCell::uninit();

#[derive(Debug)]
pub(crate) enum InitError {
    UnsupportedPixelFormat(PixelFormat),
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
    let drawer = Drawer {
        inner: framebuffer,
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

#[derive(Debug)]
pub(crate) enum DrawError {
    InvalidPoint(Point<usize>),
}

impl fmt::Display for DrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrawError::InvalidPoint(p) => write!(f, "invalid point: {}", p),
        }
    }
}

pub(crate) struct Drawer {
    inner: FrameBuffer,
    pixel_drawer: &'static (dyn PixelDraw + Send + Sync),
}

impl Drawer {
    pub(crate) fn info(&self) -> FrameBufferInfo {
        self.inner.info()
    }

    pub(crate) fn x_range(&self) -> Range<usize> {
        0..self.info().horizontal_resolution
    }

    pub(crate) fn y_range(&self) -> Range<usize> {
        0..self.info().vertical_resolution
    }

    pub(crate) fn draw(&mut self, p: Point<usize>, c: Color) -> Result<(), DrawError> {
        let pixel_index = self.pixel_index(p).ok_or(DrawError::InvalidPoint(p))?;
        self.pixel_drawer
            .pixel_draw(self.inner.buffer_mut(), pixel_index, c);
        Ok(())
    }

    fn pixel_index(&self, p: Point<usize>) -> Option<usize> {
        let FrameBufferInfo {
            bytes_per_pixel,
            stride,
            ..
        } = self.info();

        let Point { x, y } = p.try_into().ok()?;
        if !self.x_range().contains(&x) || !self.y_range().contains(&y) {
            return None;
        }
        Some((y * stride + x) * bytes_per_pixel)
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
