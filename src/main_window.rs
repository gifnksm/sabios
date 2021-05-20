use crate::{
    font,
    graphics::{Color, Point, Size},
    layer::{self, Layer},
    prelude::*,
    window::{self, Window},
};

pub(crate) async fn handler_task() {
    let res = async {
        let window = Window::new(Size::new(160, 68));

        {
            let drawer = window.lock().drawer();
            let mut drawer = drawer.lock();
            window::draw_window(&mut *drawer, "Hello Window");
            font::draw_str(&mut *drawer, Point::new(24, 28), "Welcome to", Color::BLACK);
            font::draw_str(
                &mut *drawer,
                Point::new(24, 44),
                " sabios world!",
                Color::BLACK,
            );
        }

        let mut layer = Layer::new();
        let layer_id = layer.id();
        layer.set_window(Some(window));
        layer.move_to(Point::new(300, 100));

        let tx = layer::event_tx();
        tx.register(layer)?;
        tx.set_height(layer_id, layer::MAIN_WINDOW_ID)?;
        tx.draw()?;

        Ok::<(), Error>(())
    }
    .await;

    if let Err(err) = res {
        panic!("error occurred during handling main window event: {}", err);
    }
}
