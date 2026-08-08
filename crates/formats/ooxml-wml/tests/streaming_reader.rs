//! Tests for `ooxml_wml::batch::StreamingParser<H>` — the genuinely
//! incremental, chunk-fed DOCX reader added alongside `BatchParser` and
//! `events()`.
//!
//! Coverage:
//! - No-panic gate: arbitrary bytes, fed in arbitrary chunk sizes, must
//!   never panic.
//! - Adversarial chunking: a real fixture `.docx`, fed at several odd
//!   chunk-size boundaries (including 1 byte at a time), must produce the
//!   exact same [`OwnedWmlEvent`] sequence as `events()` does when handed
//!   the same `word/document.xml` bytes directly, and the same sequence
//!   regardless of chunk size.

use ooxml_wml::batch::StreamingParser;
use ooxml_wml::{OwnedWmlEvent, wml_events};
use std::io::Cursor;
use std::path::Path;

fn fixtures_dir() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/ooxml/wml"
    ))
}

/// Every fixture `.docx` file under `fixtures/ooxml/wml/**`.
fn all_fixture_docx() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let root = fixtures_dir();
    let Ok(top) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in top.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for f in rd.flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "docx") {
                out.push(p);
            }
        }
    }
    out
}

/// Collect the `OwnedWmlEvent` sequence `StreamingParser` produces when fed
/// `bytes` in `chunk_size`-byte pieces (last chunk may be smaller).
fn collect_streaming(bytes: &[u8], chunk_size: usize) -> (Vec<OwnedWmlEvent>, usize) {
    let mut events = Vec::new();
    let mut p = StreamingParser::new(|ev: OwnedWmlEvent| events.push(ev));
    if chunk_size == 0 {
        p.feed(bytes);
    } else {
        for chunk in bytes.chunks(chunk_size) {
            p.feed(chunk);
        }
    }
    let diags = p.finish();
    (events, diags.len())
}

/// Expected events: read `word/document.xml` via the seekable `Package`
/// and run it through `events()` directly — the independent pull-iterator
/// implementation `StreamingParser` must agree with.
fn expected_events(docx_bytes: &[u8]) -> Vec<OwnedWmlEvent> {
    let mut pkg = ooxml_opc::Package::open(Cursor::new(docx_bytes.to_vec())).unwrap();
    let doc_xml = pkg.read_part("word/document.xml").unwrap();
    wml_events(&doc_xml).map(|e| e.into_owned()).collect()
}

fn owned_events_eq(a: &[OwnedWmlEvent], b: &[OwnedWmlEvent]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| format!("{x:?}") == format!("{y:?}"))
}

#[test]
fn streaming_parser_matches_events_across_chunk_sizes_for_all_fixtures() {
    let fixtures = all_fixture_docx();
    assert!(
        !fixtures.is_empty(),
        "expected at least one fixture .docx under {:?}",
        fixtures_dir()
    );

    for path in fixtures {
        let bytes = std::fs::read(&path).unwrap();
        let expected = expected_events(&bytes);

        for chunk_size in [1usize, 3, 7, 64, 4096, 0 /* whole-input feed */] {
            let (actual, diag_count) = collect_streaming(&bytes, chunk_size);
            assert_eq!(
                diag_count,
                0,
                "{}: unexpected diagnostics at chunk_size={chunk_size}",
                path.display()
            );
            assert!(
                owned_events_eq(&actual, &expected),
                "{}: StreamingParser output diverged from events() at chunk_size={chunk_size}\n\
                 expected ({} events): {:#?}\n\
                 actual ({} events):   {:#?}",
                path.display(),
                expected.len(),
                expected,
                actual.len(),
                actual
            );
        }
    }
}

#[test]
fn streaming_parser_matches_events_on_a_single_fixture_byte_at_a_time() {
    // Focused single-fixture check with the most adversarial chunk size
    // (1 byte), kept separate from the sweep above so a failure here is
    // fast to isolate.
    let fixtures = all_fixture_docx();
    let Some(path) = fixtures.first() else {
        return;
    };
    let bytes = std::fs::read(path).unwrap();
    let expected = expected_events(&bytes);
    let (actual, diags) = collect_streaming(&bytes, 1);
    assert_eq!(diags, 0);
    assert!(owned_events_eq(&actual, &expected));
}

#[test]
fn streaming_parser_no_panic_on_arbitrary_bytes() {
    // Deterministic pseudo-random byte soup — not a full fuzzing corpus
    // (that's fuzz/fuzz_targets/docx_reader.rs's job), but a fast, always-
    // run regression gate: arbitrary (including totally malformed
    // ZIP/OPC/XML) bytes must never panic, at any chunk size.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for trial in 0..64u32 {
        let len = (next() % 4096) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| (next() % 256) as u8).collect();
        let chunk_size = 1 + (next() % 37) as usize;

        let mut events: Vec<OwnedWmlEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev: OwnedWmlEvent| events.push(ev));
        for chunk in bytes.chunks(chunk_size) {
            p.feed(chunk);
        }
        let _diags = p.finish();
        // Reaching here without panicking is the assertion; `trial` and
        // `events` are inspected only to keep them live under -D warnings.
        let _ = (trial, events.len());
    }
}

#[test]
fn streaming_parser_no_panic_on_empty_input() {
    let events: Vec<OwnedWmlEvent> = Vec::new();
    let mut events = events;
    let p = StreamingParser::new(|ev: OwnedWmlEvent| events.push(ev));
    let diags = p.finish();
    assert!(events.is_empty());
    assert!(diags.is_empty());
}

#[test]
fn streaming_parser_ignores_parts_other_than_main_document() {
    // A fixture with images/styles/etc. must still produce only the
    // word/document.xml-derived events — no events from any other part.
    let fixtures = all_fixture_docx();
    let Some(path) = fixtures
        .iter()
        .find(|p| p.to_string_lossy().contains("image"))
    else {
        return;
    };
    let bytes = std::fs::read(path).unwrap();
    let expected = expected_events(&bytes);
    let (actual, _diags) = collect_streaming(&bytes, 512);
    assert!(owned_events_eq(&actual, &expected));
}
