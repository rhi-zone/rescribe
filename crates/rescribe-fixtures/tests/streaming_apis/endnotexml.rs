//! Streaming-API cross-checks for endnotexml. Split out of the former monolithic
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
// endnotexml-fmt: well-nested XML like opml-fmt/docbook-fmt/jats-fmt/tei-fmt
// above. Every known EndNote container (record/contributors/an author-role
// list/titles/periodical/urls/a url-role list/keywords/dates/pub-dates/
// foreign-keys) gets its own Start*/End* event pair; a leaf field or any
// unrecognized element becomes StartElement{name,attrs}/inline-content/
// EndElement, keyed by its exact source tag name (see ast.rs/events.rs
// module docs). `endnotexml_fmt::events::events_from_doc(&EndNoteDoc)` is
// the crate's own documented AST->events projection, used here as the
// equivalence oracle.
// ---------------------------------------------------------------------------

#[test]
fn endnotexml_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("endnotexml");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/endnotexml dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = endnotexml_fmt::EndNoteDoc::parse(&input);
        let expected = endnotexml_fmt::events::events_from_doc(&doc);
        let actual: Vec<_> = endnotexml_fmt::EndNoteDoc::events(&input)
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
        checked > 5,
        "expected to check a substantial number of endnotexml fixtures, got {checked}"
    );
    assert_or_known_failure("endnotexml", "events", result);
}

#[test]
fn endnotexml_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("endnotexml");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/endnotexml dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<endnotexml_fmt::OwnedEvent> = endnotexml_fmt::EndNoteDoc::events(&input)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                endnotexml_fmt::StreamingParser::new(|e: endnotexml_fmt::OwnedEvent| {
                    streamed.push(e)
                });
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
        checked > 5,
        "expected to check a substantial number of endnotexml fixtures, got {checked}"
    );

    // Hand-built probe with a provably complete prefix (same rationale as
    // opml-fmt's identical probe above — an arbitrary byte split of a real
    // fixture can legitimately land mid-token, which is not a
    // StreamingParser defect).
    if result.is_ok() {
        let probe_input = b"<xml><records><record><ref-type>17</ref-type>";
        let mut delivered: Vec<endnotexml_fmt::OwnedEvent> = Vec::new();
        let mut parser = endnotexml_fmt::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("endnotexml", !delivered.is_empty());
    }
    assert_or_known_failure("endnotexml", "streaming_parser", result);
}

#[test]
fn endnotexml_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("endnotexml");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/endnotexml dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = endnotexml_fmt::EndNoteDoc::parse(&input);
        let built = doc.emit();

        let mut w = endnotexml_fmt::Writer::new(Vec::<u8>::new());
        for e in endnotexml_fmt::EndNoteDoc::events(&input) {
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
        checked > 5,
        "expected to check a substantial number of endnotexml fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = endnotexml_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(endnotexml_fmt::Event::StartDocument);
        w.write_event(endnotexml_fmt::Event::StartRecord);
        w.write_event(endnotexml_fmt::Event::StartElement {
            name: "ref-type".into(),
            attrs: vec![],
        });
        w.write_event(endnotexml_fmt::Event::Text("17".into()));
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink before finish() for a complete \
                 StartDocument/StartRecord/StartElement/Text sequence — expected genuine \
                 incremental writing per writer.rs's direct quick_xml::Writer calls"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("endnotexml", "streaming_writer", result);
}
