//! This is a simple memory allocator written in Rust.
// TODO: Use mremap to grow memory allocations instead of reallocating them
// TODO: Make this work on stable, add stable to ci

mod large_allocs;
#[cfg(feature = "track_allocations")]
mod tracker;

use std::cell::UnsafeCell;
use std::cmp::max;
use std::ptr::null_mut;
use std::{alloc::GlobalAlloc, os::raw::c_void};

const STEPS: usize = 10;
const PAGE_SIZE: usize = 4096;
const MIN_SIZE_SHIFT: usize = 3;

#[cfg(feature = "debug")]
use std::alloc::Layout;

use allocations::{allocate, deallocate};

use crate::large_allocs::allocate_large;

#[cfg(not(target_os = "macos"))]
thread_local! {
    static CURRENT_THREAD_ALLOCATOR: UnsafeCell<InternalState> = const {UnsafeCell::new(InternalState::new()) };

    #[cfg(feature = "track_allocations")]
    static THREAD_TRACKER: UnsafeCell<tracker::Tracker> = const {UnsafeCell::new(tracker::Tracker::new()) };
}

// A segment in a page
#[repr(C)]
struct SegmentHeader {
    next: *mut SegmentHeader,
}

struct InternalState {
    // Size classes up to 2^STEPS
    // On deallocation a thread has to (somehow) contact every other thread to deallocate memory
    size_classes: [*mut SegmentHeader; STEPS],
}

impl InternalState {
    const fn new() -> Self {
        // The smallest size there can be is sizeof(SegmentHeader)
        Self {
            size_classes: [null_mut(); STEPS],
        }
    }

    #[inline]
    fn get_allocation(&mut self, size: usize, align: usize) -> *mut u8 {
        let index = Self::size_class(max(size, align));
        if index >= STEPS {
            let mut page = allocate_large(size, align);
            // `mmap` reports failure as MAP_FAILED (`(void *) -1`
            if page as usize == usize::MAX {
                page = null_mut();
            }
            return page as *mut u8;
        }

        let ptr = self.size_classes[index];
        // TODO: Simplify this
        if !ptr.is_null() {
            self.size_classes[index] = unsafe { (*ptr).next };
        } else {
            if self.fill_size_class(index) {
                let ptr = self.size_classes[index];
                self.size_classes[index] = unsafe { (*ptr).next };
                return ptr as *mut u8;
            } else {
                return null_mut();
            }
        }
        ptr as *mut u8
    }

    fn deallocate(&mut self, ptr: *mut SegmentHeader, size: usize, align: usize) {
        let index = Self::size_class(max(size, align));
        if index >= STEPS {
            // SAFETY: Because this is our allocation and it's not in a size class, we can safely deallocate
            // it as it must have been allocated with `allocate`.
            unsafe {
                return large_allocs::free_large(ptr as *mut u8);
            }
        }
        let next_ptr = self.size_classes[index];
        unsafe { (*ptr).next = next_ptr };
        self.size_classes[index] = ptr;
    }

    #[inline]
    fn size_class(size: usize) -> usize {
        let size = size.max(1usize << MIN_SIZE_SHIFT);

        let shift = usize::BITS as usize - (size - 1).leading_zeros() as usize;

        shift - MIN_SIZE_SHIFT
    }

    /// Adds one page of free segments to `index`.
    ///
    /// Returns `false` when the operating system could not supply a page.
    pub fn fill_size_class(&mut self, index: usize) -> bool {
        debug_assert!(index < STEPS);
        let page = allocate(PAGE_SIZE) as *mut u8;
        // `mmap` reports failure as MAP_FAILED (`(void *) -1`), while
        // VirtualAlloc reports it as null.
        if page.is_null() || page as usize == usize::MAX {
            return false;
        }

        // Split pages into segments and write a SegmentHeader at the start of each segment
        let segment_size = 1 << (MIN_SIZE_SHIFT + index);
        let count = PAGE_SIZE / segment_size;
        for i in 0..count {
            let segment = unsafe { page.add(i * segment_size) } as *mut SegmentHeader;
            let next_segment_ptr = if i + 1 < count {
                (unsafe { page.add((i + 1) * segment_size) }) as *mut SegmentHeader
            } else {
                self.size_classes[index]
            };
            unsafe { (*segment).next = next_segment_ptr };
        }
        self.size_classes[index] = page as *mut _;
        true
    }
}

