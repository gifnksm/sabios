use crate::{
    error::ConvertErr as _,
    interrupt::InterruptIndex,
    memory, mouse, paging,
    pci::{self, Device, MsiDeliveryMode, MsiTriggerMode},
    prelude::*,
};
use conquer_once::spin::OnceCell;
use mikanos_usb as usb;
use volatile::Volatile;
use x86_64::structures::{idt::InterruptStackFrame, paging::OffsetPageTable};

static XHC: OnceCell<spin::Mutex<&'static mut usb::xhci::Controller>> = OnceCell::uninit();

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

    let bsp_local_apic_id = unsafe { *(0xfee00020 as *const u32) } >> 24;
    pci::configure_msi_fixed_destination(
        xhc_dev,
        bsp_local_apic_id,
        MsiTriggerMode::Level,
        MsiDeliveryMode::Fixed,
        InterruptIndex::Xhci,
        0,
    )?;

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

    XHC.try_init_once(move || spin::Mutex::new(xhc))
        .convert_err("xhc::XHC")?;

    Ok(())
}

fn map_xhc_mmio(mapper: &mut OffsetPageTable, xhc_mmio_base: u64) -> Result<()> {
    // Map [xhc_mmio_base..(xhc_mmio_base+64kib)] as identity map
    let mut allocator = memory::lock_memory_manager()?;
    paging::make_identity_mapping(mapper, &mut *allocator, xhc_mmio_base, 16)
}

fn alloc_memory_pool(mapper: &mut OffsetPageTable) -> Result<()> {
    let num_frames = 32;
    let mut allocator = memory::lock_memory_manager()?;
    let frame_range = allocator.allocate(num_frames)?;
    let base_addr = frame_range.start.start_address().as_u64();
    paging::make_identity_mapping(mapper, &mut *allocator, base_addr, num_frames)?;
    unsafe { usb::set_memory_pool(base_addr, num_frames * (memory::BYTES_PER_FRAME as usize)) };
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

pub(crate) extern "x86-interrupt" fn interrupt_handler(_stack_frame: InterruptStackFrame) {
    if let Err(err) = interrupt_handler_inner() {
        error!("error while interrupt handling: {}", err);
    }
}

fn interrupt_handler_inner() -> Result<()> {
    let mut xhc = XHC
        .try_get()
        .convert_err("xhc::XHC")?
        .try_lock()
        .ok_or_else(|| make_error!(ErrorKind::WouldBlock("xhc::XHC")))?;
    while xhc.has_event() {
        if let Err(err) = xhc.process_event().map_err(Error::from) {
            error!("error while process_event: {}", err);
        }
    }
    notify_end_of_interrupt();
    Ok(())
}

fn notify_end_of_interrupt() {
    #[allow(clippy::unwrap_used)]
    let mut memory = Volatile::new(unsafe { (0xfee000b0 as *mut u32).as_mut().unwrap() });
    memory.write(0);
}
