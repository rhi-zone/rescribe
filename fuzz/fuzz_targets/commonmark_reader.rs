#![no_main]

//! commonmark-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to commonmark_fmt::parse and commonmark_fmt::events.
//! Must not panic regardless of input.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = commonmark_fmt::parse(data);
    if let Some(iter) = commonmark_fmt::events(data) {
        for _ in iter {}
    }
});
