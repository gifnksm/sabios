#![warn(unsafe_op_in_unsafe_fn)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![no_std]
#![no_main]

use self::prelude::*;
use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::mem;

mod console;
mod desktop;
mod error;
mod font;
mod framebuffer;
mod graphics;
mod log;
mod mouse;
mod pci;
mod prelude;
mod xhc;

entry_point!(kernel_main);

#[allow(clippy::expect_used)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    log::set_level(log::Level::Debug);

    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .expect("framebuffer not supported");
    framebuffer::init(framebuffer).expect("failed to initialize framebuffer");

    desktop::draw().expect("failed to draw desktop");

    println!("Welcome to sabios!");

    mouse::draw_cursor().expect("failed to draw mouse cursor");

    let devices = pci::scan_all_bus().expect("failed to scan PCI devices");
    for device in &devices {
        debug!("{}", device);
    }
    xhc::init(&devices).expect("failed to initialize xHC");

    hlt_loop();
}

fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    hlt_loop();
}
