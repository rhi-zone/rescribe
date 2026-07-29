//! Guard against reintroducing buffer-everything behaviour in `WmlWriter`.
//!
//! `WmlWriter` used to push every event into a `Vec<OwnedWmlEvent>` and
//! reconstruct a `Paragraph`/`Run`/`Table` AST before emitting anything, so peak
//! memory grew with the document. The current writer emits each event straight
//! into the open ZIP entry, so peak live heap must be flat in document size.
//!
//! This measures the *cost shape*, not the output: a 100x longer document must
//! not cost meaningfully more live heap at its peak. A regression to event
//! buffering or AST reconstruction would show up here as a ~100x ratio.
//!
//! The test binary owns the global allocator, so it deliberately contains only
//! this one test.

use std::alloc::{GlobalAlloc, Layout, System};
use std::borrow::Cow;
use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use ooxml_wml::{WmlEvent, WmlWriter};

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

/// A `Write + Seek` sink that discards bytes, so the output image itself does
/// not enter the measurement.
struct NullSink {
    pos: u64,
    len: u64,
}

impl Write for NullSink {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.pos += data.len() as u64;
        self.len = self.len.max(self.pos);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for NullSink {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        self.pos = match from {
            SeekFrom::Start(n) => n,
            SeekFrom::End(n) => (self.len as i64 + n) as u64,
            SeekFrom::Current(n) => (self.pos as i64 + n) as u64,
        };
        Ok(self.pos)
    }
}

/// Feed `paragraphs` paragraphs through the writer and return the peak live
/// heap observed during the run, in bytes.
///
/// Events are constructed inside the loop deliberately: each is a single owned
/// value that the writer must not retain. Nothing is pre-built into a `Vec`,
/// so any growth measured here belongs to the writer.
fn peak_bytes_for(paragraphs: usize) -> usize {
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);

    let mut w = WmlWriter::new(NullSink { pos: 0, len: 0 });
    w.write_event(WmlEvent::StartDocument);
    for i in 0..paragraphs {
        w.write_event(WmlEvent::StartParagraph {
            props: Box::default(),
        });
        w.write_event(WmlEvent::StartRun {
            props: Box::default(),
        });
        w.write_event(WmlEvent::Text(Cow::Owned(format!(
            "paragraph {i} with enough text to be worth measuring"
        ))));
        w.write_event(WmlEvent::EndRun);
        w.write_event(WmlEvent::EndParagraph);
    }
    w.write_event(WmlEvent::EndDocument);
    w.finish().expect("finish");

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
