use crate::graphics::{Color, Draw, DrawError, DrawErrorExt, Point, Rectangle, Size};

const FONT_SIZE_I32: Size<i32> = Size::new(8, 16);
const FONT_SIZE: Size<usize> = Size::new(8, 16);
const FONT_RECT: Rectangle<usize> = Rectangle::new(Point::new(0, 0), FONT_SIZE);

const FONT_A: [u8; 16] = [
    0b00000000, //
    0b00011000, //    **
    0b00011000, //    **
    0b00011000, //    **
    0b00011000, //    **
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b01111110, //  ******
    0b01000010, //  *    *
    0b01000010, //  *    *
    0b01000010, //  *    *
    0b11100111, // ***  ***
    0b00000000, //
    0b00000000, //
];

pub(crate) fn draw_ascii<D>(
    drawer: &mut D,
    pos: Point<i32>,
    character: u8,
    color: Color,
    ignore_out_of_range: bool,
) -> Result<(), DrawError>
where
    D: Draw,
{
    if character != b'A' {
        return Ok(());
    }

    let draw_rect = Rectangle {
        pos,
        size: FONT_SIZE_I32,
    };

    for (draw_p, font_p) in draw_rect.points().zip(FONT_RECT.points()) {
        if ((FONT_A[font_p.y] << font_p.x) & 0x80) != 0 {
            drawer
                .draw(draw_p, color)
                .ignore_out_of_range(ignore_out_of_range)?;
        }
    }

    Ok(())
}
