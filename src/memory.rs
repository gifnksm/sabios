use crate::{
    prelude::*,
    sync::{Mutex, MutexGuard},
};
use bootloader::boot_info::{MemoryRegion, MemoryRegionKind};
use core::cmp;
use x86_64::{
    structures::paging::{frame::PhysFrameRange, FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

const fn kib(kib: u64) -> u64 {
    kib * 1024
}
const fn mib(mib: u64) -> u64 {
    mib * kib(1024)
}
const fn gib(gib: u64) -> u64 {
    gib * mib(1024)
}

pub(crate) const BYTES_PER_FRAME: u64 = kib(4);
const MAX_PHYSICAL_MEMORY_BYTE: u64 = gib(128);
const FRAME_COUNT: u64 = MAX_PHYSICAL_MEMORY_BYTE / BYTES_PER_FRAME;

type MapLine = u64;
const BITS_PER_MAP_LINE: u64 = MapLine::BITS as u64;
const ALLOC_MAP_LEN: usize = (FRAME_COUNT / (BITS_PER_MAP_LINE as u64)) as usize;

pub(crate) struct BitmapMemoryManager {
    alloc_map: [MapLine; ALLOC_MAP_LEN],
    range: PhysFrameRange,
}

static MEMORY_MANAGER: Mutex<BitmapMemoryManager> = Mutex::new(BitmapMemoryManager {
    alloc_map: [0; ALLOC_MAP_LEN],
    range: PhysFrameRange {
        start: unsafe {
            PhysFrame::from_start_address_unchecked(PhysAddr::new_truncate(
                MAX_PHYSICAL_MEMORY_BYTE,
            ))
        },
        end: unsafe { PhysFrame::from_start_address_unchecked(PhysAddr::new_truncate(0)) },
    },
});

pub(crate) fn lock_memory_manager() -> MutexGuard<'static, BitmapMemoryManager> {
    MEMORY_MANAGER.lock()
}

impl BitmapMemoryManager {
    pub(crate) fn init(&mut self, regions: &[MemoryRegion]) -> Result<()> {
        let regions = MergedMemoryRegion::new(regions);
        let frame_size = 4096u64;

        let mut available_start = self.range.start;
        let mut available_end = self.range.end;
        for region in regions {
            let usable = region.kind == MemoryRegionKind::Usable;

            let start = PhysAddr::new(region.start);
            let end = PhysAddr::new(region.end);
            let (start, end) = if usable {
                (start.align_up(frame_size), end.align_down(frame_size))
            } else {
                (start.align_down(frame_size), end.align_up(frame_size))
            };
            if start >= end {
                continue;
            }

            let start = PhysFrame::from_start_address(start)?;
            let end = PhysFrame::from_start_address(end)?;

            if available_end < start {
                self.mark_allocated(PhysFrame::range(available_end, start));
            }

            if usable {
                available_start = cmp::min(available_start, start);
                available_end = cmp::max(available_end, end);
            } else {
                self.mark_allocated(PhysFrame::range(start, end));
            }
        }

        self.range = PhysFrame::range(available_start, available_end);
        Ok(())
    }

    pub(crate) fn mark_allocated(&mut self, range: PhysFrameRange) {
        for frame in range {
            self.set_bit(frame, true);
        }
        // update range for faster allocation
        if self.range.start == range.start {
            self.range.start = range.end;
            while self.range.start < self.range.end && self.get_bit(self.range.start) {
                self.range.start += 1;
            }
        }
    }

    // fn mark_freed(&mut self, range: PhysFrameRange) {
    //     for frame in range {
    //         self.set_bit(frame, false)
    //     }
    //     // update range if needed
    //     if self.range.start <= range.end {
    //         self.range.start = range.start;
    //     }
    // }

    pub(crate) fn allocate(&mut self, num_frames: usize) -> Result<PhysFrameRange> {
        let mut start_frame = self.range.start;
        loop {
            let end_frame = start_frame + num_frames as u64;
            if end_frame > self.range.end {
                bail!(ErrorKind::NoEnoughMemory);
            }

            let range = PhysFrame::range(start_frame, end_frame);
            if let Some(allocated) = range.clone().find(|frame| self.get_bit(*frame)) {
                start_frame = allocated + 1;
                continue;
            }

            self.mark_allocated(range);
            return Ok(range);
        }
    }

    // pub(crate) fn free(&mut self, range: PhysFrameRange) {
    //     for frame in range {
    //         self.set_bit(frame, false)
    //     }
    // }

    fn get_bit(&self, frame: PhysFrame) -> bool {
        let frame_index = frame.start_address().as_u64() / BYTES_PER_FRAME;
        let line_index = (frame_index / BITS_PER_MAP_LINE) as usize;
        let bit_index = frame_index % BITS_PER_MAP_LINE;

        (self.alloc_map[line_index] & (1 << bit_index)) != 0
    }

    fn set_bit(&mut self, frame: PhysFrame, allocated: bool) {
        let frame_index = frame.start_address().as_u64() / BYTES_PER_FRAME;
        let line_index = (frame_index / BITS_PER_MAP_LINE) as usize;
        let bit_index = frame_index % BITS_PER_MAP_LINE;

        if allocated {
            self.alloc_map[line_index] |= 1 << bit_index;
        } else {
            self.alloc_map[line_index] &= !(1 << bit_index);
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapMemoryManager {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.allocate(1).map(|range| range.start).ok()
    }
}

#[derive(Debug)]
struct MergedMemoryRegion<'a> {
    regions: core::slice::Iter<'a, MemoryRegion>,
}

impl<'a> MergedMemoryRegion<'a> {
    fn new(regions: &'a [MemoryRegion]) -> Self {
        let regions = regions.iter();
        Self { regions }
    }
}

impl<'a> Iterator for MergedMemoryRegion<'a> {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        let mut current = *self.regions.next()?;
        loop {
            #[allow(clippy::suspicious_operation_groupings)]
            match self.regions.as_slice().get(0) {
                Some(next) if current.kind == next.kind && current.end == next.start => {
                    current.end = next.end;
                    let _ = self.regions.next();
                    continue;
                }
                _ => return Some(current),
            }
        }
    }
}
