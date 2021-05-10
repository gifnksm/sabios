use crate::{
    framebuffer,
    graphics::{Color, Draw, Point, Rectangle, Size},
    Result,
};

pub(crate) const BG_COLOR: Color = Color::new(45, 118, 237);
pub(crate) const FG_COLOR: Color = Color::WHITE;

pub(crate) fn draw() -> Result<()> {
    let screen = *framebuffer::info()?;
    let mut drawer = framebuffer::lock_drawer().expect("failed to get framebuffer");
    drawer.fill_rect(
        Rectangle::new(
            Point::new(0, 0),
            Size::new(screen.width, screen.height - 50),
        ),
        BG_COLOR,
    );
    drawer.fill_rect(
        Rectangle::new(
            Point::new(0, screen.height - 50),
            Size::new(screen.width, 50),
        ),
        Color::new(1, 8, 17),
    );
    drawer.fill_rect(
        Rectangle::new(
            Point::new(0, screen.height - 50),
            Size::new(screen.width / 5, 50),
        ),
        Color::new(80, 80, 80),
    );
    drawer.draw_rect(
        Rectangle::new(Point::new(10, screen.height - 40), Size::new(30, 30)),
        Color::new(160, 160, 160),
    );
    Ok(())
}
