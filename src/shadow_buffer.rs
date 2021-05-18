use crate::{
    framebuffer::{self, PixelDraw},
    graphics::{Color, Draw, Point, Size},
};
use alloc::{vec, vec::Vec};
use core::convert::TryFrom;

pub(crate) struct ShadowBuffer {
    size: Size<i32>,
    bytes_per_pixel: i32,
    data: Vec<u8>,
    pixel_drawer: &'static (dyn PixelDraw + Send + Sync),
}

impl Draw for ShadowBuffer {
    fn size(&self) -> Size<i32> {
        self.size
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        if let Some(pixel_index) = self.pixel_index(p) {
            self.pixel_drawer.pixel_draw(&mut self.data, pixel_index, c)
        }
    }
}

impl ShadowBuffer {
    pub(crate) fn new(size: Size<i32>) -> Self {
        let screen_info = *framebuffer::info();

        #[allow(clippy::unwrap_used)]
        let data = vec![0; usize::try_from(size.x * size.y * screen_info.bytes_per_pixel).unwrap()];
        let bytes_per_pixel = screen_info.bytes_per_pixel;
        #[allow(clippy::unwrap_used)]
        let pixel_drawer = framebuffer::select_pixel_drawer(screen_info.pixel_format).unwrap();

        Self {
            size,
            bytes_per_pixel,
            data,
            pixel_drawer,
        }
    }

    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }

    fn pixel_index(&self, p: Point<i32>) -> Option<usize> {
        if !self.area().contains(&p) {
            return None;
        }
        usize::try_from((p.y * self.size.x + p.x) * self.bytes_per_pixel).ok()
    }
}
