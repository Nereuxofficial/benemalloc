use super::*;
use std::collections::HashSet;

// Tests inspect the global orphan pool. Serialize them so one test cannot
// legitimately adopt another test's still-live span.
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn allocation_after_heap_tls_destructor_uses_standalone_mapping() {
    let _isolation = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    static CHECKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    struct LateDestructor;
    impl Drop for LateDestructor {
        fn drop(&mut self) {
            let allocator = BeneAlloc::new();
            let layout = Layout::from_size_align(8, 8).unwrap();
            unsafe {
                let p = allocator.alloc(layout);
                assert!(!p.is_null());
                assert!(p.cast::<*mut Span>().sub(1).read().is_null());
                allocator.dealloc(p, layout);
            }
            CHECKED.store(true, Ordering::Release);
        }
    }
    thread_local! { static LATE: LateDestructor = const { LateDestructor }; }
    std::thread::spawn(|| {
        LATE.with(|_| {});
        let allocator = BeneAlloc::new();
        let layout = Layout::from_size_align(8, 8).unwrap();
        unsafe {
            let p = allocator.alloc(layout);
            allocator.dealloc(p, layout);
        }
    })
    .join()
    .unwrap();
    assert!(CHECKED.load(Ordering::Acquire));
}

#[test]
fn alignment_payload_and_reuse() {
    let _isolation = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let allocator = BeneAlloc::new();
    for (size, align) in [(1, 1), (9, 8), (65, 64), (4096, 4096), (17, 8192)] {
        let layout = Layout::from_size_align(size, align).unwrap();
        unsafe {
            let p = allocator.alloc(layout);
            assert!(!p.is_null());
            assert_eq!(p as usize % align, 0);
            p.write_bytes(0xa5, size);
            allocator.dealloc(p, layout);
            let q = allocator.alloc(layout);
            assert_eq!(p, q);
            allocator.dealloc(q, layout);
        }
    }
}

#[test]
fn remote_returns_are_counted_until_drained() {
    let _isolation = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let allocator = BeneAlloc::new();
    let layout = Layout::from_size_align(32, 8).unwrap();
    let p = unsafe { allocator.alloc(layout) };
    let span = unsafe { p.cast::<*mut Span>().sub(1).read() };
    let before = unsafe { (*(*span).local.get()).live_count };
    let addr = p as usize;
    std::thread::spawn(move || unsafe {
        BeneAlloc::new().dealloc(addr as *mut u8, layout);
    })
    .join()
    .unwrap();
    unsafe {
        assert_eq!((*(*span).local.get()).live_count, before);
        assert!(!(*span).remote.load(Ordering::Relaxed).is_null());
        Heap::drain(span);
        assert_eq!((*(*span).local.get()).live_count, before - 1);
        assert!((*span).remote.load(Ordering::Relaxed).is_null());
    }
}

#[test]
fn owner_exit_then_remote_free_and_adoption() {
    let _isolation = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let layout = Layout::from_size_align(64, 8).unwrap();
    let addr = std::thread::spawn(move || unsafe {
        let p = BeneAlloc::new().alloc(layout);
        assert!(!p.is_null());
        p.write_bytes(0x5a, layout.size());
        p as usize
    })
    .join()
    .unwrap();
    unsafe {
        let p = addr as *mut u8;
        assert_eq!(*p, 0x5a);
        // Adoption while this allocation is live must not release its span.
        BeneAlloc::new().collect();
        let span = p.cast::<*mut Span>().sub(1).read();
        CURRENT_THREAD_ALLOCATOR.with(|tls| {
            tls.with(|heap| {
                assert_eq!((*span).owner.load(Ordering::Acquire), heap.id);
            })
        });
        BeneAlloc::new().dealloc(p, layout);
        BeneAlloc::new().collect();
    }
}

#[test]
fn simultaneous_remote_frees_and_owner_collection() {
    let _isolation = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let allocator = BeneAlloc::new();
    let layout = Layout::from_size_align(64, 8).unwrap();
    let pointers: Vec<usize> = (0..4096)
        .map(|_| unsafe {
            let p = allocator.alloc(layout);
            assert!(!p.is_null());
            p.write_bytes(0x7b, layout.size());
            p as usize
        })
        .collect();
    assert_eq!(
        pointers.iter().copied().collect::<HashSet<_>>().len(),
        pointers.len()
    );
    std::thread::scope(|scope| {
        for chunk in pointers.chunks(512) {
            scope.spawn(move || {
                for &addr in chunk {
                    unsafe {
                        let p = addr as *mut u8;
                        assert_eq!(*p.add(63), 0x7b);
                        BeneAlloc::new().dealloc(p, layout);
                    }
                }
            });
        }
        for _ in 0..256 {
            allocator.collect();
            let p = unsafe { allocator.alloc(layout) };
            assert!(!p.is_null());
            unsafe {
                allocator.dealloc(p, layout);
            }
        }
    });
    allocator.collect();
    CURRENT_THREAD_ALLOCATOR.with(|tls| {
        tls.with(|heap| unsafe {
            let mut span = heap.classes[Heap::class(layout)];
            let mut empty = 0;
            while !span.is_null() {
                let local = &*(*span).local.get();
                assert_eq!(local.live_count, 0);
                empty += 1;
                span = local.next;
            }
            assert!(empty <= 1, "surplus empty spans should be released");
        })
    });
}

#[test]
fn reentrant_fallback_has_unambiguous_provenance() {
    let _isolation = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let allocator = BeneAlloc::new();
    let layout = Layout::from_size_align(1, 1).unwrap();
    let p = CURRENT_THREAD_ALLOCATOR
        .with(|tls| tls.with(|_| unsafe { allocator.alloc(layout) }).unwrap());
    unsafe {
        assert!(!p.is_null());
        assert!(p.cast::<*mut Span>().sub(1).read().is_null());
        allocator.dealloc(p, layout);
    }
}

#[test]
fn large_mapping_alignment() {
    let _isolation = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let allocator = BeneAlloc::new();
    let layout = Layout::from_size_align((1 << 22) + 1, 8192).unwrap();
    unsafe {
        let p = allocator.alloc(layout);
        assert!(!p.is_null());
        assert_eq!(p as usize % layout.align(), 0);
        p.write(1);
        p.add(layout.size() - 1).write(2);
        allocator.dealloc(p, layout);
    }
}
