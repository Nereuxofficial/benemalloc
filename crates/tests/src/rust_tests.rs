use benemalloc::BeneAlloc;
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use std::alloc::{Allocator, GlobalAlloc, Layout};
use std::fmt::Arguments;

#[test]
#[should_panic]
pub fn test_panic() {
    core::panicking::panic_fmt(Arguments::new_const(&["This is a test panicking"]));
}

#[test]
fn test_grow() {
    let mut allocator = BeneAlloc::new();
    let layout = Layout::from_size_align(1, 1).unwrap();
    let ptr = unsafe { allocator.alloc(layout) };
    assert!(!ptr.is_null());
    let ptr = unsafe { allocator.realloc(ptr, layout, 100) };
    assert!(!ptr.is_null());
    unsafe { allocator.dealloc(ptr, layout) };
}

fn check_can_access(allocations: &Vec<(*mut u8, Layout)>) {
    let mut rng = thread_rng();
    for (ptr, layout) in allocations {
        let mut nums = Vec::with_capacity(layout.size());
        nums.choose(&mut rng);
        unsafe {
            for i in 0..layout.size() {
                let mut num = *ptr.add(i);
                num = nums[i];
            }
        }
    }
}

fn dealloc<A: Allocator + GlobalAlloc>(allocator: &mut A, allocation: (*mut u8, Layout)) {
    unsafe { allocator.dealloc(allocation.0, allocation.1) };
}
