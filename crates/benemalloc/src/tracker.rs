// TODO: Allow for tracking Allocation and Deallocation of memory

struct Allocation {
    size: usize,
}

enum TrackedAllocation {
    Allocated(Block),
    Deallocated(Block),
}
