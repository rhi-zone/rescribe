//! Streaming-API cross-checks for creole. Split out of the former monolithic
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
// creole: events() vs. an AST projection, StreamingParser adversarial
// chunking, streaming writer vs. builder
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`creole::Event`] sequence `events()` must produce
/// for `doc`, directly from the AST `parse()` returned. Mirrors
/// `creole::events::{collect_block_events, collect_inline_events}`
/// structurally (unavoidable: `creole::events::EventIter::new` is literally
/// `crate::parse::parse(input)` followed by `collect_events(&doc)`, a
/// depth-first walk over the AST — see `crates/formats/creole/src/events.rs`
/// lines 123-127 — the same non-independent shape as bbcode-fmt's and
/// html-fmt's `events()`. Per the bbcode/asciidoc precedent this is still
/// wired as `Wired` rather than `NotApplicable`, since nothing in the
/// Creole format itself forces the coupling (unlike html5ever's tree
/// construction) — it is an implementation choice, not a structural
/// necessity. This check therefore pins the AST<->Event correspondence
/// (and would catch a `collect_events` that dropped or reordered a field)
/// rather than proving two independent implementations agree; the `Event`
/// enum's own `PartialEq` gives exact equality, not merely a lossy shape
/// comparison.
fn creole_ast_to_events(doc: &creole::CreoleDoc) -> Vec<creole::OwnedEvent> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        creole_block_events(block, &mut out);
    }
    out
}

