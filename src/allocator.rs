use crate::{interrupt, prelude::*, sync::SpinMutex};
use core::{
    alloc::{GlobalAlloc, Layout},
    mem,
    ptr::{self, NonNull},
};
use x86_64::{
    instructions::interrupts,
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

#[global_allocator]
static ALLOCATOR: SpinMutex<FixedSizeBlockAllocator> =
    SpinMutex::new(FixedSizeBlockAllocator::new());

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 64 * 512 * 4096; // 128MiB

pub(crate) fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<()> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("allocation error {:?}", layout)
}

struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// The block sizes to use.
///
/// The sizes must each be power of 2 because they are also used as
/// the block alignment (alignments must be always powers of 2).
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// Choose an appropriate block size for the given layout.
///
/// Returns an index into the `BLOCK_SIZES` array.
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    /// Creates an empty `FixedSizeBlockAllocator`.
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        Self {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.fallback_allocator.init(heap_start, heap_size);
        }
    }

    /// Allocates using the fallback allocator.
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }

    /// Allocate memory as described by the given `layout`.
    ///
    /// Returns a pointer to newly-allocated memory,
    /// or null to indicate allocation failure.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure that `layout` has non-zero size.
    ///
    /// (Extension subtraits might provide more specific bounds on
    /// behavior, e.g., guarantee a sentinel address or a null pointer
    /// in response to a zero-size allocation request.)
    ///
    /// The allocated block of memory may or may not be initialized.
    ///
    /// # Errors
    ///
    /// Returning a null pointer indicates that either memory is exhausted
    /// or `layout` does not meet this allocator's size or alignment constraints.
    ///
    /// Implementations are encouraged to return null on memory
    /// exhaustion rather than aborting, but this is not
    /// a strict requirement. (Specifically: it is *legal* to
    /// implement this trait atop an underlying native allocation
    /// library that aborts on memory exhaustion.)
    ///
    /// Clients wishing to abort computation in response to an
    /// allocation error are encouraged to call the [`handle_alloc_error`] function,
    /// rather than directly invoking `panic!` or similar.
    ///
    /// [`handle_alloc_error`]: alloc::alloc::handle_alloc_error
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        match list_index(&layout) {
            Some(index) => {
                match self.list_heads[index].take() {
                    Some(node) => {
                        self.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    }
                    None => {
                        // no block exist in list => allocate new block
                        let block_size = BLOCK_SIZES[index];
                        // only works if all block sizes are power of 2
                        let block_align = block_size;
                        #[allow(clippy::unwrap_used)]
                        let layout = Layout::from_size_align(block_size, block_align).unwrap();
                        self.fallback_alloc(layout)
                    }
                }
            }
            None => self.fallback_alloc(layout),
        }
    }

    /// Deallocate the block of memory at the given `ptr` pointer with the given `layout`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because undefined behavior can result
    /// if the caller does not ensure all of the following:
    ///
    /// * `ptr` must denote a block of memory currently allocated via
    ///   this allocator,
    ///
    /// * `layout` must be the same layout that was used
    ///   to allocate that block of memory.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        match list_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: self.list_heads[index].take(),
                };
                // verify that block has size and alignment required for storing node
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);
                let new_node_ptr = ptr as *mut ListNode;
                unsafe {
                    new_node_ptr.write(new_node);
                    self.list_heads[index] = Some(&mut *new_node_ptr);
                }
            }
            None => {
                #[allow(clippy::unwrap_used)]
                let ptr = NonNull::new(ptr).unwrap();
                unsafe {
                    self.fallback_allocator.deallocate(ptr, layout);
                }
            }
        }
    }
}

unsafe impl GlobalAlloc for SpinMutex<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        assert!(!interrupt::is_interrupt_context());

        // Disable interrupts to prevent deadlocks.
        //
        // If a context switch occurs while another task is acquiring a lock,
        // and the task after the switch tries to acquire a lock with interrupts
        // disabled, a deadlock will occur. To prevent this deadlock, disable
        // interrupts before acquiring the lock.
        interrupts::without_interrupts(|| unsafe { self.lock().alloc(layout) })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        assert!(!interrupt::is_interrupt_context());

        // Disable interrupts to prevent deadlocks.
        //
        // If a context switch occurs while another task is acquiring a lock,
        // and the task after the switch tries to acquire a lock with interrupts
        // disabled, a deadlock will occur. To prevent this deadlock, disable
        // interrupts before acquiring the lock.
        interrupts::without_interrupts(|| unsafe { self.lock().dealloc(ptr, layout) })
    }
}
