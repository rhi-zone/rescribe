#![no_main]

use libfuzzer_sys::fuzz_target;
use rescribe_format_api::Parse as _;

fuzz_target!(|data: &[u8]| {
    // Should never panic, regardless of input (including non-UTF-8 bytes)
    let _ = rtf_fmt::RtfDoc::parse(data);
});
