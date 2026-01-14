//! Benchmark utilities for benemalloc
//!
//! This crate provides comprehensive benchmarks for the benemalloc memory allocator,
//! comparing its performance against the system allocator across various allocation patterns.

use std::alloc::Layout;

/// Create a layout with the given size and alignment
///
/// # Panics
/// Panics if the layout is invalid
pub fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).expect("Invalid layout")
}

/// Common allocation sizes used in benchmarks
pub const COMMON_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

/// Common alignment values used in benchmarks
pub const COMMON_ALIGNMENTS: &[usize] = &[8, 16, 32, 64, 128, 256];

/// Generate a random allocation size within a reasonable range
pub fn random_size(min_exp: u32, max_exp: u32) -> usize {
    use rand::Rng;
    let exp = rand::thread_rng().gen_range(min_exp..=max_exp);
    1 << exp
}

/// Allocation pattern for testing different workloads
#[derive(Debug, Clone)]
pub enum AllocationPattern {
    /// Many small allocations
    SmallBurst { count: usize, size: usize },
    /// Few large allocations
    LargeBurst { count: usize, size: usize },
    /// Mixed allocation sizes
    Mixed {
        count: usize,
        min_size: usize,
        max_size: usize,
    },
    /// Alternating allocation and deallocation
    Alternating { count: usize, size: usize },
}

impl AllocationPattern {
    /// Execute the allocation pattern with the given allocator
    pub unsafe fn execute<A>(&self, allocator: &A)
    where
        A: std::alloc::GlobalAlloc,
    {
        match self {
            AllocationPattern::SmallBurst { count, size } => {
                let layout = layout(*size, 8);
                let mut ptrs = Vec::with_capacity(*count);

                for _ in 0..*count {
                    let ptr = allocator.alloc(layout);
                    if !ptr.is_null() {
                        ptrs.push(ptr);
                    }
                }

                for ptr in ptrs {
                    allocator.dealloc(ptr, layout);
                }
            }
            AllocationPattern::LargeBurst { count, size } => {
                let layout = layout(*size, 8);
                let mut ptrs = Vec::with_capacity(*count);

                for _ in 0..*count {
                    let ptr = allocator.alloc(layout);
                    if !ptr.is_null() {
                        ptrs.push(ptr);
                    }
                }

                for ptr in ptrs {
                    allocator.dealloc(ptr, layout);
                }
            }
            AllocationPattern::Mixed {
                count,
                min_size,
                max_size,
            } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let mut allocations = Vec::with_capacity(*count);

                for _ in 0..*count {
                    let size = rng.gen_range(*min_size..=*max_size);
                    let layout = layout(size, 8);
                    let ptr = allocator.alloc(layout);
                    if !ptr.is_null() {
                        allocations.push((ptr, layout));
                    }
                }

                for (ptr, layout) in allocations {
                    allocator.dealloc(ptr, layout);
                }
            }
            AllocationPattern::Alternating { count, size } => {
                let layout = layout(*size, 8);

                for _ in 0..*count {
                    let ptr = allocator.alloc(layout);
                    if !ptr.is_null() {
                        allocator.dealloc(ptr, layout);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_creation() {
        let layout = layout(64, 8);
        assert_eq!(layout.size(), 64);
        assert_eq!(layout.align(), 8);
    }

    #[test]
    fn test_random_size() {
        let size = random_size(3, 10);
        assert!(size >= 8 && size <= 1024);
        assert!(size.is_power_of_two());
    }

    #[test]
    fn test_allocation_pattern() {
        let pattern = AllocationPattern::SmallBurst {
            count: 10,
            size: 64,
        };
        unsafe {
            pattern.execute(&std::alloc::System);
        }
    }
}
