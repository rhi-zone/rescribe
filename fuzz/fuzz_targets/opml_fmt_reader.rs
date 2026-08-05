#![no_main]

//! opml-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to opml_fmt::parse. Must not panic regardless of
//! input — malformed OPML/XML is reported via Diagnostics, never a panic.

use libfuzzer_sys::fuzz_target;
use opml_fmt::OpmlDoc;
use rescribe_format_api::{Events as _, Parse as _};

fuzz_target!(|data: &[u8]| {
    let _ = OpmlDoc::parse(data);
    // Also exercise the streaming events iterator on the same data.
    let _ = OpmlDoc::events(data).count();

    // And the chunked batch parser, split at an arbitrary point so short
    // inputs still exercise a genuine multi-feed() call.
    let split = if data.is_empty() { 0 } else { data.len() / 2 };
    let mut p = opml_fmt::StreamingParser::new(|_ev| {});
    p.feed(&data[..split]);
    p.feed(&data[split..]);
    let _ = p.finish();
});
