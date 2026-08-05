//! Streaming-API cross-checks for man. Split out of the former monolithic
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
// man-fmt: `man_fmt::man_events()` (`events::events`, src/events.rs) is
// `EventIter::new(&parse(input).0).map(into_owned).collect()` — a lazy
// frame-stack walk of the AST `parse()` already built, then eagerly
// collected, not an independently implemented reader (same pattern as
// t2t/pod/haddock/asciidoc/fountain above).
//
// Fixed 2026-08-03: `ManEvent` now carries a `Metadata` variant
// (`ManDoc::title`/`section`/`date`/`source`/`manual`), emitted once by
// `EventIter` right after `StartDocument` (mirroring t2t-fmt's
// `Event::Header`) and consumed by `collect_doc_from_events` — the
// ast_to_events-vs-events() check below now asserts on it via
// `man_ast_to_events` inserting the same `Metadata` event.
//
// StreamingParser (batch.rs) is a genuine incremental line-buffered block
// splitter — `feed_line` accumulates lines until a blank line, a `.nf`/`.EX`
// preformatted-block boundary, or a new macro line ends the current block,
// then `emit_block()` re-parses just that block's text via
// `crate::events::events(&text)`. `events()` always wraps its output in its
// own `StartDocument`/`EndDocument` pair (see the `ManEvent` walk in
// events.rs); this used to leak through into `StreamingParser`, which
// forwarded every event each re-parse yielded with no filtering, producing
// one such pair per accumulated block instead of one for the whole document
// — the same re-parse-each-block-in-isolation root cause already tracked for
// t2t-fmt/fountain-fmt. Fixed 2026-08-03 the same way: `StreamingParser` now
// dispatches its own single `StartDocument` in `new()` and `EndDocument` in
// `finish()`, and `emit_block()` filters `StartDocument`/`EndDocument` out
// of each re-parsed block's forwarded events. man-fmt's `events()` has no
// title-page-style "only the first block can mean X" wrinkle (unlike
// fountain), so no `events_body()`-style second entry point was needed.
//
// Writer fixed 2026-08-03: rewritten from a buffer-all-events-then-
// reconstruct-the-AST-in-finish() writer (the same fake-streaming pattern
// as t2t/pod/haddock/fountain/commonmark) to a genuine incremental writer —
// see `crates/formats/man-fmt/src/writer.rs`'s module doc for the full
// construct-by-construct classification. It also picks up the
// `ManEvent::Metadata` fix above: a `.TH` line's title/section/date/source
// is now preserved end to end through `events()` — verified directly on
// fixture th-header's `.TH` line (`build()` and the events()-fed streaming
// Writer both now emit `.TH TEST 1 "2024-01-01" "Version 1.0" ""`).
// ---------------------------------------------------------------------------

fn man_ast_to_events(doc: &man_fmt::ManDoc) -> Vec<man_fmt::OwnedManEvent> {
    let mut out = Vec::new();
    out.push(man_fmt::ManEvent::StartDocument);
    out.push(man_fmt::ManEvent::Metadata {
        title: doc.title.clone().map(Into::into),
        section: doc.section.clone().map(Into::into),
        date: doc.date.clone().map(Into::into),
        source: doc.source.clone().map(Into::into),
        manual: doc.manual.clone().map(Into::into),
    });
    for b in &doc.blocks {
        man_block_events(b, &mut out);
    }
    out.push(man_fmt::ManEvent::EndDocument);
    out
}

