//! Compare the event-driven `WmlWriter` against the `DocumentBuilder` path.
//!
//! Both routes start from the same pre-built `Vec<String>` of paragraph texts —
//! prepared outside the timed region — and end with a complete DOCX image in a
//! discarding sink. Each route builds its own per-paragraph values inside the
//! timed region, because that construction is part of what the route costs.
//!
//! Run with `cargo run --release --example streaming_writer_throughput`.

use std::borrow::Cow;
use std::hint::black_box;
use std::io::{Seek, SeekFrom, Write};
use std::time::Instant;

use ooxml_wml::writer::DocumentBuilder;
use ooxml_wml::{WmlEvent, WmlWriter};

/// Discards bytes but tracks length, so I/O does not dominate the measurement.
struct NullSink {
    pos: u64,
    len: u64,
}

impl NullSink {
    fn new() -> Self {
        NullSink { pos: 0, len: 0 }
    }
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

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);

    // Prepared once, outside every timed region.
    let texts: Vec<String> = (0..n)
        .map(|i| format!("paragraph {i} with enough text to be worth measuring"))
        .collect();

    // Warm up both paths so first-touch costs do not land in a measurement.
    run_streaming(&texts[..texts.len().min(1000)]);
    run_builder(&texts[..texts.len().min(1000)]);

    let mut streaming = Vec::new();
    let mut builder = Vec::new();
    for _ in 0..5 {
        streaming.push(run_streaming(&texts));
        builder.push(run_builder(&texts));
    }
    streaming.sort_unstable();
    builder.sort_unstable();
    let s = streaming[streaming.len() / 2];
    let b = builder[builder.len() / 2];

    println!("{n} paragraphs, median of 5:");
    println!("  WmlWriter (streaming): {:>8.1} ms", s as f64 / 1e6);
    println!("  DocumentBuilder      : {:>8.1} ms", b as f64 / 1e6);
    println!("  streaming / builder  : {:>8.2}x", s as f64 / b as f64);
}

fn run_streaming(texts: &[String]) -> u128 {
    let start = Instant::now();
    let mut w = WmlWriter::new(NullSink::new());
    w.write_event(WmlEvent::StartDocument);
    for t in texts {
        w.write_event(WmlEvent::StartParagraph {
            props: Box::default(),
        });
        w.write_event(WmlEvent::StartRun {
            props: Box::default(),
        });
        w.write_event(WmlEvent::Text(Cow::Borrowed(t.as_str())));
        w.write_event(WmlEvent::EndRun);
        w.write_event(WmlEvent::EndParagraph);
    }
    w.write_event(WmlEvent::EndDocument);
    w.finish().expect("streaming finish");
    let elapsed = start.elapsed().as_nanos();
    black_box(&elapsed);
    elapsed
}

fn run_builder(texts: &[String]) -> u128 {
    let start = Instant::now();
    let mut b = DocumentBuilder::new();
    for t in texts {
        b.add_paragraph(t);
    }
    b.write(NullSink::new()).expect("builder write");
    let elapsed = start.elapsed().as_nanos();
    black_box(&elapsed);
    elapsed
}
