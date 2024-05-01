//! This is a simple memory allocator written in Rust.
// TODO: Use mremap to grow memory allocations instead of reallocating them
#![feature(c_size_t)]

#[cfg(feature = "track_allocations")]
mod tracker;

use core::ffi::c_size_t;
use std::{alloc::GlobalAlloc, num::NonZeroUsize, os::raw::c_void, ptr::null_mut, sync::Mutex};

use allocations::{allocate, deallocate};

// Defines the bounds of a memory block. Rust says ptr is not Thread-safe, however since we are the allocator it should be.
#[derive(Debug, Copy, Clone)]
struct Block {
    size: c_size_t,
    ptr: *mut u8,
}
unsafe impl Send for Block {}
unsafe impl Sync for Block {}

struct InternalState {
    size: usize,
    // TODO: Maybe we could use skip lists to make this more efficient. Also: Thread-local storage for more efficient freeing?
    free_array: [Option<Block>; 2048],
}

impl InternalState {
    const fn new() -> Self {
        Self {
            size: 0,
            free_array: [None; 2048],
        }
    }
    fn insert(&mut self, block: Block) {
        self.free_array[self.size] = Some(block);
        self.size += 1;
    }
}

pub struct BeneAlloc {
    internal_state: Mutex<InternalState>,
}

impl BeneAlloc {
    pub const fn new() -> Self {
        Self {
            internal_state: Mutex::new(InternalState::new()),
        }
    }
}

unsafe impl GlobalAlloc for BeneAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let mut lock = self.internal_state.lock();
        let state_lock = lock.as_mut();
        if state_lock.is_err() {
            return null_mut();
        }
        let state_lock = state_lock.unwrap_unchecked();
        let freeblocks_size = state_lock.size;
        for i in 0..freeblocks_size {
            if let Some(block) = state_lock.free_array[i] {
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
                        state_lock.free_array[i] = Some(new_block);
                    } else {
                        // Place the last block at the current position
                        state_lock.free_array[i] = state_lock.free_array[freeblocks_size - 1];
                        state_lock.free_array[freeblocks_size - 1] = None;
                        state_lock.size -= 1;
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

    // The caller must ensure the ptr and layout are valid, so we do not have to keep track of
    // how much memory was allocated for a given pointer
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        let mut lock = self.internal_state.lock();
        let state_lock = lock.as_mut().unwrap();
        if state_lock.size < state_lock.free_array.len() {
            state_lock.insert(Block {
                size: layout.size(),
                ptr,
            });
        } else {
            deallocate(ptr as *mut c_void, layout.size());
        }
    }
    // TODO: On windows alloc_zeroed initializes the memory to be zero so we could save performance by skipping directly to malloc if we need it...
}


