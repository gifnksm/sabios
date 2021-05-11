use crate::{
    pci::{self, Device},
    prelude::*,
};
use mikanos_usb as usb;

pub(crate) fn init(devices: &[Device], physical_memory_offset: u64) -> Result<()> {
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

    let xhc_bar = physical_memory_offset + pci::read_bar(&xhc_dev, 0)?;
    debug!("xHC BAR0 = {:08x}", xhc_bar);
    let xhc_mmio_base = xhc_bar & !0xf;
    debug!("xHC mmio_base = {:08x}", xhc_mmio_base);

    unsafe {
        // page fault occurs... xhc_mmio_base is not mapped to virtual memory
        let _xhc = usb::xhc_controller_new(xhc_mmio_base);
        //xhc.init();
    }

    warn!("OK");

    Ok(())
}
