use crate::{
    framebuffer,
    graphics::{Color, Draw, Point, Rectangle, Size},
    layer::{self, Layer, LayerDrawer},
    prelude::*,
    window::Window,
};

pub(crate) const BG_COLOR: Color = Color::new(45, 118, 237);
pub(crate) const FG_COLOR: Color = Color::WHITE;

fn draw(drawer: &mut dyn Draw, size: Size<i32>) {
    drawer.fill_rect(
        Rectangle::new(Point::new(0, 0), Size::new(size.x, size.y - 50)),
        BG_COLOR,
    );
    drawer.fill_rect(
        Rectangle::new(Point::new(0, size.y - 50), Size::new(size.x, 50)),
        Color::new(1, 8, 17),
    );
    drawer.fill_rect(
        Rectangle::new(Point::new(0, size.y - 50), Size::new(size.x / 5, 50)),
        Color::new(80, 80, 80),
    );
    drawer.draw_rect(
        Rectangle::new(Point::new(10, size.y - 40), Size::new(30, 30)),
        Color::new(160, 160, 160),
    );
}

pub(crate) async fn handler_task() {
    let res = async {
        let screen_info = *framebuffer::info();
        let mut window = Window::new(screen_info.size)?;

        draw(&mut window, screen_info.size);

        let mut layer = Layer::new();
        let layer_id = layer.id();
        layer.move_to(Point::new(0, 0));

        let tx = layer::event_tx();
        let mut drawer = LayerDrawer::new();
        tx.register(layer)?;
        tx.set_height(layer_id, layer::DESKTOP_HEIGHT)?;
        drawer.draw(layer_id, &window).await?;

        Ok::<(), Error>(())
    }
    .await;

    if let Err(err) = res {
        panic!("error occurred during handling desktop drawing: {}", err);
    }
}
