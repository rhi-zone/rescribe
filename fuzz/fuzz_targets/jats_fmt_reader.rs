#![no_main]

//! jats-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to jats_fmt::parse. Must not panic regardless of
//! input — malformed XML is reported via Diagnostics, never a panic.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = jats_fmt::parse(data);
    // Also exercise the streaming events iterator on the same data.
    let _ = jats_fmt::events(data).count();
});
