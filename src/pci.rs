use crate::{interrupt::InterruptIndex, prelude::*, sync::SpinMutex};
use arrayvec::ArrayVec;
use bit_field::BitField;
use core::{fmt, ops::Range};
use custom_debug_derive::Debug as CustomDebug;
use x86_64::instructions::port::Port;

const INVALID_VENDOR_ID: u16 = 0xffff;

struct Addr(u32);

impl Addr {
    const BITS_ADDR: Range<usize> = 0..8;
    const BITS_FUNCTION: Range<usize> = 8..11;
    const BITS_DEVICE: Range<usize> = 11..16;
    const BITS_BUS: Range<usize> = 16..24;
    const BITS_RESERVED: Range<usize> = 24..31;
    const BIT_ENABLE: usize = 31;

    fn new(bus: u8, device: u8, function: u8, reg_addr: u8) -> Self {
        assert_eq!(reg_addr & 0x3, 0);
        let mut value = 0u32;
        value.set_bits(Self::BITS_ADDR, u32::from(reg_addr));
        value.set_bits(Self::BITS_FUNCTION, u32::from(function));
        value.set_bits(Self::BITS_DEVICE, u32::from(device));
        value.set_bits(Self::BITS_BUS, u32::from(bus));
        value.set_bits(Self::BITS_RESERVED, 0);
        value.set_bit(Self::BIT_ENABLE, true);
        Self(value)
    }

    // fn reg_addr(&self) -> u8 {
    //     self.0.get_bits(Self::BITS_ADDR) as u8
    // }
    // fn function(&self) -> u8 {
    //     self.0.get_bits(Self::BITS_FUNCTION) as u8
    // }
    // fn device(&self) -> u8 {
    //     self.0.get_bits(Self::BITS_DEVICE) as u8
    // }
    // fn bus(&self) -> u8 {
    //     self.0.get_bits(Self::BITS_BUS) as u8
    // }
    // fn enable(&self) -> bool {
    //     self.0.get_bit(Self::BIT_ENABLE)
    // }
}

#[derive(Debug)]
struct PortSet {
    addr: Port<u32>,
    data: Port<u32>,
}

#[derive(Debug)]
struct Config(SpinMutex<PortSet>);

static CONFIG: Config = Config(SpinMutex::new(PortSet {
    addr: Port::new(0x0cf8),
    data: Port::new(0xcfc),
}));

impl Config {
    fn read(&self, addr: Addr) -> u32 {
        let mut ports = self.0.lock();
        unsafe {
            ports.addr.write(addr.0);
            ports.data.read()
        }
    }

    fn write(&self, addr: Addr, data: u32) {
        let mut ports = self.0.lock();
        unsafe {
            ports.addr.write(addr.0);
            ports.data.write(data)
        }
    }
}

fn read_vendor_id(bus: u8, device: u8, function: u8) -> u16 {
    let addr = Addr::new(bus, device, function, 0x00);
    (CONFIG.read(addr) & 0xffff) as u16
}
// fn read_device_id(bus: u8, device: u8, function: u8) -> u16 {
//     let addr = Addr::new(bus, device, function, 0x00);
//     (CONFIG.read(addr) >> 16) as u16
// }
fn read_header_type(bus: u8, device: u8, function: u8) -> u8 {
    let addr = Addr::new(bus, device, function, 0x0c);
    ((CONFIG.read(addr) >> 16) & 0xff) as u8
}
fn read_class_code(bus: u8, device: u8, function: u8) -> ClassCode {
    let addr = Addr::new(bus, device, function, 0x08);
    let reg = CONFIG.read(addr);
    ClassCode {
        base: ((reg >> 24) & 0xff) as u8,
        sub: ((reg >> 16) & 0xff) as u8,
        interface: ((reg >> 8) & 0xff) as u8,
    }
}
fn read_bus_number(bus: u8, device: u8, function: u8) -> u32 {
    let addr = Addr::new(bus, device, function, 0x18);
    CONFIG.read(addr)
}
fn is_single_function_device(header_type: u8) -> bool {
    (header_type & 0x80) == 0
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Device {
    pub(crate) bus: u8,
    pub(crate) device: u8,
    pub(crate) function: u8,
    pub(crate) vendor_id: u16,
    pub(crate) class_code: ClassCode,
    pub(crate) header_type: u8,
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}: vend {:04x}, class {}, head {:02x}",
            self.bus, self.device, self.function, self.vendor_id, self.class_code, self.header_type
        )
    }
}

