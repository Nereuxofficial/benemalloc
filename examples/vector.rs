extern crate benemalloc;
use benemalloc::BeneAlloc;

#[global_allocator]
static ALLOCATOR: BeneAlloc = BeneAlloc;

fn main() {
    let mut vec = vec![];
    for _ in 0..100 {
        vec.push(2);
    }
}
