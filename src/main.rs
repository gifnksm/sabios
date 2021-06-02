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

use crate::task::Task;

use self::{
    co_task::{CoTask, Executor},
    graphics::Point,
    prelude::*,
};
use bootloader::{
    boot_info::{FrameBuffer, Optional},
    entry_point, BootInfo,
};
use core::{mem, panic::PanicInfo};
use futures_util::StreamExt;
use sync::{mpsc, OnceCell};
use x86_64::VirtAddr;

mod acpi;
mod allocator;
mod buffer_drawer;
mod co_task;
mod console;
mod counter_window;
mod cxx_support;
mod desktop;
mod emergency_console;
mod error;
mod font;
mod framebuffer;
mod framed_window;
mod gdt;
mod graphics;
mod interrupt;
mod keyboard;
mod layer;
mod log;
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

    task::init();

    info!("Initialization completed");

    Ok(())
}

static KEYBOARD_EVENT_TX: OnceCell<mpsc::Sender<keyboard::KeyboardEvent>> = OnceCell::uninit();

#[allow(clippy::expect_used)]
fn start_window() -> ! {
    let layer_task = layer::handler_task();
    let console_param = console::start_window_mode().expect("failed to start console window mode");

    let task_id = task::current().id();

    // Initialize executor & co-tasks
    let mut executor = Executor::new(task_id);
    executor.spawn(CoTask::new(xhc::handler_task()));
    executor.spawn(CoTask::new(timer::lapic::handler_task()));
    executor.spawn(CoTask::new(mouse::handler_task()));
    executor.spawn(CoTask::new(keyboard::handler_task()));
    executor.spawn(CoTask::new(desktop::handler_task()));
    executor.spawn(CoTask::new(console::handler_task(console_param)));
    executor.spawn(CoTask::new(layer_task));

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

    task::spawn(Task::new(counter_window::handler_task(
        "Hello Window".into(),
        Point::new(300, 100),
    )));
    task::spawn(Task::new(text_window::handler_task()));
    let task_b_id = task::spawn(Task::new(counter_window::handler_task(
        "TaskB Window".into(),
        Point::new(100, 100),
    )));

    let (tx, mut rx) = mpsc::channel(100);
    KEYBOARD_EVENT_TX.init_once(|| tx);
    executor.spawn(CoTask::new(async move {
        let layer_tx = layer::event_tx();
        while let Some(event) = rx.next().await {
            match event.ascii {
                's' => {
                    x86_64::instructions::interrupts::without_interrupts(|| task::sleep(task_b_id));
                }
                'w' => {
                    x86_64::instructions::interrupts::without_interrupts(|| task::wake(task_b_id));
                }
                _ => {}
            }
            #[allow(clippy::unwrap_used)]
            layer_tx.keyboard_event(event).await.unwrap();
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
