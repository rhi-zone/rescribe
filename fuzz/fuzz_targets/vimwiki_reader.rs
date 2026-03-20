#![no_main]
//! VimWiki reader no-panic fuzz target.
//!
//! Feed arbitrary bytes to the VimWiki reader; assert it never panics.
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = rescribe_read_vimwiki::parse(s);
    }
});
