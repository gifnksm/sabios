#![warn(unsafe_op_in_unsafe_fn)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![no_std]
#![no_main]

extern crate alloc;

use self::{
    co_task::{CoTask, Executor},
    prelude::*,
};
use bootloader::{
    boot_info::{FrameBuffer, Optional},
    entry_point, BootInfo,
};
use core::mem;
use futures_util::StreamExt;
use x86_64::VirtAddr;

mod acpi;
mod allocator;
mod buffer_drawer;
mod co_task;
mod console;
mod cxx_support;
mod desktop;
mod emergency_console;
mod error;
mod font;
mod framebuffer;
mod gdt;
mod graphics;
mod interrupt;
mod keyboard;
mod layer;
mod log;
mod main_window;
mod memory;
mod mouse;
mod paging;
mod pci;
mod prelude;
mod sync;
mod text_window;
mod timer;
mod window;
mod xhc;

entry_point!(kernel_main);

#[allow(clippy::expect_used)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    log::set_level(log::Level::Info);

    let (framebuffer, physical_memory_offset, rsdp) =
        extract_boot_info(boot_info).expect("failed to extract boot_info");

    // Initialize framebuffer for boot log
    framebuffer::init(framebuffer).expect("failed to initialize framebuffer");

    // Initialize memory mapping / frame allocator / heap
    let mut mapper = unsafe { paging::init(physical_memory_offset) };
    {
        let mut allocator = memory::lock_memory_manager();

        allocator
            .init(&*boot_info.memory_regions)
            .expect("failed to initialize bitmap memory manager");

        // Map CPU register addresses as identity mapping
        paging::make_identity_mapping(&mut mapper, &mut *allocator, 0xfee00000, 1)
            .expect("failed to map CPU register addresses");

        allocator::init_heap(&mut mapper, &mut *allocator).expect("failed to initialize heap");
    }

    // Initialize GDT/IDT
    gdt::init();
    interrupt::init();

    // Initialize PCI devices
    let devices = pci::scan_all_bus().expect("failed to scan PCI devices");
    xhc::init(&devices, &mut mapper).expect("failed to initialize xHC");

    // Initialize LAPIC timer
    unsafe { acpi::init(&mut mapper, rsdp) }.expect("failed to initialize acpi");
    timer::lapic::init();

    let console_param = console::start_window_mode().expect("failed to start console window mode");

    // Initialize executor & co-tasks
    let mut executor = Executor::new();
    executor.spawn(CoTask::new(xhc::handler_task()));
    executor.spawn(CoTask::new(timer::lapic::handler_task()));
    executor.spawn(CoTask::new(mouse::handler_task()));
    executor.spawn(CoTask::new(keyboard::handler_task()));
    executor.spawn(CoTask::new(desktop::handler_task()));
    executor.spawn(CoTask::new(console::handler_task(console_param)));
    executor.spawn(CoTask::new(main_window::handler_task()));
    executor.spawn(CoTask::new(text_window::handler_task()));
    executor.spawn(CoTask::new(layer::handler_task()));

    executor.spawn(CoTask::new(async {
        #[allow(clippy::unwrap_used)]
        let timeout = timer::lapic::oneshot(600).unwrap();
        println!("Timer interrupt, timeout = {}", timeout.await);
    }));
    executor.spawn(CoTask::new(async {
        let mut i = 0;
        #[allow(clippy::unwrap_used)]
        let mut timer = timer::lapic::interval(200, 100).unwrap();
        while let Some(Ok(timeout)) = timer.next().await {
            println!("Timer interrupt, timeout = {}, value = {}", timeout, i);
            i += 1;
        }
    }));

    x86_64::instructions::interrupts::enable();

    // Start running
    println!("Welcome to sabios!");

    executor.run();
}

fn extract_boot_info(boot_info: &mut BootInfo) -> Result<(FrameBuffer, VirtAddr, VirtAddr)> {
    let framebuffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
        .into_option()
        .ok_or(ErrorKind::FrameBufferNotSupported)?;

    let physical_memory_offset = boot_info
        .physical_memory_offset
        .as_ref()
        .copied()
        .ok_or(ErrorKind::PhysicalMemoryNotMapped)?;
    let physical_memory_offset = VirtAddr::new(physical_memory_offset);

    let rsdp = boot_info
        .rsdp_addr
        .as_ref()
        .copied()
        .ok_or(ErrorKind::RsdpNotMapped)?;
    let rsdp = VirtAddr::new(rsdp);

    Ok((framebuffer, physical_memory_offset, rsdp))
}

fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write as _;
    emergency_console::with_console(|console| {
        let _ = write!(console, "{}", info);
    });
}
