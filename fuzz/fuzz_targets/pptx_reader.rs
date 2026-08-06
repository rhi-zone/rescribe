#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Should never panic, regardless of input (including malformed ZIP/PPTX)
    let _ = rescribe_fmt_ooxml::pptx::parse(data);
});
