//! A lightweight allocator comparison.
//!
//! Run with:
//! `cargo +nightly run -p benemalloc-benches --example quick_comparison --release`

use benemalloc::BeneAlloc;
use jemallocator::Jemalloc;
use mimalloc::MiMalloc;
use std::alloc::{handle_alloc_error, GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::time::{Duration, Instant};

const SAMPLE_COUNT: usize = 7;
const SINGLE_SIZES: [usize; 9] = [
    32,
    64,
    256,
    1024,
    4096,
    8192,
    16 * 1024,
    64 * 1024,
    1024 * 1024,
];
const BULK_SIZES: [usize; 3] = [64, 8 * 1024, 64 * 1024];
const BULK_COUNTS: [usize; 3] = [64, 512, 2048];
const BULK_MAX_LIVE_BYTES: usize = 16 * 1024 * 1024;
const BULK_BYTES_PER_SAMPLE: usize = 32 * 1024 * 1024;
const BULK_MAX_OPERATIONS_PER_SAMPLE: usize = 32_768;

static BENE_ALLOC: BeneAlloc = BeneAlloc::new();
static MIMALLOC: MiMalloc = MiMalloc;
static JEMALLOC: Jemalloc = Jemalloc;

#[derive(Clone, Copy)]
enum AllocatorKind {
    Bene,
    Mimalloc,
    Jemalloc,
    System,
}

const ALLOCATORS: [AllocatorKind; 4] = [
    AllocatorKind::Bene,
    AllocatorKind::Mimalloc,
    AllocatorKind::Jemalloc,
    AllocatorKind::System,
];

#[derive(Clone, Copy)]
struct AllocatorSamples<T> {
    bene: T,
    mimalloc: T,
    jemalloc: T,
    system: T,
}

impl AllocatorSamples<Duration> {
    const ZERO: Self = Self {
        bene: Duration::ZERO,
        mimalloc: Duration::ZERO,
        jemalloc: Duration::ZERO,
        system: Duration::ZERO,
    };

    fn record(&mut self, allocator: AllocatorKind, duration: Duration) {
        match allocator {
            AllocatorKind::Bene => self.bene = duration,
            AllocatorKind::Mimalloc => self.mimalloc = duration,
            AllocatorKind::Jemalloc => self.jemalloc = duration,
            AllocatorKind::System => self.system = duration,
        }
    }

    fn median(samples: [Self; SAMPLE_COUNT]) -> Self {
        Self {
            bene: median(samples.map(|sample| sample.bene)),
            mimalloc: median(samples.map(|sample| sample.mimalloc)),
            jemalloc: median(samples.map(|sample| sample.jemalloc)),
            system: median(samples.map(|sample| sample.system)),
        }
    }

    fn ns_per_operation(self, operations: usize) -> AllocatorSamples<f64> {
        AllocatorSamples {
            bene: ns_per_operation(self.bene, operations),
            mimalloc: ns_per_operation(self.mimalloc, operations),
            jemalloc: ns_per_operation(self.jemalloc, operations),
            system: ns_per_operation(self.system, operations),
        }
    }
}

#[derive(Clone, Copy)]
struct BulkSample {
    allocation: Duration,
    deallocation: Duration,
}

fn layout(size: usize) -> Layout {
    Layout::from_size_align(size, 8).expect("benchmark layout should be valid")
}

fn allocate<A: GlobalAlloc>(allocator: &A, layout: Layout) -> *mut u8 {
    // SAFETY: `layout` is created by `layout` above.
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

        // SAFETY: `pointer` was returned by `allocator` for this exact layout.
        unsafe { allocator.dealloc(pointer, layout) };
    }
}

fn time_cycles<A: GlobalAlloc>(allocator: &A, layout: Layout, iterations: usize) -> Duration {
    let start = Instant::now();
    run_cycles(allocator, layout, iterations);
    start.elapsed()
}

fn time_cycles_for(allocator: AllocatorKind, layout: Layout, iterations: usize) -> Duration {
    match allocator {
        AllocatorKind::Bene => time_cycles(&BENE_ALLOC, layout, iterations),
        AllocatorKind::Mimalloc => time_cycles(&MIMALLOC, layout, iterations),
        AllocatorKind::Jemalloc => time_cycles(&JEMALLOC, layout, iterations),
        AllocatorKind::System => time_cycles(&System, layout, iterations),
    }
}

fn compare_cycles(layout: Layout, iterations: usize) -> AllocatorSamples<Duration> {
    let warmup_iterations = (iterations / 10).max(1_000);
    for allocator in ALLOCATORS {
        run_cycles_for(allocator, layout, warmup_iterations);
    }

    let mut samples = [AllocatorSamples::ZERO; SAMPLE_COUNT];
    for sample in 0..SAMPLE_COUNT {
        // Rotate the execution order to reduce clock/frequency bias.
        for offset in 0..ALLOCATORS.len() {
            let allocator = ALLOCATORS[(sample + offset) % ALLOCATORS.len()];
            samples[sample].record(allocator, time_cycles_for(allocator, layout, iterations));
        }
    }
    AllocatorSamples::median(samples)
}

