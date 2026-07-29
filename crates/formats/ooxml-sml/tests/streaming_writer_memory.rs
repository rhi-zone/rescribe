//! Guard against reintroducing buffer-everything behaviour in `SmlWriter`.
//!
//! `SmlWriter` used to accumulate every cell into a `WorkbookBuilder`
//! (`HashMap<(row, col), BuilderCell>` per sheet) and serialise the whole
//! workbook at `finish()`, so peak memory grew with the document. Measured
//! before this rework: 100k rows x 3 cells peaked at ~223 MB. The current
//! writer emits each row/cell straight into the open ZIP entry and only
//! retains a distinct-string dedup table (O(distinct shared strings), not
//! O(cells)) plus a sheet-name list (O(sheet count)), so peak live heap must
//! stay flat as row count grows, for a fixed number of distinct strings.
//!
//! This measures the *cost shape*, not the output: a 100x longer document
//! (same string vocabulary) must not cost meaningfully more live heap at its
//! peak. A regression to per-cell accumulation would show up here as a
//! ~100x ratio.
//!
//! The test binary owns the global allocator, so it deliberately contains
//! only this one test.

use std::alloc::{GlobalAlloc, Layout, System};
use std::borrow::Cow;
use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use ooxml_sml::generated::{Cell as GenCell, CellType, Row as GenRow};
use ooxml_sml::{SmlEvent, SmlWriter};

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

const DISTINCT_STRINGS: usize = 20;

/// Feed `rows` rows (one string cell, cycling through a fixed small
/// vocabulary, plus one number cell each) through the writer and return the
/// peak live heap observed during the run, in bytes.
///
/// Events are constructed inside the loop deliberately: each is a single
/// owned value that the writer must not retain. Nothing is pre-built into a
/// `Vec`, so any growth measured here belongs to the writer.
fn peak_bytes_for(rows: usize) -> usize {
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);

    let mut w = SmlWriter::new(NullSink { pos: 0, len: 0 });
    w.write_event(SmlEvent::StartWorkbook);
    w.write_event(SmlEvent::StartWorksheet);
    w.write_event(SmlEvent::StartSheetData);
    for r in 0..rows {
        w.write_event(SmlEvent::StartRow {
            props: Box::new(GenRow {
                reference: Some((r + 1) as u32),
                ..Default::default()
            }),
        });
        w.write_event(SmlEvent::StartCell {
            props: Box::new(GenCell {
                reference: Some(format!("A{}", r + 1)),
                cell_type: Some(CellType::String),
                ..Default::default()
            }),
        });
        w.write_event(SmlEvent::CellValue(Cow::Owned(format!(
            "value-{}",
            r % DISTINCT_STRINGS
        ))));
        w.write_event(SmlEvent::EndCell);
        w.write_event(SmlEvent::StartCell {
            props: Box::new(GenCell {
                reference: Some(format!("B{}", r + 1)),
                ..Default::default()
            }),
        });
        w.write_event(SmlEvent::CellValue(Cow::Owned(format!("{r}.5"))));
        w.write_event(SmlEvent::EndCell);
        w.write_event(SmlEvent::EndRow);
    }
    w.write_event(SmlEvent::EndSheetData);
    w.write_event(SmlEvent::EndWorksheet);
    w.write_event(SmlEvent::EndWorkbook);
    w.finish().expect("finish");

    PEAK.load(Ordering::Relaxed).saturating_sub(before)
}

#[test]
fn writer_peak_memory_is_flat_in_row_count() {
    // Warm up so lazily-initialised globals do not land inside a measurement.
    let _ = peak_bytes_for(10);

    let small = peak_bytes_for(1_000).max(1);
    let large = peak_bytes_for(100_000);

    let ratio = large as f64 / small as f64;
    println!("peak live heap: 1k rows = {small} B, 100k rows = {large} B (ratio {ratio:.2}x)");
    assert!(
        ratio < 2.0,
        "peak live heap grew {ratio:.2}x for a 100x longer document \
         ({small} -> {large} bytes). The streaming writer must not accumulate \
         cells into a WorkbookBuilder; peak memory must track distinct \
         shared strings and sheet count, not row/cell count."
    );

    // Absolute ceiling, with headroom over the measured post-rework number
    // (~485 KB at 100k rows / 20 distinct strings on this machine): a
    // regression back toward the pre-rework ~223 MB would trip this even if
    // the ratio check above were somehow satisfied by coincidence.
    assert!(
        large < 5_000_000,
        "peak live heap for 100k rows was {large} B, expected well under 5 MB \
         for O(distinct strings + sheet count) memory"
    );
}
