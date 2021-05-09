#![no_std]
#![no_main]

use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::mem;
use graphics::Color;

mod framebuffer;
mod graphics;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .expect("framebuffer not supported");
    framebuffer::init(framebuffer);

    for _ in 0..10 {
        let mut drawer = framebuffer::lock_drawer();
        for x in drawer.x_range() {
            for y in drawer.y_range() {
                drawer.draw((x, y), Color::WHITE);
            }
        }
        for x in drawer.x_range() {
            for y in drawer.y_range() {
                drawer.draw((x, y), Color::BLACK);
            }
        }
        for x in 0..200 {
            for y in 0..100 {
                drawer.draw((x, y), Color::GREEN);
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
