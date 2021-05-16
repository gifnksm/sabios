use crate::{memory::BitmapMemoryManager, prelude::*};
use x86_64::{
    structures::paging::{Mapper, OffsetPageTable, Page, PageTable, PhysFrame},
    PhysAddr, VirtAddr,
};

/// Initialize a new OffsetPageTable.
///
/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub(crate) unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
    unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub(crate) unsafe fn active_level_4_table(
    physical_memory_offset: VirtAddr,
) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

pub(crate) fn make_identity_mapping(
    mapper: &mut OffsetPageTable,
    allocator: &mut BitmapMemoryManager,
    base_addr: u64,
    num_pages: usize,
) -> Result<()> {
    use x86_64::structures::paging::PageTableFlags as Flags;
    let base_page = Page::from_start_address(VirtAddr::new(base_addr))?;
    let base_frame = PhysFrame::from_start_address(PhysAddr::new(base_addr))?;
    let flags = Flags::PRESENT | Flags::WRITABLE;
    for i in 0..num_pages {
        let page = base_page + i as u64;
        let frame = base_frame + i as u64;
        unsafe { mapper.map_to(page, frame, flags, &mut *allocator) }?.flush();
    }
    Ok(())
}
