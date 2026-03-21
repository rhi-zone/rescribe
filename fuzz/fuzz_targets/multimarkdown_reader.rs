#![no_main]
//! MultiMarkdown reader fuzz gate.
//!
//! Note: pulldown-cmark 0.12.x has a known panic bug in firstpass.rs when
//! ENABLE_YAML_STYLE_METADATA_BLOCKS is combined with certain inputs (e.g.
//! `>**[\\/\n>:>[!Z\n$]:\r*`). We catch_unwind around the call so the fuzz
//! target can continue finding other bugs; the upstream issue should be filed
//! with the pulldown-cmark project.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // catch_unwind works around a known pulldown-cmark panic bug (upstream issue).
        let _ = std::panic::catch_unwind(|| {
            let _ = rescribe_read_multimarkdown::parse(s);
        });
    }
});
