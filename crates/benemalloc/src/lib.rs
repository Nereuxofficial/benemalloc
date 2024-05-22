//! This is a simple memory allocator written in Rust.
// TODO: Use mremap to grow memory allocations instead of reallocating them
// TODO: Make this work on stable, add stable to ci
#![feature(c_size_t)]

#[cfg(feature = "track_allocations")]
mod tracker;

use core::ffi::c_size_t;
use std::cell::UnsafeCell;
use std::{alloc::GlobalAlloc, num::NonZeroUsize, os::raw::c_void, sync::Mutex};

use allocations::{allocate, deallocate};

#[cfg(not(target_os = "macos"))]
thread_local! {
    // TODO: Wrap in a UnsafeCell or Cell for mutable access
    static CURRENT_THREAD_ALLOCATOR: UnsafeCell<InternalState<512>> = const {UnsafeCell::new(InternalState::new()) };
}

// Defines the bounds of a memory block. Rust says ptr is not Thread-safe, however since we are the allocator it should be.
#[derive(Debug, Copy, Clone)]
struct Block {
    size: c_size_t,
    ptr: *mut u8,
}
unsafe impl Send for Block {}
unsafe impl Sync for Block {}

struct InternalState<const Size: usize> {
    size: usize,
    free_array: [Option<Block>; Size],
}

impl<const Size: usize> InternalState<Size> {
    const fn new() -> Self {
        Self {
            size: 0,
            free_array: [None; Size],
        }
    }
    fn insert(&mut self, block: Block) {
        self.free_array[self.size] = Some(block);
        self.size += 1;
    }
}

pub struct BeneAlloc {
}

impl BeneAlloc {
    pub const fn new() -> Self {
        Self {
        }
    }
}

unsafe impl GlobalAlloc for BeneAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let state = CURRENT_THREAD_ALLOCATOR.with(|state| unsafe { &mut *state.get() });
        let freeblocks_size = state.size;
        for i in 0..freeblocks_size {
            if let Some(block) = state.free_array[i] {
                // Since align must be a power of two and cannot be zero we can safely do new_unchecked
                // TODO: This is somehow slower according to mca as align is first converted to NonZero
                if block.size >= layout.size()
                    && (block.ptr as usize % NonZeroUsize::new_unchecked(layout.align()) == 0)
                {
                    let original_ptr = block.ptr;
                    if block.size > layout.size() {
                        // Split the block
                        let new_block = Block {
                            size: block.size - layout.size(),
                            ptr: block.ptr.add(layout.size()),
                        };
                        state.free_array[i] = Some(new_block);
                    } else {
                        // Place the last block at the current position
                        state.free_array[i] = state.free_array[freeblocks_size - 1];
                        state.free_array[freeblocks_size - 1] = None;
                        state.size -= 1;
                    }
                    debug_assert!(
                        original_ptr as usize % layout.align() == 0,
                        "Alignment error. ptr: {:p}, align: {}",
                        original_ptr,
                        layout.align()
                    );
                    return original_ptr;
                }
            }
        }
        let ret = allocate(layout.size());
        debug_assert!(ret as usize % layout.align() == 0);
        ret as *mut u8
    }

    /// The caller must ensure the ptr and layout are valid, so we do not have to keep track of
    /// how much memory was allocated for a given pointer. This helps us, because we do not have to
    /// modify the allocated list in other threads, which would require some kind of synchronization.
    /// Instead, we can add it to the local `free` list or deallocate it directly.
    ///
    /// # Safety
    /// The caller must ensure ptr and layout are valid. Additionally, the ptr may not be used after this function is called as any use would be UAF
    /// The caller must ensure the ptr was allocated by this allocator. This may actually be a larger
    /// limitation than originally thought, as memory allocated from outside Rust(like from C)
    /// will be allocated correctly, however the old allocator will not know about it and will still track it as
    /// used memory, which may cause double frees to not be detected
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        let mut state = CURRENT_THREAD_ALLOCATOR.with(|state| unsafe { &mut *state.get() });
        if state.size < state.free_array.len() {
            state.insert(Block {
                size: layout.size(),
                ptr,
            });
        } else {
            deallocate(ptr as *mut c_void, layout.size());
        }
    }
    // TODO: On windows alloc_zeroed initializes the memory to be zero so we could save performance by skipping directly to malloc if we need it...
}
