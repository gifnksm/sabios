use crate::{
    co_task, font,
    framed_window::FramedWindow,
    graphics::{Color, Draw, Point, Rectangle, Size},
    prelude::*,
};
use alloc::format;

pub(crate) async fn handler_task() {
    let res = async {
        let mut window = FramedWindow::builder("Hello Window".into())
            .pos(Point::new(300, 100))
            .size(Size::new(150, 24))
            .build()?;
        window.flush().await?;

        for count in 0.. {
            window.fill_rect(
                Rectangle::new(Point::new(20, 4), Size::new(8 * 10, 16)),
                Color::from_code(0xc6c6c6),
            );
            let s = format!("{:010}", count);
            font::draw_str(&mut window, Point::new(20, 4), &s, Color::BLACK);
            window.flush().await?;
            co_task::yield_now().await;
        }

        Ok::<(), Error>(())
    }
    .await;

    if let Err(err) = res {
        panic!("error occurred during handling main window event: {}", err);
    }
}
