use crate::{error::ConvertErr as _, prelude::*, println, xhc};
use conquer_once::spin::OnceCell;
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

pub(crate) fn init() -> Result<()> {
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
    IDT.try_get().convert_err("interrupt::IDT")?.load();
    Ok(())
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

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:x}", error_code);
    println!("{:#?}", stack_frame);

    crate::hlt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("EXCEPTION: GENERAL PROTECTION FAULT");
    println!("Error Code: {:x}", error_code);
    println!("{:#?}", stack_frame);

    crate::hlt_loop();
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("EXCEPTION: STACK NOT PRESENT");
    println!("Error Code: {:x}", error_code);
    println!("{:#?}", stack_frame);

    crate::hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT\nError Code: {:x}\n{:#?}",
        error_code, stack_frame
    );
}
