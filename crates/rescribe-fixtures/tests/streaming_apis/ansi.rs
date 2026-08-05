//! Streaming-API cross-checks for ansi. Split out of the former monolithic
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
// ansi-fmt: events(), StreamingParser, and the streaming Writer are all
// genuinely independent, incremental implementations (verified by reading
// events.rs/batch.rs/writer.rs directly, not assumed from crate docs):
// EventIter advances its own position through the byte slice per next() call;
// StreamingParser::drain_complete only re-parses the *safe* prefix of its
// buffer (up to the last position that could not be an in-progress escape
// sequence, via find_safe_boundary/is_complete_escape) and drains it, so
// memory is bounded by the longest in-progress escape sequence, not the
// whole input; Writer::write_event writes straight to the sink per event
// with no buffering.
//
// No `events`-vs-AST-projection check is wired: parse()'s AnsiNode has no
// variant for a bare SGR sequence at all — apply_sgr() (parse.rs) folds SGR
// codes into a running `style` variable and returns `(None, pos)`, so a run
// of SGR codes not immediately followed by text produces zero AST nodes,
// while events()'s EventIter unconditionally emits one SetStyle/ResetStyle
// event per 'm'-terminated CSI group regardless of what follows. This is the
// reverse of the usual "events() drops information the AST has" defect
// shape: here parse()'s own AST is the lossier side, so there is no way to
// reconstruct a faithful ast_to_events projection purely from the AST for
// inputs with standalone/trailing SGR sequences (fixtures adv-bare-esc,
// adv-esc-eof and similar) — the two implementations are independent but not
// equally expressive, not a bug in either one. See TODO.md.
// ---------------------------------------------------------------------------

#[test]
fn ansi_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("ansi");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/ansi dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<ansi_fmt::OwnedEvent> = ansi_fmt::AnsiDoc::events(&input)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                ansi_fmt::batch::StreamingParser::new(|e: ansi_fmt::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of ansi fixtures, got {checked}"
    );
    assert_or_known_failure("ansi", "streaming_parser", result);
}

#[test]
fn ansi_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("ansi");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/ansi dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = ansi_fmt::AnsiDoc::parse(&input);
        let built_bytes = doc.emit();
        let built = String::from_utf8_lossy(&built_bytes).into_owned();

        let mut w = ansi_fmt::Writer::new(Vec::<u8>::new());
        for e in ansi_fmt::AnsiDoc::events(&input) {
            w.write_event(e.into_owned());
        }
        let streamed_bytes = w.finish();
        let streamed = String::from_utf8_lossy(&streamed_bytes).into_owned();

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build(): \
                 {built:?}\n  Writer:  {streamed:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of ansi fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = ansi_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(ansi_fmt::OwnedEvent::Text {
            text: "Hello world".to_string().into(),
            style: ansi_fmt::ast::Style::default(),
        });
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink before finish() for a Text event — \
                 expected genuine incremental writing per writer.rs's direct sink.write_all calls"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("ansi", "streaming_writer", result);
}
