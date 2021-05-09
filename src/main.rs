#![warn(unsafe_op_in_unsafe_fn)]
#![no_std]
#![no_main]

use crate::graphics::{Color, Point};
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
        for x in drawer.x_range() {
            for y in drawer.y_range() {
                drawer
                    .draw(Point::new(x, y), Color::WHITE)
                    .expect("failed to draw");
            }
        }
        for x in drawer.x_range() {
            for y in drawer.y_range() {
                drawer
                    .draw(Point::new(x, y), Color::BLACK)
                    .expect("failed to draw");
            }
        }
        for x in 0..200 {
            for y in 0..100 {
                drawer
                    .draw(Point::new(x, y), Color::GREEN)
                    .expect("failed to draw");
            }
        }
    }

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