fn man_block_events(b: &man_fmt::Block, out: &mut Vec<man_fmt::OwnedManEvent>) {
    use man_fmt::{Block, ManEvent};
    match b {
        Block::Heading { level, inlines, .. } => {
            out.push(ManEvent::StartHeading { level: *level });
            man_inline_events(inlines, out);
            out.push(ManEvent::EndHeading);
        }
        Block::Paragraph { inlines, .. } => {
            out.push(ManEvent::StartParagraph);
            man_inline_events(inlines, out);
            out.push(ManEvent::EndParagraph);
        }
        Block::IndentedParagraph { inlines, .. } => {
            out.push(ManEvent::StartIndentedParagraph);
            man_inline_events(inlines, out);
            out.push(ManEvent::EndIndentedParagraph);
        }
        Block::CodeBlock { content, .. } => out.push(ManEvent::CodeBlock {
            content: content.clone().into(),
        }),
        Block::ExampleBlock { content, .. } => out.push(ManEvent::ExampleBlock {
            content: content.clone().into(),
        }),
        Block::HorizontalRule { .. } => out.push(ManEvent::HorizontalRule),
        Block::Comment { text, .. } => out.push(ManEvent::Comment {
            text: text.clone().into(),
        }),
        Block::List { ordered, items, .. } => {
            out.push(ManEvent::StartList { ordered: *ordered });
            for item in items {
                out.push(ManEvent::StartListItem);
                for c in item {
                    man_block_events(c, out);
                }
                out.push(ManEvent::EndListItem);
            }
            out.push(ManEvent::EndList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(ManEvent::StartDefinitionList);
            for (term, desc) in items {
                out.push(ManEvent::StartDefinitionTerm);
                man_inline_events(term, out);
                out.push(ManEvent::EndDefinitionTerm);
                out.push(ManEvent::StartDefinitionDesc);
                for c in desc {
                    man_block_events(c, out);
                }
                out.push(ManEvent::EndDefinitionDesc);
            }
            out.push(ManEvent::EndDefinitionList);
        }
    }
}

fn man_inline_events(inlines: &[man_fmt::Inline], out: &mut Vec<man_fmt::OwnedManEvent>) {
    use man_fmt::{Inline, ManEvent};
    for i in inlines {
        match i {
            Inline::Text(s, _) => out.push(ManEvent::Text(s.clone().into())),
            Inline::Code(s, _) => out.push(ManEvent::Code(s.clone().into())),
            Inline::Bold(children, _) => {
                out.push(ManEvent::StartBold);
                man_inline_events(children, out);
                out.push(ManEvent::EndBold);
            }
            Inline::Italic(children, _) => {
                out.push(ManEvent::StartItalic);
                man_inline_events(children, out);
                out.push(ManEvent::EndItalic);
            }
            Inline::Superscript(children, _) => {
                out.push(ManEvent::StartSuperscript);
                man_inline_events(children, out);
                out.push(ManEvent::EndSuperscript);
            }
            Inline::Subscript(children, _) => {
                out.push(ManEvent::StartSubscript);
                man_inline_events(children, out);
                out.push(ManEvent::EndSubscript);
            }
            Inline::Link { url, children, .. } => {
                out.push(ManEvent::StartLink {
                    url: url.clone().into(),
                });
                man_inline_events(children, out);
                out.push(ManEvent::EndLink);
            }
        }
    }
}

#[test]
fn man_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("man");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/man dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = man_fmt::parse(&input);
        let expected = man_ast_to_events(&doc);
        let actual: Vec<_> = man_fmt::man_events(&input)
            .map(|e| e.into_owned())
            .collect();
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
        "expected to check a substantial number of man fixtures, got {checked}"
    );
}

/// `StreamingParser` re-parses each accumulated block in isolation via
/// `crate::events::events()`, which always wraps its output in its own
/// `StartDocument`/`EndDocument` pair. Fixed 2026-08-03: `StreamingParser`
/// now owns exactly one `StartDocument`/`EndDocument` pair itself
/// (`StartDocument` dispatched in `new()`, `EndDocument` in `finish()`) and
/// filters the re-parsed block's own pair out of `emit_block()`'s forwarded
/// events — see `man_fmt::batch`'s module doc and `StreamingParser::new`'s
/// doc comment. This test now exercises the fix across every man fixture
/// under adversarial chunking.
#[test]
fn man_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("man");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/man dir") {
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
        let bulk: Vec<man_fmt::OwnedManEvent> = man_fmt::man_events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = man_fmt::batch::StreamingParser::new(|e: man_fmt::OwnedManEvent| {
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
        "expected to check a substantial number of man fixtures, got {checked}"
    );

    if result.is_ok() {
        let probe_input = b".SH NAME\ntest\n\n.PP\nUnterminated paragraph text";
        let mut delivered: Vec<man_fmt::OwnedManEvent> = Vec::new();
        let mut parser = man_fmt::batch::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("man", !delivered.is_empty());
    }
    assert_or_known_failure("man", "streaming_parser", result);
}

/// `Writer` is now a genuine incremental writer (see
/// `crates/formats/man-fmt/src/writer.rs`'s module doc) and `ManEvent`
/// carries `.TH` metadata via `ManEvent::Metadata`, so this fixture sweep is
/// no longer a `KnownFailure`. Checked via byte-identical comparison against
/// the builder path, plus an incrementality probe.
#[test]
fn man_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("man");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/man dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = man_fmt::parse(&input);
        let built = man_fmt::build(&doc);

        let mut w = man_fmt::Writer::new(Vec::<u8>::new());
        for e in man_fmt::man_events(&input) {
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
        "expected to check a substantial number of man fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use man_fmt::ManEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = man_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(ManEvent::StartParagraph);
        w.write_event(ManEvent::Text("Hello world".to_string().into()));
        w.write_event(ManEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — man_fmt::writer::Writer buffers \
                 all events into a Vec<OwnedManEvent> and only reconstructs the AST + calls \
                 emit::build() inside finish() (crates/formats/man-fmt/src/writer.rs, \
                 self-admitted in its own module doc), so it is not a genuine incremental \
                 streaming writer despite content round-tripping correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("man", "streaming_writer", result);
}