fn creole_block_events(block: &creole::Block, out: &mut Vec<creole::OwnedEvent>) {
    use creole::Block;
    use creole::Event;
    use std::borrow::Cow;
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            creole_inline_events(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            creole_inline_events(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { content, .. } => {
            out.push(Event::CodeBlock {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                creole_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for child in item {
                    creole_block_events(child, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow);
                for cell in &row.cells {
                    out.push(Event::StartTableCell {
                        is_header: cell.is_header,
                    });
                    creole_inline_events(&cell.inlines, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                creole_inline_events(&item.term, out);
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                creole_inline_events(&item.desc, out);
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::HorizontalRule(_) => {
            out.push(Event::HorizontalRule);
        }
    }
}

fn creole_inline_events(inlines: &[creole::Inline], out: &mut Vec<creole::OwnedEvent>) {
    use creole::Event;
    use creole::Inline;
    use std::borrow::Cow;
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => {
                out.push(Event::Text(Cow::Owned(s.clone())));
            }
            Inline::LineBreak(_) => {
                out.push(Event::LineBreak);
            }
            Inline::Code(s, _) => {
                out.push(Event::InlineCode(Cow::Owned(s.clone())));
            }
            Inline::Bold(children, _) => {
                out.push(Event::StartBold);
                creole_inline_events(children, out);
                out.push(Event::EndBold);
            }
            Inline::Italic(children, _) => {
                out.push(Event::StartItalic);
                creole_inline_events(children, out);
                out.push(Event::EndItalic);
            }
            Inline::Link { url, children, .. } => {
                out.push(Event::StartLink { url: url.clone() });
                creole_inline_events(children, out);
                out.push(Event::EndLink);
            }
            Inline::Image { url, alt, .. } => {
                out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                });
            }
        }
    }
}

#[test]
fn creole_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("creole");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/creole dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = creole::parse(&input);
        let expected = creole_ast_to_events(&doc);
        let actual: Vec<_> = creole::events(&input).collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  \
                 ast-derived: {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of creole fixtures, got {checked}"
    );
    assert_or_known_failure("creole", "events", result);
}

/// `creole::batch::StreamingParser` accumulates lines into blocks and calls
/// `emit_block()` (which re-parses just the accumulated block text via
/// `crate::events::events()`) as soon as a block boundary is recognized —
/// see `crates/formats/creole/src/batch.rs`'s `feed_line`/`emit_block` — so
/// unlike texinfo/fb2/textile's `StreamingParser` it is not a hollow
/// buffer-then-`finish()` stub. Both halves of this check pass for real:
/// the adversarial-chunking equivalence check against `events()` holds over
/// every creole fixture, and a hand-built probe (see the incrementality
/// check inline below) confirms `feed()` alone delivers events before
/// `finish()` is ever called. One inspected-but-unobserved edge case:
/// `feed_line`'s in-nowiki close test (batch.rs, `is_end = line.trim() ==
/// "}}}"`) requires the closing marker to be the *entire* trimmed line,
/// while `parse.rs`'s `parse_nowiki_block` finds `"}}}"` anywhere in the
/// line (dropping any trailing text after it) — so a nowiki block closed by
/// a line like `"tail}}}"` never trips the streaming splitter's boundary
/// and everything from that opener onward is swept into one oversized
/// block, delivered only at `finish()`. Verified by hand
/// (`{{{\ncode\nsome}}}\nmore\n`) that this degrades *incrementality*, not
/// *correctness*: the oversized block is still handed whole to
/// `crate::events::events()`, which re-derives the identical block split a
/// bulk `parse()` over that span would produce, so the final event sequence
/// still matches `events()` exactly — not a tracked `KnownFailure`, since
/// nothing observable diverges.
#[test]
fn creole_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("creole");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/creole dir") {
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
        let bulk: Vec<creole::OwnedEvent> = creole::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                creole::batch::StreamingParser::new(|e: creole::OwnedEvent| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():        {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of creole fixtures, got {checked}"
    );

    // Incrementality probe: confirm the completed first block's events reach
    // the handler before finish() is ever called, for a multi-block input
    // (a completed heading, a blank line, then unterminated trailing text).
    if result.is_ok() {
        let probe_input = b"= Hello\n\nUnterminated tail with no blank line after it";
        let mut delivered: Vec<creole::OwnedEvent> = Vec::new();
        let mut parser = creole::batch::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        if delivered.is_empty() {
            result = Err(
                "StreamingParser delivered zero events to the handler after feed() with a \
                 complete `= Hello` heading followed by a blank line and unterminated trailing \
                 text, and before finish() was called — expected the completed first block to \
                 have been flushed as soon as its terminating blank line arrived"
                    .to_string(),
            );
        }
        // `parser` intentionally dropped without `finish()`: this probe only
        // needs to observe pre-finish handler state.
    }
    assert_or_known_failure("creole", "streaming_parser", result);
}

/// `creole::writer::Writer` buffers all fed events into an internal
/// `Vec<OwnedEvent>` (`write_event()`, writer.rs:38-40, only pushes) and
/// only reconstructs the AST (`events_to_doc`) + calls `crate::emit::build`
/// inside `finish()` (writer.rs:43-48) — a hollow buffer-then-finish
/// implementation, not a genuine incremental streaming writer. Checked the
/// same way as bbcode/textile/commonmark's writers: byte-identical-to-builder
/// content correctness (expected to pass, since `finish()` ultimately drives
/// the same `build()` the builder path uses) plus an incrementality probe.
#[test]
fn creole_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("creole");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/creole dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = creole::parse(&input);
        let built = creole::build(&doc);

        let mut w = creole::writer::Writer::new(Vec::<u8>::new());
        for e in creole::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");
        checked += 1;

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build():  \
                 {built:?}\n  streamed: {streamed:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of creole fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = creole::writer::Writer::new(ObservableSink(observed.clone()));
        w.write_event(creole::OwnedEvent::StartParagraph);
        w.write_event(creole::OwnedEvent::StartBold);
        w.write_event(creole::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(creole::OwnedEvent::EndBold);
        w.write_event(creole::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/StartBold/Text/EndBold/EndParagraph sequence and before \
                 finish() — creole::writer::Writer buffers all events into a Vec<OwnedEvent> \
                 and only reconstructs the AST + calls build() inside finish() \
                 (crates/formats/creole/src/writer.rs), so it is not a genuine incremental \
                 streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("creole", "streaming_writer", result);
}
