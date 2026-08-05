//! Streaming-API cross-checks for xwiki. Split out of the former monolithic
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
// xwiki: events() is a genuine lazy pull-iterator over &XwikiDoc (unlike
// zimwiki/markua/muse-fmt below, which eagerly materialize a Vec/VecDeque of
// events before iteration begins). StreamingParser and Writer are both
// confirmed-fake buffer-then-finish wrappers.
// ---------------------------------------------------------------------------
//
// xwiki::events::events() takes `&XwikiDoc`, not `&str` — EventIter::next()
// (crates/formats/xwiki/src/events.rs:168-385) is a true frame-stack walker
// pulled on demand, so this check validates that walk directly against an
// independently hand-written projection.
mod xwiki_events_check {
    use super::{find_input, fixtures_root};
    use std::borrow::Cow;
    use xwiki::{Block, Event, Inline, XwikiDoc};

    /// Reconstruct the exact [`xwiki::Event`] sequence `events()` must produce
    /// for `doc`.
    ///
    /// One non-obvious mapping: `Inline::Link { url, label, .. }` stores `label`
    /// as a plain `String` (not nested inlines), but the event vocabulary only
    /// has `StartLink`/`EndLink` with no leaf "link text" event — confirmed by
    /// reading `EventIter::next()`'s `Inline::Link` arm (events.rs:361-368),
    /// which emits `StartLink`, queues a single `Text(label)` as `self.pending`,
    /// and closes with `EndLink`. The projection below mirrors that exactly.
    fn xwiki_ast_to_events(doc: &XwikiDoc) -> Vec<Event<'_>> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            xwiki_block_events(b, &mut out);
        }
        out
    }

    fn xwiki_block_events<'a>(b: &'a Block, out: &mut Vec<Event<'a>>) {
        match b {
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                xwiki_inline_events(inlines, out);
                out.push(Event::EndHeading);
            }
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                xwiki_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::CodeBlock {
                content, language, ..
            } => out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Borrowed(content),
            }),
            Block::Table { rows, .. } => {
                out.push(Event::StartTable);
                for row in rows {
                    out.push(Event::StartTableRow);
                    for cell in &row.cells {
                        out.push(Event::StartTableCell {
                            is_header: cell.is_header,
                        });
                        xwiki_inline_events(&cell.inlines, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item in items {
                    out.push(Event::StartListItem);
                    for c in item {
                        xwiki_block_events(c, out);
                    }
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
            Block::Blockquote { children, .. } => {
                out.push(Event::StartBlockquote);
                for c in children {
                    xwiki_block_events(c, out);
                }
                out.push(Event::EndBlockquote);
            }
            Block::MacroBlock {
                name,
                params,
                content,
                ..
            } => out.push(Event::MacroBlock {
                name: name.clone(),
                params: params.clone(),
                content: content.clone(),
            }),
            Block::MacroInline { name, params, .. } => out.push(Event::MacroInline {
                name: name.clone(),
                params: params.clone(),
            }),
        }
    }

    fn xwiki_inline_events<'a>(inlines: &'a [Inline], out: &mut Vec<Event<'a>>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(Event::Text(Cow::Borrowed(s))),
                Inline::Bold(c, _) => {
                    out.push(Event::StartBold);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(Event::StartItalic);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndItalic);
                }
                Inline::Underline(c, _) => {
                    out.push(Event::StartUnderline);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndUnderline);
                }
                Inline::Strikeout(c, _) => {
                    out.push(Event::StartStrikeout);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndStrikeout);
                }
                Inline::Superscript(c, _) => {
                    out.push(Event::StartSuperscript);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(Event::StartSubscript);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Borrowed(s))),
                Inline::Link { url, label, .. } => {
                    out.push(Event::StartLink { url: url.clone() });
                    out.push(Event::Text(Cow::Borrowed(label)));
                    out.push(Event::EndLink);
                }
                Inline::Image {
                    url, alt, params, ..
                } => out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                    params: params.clone(),
                }),
                Inline::LineBreak { .. } => out.push(Event::LineBreak),
                Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
            }
        }
    }

    #[test]
    fn xwiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("xwiki");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/xwiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = xwiki::parse::parse(&input);
            let expected = xwiki_ast_to_events(&doc);
            let actual: Vec<_> = xwiki::events::events(&doc).collect();
            checked += 1;
            if expected != actual {
                let at = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual.len()));
                failures.push(format!(
                    "{}: first divergence at event #{at} (expected len {}, actual len {})",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                ));
            }
        }
        assert!(
            checked > 20,
            "expected to check a substantial number of xwiki fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} xwiki fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `xwiki::batch::StreamingParser` was rewritten 2026-07-31 (this session) to
/// accumulate one top-level block at a time — a paragraph, heading, list,
/// table, code block, or macro/quote block, the same boundaries
/// `crate::parse::parse`'s dispatch loop uses — flushing each block to the
/// handler (reparsed in isolation via `parse::parse` + walked with
/// `events::events`) as soon as its boundary is confirmed, instead of the
/// old `buf.extend_from_slice`-only `feed()` that only parsed inside
/// `finish()`. The adversarial-chunking equivalence check below is a
/// genuine correctness check now (not the "passes trivially" case the old
/// buffer-then-finish implementation made it): each fixture's `parse()` ->
/// `events()` sequence must match `StreamingParser` fed under whole/
/// single-byte/N-byte/mid-UTF-8 chunkings, which now exercises real
/// block-by-block state transitions instead of one final reparse.
#[test]
fn xwiki_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("xwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/xwiki dir") {
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
        let (doc, _diags) = xwiki::parse::parse(input_str);
        let bulk: Vec<xwiki::OwnedEvent> = xwiki::events::events(&doc)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                xwiki::batch::StreamingParser::new(|e: xwiki::OwnedEvent| streamed.push(e));
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
        checked > 20,
        "expected to check a substantial number of xwiki fixtures, got {checked}"
    );

    // Direct incrementality probe, using a synthetic multi-block document
    // rather than iterating fixture-by-fixture: several real fixtures (e.g.
    // `blockquote`, a single `{{quote}}...{{/quote}}` spanning the whole
    // ~39-byte file) are just one indivisible top-level block, so feeding
    // exactly their first half can legitimately deliver zero events — that
    // block's closing tag hasn't been seen yet, which is inherent to
    // line-oriented block-boundary parsing (the same constraint RST's
    // directive bodies and XWiki's own macro/quote blocks have), not a
    // buffer-then-finish defect. A document with two clearly separated
    // top-level blocks avoids that false positive while still proving
    // `feed()` delivers events before `finish()` for a normal document.
    if result.is_ok() {
        let input = b"= Title =\n\nFirst paragraph.\n\n== Sub ==\n\nSecond paragraph.\n";
        let mid = input.len() / 2;
        let mut delivered: Vec<xwiki::OwnedEvent> = Vec::new();
        let mut parser = xwiki::batch::StreamingParser::new(|e| delivered.push(e));
        parser.feed(&input[..mid]);
        result = assert_streaming_parser_is_incremental("xwiki", !delivered.is_empty());
        // `parser` intentionally dropped without calling finish(): this probe
        // only needs to observe pre-finish handler state.
    }

    assert_or_known_failure("xwiki", "streaming_parser", result);
}

