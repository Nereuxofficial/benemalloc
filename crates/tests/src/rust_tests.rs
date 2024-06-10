use benemalloc::BeneAlloc;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::alloc::{Allocator, GlobalAlloc, Layout};
use std::fmt::Arguments;

#[test]
#[should_panic]
pub fn test_panic() {
    core::panicking::panic_fmt(Arguments::new_const(&["This is a test panicking"]));
}

#[test]
pub fn basic_alloc() {
    let mut allocator = BeneAlloc::new();
    let rng = &mut rand::thread_rng();
    let num = 1000;
    let mut allocations = vec![];
    for i in 0..num {
        let num: u16 = rng.gen();
        let num = num as usize;
        let layout = std::alloc::Layout::from_size_align(num, num).unwrap();
        let ptr = unsafe { allocator.alloc(layout) };
        assert!(!ptr.is_null());
        allocations.push((ptr, layout));
    }
    #[cfg(feature = "track_allocations")]
    allocator.print();
}

fn check_can_access(allocations: &Vec<(*mut u8, Layout)>) {
    for (ptr, layout) in allocations {
        let mut sum = 0;
        for i in 0..layout.size() {
            unsafe {
                sum += *ptr.add(i) as u8;
            }
        }
        assert!(sum != 0);
    }
}

fn dealloc<A: Allocator + GlobalAlloc>(allocator: &mut A, allocation: (*mut u8, Layout)) {
    unsafe { allocator.dealloc(allocation.0, allocation.1) };
}
