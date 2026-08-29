set quiet

benchmark_report := "target/criterion/report/index.html"

alias b := build
alias bench := benchmark
alias c := clean
alias t := test

install:
    echo "Installing dependencies"
    cargo install cargo-binstall
    cargo binstall cargo-nextest
    cargo binstall cargo-careful

clean:
    echo "Cleaning"
    cargo clean

build:
    echo "Building"
    cargo build

fmt:
    cargo fmt

lint:
    cargo clippy

test:
    echo "Running tests"
    cargo nextest run

test_tracking:
    echo "Running tests with allocation tracking"
    cargo nextest run --features track_allocations --success-output final

test_careful:
    echo "Running tests with cargo-careful"
    cargo +nightly careful test

benchmark:
    echo "Running benchmarks"
    cargo bench

benchmark_open: benchmark
    echo "Opening benchmark report"
    xdg-open {{ benchmark_report }} 2>/dev/null \
        || open {{ benchmark_report }} 2>/dev/null \
        || echo "Please open {{ benchmark_report }} manually"

benchmark_group GROUP:
    printf 'Running benchmark group: %s\n' {{ quote(GROUP) }}
    cargo bench -- {{ quote(GROUP) }}

benchmark_clean:
    echo "Cleaning benchmark results"
    rm -rf target/criterion

quick_comparison:
    echo "Running quick performance comparison"
    cargo +nightly run -p benemalloc-benches --example quick_comparison --release
