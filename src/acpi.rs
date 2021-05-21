use crate::{memory, paging, prelude::*};
use core::{mem, slice};
use x86_64::{structures::paging::OffsetPageTable, VirtAddr};

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

/// Extended System Descriptor Table
#[derive(Debug)]
#[repr(C)]
struct Xsdt {
    header: DescriptionHeader,
    array_head: (),
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

    fn entries(&self) -> &[u64] {
        let array_head = &self.array_head as *const () as *const u64;
        unsafe { slice::from_raw_parts(array_head, self.len()) }
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

/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// data has size larger than or equal to `bytes`.
unsafe fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    let data = data as *const T as *const u8;
    let data = unsafe { slice::from_raw_parts(data, bytes) };
    data.iter().sum()
}

/// # Safety
///
/// This function is unsafe because the caller must guarantee that the
/// complete RSDP is mapped to virtual memory at the passed `rsdp`.
pub(crate) unsafe fn init(mapper: &mut OffsetPageTable, rsdp: VirtAddr) -> Result<&'static Rsdp> {
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

    let fadt = xsdt
        .entries()
        .iter()
        .copied()
        .filter_map(|entry| {
            debug!("entry: {:x}", entry);
            map_page(mapper, VirtAddr::new(entry)).unwrap();
            unsafe { (entry as *const DescriptionHeader).as_ref() }
        })
        // FACP is the signature of FADT
        .find(|&entry| entry.is_valid(b"FACP"))
        .and_then(|entry| unsafe { (entry as *const DescriptionHeader as *const Fadt).as_ref() })
        .ok_or(ErrorKind::FadtNotFound)?;

    Ok(rsdp)
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
