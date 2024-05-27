// TODO: Allow for tracking Allocation and Deallocation of memory

use std::alloc::Layout;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) struct Tracker {
    //FIXME: We cannot allocate, so maybe use atomic integers?
    allocations: AtomicU64,
    allocated_size: AtomicU64,
}

impl Tracker {
    pub const fn new() -> Self {
        Self {
            allocations: AtomicU64::new(0),
            allocated_size: AtomicU64::new(0),
        }
    }

    pub fn track_allocation(&mut self, layout: Layout) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.allocated_size
            .fetch_add(layout.size() as u64, Ordering::Relaxed);
    }

    pub fn track_deallocation(&mut self, layout: Layout) {}

    pub fn print(&self) {
        println!("Allocations: {:?}", self.allocations);
        println!("Allocated Size: {:?}", self.allocated_size);
    }
}
