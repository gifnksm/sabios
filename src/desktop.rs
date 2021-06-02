use crate::{
    framebuffer,
    graphics::{Color, Draw, Point, Rectangle, Size},
    layer,
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

pub(crate) async fn handler_task() -> Result<()> {
    let screen_info = *framebuffer::info();
    let mut window = Window::builder()
        .size(screen_info.size)
        .height(layer::DESKTOP_HEIGHT)
        .build()?;

    draw(&mut window, screen_info.size);
    window.flush().await?;

    Ok(())
}
