use crate::{
    byte_getter,
    fmt::{ByteArray, ByteString},
};
use core::{
    convert::TryFrom,
    fmt::{self, DebugStruct},
    mem, slice,
};

use super::{DirectoryEntries, DirectoryEntry, FatType};

#[repr(C)]
pub(crate) struct BpbCommon {
    jump_boot: [u8; 3],             // offset:  0 ([u8; 3])
    oem_name: [u8; 8],              // offset:  3 ([u8; 8])
    bytes_per_sector: [u8; 2],      // offset: 11 (u16)
    sectors_per_cluster: [u8; 1],   // offset: 13 (u8)
    reserved_sector_count: [u8; 2], // offset: 14 (u16)
    num_fats: [u8; 1],              // offset: 16 (u8)
    root_entry_count: [u8; 2],      // offset: 17 (u16)
    total_sectors_16: [u8; 2],      // offset: 19 (u16)
    media: [u8; 1],                 // offset: 21 (u8)
    fat_size_16: [u8; 2],           // offset: 22 (u16)
    sectors_per_track: [u8; 2],     // offset: 24 (u16)
    num_heads: [u8; 2],             // offset: 26 (u16)
    hidden_sectors: [u8; 4],        // offset: 28 (u32)
    total_sectors_32: [u8; 4],      // offset: 32 (u32)
}
static_assertions::const_assert_eq!(mem::size_of::<BpbCommon>(), 36);
static_assertions::const_assert_eq!(mem::align_of::<BpbCommon>(), 1);

#[repr(C)]
struct BpbFat12_16 {
    common: BpbCommon,
    drive_number: [u8; 1],   // offset: 36 (u8)
    _reserved: [u8; 1],      // offset: 37 (u8)
    boot_signature: [u8; 1], // offset: 38 (u8)
    volume_id: [u8; 4],      // offset: 39 ([u8; 4])
    volume_label: [u8; 11],  // offset: 43 ([u8; 11])
    fs_type: [u8; 8],        // offset: 54 ([u8; 8])
    _boot_code: [u8; 448],   // offset: 62 ([u8; 448])
    boot_sign: [u8; 2],      // offset: 510 (u16)
}
static_assertions::const_assert_eq!(mem::size_of::<BpbFat12_16>(), 512);
static_assertions::const_assert_eq!(mem::align_of::<BpbFat12_16>(), 1);

#[repr(transparent)]
struct BpbFat12(BpbFat12_16);
#[repr(transparent)]
struct BpbFat16(BpbFat12_16);

#[repr(C)]
struct BpbFat32 {
    common: BpbCommon,
    fat_size_32: [u8; 4],        // offset: 36 (u32)
    ext_flags: [u8; 2],          // offset: 40 (u16)
    fs_version: [u8; 2],         // offset: 42 (u16)
    root_cluster: [u8; 4],       // offset: 44 (u32)
    fs_info: [u8; 2],            // offset: 48 (u16)
    backup_boot_sector: [u8; 2], // offset: 50 (u16)
    _reserved: [u8; 12],         // offset: 52
    drive_number: [u8; 1],       // offset: 64 (u8)
    _reserved1: [u8; 1],         // offset: 65
    boot_signature: [u8; 1],     // offset: 66 (u8)
    volume_id: [u8; 4],          // offset: 67 (u32)
    volume_label: [u8; 11],      // offset: 71 ([u8; 11])
    fs_type: [u8; 8],            // offset: 82 ([u8; 8])
    _boot_code32: [u8; 420],     // offset: 90 ([u8; 420])
    boot_sign: [u8; 2],          // offset: 510 (u16)
}
static_assertions::const_assert_eq!(mem::size_of::<BpbFat32>(), 512);
static_assertions::const_assert_eq!(mem::align_of::<BpbFat32>(), 1);

