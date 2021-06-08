use crate::byte_getter;
use core::{fmt, mem, slice};
use enumflags2::{bitflags, make_bitflags, BitFlags};

#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileAttribute {
    ReadOnly = 0x01,
    Hidden = 0x02,
    System = 0x04,
    VolumeId = 0x08,
    Directory = 0x10,
    Archive = 0x20,
}
impl FileAttribute {
    pub(crate) const LFN: BitFlags<FileAttribute> =
        make_bitflags!(FileAttribute::{ReadOnly | Hidden | System | VolumeId});
}

#[repr(C)]
pub(crate) struct DirectoryEntry {
    name: [u8; 11],              // offset:  0 ([u8; 11])
    attr: [u8; 1],               // offset: 11 (u8)
    nt_res: [u8; 1],             // offset: 12 (u8)
    create_time_tenth: [u8; 1],  // offset: 13 (u8)
    create_time: [u8; 2],        // offset: 14 (u16)
    create_date: [u8; 2],        // offset: 16 (u16)
    last_access_date: [u8; 2],   // offset: 18 (u16)
    first_cluster_high: [u8; 2], // offset: 20 (u16)
    write_time: [u8; 2],         // offset: 22 (u16)
    write_date: [u8; 2],         // offset: 24 (u16)
    first_cluster_low: [u8; 2],  // offset: 26 (u16)
    file_size: [u8; 4],          // offset: 28 (u32)
}
static_assertions::const_assert_eq!(mem::size_of::<DirectoryEntry>(), 32);
static_assertions::const_assert_eq!(mem::align_of::<DirectoryEntry>(), 1);

impl DirectoryEntry {
    byte_getter!(name: [u8; 11]);
    pub(crate) fn attr(&self) -> BitFlags<FileAttribute> {
        BitFlags::from_bits_truncate(u8::from_le_bytes(self.attr))
    }
    byte_getter!(nt_res: u8);
    byte_getter!(create_time_tenth: u8);
    byte_getter!(create_time: u16);
    byte_getter!(create_date: u16);
    byte_getter!(last_access_date: u16);
    byte_getter!(first_cluster_high: u16);
    byte_getter!(write_time: u16);
    byte_getter!(write_date: u16);
    byte_getter!(first_cluster_low: u16);
    byte_getter!(file_size: u32);
}

impl fmt::Debug for DirectoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DirectoryEntry")
            .field("name", &self.name())
            .field("attr", &self.attr())
            .field("nt_res", &self.nt_res())
            .field("create_time_tenth", &self.create_time_tenth())
            .field("create_time", &self.create_time())
            .field("create_date", &self.create_date())
            .field("last_access_date", &self.last_access_date())
            .field("first_cluster_high", &self.first_cluster_high())
            .field("write_time", &self.write_time())
            .field("write_date", &self.write_date())
            .field("first_cluster_low", &self.first_cluster_low())
            .field("file_size", &self.file_size())
            .finish()
    }
}

fn trim_trailing(bytes: &[u8], byte: u8) -> &[u8] {
    let mut bytes = bytes;
    while let Some(stripped) = bytes.strip_suffix(&[byte]) {
        bytes = stripped;
    }
    bytes
}

impl DirectoryEntry {
    pub(crate) fn basename(&self) -> &[u8] {
        trim_trailing(&self.name[..8], 0x20)
    }

    pub(crate) fn extension(&self) -> &[u8] {
        trim_trailing(&self.name[8..], 0x20)
    }
}

#[derive(Debug)]
pub(crate) struct DirectoryEntries<'a> {
    iter: slice::Iter<'a, DirectoryEntry>,
}

impl<'a> DirectoryEntries<'a> {
    pub(super) fn new(entries: &'a [DirectoryEntry]) -> Self {
        Self {
            iter: entries.iter(),
        }
    }
}

impl<'a> Iterator for DirectoryEntries<'a> {
    type Item = &'a DirectoryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        for entry in self.iter.by_ref() {
            if entry.name[0] == 0x00 {
                // stop iteration
                self.iter = self.iter.as_slice()[..0].iter();
                break;
            }
            if entry.name[0] == 0xe5 {
                continue;
            }
            return Some(entry);
        }
        None
    }
}
