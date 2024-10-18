#![feature(panic_internals)]
#![feature(fmt_internals)]
#![feature(allocator_api)]

#[cfg(test)]
pub mod rust_tests;
/* This should probably moved to another crate, since setting globalalloc influences rust_tests
#[cfg(all(not(miri), test))]
mod global_alloc_tests;
 */
