//! Quick performance comparison between benemalloc and system allocator
//!
//! Run with: cargo run --example quick_comparison

use benemalloc::BeneAlloc;
use std::alloc::{GlobalAlloc, Layout};
use std::time::Instant;
use tracy_client::{span, Client};

static BENE_ALLOC: BeneAlloc = BeneAlloc::new();

fn layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size, align).unwrap()
}

fn time_allocator<A: GlobalAlloc>(allocator: &A, name: &str, iterations: usize, size: usize) {
    let layout = layout(size, 8);

    let start = Instant::now();

    for _ in 0..iterations {
        unsafe {
            let ptr = allocator.alloc(layout);
            if !ptr.is_null() {
                allocator.dealloc(ptr, layout);
            }
        }
    }

    let duration = start.elapsed();
    let ns_per_op = duration.as_nanos() as f64 / iterations as f64;

    println!(
        "{:12} | {:8} bytes | {:10} ops | {:8.2} ns/op | {:8.2} MB/s",
        name,
        size,
        iterations,
        ns_per_op,
        (size as f64 * iterations as f64) / (duration.as_secs_f64() * 1_000_000.0)
    );
}

fn bulk_test<A: GlobalAlloc>(allocator: &A, name: &str, count: usize, size: usize) {
    let layout = layout(size, 8);
    let mut ptrs = Vec::with_capacity(count);

    // Allocation phase
    span!("Allocation phase");
    let start = Instant::now();
    for _ in 0..count {
        unsafe {
            let ptr = allocator.alloc(layout);
            if !ptr.is_null() {
                ptrs.push(ptr);
            }
        }
    }
    let alloc_time = start.elapsed();

    // Deallocation phase
    span!("Deallocation phase");
    let start = Instant::now();
    for ptr in ptrs {
        unsafe {
            allocator.dealloc(ptr, layout);
        }
    }
    let dealloc_time = start.elapsed();

    println!(
        "{:12} | {:8} bytes | {:6} allocs | {:8.2} μs | {:8.2} μs",
        name,
        size,
        count,
        alloc_time.as_micros() as f64,
        dealloc_time.as_micros() as f64
    );
}

fn main() {
    println!("BeneMalloc vs System Allocator Performance Comparison");
    println!("=====================================================");

    println!("\n🚀 Single Allocation/Deallocation Performance:");
    println!("   Allocator   |   Size   |   Ops      |  ns/op   |   MB/s");
    println!("   ----------- | -------- | ---------- | -------- | --------");

    let client = Client::start();

    for &size in &[64, 256, 1024, 4096] {
        let iterations = 1_000_000 / (size / 64).max(1);
        span!("Measuring benemalloc", 100);
        time_allocator(&BENE_ALLOC, "benemalloc", iterations, size);
        span!("Measuring system allocator", 100);
        time_allocator(&std::alloc::System, "system", iterations, size);
        println!();
    }

    println!("\n📦 Bulk Allocation Performance (alloc then dealloc all):");
    println!("   Allocator   |   Size   | Count  | Alloc μs | Dealloc μs");
    println!("   ----------- | -------- | ------ | -------- | ----------");

    let bulk_tests = span!("Running bulk tests");
    for &count in &[100, 1000, 5000] {
        bulk_tests.emit_text("benemalloc");
        bulk_test(&BENE_ALLOC, "benemalloc", count, 64);
        bulk_tests.emit_text("system");
        bulk_test(&std::alloc::System, "system", count, 64);
        println!();
    }

    println!("\n🔄 Cache Behavior Test (should favor benemalloc):");
    println!("   Testing repeated small allocations...");

    let layout = layout(32, 8);
    let iterations = 10000;

    // BeneMalloc test
    let raw_allocs = span!("Testing raw fixed allocations", 100);
    raw_allocs.emit_text("benemalloc");
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let ptr = BENE_ALLOC.alloc(layout);
            if !ptr.is_null() {
                BENE_ALLOC.dealloc(ptr, layout);
            }
        }
    }
    let bene_time = start.elapsed();

    // System allocator test
    raw_allocs.emit_text("system");
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let ptr = std::alloc::System.alloc(layout);
            if !ptr.is_null() {
                std::alloc::System.dealloc(ptr, layout);
            }
        }
    }
    let system_time = start.elapsed();

    println!(
        "   benemalloc: {:.2} μs ({:.2} ns/op)",
        bene_time.as_micros() as f64,
        bene_time.as_nanos() as f64 / iterations as f64
    );
    println!(
        "   system:     {:.2} μs ({:.2} ns/op)",
        system_time.as_micros() as f64,
        system_time.as_nanos() as f64 / iterations as f64
    );

    let speedup = system_time.as_nanos() as f64 / bene_time.as_nanos() as f64;
    if speedup > 1.0 {
        println!("   ✅ benemalloc is {:.2}x faster!", speedup);
    } else {
        println!("   ⚠️  system allocator is {:.2}x faster", 1.0 / speedup);
    }

    println!("\n💡 Tips:");
    println!("   - Run with --release for accurate performance measurements");
    println!("   - benemalloc works best with repeated small allocations");
    println!("   - Cache size is limited to 512 blocks per thread");
    println!("   - Use 'cargo bench' for comprehensive benchmarks");
}
