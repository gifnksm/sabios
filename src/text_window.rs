use crate::{
    font,
    framed_window::{FramedWindow, FramedWindowEvent},
    graphics::{Color, Draw, Offset, Point, Rectangle, Size},
    prelude::*,
    timer,
};
use alloc::string::String;
use futures_util::select_biased;

#[derive(Debug)]
struct TextWindow {
    window: FramedWindow,
    index: i32,
    max_chars: i32,
    cursor_visible: bool,
}

impl TextWindow {
    async fn new(title: String, pos: Point<i32>) -> Result<Self> {
        let window_size = Size::new(160, 24);
        let mut window = FramedWindow::builder(title)
            .size(window_size)
            .pos(pos)
            .build()?;
        draw_text_box(
            &mut window,
            Rectangle::new(Point::new(0, 0), Size::new(window_size.x, window_size.y)),
        );
        window.flush().await?;

        Ok(Self {
            window,
            index: 0,
            max_chars: (window_size.x - 8) / 8 - 1,
            cursor_visible: true,
        })
    }

    fn insert_pos(&self) -> Point<i32> {
        Point::new(4 + 8 * self.index, 6)
    }

    fn draw_cursor(&mut self, visible: bool) {
        let color = if visible { Color::BLACK } else { Color::WHITE };
        let pos = self.insert_pos() - Offset::new(0, 1);
        self.window
            .fill_rect(Rectangle::new(pos, Size::new(7, 15)), color);
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

    async fn run(&mut self) -> Result<()> {
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

pub(crate) async fn handler_task(title: String, pos: Point<i32>) -> Result<()> {
    TextWindow::new(title, pos).await?.run().await
}

const EDGE_DARK: Color = Color::from_code(0x848484);
const EDGE_LIGHT: Color = Color::from_code(0xc6c6c6);

fn draw_text_box<D>(drawer: &mut D, area: Rectangle<i32>)
where
    D: Draw,
{
    // fill main box
    drawer.fill_rect(
        Rectangle::new(area.pos + Offset::new(1, 1), area.size - Offset::new(2, 2)),
        Color::WHITE,
    );

    // draw border lines
    drawer.fill_rect(
        Rectangle::new(area.pos, Size::new(area.size.x, 1)),
        EDGE_DARK,
    );
    drawer.fill_rect(
        Rectangle::new(area.pos, Size::new(1, area.size.y)),
        EDGE_DARK,
    );
    drawer.fill_rect(
        Rectangle::new(
            area.pos + Offset::new(0, area.size.y),
            Size::new(area.size.x, 1),
        ),
        EDGE_LIGHT,
    );
    drawer.fill_rect(
        Rectangle::new(
            area.pos + Offset::new(area.size.x, 0),
            Size::new(1, area.size.y),
        ),
        EDGE_LIGHT,
    );
}
