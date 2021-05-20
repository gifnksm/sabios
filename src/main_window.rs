use crate::{
    font,
    graphics::{Color, Draw, Point, Rectangle, Size},
    layer::{self, Layer},
    prelude::*,
    window::{self, Window},
};
use alloc::format;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

struct Yield {
    yielded: bool,
}

impl Yield {
    fn new() -> Self {
        Self { yielded: false }
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.yielded {
            cx.waker().wake_by_ref();
            self.yielded = true;
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub(crate) async fn handler_task() {
    let res = async {
        let window = Window::new(Size::new(160, 52));

        {
            let drawer = window.lock().drawer();
            let mut drawer = drawer.lock();
            window::draw_window(&mut *drawer, "Hello Window");
        }

        let mut layer = Layer::new();
        let layer_id = layer.id();
        layer.set_window(Some(window.clone()));
        layer.move_to(Point::new(300, 100));

        let tx = layer::event_tx();
        tx.register(layer)?;
        tx.set_height(layer_id, layer::MAIN_WINDOW_ID)?;
        tx.draw_layer(layer_id)?;

        for count in 0.. {
            {
                let drawer = window.lock().drawer();
                let mut drawer = drawer.lock();
                drawer.fill_rect(
                    Rectangle::new(Point::new(24, 28), Size::new(8 * 10, 16)),
                    Color::from_code(0xc6c6c6),
                );
                let s = format!("{:010}", count);
                font::draw_str(&mut *drawer, Point::new(24, 28), &s, Color::BLACK);
                tx.draw_layer(layer_id)?;
            }
            Yield::new().await;
        }

        Ok::<(), Error>(())
    }
    .await;

    if let Err(err) = res {
        panic!("error occurred during handling main window event: {}", err);
    }
}
