//! This is a simple memory allocator written in Rust.
// TODO: Use mremap to grow memory allocations instead of reallocating them
// TODO: Make this work on stable, add stable to ci

#[cfg(feature = "track_allocations")]
mod tracker;

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{alloc::GlobalAlloc, num::NonZeroUsize, os::raw::c_void};

// Global flag to disable thread-local caching when process is in unstable state
static GLOBAL_CACHE_ENABLED: AtomicBool = AtomicBool::new(true);

#[cfg(feature = "debug")]
use std::alloc::Layout;

use allocations::{allocate, deallocate};

#[cfg(not(target_os = "macos"))]
thread_local! {
    static CURRENT_THREAD_ALLOCATOR: UnsafeCell<InternalState<512>> = const {UnsafeCell::new(InternalState::new()) };

    #[cfg(feature = "track_allocations")]
    static THREAD_TRACKER: UnsafeCell<tracker::Tracker> = const {UnsafeCell::new(tracker::Tracker::new()) };
}

#[cfg(target_os = "macos")]
thread_local! {
    static CURRENT_THREAD_ALLOCATOR: UnsafeCell<InternalState<512>> = const {UnsafeCell::new(InternalState::new()) };

    #[cfg(feature = "track_allocations")]
    static THREAD_TRACKER: UnsafeCell<tracker::Tracker> = const {UnsafeCell::new(tracker::Tracker::new()) };
}

// Defines the bounds of a memory block. Rust says ptr is not Thread-safe, however since we are the allocator it should be.
#[derive(Debug, Copy, Clone)]
struct Block {
    size: usize,
    ptr: *mut u8,
}
unsafe impl Send for Block {}
unsafe impl Sync for Block {}

struct InternalState<const SIZE: usize> {
    size: usize,
    // TODO: The elements should not be Option<Block> but a union since we track the size manually
    free_array: [Option<Block>; SIZE],
}

impl<const SIZE: usize> InternalState<SIZE> {
    const fn new() -> Self {
        Self {
            size: 0,
            free_array: [None; SIZE],
        }
    }
    fn insert(&mut self, block: Block) {
        self.free_array[self.size] = Some(block);
        self.size += 1;
    }

    fn get_fitting_index(&self, size: usize, align: NonZeroUsize) -> Option<usize> {
        let freeblocks_size = self.size;
        for i in 0..freeblocks_size {
            if let Some(block) = self.free_array[i] {
                // Since align must be a power of two and cannot be zero we can safely do new_unchecked
                // TODO: This is somehow slower according to mca as align is first converted to NonZero
                if block.size >= size && (block.ptr as usize % align) == 0 {
                    return Some(i);
                }
            }
        }
        None
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
        // During panic unwinding, bypass thread-local cache to avoid issues
        if std::thread::panicking() {
            return allocate(layout.size()) as *mut u8;
        }

        // Check global cache flag - if disabled, use system allocator
        if !GLOBAL_CACHE_ENABLED.load(Ordering::Relaxed) {
            return allocate(layout.size()) as *mut u8;
        }

        // Try to get a block from the cache
        let result = CURRENT_THREAD_ALLOCATOR.try_with(|state| unsafe {
            let state = &mut *state.get();
            let freeblocks_size = state.size;
            let _size = layout.size();
            // The trait guarantees us that align is not zero, so this is safe.
            let align = NonZeroUsize::new_unchecked(layout.align());

            for i in 0..freeblocks_size {
                if let Some(block) = state.free_array[i] {
                    // Check if block is suitable: large enough and properly aligned
                    if block.size >= layout.size() && (block.ptr as usize % align.get()) == 0 {
                        let original_ptr = block.ptr;

                        // Remove this block from the free list
                        // Place the last block at the current position
                        state.free_array[i] = state.free_array[freeblocks_size.saturating_sub(1)];
                        state.free_array[freeblocks_size.saturating_sub(1)] = None;
                        state.size -= 1;

                        debug_assert!(
                            original_ptr as usize % layout.align() == 0,
                            "Alignment error. ptr: {:?}, align: {}",
                            original_ptr,
                            layout.align()
                        );

                        #[cfg(feature = "track_allocations")]
                        {
                            use crate::tracker::Action;
                            use crate::tracker::Event;
                            let _ = THREAD_TRACKER.try_with(|tracker| {
                                let tracker = &mut *tracker.get();
                                tracker.track(Event::Alloc {
                                    addr: original_ptr as usize,
                                    size: layout.size() as usize,
                                    source: Action::Cache,
                                });
                            });
                        }
                        return Some(original_ptr as *mut u8);
                    }
                }
            }
            None
        });

        match result {
            Ok(Some(ptr)) => ptr,
            Ok(None) | Err(_) => {
                // No suitable block in cache or thread-local unavailable, allocate from system
                if result.is_err() {
                    // Thread-local access failed, disable cache globally
                    GLOBAL_CACHE_ENABLED.store(false, Ordering::Relaxed);
                }

                let ret = allocate(layout.size());
                debug_assert!(ret as usize % layout.align() == 0);
                #[cfg(feature = "track_allocations")]
                {
                    use crate::tracker::Action;
                    use crate::tracker::Event;
                    let _ = THREAD_TRACKER.try_with(|tracker| {
                        let tracker = &mut *tracker.get();
                        tracker.track(Event::Alloc {
                            addr: ret as usize,
                            size: layout.size() as usize,
                            source: Action::System,
                        });
                    });
                }
                ret as *mut u8
            }
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
        // During panic unwinding, bypass thread-local cache to avoid issues
        if std::thread::panicking() {
            deallocate(ptr as *mut c_void, layout.size());
            return;
        }

        // Check global cache flag - if disabled, use system allocator
        if !GLOBAL_CACHE_ENABLED.load(Ordering::Relaxed) {
            deallocate(ptr as *mut c_void, layout.size());
            return;
        }

        let result = CURRENT_THREAD_ALLOCATOR.try_with(|state| unsafe {
            let state = &mut *state.get();
            if state.size < state.free_array.len() {
                #[cfg(feature = "track_allocations")]
                {
                    use crate::tracker::Action;
                    let _ = THREAD_TRACKER.try_with(|tracker| {
                        let tracker = &mut *tracker.get();
                        tracker.track(tracker::Event::Free {
                            addr: ptr as usize,
                            size: layout.size() as usize,
                            action: Action::Cache,
                        });
                    });
                }
                state.insert(Block {
                    size: layout.size(),
                    ptr,
                });
                true // Cached
            } else {
                false // Need to deallocate
            }
        });

        match result {
            Ok(true) => {
                // Successfully cached in free list
            }
            Ok(false) => {
                // Free list is full, deallocate via system
                #[cfg(feature = "track_allocations")]
                {
                    use crate::tracker::Action;
                    let _ = THREAD_TRACKER.try_with(|tracker| {
                        let tracker = &mut *tracker.get();
                        tracker.track(tracker::Event::Free {
                            addr: ptr as usize,
                            size: layout.size() as usize,
                            action: Action::System,
                        });
                    });
                }
                deallocate(ptr as *mut c_void, layout.size());
            }
            Err(_) => {
                // Thread-local is being destroyed, disable cache globally and fallback to system
                GLOBAL_CACHE_ENABLED.store(false, Ordering::Relaxed);
                deallocate(ptr as *mut c_void, layout.size());
            }
        }
    }
    // TODO: On windows alloc_zeroed initializes the memory to be zero so we could save performance by skipping directly to malloc if we need it...
}
