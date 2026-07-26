#![no_main]

//! tei-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to tei_fmt::parse. Must not panic regardless of
//! input — malformed XML is reported via Diagnostics, never a panic.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tei_fmt::parse(data);
    // Also exercise the streaming events iterator on the same data.
    let _ = tei_fmt::events(data).count();
});
