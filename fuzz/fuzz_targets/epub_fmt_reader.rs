#![no_main]

//! epub-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to every reader API: `parse()`, `events()`, and
//! the chunk-fed `StreamingParser`. None may panic regardless of input —
//! a malformed/truncated EPUB (or non-ZIP garbage) is reported via
//! `Diagnostic`s, never a panic.

use epub_fmt::EpubDoc;
use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Events as _, Parse as _};

fuzz_target!(|data: &[u8]| {
    let _ = EpubDoc::parse(data);
    let _ = EpubDoc::events(data).count();

    let split = if data.is_empty() { 0 } else { data.len() / 2 };
    let mut p = epub_fmt::StreamingParser::new(|_ev| {});
    p.feed(&data[..split]);
    p.feed(&data[split..]);
    let _ = p.finish();
});
