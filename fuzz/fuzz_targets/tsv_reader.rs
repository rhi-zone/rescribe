#![no_main]
//! TSV reader no-panic fuzz target.
//!
//! Feed arbitrary bytes to the TSV parser; assert it never panics.
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = tsv_fmt::parse(s);
    }
});
