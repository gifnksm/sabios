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
}

impl Drawer {
    fn new(inner: FrameBuffer) -> Self {
        Self { inner }
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
        match self.info().pixel_format {
            PixelFormat::RGB => {
                self.inner.buffer_mut()[pixel_index] = c.r;
                self.inner.buffer_mut()[pixel_index + 1] = c.g;
                self.inner.buffer_mut()[pixel_index + 2] = c.b;
            }
            PixelFormat::BGR => {
                self.inner.buffer_mut()[pixel_index] = c.b;
                self.inner.buffer_mut()[pixel_index + 1] = c.g;
                self.inner.buffer_mut()[pixel_index + 2] = c.r;
            }
            PixelFormat::U8 => {
                self.inner.buffer_mut()[pixel_index] = c.to_grayscale();
            }
            _ => return false,
        }
        true
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
