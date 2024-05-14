use benemalloc::BeneAlloc;
use rand::thread_rng;
use rand::RngCore;
use std::{collections::BinaryHeap, hint::black_box, thread, thread::available_parallelism};

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
    let binary_heap = BinaryHeap::from(vec);
    let mut vec = binary_heap.into_sorted_vec();
    vec.into_iter().take(10).for_each(|(k, v)| {
        println!("Key: {}, Value: {}", k, v);
    });
}

#[test]
fn test_threads() {
    let mut handles = vec![];
    for _ in 0..available_parallelism().unwrap().get() {
        let handle = std::thread::spawn(|| {
            test_small_allocs();
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_box_allocation() {
    let mut value = Box::new(10);
    *value = 20;
    println!("Value: {}", value);
    drop(value);
    let new_value = Box::new(30);
    println!("New Value: {}", new_value);
    thread::spawn(move || {
        println!("New Value: {}", new_value);
        drop(new_value);
    })
    .join()
    .unwrap();
}
