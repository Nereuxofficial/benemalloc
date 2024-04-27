// TODO: Allow for tracking Allocation and Deallocation of memory

struct Allocation {
    size: usize,
}

enum TrakedAllocation {
    Allocated(Block),
    Deallocated(Block),
}
