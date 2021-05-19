use crate::{emergency_console, println, sync::OnceCell, xhc};
use core::fmt::Write as _;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum InterruptIndex {
    Xhci = 0x40,
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
        idt
    });
    IDT.get().load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT");
    println!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

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
    emergency_console::with_console(|console| {
        let _ = writeln!(console, "EXCEPTION: DOUBLE FAULT",);
        let _ = writeln!(console, "Error Code: {:x}", error_code);
        let _ = writeln!(console, "{:#?}", stack_frame);
    });
}
