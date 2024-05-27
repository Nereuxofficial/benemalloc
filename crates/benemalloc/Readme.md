# benemalloc
> A fast memory allocator for Rust.

> [!Caution]
> This is currently a research project and is not useable in production. Use it at your own risk!

# Usage
```
cargo add benemalloc
```

```rust
use benemalloc::BeneAlloc;

#[global_allocator]
static ALLOCATOR: BeneAlloc = BeneAlloc::new();
```

# License
GPL-3.0