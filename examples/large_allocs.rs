//! This is for testing fairly large allocations

extern crate benemalloc;

use benemalloc::BeneAlloc;
use rand::{thread_rng, RngCore};
use std::hint::black_box;

#[global_allocator]
static ALLOCATOR: BeneAlloc = BeneAlloc::new();
fn main() {
    let num: usize = 200_000_000;
    let mut rng = thread_rng();
    println!("Allocating {}MB of memory", num * 8 / 1024 / 1024);
    let mut vec = Vec::with_capacity(num);
    for _ in 0..vec.capacity() {
        vec.push(rng.next_u64());
    }
    println!(
        "Sum: {}",
        black_box(vec.iter().map(|&x| x as u128).sum::<u128>())
    );
    for _ in 0..vec.capacity() {
        vec.pop();
    }
    vec.resize(100, 0)
}
