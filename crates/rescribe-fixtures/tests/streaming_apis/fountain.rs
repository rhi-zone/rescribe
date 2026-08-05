//! Streaming-API cross-checks for fountain. Split out of the former monolithic
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
// fountain-fmt: `fountain_fmt::events()` (src/lib.rs) returns
// `events::OwnedEventIter`, which is `parse(input)` then a lazy walk of the
// AST already built — the same events()-is-parse()+AST-walk pattern already
// documented for t2t/pod/haddock/asciidoc above. (Note: events.rs also
// defines a second, *borrowed* `EventIter<'a>` with its own `pub fn
// new(doc: &'a FountainDoc)` — but it is not re-exported from lib.rs and is
// not what `events()` returns, so it is out of scope for this harness; it
// independently appears to double-emit `Event::PageBreak` and never emit a
// `Text` event for any non-Character/Dialogue/Parenthetical block, per a
// direct reading of its `Blocks`-phase match arms, which is worth a
// follow-up look but is not part of the `events()` API this harness checks.)
// The ast_to_events-vs-events() check below is real and passes, but
// validates the AST->event expansion layer, not two independent parsers.
//
// StreamingParser (batch.rs) flushes events per accumulated block as fed
// (blank line, boneyard close, or EOF triggers emit_block()), but
// emit_block() re-parses the block's text via `crate::events::events(&text)`
// and forwards *every* event it yields — including that call's own
// StartDocument/EndDocument pair — straight to the handler with no
// filtering (batch.rs: `for event in crate::events::events(&text) {
// self.handler.handle(event); }`). Since bulk `events()` over the whole
// input emits exactly one StartDocument/EndDocument pair spanning the
// document, but StreamingParser emits one such pair *per accumulated
// block*, this diverges on every fixture with more than one
// blank-line-separated block — the majority of the suite, not an edge case
// the way t2t's header-lookahead bug was. A second, narrower defect shares
// the same root cause: `parse_title_page()` (parse.rs:81) runs
// unconditionally at the start of every `parse()` call with no "is this
// really the first block of the document" guard, so a body block that
// happens to match `key: value` for one of the 9 recognized title-page
// field names (title/credit/author/authors/source/draft date/contact/
// copyright/notes) gets misread as metadata when it is re-parsed in
// isolation, the same class of bug already tracked for t2t's
// try_parse_header().
//
// Writer buffers all fed events into a Vec<OwnedEvent> and only
// reconstructs the AST + calls emit() inside finish() (writer.rs's own
// module doc: "This implementation buffers all events, reconstructs the
// AST, then emits") — the same fake-streaming-writer pattern as
// t2t/pod/haddock/textile/commonmark/org/texinfo.
// ---------------------------------------------------------------------------

fn fountain_ast_to_events(doc: &fountain_fmt::FountainDoc) -> Vec<fountain_fmt::OwnedEvent> {
    use fountain_fmt::Block;
    use fountain_fmt::events::Event;

    let mut out = vec![Event::StartDocument];
    for (key, value) in &doc.metadata {
        out.push(Event::Metadata {
            key: key.clone().into(),
            value: value.clone().into(),
        });
    }

    let blocks = &doc.blocks;
    let mut i = 0;
    while i < blocks.len() {
        if let Block::Character { name, dual, .. } = &blocks[i] {
            out.push(Event::StartDialogueBlock);
            out.push(Event::StartCharacter { dual: *dual });
            out.push(Event::Text(name.clone().into()));
            out.push(Event::EndCharacter);
            i += 1;
            while i < blocks.len()
                && matches!(
                    blocks[i],
                    Block::Dialogue { .. } | Block::Parenthetical { .. }
                )
            {
                fountain_leaf_block_events(&blocks[i], &mut out);
                i += 1;
            }
            out.push(Event::EndDialogueBlock);
        } else {
            fountain_leaf_block_events(&blocks[i], &mut out);
            i += 1;
        }
    }

    out.push(Event::EndDocument);
    out
}

