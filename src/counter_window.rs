use crate::{
    co_task, font,
    framed_window::{FramedWindow, FramedWindowEvent},
    graphics::{Color, Draw, Point, Rectangle, Size},
    prelude::*,
};
use alloc::{format, string::String};
use futures_util::select_biased;

#[derive(Debug)]
struct CounterWindow {
    window: FramedWindow,
    count: usize,
}

impl CounterWindow {
    async fn new(title: String, pos: Point<i32>) -> Result<Self> {
        let mut window = FramedWindow::builder(title)
            .pos(pos)
            .size(Size::new(152, 25))
            .build()?;
        window.flush().await?;
        Ok(Self { window, count: 0 })
    }

    fn handle_event(&mut self, event: FramedWindowEvent) {
        match event {
            FramedWindowEvent::Keyboard(_) => {}
        }
    }

    fn handle_yield(&mut self) {
        self.window.fill_rect(
            Rectangle::new(Point::new(20, 4), Size::new(8 * 10, 16)),
            Color::from_code(0xc6c6c6),
        );
        let s = format!("{:010}", self.count);
        font::draw_str(&mut self.window, Point::new(20, 4), &s, Color::BLACK);
        self.count += 1;
    }

    async fn run(&mut self) -> Result<()> {
        loop {
            select_biased! {
                event = self.window.recv_event().fuse() => {
                    let event = match event {
                        Some(event) => event?,
                        None => return Ok(()),
                    };
                    self.handle_event(event);
                }
                _ = co_task::yield_now().fuse() => self.handle_yield(),
            }
            self.window.flush().await?;
        }
    }
}

pub(crate) async fn handler_task(title: String, pos: Point<i32>) -> Result<()> {
    CounterWindow::new(title, pos).await?.run().await
}
