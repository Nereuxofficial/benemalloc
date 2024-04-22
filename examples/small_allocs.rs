extern crate benemalloc;
use benemalloc::BeneAlloc;
#[global_allocator]
static ALLOCATOR: BeneAlloc = BeneAlloc::new();

fn main() {
    println!("Creating Vector...");
    let mut vec = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
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
    btreemap.insert(1, 2);
    btreemap.insert(2, 3);
    btreemap.insert(3, 4);
    btreemap.insert(4, 5);
    btreemap.insert(5, 6);
    println!("BTreeMap: {:?}", btreemap);
    let mut hashmap = std::collections::HashMap::new();
    hashmap.insert("Hello", "World");
    hashmap.insert("Foo", "Bar");
    hashmap.insert("Fizz", "Buzz");
    hashmap.insert("Tom", "Jerry");
    hashmap.insert("Rust", "Lang");
    hashmap.insert("C", "Segfault");
    hashmap.insert("Python", "IndentationError");
    println!("HashMap: {:?}", hashmap);
}
