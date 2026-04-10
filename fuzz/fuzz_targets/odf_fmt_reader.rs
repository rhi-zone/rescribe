#![no_main]

//! odf-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to odf_fmt::parse. Must not panic regardless of input.
//! ODF is a ZIP archive; most random inputs will fail ZIP parsing — that is fine
//! as long as no panic occurs.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = odf_fmt::parse(data);
    // Also exercise the events iterator on the same data.
    let _ = odf_fmt::events(data).count();
});
