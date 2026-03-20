#![no_main]

//! ANSI no-panic fuzz gate.
//!
//! Feeds arbitrary bytes to the rescribe ANSI reader and asserts it never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = rescribe_read_ansi::parse(s);
    }
});
