use crate::{
    graphics::{font, frame_buffer, Color, Draw, FrameBufferDrawer, Point, Rectangle, ScreenInfo},
    serial_print,
};
use core::fmt;

pub(crate) fn with_console(f: impl FnOnce(&mut EmergencyConsole<'_>)) -> ! {
    let screen_info = ScreenInfo::get();
    let mut drawer = unsafe { frame_buffer::emergency_lock_drawer() };
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
    drawer: &'a mut FrameBufferDrawer,
}

impl fmt::Write for EmergencyConsole<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial_print!("{}", s);

        for ch in s.chars() {
            if ch != '\n' {
                self.drawer.fill_rect(
                    Rectangle::new(self.pos, font::FONT_PIXEL_SIZE),
                    Color::WHITE,
                );
                self.drawer.draw_char(self.pos, ch, Color::RED);
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
