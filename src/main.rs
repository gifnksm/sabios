#![warn(unsafe_op_in_unsafe_fn)]
#![no_std]
#![no_main]

use self::error::{Error, ErrorKind, Result};
use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::mem;

mod console;
mod desktop;
mod error;
mod font;
mod framebuffer;
mod graphics;
mod mouse;
mod pci;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .expect("framebuffer not supported");
    framebuffer::init(framebuffer).expect("failed to initialize framebuffer");

    desktop::draw().expect("failed to draw desktop");

    println!("Welcome to sabios!");

    mouse::draw_cursor().expect("failed to draw mouse cursor");

    let devices = pci::scan_all_bus().expect("failed to scan PCI devices");

    for device in &devices {
        println!("{}", device);
    }

    Err::<(), _>(make_error!(ErrorKind::Uninit("test test test"))).expect("test");

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
    println!("{}", info);
    hlt_loop();
}
