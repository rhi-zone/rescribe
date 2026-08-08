//! Fuzz `ooxml_sml::StreamingParser` against arbitrary bytes.
//!
//! This target tests that the genuinely incremental, chunk-fed XLSX reader
//! (built on `ooxml_opc::StreamingParser` + per-part SAX parsing of
//! worksheet/sharedStrings XML) never panics, regardless of input —
//! including malformed ZIP/OPC/XML content, truncated archives, and
//! byte-at-a-time feeding.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ooxml_sml::StreamingParser;

fuzz_target!(|data: &[u8]| {
    // Feed the whole input in one shot.
    {
        let mut events = Vec::new();
        let mut p = StreamingParser::new(|ev| events.push(ev));
        p.feed(data);
        let _diags = p.finish();
    }

    // Feed byte-at-a-time to exercise chunk-boundary handling at every
    // possible split point (bounded so pathologically large fuzz inputs
    // don't make this target itself slow).
    if data.len() <= 4096 {
        let mut events = Vec::new();
        let mut p = StreamingParser::new(|ev| events.push(ev));
        for byte in data {
            p.feed(std::slice::from_ref(byte));
        }
        let _diags = p.finish();
    }
});
