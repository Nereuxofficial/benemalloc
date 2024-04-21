//! This is a simple memory allocator written in Rust.
#![feature(c_size_t)]
use core::ffi::c_size_t;
use std::{alloc::GlobalAlloc, os::raw::c_void, ptr::null_mut};

use libc::{mmap, munmap, MAP_ANON, MAP_PRIVATE, PROT_READ, PROT_WRITE};

// Defines the bounds of a memory block. Rust says ptr is not Thread-safe, however since we are the allocator it should be.
struct Block {
    size: c_size_t,
    used: bool,
    ptr: *mut u8,
}

pub struct BeneAlloc;

unsafe impl GlobalAlloc for BeneAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        // We have to have an alignment of two according to Layout docs

        malloc(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        munmap_size(ptr as *mut c_void, layout.size());
    }
}

#[no_mangle]
pub extern "C" fn malloc(size: c_size_t) -> *mut c_void {
    let ptr = unsafe {
        // With the first argument being zero the kernel picks a page-aligned address to start
        // Then the size(for now is 1024). This is Read/Write Memory so we need those flags.
        // MAP_PRIVATE makes a copy-on-write mapping, where updates to the mapping are not visible to other processes.
        // MAP_ANON means it is not backed by a file, so fd is ignored, however some implementations want it to be -1 so it's -1
        // Offset is 0.
        mmap(
            null_mut(),
            1024,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            -1,
            0,
        )
    };
    ptr
}

/// **SAFETY**: ptr should be a valid pointer into a program allocated structure. size+ptr should never be larger than the allocation bound.
#[no_mangle]
pub unsafe fn munmap_size(ptr: *mut c_void, size: c_size_t) -> i32 {
    munmap(ptr, size)
}
