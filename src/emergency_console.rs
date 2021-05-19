use crate::{
    font,
    framebuffer::{self, Drawer, ScreenInfo},
    graphics::{Color, Draw, Point, Rectangle},
};
use core::fmt;

pub(crate) fn with_console(f: impl FnOnce(&mut EmergencyConsole<'_>)) -> ! {
    let screen_info = *framebuffer::info();
    let mut drawer = unsafe { framebuffer::emergency_lock_drawer() };
    let mut console = EmergencyConsole {
        screen_info,
        pos: Point::new(0, 0),
        drawer: &mut *drawer,
    };

    f(&mut console);

    crate::hlt_loop();
}

pub(crate) struct EmergencyConsole<'a> {
    screen_info: ScreenInfo,
    pos: Point<i32>,
    drawer: &'a mut Drawer,
}

impl fmt::Write for EmergencyConsole<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for ch in s.chars() {
            if ch != '\n' {
                self.drawer.fill_rect(
                    Rectangle::new(self.pos, font::FONT_PIXEL_SIZE),
                    Color::WHITE,
                );
                font::draw_char(self.drawer, self.pos, ch, Color::RED);
                self.pos.x += font::FONT_PIXEL_SIZE.x;
            }

            if ch == '\n' || self.pos.x + font::FONT_PIXEL_SIZE.x > self.screen_info.size.x {
                self.pos.y += font::FONT_PIXEL_SIZE.y;
                self.pos.x = 0;
            }
        }
        Ok(())
    }
}