impl BpbCommon {
    byte_getter!(jump_boot: [u8; 3]);
    byte_getter!(oem_name: [u8; 8]);
    byte_getter!(bytes_per_sector: u16);
    byte_getter!(sectors_per_cluster: u8);
    byte_getter!(reserved_sector_count: u16);
    byte_getter!(num_fats: u8);
    byte_getter!(root_entry_count: u16);
    byte_getter!(total_sectors_16: u16);
    byte_getter!(media: u8);
    byte_getter!(fat_size_16: u16);
    byte_getter!(sectors_per_track: u16);
    byte_getter!(num_heads: u16);
    byte_getter!(hidden_sectors: u32);
    byte_getter!(total_sectors_32: u32);
}

impl BpbFat12_16 {
    byte_getter!(drive_number: u8);
    byte_getter!(boot_signature: u8);
    byte_getter!(volume_id: u32);
    byte_getter!(volume_label: [u8; 11]);
    byte_getter!(fs_type: [u8; 8]);
    byte_getter!(boot_sign: u16);
}

impl BpbFat32 {
    byte_getter!(fat_size_32: u32);
    byte_getter!(ext_flags: u16);
    byte_getter!(fs_version: u16);
    byte_getter!(root_cluster: u32);
    byte_getter!(fs_info: u16);
    byte_getter!(backup_boot_sector: u16);
    byte_getter!(drive_number: u8);
    byte_getter!(boot_signature: u8);
    byte_getter!(volume_id: u32);
    byte_getter!(volume_label: [u8; 11]);
    byte_getter!(fs_type: [u8; 8]);
    byte_getter!(boot_sign: u16);
}

impl BpbCommon {
    fn dump_fields(&self, f: &mut DebugStruct) {
        f.field("jump_boot", &ByteArray(&self.jump_boot()))
            .field("oem_name", &ByteString(&self.oem_name()))
            .field("bytes_per_sector", &self.bytes_per_sector())
            .field("sectors_per_cluster", &self.sectors_per_cluster())
            .field("reserved_sector_count", &self.reserved_sector_count())
            .field("num_fats", &self.num_fats())
            .field("root_entry_count", &self.root_entry_count())
            .field("total_sectors_16", &self.total_sectors_16())
            .field("media", &self.media())
            .field("fat_size_16", &self.fat_size_16())
            .field("sectors_per_track", &self.sectors_per_track())
            .field("num_heads", &self.num_heads())
            .field("hidden_sectors", &self.hidden_sectors())
            .field("total_sectors_32", &self.total_sectors_32());
    }
}

impl fmt::Debug for BpbCommon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("BpbCommon");
        self.dump_fields(&mut f);
        f.finish()
    }
}

impl BpbFat12_16 {
    fn dump_fields(&self, f: &mut DebugStruct) {
        self.common.dump_fields(f);
        f.field("drive_number", &self.drive_number())
            .field("boot_signature", &self.boot_signature())
            .field("volume_id", &self.volume_id())
            .field("volume_label", &ByteString(&self.volume_label()))
            .field("fs_type", &ByteString(&self.fs_type()))
            .field("boot_sign", &self.boot_sign());
    }
}

impl fmt::Debug for BpbFat12_16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("BpbFat12_16");
        self.dump_fields(&mut f);
        f.finish()
    }
}

impl fmt::Debug for BpbFat12 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("BpbFat12");
        self.0.dump_fields(&mut f);
        f.finish()
    }
}

impl fmt::Debug for BpbFat16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("BpbFat16");
        self.0.dump_fields(&mut f);
        f.finish()
    }
}

impl fmt::Debug for BpbFat32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("BpbFat32");
        self.common.dump_fields(&mut f);
        f.field("fat_size_32", &self.fat_size_32())
            .field("ext_flags", &self.ext_flags())
            .field("fs_version", &self.fs_version())
            .field("root_cluster", &self.root_cluster())
            .field("fs_info", &self.fs_info())
            .field("backup_boot_sector", &self.backup_boot_sector())
            .field("drive_number", &self.drive_number())
            .field("boot_signature", &self.boot_signature())
            .field("volume_id", &self.volume_id())
            .field("volume_label", &ByteString(&self.volume_label()))
            .field("fs_type", &ByteString(&self.fs_type()))
            .field("boot_sign", &self.boot_sign())
            .finish()
    }
}

