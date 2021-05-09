#![warn(unsafe_op_in_unsafe_fn)]
#![no_std]
#![no_main]

use self::{
    console::Console,
    graphics::{Color, Draw, Point, Rectangle, Size},
};
use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::{fmt::Write, mem};

mod console;
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
        let screen_rect = drawer.area();
        drawer.fill_rect(screen_rect, Color::WHITE);
        let green_rect = Rectangle::new(Point::new(0, 0), Size::new(200, 100));
        drawer.fill_rect(green_rect, Color::GREEN);

        let mut console = Console::new(&mut *drawer, Color::BLACK, Color::WHITE);
        for i in 0..80 {
            writeln!(&mut console, "line {}", i).expect("failed to draw");
        }
    }

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
