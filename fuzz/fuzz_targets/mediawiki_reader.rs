#![no_main]
//! MediaWiki reader no-panic fuzz target.
//!
//! Feed arbitrary bytes to the MediaWiki parser; assert it never panics.
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = mediawiki_fmt::parse(s);
    }
});
