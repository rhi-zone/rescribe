#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Should never panic, regardless of input (including malformed ZIP/DOCX)
    let _ = rescribe_fmt_ooxml::docx::parse_bytes(data);

    // Same no-panic gate for ooxml-wml's genuinely incremental
    // `StreamingParser<H>` (crates/formats/ooxml-wml/src/batch.rs), fed in
    // two different chunkings so both the single-feed and adversarial
    // byte-at-a-time paths are exercised.
    let mut events = Vec::new();
    let mut p = ooxml_wml::batch::StreamingParser::new(|ev: ooxml_wml::OwnedWmlEvent| {
        events.push(ev);
    });
    p.feed(data);
    let _ = p.finish();

    let mut p = ooxml_wml::batch::StreamingParser::new(|_ev: ooxml_wml::OwnedWmlEvent| {});
    for chunk in data.chunks(1) {
        p.feed(chunk);
    }
    let _ = p.finish();
});
