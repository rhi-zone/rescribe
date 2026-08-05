#![no_main]
//! MediaWiki reader no-panic fuzz target.
//!
//! Feed arbitrary bytes to the MediaWiki parser; assert it never panics.
use libfuzzer_sys::fuzz_target;
use mediawiki_fmt::MediawikiDoc;
use rescribe_format_api::Parse as _;
fuzz_target!(|data: &[u8]| {
    let _ = MediawikiDoc::parse(data);
});
