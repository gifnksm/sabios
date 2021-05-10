use crate::{
    framebuffer,
    graphics::{Color, Draw, Point},
};

const MOUSE_CURSOR_WIDTH: usize = 15;
const MOUSE_CURSOR_HEIGHT: usize = 24;

const MOUSE_CURSOR_SHAPE: [[u8; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] = [
    *b"@              ",
    *b"@@             ",
    *b"@.@            ",
    *b"@..@           ",
    *b"@...@          ",
    *b"@....@         ",
    *b"@.....@        ",
    *b"@......@       ",
    *b"@.......@      ",
    *b"@........@     ",
    *b"@.........@    ",
    *b"@..........@   ",
    *b"@...........@  ",
    *b"@............@ ",
    *b"@......@@@@@@@@",
    *b"@......@       ",
    *b"@....@@.@      ",
    *b"@...@ @.@      ",
    *b"@..@   @.@     ",
    *b"@.@    @.@     ",
    *b"@@      @.@    ",
    *b"@       @.@    ",
    *b"         @.@   ",
    *b"         @@@   ",
];

pub(crate) fn draw_cursor() -> Result<(), framebuffer::AccessError> {
    let mut drawer = framebuffer::lock_drawer()?;
    for (dy, row) in (0..).zip(MOUSE_CURSOR_SHAPE) {
        for (dx, ch) in (0..).zip(row) {
            let p = Point::new(200 + dx, 100 + dy);
            match ch {
                b'@' => drawer.draw(p, Color::BLACK),
                b'.' => drawer.draw(p, Color::WHITE),
                _ => {}
            }
        }
    }
    Ok(())
}
