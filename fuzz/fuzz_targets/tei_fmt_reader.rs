#![no_main]

//! tei-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to tei_fmt::parse. Must not panic regardless of
//! input — malformed XML is reported via Diagnostics, never a panic.

use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Events as _, Parse as _};
use tei_fmt::TeiDoc;

fuzz_target!(|data: &[u8]| {
    let _ = TeiDoc::parse(data);
    // Also exercise the streaming events iterator on the same data.
    let _ = TeiDoc::events(data).count();
});
