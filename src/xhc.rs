use crate::{
    memory, mouse,
    pci::{self, Device},
    prelude::*,
};
use mikanos_usb as usb;
use x86_64::{
    structures::paging::{Mapper, OffsetPageTable, Page, PhysFrame},
    PhysAddr, VirtAddr,
};

pub(crate) fn init(
    devices: &[Device],
    mapper: &mut OffsetPageTable,
) -> Result<&'static mut usb::xhci::Controller> {
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

    map_xhc_mmio(mapper, xhc_mmio_base)?;
    alloc_memory_pool(mapper)?;

    let xhc = unsafe { usb::xhci::Controller::new(xhc_mmio_base) };

    if xhc_dev.vendor_id == 0x8086 {
        switch_ehci_to_xhci(devices, xhc_dev);
    }

    xhc.init();
    debug!("xhc starting");
    xhc.run()?;

    usb::HidMouseDriver::set_default_observer(mouse::observer);

    xhc.configure_connected_ports();

    Ok(xhc)
}

fn map_xhc_mmio(mapper: &mut OffsetPageTable, xhc_mmio_base: u64) -> Result<()> {
    // Map [xhc_mmio_base..(xhc_mmio_base+64kib)] as identity map
    use x86_64::structures::paging::PageTableFlags as Flags;
    let base_page = Page::from_start_address(VirtAddr::new(xhc_mmio_base))?;
    let base_frame = PhysFrame::from_start_address(PhysAddr::new(xhc_mmio_base))?;
    let flags = Flags::PRESENT | Flags::WRITABLE;
    let mut allocator = memory::lock_memory_manager()?;
    for i in 0..16 {
        let page = base_page + i;
        let frame = base_frame + i;
        unsafe { mapper.map_to(page, frame, flags, &mut *allocator) }?.flush();
    }
    Ok(())
}

fn alloc_memory_pool(mapper: &mut OffsetPageTable) -> Result<()> {
    use x86_64::structures::paging::PageTableFlags as Flags;
    let num_frames = 32;
    let mut allocator = memory::lock_memory_manager()?;
    let frame_range = allocator.allocate(num_frames)?;
    let page_range = Page::range(
        Page::from_start_address(VirtAddr::new(frame_range.start.start_address().as_u64()))?,
        Page::from_start_address(VirtAddr::new(frame_range.end.start_address().as_u64()))?,
    );
    let flags = Flags::PRESENT | Flags::WRITABLE;
    for (frame, page) in frame_range.zip(page_range) {
        unsafe { mapper.map_to(page, frame, flags, &mut *allocator) }?.flush();
    }
    unsafe {
        usb::set_memory_pool(
            page_range.start.start_address().as_u64(),
            num_frames * (memory::BYTES_PER_FRAME as usize),
        );
    }
    Ok(())
}

fn switch_ehci_to_xhci(devices: &[Device], xhc_dev: &Device) {
    let intel_ehc_exists = devices.iter().any(|dev| {
        dev.class_code.test3(0x0c, 0x03, 0x20) &&  // EHCI
        dev.vendor_id == 0x8086 // Intel
    });
    if !intel_ehc_exists {
        return;
    }

    let superspeed_ports = pci::read_conf_reg(xhc_dev, 0xdc); // USB3PRM
    pci::write_conf_reg(xhc_dev, 0xf8, superspeed_ports); // USB3_PSSEN
    let ehci2xhci_ports = pci::read_conf_reg(xhc_dev, 0xd4); // XUSB2PRM
    pci::write_conf_reg(xhc_dev, 0xd0, ehci2xhci_ports); // XUSB2PR
    debug!(
        "switch_ehci_to_xhci: SS={:2x}, xHCI={:2x}",
        superspeed_ports, ehci2xhci_ports
    );
}
