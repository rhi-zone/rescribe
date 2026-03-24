#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Tree-sitter backend: should never panic on any UTF-8 input
        let _ = rescribe_read_markdown::backend_treesitter::parse(s);
    }
});
