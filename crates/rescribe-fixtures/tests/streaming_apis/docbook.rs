//! Streaming-API cross-checks for docbook. Split out of the former monolithic
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
// docbook-fmt / jats-fmt / tei-fmt: three byte-identical-shape XML-passthrough
// crates (verified via `diff` across batch.rs/writer.rs — only doc comments
// and AST/event type names differ). Unlike html-fmt, XML is well-nested by
// construction, so all three APIs beyond parse()/emit() are genuinely
// independent, incremental implementations, not parse()-then-wrap fakes:
//
// - `events()` (`EventIter`) wraps `quick_xml::Reader` directly and pulls one
//   token at a time from the input slice; it never materializes an AST.
// - `StreamingParser<H>` (batch.rs's `drain()`) dispatches every event it can
//   prove is complete from the buffered-so-far bytes, shrinking the buffer as
//   tokens are consumed — bounded by the largest in-progress token, not the
//   whole input; a documented ambiguous-text-at-chunk-boundary rule holds
//   text events back only when the buffer's end coincides with input's end.
// - The streaming `Writer` calls `quick_xml::Writer` directly per event, no
//   buffering.
//
// Each crate also exports `events::events_from_doc(&Ast) -> Vec<OwnedEvent>`
// as a documented, crate-provided (not test-hand-rolled) AST->events
// projection used by the crate's own round-trip tests — using it here as the
// equivalence oracle is stronger than a hand-rolled projection since it's
// exactly the mapping the crate itself commits to.
// ---------------------------------------------------------------------------

#[test]
fn docbook_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("docbook");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/docbook dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = docbook_fmt::parse(&input);
        let expected = docbook_fmt::events::events_from_doc(&doc);
        let actual: Vec<_> = docbook_fmt::events(&input)
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
        "expected to check a substantial number of docbook fixtures, got {checked}"
    );
    assert_or_known_failure("docbook", "events", result);
}

#[test]
fn docbook_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("docbook");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/docbook dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<docbook_fmt::OwnedEvent> = docbook_fmt::events(&input)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                docbook_fmt::StreamingParser::new(|e: docbook_fmt::OwnedEvent| streamed.push(e));
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

        if input.len() > 32 && !bulk.is_empty() {
            let mid = input.len() / 2;
            let mut delivered: Vec<docbook_fmt::OwnedEvent> = Vec::new();
            let mut parser = docbook_fmt::StreamingParser::new(|e| delivered.push(e));
            parser.feed(&input[..mid]);
            if delivered.is_empty() && result.is_ok() {
                result = Err(format!(
                    "StreamingParser delivered zero events to the handler after feed() with \
                     half of fixture {name} ({mid} bytes) and before finish() — expected real \
                     incremental delivery per batch.rs's drain() design"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of docbook fixtures, got {checked}"
    );
    assert_or_known_failure("docbook", "streaming_parser", result);
}

#[test]
fn docbook_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("docbook");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/docbook dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = docbook_fmt::parse(&input);
        let built = docbook_fmt::emit(&doc);

        let mut w = docbook_fmt::Writer::new(Vec::<u8>::new());
        for e in docbook_fmt::events(&input) {
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
        "expected to check a substantial number of docbook fixtures, got {checked}"
    );

    if result.is_ok() {
        use std::borrow::Cow;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = docbook_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(docbook_fmt::Event::StartElement {
            name: Cow::Borrowed("para"),
            attrs: vec![],
        });
        w.write_event(docbook_fmt::Event::Text(Cow::Borrowed("Hello world")));
        w.write_event(docbook_fmt::Event::EndElement {
            name: Cow::Borrowed("para"),
        });
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink before finish() for a complete \
                 StartElement/Text/EndElement sequence — expected genuine incremental \
                 writing per writer.rs's direct quick_xml::Writer calls"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("docbook", "streaming_writer", result);
}
