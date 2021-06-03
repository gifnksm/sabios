use crate::{
    font,
    framed_window::{FramedWindow, FramedWindowEvent},
    graphics::{self, Color, Draw, Point, Rectangle, Size},
    prelude::*,
    timer,
};
use alloc::string::String;
use futures_util::select_biased;

const BACKGROUND: Color = Color::BLACK;
const BORDER_DARK: Color = Color::from_code(0x848484);
const BORDER_LIGHT: Color = Color::from_code(0xc6c6c6);

#[derive(Debug)]
pub(crate) struct Terminal {
    text_size: Size<i32>,
    cursor: Point<i32>,
    cursor_visible: bool,
    window: FramedWindow,
}

impl Terminal {
    pub(crate) fn new(title: String, pos: Point<i32>, text_size: Size<i32>) -> Result<Self> {
        let font_size = font::FONT_PIXEL_SIZE;
        let window = FramedWindow::builder(title)
            .pos(pos)
            .size(Size::new(
                text_size.x * font_size.x,
                text_size.y * font_size.y,
            ))
            .build()?;
        Ok(Self {
            text_size,
            cursor: Point::new(0, 0),
            cursor_visible: false,
            window,
        })
    }

    fn draw_terminal(&mut self) {
        let area = self.window.area();
        graphics::draw_box(
            &mut self.window,
            area,
            BACKGROUND,
            BORDER_DARK,
            BORDER_LIGHT,
        )
    }

    fn insert_pos(&self) -> Point<i32> {
        let font_size = font::FONT_PIXEL_SIZE;
        Point::new(
            4 + font_size.x * self.cursor.x,
            6 + font_size.y * self.cursor.y,
        )
    }

    fn draw_cursor(&mut self, visible: bool) {
        let font_size = font::FONT_PIXEL_SIZE;
        let color = if visible { Color::WHITE } else { Color::BLACK };
        let pos = self.insert_pos();
        self.window
            .fill_rect(Rectangle::new(pos, font_size - Size::new(1, 1)), color);
    }

    fn handle_event(&mut self, event: FramedWindowEvent) {
        match event {
            FramedWindowEvent::Keyboard(_) => {}
        }
    }

    fn handle_timeout(&mut self) {
        self.cursor_visible = !self.cursor_visible;
        self.draw_cursor(self.cursor_visible);
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        self.draw_terminal();
        self.window.flush().await?;

        let mut interval = timer::lapic::interval(0, 50)?;
        loop {
            select_biased! {
                event = self.window.recv_event().fuse() => {
                    let event = match event {
                        Some(event) => event?,
                        None => return Ok(()),
                    };
                    self.handle_event(event);
                }
                timeout = interval.next().fuse() => {
                    let _timeout = match timeout {
                        Some(event) => event?,
                        _ => return Ok(()),
                    };
                    self.handle_timeout();
                }
            }
            self.window.flush().await?;
        }
    }
}
