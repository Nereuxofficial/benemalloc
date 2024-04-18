//! This is a simple memory allocator written in Rust.
#![feature(c_size_t)]
use core::ffi::c_size_t;
use std::{os::raw::c_void, ptr::null_mut};

use libc::{mmap, MAP_ANON, MAP_PRIVATE, PROT_READ, PROT_WRITE};

struct Block {
    size: c_size_t,
    used: bool,
    ptr: *mut c_void,
}

#[no_mangle]
pub extern "C" fn malloc(size: c_size_t) -> *mut c_void {
    let ptr = unsafe {
        // With the first argument being zero the kernel picks a page-aligned address to start
        // Then the size(for now is 1024). This is Read/Write Memory so we need those flags.
        // MAP_PRIVATE makes a copy-on-write mapping, where updates to the mapping are not visible to other processes.
        // MAP_ANON means it is not backed by a file, so fd is ignored, however some implementations want it to be -1 so it's -1
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