pub struct BeneAlloc {
    #[cfg(feature = "debug")]
    pub allocations: [Option<Layout>; 4096],
}

unsafe impl Sync for BeneAlloc {}
unsafe impl Send for BeneAlloc {}

impl BeneAlloc {
    pub const fn new() -> Self {
        Self {
            #[cfg(feature = "debug")]
            allocations: [None; 4096],
        }
    }

    #[cfg(feature = "track_allocations")]
    pub fn print(&self) {
        let _ = THREAD_TRACKER.try_with(|tracker| unsafe {
            tracker.get().as_ref().unwrap().print();
        });
    }
}

unsafe impl GlobalAlloc for BeneAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        // Try to get a block from the cache
        match CURRENT_THREAD_ALLOCATOR.try_with(|state| unsafe {
            let state = &mut *state.get();
            state.get_allocation(layout.size(), layout.align())
        }) {
            Ok(ptr) => ptr,
            Err(_) => match allocate_large(layout.size(), layout.align()) {
                ptr if ptr as usize == usize::MAX => std::ptr::null_mut(),
                ptr => ptr,
            },
        }
    }

    /// The caller must ensure the ptr and layout are valid, so we do not have to keep track of
    /// how much memory was allocated for a given pointer. This helps us, because we do not have to
    /// modify the allocated list in other threads, which would require some kind of synchronization.
    /// Instead, we can add it to the local `free` list or deallocate it directly.
    ///
    /// # Safety
    /// The caller must ensure ptr and layout are valid. Additionally, the ptr may not be used after this function is called as any use would be UAF
    /// The caller must ensure the ptr was allocated by this allocator. Other allocators used(say for C libraries) do need to be deallocated by
    /// that allocator as to not corrupt this allocator's state
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        let _ = CURRENT_THREAD_ALLOCATOR.try_with(|state| {
            let state = unsafe { &mut *state.get() };
            state.deallocate(ptr as *mut SegmentHeader, layout.size(), layout.align());
        });
    }
}

#[cfg(test)]
mod fill_size_class_tests {
    use super::*;

    unsafe fn assert_page_chain(head: *mut SegmentHeader, segment_size: usize) {
        let count = PAGE_SIZE / segment_size;
        let mut current = head;

        for i in 0..count {
            let expected = unsafe { head.byte_add(i * segment_size).cast() };
            assert_eq!(current, expected);
            current = unsafe { (*current).next };
        }

        assert!(current.is_null());
    }

    #[test]
    fn fill_size_class_populates_every_segment_in_a_page() {
        for index in 0..STEPS {
            let mut state = InternalState::<512>::new();
            assert!(state.fill_size_class(index));

            let head = state.size_classes[index];
            assert!(!head.is_null());
            let segment_size = 1 << (MIN_SIZE_SHIFT + index);

            unsafe {
                assert_page_chain(head, segment_size);
                deallocate(head.cast(), PAGE_SIZE);
            }
        }
    }

    #[test]
    fn refill_prepends_a_page_without_losing_the_existing_free_list() {
        let index = 2;
        let segment_size = 1 << (MIN_SIZE_SHIFT + index);
        let count = PAGE_SIZE / segment_size;
        let mut state = InternalState::<512>::new();

        assert!(state.fill_size_class(index));
        let first_page = state.size_classes[index];
        assert!(state.fill_size_class(index));
        let second_page = state.size_classes[index];

        assert_ne!(second_page, first_page);
        unsafe {
            let mut current = second_page;
            for i in 0..count {
                assert_eq!(current, second_page.byte_add(i * segment_size).cast());
                current = (*current).next;
            }
            assert_eq!(current, first_page);
            assert_page_chain(first_page, segment_size);

            deallocate(first_page.cast(), PAGE_SIZE);
            deallocate(second_page.cast(), PAGE_SIZE);
        }
    }
}
