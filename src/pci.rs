use crate::prelude::*;
use arrayvec::ArrayVec;
use bit_field::BitField;
use core::{fmt, ops::Range};
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
struct Config(spin::Mutex<PortSet>);

static CONFIG: Config = Config(spin::Mutex::new(PortSet {
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

    // fn write(&self, addr: Addr, data: u32) {
    //     let mut ports = self.0.lock();
    //     unsafe {
    //         ports.addr.write(addr.0);
    //         ports.data.write(data)
    //     }
    // }
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
    devices
        .try_push(Device {
            bus,
            device,
            function,
            vendor_id,
            class_code,
            header_type,
        })
        .map_err(|_| make_error!(ErrorKind::Full))?;

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

fn read_conf_reg(dev: &Device, reg_addr: u8) -> u32 {
    CONFIG.read(dev.addr(reg_addr))
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
