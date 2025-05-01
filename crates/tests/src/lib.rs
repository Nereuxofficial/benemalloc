#![feature(panic_internals)]
#![feature(fmt_internals)]
#![feature(allocator_api)]

use benemalloc::BeneAlloc;

#[global_allocator]
static ALLOCATOR: BeneAlloc = BeneAlloc::new();

#[cfg(test)]
pub mod rust_tests;

#[cfg(test)]
mod global_alloc_tests;
