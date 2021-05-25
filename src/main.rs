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
    prelude::*,
    sync::OnceCell,
    task::Task,
};
use bootloader::{
    boot_info::{FrameBuffer, Optional},
    entry_point, BootInfo,
};
use core::{mem, panic::PanicInfo};
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
mod serial;
mod sync;
mod task;
mod text_window;
mod timer;
mod window;
mod xhc;

entry_point!(kernel_main);

#[allow(clippy::expect_used)]
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    log::set_level(log::Level::Info);

    init(boot_info).expect("failed to initialize kernel");

    #[cfg(test)]
    test_main();

    start_window();
}

fn init(boot_info: &'static mut BootInfo) -> Result<()> {
    let (framebuffer, physical_memory_offset, rsdp) = extract_boot_info(boot_info)?;

    // Initialize framebuffer for boot log
    framebuffer::init(framebuffer)?;

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

    info!("Initialization completed");

    Ok(())
}

#[allow(clippy::expect_used)]
fn start_window() -> ! {
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

    TASK_MAIN.init_once(|| Task::new(dummy_task, 0, 0));
    TASK_B.init_once(|| Task::new(task_b, 1, 42));

    executor.spawn(CoTask::new(async {
        for i in 0.. {
            println!("Hello from taskA {}", i);
            co_task::yield_now().await;
            Task::switch(TASK_B.get(), TASK_MAIN.get());
        }
    }));

    x86_64::instructions::interrupts::enable();

    // Start running
    println!("Welcome to sabios!");

    executor.run();
}

static TASK_MAIN: OnceCell<Task> = OnceCell::uninit();
static TASK_B: OnceCell<Task> = OnceCell::uninit();

extern "C" fn dummy_task(_arg0: u64, _arg1: u64) {
    panic!("dummy task called;")
}

extern "C" fn task_b(_arg0: u64, _arg1: u64) {
    for i in 0.. {
        println!("Hello from taskB {}", i);
        Task::switch(TASK_MAIN.get(), TASK_B.get());
    }
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

#[cfg(test)]
mod tests {
    #[test_case]
    fn trivial_assertion() {
        assert_eq!(1, 1);
    }
}
