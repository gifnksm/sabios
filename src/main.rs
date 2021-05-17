#![warn(unsafe_op_in_unsafe_fn)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![no_std]
#![no_main]

extern crate alloc;

use crate::co_task::CoTask;

use self::{co_task::Executor, prelude::*};
use bootloader::{boot_info::Optional, entry_point, BootInfo};
use core::mem;
use x86_64::VirtAddr;

mod allocator;
mod co_task;
mod console;
mod cxx_support;
mod desktop;
mod error;
mod font;
mod framebuffer;
mod gdt;
mod graphics;
mod interrupt;
mod log;
mod memory;
mod mouse;
mod paging;
mod pci;
mod prelude;
mod util;
mod xhc;

entry_point!(kernel_main);

#[allow(clippy::expect_used)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    log::set_level(log::Level::Info);

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
    let mut mapper = unsafe { paging::init(physical_memory_offset) };

    desktop::draw().expect("failed to draw desktop");

    {
        let mut allocator = memory::lock_memory_manager().expect("failed to lock memory manager");

        allocator
            .init(&*boot_info.memory_regions)
            .expect("failed to initialize bitmap memory manager");

        // Map CPU register addresses as identity mapping
        paging::make_identity_mapping(&mut mapper, &mut *allocator, 0xfee00000, 1)
            .expect("failed to map CPU register addresses");

        allocator::init_heap(&mut mapper, &mut *allocator).expect("failed to initialize heap");
    }

    gdt::init().expect("failed to init gdt");
    interrupt::init().expect("failed to init interrupts");

    let devices = pci::scan_all_bus().expect("failed to scan PCI devices");
    for device in &devices {
        debug!("{}", device);
    }
    xhc::init(&devices, &mut mapper).expect("failed to initialize xHC");

    println!("Welcome to sabios!");

    let mut executor = Executor::new();
    executor.spawn(CoTask::new(xhc::handle_xhc_interrupt()));
    executor.spawn(CoTask::new(mouse::handle_mouse_event()));

    x86_64::instructions::interrupts::enable();

    executor.run();
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
