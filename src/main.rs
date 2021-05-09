#![warn(unsafe_op_in_unsafe_fn)]
#![no_std]
#![no_main]

use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::{fmt::Write, mem};
use font::StringDrawer;
use graphics::{Color, Draw, Point, Rectangle, Size};

mod font;
mod framebuffer;
mod graphics;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .expect("framebuffer not supported");
    framebuffer::init(framebuffer).expect("failed to initialize framebuffer");

    {
        let mut drawer = framebuffer::lock_drawer().expect("failed to get framebuffer");
        for p in drawer.area().points() {
            drawer.draw(p, Color::WHITE).expect("failed to draw");
        }
        let rect = Rectangle::new(Point::new(0, 0), Size::new(200, 100));
        for p in rect.points() {
            drawer.draw(p, Color::GREEN).expect("failed to draw");
        }
        for (i, ch) in (0..).zip(('!'..='~').chain('あ'..'お')) {
            font::draw_char(&mut *drawer, Point::new(8 * i, 50), ch, Color::BLACK, true)
                .expect("failed to draw_ascii");
        }
        font::draw_string(
            &mut *drawer,
            Point::new(0, 66),
            "Hello, world!",
            Color::BLACK,
            false,
        )
        .expect("failed to draw");
        let mut string_drawer =
            StringDrawer::new(&mut *drawer, Point::new(0, 82), Color::BLACK, false);
        write!(string_drawer, "1 + 2 = {}", 1 + 2).expect("failed to draw");
    }

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
