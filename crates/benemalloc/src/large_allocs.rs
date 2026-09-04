use std::ptr::null_mut;

use allocations::{allocate, deallocate};

#[repr(C)]
struct LargeHeader {
    mapping_base: *mut u8,
    mapping_len: usize,
}

/// Allocates a large block of memory with metadata for freeing it.
pub fn allocate_large(size: usize, align: usize) -> *mut u8 {
    let header_size = size_of::<LargeHeader>();

    let mapping_len =
        // TODO: Handle unlikely overflows here
        unsafe {size.unchecked_add(header_size).unchecked_add(align - 1)};

    let mapping_base = allocate(mapping_len) as *mut u8;
    if mapping_base as usize == usize::MAX {
        return null_mut();
    }
    let user_addr =
        unsafe { (mapping_base.add(header_size).wrapping_add(align - 1)) as usize & !(align - 1) };
    let user = user_addr as *mut u8;
    let header = unsafe { user.sub(header_size) as *mut LargeHeader };
    unsafe {
        header.write(LargeHeader {
            mapping_base,
            mapping_len,
        });
    }
    user
}

/// Unsafe because ANY allocation pointer passed to this HAS to be created via the corresponding [`allocate_large`] function.
pub unsafe fn free_large(user: *mut u8) {
    let header_addr = user.sub(size_of::<LargeHeader>()) as *mut LargeHeader;
    let header = header_addr.read();

    deallocate(header.mapping_base.cast(), header.mapping_len);
}
