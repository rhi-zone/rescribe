#![no_main]

//! Haddock no-panic fuzz gate.
//!
//! Feeds arbitrary bytes to the rescribe Haddock reader and asserts it never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = rescribe_read_haddock::parse(s);
    }
});
