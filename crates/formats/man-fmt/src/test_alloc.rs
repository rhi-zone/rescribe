//! Shared allocation-instrumentation support for this crate's tests.
//!
//! Rust permits at most one `#[global_allocator]` per test binary, and every
//! `#[cfg(test)] mod tests` block in this crate compiles into the same
//! `cargo test -p man-fmt` binary — so the tracking allocator must be
//! declared exactly once, here, and reused by every module that needs
//! allocation counting or peak-memory tracking (currently `events.rs` and
//! `writer.rs`).
//!
//! Peak/current bytes are tracked per-thread (`thread_local!`, not a shared
//! `AtomicUsize`): `cargo test` runs this crate's other tests concurrently
//! with allocator-instrumented ones by default, and a shared counter lets an
//! unrelated concurrently-running test's allocations inflate a test's
//! measured peak — a shared-counter design caused a real cross-thread flake
//! in 12+ crates this session (a spurious multi-hundred-x ratio under
//! full-workspace `cargo test -q`, passing cleanly under
//! `--test-threads=1`). Thread-local counters make the measurement immune
//! to what other threads in the same binary do.

// This crate denies `unsafe` in production code (see lib.rs); the
// `GlobalAlloc` impl below is test-only harness plumbing to measure
// allocation behavior and is explicitly opted back in here.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct TrackingAlloc;

/// Total number of `alloc` calls observed, across all threads.
pub static ALLOCS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Bytes currently allocated and not yet freed, for the calling thread.
    pub static CURRENT: Cell<usize> = const { Cell::new(0) };
    /// High-water mark of `CURRENT` for the calling thread.
    pub static PEAK: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for TrackingAlloc {
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
static GLOBAL: TrackingAlloc = TrackingAlloc;