/// `xwiki::writer::Writer::write_event()` only pushes to a `Vec`
/// (crates/formats/xwiki/src/writer.rs:39-41); `finish()` reconstructs the
/// AST via `collect_doc_from_events` and calls `emit::build` once
/// (writer.rs:44-49). Content-wise this round-trips correctly (checked
/// below), but an incrementality probe shows zero bytes reach the sink
/// before `finish()`.
#[test]
fn xwiki_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("xwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/xwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = xwiki::parse::parse(&input);
        let built = String::from_utf8(doc.emit()).expect("xwiki emit output is UTF-8");

        let mut w = xwiki::Writer::new(Vec::<u8>::new());
        for e in xwiki::events::events(&doc) {
            w.write_event(e.into_owned());
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
        checked > 20,
        "expected to check a substantial number of xwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = xwiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(xwiki::OwnedEvent::StartHeading { level: 1 });
        w.write_event(xwiki::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(xwiki::OwnedEvent::EndHeading);
        w.write_event(xwiki::OwnedEvent::StartParagraph);
        w.write_event(xwiki::OwnedEvent::Text("World".to_string().into()));
        w.write_event(xwiki::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — xwiki::writer::Writer buffers \
                 all events into a Vec<OwnedEvent> and only reconstructs the AST + calls \
                 emit::build() inside finish() (crates/formats/xwiki/src/writer.rs:39-49), so it \
                 is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("xwiki", "streaming_writer", result);
}
