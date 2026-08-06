#![no_main]

//! latex-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to latex_fmt::parse, events(), and the chunked
//! StreamingParser. Must not panic regardless of input — malformed LaTeX
//! is reported via Diagnostics (parse()) or simply produces some best-effort
//! token/event stream, never a panic.

use latex_fmt::LatexDoc;
use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Events as _, Parse as _};

fuzz_target!(|data: &[u8]| {
    let _ = LatexDoc::parse(data);
    let _ = LatexDoc::events(data).count();

    // Chunked batch parser, split at an arbitrary point so short inputs
    // still exercise a genuine multi-feed() call.
    let split = if data.is_empty() { 0 } else { data.len() / 2 };
    let mut p = latex_fmt::batch::StreamingParser::new(|_ev| {});
    p.feed(&data[..split]);
    p.feed(&data[split..]);
    p.finish();
});
