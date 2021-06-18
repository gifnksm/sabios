#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FatEntry {
    Unused,
    Reserved,
    Used(u32),
    UsedEof(u32),
    Bad,
}

impl FatEntry {
    pub(super) fn from_fat12(value: u16) -> Self {
        match value {
            0x000 => Self::Unused,
            0x001 => Self::Reserved,
            0x002..=0xff6 => Self::Used(u32::from(value)),
            0xff7 => Self::Bad,
            0xff8..=0xfff => Self::UsedEof(u32::from(value)),
            _ => Self::Bad,
        }
    }

    // fn to_fat12(&self) -> u16 {
    //     match self {
    //         FatEntry::Unused => 0x000,
    //         FatEntry::Reserved => 0x001,
    //         FatEntry::Used(value) => *value as u16,
    //         FatEntry::UsedEof(value) => *value as u16,
    //         FatEntry::Bad => 0xff7,
    //     }
    // }

    pub(super) fn from_fat16(value: u16) -> Self {
        match value {
            0x0000 => Self::Unused,
            0x0001 => Self::Reserved,
            0x0002..=0xfff6 => Self::Used(u32::from(value)),
            0xfff7 => Self::Bad,
            0xfff8..=0xffff => Self::UsedEof(u32::from(value)),
        }
    }

    // fn to_fat16(&self) -> u16 {
    //     match self {
    //         FatEntry::Unused => 0x0000,
    //         FatEntry::Reserved => 0x0001,
    //         FatEntry::Used(value) => *value as u16,
    //         FatEntry::UsedEof(value) => *value as u16,
    //         FatEntry::Bad => 0xfff7,
    //     }
    // }

    pub(super) fn from_fat32(value: u32) -> Self {
        match value {
            0x0000_0000 => Self::Unused,
            0x0000_0001 => Self::Reserved,
            0x0000_0002..=0x0fff_fff6 => Self::Used(value),
            0x0fff_fff7 => Self::Bad,
            0x0fff_fff8..=0x0fff_ffff => Self::UsedEof(value),
            _ => Self::Bad,
        }
    }

    // fn to_fat32(&self) -> u32 {
    //     match self {
    //         FatEntry::Unused => 0x0000_0000,
    //         FatEntry::Reserved => 0x0000_0001,
    //         FatEntry::Used(value) => *value,
    //         FatEntry::UsedEof(value) => *value,
    //         FatEntry::Bad => 0xffff_fff7,
    //     }
    // }
}
