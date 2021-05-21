use crate::{
    font,
    graphics::{Color, Draw, Point, Rectangle, Size},
    layer::{self, Layer},
    prelude::*,
    window::{self, Window},
};
use alloc::format;

pub(crate) async fn handler_task() {
    let res = async {
        let window = Window::new(Size::new(160, 52))?;

        window.with_lock(|window| {
            window::draw_window(window, "Hello Window");
        });

        let mut layer = Layer::new();
        let layer_id = layer.id();
        layer.set_draggable(true);
        layer.set_window(Some(window.clone()));
        layer.move_to(Point::new(300, 100));

        let tx = layer::event_tx();
        tx.register(layer)?;
        tx.set_height(layer_id, layer::MAIN_WINDOW_ID)?;
        tx.draw_layer(layer_id)?;

        for count in 0..1 {
            window.with_lock(|window| {
                window.fill_rect(
                    Rectangle::new(Point::new(24, 28), Size::new(8 * 10, 16)),
                    Color::from_code(0xc6c6c6),
                );
                let s = format!("{:010}", count);
                font::draw_str(window, Point::new(24, 28), &s, Color::BLACK);
            });
            tx.draw_layer(layer_id)?;
        }

        Ok::<(), Error>(())
    }
    .await;

    if let Err(err) = res {
        panic!("error occurred during handling main window event: {}", err);
    }
}
