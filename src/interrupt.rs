use crate::{emergency_console, println, sync::OnceCell, timer, xhc};
use core::{
    fmt::Write as _,
    sync::atomic::{AtomicBool, Ordering},
};
use volatile::Volatile;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum InterruptIndex {
    Xhci = 0x40,
    Timer = 0x41,
}

impl InterruptIndex {
    pub(crate) fn as_u8(self) -> u8 {
        self as u8
    }

    pub(crate) fn as_u32(self) -> u32 {
        u32::from(self.as_u8())
    }

    pub(crate) fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

static IDT: OnceCell<InterruptDescriptorTable> = OnceCell::uninit();

pub(crate) fn init() {
    IDT.init_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt.segment_not_present
            .set_handler_fn(segment_not_present_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt[InterruptIndex::Xhci.as_usize()].set_handler_fn(xhc::interrupt_handler);
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer::lapic::interrupt_handler);
        idt
    });
    IDT.get().load();
}

static INTERRUPT_CONTEXT: AtomicBool = AtomicBool::new(false);

pub(crate) fn is_interrupt_context() -> bool {
    INTERRUPT_CONTEXT.load(Ordering::Relaxed)
}

pub(crate) struct InterruptContextGuard {}

impl InterruptContextGuard {
    pub(crate) fn new() -> Self {
        let old_value = INTERRUPT_CONTEXT.swap(true, Ordering::Relaxed);
        assert!(!old_value);
        Self {}
    }
}

impl Drop for InterruptContextGuard {
    fn drop(&mut self) {
        let old_value = INTERRUPT_CONTEXT.swap(false, Ordering::Relaxed);
        assert!(old_value);
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    let _guard = InterruptContextGuard::new();
    println!("EXCEPTION: BREAKPOINT");
    println!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    let _guard = InterruptContextGuard::new();
    emergency_console::with_console(|console| {
        let _ = writeln!(console, "EXCEPTION: PAGE FAULT");
        let _ = writeln!(console, "Accessed Address: {:?}", Cr2::read());
        let _ = writeln!(console, "Error Code: {:x}", error_code);
        let _ = writeln!(console, "{:#?}", stack_frame);
    });
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    let _guard = InterruptContextGuard::new();
    emergency_console::with_console(|console| {
        let _ = writeln!(console, "EXCEPTION: GENERAL PROTECTION FAULT");
        let _ = writeln!(console, "Error Code: {:x}", error_code);
        let _ = writeln!(console, "{:#?}", stack_frame);
    });
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    let _guard = InterruptContextGuard::new();
    emergency_console::with_console(|console| {
        let _ = writeln!(console, "EXCEPTION: STACK NOT PRESENT");
        let _ = writeln!(console, "Error Code: {:x}", error_code);
        let _ = writeln!(console, "{:#?}", stack_frame);
    });
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    let _guard = InterruptContextGuard::new();
    emergency_console::with_console(|console| {
        let _ = writeln!(console, "EXCEPTION: DOUBLE FAULT",);
        let _ = writeln!(console, "Error Code: {:x}", error_code);
        let _ = writeln!(console, "{:#?}", stack_frame);
    });
}

pub(crate) fn notify_end_of_interrupt() {
    assert!(is_interrupt_context());

    #[allow(clippy::unwrap_used)]
    let mut memory = Volatile::new(unsafe { (0xfee000b0 as *mut u32).as_mut().unwrap() });
    memory.write(0);
}
