// TODO: Allow for tracking Allocation and Deallocation of memory



use std::alloc::Layout;

pub(crate) struct Tracker {
    //FIXME: We cannot allocate, so maybe use atomic integers?
    allocations: Vec<Layout>,
    deallocations: Vec<Layout>,
}

impl Tracker{
    pub const fn new() -> Self {
        Self {
            allocations: Vec::new(),
            deallocations: Vec::new(),
        }
    }

    pub fn track_allocation(&mut self, layout: Layout) {
        self.allocations.push(layout);
    }

    pub fn track_deallocation(&mut self, layout: Layout) {
        self.deallocations.push(layout);
    }

    pub fn print(&self) {
        println!("Allocations: {:?}", self.allocations);
        println!("Deallocations: {:?}", self.deallocations);
    }
}