#![no_main]

//! multimarkdown-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to multimarkdown_fmt::parse. Must not panic
//! regardless of input — malformed MultiMarkdown is reported via
//! Diagnostics, never a panic. Also exercises events() and the batch/
//! streaming-parser API (which buffers to finish() — see batch.rs's module
//! docs for the sanctioned, inherited exemption this reflects).

use libfuzzer_sys::fuzz_target;
use multimarkdown_fmt::MmdDoc;
use rescribe_format_api::Parse as _;

fuzz_target!(|data: &[u8]| {
    let _ = MmdDoc::parse(data);

    if let Some(iter) = multimarkdown_fmt::events(data) {
        let _ = iter.count();
    }

    let split = if data.is_empty() { 0 } else { data.len() / 2 };
    let mut p = multimarkdown_fmt::StreamingParser::new(|_ev| {});
    p.feed(&data[..split]);
    p.feed(&data[split..]);
    p.finish();
});
