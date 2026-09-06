use std::ptr::null_mut;

use allocations::{allocate, deallocate};

#[repr(C)]
struct LargeHeader {
    mapping_base: *mut u8,
    mapping_len: usize,
    // Same final prefix word as span allocations; null means standalone.
    span: *mut u8,
}

/// Allocates a large block of memory with metadata for freeing it.
pub fn allocate_large(size: usize, align: usize) -> *mut u8 {
    let header_size = size_of::<LargeHeader>();

    let align = align.max(align_of::<LargeHeader>());
    let Some(mapping_len) = size
        .checked_add(header_size)
        .and_then(|n| n.checked_add(align - 1))
    else {
        return null_mut();
    };

    let mapping_base = allocate(mapping_len) as *mut u8;
    if mapping_base.is_null() || mapping_base as usize == usize::MAX {
        return null_mut();
    }
    let start = unsafe { mapping_base.add(header_size) };
    let user = unsafe { start.add(start.align_offset(align)) };
    let header = unsafe { user.sub(header_size) as *mut LargeHeader };
    unsafe {
        header.write(LargeHeader {
            mapping_base,
            mapping_len,
            span: null_mut(),
        });
    }
    user
}

/// Unsafe because ANY allocation pointer passed to this HAS to be created via the corresponding [`allocate_large`] function.
pub unsafe fn free_large(user: *mut u8) {
    unsafe {
        let header_addr = user.sub(size_of::<LargeHeader>()) as *mut LargeHeader;
        let header = header_addr.read();
        deallocate(header.mapping_base.cast(), header.mapping_len);
    }
}
