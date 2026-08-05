#![no_main]

//! zip-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to every reader API: `parse()`, `events()`, and
//! the hand-rolled `StreamingParser`. None may panic regardless of input —
//! a malformed/truncated ZIP archive is reported via `Diagnostic`s, never
//! a panic. Arbitrary bytes are extremely unlikely to look like a valid
//! ZIP local file header, so this target mostly exercises the "reject
//! gracefully" paths; `zip_fmt_roundtrip.rs` is what actually drives the
//! parsers over well-formed archives.

use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Events as _, Parse as _};
use zip_fmt::Archive;

fuzz_target!(|data: &[u8]| {
    let _ = Archive::parse(data);
    let _ = Archive::events(data).count();

    // Chunked batch parser, split at an arbitrary point so short inputs
    // still exercise a genuine multi-feed() call.
    let split = if data.is_empty() { 0 } else { data.len() / 2 };
    let mut p = zip_fmt::StreamingParser::new(|_ev| {});
    p.feed(&data[..split]);
    p.feed(&data[split..]);
    let _ = p.finish();
});
