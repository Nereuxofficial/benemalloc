use libc::size_t;
use std::ffi::c_void;

#[cfg(windows)]
use windows::Win32::System::{Memory, SystemInformation};

#[cfg(unix)]
use libc::{mmap, munmap, MAP_ANON, MAP_PRIVATE, PROT_READ, PROT_WRITE};
#[cfg(unix)]
use std::ptr::null_mut;

#[cfg(unix)]
pub fn allocate(size: size_t) -> *mut c_void {
    unsafe {
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
    }
}
/// # Safety
/// ptr should be a valid pointer into a program allocated structure. size+ptr should never be larger than the allocation bound.
/// Furthermore ptr should no longer be stored as it is a dangling pointer after deallocation
#[cfg(unix)]
pub unsafe fn deallocate(ptr: *mut c_void, size: size_t) -> i32 {
    munmap(ptr, size)
}

#[cfg(windows)]
pub fn allocate(size: usize) -> *mut c_void {
    unsafe {
        let protection = Memory::PAGE_READWRITE;
        let flags = Memory::MEM_RESERVE | Memory::MEM_COMMIT;
        // https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc
        let address = Memory::VirtualAlloc(None, size, flags, protection);
        // In an allocator we shall return zero if the allocation failed. Panicing during an allocation is undefined behavior
        address
    }
}

/// # Safety
/// ptr should be a valid pointer into a program allocated structure. size+ptr should never be larger than the allocation bound.
/// Furthermore, ptr should no longer be stored as it is a dangling pointer after deallocation and using it is Use-After-Free
#[cfg(windows)]
pub unsafe fn deallocate(ptr: *mut c_void, size: size_t) -> i32 {
    // https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualfree
    // We can apparently SKIP the length which is really confusing...
    let length = 0;
    let flags = Memory::MEM_RELEASE;
    match Memory::VirtualFree(ptr, length, flags) {
        Ok(()) => 0,
        _ => -1,
    }
}
