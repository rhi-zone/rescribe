//! Guard against reintroducing buffer-everything behaviour in
//! `tikiwiki::writer::Writer`.
//!
//! `Writer` used to push every event into a `Vec<OwnedEvent>` and reconstruct
//! a `TikiwikiDoc` AST before emitting anything (via `emit::build`), so peak
//! memory grew with the whole document. The current writer emits each event
//! straight into the shared output buffer and flushes to the sink as soon as
//! a top-level block closes, so peak live heap must stay flat as the
//! document grows.
//!
//! This measures the *cost shape*, not the output: a 100x longer document
//! must not cost meaningfully more live heap at its peak. A regression to
//! event buffering or AST reconstruction would show up here as a ~100x
//! ratio.
//!
//! The test binary owns the global allocator, so it deliberately contains
//! only this one test — a process may only define one
//! `#[global_allocator]`, and sharing it with tikiwiki's other (parallel,
//! concurrently-running) unit tests would pollute the peak reading with
//! unrelated allocations. Modeled on
//! `ooxml-wml/tests/streaming_writer_memory.rs`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::borrow::Cow;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use tikiwiki::OwnedEvent;
use tikiwiki::writer::Writer;

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

/// A sink that discards bytes rather than accumulating them, so the emitted
/// output itself (which necessarily grows with the document — that's
/// inherent to producing it, not a property of the writer) never enters the
/// measurement. Mirrors what a real streaming consumer (a socket, a file, a
/// downstream transform) would do.
struct NullSink {
    total: u64,
}

impl Write for NullSink {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.total += data.len() as u64;
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Feed `paragraphs` paragraphs through the writer and return the peak live
/// heap observed during the run, in bytes.
///
/// Each paragraph's text is a freshly formatted owned `String`, constructed
/// inside the loop: the writer must not retain it past the event that
/// carries it. Nothing is pre-built into a `Vec`, so any growth measured
/// here belongs to the writer itself.
fn peak_bytes_for(paragraphs: usize) -> u64 {
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);

    let mut w = Writer::new(NullSink { total: 0 });
    for i in 0..paragraphs {
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text(Cow::Owned(format!(
            "paragraph {i} with enough text to be worth measuring"
        ))));
        w.write_event(OwnedEvent::EndParagraph);
    }
    w.finish();

    PEAK.load(Ordering::Relaxed).saturating_sub(before) as u64
}

#[test]
fn writer_peak_memory_is_flat_in_document_size() {
    // Warm up so lazily-initialised globals do not land inside a
    // measurement.
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
