// TODO: Allow for tracking Allocation and Deallocation of memory

use serde::{Deserialize, Serialize};
use std::alloc::Layout;
use std::alloc::System;
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Serialize)]
pub enum Action {
    Cache,
    System,
}

#[derive(Clone, Copy, Serialize)]
pub enum Event {
    Alloc {
        addr: usize,
        size: usize,
        source: Action,
    },
    Free {
        addr: usize,
        size: usize,
        action: Action,
    },
    Resize {
        addr: usize,
        new_size: usize,
    },
}

pub struct Tracker {
    //FIXME: We cannot allocate, so maybe use atomic integers?
    allocations: AtomicU64,
    allocated_size: AtomicU64,
    system_alloc: System,
}

impl Tracker {
    pub const fn new() -> Self {
        Self {
            allocations: AtomicU64::new(0),
            allocated_size: AtomicU64::new(0),
            system_alloc: System,
        }
    }

    pub fn track(&mut self, event: Event) {
        let mut buf = [0u8; 2048];
        let mut cursor = Cursor::new(&mut buf[..]);
        serde_json::to_writer(&mut cursor, &event).unwrap();
        let end = cursor.position() as usize;
        let section = &buf[..end];
        self.write(&buf[..end]);
        self.write(b"\n");
    }
    fn write(&self, s: &[u8]) {
        unsafe {
            libc::write(libc::STDERR_FILENO, s.as_ptr() as _, s.len() as _);
        }
    }

    pub fn print(&self) {
        println!("Allocations: {:?}", self.allocations);
        println!("Allocated Size: {:?}", self.allocated_size);
    }
}
