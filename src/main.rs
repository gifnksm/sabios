#![no_std]
#![no_main]

use crate::framebuffer::Drawer;
use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::{mem, panic::PanicInfo};
use framebuffer::{Color, Point};

mod framebuffer;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    if let Some(framebuffer) = mem::replace(&mut boot_info.framebuffer, Optional::None).into() {
        let mut drawer = Drawer::new(framebuffer);
        for x in drawer.x_range() {
            for y in drawer.y_range() {
                drawer.draw(Point::new(x, y), Color::WHITE);
            }
        }
        for x in 0..200 {
            for y in 0..100 {
                drawer.draw(Point::new(x, y), Color::GREEN);
            }
        }
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
