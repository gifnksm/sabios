use crate::{
    font,
    graphics::{Color, Draw, Point, Rectangle, Size},
    prelude::*,
    window::{self, Window},
};
use alloc::string::String;

const PADDING_TOP: i32 = 24;
const PADDING_BOTTOM: i32 = 4;
const PADDING_LEFT: i32 = 4;
const PADDING_RIGHT: i32 = 4;
const PADDING_POS: Point<i32> = Point::new(PADDING_LEFT, PADDING_TOP);
const PADDING_SIZE: Size<i32> =
    Size::new(PADDING_LEFT + PADDING_RIGHT, PADDING_TOP + PADDING_BOTTOM);

#[derive(Debug, Clone)]
pub(crate) struct Builder {
    title: String,
    inner: window::Builder,
}

impl Builder {
    pub(crate) fn new(title: String) -> Self {
        let mut inner = window::Builder::new();
        inner.draggable(true);
        inner.height(usize::MAX);
        Self { title, inner }
    }

    pub(crate) fn pos(mut self, pos: Point<i32>) -> Self {
        self.inner.pos(pos);
        self
    }

    pub(crate) fn size(mut self, size: Size<i32>) -> Self {
        self.inner.size(size + PADDING_SIZE);
        self
    }

    pub(crate) fn build(mut self) -> Result<FramedWindow> {
        let window = self.inner.build()?;
        let mut window = FramedWindow {
            title: self.title,
            window,
        };
        window.draw_frame();
        Ok(window)
    }
}

#[derive(Debug)]
pub(crate) struct FramedWindow {
    title: String,
    window: Window,
}

impl Draw for FramedWindow {
    fn size(&self) -> Size<i32> {
        self.window.size() - PADDING_SIZE
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        if self.area().contains(&p) {
            self.window.draw(p + PADDING_POS, c);
        }
    }

    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>) {
        if offset.x == 0 && offset.y == 0 {
            return;
        }

        (|| {
            let dst = (((src & self.area())? + offset) & self.area())?;
            let src = dst - offset;

            self.window.move_area(
                offset + PADDING_POS,
                Rectangle::new(src.pos + PADDING_POS, src.size),
            );

            Some(())
        })();
    }
}

const CLOSE_BUTTON_WIDTH: usize = 16;
const CLOSE_BUTTON_HEIGHT: usize = 14;
const CLOSE_BUTTON: [[u8; CLOSE_BUTTON_WIDTH]; CLOSE_BUTTON_HEIGHT] = [
    *b"...............@",
    *b".:::::::::::::$@",
    *b".:::::::::::::$@",
    *b".:::@@::::@@::$@",
    *b".::::@@::@@:::$@",
    *b".:::::@@@@::::$@",
    *b".::::::@@:::::$@",
    *b".:::::@@@@::::$@",
    *b".::::@@::@@:::$@",
    *b".:::@@::::@@::$@",
    *b".:::::::::::::$@",
    *b".:::::::::::::$@",
    *b".$$$$$$$$$$$$$$@",
    *b"@@@@@@@@@@@@@@@@",
];

const EDGE_DARK: Color = Color::from_code(0x848484);
const EDGE_LIGHT: Color = Color::from_code(0xc6c6c6);
const BACKGROUND: Color = Color::from_code(0x000084);

impl FramedWindow {
    pub(crate) fn builder(title: String) -> Builder {
        Builder::new(title)
    }

    pub(crate) async fn flush(&mut self) -> Result<()> {
        self.window.flush().await
    }

    fn draw_frame(&mut self) {
        let win_size = self.window.size();
        let (wx, wy) = (win_size.x, win_size.y);

        let data = &[
            ((0, 0), (wx, 1), EDGE_LIGHT),
            ((1, 1), (wx - 2, 1), Color::WHITE),
            ((0, 0), (1, wy), EDGE_LIGHT),
            ((1, 1), (1, wy - 2), Color::WHITE),
            ((wx - 2, 1), (1, wy - 2), EDGE_DARK),
            ((wx - 1, 0), (1, wy), Color::BLACK),
            ((2, 2), (wx - 4, wy - 4), EDGE_LIGHT),
            ((3, 3), (wx - 6, 18), BACKGROUND),
            ((1, wy - 2), (wx - 2, 1), EDGE_DARK),
            ((0, wy - 1), (wx, 1), Color::BLACK),
        ];

        for (pos, size, color) in data {
            self.window.fill_rect(
                Rectangle::new(Point::new(pos.0, pos.1), Size::new(size.0, size.1)),
                *color,
            );
        }

        font::draw_str(
            &mut self.window,
            Point::new(24, 4),
            &self.title,
            Color::WHITE,
        );

        for (y, row) in (0..).zip(CLOSE_BUTTON) {
            for (x, ch) in (0..).zip(row) {
                let c = match ch {
                    b'@' => Color::BLACK,
                    b'$' => EDGE_DARK,
                    b':' => EDGE_LIGHT,
                    b'.' => Color::WHITE,
                    _ => panic!("invalid char: {}", ch),
                };
                self.window
                    .draw(Point::new(wx - 5 - CLOSE_BUTTON_WIDTH as i32 + x, 5 + y), c);
            }
        }
    }
}