pub(crate) trait BiosParameterBlock: fmt::Debug + Send {
    fn fat_type(&self) -> FatType;
    fn as_common(&self) -> &BpbCommon;
    fn fat_size(&self) -> u32;
    fn root_dir_entries(&self) -> DirectoryEntries;

    fn bytes_per_sector(&self) -> u16 {
        self.as_common().bytes_per_sector()
    }

    fn sectors_per_cluster(&self) -> u8 {
        self.as_common().sectors_per_cluster()
    }

    fn reserved_sector_count(&self) -> u16 {
        self.as_common().reserved_sector_count()
    }

    fn num_fats(&self) -> u8 {
        self.as_common().num_fats()
    }

    fn root_entry_count(&self) -> u16 {
        self.as_common().root_entry_count()
    }

    fn total_sectors(&self) -> u32 {
        let common = self.as_common();
        if common.total_sectors_16() != 0 {
            u32::from(common.total_sectors_16())
        } else {
            common.total_sectors_32()
        }
    }

    fn fat_start_sector(&self) -> u32 {
        u32::from(self.reserved_sector_count())
    }

    fn root_dir_start_sector_16(&self) -> u32 {
        let num_fats = u32::from(self.num_fats());
        let fat_size = self.fat_size();

        let fat_sectors = num_fats * fat_size;
        self.fat_start_sector() + fat_sectors
    }

    fn data_start_sector(&self) -> u32 {
        let root_entry_count = u32::from(self.root_entry_count());
        let bytes_per_sector = u32::from(self.bytes_per_sector());

        let root_dir_sectors = (32 * root_entry_count + bytes_per_sector - 1) / bytes_per_sector;
        self.root_dir_start_sector_16() + root_dir_sectors
    }

    fn cluster_sector(&self, cluster: u32) -> u32 {
        let cluster = cluster;
        let sectors_per_cluster = u32::from(self.sectors_per_cluster());

        self.data_start_sector() + (cluster - 2) * sectors_per_cluster
    }

    fn sector_offset(&self, sector: u32) -> u64 {
        let sector = u64::from(sector);
        let bytes_per_sector = u64::from(self.bytes_per_sector());
        sector * bytes_per_sector
    }

    fn sector_ptr(&self, sector: u32) -> *const u8 {
        assert!(sector < self.total_sectors());

        let bytes_per_sector = usize::from(self.bytes_per_sector());
        #[allow(clippy::unwrap_used)]
        let sector = usize::try_from(sector).unwrap();

        let count = sector * bytes_per_sector;
        unsafe { (self as *const Self as *const u8).add(count) }
    }
}
static_assertions::assert_obj_safe!(BiosParameterBlock);

impl BpbFat12_16 {
    fn fat_size(&self) -> u32 {
        u32::from(self.common.fat_size_16())
    }
}

impl BiosParameterBlock for BpbFat12 {
    fn fat_type(&self) -> FatType {
        FatType::Fat12
    }

    fn as_common(&self) -> &BpbCommon {
        &self.0.common
    }

    fn fat_size(&self) -> u32 {
        self.0.fat_size()
    }

    fn root_dir_entries(&self) -> DirectoryEntries {
        let root_dir_start_sector = self.root_dir_start_sector_16();
        let ptr = self
            .sector_ptr(root_dir_start_sector)
            .cast::<DirectoryEntry>();
        let root_entry_count = usize::from(self.root_entry_count());
        let entries = unsafe { slice::from_raw_parts(ptr, root_entry_count) };
        DirectoryEntries::new(entries)
    }
}

impl BiosParameterBlock for BpbFat16 {
    fn fat_type(&self) -> FatType {
        FatType::Fat16
    }

