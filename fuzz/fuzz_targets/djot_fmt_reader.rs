#![no_main]

//! djot-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to djot_fmt::parse. Must not panic regardless of input.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = djot_fmt::parse(s);
    }
});
