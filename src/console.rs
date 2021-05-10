use crate::{
    desktop, font, framebuffer,
    graphics::{Color, Draw, Point, Rectangle, Size},
};
use core::{convert::TryFrom, fmt};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write as _;

    if let Ok(Some(mut framebuffer)) = framebuffer::try_lock_drawer() {
        if let Some(mut console) = CONSOLE.try_lock() {
            #[allow(clippy::unwrap_used)]
            console.writer(&mut *framebuffer).write_fmt(args).unwrap();
        }
    }
}

const ROWS: usize = 25;
const COLUMNS: usize = 80;

static CONSOLE: spin::Mutex<Console> = spin::Mutex::new(Console {
    buffer: [[0; COLUMNS]; ROWS],
    fg_color: desktop::FG_COLOR,
    bg_color: desktop::BG_COLOR,
    cursor: Point::new(0, 0),
});

pub(crate) struct Console {
    buffer: [[u8; COLUMNS]; ROWS],
    fg_color: Color,
    bg_color: Color,
    cursor: Point<usize>,
}

struct RedrawArea {
    area: Rectangle<usize>,
    fill_bg: bool,
}

impl RedrawArea {
    fn new() -> Self {
        Self {
            area: Rectangle {
                pos: Point::new(0, 0),
                size: Size::new(0, 0),
            },
            fill_bg: false,
        }
    }

    fn add(&mut self, p: Point<usize>) {
        if self.area.size.x == 0 || self.area.size.y == 0 {
            self.area.pos = p;
            self.area.size = Size::new(1, 1);
            return;
        }
        self.area = self.area.extend_to_contain(p);
    }
}

impl Console {
    fn write_str(&mut self, s: &str) -> RedrawArea {
        let mut redraw = RedrawArea::new();
        for ch in s.chars() {
            let byte = font::char_to_byte(ch);
            if byte == b'\n' {
                self.newline(&mut redraw);
                continue;
            }

            if self.cursor.x >= COLUMNS - 1 {
                self.newline(&mut redraw);
            }
            redraw.add(self.cursor);
            self.buffer[self.cursor.y][self.cursor.x] = byte;
            self.cursor.x += 1;
        }
        redraw
    }

    fn newline(&mut self, redraw: &mut RedrawArea) {
        self.cursor.x = 0;
        if self.cursor.y < ROWS - 1 {
            self.cursor.y += 1;
            return;
        }

        for (src, dst) in (1..).zip(0..(ROWS - 1)) {
            self.buffer[dst] = self.buffer[src];
        }
        self.buffer[ROWS - 1].fill(0b0);

        // redraw whole console
        redraw.fill_bg = true;
        redraw.area = Rectangle {
            pos: Point::new(0, 0),
            size: Size::new(COLUMNS, ROWS),
        };
    }

    pub(crate) fn writer<'d, 'c, D>(&'c mut self, drawer: &'d mut D) -> ConsoleWriter<'d, 'c, D> {
        ConsoleWriter {
            drawer,
            console: self,
        }
    }
}

pub(crate) struct ConsoleWriter<'d, 'c, D> {
    drawer: &'d mut D,
    console: &'c mut Console,
}

impl<'d, 'c, D> ConsoleWriter<'d, 'c, D>
where
    D: Draw,
{
    fn to_draw_point(&self, p: Point<usize>) -> Point<i32> {
        let font_size = font::FONT_PIXEL_SIZE;
        #[allow(clippy::unwrap_used)]
        Point {
            x: i32::try_from(p.x).unwrap() * font_size.x,
            y: i32::try_from(p.y).unwrap() * font_size.y,
        }
    }

    fn to_draw_rect(&self, rect: Rectangle<usize>) -> Rectangle<i32> {
        Rectangle {
            pos: self.to_draw_point(rect.pos),
            size: self.to_draw_point(rect.size),
        }
    }
}

impl<'d, 'c, D> fmt::Write for ConsoleWriter<'d, 'c, D>
where
    D: Draw,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let redraw = self.console.write_str(s);
        if redraw.fill_bg {
            let rect = self.to_draw_rect(redraw.area);
            self.drawer.fill_rect(rect, self.console.bg_color);
        }

        for console_y in redraw.area.y_range() {
            let x_range = redraw.area.x_range();
            let console_p = Point::new(redraw.area.x_start(), console_y);

            let bytes = &self.console.buffer[console_y][x_range];
            let draw_p = self.to_draw_point(console_p);
            font::draw_byte_str(self.drawer, draw_p, bytes, self.console.fg_color);
        }

        Ok(())
    }
}
