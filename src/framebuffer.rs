use crate::graphics::{Color, Point};
use bootloader::boot_info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use conquer_once::spin::OnceCell;
use core::{convert::TryInto, ops::Range};

static INFO: OnceCell<FrameBufferInfo> = OnceCell::uninit();
static DRAWER: OnceCell<spin::Mutex<Drawer>> = OnceCell::uninit();

pub(crate) fn init(framebuffer: FrameBuffer) {
    INFO.try_init_once(|| framebuffer.info())
        .expect("failed to initialize INFO");
    DRAWER
        .try_init_once(|| spin::Mutex::new(Drawer::new(framebuffer)))
        .expect("failed to initialize DRAWER");
}

pub(crate) fn info() -> &'static FrameBufferInfo {
    INFO.try_get().expect("INFO is not initialized")
}

pub(crate) fn lock_drawer() -> spin::MutexGuard<'static, Drawer> {
    // TODO: consider interrupts
    DRAWER.try_get().expect("DRAWER is not initialized").lock()
}

pub(crate) struct Drawer {
    inner: FrameBuffer,
    pixel_drawer: &'static (dyn PixelDraw + Send + Sync),
}

impl Drawer {
    fn new(inner: FrameBuffer) -> Self {
        let pixel_drawer = select_pixel_drawer(inner.info().pixel_format);
        Self {
            inner,
            pixel_drawer,
        }
    }

    pub(crate) fn info(&self) -> FrameBufferInfo {
        self.inner.info()
    }

    pub(crate) fn x_range(&self) -> Range<usize> {
        0..self.info().horizontal_resolution
    }

    pub(crate) fn y_range(&self) -> Range<usize> {
        0..self.info().vertical_resolution
    }

    pub(crate) fn draw(&mut self, p: impl TryInto<Point<usize>>, c: Color) -> bool {
        let pixel_index = match self.pixel_index(p) {
            Some(p) => p,
            None => return false,
        };
        self.pixel_drawer
            .pixel_draw(self.inner.buffer_mut(), pixel_index, c)
    }

    fn pixel_index(&self, p: impl TryInto<Point<usize>>) -> Option<usize> {
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
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color) -> bool;
}

fn select_pixel_drawer(pixel_format: PixelFormat) -> &'static (dyn PixelDraw + Send + Sync) {
    match pixel_format {
        PixelFormat::RGB => &RGB_PIXEL_DRAWER as _,
        PixelFormat::BGR => &BGR_PIXEL_DRAWER as _,
        PixelFormat::U8 => &U8_PIXEL_DRAWER as _,
        _ => &UNSUPPORTED_PIXEL_DRAWER as _,
    }
}

#[derive(Debug, Clone, Copy)]
struct RgbPixelDrawer;
static RGB_PIXEL_DRAWER: RgbPixelDrawer = RgbPixelDrawer;
impl PixelDraw for RgbPixelDrawer {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color) -> bool {
        buffer[pixel_index] = c.r;
        buffer[pixel_index + 1] = c.g;
        buffer[pixel_index + 2] = c.b;
        true
    }
}

#[derive(Debug, Clone, Copy)]
struct BgrPixelDrawer;
static BGR_PIXEL_DRAWER: BgrPixelDrawer = BgrPixelDrawer;
impl PixelDraw for BgrPixelDrawer {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color) -> bool {
        buffer[pixel_index] = c.b;
        buffer[pixel_index + 1] = c.g;
        buffer[pixel_index + 2] = c.r;
        true
    }
}

#[derive(Debug, Clone, Copy)]
struct U8PixelDrawer;
static U8_PIXEL_DRAWER: U8PixelDrawer = U8PixelDrawer;
impl PixelDraw for U8PixelDrawer {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color) -> bool {
        buffer[pixel_index] = c.to_grayscale();
        true
    }
}

#[derive(Debug, Clone, Copy)]
struct UnsupportedPixelDrawer;
static UNSUPPORTED_PIXEL_DRAWER: UnsupportedPixelDrawer = UnsupportedPixelDrawer;
impl PixelDraw for UnsupportedPixelDrawer {
    fn pixel_draw(&self, _buffer: &mut [u8], _pixel_index: usize, _c: Color) -> bool {
        false
    }
}
