use crate::graphics::{Color, Draw, DrawError, DrawErrorExt, Point, Rectangle, Size};
use core::{convert::TryFrom, fmt};

const FONT_SIZE_I32: Size<i32> = Size::new(8, 16);
const FONT_SIZE: Size<usize> = Size::new(8, 16);
const FONT_RECT: Rectangle<usize> = Rectangle::new(Point::new(0, 0), FONT_SIZE);

include!(concat!(env!("OUT_DIR"), "/ascii_font.rs"));

type Font = [u8; 16];

fn get_ascii_font(ch: u8) -> Font {
    static_assertions::const_assert_eq!(ASCII_FONT.len(), 256);
    ASCII_FONT[usize::from(ch)]
}

pub(crate) fn draw_char<D>(
    drawer: &mut D,
    pos: Point<i32>,
    ch: char,
    color: Color,
    ignore_out_of_range: bool,
) -> Result<(), DrawError>
where
    D: Draw,
{
    let codepoint = u32::from(ch);
    let ch = u8::try_from(codepoint).unwrap_or(b'?');
    let font = get_ascii_font(ch);

    let draw_rect = Rectangle {
        pos,
        size: FONT_SIZE_I32,
    };

    for (draw_p, font_p) in draw_rect.points().zip(FONT_RECT.points()) {
        if ((font[font_p.y] << font_p.x) & 0x80) != 0 {
            drawer
                .draw(draw_p, color)
                .ignore_out_of_range(ignore_out_of_range)?;
        }
    }

    Ok(())
}

pub(crate) fn draw_string<D>(
    drawer: &mut D,
    pos: Point<i32>,
    s: &str,
    color: Color,
    ignore_out_of_range: bool,
) -> Result<(), DrawError>
where
    D: Draw,
{
    let mut pos = pos;
    for ch in s.chars() {
        draw_char(drawer, pos, ch, color, ignore_out_of_range)?;
        pos.x += FONT_SIZE_I32.x;
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct StringDrawer<'d, D> {
    drawer: &'d mut D,
    pos: Point<i32>,
    color: Color,
    ignore_out_of_range: bool,
}

impl<'d, D> StringDrawer<'d, D> {
    pub(crate) fn new(
        drawer: &'d mut D,
        start_pos: Point<i32>,
        color: Color,
        ignore_out_of_range: bool,
    ) -> Self {
        Self {
            drawer,
            pos: start_pos,
            color,
            ignore_out_of_range,
        }
    }
}

impl<'d, D> fmt::Write for StringDrawer<'d, D>
where
    D: Draw,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for ch in s.chars() {
            draw_char(
                self.drawer,
                self.pos,
                ch,
                self.color,
                self.ignore_out_of_range,
            )
            .map_err(|_| fmt::Error)?;
            self.pos.x += FONT_SIZE_I32.x;
        }
        Ok(())
    }
}
