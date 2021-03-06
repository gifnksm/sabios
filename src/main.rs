#![warn(unsafe_op_in_unsafe_fn)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(custom_test_frameworks)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![no_std]
#![no_main]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use self::{
    co_task::{CoTask, Executor},
    graphics::{Point, Size},
    prelude::*,
    task::Task,
    terminal::Terminal,
    text_window::TextWindow,
};
use bootloader::{
    boot_info::{FrameBuffer, Optional},
    entry_point, BootInfo,
};
use core::{mem, panic::PanicInfo};
use x86_64::VirtAddr;

mod acpi;
mod allocator;
mod co_task;
mod console;
mod cxx_support;
mod desktop;
mod emergency_console;
mod error;
mod fat;
mod fmt;
mod framed_window;
mod gdt;
mod graphics;
mod interrupt;
mod keyboard;
mod layer;
mod log;
mod macros;
mod memory;
mod mouse;
mod paging;
mod pci;
mod prelude;
mod serial;
mod sync;
mod task;
mod terminal;
mod text_window;
mod timer;
mod triple_buffer;
mod window;
mod xhc;

entry_point!(kernel_main);

#[allow(clippy::expect_used)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    log::set_level(log::Level::Warn, log::Level::Debug);

    init(boot_info).expect("failed to initialize kernel");

    #[cfg(test)]
    test_main();

    start_window();
}

fn init(boot_info: &'static mut BootInfo) -> Result<()> {
    let (frame_buffer, physical_memory_offset, rsdp) = extract_boot_info(boot_info)?;

    // Initialize graphics for boot log
    graphics::init(frame_buffer)?;

    // Initialize memory mapping / frame allocator / heap
    let mut mapper = unsafe { paging::init(physical_memory_offset) };
    {
        let mut allocator = memory::lock_memory_manager();

        allocator.init(&*boot_info.memory_regions)?;

        // Map CPU register addresses as identity mapping
        paging::make_identity_mapping(&mut mapper, &mut *allocator, 0xfee00000, 1)?;

        allocator::init_heap(&mut mapper, &mut *allocator)?;
    }

    // Initialize GDT/IDT
    gdt::init();
    interrupt::init();

    // Initialize PCI devices
    let devices = pci::scan_all_bus()?;
    xhc::init(&devices, &mut mapper)?;

    // Initialize LAPIC timer
    unsafe { acpi::init(&mut mapper, rsdp) }?;
    timer::lapic::init();

    // Initialize file system
    fat::init();

    task::init();

    info!("Initialization completed");

    Ok(())
}

#[allow(clippy::expect_used)]
fn start_window() -> ! {
    let layer_task = layer::handler_task().unwrap();
    let console_param = console::start_window_mode().expect("failed to start console window mode");

    let task_id = task::current().id();

    // Initialize executor & co-tasks
    let mut executor = Executor::new(task_id);
    executor.spawn(CoTask::new(xhc::handler_task()));
    executor.spawn(CoTask::new(timer::lapic::handler_task()));
    executor.spawn(CoTask::new(mouse::handler_task().unwrap()));
    executor.spawn(CoTask::new(keyboard::handler_task().unwrap()));
    executor.spawn(CoTask::new(desktop::handler_task().unwrap()));
    executor.spawn(CoTask::new(console::handler_task(console_param).unwrap()));
    executor.spawn(CoTask::new(layer_task));

    #[allow(clippy::unwrap_used)]
    task::spawn(Task::new(
        TextWindow::new("Text Box test".into(), Point::new(500, 100))
            .unwrap()
            .run()
            .unwrap(),
    ));
    #[allow(clippy::unwrap_used)]
    task::spawn(Task::new(
        Terminal::new(
            "sabios Terminal".into(),
            Point::new(100, 200),
            Size::new(60, 15),
        )
        .unwrap()
        .run()
        .unwrap(),
    ));

    x86_64::instructions::interrupts::enable();

    // Start running
    println!("Welcome to sabios!");

    executor.run();
}

fn extract_boot_info(boot_info: &mut BootInfo) -> Result<(FrameBuffer, VirtAddr, VirtAddr)> {
    let frame_buffer = mem::replace(&mut boot_info.framebuffer, Optional::None)
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

    Ok((frame_buffer, physical_memory_offset, rsdp))
}

fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use core::fmt::Write as _;
    emergency_console::with_console(|console| {
        let _ = write!(console, "{}", info);
    });
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

#[cfg(test)]
fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }

    hlt_loop();
}
