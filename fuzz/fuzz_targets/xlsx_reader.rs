#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Should never panic, regardless of input (including malformed ZIP/XLSX)
    let _ = rescribe_fmt_ooxml::xlsx::parse_bytes(data);
});
