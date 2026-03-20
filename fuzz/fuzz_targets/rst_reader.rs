#![no_main]

//! RST reader no-panic fuzz target.
//!
//! Feeds arbitrary UTF-8 strings into the RST reader and asserts it never panics.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = rescribe_read_rst::parse(s);
    }
});
