use crate::{
    co_task, font,
    framed_window::FramedWindow,
    graphics::{Color, Draw, Point, Rectangle, Size},
    prelude::*,
};
use alloc::{format, string::String};
use futures_util::{select_biased, FutureExt};

pub(crate) async fn handler_task(title: String, pos: Point<i32>) {
    let res = async {
        let mut window = FramedWindow::builder(title)
            .pos(pos)
            .size(Size::new(152, 24))
            .build()?;
        window.flush().await?;

        let mut count = 0;
        'outer: loop {
            select_biased! {
                event = window.recv_event().fuse() => {
                    let event = match event {
                        Some(Ok(event)) => event,
                        Some(Err(err)) => bail!(err),
                        None => break 'outer,
                    };
                    match event {}
                }
                _ = co_task::yield_now().fuse() => {
                    window.fill_rect(
                        Rectangle::new(Point::new(20, 4), Size::new(8 * 10, 16)),
                        Color::from_code(0xc6c6c6),
                    );
                    let s = format!("{:010}", count);
                    font::draw_str(&mut window, Point::new(20, 4), &s, Color::BLACK);
                    count += 1;
                }
            }
            window.flush().await?;
        }

        Ok::<(), Error>(())
    }
    .await;

    if let Err(err) = res {
        panic!(
            "error occurred during handling counter window event: {}",
            err
        );
    }
}