impl Device {
    fn addr(&self, reg_addr: u8) -> Addr {
        Addr::new(self.bus, self.device, self.function, reg_addr)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ClassCode {
    pub(crate) base: u8,
    pub(crate) sub: u8,
    pub(crate) interface: u8,
}

impl fmt::Display for ClassCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}.{:02x}.{:02x}",
            self.base, self.sub, self.interface
        )
    }
}

impl ClassCode {
    pub(crate) fn test3(&self, base: u8, sub: u8, interface: u8) -> bool {
        self.interface == interface && self.test2(base, sub)
    }
    pub(crate) fn test2(&self, base: u8, sub: u8) -> bool {
        self.sub == sub && self.test1(base)
    }
    pub(crate) fn test1(&self, base: u8) -> bool {
        self.base == base
    }
}

pub(crate) type Devices = ArrayVec<Device, 32>;

pub(crate) fn scan_all_bus() -> Result<Devices> {
    let mut devices = Devices::new();

    let header_type = read_header_type(0, 0, 0);
    if is_single_function_device(header_type) {
        scan_bus(&mut devices, 0)?;
        return Ok(devices);
    }

    for bus in 0..8 {
        if read_vendor_id(0, 0, bus) == INVALID_VENDOR_ID {
            continue;
        }
        scan_bus(&mut devices, bus)?;
    }
    Ok(devices)
}

pub(crate) fn scan_bus(devices: &mut Devices, bus: u8) -> Result<()> {
    for device in 0..32 {
        if read_vendor_id(bus, device, 0) == INVALID_VENDOR_ID {
            continue;
        }
        scan_device(devices, bus, device)?;
    }
    Ok(())
}

fn scan_device(devices: &mut Devices, bus: u8, device: u8) -> Result<()> {
    scan_function(devices, bus, device, 0)?;
    if is_single_function_device(read_header_type(bus, device, 0)) {
        return Ok(());
    }
    for function in 1..8 {
        if read_vendor_id(bus, device, function) == INVALID_VENDOR_ID {
            continue;
        }
        scan_function(devices, bus, device, function)?;
    }
    Ok(())
}

fn scan_function(devices: &mut Devices, bus: u8, device: u8, function: u8) -> Result<()> {
    let vendor_id = read_vendor_id(bus, device, function);
    let class_code = read_class_code(bus, device, function);
    let header_type = read_header_type(bus, device, function);
    let dev = Device {
        bus,
        device,
        function,
        vendor_id,
        class_code,
        header_type,
    };
    debug!("{}", dev);
    devices.try_push(dev).map_err(|_| ErrorKind::Full)?;

    if class_code.base == 0x06 && class_code.sub == 0x04 {
        // standard PCI-PCI bridge
        let bus_numbers = read_bus_number(bus, device, function);
        let secondary_bus = ((bus_numbers >> 8) & 0xff) as u8;
        scan_bus(devices, secondary_bus)?;
    }

    Ok(())
}

fn calc_bar_addr(bar_index: u8) -> u8 {
    0x10 + 4 * bar_index
}

pub(crate) fn read_conf_reg(dev: &Device, reg_addr: u8) -> u32 {
    CONFIG.read(dev.addr(reg_addr))
}

pub(crate) fn write_conf_reg(dev: &Device, reg_addr: u8, value: u32) {
    CONFIG.write(dev.addr(reg_addr), value)
}

