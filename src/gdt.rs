use crate::{error::ConvertErr as _, prelude::*};
use conquer_once::spin::OnceCell;
use x86_64::{
    instructions::segmentation,
    structures::gdt::{Descriptor, GlobalDescriptorTable},
};

static GDT: OnceCell<GlobalDescriptorTable> = OnceCell::uninit();

pub(crate) fn init() -> Result<()> {
    let mut code_selector = None;
    let mut stack_selector = None;
    GDT.init_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        code_selector = Some(gdt.add_entry(Descriptor::kernel_code_segment()));
        stack_selector = Some(gdt.add_entry(Descriptor::kernel_data_segment()));
        gdt
    });
    GDT.try_get().convert_err("gdt::GDT")?.load();

    if let Some(stack_selector) = stack_selector {
        unsafe { segmentation::load_ss(stack_selector) };
    }
    if let Some(code_selector) = code_selector {
        unsafe { segmentation::set_cs(code_selector) };
    }
    Ok(())
}
