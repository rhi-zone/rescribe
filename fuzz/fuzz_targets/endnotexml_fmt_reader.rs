#![no_main]

//! endnotexml-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to endnotexml_fmt::parse. Must not panic
//! regardless of input — malformed EndNote XML/XML is reported via
//! Diagnostics, never a panic.

use endnotexml_fmt::EndNoteDoc;
use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Events as _, Parse as _};

fuzz_target!(|data: &[u8]| {
    let _ = EndNoteDoc::parse(data);
    // Also exercise the streaming events iterator on the same data.
    let _ = EndNoteDoc::events(data).count();

    // And the chunked batch parser, split at an arbitrary point so short
    // inputs still exercise a genuine multi-feed() call.
    let split = if data.is_empty() { 0 } else { data.len() / 2 };
    let mut p = endnotexml_fmt::StreamingParser::new(|_ev| {});
    p.feed(&data[..split]);
    p.feed(&data[split..]);
    let _ = p.finish();
});