    fn as_common(&self) -> &BpbCommon {
        &self.0.common
    }

    fn fat_size(&self) -> u32 {
        self.0.fat_size()
    }

    fn root_dir_entries(&self) -> DirectoryEntries {
        let root_dir_start_sector = self.root_dir_start_sector_16();
        let ptr = self
            .sector_ptr(root_dir_start_sector)
            .cast::<DirectoryEntry>();
        let root_entry_count = usize::from(self.root_entry_count());
        let entries = unsafe { slice::from_raw_parts(ptr, root_entry_count) };
        DirectoryEntries::new(entries)
    }
}

impl BiosParameterBlock for BpbFat32 {
    fn fat_type(&self) -> FatType {
        FatType::Fat32
    }

    fn as_common(&self) -> &BpbCommon {
        &self.common
    }

    fn fat_size(&self) -> u32 {
        self.fat_size_32()
    }

    fn root_dir_entries(&self) -> DirectoryEntries {
        let bytes_per_sector = usize::from(self.bytes_per_sector());
        let sectors_per_cluster = usize::from(self.sectors_per_cluster());

        let root_dir_cluster = self.root_cluster();
        let root_dir_start_sector = self.cluster_sector(root_dir_cluster);
        let ptr = self
            .sector_ptr(root_dir_start_sector)
            .cast::<DirectoryEntry>();
        let root_entry_count =
            bytes_per_sector * sectors_per_cluster / mem::size_of::<DirectoryEntry>();

        let entries = unsafe { slice::from_raw_parts(ptr, root_entry_count) };
        DirectoryEntries::new(entries)
    }
}

unsafe fn detect_fat_type(fs: &u8) -> FatType {
    #[allow(clippy::unwrap_used)]
    let fs_common = unsafe { (fs as *const u8 as *const BpbCommon).as_ref().unwrap() };
    #[allow(clippy::unwrap_used)]
    let fs_fat32 = unsafe { (fs as *const u8 as *const BpbFat32).as_ref().unwrap() };

    let root_entry_count = u64::from(fs_common.root_entry_count());
    let bytes_per_sector = u64::from(fs_common.bytes_per_sector());
    let sector_per_cluster = u64::from(fs_common.sectors_per_cluster());

    // 32 : size of DirectoryEntry
    let root_dir_sectors = (root_entry_count * 32 + bytes_per_sector - 1) / bytes_per_sector;
    let fat_size = if fs_common.fat_size_16() != 0 {
        u64::from(fs_common.fat_size_16())
    } else {
        u64::from(fs_fat32.fat_size_32())
    };
    let total_sectors = if fs_common.total_sectors_16() != 0 {
        u64::from(fs_common.total_sectors_16())
    } else {
        u64::from(fs_common.total_sectors_32())
    };
    let reserved_sector_count = u64::from(fs_common.reserved_sector_count());
    let num_fats = u64::from(fs_common.num_fats());

    let data_sectors =
        total_sectors - (reserved_sector_count + num_fats * fat_size + root_dir_sectors);

    let count_of_clusters = data_sectors / sector_per_cluster;
    if count_of_clusters < 4085 {
        FatType::Fat12
    } else if count_of_clusters < 65525 {
        FatType::Fat16
    } else {
        FatType::Fat32
    }
}

pub(super) unsafe fn get(fs: &mut u8) -> &mut dyn BiosParameterBlock {
    #[allow(clippy::unwrap_used)]
    match unsafe { detect_fat_type(fs) } {
        FatType::Fat12 => unsafe { (fs as *mut u8 as *mut BpbFat12).as_mut().unwrap() as _ },
        FatType::Fat16 => unsafe { (fs as *mut u8 as *mut BpbFat16).as_mut().unwrap() as _ },
        FatType::Fat32 => unsafe { (fs as *mut u8 as *mut BpbFat32).as_mut().unwrap() as _ },
    }
}
