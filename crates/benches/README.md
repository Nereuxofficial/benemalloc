# BeneMalloc Benchmarks

This crate provides comprehensive benchmarks for the `benemalloc` memory allocator, comparing its performance against the system allocator across various allocation patterns.

## Overview

The benchmark suite tests `benemalloc` against different allocation scenarios to evaluate:

- **Allocation Speed**: How fast allocations and deallocations are performed
- **Cache Efficiency**: How well the allocator utilizes its internal free block cache
- **Fragmentation Handling**: How the allocator performs under fragmented memory conditions
- **Real-world Workloads**: Performance in realistic usage patterns

## Running Benchmarks

### Prerequisites

- Rust nightly toolchain (required for `benemalloc`)
- Criterion benchmarking framework

### Basic Usage

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench -- basic_allocation

# Run with HTML reports
cargo bench --features html_reports

# Run benchmarks and open results
cargo bench && open target/criterion/report/index.html
```

### Benchmark Configuration

The benchmarks can be configured through environment variables:

```bash
# Set sample size (default: 100)
CRITERION_SAMPLE_SIZE=1000 cargo bench

# Set measurement time (default: 5 seconds)
CRITERION_MEASUREMENT_TIME=10 cargo bench
```

## Benchmark Categories

### 1. Basic Allocation (`bench_basic_allocation`)

Tests fundamental allocation and deallocation performance across various sizes:
- Sizes: 8B to 4KB
- Compares `benemalloc` vs system allocator
- Measures single allocation/deallocation cycles

### 2. Bulk Allocation (`bench_bulk_allocation`)

Tests performance when allocating many blocks at once:
- Batch sizes: 10, 100, 1000 allocations
- Fixed 64-byte blocks
- Tests both allocation and deallocation phases

### 3. Cache Stress (`bench_cache_stress`)

Tests the allocator's cache behavior under stress:
- Allocates more blocks than cache can hold (600 vs 512 cache size)
- Tests cache overflow handling
- Measures performance degradation when cache is full

### 4. Mixed Pattern (`bench_mixed_pattern`)

Simulates realistic allocation patterns:
- Random allocation/deallocation operations
- Variable block sizes (8B to 1KB)
- 60% allocation probability, 40% deallocation
- Tests fragmentation and cache utilization

### 5. Alignment (`bench_alignment`)

Tests allocation performance with different alignment requirements:
- Alignments: 8, 16, 32, 64, 128, 256 bytes
- Fixed 64-byte allocation size
- Verifies correct alignment in debug builds

### 6. Fragmentation (`bench_fragmentation`)

Tests allocator behavior under fragmented conditions:
- Creates fragmentation by deallocating every other small block
- Attempts large allocations in fragmented space
- Measures fragmentation handling efficiency

### 7. Realistic Workload (`bench_realistic_workload`)

Simulates real-world application patterns:
- **Web Server Simulation**: Variable request/response buffer sizes
- Realistic allocation/deallocation timing
- Models typical server workload patterns

## Understanding Results

### Metrics

- **Time**: Average time per operation (lower is better)
- **Throughput**: Operations per second (higher is better)
- **Variance**: Consistency of performance (lower is better)

### Interpreting Output

```
basic_allocation/bene_alloc/64  time:   [45.2 ns 46.1 ns 47.0 ns]
basic_allocation/system_alloc/64 time: [52.3 ns 53.1 ns 54.2 ns]
```

This shows `benemalloc` is ~15% faster than system allocator for 64-byte allocations.

### Performance Expectations

`benemalloc` should show advantages in:
- **Small allocations**: Cache hits should be faster than system calls
- **Repeated patterns**: Cache reuse should improve performance
- **Thread-local allocations**: No contention with other threads

`benemalloc` may show disadvantages in:
- **Large allocations**: Direct system calls may be faster
- **Cache overflow**: Performance degrades when cache is full
- **Highly fragmented scenarios**: Simple cache may not optimize well

## Benchmark Utilities

The crate provides utility functions for custom benchmarks:

```rust
use benemalloc_benches::{layout, AllocationPattern, COMMON_SIZES};

// Create allocation layout
let layout = layout(64, 8);

// Execute allocation patterns
let pattern = AllocationPattern::SmallBurst { count: 100, size: 64 };
unsafe { pattern.execute(&allocator); }
```

## Customizing Benchmarks

### Adding New Benchmarks

1. Add benchmark function to `benches/allocator_benchmarks.rs`
2. Include in `criterion_group!` macro
3. Follow existing patterns for consistency

### Benchmark Structure

```rust
fn bench_custom_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("custom_pattern");

    group.bench_function("pattern_name", |b| {
        b.iter(|| {
            // Benchmark code here
            black_box(result);
        });
    });

    group.finish();
}
```

## Troubleshooting

### Common Issues

1. **Nightly Rust Required**: `benemalloc` uses unstable features
2. **Platform Differences**: Results vary between operating systems
3. **Debug vs Release**: Always benchmark in release mode
4. **System Load**: Background processes affect results

### Debugging Allocator Issues

Enable allocation tracking for debugging:

```bash
cargo bench --features track_allocations
```

## Contributing

When adding new benchmarks:

1. Document the benchmark purpose and expected behavior
2. Include both `benemalloc` and system allocator comparisons
3. Use appropriate sample sizes and measurement durations
4. Add relevant test cases to verify benchmark correctness

## License

This benchmark suite is licensed under GPL-3.0-only, same as the main `benemalloc` project.
