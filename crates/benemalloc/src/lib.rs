//! Thread-owned spans with intrusive local and remote free lists.

mod large_allocs;

use allocations::{allocate, deallocate};
use std::alloc::{GlobalAlloc, Layout};
use std::cell::{Cell, UnsafeCell};
use std::ptr::null_mut;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

const STEPS: usize = 20;
const MIN_SIZE_SHIFT: usize = 3;
const COLLECTION_INTERVAL: usize = 256;
static NEXT_OWNER: AtomicUsize = AtomicUsize::new(1);

#[repr(C)]
struct SegmentHeader {
    next: *mut SegmentHeader,
}

// Only the current owner accesses this state. No mutable reference to the
// whole Span may exist while remote producers access its atomic fields.
struct LocalSpan {
    free: *mut SegmentHeader,
    live_count: usize,
    next: *mut Span,
}

struct Span {
    local: UnsafeCell<LocalSpan>,
    remote: AtomicPtr<SegmentHeader>,
    owner: AtomicUsize,
    class: usize,
    mapping_len: usize,
}

struct Orphans(*mut Span);
// The mutex transfers exclusive ownership of the list and its LocalSpan fields.
unsafe impl Send for Orphans {}
static ORPHANS: Mutex<Orphans> = Mutex::new(Orphans(null_mut()));

struct Heap {
    id: usize,
    classes: [*mut Span; STEPS],
    ticks: usize,
}

impl Heap {
    const fn new() -> Self {
        Self {
            id: 0,
            classes: [null_mut(); STEPS],
            ticks: 0,
        }
    }

    fn initialize(&mut self) -> bool {
        if self.id == 0 {
            // Never reuse identities, including when a TLS address is reused.
            let Ok(id) =
                NEXT_OWNER.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            else {
                return false;
            };
            self.id = id;
            self.collect();
        }
        true
    }

    fn class(layout: Layout) -> usize {
        let size = layout.size().max(layout.align()).max(1 << MIN_SIZE_SHIFT);
        usize::BITS as usize - (size - 1).leading_zeros() as usize - MIN_SIZE_SHIFT
    }

    // Detach first as producers may immediately start building a new inbox.
    unsafe fn drain(span: *mut Span) {
        unsafe {
            let mut item = (*span).remote.swap(null_mut(), Ordering::Acquire);
            let local = &mut *(*span).local.get();
            while !item.is_null() {
                let next = (*item).next;
                (*item).next = local.free;
                local.free = item;
                local.live_count -= 1;
                item = next;
            }
        }
    }

    fn collect(&mut self) {
        // Do not hold the pool lock while walking inboxes or unmapping memory.
        let mut orphan = {
            let mut pool = ORPHANS.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::replace(&mut pool.0, null_mut())
        };
        unsafe {
            while !orphan.is_null() {
                let next = (*(*orphan).local.get()).next;
                let class = (*orphan).class;
                (*orphan).owner.store(self.id, Ordering::Release);
                (*(*orphan).local.get()).next = self.classes[class];
                self.classes[class] = orphan;
                orphan = next;
            }
            for head in &mut self.classes {
                let mut link = head as *mut *mut Span;
                let mut kept_empty = false;
                while !(*link).is_null() {
                    let span = *link;
                    Self::drain(span);
                    let local = &mut *(*span).local.get();
                    if local.live_count == 0 && kept_empty {
                        *link = local.next;
                        let len = (*span).mapping_len;
                        deallocate(span.cast(), len);
                    } else {
                        kept_empty |= local.live_count == 0;
                        link = &mut local.next;
                    }
                }
            }
        }
    }

    fn fill_size_class(&mut self, class: usize) -> bool {
        let size = 1usize << (MIN_SIZE_SHIFT + class);
        // Each slot reserves one size-aligned prefix region followed by its
        // payload. The last pointer before the payload identifies the span.
        let stride = size * 2;
        let count = (4096 / stride).max(8);
        let len = size_of::<Span>() + size - 1 + count * stride;
        let base = allocate(len).cast::<u8>();
        if base.is_null() || base as usize == usize::MAX {
            return false;
        }
        unsafe {
            let span = base.cast::<Span>();
            let start = base.add(size_of::<Span>());
            let offset = start.align_offset(size);
            let payload = start.add(offset + size);
            let mut free = null_mut();
            for i in (0..count).rev() {
                let ptr = payload.add(i * stride).cast::<SegmentHeader>();
                ptr.cast::<*mut Span>().sub(1).write(span);
                ptr.write(SegmentHeader { next: free });
                free = ptr;
            }
            span.write(Span {
                local: UnsafeCell::new(LocalSpan {
                    free,
                    live_count: 0,
                    next: self.classes[class],
                }),
                remote: AtomicPtr::new(null_mut()),
                owner: AtomicUsize::new(self.id),
                class,
                mapping_len: len,
            });
            self.classes[class] = span;
        }
        true
    }

    fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let class = Self::class(layout);
        if class >= STEPS || !self.initialize() {
            return large_allocs::allocate_large(layout.size(), layout.align());
        }
        self.ticks += 1;
        if self.ticks == COLLECTION_INTERVAL {
            self.ticks = 0;
            self.collect();
        }
        unsafe {
            let mut span = self.classes[class];
            while !span.is_null() {
                if (*(*span).local.get()).free.is_null() {
                    Self::drain(span);
                }
                let local = &mut *(*span).local.get();
                if !local.free.is_null() {
                    let ptr = local.free;
                    local.free = (*ptr).next;
                    local.live_count += 1;
                    return ptr.cast();
                }
                span = local.next;
            }
            if !self.fill_size_class(class) {
                return null_mut();
            }
            let local = &mut *(*self.classes[class]).local.get();
            let ptr = local.free;
            local.free = (*ptr).next;
            local.live_count += 1;
            ptr.cast()
        }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        // TLS cannot be used by a new caller once its destructor starts.
        // Outstanding includes producers that have not published yet.
        unsafe {
            let mut pool = ORPHANS.lock().unwrap_or_else(|e| e.into_inner());
            for head in &mut self.classes {
                let mut span = std::mem::replace(head, null_mut());
                while !span.is_null() {
                    Self::drain(span);
                    let local = &mut *(*span).local.get();
                    let next = local.next;
                    if local.live_count == 0 {
                        deallocate(span.cast(), (*span).mapping_len);
                    } else {
                        (*span).owner.store(0, Ordering::Release);
                        local.next = pool.0;
                        pool.0 = span;
                    }
                    span = next;
                }
            }
        }
    }
}

struct ThreadHeap {
    heap: UnsafeCell<Heap>,
    busy: Cell<bool>,
}
impl ThreadHeap {
    fn with<R>(&self, f: impl FnOnce(&mut Heap) -> R) -> Option<R> {
        if self.busy.replace(true) {
            return None;
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.busy);
        Some(f(unsafe { &mut *self.heap.get() }))
    }
}
thread_local! {
    static CURRENT_THREAD_ALLOCATOR: ThreadHeap = const {
        ThreadHeap { heap: UnsafeCell::new(Heap::new()), busy: Cell::new(false) }
    };
}

/// Stateless handle to the current thread's heap.
pub struct BeneAlloc {
    #[cfg(feature = "debug")]
    pub allocations: [Option<Layout>; 4096],
}
impl BeneAlloc {
    pub const fn new() -> Self {
        Self {
            #[cfg(feature = "debug")]
            allocations: [None; 4096],
        }
    }

    /// Adopt abandoned spans, drain remote returns, and release surplus empty
    /// spans. Retains at most one empty span per class in this thread's heap.
    pub fn collect(&self) {
        let _ = CURRENT_THREAD_ALLOCATOR.try_with(|tls| {
            tls.with(|heap| {
                if heap.initialize() {
                    heap.collect();
                }
            })
        });
    }
}

unsafe impl GlobalAlloc for BeneAlloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        CURRENT_THREAD_ALLOCATOR
            .try_with(|tls| tls.with(|heap| heap.alloc(layout)))
            .ok()
            .flatten()
            .unwrap_or_else(|| large_allocs::allocate_large(layout.size(), layout.align()))
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe {
            // Both allocation routes carry this prefix. Null marks a standalone
            // mapping, including allocations made during TLS teardown/reentry.
            let span = ptr.cast::<*mut Span>().sub(1).read();
            if span.is_null() {
                large_allocs::free_large(ptr);
                return;
            }
            let item = ptr.cast::<SegmentHeader>();
            let local = CURRENT_THREAD_ALLOCATOR
                .try_with(|tls| {
                    tls.with(|heap| {
                        if heap.id == 0 || (*span).owner.load(Ordering::Acquire) != heap.id {
                            return false;
                        }
                        let local = &mut *(*span).local.get();
                        (*item).next = local.free;
                        local.free = item;
                        local.live_count -= 1;
                        true
                    })
                })
                .ok()
                .flatten()
                .unwrap_or(false);
            if local {
                return;
            }

            // This allocation remains counted until an owner drains it.
            // Producers never dereference the observed head, so detaching and
            // reusing that head cannot invalidate a producer's reads.
            let inbox = &(*span).remote;
            let mut head = inbox.load(Ordering::Relaxed);
            loop {
                (*item).next = head;
                match inbox.compare_exchange_weak(head, item, Ordering::Release, Ordering::Relaxed)
                {
                    Ok(_) => return, // No span/item access after publication.
                    Err(current) => head = current,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
