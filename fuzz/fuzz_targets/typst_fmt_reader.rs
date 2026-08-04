#![no_main]

//! typst-fmt no-panic gate.
//!
//! Feeds arbitrary bytes (as UTF-8, when valid) to typst_fmt::parse. Must
//! not panic regardless of input — malformed Typst is reported via
//! Diagnostics, never a panic. Also exercises events() and the batch/
//! streaming-parser APIs (both of which buffer to finish() — see batch.rs's
//! module docs for the sanctioned exemption this reflects).

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let _ = typst_fmt::parse(s);
    // Also exercise the streaming events iterator on the same input.
    let _ = typst_fmt::events(s).count();

    // And the chunked batch parser, split at an arbitrary point so short
    // inputs still exercise a genuine multi-feed() call.
    let split = if data.is_empty() { 0 } else { data.len() / 2 };
    let mut p = typst_fmt::StreamingParser::new(|_ev| {});
    p.feed(&data[..split]);
    p.feed(&data[split..]);
    let _ = p.finish();
});
