//! A lightweight comparison of BeneMalloc and the system allocator.
//!
//! Run with:
//! `cargo +nightly run -p benemalloc-benches --example quick_comparison --release`

use benemalloc::BeneAlloc;
use std::alloc::{handle_alloc_error, GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::time::{Duration, Instant};

const SAMPLE_COUNT: usize = 7;
const SINGLE_SIZES: [usize; 5] = [32, 64, 256, 1024, 4096];
const BULK_COUNTS: [usize; 3] = [64, 512, 2048];
const BULK_SIZE: usize = 64;
const BULK_OPERATIONS_PER_SAMPLE: usize = 32_768;

static BENE_ALLOC: BeneAlloc = BeneAlloc::new();

#[derive(Clone, Copy)]
struct BulkSample {
    allocation: Duration,
    deallocation: Duration,
}

fn layout(size: usize) -> Layout {
    Layout::from_size_align(size, 8).expect("benchmark layout should be valid")
}

fn allocate<A: GlobalAlloc>(allocator: &A, layout: Layout) -> *mut u8 {
    // SAFETY: `layout` has a non-zero size and comes from `Layout::from_size_align`.
    let pointer = unsafe { allocator.alloc(layout) };
    if pointer.is_null() {
        handle_alloc_error(layout);
    }
    pointer
}

fn run_cycles<A: GlobalAlloc>(allocator: &A, layout: Layout, iterations: usize) {
    for _ in 0..iterations {
        let pointer = allocate(allocator, layout);
        black_box(pointer);

        // SAFETY: `pointer` was returned by `allocator` for this exact layout and
        // has not been deallocated yet.
        unsafe { allocator.dealloc(pointer, layout) };
    }
}

fn time_cycles<A: GlobalAlloc>(allocator: &A, layout: Layout, iterations: usize) -> Duration {
    let start = Instant::now();
    run_cycles(allocator, layout, iterations);
    start.elapsed()
}

fn compare_cycles(layout: Layout, iterations: usize) -> (Duration, Duration) {
    let warmup_iterations = (iterations / 10).max(1_000);
    run_cycles(&BENE_ALLOC, layout, warmup_iterations);
    run_cycles(&System, layout, warmup_iterations);

    let mut bene_samples = [Duration::ZERO; SAMPLE_COUNT];
    let mut system_samples = [Duration::ZERO; SAMPLE_COUNT];

    // Alternate the order to reduce bias from CPU frequency and background load.
    for sample in 0..SAMPLE_COUNT {
        if sample % 2 == 0 {
            bene_samples[sample] = time_cycles(&BENE_ALLOC, layout, iterations);
            system_samples[sample] = time_cycles(&System, layout, iterations);
        } else {
            system_samples[sample] = time_cycles(&System, layout, iterations);
            bene_samples[sample] = time_cycles(&BENE_ALLOC, layout, iterations);
        }
    }

    (median(bene_samples), median(system_samples))
}

fn time_bulk<A: GlobalAlloc>(
    allocator: &A,
    layout: Layout,
    count: usize,
    batches: usize,
) -> BulkSample {
    let mut pointers = Vec::with_capacity(count);
    let mut allocation = Duration::ZERO;
    let mut deallocation = Duration::ZERO;

    for _ in 0..batches {
        let start = Instant::now();
        for _ in 0..count {
            pointers.push(allocate(allocator, layout));
        }
        allocation += start.elapsed();

        let start = Instant::now();
        for &pointer in &pointers {
            // SAFETY: every pointer was returned by `allocator` for this exact
            // layout and each pointer is visited once in this batch.
            unsafe { allocator.dealloc(pointer, layout) };
        }
        deallocation += start.elapsed();
        pointers.clear();
    }

    BulkSample {
        allocation,
        deallocation,
    }
}

fn compare_bulk(count: usize) -> (BulkSample, BulkSample, usize) {
    let layout = layout(BULK_SIZE);
    let batches = (BULK_OPERATIONS_PER_SAMPLE / count).max(1);
    let operations = count * batches;

    time_bulk(&BENE_ALLOC, layout, count, 1);
    time_bulk(&System, layout, count, 1);

    let empty = BulkSample {
        allocation: Duration::ZERO,
        deallocation: Duration::ZERO,
    };
    let mut bene_samples = [empty; SAMPLE_COUNT];
    let mut system_samples = [empty; SAMPLE_COUNT];

    for sample in 0..SAMPLE_COUNT {
        if sample % 2 == 0 {
            bene_samples[sample] = time_bulk(&BENE_ALLOC, layout, count, batches);
            system_samples[sample] = time_bulk(&System, layout, count, batches);
        } else {
            system_samples[sample] = time_bulk(&System, layout, count, batches);
            bene_samples[sample] = time_bulk(&BENE_ALLOC, layout, count, batches);
        }
    }

    let bene = BulkSample {
        allocation: median(bene_samples.map(|sample| sample.allocation)),
        deallocation: median(bene_samples.map(|sample| sample.deallocation)),
    };
    let system = BulkSample {
        allocation: median(system_samples.map(|sample| sample.allocation)),
        deallocation: median(system_samples.map(|sample| sample.deallocation)),
    };

    (bene, system, operations)
}

fn median(mut samples: [Duration; SAMPLE_COUNT]) -> Duration {
    samples.sort_unstable();
    samples[SAMPLE_COUNT / 2]
}

fn ns_per_operation(duration: Duration, operations: usize) -> f64 {
    duration.as_secs_f64() * 1_000_000_000.0 / operations as f64
}

fn comparison(bene: f64, system: f64) -> String {
    let ratio = system / bene;
    if (0.99..=1.01).contains(&ratio) {
        "about equal".to_owned()
    } else if ratio > 1.0 {
        format!("{ratio:.2}x faster")
    } else {
        format!("{:.2}x slower", ratio.recip())
    }
}

fn print_row(label: &str, bene: f64, system: f64) {
    println!(
        "{label:<18} {:>15.2} {:>15.2} {:>18}",
        bene,
        system,
        comparison(bene, system),
    );
}

fn main() {
    println!("BeneMalloc quick comparison");
    println!("Median of {SAMPLE_COUNT} samples; lower is better.\n");

    if cfg!(debug_assertions) {
        eprintln!("Warning: this is a debug build; rerun with --release.\n");
    }

    println!("Hot allocation/deallocation cycles");
    println!(
        "{:<18} {:>15} {:>15} {:>18}",
        "Block size", "Bene (ns/op)", "System (ns/op)", "Bene vs system"
    );
    println!("{}", "-".repeat(70));

    for size in SINGLE_SIZES {
        let iterations = (16 * 1024 * 1024 / size).clamp(10_000, 500_000);
        let (bene, system) = compare_cycles(layout(size), iterations);
        print_row(
            &format!("{size} B"),
            ns_per_operation(bene, iterations),
            ns_per_operation(system, iterations),
        );
    }

    println!("\nBulk phases ({BULK_SIZE} B blocks)");
    println!(
        "{:<18} {:>15} {:>15} {:>18}",
        "Batch / phase", "Bene (ns/op)", "System (ns/op)", "Bene vs system"
    );
    println!("{}", "-".repeat(70));

    for count in BULK_COUNTS {
        let (bene, system, operations) = compare_bulk(count);
        print_row(
            &format!("{count} allocate"),
            ns_per_operation(bene.allocation, operations),
            ns_per_operation(system.allocation, operations),
        );
        print_row(
            &format!("{count} deallocate"),
            ns_per_operation(bene.deallocation, operations),
            ns_per_operation(system.deallocation, operations),
        );
    }

    println!("Use `cargo bench` for statistically rigorous measurements and reports.");
}
