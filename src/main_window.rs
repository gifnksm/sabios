use crate::{
    co_task, font,
    graphics::{Color, Draw, Point, Rectangle, Size},
    layer,
    prelude::*,
    window::{self, Window},
};
use alloc::format;

pub(crate) async fn handler_task() {
    let res = async {
        let mut window = Window::builder()
            .size(Size::new(160, 52))
            .pos(Point::new(300, 100))
            .height(layer::MAIN_WINDOW_ID)
            .build()?;
        window::draw_window(&mut window, "Hello Window");
        window.flush()?;

        for count in 0.. {
            window.fill_rect(
                Rectangle::new(Point::new(24, 28), Size::new(8 * 10, 16)),
                Color::from_code(0xc6c6c6),
            );
            let s = format!("{:010}", count);
            font::draw_str(&mut window, Point::new(24, 28), &s, Color::BLACK);
            window.flush()?;
            co_task::yield_now().await;
        }

        Ok::<(), Error>(())
    }
    .await;

    if let Err(err) = res {
        panic!("error occurred during handling main window event: {}", err);
    }
}
