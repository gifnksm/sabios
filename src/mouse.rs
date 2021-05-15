use crate::{
    desktop, framebuffer,
    graphics::{Color, Draw, Point, Vector2d},
    prelude::*,
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

static MOUSE_CURSOR: spin::Mutex<MouseCursor> = spin::Mutex::new(MouseCursor {
    erase_color: desktop::BG_COLOR,
    position: Vector2d::new(300, 200),
});

struct MouseCursor {
    erase_color: Color,
    position: Point<i32>,
}

impl MouseCursor {
    fn move_relative(&mut self, displacement: Vector2d<i32>) -> Result<()> {
        self.erase()?;
        self.position += displacement;
        self.draw()?;
        Ok(())
    }

    fn erase(&mut self) -> Result<()> {
        let mut drawer = framebuffer::lock_drawer()?;
        for (dy, row) in (0..).zip(MOUSE_CURSOR_SHAPE) {
            for (dx, ch) in (0..).zip(row) {
                let p = self.position + Vector2d::new(dx, dy);
                if ch != b' ' {
                    drawer.draw(p, self.erase_color);
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<()> {
        let mut drawer = framebuffer::lock_drawer()?;
        for (dy, row) in (0..).zip(MOUSE_CURSOR_SHAPE) {
            for (dx, ch) in (0..).zip(row) {
                let p = self.position + Vector2d::new(dx, dy);
                match ch {
                    b'@' => drawer.draw(p, Color::BLACK),
                    b'.' => drawer.draw(p, Color::WHITE),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

#[track_caller]
fn lock() -> Result<spin::MutexGuard<'static, MouseCursor>> {
    MOUSE_CURSOR
        .try_lock()
        .ok_or_else(|| make_error!(ErrorKind::WouldBlock("MOUSE_CURSOR")))
}

pub(crate) fn init() -> Result<()> {
    lock()?.draw()?;
    Ok(())
}

pub(crate) extern "C" fn observer(displacement_x: i8, displacement_y: i8) {
    let displacement = Vector2d::new(i32::from(displacement_x), i32::from(displacement_y));
    #[allow(clippy::expect_used)]
    let mut mouse_cursor = lock().expect("failed to lock MOUSE_CURSOR");
    #[allow(clippy::expect_used)]
    mouse_cursor
        .move_relative(displacement)
        .expect("failed to move mouse cursor");
}
