use crate::{
    graphics::{Color, Draw, Offset, Point, Rectangle, ScreenInfo, Size},
    prelude::*,
};
use alloc::{vec, vec::Vec};
use bootloader::boot_info::{FrameBuffer, PixelFormat};
use core::{cmp::Ordering, convert::TryFrom, ptr};
use custom_debug_derive::Debug as CustomDebug;
use derivative::Derivative;

pub(crate) type FrameBufferDrawer = BufferDrawer<FrameBuffer>;
pub(crate) type ShadowBuffer = BufferDrawer<Vec<u8>>;

pub(crate) trait Buffer {
    fn buffer(&self) -> &[u8];
    fn buffer_mut(&mut self) -> &mut [u8];
}

impl Buffer for FrameBuffer {
    fn buffer(&self) -> &[u8] {
        self.buffer()
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }
}

impl Buffer for Vec<u8> {
    fn buffer(&self) -> &[u8] {
        self
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        self
    }
}

#[derive(Derivative)]
#[derivative(Clone(clone_from = "true"))]
#[derive(CustomDebug)]
pub(crate) struct BufferDrawer<B> {
    size: Size<i32>,
    stride: i32,
    bytes_per_pixel: i32,
    pixel_format: PixelFormat,
    #[debug(skip)]
    pixel_drawer: &'static (dyn PixelDraw + Send + Sync),
    buffer: B,
}

impl<B> BufferDrawer<B> {
    fn new_common(
        size: Size<i32>,
        stride: i32,
        bytes_per_pixel: i32,
        pixel_format: PixelFormat,
        buffer: B,
    ) -> Result<Self> {
        let pixel_drawer = select_pixel_drawer(pixel_format)?;
        Ok(Self {
            size,
            stride,
            bytes_per_pixel,
            pixel_format,
            pixel_drawer,
            buffer,
        })
    }
}

impl FrameBufferDrawer {
    pub(crate) fn new_frame_buffer(buffer: FrameBuffer) -> Result<Self> {
        let info = buffer.info();
        let size = Size::new(
            i32::try_from(info.horizontal_resolution)?,
            i32::try_from(info.vertical_resolution)?,
        );
        let stride = i32::try_from(info.stride)?;
        let bytes_per_pixel = i32::try_from(info.bytes_per_pixel)?;
        let pixel_format = info.pixel_format;
        Self::new_common(size, stride, bytes_per_pixel, pixel_format, buffer)
    }
}

impl ShadowBuffer {
    pub(crate) fn new_shadow(size: Size<i32>, screen_info: ScreenInfo) -> Result<Self> {
        let stride = size.x;
        let bytes_per_pixel = screen_info.bytes_per_pixel;
        let pixel_format = screen_info.pixel_format;
        let buffer = vec![0; usize::try_from(size.x * size.y * bytes_per_pixel)?];
        Self::new_common(size, stride, bytes_per_pixel, pixel_format, buffer)
    }
}

impl<B> Draw for BufferDrawer<B>
where
    B: Buffer,
{
    fn size(&self) -> Size<i32> {
        self.size
    }

    fn draw(&mut self, p: crate::graphics::Point<i32>, c: crate::graphics::Color) {
        if let Some(pixel_index) = self.pixel_index(p) {
            self.pixel_drawer
                .pixel_draw(self.buffer.buffer_mut(), pixel_index, c)
        }
    }

    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>) {
        if offset.x == 0 && offset.y == 0 {
            return;
        }

        (|| {
            let dst = (((src & self.area())? + offset) & self.area())?;
            let src = dst - offset;

            assert_eq!(dst.size, src.size);
            let line_copy_bytes = (dst.size.x * self.bytes_per_pixel) as usize;

            match i32::cmp(&dst.pos.y, &src.pos.y) {
                Ordering::Less => {
                    // move up
                    for dy in 0..dst.size.y {
                        let dp = Point::new(0, dy);
                        let dst_idx = self.pixel_index(dst.pos + dp)?;
                        let src_idx = self.pixel_index(src.pos + dp)?;
                        let dst_ptr = self.buffer.buffer_mut()[dst_idx..].as_mut_ptr();
                        let src_ptr = self.buffer.buffer()[src_idx..].as_ptr();
                        unsafe { ptr::copy_nonoverlapping(src_ptr, dst_ptr, line_copy_bytes) };
                    }
                }
                Ordering::Equal => {
                    // move left / move right
                    for dy in 0..dst.size.y {
                        let dp = Point::new(0, dy);
                        let dst_idx = self.pixel_index(dst.pos + dp)?;
                        let src_idx = self.pixel_index(src.pos + dp)?;
                        let dst_ptr = self.buffer.buffer_mut()[dst_idx..].as_mut_ptr();
                        let src_ptr = self.buffer.buffer()[src_idx..].as_ptr();
                        unsafe { ptr::copy(src_ptr, dst_ptr, line_copy_bytes) };
                    }
                }
                Ordering::Greater => {
                    // move down
                    for dy in (0..dst.size.y).rev() {
                        let dp = Point::new(0, dy);
                        let dst_idx = self.pixel_index(dst.pos + dp)?;
                        let src_idx = self.pixel_index(src.pos + dp)?;
                        let dst_ptr = self.buffer.buffer_mut()[dst_idx..].as_mut_ptr();
                        let src_ptr = self.buffer.buffer()[src_idx..].as_ptr();
                        unsafe { ptr::copy_nonoverlapping(src_ptr, dst_ptr, line_copy_bytes) };
                    }
                }
            }

            Some(())
        })();
    }
}

