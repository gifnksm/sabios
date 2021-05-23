use crate::sync::OnceCell;
use x86_64::{
    instructions::segmentation,
    structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
};

#[derive(Debug)]
pub(crate) struct Selectors {
    pub(crate) kernel_code_selector: SegmentSelector,
    pub(crate) kernel_stack_selector: SegmentSelector,
}

static GDT: OnceCell<GlobalDescriptorTable> = OnceCell::uninit();
static SELECTORS: OnceCell<Selectors> = OnceCell::uninit();

pub(crate) fn init() {
    let null_segment = SegmentSelector(0);
    let mut selectors = Selectors {
        kernel_code_selector: null_segment,
        kernel_stack_selector: null_segment,
    };
    GDT.init_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        selectors.kernel_code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        selectors.kernel_stack_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        gdt
    });
    GDT.get().load();

    unsafe {
        segmentation::load_ds(null_segment);
        segmentation::load_es(null_segment);
        segmentation::load_fs(null_segment);
        segmentation::load_gs(null_segment);
    }

    unsafe { segmentation::load_ss(selectors.kernel_stack_selector) };
    unsafe { segmentation::set_cs(selectors.kernel_code_selector) };

    SELECTORS.init_once(|| selectors);
}

pub(crate) fn selectors() -> &'static Selectors {
    SELECTORS.get()
}
