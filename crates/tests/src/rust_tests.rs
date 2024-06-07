use benemalloc::BeneAlloc;
use std::alloc::GlobalAlloc;
use std::fmt::Arguments;

#[test]
#[should_panic]
pub fn test_panic() {
    core::panicking::panic_fmt(Arguments::new_const(&["This is a test panicking"]));
}

#[test]
pub fn basic_alloc() {
    let mut allocator = BeneAlloc::new();
    let layout = std::alloc::Layout::from_size_align(8, 8).unwrap();
    let num = 1000;
    let mut allocations = vec![];
    for i in 0..num {
        let ptr = unsafe { allocator.alloc(layout) };
        assert!(!ptr.is_null());
        allocations.push(ptr);
    }
    for _ in 0..num / 2 {
        unsafe { allocator.dealloc(allocations.pop().unwrap(), layout) };
    }
    for _ in 0..num / 2 {
        let ptr = unsafe { allocator.alloc(layout) };
        assert!(!ptr.is_null());
        allocations.push(ptr);
    }
}
