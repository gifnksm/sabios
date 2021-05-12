use crate::{
    memory,
    pci::{self, Device},
    prelude::*,
};
use mikanos_usb as usb;
use x86_64::{
    structures::paging::{Mapper, OffsetPageTable, Page, PhysFrame},
    PhysAddr, VirtAddr,
};

pub(crate) fn init(devices: &[Device], mapper: &mut OffsetPageTable) -> Result<()> {
    let mut xhc_dev = None;
    for dev in devices {
        // is the device is xHC?
        if dev.class_code.test3(0x0c, 0x03, 0x30) {
            xhc_dev = Some(dev);

            if dev.vendor_id == 0x8086 {
                // prefer Intel's xHC
                break;
            }
        }
    }

    let xhc_dev = xhc_dev.ok_or_else(|| make_error!(ErrorKind::XhcNotFound))?;
    info!("xHC has been found: {}", xhc_dev);

    let xhc_bar = pci::read_bar(&xhc_dev, 0)?;
    debug!("xHC BAR0 = {:08x}", xhc_bar);
    let xhc_mmio_base = xhc_bar & !0xf;
    debug!("xHC mmio_base = {:08x}", xhc_mmio_base);

    {
        // Map [xhc_mmio_base..(xhc_mmio_base+64kib)] as identity map
        use x86_64::structures::paging::PageTableFlags as Flags;
        let base_page = Page::from_start_address(VirtAddr::new(xhc_mmio_base))?;
        let base_frame = PhysFrame::from_start_address(PhysAddr::new(xhc_mmio_base))?;
        let flags = Flags::PRESENT | Flags::WRITABLE;
        let mut allocator = memory::lock_memory_manager();
        for i in 0..16 {
            let page = base_page + i;
            let frame = base_frame + i;
            unsafe { mapper.map_to(page, frame, flags, &mut *allocator) }?.flush();
        }
    }

    unsafe {
        let _xhc = usb::xhc_controller_new(xhc_mmio_base);
        //xhc.init();
    }

    warn!("OK");

    Ok(())
}