impl<B> BufferDrawer<B>
where
    B: Buffer,
{
    pub(crate) fn info(&self) -> ScreenInfo {
        ScreenInfo {
            size: self.size,
            bytes_per_pixel: self.bytes_per_pixel,
            pixel_format: self.pixel_format,
        }
    }

    pub(crate) fn color_at(&self, p: Point<i32>) -> Option<Color> {
        self.pixel_index(p).map(|pixel_index| {
            self.pixel_drawer
                .color_at(self.buffer.buffer(), pixel_index)
        })
    }

    pub(crate) fn copy<C>(
        &mut self,
        src_dst_offset: Offset<i32>,
        src: &BufferDrawer<C>,
        src_area: Rectangle<i32>,
    ) where
        C: Buffer,
    {
        assert_eq!(self.pixel_format, src.pixel_format);
        assert_eq!(self.bytes_per_pixel, src.bytes_per_pixel);

        (|| {
            // trim overflow area
            let src_area = (src_area & src.area())?;
            let dst_area = ((src_area + src_dst_offset) & self.area())?;
            let src_area = dst_area - src_dst_offset;
            assert_eq!(dst_area.size, src_area.size);

            let bytes_per_pixel = self.bytes_per_pixel;
            let bytes_per_copy_line = (bytes_per_pixel * dst_area.size.x) as usize;

            let dst_start_idx = self.pixel_index(dst_area.pos)?;
            let src_start_idx = src.pixel_index(src_area.pos)?;

            let dst_buf = &mut self.buffer.buffer_mut()[dst_start_idx..];
            let src_buf = &src.buffer.buffer()[src_start_idx..];

            for dy in 0..dst_area.size.y {
                let dst = &mut dst_buf[(bytes_per_pixel * dy * self.stride) as usize..]
                    [..bytes_per_copy_line];
                let src =
                    &src_buf[(bytes_per_pixel * dy * src.stride) as usize..][..bytes_per_copy_line];
                dst.copy_from_slice(src);
            }
            Some(())
        })();
    }

    fn pixel_index(&self, p: Point<i32>) -> Option<usize> {
        if !self.area().contains(&p) {
            return None;
        }
        usize::try_from((p.y * self.stride + p.x) * self.bytes_per_pixel).ok()
    }
}

pub(crate) trait PixelDraw {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color);
    fn color_at(&self, buffer: &[u8], pixel_index: usize) -> Color;
}

fn select_pixel_drawer(
    pixel_format: PixelFormat,
) -> Result<&'static (dyn PixelDraw + Send + Sync)> {
    match pixel_format {
        PixelFormat::RGB => Ok(&RGB_PIXEL_DRAWER as _),
        PixelFormat::BGR => Ok(&BGR_PIXEL_DRAWER as _),
        PixelFormat::U8 => Ok(&U8_PIXEL_DRAWER as _),
        _ => bail!(ErrorKind::UnsupportedPixelFormat(pixel_format)),
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

    fn color_at(&self, buffer: &[u8], pixel_index: usize) -> Color {
        Color::new(
            buffer[pixel_index],
            buffer[pixel_index + 1],
            buffer[pixel_index + 2],
        )
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

    fn color_at(&self, buffer: &[u8], pixel_index: usize) -> Color {
        Color::new(
            buffer[pixel_index + 2],
            buffer[pixel_index + 1],
            buffer[pixel_index],
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct U8PixelDrawer;
static U8_PIXEL_DRAWER: U8PixelDrawer = U8PixelDrawer;
impl PixelDraw for U8PixelDrawer {
    fn pixel_draw(&self, buffer: &mut [u8], pixel_index: usize, c: Color) {
        buffer[pixel_index] = c.to_grayscale();
    }

    fn color_at(&self, buffer: &[u8], pixel_index: usize) -> Color {
        Color::from_grayscale(buffer[pixel_index])
    }
}
