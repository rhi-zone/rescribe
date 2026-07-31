//! Shared instrumented global allocator for this crate's own test suite.
//!
//! A process may only define one `#[global_allocator]` per binary. More than
//! one of this crate's test modules needs an allocation/peak-memory probe
//! ([`crate::writer`]'s no-subtree-reconstruction-blowup guard,
//! [`crate::batch`]'s `StreamingParser` peak-memory guard), so it lives here
//! once and every `#[cfg(test)]` module that needs it imports from here
//! instead of declaring its own.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) static ALLOCS: AtomicUsize = AtomicUsize::new(0);

// current/peak bytes are tracked per-thread (`thread_local!`, not a shared
// `AtomicUsize`): the allocator is process-wide, and `cargo test` runs other
// tests concurrently on other threads by default, so a shared counter lets
// an unrelated test's allocations inflate a measurement — confirmed as a
// real flake in this batch's `pod-fmt` sibling crate (a spurious 407x ratio
// under full-workspace `cargo test -q`, passing cleanly under
// `--test-threads=1`). Thread-local counters make measurements immune to
// what other threads in the same binary do.
thread_local! {
    pub(crate) static CURRENT: Cell<usize> = const { Cell::new(0) };
    pub(crate) static PEAK: Cell<usize> = const { Cell::new(0) };
}

pub(crate) struct InstrumentedAlloc;

unsafe impl GlobalAlloc for InstrumentedAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        let cur = CURRENT.with(|c| {
            let v = c.get() + layout.size();
            c.set(v);
            v
        });
        PEAK.with(|p| {
            if cur > p.get() {
                p.set(cur);
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT.with(|c| c.set(c.get().saturating_sub(layout.size())));
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: InstrumentedAlloc = InstrumentedAlloc;
