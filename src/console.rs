#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

use crate::{
    font,
    graphics::{Color, Draw, Point, Rectangle, Size},
};
use core::{convert::TryFrom, fmt};

const ROWS: usize = 25;
const COLUMNS: usize = 80;
const ROWS_I32: i32 = ROWS as i32;
const COLUMNS_I32: i32 = COLUMNS as i32;

pub(crate) struct Console<'d, D> {
    drawer: &'d mut D,
    buffer: [[u8; COLUMNS]; ROWS],
    fg_color: Color,
    bg_color: Color,
    cursor: Point<usize>,
}

impl<'d, D> Console<'d, D> {
    pub(crate) fn new(drawer: &'d mut D, fg_color: Color, bg_color: Color) -> Self {
        Self {
            drawer,
            buffer: [[0; COLUMNS]; ROWS],
            fg_color,
            bg_color,
            cursor: Point::new(0, 0),
        }
    }

    fn cursor_draw_pos(&self) -> Point<i32> {
        let font_size = font::FONT_PIXEL_SIZE;
        #[allow(clippy::unwrap_used)]
        Point {
            x: i32::try_from(self.cursor.x).unwrap() * font_size.x,
            y: i32::try_from(self.cursor.y).unwrap() * font_size.y,
        }
    }
}

impl<'d, D> fmt::Write for Console<'d, D>
where
    D: Draw,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for ch in s.chars() {
            let byte = font::char_to_byte(ch);
            if byte == b'\n' {
                self.newline();
                continue;
            }

            if self.cursor.x < COLUMNS - 1 {
                font::draw_byte(self.drawer, self.cursor_draw_pos(), byte, self.fg_color);
                self.buffer[self.cursor.y][self.cursor.x] = byte;
                self.cursor.x += 1;
            }
        }
        Ok(())
    }
}

impl<'d, D> Console<'d, D>
where
    D: Draw,
{
    fn newline(&mut self) {
        // update cursor position
        self.cursor.x = 0;
        if self.cursor.y < ROWS - 1 {
            self.cursor.y += 1;
            return;
        }

        // update buffer
        for (src, dst) in (1..).zip(0..(ROWS - 1)) {
            self.buffer[dst] = self.buffer[src];
        }
        self.buffer[ROWS - 1].fill(0b0);

        // redraw whole console
        let font_size = font::FONT_PIXEL_SIZE;
        let draw_area = Rectangle {
            pos: Point::new(0, 0),
            size: Size::new(COLUMNS_I32 * font_size.x, ROWS_I32 * font_size.y),
        };
        self.drawer.fill_rect(draw_area, self.bg_color);
        for (y, line) in (0..).zip(self.buffer) {
            let font_size = font::FONT_PIXEL_SIZE;
            let p = Point::new(0, y * font_size.y);
            font::draw_byte_str(self.drawer, p, &line, self.fg_color);
        }
    }
}
