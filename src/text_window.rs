use crate::{
    font,
    framed_window::{FramedWindow, FramedWindowEvent},
    graphics::{self, Color, Draw, Point, Rectangle, Size},
    prelude::*,
    timer,
};
use alloc::string::String;
use futures_util::select_biased;

const BACKGROUND: Color = Color::WHITE;
const BORDER_DARK: Color = Color::from_code(0x848484);
const BORDER_LIGHT: Color = Color::from_code(0xc6c6c6);

#[derive(Debug)]
pub(crate) struct TextWindow {
    window: FramedWindow,
    index: i32,
    max_chars: i32,
    cursor_visible: bool,
}

impl TextWindow {
    pub(crate) fn new(title: String, pos: Point<i32>) -> Result<Self> {
        let font_size = font::FONT_PIXEL_SIZE;
        let window_size = Size::new(160, font_size.y + 8);
        let window = FramedWindow::builder(title)
            .size(window_size)
            .pos(pos)
            .build()?;
        Ok(Self {
            window,
            index: 0,
            max_chars: (window_size.x - 8) / font_size.x - 1,
            cursor_visible: true,
        })
    }

    fn insert_pos(&self) -> Point<i32> {
        let font_size = font::FONT_PIXEL_SIZE;
        Point::new(4 + font_size.x * self.index, 6)
    }

    fn draw_text_box(&mut self) {
        let area = self.window.area();
        graphics::draw_box(
            &mut self.window,
            area,
            BACKGROUND,
            BORDER_DARK,
            BORDER_LIGHT,
        );
    }

    fn draw_cursor(&mut self, visible: bool) {
        let font_size = font::FONT_PIXEL_SIZE;
        let color = if visible { Color::BLACK } else { Color::WHITE };
        let pos = self.insert_pos();
        self.window
            .fill_rect(Rectangle::new(pos, font_size - Size::new(1, 1)), color);
    }

    fn handle_event(&mut self, event: FramedWindowEvent) {
        match event {
            FramedWindowEvent::Keyboard(event) => {
                if event.ascii == '\0' {
                    return;
                }

                if event.ascii == '\x08' && self.index > 0 {
                    self.draw_cursor(false);
                    self.index -= 1;
                    self.window.fill_rect(
                        Rectangle::new(self.insert_pos(), Size::new(8, 16)),
                        Color::WHITE,
                    );
                    self.draw_cursor(self.cursor_visible);
                } else if event.ascii >= ' ' && self.index < self.max_chars {
                    self.draw_cursor(false);
                    let pos = self.insert_pos();
                    font::draw_char(&mut self.window, pos, event.ascii, Color::BLACK);
                    self.index += 1;
                    self.draw_cursor(self.cursor_visible);
                }
            }
        }
    }

    fn handle_timeout(&mut self) {
        self.cursor_visible = !self.cursor_visible;
        self.draw_cursor(self.cursor_visible);
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        self.draw_text_box();
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
