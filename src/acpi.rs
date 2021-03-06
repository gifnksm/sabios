use crate::{memory, paging, prelude::*, sync::OnceCell};
use core::{mem, slice};
use x86_64::{instructions::port::PortReadOnly, structures::paging::OffsetPageTable, VirtAddr};

/// Root System Description Pointer
#[derive(Debug)]
#[repr(C)]
pub(crate) struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

impl Rsdp {
    fn is_valid(&self) -> bool {
        if self.signature != *b"RSD PTR " {
            warn!("invalid signature: {:?}", self.signature);
            return false;
        }
        if self.revision != 2 {
            warn!("ACPI revision must be 2: {}", self.revision);
            return false;
        }
        let sum = unsafe { sum_bytes(self, 20) };
        if sum != 0 {
            warn!("sum of 20 bytes must be 0: {}", sum);
            return false;
        }
        let sum = unsafe { sum_bytes(self, 36) };
        if sum != 0 {
            warn!("sum of 36 bytes must be 0: {}", sum);
            return false;
        }
        true
    }
}

#[derive(Debug)]
#[repr(C)]
struct DescriptionHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}
static_assertions::const_assert_eq!(mem::size_of::<DescriptionHeader>(), 36);

/// Extended System Descriptor Table
#[derive(Debug)]
#[repr(C)]
struct Xsdt {
    header: DescriptionHeader,
}

impl DescriptionHeader {
    fn is_valid(&self, expected_signature: &[u8]) -> bool {
        if self.signature != expected_signature {
            warn!("invalid signature: {:?}", self.signature);
            return false;
        }
        let sum = unsafe { sum_bytes(self, self.len()) };
        if sum != 0 {
            warn!("sum of {} bytes must be 0: {}", self.length, sum);
            return false;
        }
        true
    }

    fn len(&self) -> usize {
        self.length as usize
    }
}

impl Xsdt {
    fn len(&self) -> usize {
        (self.header.len() - mem::size_of::<DescriptionHeader>()) / mem::size_of::<usize>()
    }

    fn entries(&self) -> impl Iterator<Item = u64> {
        // `array_head` is not 8-byte aligned, so we cannot treat it as normal `*const u64`.
        // For example, `slice::from_raw_parts(array_head, len)` panics in debug build.
        let array_head =
            unsafe { (&self.header as *const DescriptionHeader).add(1) as *const [u8; 8] };
        (0..self.len()).map(move |idx| {
            let bytes = unsafe { array_head.add(idx).read() };
            u64::from_le_bytes(bytes)
        })
    }
}

/// Extended System Descriptor Table
#[derive(Debug)]
#[repr(C)]
struct Fadt {
    header: DescriptionHeader,
    reserved: [u8; 76 - mem::size_of::<DescriptionHeader>()],
    pm_tmr_blk: u32,
    reserved2: [u8; 112 - 80],
    flags: u32,
    reserved3: [u8; 276 - 116],
}

static FADT: OnceCell<&Fadt> = OnceCell::uninit();

/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// data has size larger than or equal to `bytes`.
unsafe fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    let data = data as *const T as *const u8;
    let data = unsafe { slice::from_raw_parts(data, bytes) };
    data.iter().copied().fold(0, u8::wrapping_add)
}

/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete RSDP is mapped to virtual memory at the passed `rsdp`.
pub(crate) unsafe fn init(mapper: &mut OffsetPageTable, rsdp: VirtAddr) -> Result<()> {
    debug!("RSDP: {:x}", rsdp.as_u64());
    map_page(mapper, rsdp)?;

    #[allow(clippy::unwrap_used)]
    let rsdp = unsafe { rsdp.as_ptr::<Rsdp>().as_ref() }.unwrap();
    if !rsdp.is_valid() {
        bail!(ErrorKind::InvalidRsdp);
    }

    let xsdt = VirtAddr::new(rsdp.xsdt_address);
    debug!("XSDT: {:x}", xsdt.as_u64());
    map_page(mapper, xsdt)?;

    #[allow(clippy::unwrap_used)]
    let xsdt = unsafe { xsdt.as_ptr::<Xsdt>().as_ref() }.unwrap();
    if !xsdt.header.is_valid(b"XSDT") {
        bail!(ErrorKind::InvalidXsdt);
    }

    #[allow(clippy::unwrap_used)]
    let fadt = xsdt
        .entries()
        .filter_map(|entry| {
            debug!("entry: {:x}", entry);
            map_page(mapper, VirtAddr::new(entry)).unwrap();
            unsafe { (entry as *const DescriptionHeader).as_ref() }
        })
        // FACP is the signature of FADT
        .find(|&entry| entry.is_valid(b"FACP"))
        .and_then(|entry| unsafe { (entry as *const DescriptionHeader as *const Fadt).as_ref() })
        .ok_or(ErrorKind::FadtNotFound)?;

    FADT.init_once(|| fadt);

    Ok(())
}

pub(crate) const PM_TIMER_FREQ: u32 = 3579545;

pub(crate) fn wait_milliseconds(msec: u32) {
    let fadt = FADT.get();
    let pm_timer_32 = ((fadt.flags >> 8) & 1) != 0;

    let mut port = PortReadOnly::<u32>::new(fadt.pm_tmr_blk as u16);
    let start = unsafe { port.read() };
    let mut end = start + PM_TIMER_FREQ * msec / 1000;
    if !pm_timer_32 {
        end &= 0x00ffffff;
    }

    if end < start {
        while unsafe { port.read() } >= start {}
    }
    while unsafe { port.read() } < end {}
}

fn map_page(mapper: &mut OffsetPageTable, addr: VirtAddr) -> Result<()> {
    let mut allocator = memory::lock_memory_manager();
    paging::make_identity_mapping(
        mapper,
        &mut *allocator,
        addr.align_down(4096u64).as_u64(),
        1,
    )?;
    Ok(())
}
