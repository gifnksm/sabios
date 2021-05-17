use crate::{error::ConvertErr as _, prelude::*};
use conquer_once::noblock::OnceCell;
use x86_64::{
    instructions::segmentation,
    structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
};

static GDT: OnceCell<GlobalDescriptorTable> = OnceCell::uninit();

pub(crate) fn init() -> Result<()> {
    let mut code_selector = None;
    let mut stack_selector = None;
    GDT.try_init_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        code_selector = Some(gdt.add_entry(Descriptor::kernel_code_segment()));
        stack_selector = Some(gdt.add_entry(Descriptor::kernel_data_segment()));
        gdt
    })
    .convert_err("gdt::GDT")?;
    GDT.try_get().convert_err("gdt::GDT")?.load();

    let null_segment = SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0);

    unsafe {
        segmentation::load_ds(null_segment);
        segmentation::load_es(null_segment);
        segmentation::load_fs(null_segment);
        segmentation::load_gs(null_segment);
    }

    if let Some(stack_selector) = stack_selector {
        unsafe { segmentation::load_ss(stack_selector) };
    }
    if let Some(code_selector) = code_selector {
        unsafe { segmentation::set_cs(code_selector) };
    }
    Ok(())
}
