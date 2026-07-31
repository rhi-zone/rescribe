//! Guard against reintroducing buffer-everything behaviour in
//! `markua::writer::Writer`.
//!
//! `Writer` used to push every event into a `Vec<OwnedMarkuaEvent>` and
//! reconstruct a `Block`/`Inline` tree, then call `emit::emit` on it, all
//! inside `finish()` — so peak memory grew with the document and zero bytes
//! reached the sink until the very end. The current writer emits each event
//! straight into the shared output buffer and flushes at each top-level
//! block boundary, so peak live heap must be flat in document size.
//!
//! This measures the *cost shape*, not the output: a 10x longer document
//! must not cost meaningfully more live heap at its peak. A regression to
//! event buffering or AST reconstruction would show up here as a ~10x ratio.
//!
//! The test binary owns the global allocator, so it deliberately contains
//! only this one test — see `crates/formats/markua/src/writer.rs`'s test
//! module for why the allocation-*count* regression guard stays there
//! instead (count deltas are far less sensitive to noise from concurrently
//! running unrelated tests than a peak-live-bytes high-water mark is).

use std::alloc::{GlobalAlloc, Layout, System};
use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

use markua::OwnedMarkuaEvent;
use markua::writer::Writer;

// ---------------------------------------------------------------------------
// Peak-live-bytes allocator
// ---------------------------------------------------------------------------

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

struct PeakAlloc;

unsafe impl GlobalAlloc for PeakAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(live, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let live = LIVE.fetch_add(new_size, Ordering::Relaxed) + new_size;
        PEAK.fetch_max(live, Ordering::Relaxed);
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: PeakAlloc = PeakAlloc;

/// Feed `paragraphs` paragraphs through the writer into `std::io::sink()`
/// (discards bytes, so the output itself never enters the measurement) and
/// return the peak live heap observed during the run, in bytes.
///
/// Events are constructed inside the loop deliberately: each is a single
/// owned value the writer must not retain. Nothing is pre-built into a
/// `Vec`, so any growth measured here belongs to the writer.
fn peak_bytes_for(paragraphs: usize) -> usize {
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);

    let mut w = Writer::new(std::io::sink());
    for i in 0..paragraphs {
        w.write_event(OwnedMarkuaEvent::StartParagraph);
        w.write_event(OwnedMarkuaEvent::Text(Cow::Owned(format!(
            "paragraph {i} with enough text to be worth measuring"
        ))));
        w.write_event(OwnedMarkuaEvent::EndParagraph);
    }
    w.finish();

    PEAK.load(Ordering::Relaxed).saturating_sub(before)
}

#[test]
fn writer_peak_memory_is_flat_in_document_size() {
    // Warm up so lazily-initialised globals do not land inside a measurement.
    let _ = peak_bytes_for(10);

    let small = peak_bytes_for(1_000).max(1);
    let large = peak_bytes_for(100_000);

    let ratio = large as f64 / small as f64;
    println!(
        "peak live heap: 1k paragraphs = {small} B, 100k paragraphs = {large} B (ratio {ratio:.2}x)"
    );
    assert!(
        ratio < 2.0,
        "peak live heap grew {ratio:.2}x for a 100x longer document \
         ({small} -> {large} bytes). The streaming writer must not accumulate \
         events or reconstruct an AST; peak memory must track nesting depth, \
         not document length."
    );
}
