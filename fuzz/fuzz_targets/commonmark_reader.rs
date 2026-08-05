#![no_main]

//! commonmark-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to commonmark_fmt::parse and commonmark_fmt::events.
//! Must not panic regardless of input.

use commonmark_fmt::CmDoc;
use libfuzzer_sys::fuzz_target;
use rescribe_format_api::Parse as _;

fuzz_target!(|data: &[u8]| {
    let _ = CmDoc::parse(data);
    if let Some(iter) = commonmark_fmt::events(data) {
        for _ in iter {}
    }
});
