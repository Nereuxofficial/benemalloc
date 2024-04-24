extern crate benemalloc;
use benemalloc::BeneAlloc;
use rand::{Rng, RngCore};
use std::thread::sleep;
#[global_allocator]
static ALLOCATOR: BeneAlloc = BeneAlloc::new();

fn main() {
    println!("Creating Vector...");
    let mut vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut rng = rand::thread_rng();
    println!("Pushing 100 elements into the vector...");
    for _ in 0..100 {
        vec.push(2);
    }
    println!("Popping 100 elements from the vector...");
    for _ in 0..100 {
        vec.pop();
    }
    let some_string = String::from("Hello, World!");
    println!("String: {}", some_string);
    let some_string = format!("{}, {:?}", some_string, vec);
    println!("String: {}", some_string);
    let mut btreemap = std::collections::BTreeMap::new();
    for _ in 0..10000 {
        btreemap.insert(rng.next_u64(), rng.next_u64());
    }
    let mut vec = btreemap.iter().collect::<Vec<_>>();
    vec.sort_by(|a, b| a.0.cmp(b.0));
    vec.reverse();
    println!("BTreeMap: {:?}", btreemap);
    println!("Vec: {:?}", vec);
    let mut hashmap = std::collections::HashMap::new();
    for _ in 0..10000 {
        hashmap.insert(rng.next_u64(), rng.next_u64());
    }
    println!("HashMap: {:?}", hashmap);
}
