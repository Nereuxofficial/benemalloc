//! This is for testing fairly large allocations

extern crate benemalloc;

use std::hint::black_box;
use std::thread::sleep;
use rand::prelude::ThreadRng;
use rand::{RngCore, thread_rng};
use benemalloc::BeneAlloc;

#[global_allocator]
static ALLOCATOR: BeneAlloc = BeneAlloc::new();
fn main(){
    let num = 200_000_000;
    let mut rng = thread_rng();
    println!("Allocating {}MB of memory", num * 8 / 1024 / 1024);
    let mut vec = Vec::with_capacity(num);
    for _ in 0..vec.capacity() {
        vec.push(rng.next_u64());
    }
    sleep(std::time::Duration::from_secs(10));
    println!("Sum: {}", black_box(vec.iter().map(|&x|x as u128).sum::<u128>()));
}