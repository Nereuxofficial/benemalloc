use benemalloc::BeneAlloc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::prelude::*;
use rand::rngs::SmallRng;
use std::alloc::{GlobalAlloc, Layout};

// Global allocator instances for testing
static BENE_ALLOC: BeneAlloc = BeneAlloc::new();

// Helper function to create layouts
fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).unwrap()
}

// Basic allocation benchmark
fn bench_basic_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_allocation");

    for size in [8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096].iter() {
        group.bench_with_input(BenchmarkId::new("bene_alloc", size), size, |b, &size| {
            b.iter(|| {
                let layout = layout(size, 8);
                let ptr = unsafe { BENE_ALLOC.alloc(layout) };
                if !ptr.is_null() {
                    unsafe { BENE_ALLOC.dealloc(ptr, layout) };
                }
                black_box(ptr);
            });
        });

        group.bench_with_input(BenchmarkId::new("system_alloc", size), size, |b, &size| {
            b.iter(|| {
                let layout = layout(size, 8);
                let ptr = unsafe { std::alloc::System.alloc(layout) };
                if !ptr.is_null() {
                    unsafe { std::alloc::System.dealloc(ptr, layout) };
                }
                black_box(ptr);
            });
        });
    }

    group.finish();
}

// Bulk allocation benchmark
fn bench_bulk_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_allocation");

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("bene_alloc", count), count, |b, &count| {
            b.iter(|| {
                let layout = layout(64, 8);
                let mut ptrs = Vec::with_capacity(count);

                // Allocate
                for _ in 0..count {
                    let ptr = unsafe { BENE_ALLOC.alloc(layout) };
                    if !ptr.is_null() {
                        ptrs.push(ptr);
                    }
                }

                // Deallocate
                for ptr in ptrs {
                    unsafe { BENE_ALLOC.dealloc(ptr, layout) };
                }
            });
        });

        group.bench_with_input(
            BenchmarkId::new("system_alloc", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let layout = layout(64, 8);
                    let mut ptrs = Vec::with_capacity(count);

                    // Allocate
                    for _ in 0..count {
                        let ptr = unsafe { std::alloc::System.alloc(layout) };
                        if !ptr.is_null() {
                            ptrs.push(ptr);
                        }
                    }

                    // Deallocate
                    for ptr in ptrs {
                        unsafe { std::alloc::System.dealloc(ptr, layout) };
                    }
                });
            },
        );
    }

    group.finish();
}

// Cache stress test - many small allocations
fn bench_cache_stress(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_stress");

    // Test with small allocations that should fit in cache
    group.bench_function("small_allocs_bene", |b| {
        b.iter(|| {
            let layout = layout(32, 8);
            let mut ptrs = Vec::with_capacity(600); // More than cache size (512)

            // Allocate more than cache can hold
            for _ in 0..600 {
                let ptr = unsafe { BENE_ALLOC.alloc(layout) };
                if !ptr.is_null() {
                    ptrs.push(ptr);
                }
            }

            // Deallocate in reverse order
            for ptr in ptrs.into_iter().rev() {
                unsafe { BENE_ALLOC.dealloc(ptr, layout) };
            }
        });
    });

    group.bench_function("small_allocs_system", |b| {
        b.iter(|| {
            let layout = layout(32, 8);
            let mut ptrs = Vec::with_capacity(600);

            for _ in 0..600 {
                let ptr = unsafe { std::alloc::System.alloc(layout) };
                if !ptr.is_null() {
                    ptrs.push(ptr);
                }
            }

            for ptr in ptrs.into_iter().rev() {
                unsafe { std::alloc::System.dealloc(ptr, layout) };
            }
        });
    });

    group.finish();
}

// Mixed allocation pattern
fn bench_mixed_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_pattern");

    group.bench_function("mixed_bene", |b| {
        let mut rng = SmallRng::from_entropy();
        b.iter(|| {
            let mut allocations = Vec::new();

            // Perform 1000 random operations
            for _ in 0..1000 {
                if allocations.is_empty() || rng.gen_bool(0.6) {
                    // Allocate
                    let size = 1 << rng.gen_range(3..=10); // 8 to 1024 bytes
                    let layout = layout(size, 8);
                    let ptr = unsafe { BENE_ALLOC.alloc(layout) };
                    if !ptr.is_null() {
                        allocations.push((ptr, layout));
                    }
                } else {
                    // Deallocate random allocation
                    if !allocations.is_empty() {
                        let idx = rng.gen_range(0..allocations.len());
                        let (ptr, layout) = allocations.swap_remove(idx);
                        unsafe { BENE_ALLOC.dealloc(ptr, layout) };
                    }
                }
            }

            // Clean up remaining allocations
            for (ptr, layout) in allocations {
                unsafe { BENE_ALLOC.dealloc(ptr, layout) };
            }
        });
    });

    group.bench_function("mixed_system", |b| {
        let mut rng = SmallRng::from_entropy();
        b.iter(|| {
            let mut allocations = Vec::new();

            for _ in 0..1000 {
                if allocations.is_empty() || rng.gen_bool(0.6) {
                    let size = 1 << rng.gen_range(3..=10);
                    let layout = layout(size, 8);
                    let ptr = unsafe { std::alloc::System.alloc(layout) };
                    if !ptr.is_null() {
                        allocations.push((ptr, layout));
                    }
                } else {
                    if !allocations.is_empty() {
                        let idx = rng.gen_range(0..allocations.len());
                        let (ptr, layout) = allocations.swap_remove(idx);
                        unsafe { std::alloc::System.dealloc(ptr, layout) };
                    }
                }
            }

            for (ptr, layout) in allocations {
                unsafe { std::alloc::System.dealloc(ptr, layout) };
            }
        });
    });

    group.finish();
}

