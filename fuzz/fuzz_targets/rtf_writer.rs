#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Parse RTF into a rescribe Document, then emit it as RTF.
        // Must never panic regardless of what the reader produces.
        if let Ok(result) = rescribe_read_rtf::parse(s) {
            let _ = rescribe_write_rtf::emit(&result.value);
        }
    }
});
