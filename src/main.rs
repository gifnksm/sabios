#![warn(unsafe_op_in_unsafe_fn)]
#![no_std]
#![no_main]

use crate::graphics::{Color, Draw, Point, Rectangle, Size};
use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::mem;

mod framebuffer;
mod graphics;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .expect("framebuffer not supported");
    framebuffer::init(framebuffer).expect("failed to initialize framebuffer");

    for _ in 0..10 {
        let mut drawer = framebuffer::lock_drawer().expect("failed to get framebuffer");
        for p in drawer.area().points() {
            drawer.draw(p, Color::WHITE).expect("failed to draw");
        }
        for p in drawer.area().points() {
            drawer.draw(p, Color::BLACK).expect("failed to draw");
        }
        let rect = Rectangle::new(Point::new(0, 0), Size::new(200, 100));
        for p in rect.points() {
            drawer.draw(p, Color::GREEN).expect("failed to draw");
        }
    }

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
