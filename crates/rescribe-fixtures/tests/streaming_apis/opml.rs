//! Streaming-API cross-checks for opml. Split out of the former monolithic
//! `streaming_apis.rs` (see `crates/rescribe-fixtures/tests/streaming_apis.rs`
//! for the harness overview and `common.rs` for shared helpers) so concurrent
//! per-format edits stop colliding on one file.

#[allow(unused_imports)]
use crate::common::{assert_streaming_parser_is_incremental, find_input, fixtures_root};
#[allow(unused_imports)]
use rescribe_fixtures::streaming_harness::{
    CAPABILITIES, NOT_YET_AUDITED, ObservableSink, adversarial_chunkings, assert_or_known_failure,
};
#[allow(unused_imports)]
use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// opml-fmt: well-nested XML like docbook-fmt/jats-fmt/tei-fmt above, but with
// a domain-typed AST/event vocabulary (OpmlDoc/Head/Body/Outline,
// StartOutline/HeadField/...) rather than a generic element tree — OPML's
// grammar is small and fixed, unlike DocBook/JATS/TEI's hundreds of
// document-specific element names. `events()` (`EventIter`) wraps
// `quick_xml::Reader` directly; `StreamingParser<H>` (batch.rs's `drain()`)
// dispatches every event it can prove complete from the buffered-so-far
// bytes; the streaming `Writer` calls `quick_xml::Writer` directly per
// event. `opml_fmt::events::events_from_doc(&OpmlDoc)` is the crate's own
// documented AST->events projection, used here as the equivalence oracle.
// ---------------------------------------------------------------------------

#[test]
fn opml_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("opml");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/opml dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = opml_fmt::OpmlDoc::parse(&input);
        let expected = opml_fmt::events::events_from_doc(&doc);
        let actual: Vec<_> = opml_fmt::OpmlDoc::events(&input)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from events_from_doc(&parse(input)) for fixture {name}:\n  \
                 ast-derived: {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of opml fixtures, got {checked}"
    );
    assert_or_known_failure("opml", "events", result);
}

#[test]
fn opml_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("opml");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/opml dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<opml_fmt::OwnedEvent> = opml_fmt::OpmlDoc::events(&input)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                opml_fmt::StreamingParser::new(|e: opml_fmt::OwnedEvent| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of opml fixtures, got {checked}"
    );

    // Hand-built probe with a provably complete prefix (same rationale as
    // jats-fmt's identical probe above — an arbitrary 50%-byte split of a
    // real fixture can legitimately land mid-attribute-value, which is not a
    // StreamingParser defect).
    if result.is_ok() {
        let probe_input =
            b"<?xml version=\"1.0\"?><opml version=\"2.0\"><body><outline text=\"Hello\"/>";
        let mut delivered: Vec<opml_fmt::OwnedEvent> = Vec::new();
        let mut parser = opml_fmt::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("opml", !delivered.is_empty());
    }
    assert_or_known_failure("opml", "streaming_parser", result);
}

#[test]
fn opml_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("opml");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/opml dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = opml_fmt::OpmlDoc::parse(&input);
        let built = doc.emit();

        let mut w = opml_fmt::Writer::new(Vec::<u8>::new());
        for e in opml_fmt::OpmlDoc::events(&input) {
            w.write_event(e);
        }
        let streamed = w.finish();

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of opml fixtures, got {checked}"
    );

    if result.is_ok() {
        use std::borrow::Cow;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = opml_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(opml_fmt::Event::StartOpml {
            version: Cow::Borrowed("2.0"),
        });
        w.write_event(opml_fmt::Event::StartBody);
        w.write_event(opml_fmt::Event::EmptyOutline {
            attrs: vec![("text".to_string(), Cow::Borrowed("Hello world"))],
        });
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink before finish() for a complete \
                 StartOpml/StartBody/EmptyOutline sequence — expected genuine incremental \
                 writing per writer.rs's direct quick_xml::Writer calls"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("opml", "streaming_writer", result);
}
