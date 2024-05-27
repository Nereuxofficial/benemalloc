alias b:= build
alias c:= clean
alias t:= test

install:
    @echo "Installing dependencies"
    @cargo install cargo-binstall
    @cargo binstall cargo-nextest
    @cargo binstall cargo-careful

clean:
    @echo "Cleaning"
    @cargo clean

test:
    @echo "Running tests"
    @cargo nextest run

test_tracking:
    @echo "Running tests with allocation tracking"
    @cargo nextest run --features track_allocations --success-output final

test_careful:
    @echo "Running tests with careful"
    @cargo +nightly careful test

fmt:
    @cargo fmt

lint:
    @cargo clippy

build:
    @echo "Building"
    @cargo build