// Alignment testing
fn bench_alignment(c: &mut Criterion) {
    let mut group = c.benchmark_group("alignment");

    for align in [8, 16, 32, 64, 128, 256].iter() {
        group.bench_with_input(BenchmarkId::new("bene_alloc", align), align, |b, &align| {
            b.iter(|| {
                let layout = layout(64, align);
                let ptr = unsafe { BENE_ALLOC.alloc(layout) };
                if !ptr.is_null() {
                    // Verify alignment
                    debug_assert_eq!(ptr as usize % align, 0);
                    unsafe { BENE_ALLOC.dealloc(ptr, layout) };
                }
                black_box(ptr);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("system_alloc", align),
            align,
            |b, &align| {
                b.iter(|| {
                    let layout = layout(64, align);
                    let ptr = unsafe { std::alloc::System.alloc(layout) };
                    if !ptr.is_null() {
                        debug_assert_eq!(ptr as usize % align, 0);
                        unsafe { std::alloc::System.dealloc(ptr, layout) };
                    }
                    black_box(ptr);
                });
            },
        );
    }

    group.finish();
}

// Fragmentation simulation
fn bench_fragmentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("fragmentation");

    group.bench_function("fragmentation_bene", |b| {
        b.iter(|| {
            let layout_small = layout(32, 8);
            let layout_large = layout(1024, 8);
            let mut small_ptrs = Vec::new();
            let mut large_ptrs = Vec::new();

            // Allocate many small blocks
            for _ in 0..100 {
                let ptr = unsafe { BENE_ALLOC.alloc(layout_small) };
                if !ptr.is_null() {
                    small_ptrs.push(ptr);
                }
            }

            // Deallocate every other small block to create fragmentation
            for i in (0..small_ptrs.len()).step_by(2) {
                unsafe { BENE_ALLOC.dealloc(small_ptrs[i], layout_small) };
            }

            // Try to allocate large blocks (should fail to use fragmented space)
            for _ in 0..10 {
                let ptr = unsafe { BENE_ALLOC.alloc(layout_large) };
                if !ptr.is_null() {
                    large_ptrs.push(ptr);
                }
            }

            // Clean up
            for i in (1..small_ptrs.len()).step_by(2) {
                unsafe { BENE_ALLOC.dealloc(small_ptrs[i], layout_small) };
            }
            for ptr in large_ptrs {
                unsafe { BENE_ALLOC.dealloc(ptr, layout_large) };
            }
        });
    });

    group.finish();
}

// Realistic workload simulation
fn bench_realistic_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_workload");

    group.bench_function("web_server_simulation", |b| {
        b.iter(|| {
            // Simulate web server allocation patterns
            let mut request_buffers = Vec::new();
            let mut response_buffers = Vec::new();

            // Process 100 "requests"
            for _ in 0..100 {
                // Allocate request buffer (various sizes)
                let req_size = rand::thread_rng().gen_range(128..=4096);
                let req_layout = layout(req_size, 8);
                let req_ptr = unsafe { BENE_ALLOC.alloc(req_layout) };
                if !req_ptr.is_null() {
                    request_buffers.push((req_ptr, req_layout));
                }

                // Allocate response buffer
                let resp_size = rand::thread_rng().gen_range(256..=8192);
                let resp_layout = layout(resp_size, 8);
                let resp_ptr = unsafe { BENE_ALLOC.alloc(resp_layout) };
                if !resp_ptr.is_null() {
                    response_buffers.push((resp_ptr, resp_layout));
                }

                // Occasionally clean up some old buffers
                if rand::thread_rng().gen_bool(0.3) && !request_buffers.is_empty() {
                    let (ptr, layout) = request_buffers.remove(0);
                    unsafe { BENE_ALLOC.dealloc(ptr, layout) };
                }
                if rand::thread_rng().gen_bool(0.3) && !response_buffers.is_empty() {
                    let (ptr, layout) = response_buffers.remove(0);
                    unsafe { BENE_ALLOC.dealloc(ptr, layout) };
                }
            }

            // Clean up remaining buffers
            for (ptr, layout) in request_buffers {
                unsafe { BENE_ALLOC.dealloc(ptr, layout) };
            }
            for (ptr, layout) in response_buffers {
                unsafe { BENE_ALLOC.dealloc(ptr, layout) };
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_basic_allocation,
    bench_bulk_allocation,
    bench_cache_stress,
    bench_mixed_pattern,
    bench_alignment,
    bench_fragmentation,
    bench_realistic_workload
);

criterion_main!(benches);
