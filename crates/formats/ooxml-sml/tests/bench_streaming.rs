//! Manual one-off measurement: peak live heap + wall time for SmlWriter
//! (streaming) vs WorkbookBuilder (builder), feeding an identical synthetic
//! workbook through both. Not a normal correctness test; run explicitly:
//!
//!   cargo test --release --test bench_streaming -- --ignored --nocapture
//!
//! Inputs are constructed OUTSIDE the timed/measured region. `black_box` is
//! used on the finish() result so the optimizer can't elide the work.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use ooxml_sml::generated::Cell as GenCell;
use ooxml_sml::generated::Row as GenRow;
use ooxml_sml::{SmlEvent, SmlWriter, WorkbookBuilder};

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

const ROWS: usize = 100_000;
const DISTINCT_STRINGS: usize = 20;

fn col_letter(col: usize) -> char {
    (b'A' + (col as u8)) as char
}

fn events_for(rows: usize) -> Vec<SmlEvent<'static>> {
    let mut evs = Vec::with_capacity(rows * 6 + 4);
    evs.push(SmlEvent::StartWorkbook);
    evs.push(SmlEvent::StartWorksheet);
    evs.push(SmlEvent::StartSheetData);
    for r in 0..rows {
        evs.push(SmlEvent::StartRow {
            props: Box::new(GenRow {
                reference: Some((r + 1) as u32),
                ..Default::default()
            }),
        });
        // Cell A: string, cycling through a small distinct set.
        let s = format!("value-{}", r % DISTINCT_STRINGS);
        evs.push(SmlEvent::StartCell {
            props: Box::new(GenCell {
                reference: Some(format!("{}{}", col_letter(0), r + 1)),
                cell_type: Some(ooxml_sml::generated::CellType::String),
                ..Default::default()
            }),
        });
        evs.push(SmlEvent::CellValue(s.into()));
        evs.push(SmlEvent::EndCell);
        // Cell B: number.
        evs.push(SmlEvent::StartCell {
            props: Box::new(GenCell {
                reference: Some(format!("{}{}", col_letter(1), r + 1)),
                ..Default::default()
            }),
        });
        evs.push(SmlEvent::CellValue(format!("{}.5", r).into()));
        evs.push(SmlEvent::EndCell);
        // Cell C: boolean.
        evs.push(SmlEvent::StartCell {
            props: Box::new(GenCell {
                reference: Some(format!("{}{}", col_letter(2), r + 1)),
                cell_type: Some(ooxml_sml::generated::CellType::Boolean),
                ..Default::default()
            }),
        });
        evs.push(SmlEvent::CellValue(
            if r % 2 == 0 { "1" } else { "0" }.into(),
        ));
        evs.push(SmlEvent::EndCell);
        evs.push(SmlEvent::EndRow);
    }
    evs.push(SmlEvent::EndSheetData);
    evs.push(SmlEvent::EndWorksheet);
    evs.push(SmlEvent::EndWorkbook);
    evs
}

fn measure_streaming(rows: usize) -> (usize, std::time::Duration) {
    let evs = events_for(rows);

    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    let start = Instant::now();

    let mut w = SmlWriter::new(NullSink { pos: 0, len: 0 });
    for e in evs {
        w.write_event(black_box(e));
    }
    let r = w.finish();
    let elapsed = start.elapsed();
    black_box(&r);
    r.expect("finish");

    (PEAK.load(Ordering::Relaxed).saturating_sub(before), elapsed)
}

fn measure_builder(rows: usize) -> (usize, std::time::Duration) {
    // Pre-build the (row, col, value) triples outside the timed region.
    struct Triple {
        ref_a: String,
        ref_b: String,
        ref_c: String,
        val_s: String,
        val_n: f64,
        val_b: bool,
    }
    // References and values are fully pre-formatted here, outside the timed
    // region, matching `events_for`'s setup on the streaming side — otherwise
    // the comparison would measure setup-cost asymmetry, not the writers.
    let data: Vec<Triple> = (0..rows)
        .map(|r| Triple {
            ref_a: format!("{}{}", col_letter(0), r + 1),
            ref_b: format!("{}{}", col_letter(1), r + 1),
            ref_c: format!("{}{}", col_letter(2), r + 1),
            val_s: format!("value-{}", r % DISTINCT_STRINGS),
            val_n: r as f64 + 0.5,
            val_b: r % 2 == 0,
        })
        .collect();

    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    let start = Instant::now();

    let mut wb = WorkbookBuilder::new();
    let sheet = wb.add_sheet("Sheet1");
    // Consumed by value (not `&data`) so no per-row clone of `val_s` is needed.
    for t in data {
        sheet.set_cell(&t.ref_a, t.val_s);
        sheet.set_cell(&t.ref_b, t.val_n);
        sheet.set_cell(&t.ref_c, t.val_b);
    }
    let mut out = Vec::new();
    let r = wb.write(std::io::Cursor::new(&mut out));
    let elapsed = start.elapsed();
    black_box(&r);
    r.expect("write");
    black_box(&out);

    (PEAK.load(Ordering::Relaxed).saturating_sub(before), elapsed)
}

#[test]
#[ignore]
fn manual_bench() {
    let _ = measure_streaming(1_000); // warm up

    let (peak_s, dur_s) = measure_streaming(ROWS);
    let (peak_b, dur_b) = measure_builder(ROWS);

    println!(
        "rows = {ROWS}, 3 cells/row ({} distinct strings)",
        DISTINCT_STRINGS
    );
    println!(
        "streaming: peak={} B ({:.2} MB), time={:?}",
        peak_s,
        peak_s as f64 / 1_048_576.0,
        dur_s
    );
    println!(
        "builder:   peak={} B ({:.2} MB), time={:?}",
        peak_b,
        peak_b as f64 / 1_048_576.0,
        dur_b
    );
    println!(
        "peak ratio (builder/streaming) = {:.2}x",
        peak_b as f64 / peak_s.max(1) as f64
    );
    println!(
        "throughput ratio (builder_time/streaming_time) = {:.2}x",
        dur_b.as_secs_f64() / dur_s.as_secs_f64()
    );
}
