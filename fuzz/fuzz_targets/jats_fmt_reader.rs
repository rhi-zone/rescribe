#![no_main]

//! jats-fmt no-panic gate.
//!
//! Feeds arbitrary bytes to jats_fmt::parse. Must not panic regardless of
//! input — malformed XML is reported via Diagnostics, never a panic.

use jats_fmt::JatsDoc;
use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Events as _, Parse as _};

fuzz_target!(|data: &[u8]| {
    let _ = JatsDoc::parse(data);
    // Also exercise the streaming events iterator on the same data.
    let _ = JatsDoc::events(data).count();
});
