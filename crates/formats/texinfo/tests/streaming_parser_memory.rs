//! Guard against reintroducing buffer-everything behaviour in
//! `texinfo::batch::StreamingParser`.
//!
//! `StreamingParser` used to push every fed byte into a `Vec<u8>` and only
//! call `crate::parse::parse` (via `crate::events::events`) once, inside
//! `finish()` — so peak memory grew with the whole document (see the
//! module's old doc comment: "Memory usage is O(full input)"). The current
//! implementation splits the input into top-level units (paragraphs,
//! headings, `@directive ... @end directive` environments) as it is fed,
//! flushing each unit to the handler as soon as its boundary is confirmed,
//! so peak live heap must stay flat as the document grows.
//!
//! This measures the *cost shape*, not the output: a 10x longer document
//! must not cost meaningfully more live heap at its peak. A regression to
//! whole-input buffering would show up here as a ~10x ratio.
//!
//! The test binary owns the global allocator, so it deliberately contains
//! only this one test — a process may only define one
//! `#[global_allocator]`, and sharing it with texinfo's other (parallel,
//! concurrently-running) unit tests would pollute the peak reading with
//! unrelated allocations. Modeled on
//! `tikiwiki/tests/streaming_writer_memory.rs`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use texinfo::StreamingParser;

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

/// Build a synthetic multi-section Texinfo document with `sections`
/// sections, each a heading followed by a paragraph and a small itemized
/// list, so the document exercises headings, paragraphs, and an
/// `@end`-delimited environment per section.
fn synthetic_doc(sections: usize) -> Vec<u8> {
    let mut s = String::with_capacity(sections * 200);
    s.push_str("@settitle Synthetic Benchmark Document\n\n");
    for i in 0..sections {
        s.push_str(&format!("@chapter Section {i}\n\n"));
        s.push_str(&format!(
            "This is paragraph number {i} with enough text to be worth measuring, \
             covering the section's introductory material in reasonable detail.\n\n"
        ));
        s.push_str("@itemize\n@item First point\n@item Second point\n@end itemize\n\n");
    }
    s.into_bytes()
}

/// Feed `input` through `StreamingParser` in small (61-byte) chunks — large
/// enough to be realistic, small enough to exercise cross-chunk boundary
/// handling — and return the peak live heap observed above `before`, in
/// bytes.
fn peak_bytes_for(input: &[u8]) -> u64 {
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);

    let mut count = 0usize;
    let mut parser = StreamingParser::new(|_e| count += 1);
    for chunk in input.chunks(61) {
        parser.feed(chunk);
    }
    parser.finish();
    std::hint::black_box(count);

    PEAK.load(Ordering::Relaxed).saturating_sub(before) as u64
}

#[test]
fn streaming_parser_peak_memory_is_flat_in_document_size() {
    // Warm up so lazily-initialised globals do not land inside a
    // measurement.
    let _ = peak_bytes_for(&synthetic_doc(5));

    let small_doc = synthetic_doc(50);
    let large_doc = synthetic_doc(500); // 10x the sections

    let small = peak_bytes_for(&small_doc).max(1);
    let large = peak_bytes_for(&large_doc);

    let ratio = large as f64 / small as f64;
    println!(
        "peak live heap: 50 sections ({} B input) = {small} B, 500 sections ({} B input) = \
         {large} B (ratio {ratio:.2}x)",
        small_doc.len(),
        large_doc.len(),
    );
    assert!(
        ratio < 3.0,
        "peak live heap grew {ratio:.2}x for a 10x longer document ({small} -> {large} bytes, \
         input {} -> {} bytes). StreamingParser must not buffer the whole input; peak memory \
         must track the largest unit and nesting depth, not document length.",
        small_doc.len(),
        large_doc.len(),
    );
}
