use crate::{
    co_task, font,
    graphics::{Color, Draw, Point, Rectangle, Size},
    layer::{self, Layer, LayerDrawer},
    prelude::*,
    window::{self, Window},
};
use alloc::format;

pub(crate) async fn handler_task() {
    let res = async {
        let mut window = Window::new(Size::new(160, 52))?;

        window::draw_window(&mut window, "Hello Window");

        let mut layer = Layer::new();
        let layer_id = layer.id();
        layer.set_draggable(true);
        layer.move_to(Point::new(300, 100));

        let tx = layer::event_tx();
        let mut drawer = LayerDrawer::new();
        tx.register(layer)?;
        tx.set_height(layer_id, layer::MAIN_WINDOW_ID)?;
        drawer.draw(layer_id, &window).await?;

        for count in 0.. {
            window.fill_rect(
                Rectangle::new(Point::new(24, 28), Size::new(8 * 10, 16)),
                Color::from_code(0xc6c6c6),
            );
            let s = format!("{:010}", count);
            font::draw_str(&mut window, Point::new(24, 28), &s, Color::BLACK);
            drawer.draw(layer_id, &window).await?;
            co_task::yield_now().await;
        }

        Ok::<(), Error>(())
    }
    .await;

    if let Err(err) = res {
        panic!("error occurred during handling main window event: {}", err);
    }
}
