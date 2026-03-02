#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Should never panic, regardless of input (including non-UTF-8 bytes)
    let _ = rtf_fmt::parse(data);
});
