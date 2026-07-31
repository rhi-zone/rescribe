//! Test-only allocator-tracking infrastructure, shared by every module's
//! memory-shape tests in this crate.
//!
//! Only one `#[global_allocator]` is allowed per test binary, and `cargo
//! test`'s `--lib` harness links every module's `#[cfg(test)] mod tests`
//! into that single binary — so `writer.rs`'s and `batch.rs`'s memory tests
//! must share one definition rather than each declaring their own (which
//! would fail to compile, two `#[global_allocator]` statics in one binary).
//!
//! current/peak bytes are tracked per-thread (`thread_local!`, not a shared
//! `AtomicUsize`): `cargo test` runs this binary's tests on multiple threads
//! sharing one process-wide allocator, and a shared counter would let an
//! unrelated concurrently-running test's allocations inflate this test's
//! peak — found via a real flake under full-workspace `cargo test -q` (peak
//! ratio 407x from cross-thread noise, not a real regression; passed cleanly
//! under `--test-threads=1`). Thread-local counters make the measurement
//! immune to what other threads in the same binary do.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::AtomicUsize;

pub(crate) struct TrackingAlloc;

/// Running allocation *count*, for linearity regression guards.
pub(crate) static ALLOCS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Current live bytes on this thread.
    pub(crate) static CURRENT_BYTES: Cell<usize> = const { Cell::new(0) };
    /// Peak live bytes on this thread since the last reset.
    pub(crate) static PEAK_BYTES: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for TrackingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cur = CURRENT_BYTES.with(|c| {
            let v = c.get() + layout.size();
            c.set(v);
            v
        });
        PEAK_BYTES.with(|p| {
            if cur > p.get() {
                p.set(cur);
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT_BYTES.with(|c| c.set(c.get().saturating_sub(layout.size())));
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: TrackingAlloc = TrackingAlloc;
