//! Integration tests for benemalloc benchmarks

use benemalloc::BeneAlloc;
use benemalloc_benches::{layout, AllocationPattern, COMMON_ALIGNMENTS, COMMON_SIZES};
use std::alloc::GlobalAlloc;

static TEST_ALLOCATOR: BeneAlloc = BeneAlloc::new();

#[test]
fn test_basic_allocation_patterns() {
    let patterns = vec![
        AllocationPattern::SmallBurst {
            count: 10,
            size: 64,
        },
        AllocationPattern::LargeBurst {
            count: 5,
            size: 1024,
        },
        AllocationPattern::Mixed {
            count: 20,
            min_size: 32,
            max_size: 512,
        },
        AllocationPattern::Alternating {
            count: 10,
            size: 128,
        },
    ];

    for pattern in patterns {
        unsafe {
            pattern.execute(&TEST_ALLOCATOR);
            pattern.execute(&std::alloc::System);
        }
    }
}

#[test]
fn test_layout_creation() {
    for &size in COMMON_SIZES {
        for &align in COMMON_ALIGNMENTS {
            if size >= align {
                let layout = layout(size, align);
                assert_eq!(layout.size(), size);
                assert_eq!(layout.align(), align);
            }
        }
    }
}

#[test]
fn test_allocator_basic_functionality() {
    let layout = layout(64, 8);

    unsafe {
        // Test basic allocation/deallocation
        let ptr = TEST_ALLOCATOR.alloc(layout);
        assert!(!ptr.is_null());

        // Write to the memory to ensure it's valid
        std::ptr::write(ptr, 42u8);
        let value = std::ptr::read(ptr);
        assert_eq!(value, 42);

        TEST_ALLOCATOR.dealloc(ptr, layout);
    }
}

#[test]
fn test_multiple_allocations() {
    let layout = layout(32, 8);
    let mut ptrs = Vec::new();

    unsafe {
        // Allocate multiple blocks
        for _ in 0..100 {
            let ptr = TEST_ALLOCATOR.alloc(layout);
            assert!(!ptr.is_null());
            ptrs.push(ptr);
        }

        // Deallocate all blocks
        for ptr in ptrs {
            TEST_ALLOCATOR.dealloc(ptr, layout);
        }
    }
}

#[test]
fn test_alignment_correctness() {
    for &align in COMMON_ALIGNMENTS {
        let size = align * 2; // Ensure size is at least as large as alignment
        let layout = layout(size, align);

        unsafe {
            let ptr = TEST_ALLOCATOR.alloc(layout);
            assert!(!ptr.is_null());
            assert_eq!(ptr as usize % align, 0, "Allocation not properly aligned");
            TEST_ALLOCATOR.dealloc(ptr, layout);
        }
    }
}

#[test]
fn test_various_sizes() {
    for &size in COMMON_SIZES {
        let layout = layout(size, 8);

        unsafe {
            let ptr = TEST_ALLOCATOR.alloc(layout);
            assert!(!ptr.is_null());

            // Write pattern to memory
            for i in 0..size {
                std::ptr::write(ptr.add(i), (i % 256) as u8);
            }

            // Verify pattern
            for i in 0..size {
                let value = std::ptr::read(ptr.add(i));
                assert_eq!(value, (i % 256) as u8);
            }

            TEST_ALLOCATOR.dealloc(ptr, layout);
        }
    }
}

#[test]
fn test_fragmentation_pattern() {
    let small_layout = layout(32, 8);
    let large_layout = layout(1024, 8);
    let mut small_ptrs = Vec::new();

    unsafe {
        // Allocate many small blocks
        for _ in 0..50 {
            let ptr = TEST_ALLOCATOR.alloc(small_layout);
            if !ptr.is_null() {
                small_ptrs.push(ptr);
            }
        }

        // Deallocate every other small block to create fragmentation
        for i in (0..small_ptrs.len()).step_by(2) {
            TEST_ALLOCATOR.dealloc(small_ptrs[i], small_layout);
        }

        // Try to allocate large blocks
        for _ in 0..5 {
            let ptr = TEST_ALLOCATOR.alloc(large_layout);
            if !ptr.is_null() {
                TEST_ALLOCATOR.dealloc(ptr, large_layout);
            }
        }

        // Clean up remaining small blocks
        for i in (1..small_ptrs.len()).step_by(2) {
            TEST_ALLOCATOR.dealloc(small_ptrs[i], small_layout);
        }
    }
}

#[test]
fn test_stress_allocation() {
    let layout = layout(64, 8);

    unsafe {
        // Rapid allocation/deallocation cycles
        for _ in 0..1000 {
            let ptr = TEST_ALLOCATOR.alloc(layout);
            if !ptr.is_null() {
                TEST_ALLOCATOR.dealloc(ptr, layout);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_thread_local_behavior() {
    use std::thread;

    let handles: Vec<_> = (0..4)
        .map(|_| {
            thread::spawn(|| {
                let layout = layout(128, 8);
                let mut ptrs = Vec::new();

                unsafe {
                    // Each thread does its own allocation pattern
                    for _ in 0..100 {
                        let ptr = TEST_ALLOCATOR.alloc(layout);
                        if !ptr.is_null() {
                            ptrs.push(ptr);
                        }
                    }

                    for ptr in ptrs {
                        TEST_ALLOCATOR.dealloc(ptr, layout);
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}
