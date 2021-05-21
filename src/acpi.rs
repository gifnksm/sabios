use crate::{memory, paging, prelude::*};
use core::slice;
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
    {
        let mut allocator = memory::lock_memory_manager();
        paging::make_identity_mapping(mapper, &mut *allocator, rsdp.align_down(4096u64).as_u64(), 1)?;
    }

    #[allow(clippy::unwrap_used)]
    let rsdp = unsafe { rsdp.as_ptr::<Rsdp>().as_ref() }.unwrap();
    if !rsdp.is_valid() {
        bail!(ErrorKind::InvalidRsdp);
    }
    Ok(rsdp)
}
