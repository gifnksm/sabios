#![warn(unsafe_op_in_unsafe_fn)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![no_std]
#![no_main]

use self::error::{Error, ErrorKind, Result};
use bootloader::{
    boot_info::{MemoryRegion, Optional},
    entry_point, BootInfo,
};
use core::mem;

mod console;
mod desktop;
mod error;
mod font;
mod framebuffer;
mod graphics;
mod mouse;
mod pci;

struct MemoryRegions<'a> {
    regions: core::slice::Iter<'a, MemoryRegion>,
}

impl<'a> Iterator for MemoryRegions<'a> {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        let mut current = *self.regions.next()?;
        loop {
            #[allow(clippy::suspicious_operation_groupings)]
            match self.regions.as_slice().get(0) {
                Some(next) if current.kind == next.kind && current.end == next.start => {
                    current.end = next.end;
                    let _ = self.regions.next();
                    continue;
                }
                _ => return Some(current),
            }
        }
    }
}

entry_point!(kernel_main);

#[allow(clippy::expect_used)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .expect("framebuffer not supported");
    framebuffer::init(framebuffer).expect("failed to initialize framebuffer");

    desktop::draw().expect("failed to draw desktop");

    println!("Welcome to sabios!");

    for region in (MemoryRegions {
        regions: boot_info.memory_regions.iter(),
    }) {
        println!(
            "addr={:08x}-{:08x}, pages = {:08x}, kind = {:?}",
            region.start,
            region.end,
            (region.end - region.start) / 4096,
            region.kind,
        );
    }

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
