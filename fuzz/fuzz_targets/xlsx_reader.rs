#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Should never panic, regardless of input (including malformed ZIP/XLSX)
    let _ = rescribe_read_xlsx::parse_bytes(data);
});
