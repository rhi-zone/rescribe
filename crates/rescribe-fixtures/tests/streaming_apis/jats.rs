//! Streaming-API cross-checks for jats. Split out of the former monolithic
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

#[test]
fn jats_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("jats");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jats dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = jats_fmt::parse(&input);
        let expected = jats_fmt::events::events_from_doc(&doc);
        let actual: Vec<_> = jats_fmt::events(&input).map(|e| e.into_owned()).collect();
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
        "expected to check a substantial number of jats fixtures, got {checked}"
    );
    assert_or_known_failure("jats", "events", result);
}

#[test]
fn jats_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("jats");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jats dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<jats_fmt::OwnedEvent> =
            jats_fmt::events(&input).map(|e| e.into_owned()).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                jats_fmt::StreamingParser::new(|e: jats_fmt::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of jats fixtures, got {checked}"
    );

    // Deliberately NOT probed here: an arbitrary 50%-byte split of each real
    // fixture. That was this check's original design, and the matching
    // KNOWN_FAILURES entry documented exactly why it's fixture-shape-unaware:
    // a 50% split of fixture adv-malformed-xml lands mid-attribute-value
    // inside the still-open root start tag, so zero events delivered at that
    // exact split point is the correct, spec-conforming answer, not a
    // StreamingParser defect. See the hand-built probe below instead, which
    // guarantees an unambiguous complete-prefix boundary (same fix already
    // applied to fb2-fmt/texinfo/xwiki/textile-fmt/pod-fmt).
    if result.is_ok() {
        let probe_input = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><article><body><p>Hello</p>";
        let mut delivered: Vec<jats_fmt::OwnedEvent> = Vec::new();
        let mut parser = jats_fmt::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("jats", !delivered.is_empty());
    }
    assert_or_known_failure("jats", "streaming_parser", result);
}

#[test]
fn jats_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("jats");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jats dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = jats_fmt::parse(&input);
        let built = jats_fmt::emit(&doc);

        let mut w = jats_fmt::Writer::new(Vec::<u8>::new());
        for e in jats_fmt::events(&input) {
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
        "expected to check a substantial number of jats fixtures, got {checked}"
    );

    if result.is_ok() {
        use std::borrow::Cow;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = jats_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(jats_fmt::Event::StartElement {
            name: Cow::Borrowed("p"),
            attrs: vec![],
        });
        w.write_event(jats_fmt::Event::Text(Cow::Borrowed("Hello world")));
        w.write_event(jats_fmt::Event::EndElement {
            name: Cow::Borrowed("p"),
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
    assert_or_known_failure("jats", "streaming_writer", result);
}
