use crate::{
    framed_window::{FramedWindow, FramedWindowEvent},
    graphics::{font, Color, Draw, Offset, Point, Rectangle, Size},
    prelude::*,
    timer,
};
use alloc::string::String;
use futures_util::select_biased;

const BACKGROUND: Color = Color::BLACK;
const BORDER_DARK: Color = Color::from_code(0x848484);
const BORDER_LIGHT: Color = Color::from_code(0xc6c6c6);

const PADDING_TOP: i32 = 4;
const PADDING_BOTTOM: i32 = 4;
const PADDING_LEFT: i32 = 4;
const PADDING_RIGHT: i32 = 4;
const PADDING_POS: Point<i32> = Point::new(PADDING_LEFT, PADDING_TOP);
const PADDING_SIZE: Size<i32> =
    Size::new(PADDING_LEFT + PADDING_RIGHT, PADDING_TOP + PADDING_BOTTOM);

#[derive(Debug)]
pub(crate) struct Terminal {
    text_size: Size<i32>,
    cursor: Point<i32>,
    cursor_visible: bool,
    line_buf: String,
    window: FramedWindow,
}

impl Terminal {
    pub(crate) fn new(title: String, pos: Point<i32>, text_size: Size<i32>) -> Result<Self> {
        let font_size = font::FONT_PIXEL_SIZE;
        let window = FramedWindow::builder(title)
            .pos(pos)
            .size(text_size * font_size + PADDING_SIZE)
            .build()?;
        Ok(Self {
            text_size,
            cursor: Point::new(0, 0),
            cursor_visible: false,
            line_buf: String::new(),
            window,
        })
    }

    fn draw_terminal(&mut self) {
        let area = self.window.area();
        self.window
            .draw_box(area, BACKGROUND, BORDER_DARK, BORDER_LIGHT)
    }

    fn insert_pos(&self) -> Point<i32> {
        let font_size = font::FONT_PIXEL_SIZE;
        font_size * self.cursor + PADDING_POS
    }

    fn draw_cursor(&mut self, visible: bool) {
        let font_size = font::FONT_PIXEL_SIZE;
        let color = if visible { Color::WHITE } else { Color::BLACK };
        let pos = self.insert_pos();
        self.window
            .fill_rect(Rectangle::new(pos, font_size - Size::new(1, 1)), color);
    }

    fn scroll1(&mut self) {
        let font_size = font::FONT_PIXEL_SIZE;
        self.window.move_area(
            Offset::new(0, -1) * font_size,
            Rectangle::new(
                Point::new(0, 1) * font_size + PADDING_POS,
                (self.text_size - Size::new(0, 1)) * font_size,
            ),
        );
        self.window.fill_rect(
            Rectangle::new(
                Offset::new(0, self.text_size.y - 1) * font_size + PADDING_POS,
                Size::new(self.text_size.x, 1) * font_size,
            ),
            Color::BLACK,
        );
    }

    fn handle_event(&mut self, event: FramedWindowEvent) {
        let font_size = font::FONT_PIXEL_SIZE;
        match event {
            FramedWindowEvent::Keyboard(event) => {
                self.draw_cursor(false);
                match event.ascii {
                    '\0' => {}
                    '\n' => {
                        warn!("line: {}", self.line_buf);
                        self.line_buf.clear();
                        if self.cursor.y + 1 >= self.text_size.y {
                            self.scroll1();
                        } else {
                            self.cursor.y += 1;
                        }
                        self.cursor.x = 0;
                    }
                    '\x08' => {
                        if self.line_buf.pop().is_some() {
                            if self.cursor.x == 0 {
                                self.cursor.x = self.text_size.x - 1;
                                self.cursor.y -= 1;
                            } else {
                                self.cursor.x -= 1;
                            }
                            self.window.fill_rect(
                                Rectangle::new(self.insert_pos(), font_size),
                                Color::BLACK,
                            );
                        }
                    }
                    ch => {
                        self.line_buf.push(ch);
                        self.window.draw_char(self.insert_pos(), ch, Color::WHITE);
                        if self.cursor.x + 1 >= self.text_size.x {
                            if self.cursor.y + 1 >= self.text_size.y {
                                self.scroll1();
                            } else {
                                self.cursor.y += 1;
                            }
                            self.cursor.x = 0;
                        } else {
                            self.cursor.x += 1;
                        }
                    }
                }
                self.draw_cursor(true);
            }
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
