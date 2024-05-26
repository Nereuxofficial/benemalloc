install:
    @echo "Installing dependencies"
    @cargo install cargo-binstall
    @cargo binstall cargo-nextest
    @cargo binstall cargo-careful

test:
    @echo "Running tests"
    @cargo nextest run

test_tracking:
    @echo "Running tests"
    @cargo nextest run --features track_allocations --success-output final

test_careful:
    @echo "Running tests"
    @cargo +nightly careful test