fn fountain_leaf_block_events(b: &fountain_fmt::Block, out: &mut Vec<fountain_fmt::OwnedEvent>) {
    use fountain_fmt::Block;
    use fountain_fmt::events::Event;
    match b {
        Block::SceneHeading { text, .. } => {
            out.push(Event::StartSceneHeading);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndSceneHeading);
        }
        Block::Action { text, .. } => {
            out.push(Event::StartAction);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndAction);
        }
        Block::Character { name, dual, .. } => {
            // Only reached for a Character with no following dialogue at
            // document end, since the caller special-cases the common path.
            out.push(Event::StartCharacter { dual: *dual });
            out.push(Event::Text(name.clone().into()));
            out.push(Event::EndCharacter);
        }
        Block::Dialogue { text, .. } => {
            out.push(Event::StartDialogue);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndDialogue);
        }
        Block::Parenthetical { text, .. } => {
            out.push(Event::StartParenthetical);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndParenthetical);
        }
        Block::Transition { text, .. } => {
            out.push(Event::StartTransition);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndTransition);
        }
        Block::Centered { text, .. } => {
            out.push(Event::StartCentered);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndCentered);
        }
        Block::Lyric { text, .. } => {
            out.push(Event::StartLyric);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndLyric);
        }
        Block::Note { text, .. } => {
            out.push(Event::StartNote);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndNote);
        }
        Block::Synopsis { text, .. } => {
            out.push(Event::StartSynopsis);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndSynopsis);
        }
        Block::Section { level, text, .. } => {
            out.push(Event::StartSection { level: *level });
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndSection);
        }
        Block::PageBreak { .. } => out.push(Event::PageBreak),
        Block::Boneyard { text, .. } => {
            out.push(Event::StartBoneyard);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndBoneyard);
        }
    }
}

#[test]
fn fountain_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("fountain");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/fountain dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = fountain_fmt::parse(&input);
        let expected = fountain_ast_to_events(&doc);
        let actual: Vec<_> = fountain_fmt::events(&input).collect();
        assert_eq!(
            expected,
            actual,
            "events() diverged from the AST projection for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of fountain fixtures, got {checked}"
    );
}

/// `StreamingParser` re-parses each accumulated block via
/// `crate::events::events()` and forwards that call's events verbatim,
/// including its own StartDocument/EndDocument pair — so bulk `events()`'s
/// single document-boundary pair vs. one pair per block is expected to (and
/// does) diverge on any fixture with more than one block. Checked via
/// adversarial-chunking equivalence against `events()` over the whole input.
#[test]
fn fountain_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("fountain");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/fountain dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<fountain_fmt::OwnedEvent> = fountain_fmt::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                fountain_fmt::batch::StreamingParser::new(|e: fountain_fmt::OwnedEvent| {
                    streamed.push(e);
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
        checked > 10,
        "expected to check a substantial number of fountain fixtures, got {checked}"
    );
    assert_or_known_failure("fountain", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<OwnedEvent>` and only
/// reconstructs the AST + calls `emit()` inside `finish()` (see
/// `crates/formats/fountain-fmt/src/writer.rs`'s own module doc). Checked
/// via byte-identical comparison against the builder path, plus an
/// incrementality probe.
#[test]
fn fountain_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("fountain");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/fountain dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = fountain_fmt::parse(&input);
        let built = fountain_fmt::build(&doc);

        let mut w = fountain_fmt::Writer::new(Vec::<u8>::new());
        for e in fountain_fmt::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build():  \
                 {built:?}\n  streamed: {streamed:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of fountain fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use fountain_fmt::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = fountain_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartDocument);
        w.write_event(OwnedEvent::StartAction);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndAction);
        w.write_event(OwnedEvent::EndDocument);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartDocument/StartAction/\
                 Text/EndAction/EndDocument sequence and before finish() — \
                 fountain_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                 reconstructs the AST + calls emit() inside finish() \
                 (crates/formats/fountain-fmt/src/writer.rs, self-admitted in its own module \
                 doc), so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("fountain", "streaming_writer", result);
}
