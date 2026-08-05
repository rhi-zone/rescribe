#![no_main]

//! docbook-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to docbook_fmt::parse. Must not panic regardless of
//! input — malformed XML is reported via Diagnostics, never a panic.

use docbook_fmt::DocBookDoc;
use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Events as _, Parse as _};

fuzz_target!(|data: &[u8]| {
    let _ = DocBookDoc::parse(data);
    // Also exercise the streaming events iterator on the same data.
    let _ = DocBookDoc::events(data).count();
});
