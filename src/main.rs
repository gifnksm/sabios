#![no_std]
#![no_main]

use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::{mem, panic::PanicInfo};
use framebuffer::{Color, Point};

mod framebuffer;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .expect("framebuffer not supported");
    framebuffer::init(framebuffer);

    {
        let mut drawer = framebuffer::lock_drawer();
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
