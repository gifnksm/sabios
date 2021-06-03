use crate::graphics::{Color, Draw, Point, Rectangle, Size};
use core::convert::TryFrom;

pub(crate) const FONT_PIXEL_SIZE: Size<i32> = Size::new(8, 16);

include!(concat!(env!("OUT_DIR"), "/ascii_font.rs"));

type Font = [u8; 16];

fn get_ascii_font(ch: u8) -> &'static Font {
    static_assertions::const_assert_eq!(ASCII_FONT.len(), 256);
    &ASCII_FONT[usize::from(ch)]
}

pub(super) fn draw_byte_char<D>(drawer: &mut D, pos: Point<i32>, byte: u8, color: Color)
where
    D: Draw,
{
    let font = get_ascii_font(byte);
    let draw_rect = Rectangle {
        pos,
        size: FONT_PIXEL_SIZE,
    };

    for (font_y, draw_y) in draw_rect.y_range().enumerate() {
        for (font_x, draw_x) in draw_rect.x_range().enumerate() {
            if (font[font_y] << font_x) & 0x80 != 0 {
                drawer.draw(Point::new(draw_x, draw_y), color);
            }
        }
    }
}

pub(super) fn draw_byte_str<D>(drawer: &mut D, pos: Point<i32>, bytes: &[u8], color: Color)
where
    D: Draw,
{
    let mut pos = pos;
    for byte in bytes {
        draw_byte_char(drawer, pos, *byte, color);
        pos.x += FONT_PIXEL_SIZE.x;
    }
}

pub(crate) fn char_to_byte(ch: char) -> u8 {
    let codepoint = u32::from(ch);
    u8::try_from(codepoint).unwrap_or(b'?')
}

pub(super) fn draw_char<D>(drawer: &mut D, pos: Point<i32>, ch: char, color: Color)
where
    D: Draw,
{
    let byte = char_to_byte(ch);
    draw_byte_char(drawer, pos, byte, color)
}

pub(super) fn draw_str<D>(drawer: &mut D, pos: Point<i32>, s: &str, color: Color)
where
    D: Draw,
{
    let mut pos = pos;
    for ch in s.chars() {
        draw_char(drawer, pos, ch, color);
        pos.x += FONT_PIXEL_SIZE.x;
    }
}
