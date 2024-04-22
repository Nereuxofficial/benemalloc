//! This is a simple memory allocator written in Rust.
#![feature(c_size_t)]


use core::ffi::c_size_t;
use std::{alloc::GlobalAlloc, os::raw::c_void, ptr::null_mut, sync::Mutex};

use libc::{mmap, munmap, MAP_ANON, MAP_PRIVATE, PROT_READ, PROT_WRITE};

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
    free_array: [Option<Block>; 1024],
}

impl InternalState {
    const fn new() -> Self {
        Self {
            size: 0,
            free_array: [None; 1024],
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
        // TODO: Fix alignment. Right now we only have the right alignment accidentally
        let mut lock = self.internal_state.lock();
        let state_lock = lock.as_mut().unwrap();
        let freeblocks_size = state_lock.size;
        for i in 0..freeblocks_size {
            if let Some(block) = state_lock.free_array[i] {
                if block.size >= layout.size() {
                    // Place the last block at the current position
                    state_lock.free_array[i] = state_lock.free_array[freeblocks_size - 1];
                    state_lock.free_array[freeblocks_size - 1] = None;
                    state_lock.size -= 1;
                    return block.ptr;
                }
            }
        }
        let ret = malloc(layout.size());
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
        }else {
            munmap_size(ptr as *mut c_void, layout.size());
        }
    }
}

fn malloc(size: c_size_t) -> *mut c_void {
    let ptr = unsafe {
        // With the first argument being zero the kernel picks a page-aligned address to start
        // Then the size(for now is 1024). This is Read/Write Memory so we need those flags.
        // MAP_PRIVATE makes a copy-on-write mapping, where updates to the mapping are not visible to other processes.
        // MAP_ANON means it is not backed by a file, so fd is ignored, however some implementations want it to be -1 so it's -1
        // Offset is 0.
        mmap(
            null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            -1,
            0,
        )
    };
    ptr
}

/// **SAFETY**: ptr should be a valid pointer into a program allocated structure. size+ptr should never be larger than the allocation bound.
pub unsafe fn munmap_size(ptr: *mut c_void, size: c_size_t) -> i32 {
    munmap(ptr, size)
}
