use benemalloc::BeneAlloc;
use rand::RngCore;
use std::hint::black_box;
use rand::thread_rng;
#[global_allocator]
static ALLOCATOR: BeneAlloc = BeneAlloc::new();

#[test]
fn test_large_allocs() {
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

#[test]
fn test_small_allocs() {
    println!("Creating Vector...");
    let mut vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        vec.push(2);
    }
    for _ in 0..100 {
        vec.pop();
    }
    let some_string = String::from("Hello, World!");
    println!("String: {}", some_string);
    let some_string = format!("{}, {:?}", some_string, vec);
    println!("String: {}", some_string);
    let mut btreemap = std::collections::BTreeMap::new();
    for _ in 0..100 {
        btreemap.insert(rng.next_u64(), rng.next_u64());
    }
    let mut vec = btreemap.iter().collect::<Vec<_>>();
    let mut hashmap = std::collections::HashMap::new();
    for _ in 0..100 {
        hashmap.insert(rng.next_u64(), rng.next_u64());
    }
}
