# benemalloc
> A memory allocator written in Rust

> [!Caution]
> This is currently a research project and is not useable in production. Use it at your own risk!

benemalloc aims to be a general-purpose memory allocator written in Rust. Rust is a safe systems languge, yet no general-purpose memory allocator is written in it. This project aims to solve that by writing one in pure Rust.


## Performance
- 

## Design
- [ ] Additionally GrapheneOS's hardened_malloc has some really interesting techniques for examples
### Rust specific
- [ ] Rust knows the size of every struct, but doesn't tell the known size to a memory allocator, since it uses free(addr: *c_void) and the memory allocator has to search for the size of the allocation of the pointer if it wants to unmap it. Eliminating this lookup could yield a performance improvement.
- [ ] Since in this case all code is Rust code, we could design the allocator around the Builder pattern to allow users to customize the allocator. Here are some examples of these features:
  - [ ] Don't unmap memory regions at all. Useful for short programs. The memory is given back to the system, when the program exited.
- [ ] Because of Rust's borrow checker the allocator can avoid double free detection, but whether this should be the default behaviour is questionable since Rust Programs often link with C Programs. Making it a toggle-able feature could be worthwhile nonetheless.
