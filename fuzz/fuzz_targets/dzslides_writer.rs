#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(result) = rescribe_read_markdown::parse(s) {
            let _ = rescribe_write_dzslides::emit(&result.value);
        }
    }
});