fn run_cycles_for(allocator: AllocatorKind, layout: Layout, iterations: usize) {
    match allocator {
        AllocatorKind::Bene => run_cycles(&BENE_ALLOC, layout, iterations),
        AllocatorKind::Mimalloc => run_cycles(&MIMALLOC, layout, iterations),
        AllocatorKind::Jemalloc => run_cycles(&JEMALLOC, layout, iterations),
        AllocatorKind::System => run_cycles(&System, layout, iterations),
    }
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
            // SAFETY: every pointer was returned by `allocator` for this layout.
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

fn time_bulk_for(
    allocator: AllocatorKind,
    layout: Layout,
    count: usize,
    batches: usize,
) -> BulkSample {
    match allocator {
        AllocatorKind::Bene => time_bulk(&BENE_ALLOC, layout, count, batches),
        AllocatorKind::Mimalloc => time_bulk(&MIMALLOC, layout, count, batches),
        AllocatorKind::Jemalloc => time_bulk(&JEMALLOC, layout, count, batches),
        AllocatorKind::System => time_bulk(&System, layout, count, batches),
    }
}

fn compare_bulk(
    size: usize,
    requested_count: usize,
) -> (AllocatorSamples<BulkSample>, usize, usize) {
    let layout = layout(size);
    let count = capped_bulk_count(size, requested_count);
    let target_operations =
        (BULK_BYTES_PER_SAMPLE / size).clamp(count, BULK_MAX_OPERATIONS_PER_SAMPLE);
    let batches = (target_operations + count - 1) / count;
    let operations = count * batches;

    for allocator in ALLOCATORS {
        time_bulk_for(allocator, layout, count, 1);
    }

    let mut allocation_samples = [AllocatorSamples::ZERO; SAMPLE_COUNT];
    let mut deallocation_samples = [AllocatorSamples::ZERO; SAMPLE_COUNT];
    for sample in 0..SAMPLE_COUNT {
        for offset in 0..ALLOCATORS.len() {
            let allocator = ALLOCATORS[(sample + offset) % ALLOCATORS.len()];
            let result = time_bulk_for(allocator, layout, count, batches);
            allocation_samples[sample].record(allocator, result.allocation);
            deallocation_samples[sample].record(allocator, result.deallocation);
        }
    }

    (
        AllocatorSamples {
            bene: BulkSample {
                allocation: median(allocation_samples.map(|sample| sample.bene)),
                deallocation: median(deallocation_samples.map(|sample| sample.bene)),
            },
            mimalloc: BulkSample {
                allocation: median(allocation_samples.map(|sample| sample.mimalloc)),
                deallocation: median(deallocation_samples.map(|sample| sample.mimalloc)),
            },
            jemalloc: BulkSample {
                allocation: median(allocation_samples.map(|sample| sample.jemalloc)),
                deallocation: median(deallocation_samples.map(|sample| sample.jemalloc)),
            },
            system: BulkSample {
                allocation: median(allocation_samples.map(|sample| sample.system)),
                deallocation: median(deallocation_samples.map(|sample| sample.system)),
            },
        },
        count,
        operations,
    )
}

fn capped_bulk_count(size: usize, requested_count: usize) -> usize {
    requested_count.min((BULK_MAX_LIVE_BYTES / size).max(1))
}

fn median(mut samples: [Duration; SAMPLE_COUNT]) -> Duration {
    samples.sort_unstable();
    samples[SAMPLE_COUNT / 2]
}

fn ns_per_operation(duration: Duration, operations: usize) -> f64 {
    duration.as_secs_f64() * 1_000_000_000.0 / operations as f64
}

fn print_header() {
    println!(
        "{:<24} {:>12} {:>12} {:>12} {:>12}",
        "Workload", "Bene", "mimalloc", "jemalloc", "System"
    );
    println!("{}", "-".repeat(76));
}

fn print_row(label: &str, samples: AllocatorSamples<f64>) {
    println!(
        "{label:<24} {:>10.2} ns {:>10.2} ns {:>10.2} ns {:>10.2} ns",
        samples.bene, samples.mimalloc, samples.jemalloc, samples.system,
    );
}

fn main() {
    println!("Allocator quick comparison");
    println!("Median of {SAMPLE_COUNT} samples; lower is better.\n");

    if cfg!(debug_assertions) {
        eprintln!("Warning: this is a debug build; rerun with --release.\n");
    }

    println!("Hot allocation/deallocation cycles");
    print_header();
    for size in SINGLE_SIZES {
        let iterations = (16 * 1024 * 1024 / size).clamp(1_000, 500_000);
        let samples = compare_cycles(layout(size), iterations).ns_per_operation(iterations);
        print_row(&format!("{size} B"), samples);
    }

    println!("\nBulk allocation and deallocation phases");
    print_header();
    for size in BULK_SIZES {
        let mut previous_count = None;
        for requested_count in BULK_COUNTS {
            let count = capped_bulk_count(size, requested_count);
            if previous_count == Some(count) {
                continue;
            }
            previous_count = Some(count);
            let (samples, count, operations) = compare_bulk(size, requested_count);
            let allocation = AllocatorSamples {
                bene: ns_per_operation(samples.bene.allocation, operations),
                mimalloc: ns_per_operation(samples.mimalloc.allocation, operations),
                jemalloc: ns_per_operation(samples.jemalloc.allocation, operations),
                system: ns_per_operation(samples.system.allocation, operations),
            };
            let deallocation = AllocatorSamples {
                bene: ns_per_operation(samples.bene.deallocation, operations),
                mimalloc: ns_per_operation(samples.mimalloc.deallocation, operations),
                jemalloc: ns_per_operation(samples.jemalloc.deallocation, operations),
                system: ns_per_operation(samples.system.deallocation, operations),
            };
            print_row(&format!("{size} B × {count} allocate"), allocation);
            print_row(&format!("{size} B × {count} deallocate"), deallocation);
        }
    }

    println!("\nUse `cargo bench` for statistically rigorous measurements and reports.");
}
