use bootloader::boot_info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use conquer_once::spin::OnceCell;
use core::{convert::TryFrom, ops::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}

#[allow(dead_code)]
impl Color {
    pub(crate) const RED: Self = Color::new(255, 0, 0);
    pub(crate) const GREEN: Self = Color::new(0, 255, 0);
    pub(crate) const BLUE: Self = Color::new(0, 0, 255);
    pub(crate) const BLACK: Self = Color::new(0, 0, 0);
    pub(crate) const WHITE: Self = Color::new(255, 255, 255);
}

impl Color {
    pub(crate) const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    pub(crate) fn to_grayscale(self) -> u8 {
        u8::try_from((u16::from(self.r) + u16::from(self.g) + u16::from(self.b)) / 3).unwrap()
    }
}

pub(crate) struct Vector2d<T> {
    pub(crate) x: T,
    pub(crate) y: T,
}

impl<T> Vector2d<T> {
    pub(crate) fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

pub(crate) type Point<T> = Vector2d<T>;

static INFO: OnceCell<FrameBufferInfo> = OnceCell::uninit();
static DRAWER: OnceCell<spin::Mutex<Drawer>> = OnceCell::uninit();

pub(crate) fn init(framebuffer: FrameBuffer) {
    INFO.try_init_once(|| framebuffer.info())
        .expect("faield to initialize INFO");
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

    pub(crate) fn draw<T>(&mut self, p: Point<T>, c: Color) -> bool
    where
        usize: TryFrom<T>,
    {
        let pixel_index = match self.pixel_index(p) {
            Some(p) => p,
            None => return false,
        };
        self.pixel_drawer
            .pixel_draw(self.inner.buffer_mut(), pixel_index, c)
    }

    fn pixel_index<T>(&self, p: Point<T>) -> Option<usize>
    where
        usize: TryFrom<T>,
    {
        let FrameBufferInfo {
            bytes_per_pixel,
            stride,
            ..
        } = self.info();

        let x = usize::try_from(p.x).ok()?;
        let y = usize::try_from(p.y).ok()?;
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