pub(crate) fn read_bar(dev: &Device, bar_index: u8) -> Result<u64> {
    if bar_index >= 6 {
        bail!(ErrorKind::IndexOutOfRange);
    }

    let addr = calc_bar_addr(bar_index);
    let bar = read_conf_reg(dev, addr);

    // 32 bit address
    if (bar & 4) == 0 {
        return Ok(u64::from(bar));
    }
    // 64 bit address
    if bar_index >= 5 {
        bail!(ErrorKind::IndexOutOfRange);
    }

    let bar_upper = read_conf_reg(dev, addr + 4);
    Ok(u64::from(bar) | u64::from(bar_upper) << 32)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MsiTriggerMode {
    Edge,
    Level,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum MsiDeliveryMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    Smi = 0b010,
    Nmi = 0b100,
    Init = 0b101,
    ExtInt = 0b111,
}

impl MsiDeliveryMode {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_u32(self) -> u32 {
        u32::from(self.as_u8())
    }
}

const CAPABILITY_MSI: u8 = 0x05;
const CAPABILITY_MSIX: u8 = 0x11;

#[derive(CustomDebug, Clone, Copy, Default)]
#[repr(C)]
struct CapabilityHeader {
    cap_id: u8,
    #[debug(format = "{:02x}")]
    next_ptr: u8,
    #[debug(format = "{:04x}")]
    cap: u16,
}

impl From<u32> for CapabilityHeader {
    fn from(value: u32) -> Self {
        let mut header = Self::default();
        unsafe { *(&mut header as *mut _ as *mut u32) = value };
        header
    }
}

impl CapabilityHeader {
    fn as_u32(self) -> u32 {
        unsafe { *(&self as *const _ as *const u32) }
    }

    const BIT_MSI_ENABLE: usize = 0;
    const BITS_MULTI_MSG_CAPABLE: Range<usize> = 1..4;
    const BITS_MULTI_MSG_ENABLE: Range<usize> = 4..7;
    const BIT_ADDR_64_CAPABLE: usize = 7;
    const BIT_PER_VECTOR_MASK_CAPABLE: usize = 8;
    // const BITS_RESERVED: Range<usize> = 9..16;
    // fn msi_enable(self) -> bool {
    //     self.cap.get_bit(Self::BIT_MSI_ENABLE)
    // }
    fn set_msi_enable(&mut self, value: bool) -> &mut Self {
        let _ = self.cap.set_bit(Self::BIT_MSI_ENABLE, value);
        self
    }
    fn multi_msg_capable(self) -> u8 {
        self.cap.get_bits(Self::BITS_MULTI_MSG_CAPABLE) as u8
    }
    // fn multi_msg_enable(self) -> u8 {
    //     self.cap.get_bits(Self::BITS_MULTI_MSG_ENABLE) as u8
    // }
    fn set_multi_msg_enable(&mut self, value: u8) -> &mut Self {
        let _ = self
            .cap
            .set_bits(Self::BITS_MULTI_MSG_ENABLE, u16::from(value));
        self
    }
    fn addr_64_capable(self) -> bool {
        self.cap.get_bit(Self::BIT_ADDR_64_CAPABLE)
    }
    fn per_vector_mask_capable(self) -> bool {
        self.cap.get_bit(Self::BIT_PER_VECTOR_MASK_CAPABLE)
    }
}

#[derive(CustomDebug, Clone, Copy)]
#[repr(C)]
struct MsiCapability {
    header: CapabilityHeader,
    #[debug(format = "{:08x}")]
    msg_addr: u32,
    #[debug(format = "{:08x}")]
    msg_upper_addr: u32,
    #[debug(format = "{:08x}")]
    msg_data: u32,
    #[debug(format = "{:08x}")]
    mask_bits: u32,
    #[debug(format = "{:08x}")]
    pending_bits: u32,
}

pub(crate) fn configure_msi_fixed_destination(
    dev: &Device,
    apic_id: u32,
    trigger_mode: MsiTriggerMode,
    delivery_mode: MsiDeliveryMode,
    vector: InterruptIndex,
    num_vector_exponent: u8,
) -> Result<()> {
    let msg_addr = 0xfee00000 | (apic_id << 12);
    let mut msg_data = (delivery_mode.as_u32() << 8) | vector.as_u32();
    if trigger_mode == MsiTriggerMode::Level {
        msg_data |= 0xc000;
    }
    configure_msi(dev, msg_addr, msg_data, num_vector_exponent)?;
    Ok(())
}

fn configure_msi(
    dev: &Device,
    msg_addr: u32,
    msg_data: u32,
    num_vector_exponent: u8,
) -> Result<()> {
    let mut cap_addr = (read_conf_reg(dev, 0x34) & 0xff) as u8;
    let mut msi_cap_addr = None;
    let mut msix_cap_addr = None;
    while cap_addr != 0 {
        let header = read_capability_header(dev, cap_addr);
        match header.cap_id {
            CAPABILITY_MSI => msi_cap_addr = Some(cap_addr),
            CAPABILITY_MSIX => msix_cap_addr = Some(cap_addr),
            _ => {}
        }
        cap_addr = header.next_ptr;
    }
    if let Some(cap_addr) = msi_cap_addr {
        return configure_msi_register(dev, cap_addr, msg_addr, msg_data, num_vector_exponent);
    }
    if let Some(cap_addr) = msix_cap_addr {
        return configure_msix_register(dev, cap_addr, msg_addr, msg_data, num_vector_exponent);
    }
    bail!(ErrorKind::NoPciMsi)
}

fn configure_msi_register(
    dev: &Device,
    cap_addr: u8,
    msg_addr: u32,
    msg_data: u32,
    num_vector_exponent: u8,
) -> Result<()> {
    let mut msi_cap = read_msi_capability(dev, cap_addr);

    let multi_msg_enable = u8::min(msi_cap.header.multi_msg_capable(), num_vector_exponent);
    msi_cap.header.set_multi_msg_enable(multi_msg_enable);
    msi_cap.header.set_msi_enable(true);
    msi_cap.msg_addr = msg_addr;
    msi_cap.msg_data = msg_data;

    write_msi_capability(dev, cap_addr, msi_cap);

    Ok(())
}

fn configure_msix_register(
    _dev: &Device,
    _cap_addr: u8,
    _msg_addr: u32,
    _msg_data: u32,
    _num_vector_exponent: u8,
) -> Result<()> {
    bail!(ErrorKind::NotImplemented)
}

fn read_capability_header(dev: &Device, cap_addr: u8) -> CapabilityHeader {
    CapabilityHeader::from(read_conf_reg(dev, cap_addr))
}

fn read_msi_capability(dev: &Device, cap_addr: u8) -> MsiCapability {
    let header = read_capability_header(dev, cap_addr);
    let msg_addr = read_conf_reg(dev, cap_addr + 4);
    let msg_upper_addr;

    let msg_data_addr;
    if header.addr_64_capable() {
        msg_upper_addr = read_conf_reg(dev, cap_addr + 8);
        msg_data_addr = cap_addr + 12;
    } else {
        msg_upper_addr = 0;
        msg_data_addr = cap_addr + 8;
    }
    let msg_data = read_conf_reg(dev, msg_data_addr);
    let mask_bits;
    let pending_bits;

    if header.per_vector_mask_capable() {
        mask_bits = read_conf_reg(dev, msg_data_addr + 4);
        pending_bits = read_conf_reg(dev, msg_data_addr + 8);
    } else {
        mask_bits = 0;
        pending_bits = 0;
    }

    MsiCapability {
        header,
        msg_addr,
        msg_upper_addr,
        msg_data,
        mask_bits,
        pending_bits,
    }
}

fn write_msi_capability(dev: &Device, cap_addr: u8, msi_cap: MsiCapability) {
    write_conf_reg(dev, cap_addr, msi_cap.header.as_u32());
    write_conf_reg(dev, cap_addr + 4, msi_cap.msg_addr);

    let msg_data_addr;
    if msi_cap.header.addr_64_capable() {
        write_conf_reg(dev, cap_addr + 8, msi_cap.msg_upper_addr);
        msg_data_addr = cap_addr + 12;
    } else {
        msg_data_addr = cap_addr + 8;
    }
    write_conf_reg(dev, msg_data_addr, msi_cap.msg_data);

    if msi_cap.header.per_vector_mask_capable() {
        write_conf_reg(dev, msg_data_addr + 4, msi_cap.mask_bits);
        write_conf_reg(dev, msg_data_addr + 8, msi_cap.pending_bits);
    }
}
