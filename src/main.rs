#![warn(unsafe_op_in_unsafe_fn)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![no_std]
#![no_main]

use self::prelude::*;
use bootloader::{
    boot_info::{MemoryRegion, Optional},
    entry_point, BootInfo,
};
use core::mem;
use x86_64::{structures::paging::Translate, VirtAddr};

mod console;
mod desktop;
mod error;
mod font;
mod framebuffer;
mod graphics;
mod log;
mod memory;
mod mouse;
mod paging;
mod pci;
mod prelude;
mod xhc;

struct MemoryRegions<'a> {
    regions: core::slice::Iter<'a, MemoryRegion>,
}

impl<'a> MemoryRegions<'a> {
    fn new(regions: &'a [MemoryRegion]) -> Self {
        Self {
            regions: regions.iter(),
        }
    }
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
    log::set_level(log::Level::Debug);

    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .expect("framebuffer not supported");
    framebuffer::init(framebuffer).expect("failed to initialize framebuffer");

    let physical_memory_offset = boot_info
        .physical_memory_offset
        .as_ref()
        .copied()
        .expect("physical memory is not mapped");
    let physical_memory_offset = VirtAddr::new(physical_memory_offset);

    let mapper = unsafe { paging::init(physical_memory_offset) };

    desktop::draw().expect("failed to draw desktop");

    let addresses = &[
        0xb8000,
        0x201008,
        0x0100_0020_1a10,
        physical_memory_offset.as_u64(),
    ];

    for &address in addresses {
        let virt = VirtAddr::new(address);
        let phys = mapper.translate(virt);
        println!("{:?} -> {:?}", virt, phys);
    }

    println!("Welcome to sabios!");

    let mut allocator = memory::lock_memory_manager();
    allocator
        .init(MemoryRegions::new(&*boot_info.memory_regions))
        .expect("failed to initialize bitmap memory manager");

    for region in MemoryRegions::new(&*boot_info.memory_regions) {
        println!(
            "addr={:08x}-{:08x}, pages = {:08x}, kind = {:?}",
            region.start,
            region.end,
            (region.end - region.start) / 4096,
            region.kind,
        );
    }

    {
        let frames1 = allocator.allocate(3).expect("failed to allocate");
        println!("allocated: {:?}", frames1);
        let frames2 = allocator.allocate(5).expect("failed to allocate");
        println!("allocated: {:?}", frames2);
        allocator.free(frames1);
        let frames3 = allocator.allocate(4).expect("failed to allocate");
        println!("allocated: {:?}", frames3);
        let frames4 = allocator.allocate(3).expect("failed to allocate");
        println!("allocated: {:?}", frames4);
    }

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
