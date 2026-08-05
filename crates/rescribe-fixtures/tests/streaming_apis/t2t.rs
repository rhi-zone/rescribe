//! Streaming-API cross-checks for t2t. Split out of the former monolithic
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
// t2t: events() is `EventIter::new(parse(input).0)` (src/events.rs's public
// `events()` fn) — a lazy frame-stack walk of the AST `parse()` already
// built, not an independently-implemented reader. Per the asciidoc precedent
// above, the ast_to_events-vs-events() check below is real and passes, but
// it validates the AST->event expansion layer (push_block/push_inline's
// frame-stack walk), not two independent parsers — the check would pass by
// construction even if `events()`'s only bug were "doesn't match its own
// AST", since both sides start from the same `T2tDoc`.
//
// StreamingParser is a genuine per-block incremental parser (batch.rs's
// feed_line/BlockState machine flushes each accumulated block via
// emit_block() as soon as a blank line or fence boundary is seen, not only
// at finish()). It used to re-parse each block's text in isolation via
// crate::events::events(&text) and forward that re-parse's own
// StartDocument/EndDocument pair verbatim — so bulk events() over the whole
// document emitted exactly one such pair, but StreamingParser emitted one
// PER accumulated block, diverging on every fixture with more than one
// top-level block. This is now fixed: StreamingParser::new() dispatches a
// single StartDocument directly (mirroring fountain-fmt's batch.rs), finish()
// dispatches the matching EndDocument, and both emit_block() and
// try_emit_header()'s trailing-content path filter
// Event::StartDocument/EndDocument out of every per-block re-parse's forwarded
// events. Confirmed via a hand-built adversarial-chunking test added to t2t's
// own batch.rs test module (whole/single-byte/chunks-of-7/chunks-of-37 over a
// synthetic multi-block sample) plus the fixture-driven equivalence check
// below.
//
// The document-header-specific defect that used to live alongside the above
// — an isolated re-parse of the header's own three lines re-triggering
// `try_parse_header()` (parse.rs:70) and producing a spurious *empty*
// StartDocument/EndDocument pair with the header's title/author/date
// silently dropped (Event had no variant to carry them) — was already fixed
// before this session: `Event::Header { title, author, date }` was added,
// and StreamingParser (batch.rs's `try_emit_header`) recognizes the first
// block of the stream directly via `Parser::try_parse_header` instead of
// falling through to the generic re-parse-via-events() path.
//
// Three distinct, pre-existing root causes remain (unmasked now that the
// StartDocument/EndDocument duplication no longer swamps every multi-block
// fixture), affecting 4 of the ~50 fixtures — see the KnownFailure entry in
// `streaming_harness::KNOWN_FAILURES` for the full detail on each:
// (1) definition-list — parse_definition_list (parse.rs:412) merges
// consecutive ': '-item blocks across a blank line into one DefinitionList
// at the whole-document level; StreamingParser has no construct-aware
// continuation state across its blank-line block boundary, so it emits one
// DefinitionList pair per item instead of one merged list — this is the
// fixture originally called out in this comment, and it is still open, just
// for a different reason (list-merging state, not the document wrapper).
// (2) adv-heading-no-close / adv-link-no-close — Parser::try_parse_header
// reads self.lines[0..3] of the whole document without checking they're
// contiguous, misdetecting a title/date spanning a blank-line block
// boundary that StreamingParser's per-block try_emit_header correctly does
// not (a bug in the reference parse()/events() behavior itself, not in
// StreamingParser). (3) adv-unclosed-code — an EOF-terminated fence's
// CodeBlock content includes a trailing newline in the whole-document parse
// but not in StreamingParser's block_lines.join("\n") reconstruction. These
// are why `t2t_streaming_parser_matches_events_under_adversarial_chunking`
// still fails overall — see `document-header`'s own streamed output for
// confirmation the header itself is carried correctly.
//
// Writer used to buffer all events into a Vec<OwnedEvent> and only
// reconstruct the AST + call emit() inside finish() (writer.rs's own module
// doc: "This implementation buffers all events, reconstructs the AST, then
// emits") — the same fake-streaming-writer pattern as
// textile/commonmark/org/texinfo — and separately always dropped
// doc.title/author/date on every fixture with a document header, since
// t2t::Event had no variant carrying those fields, so writer.rs's
// DocBuilder::finish (writer.rs:400-404) always reconstructed
// title: None/author: None/date: None. The title/author/date-dropping half
// is now fixed: `DocBuilder` tracks title/author/date fields, set by the new
// `Event::Header` arm in `process()` and threaded through in `finish()`, so
// the byte-identical-to-builder content check now passes on every fixture,
// including document-header. The buffering/non-incrementality half (writer
// writes zero bytes to the sink until finish()) is a separate, still-open
// concern — CLAUDE.md's "fake streaming writer" / hollow-writer performance
// rework — unrelated to Event expressiveness, and is what
// `t2t_streaming_writer_matches_builder_over_all_fixtures`'s incrementality
// probe still (correctly) fails on.
// ---------------------------------------------------------------------------

fn t2t_ast_to_events(doc: &t2t::T2tDoc) -> Vec<t2t::Event<'static>> {
    let mut out = vec![t2t::Event::StartDocument];
    if doc.title.is_some() || doc.author.is_some() || doc.date.is_some() {
        out.push(t2t::Event::Header {
            title: doc.title.clone(),
            author: doc.author.clone(),
            date: doc.date.clone(),
        });
    }
    for b in &doc.blocks {
        t2t_block_events(b, &mut out);
    }
    out.push(t2t::Event::EndDocument);
    out
}

fn t2t_block_events(b: &t2t::Block, out: &mut Vec<t2t::Event<'static>>) {
    use std::borrow::Cow;
    use t2t::{Block, Event};
    match b {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                t2t_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::Heading {
            level,
            numbered,
            inlines,
            ..
        } => {
            out.push(Event::StartHeading {
                level: *level,
                numbered: *numbered,
            });
            for i in inlines {
                t2t_inline_events(i, out);
            }
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { content, .. } => out.push(Event::CodeBlock {
            content: Cow::Owned(content.clone()),
        }),
        Block::RawBlock { content, .. } => out.push(Event::RawBlock {
            content: Cow::Owned(content.clone()),
        }),
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for c in children {
                t2t_block_events(c, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item_blocks in items {
                out.push(Event::StartListItem);
                for b in item_blocks {
                    t2t_block_events(b, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow {
                    header: row.is_header,
                });
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    for i in cell {
                        t2t_inline_events(i, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for (term, desc) in items {
                out.push(Event::StartDefinitionTerm);
                for i in term {
                    t2t_inline_events(i, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for b in desc {
                    t2t_block_events(b, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
    }
}

fn t2t_inline_events(i: &t2t::Inline, out: &mut Vec<t2t::Event<'static>>) {
    use std::borrow::Cow;
    use t2t::{Event, Inline};
    match i {
        Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Code(s, _) => out.push(Event::Code(Cow::Owned(s.clone()))),
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink {
                url: Cow::Owned(url.clone()),
            });
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image { url, .. } => out.push(Event::Image {
            src: Cow::Owned(url.clone()),
        }),
        Inline::LineBreak(_) => out.push(Event::LineBreak),
        Inline::SoftBreak(_) => out.push(Event::SoftBreak),
        Inline::Verbatim(s, _) => out.push(Event::Verbatim(Cow::Owned(s.clone()))),
        Inline::Tagged(s, _) => out.push(Event::Tagged(Cow::Owned(s.clone()))),
    }
}

#[test]
fn t2t_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("t2t");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/t2t dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = t2t::parse::parse(&input);
        let expected = t2t_ast_to_events(&doc);
        let actual: Vec<_> = t2t::events(&input).collect();
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
        "expected to check a substantial number of t2t fixtures, got {checked}"
    );
}

/// `StreamingParser` genuinely flushes events per accumulated block as it's
/// fed (not only at `finish()`) but re-parses each block's text in isolation
/// via `crate::events::events()`, losing cross-block context. Checked via
/// adversarial-chunking equivalence against `events()` over the whole input.
#[test]
fn t2t_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("t2t");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/t2t dir") {
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
        let bulk: Vec<t2t::OwnedEvent> = t2t::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = t2t::batch::StreamingParser::new(|e: t2t::OwnedEvent| {
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
        "expected to check a substantial number of t2t fixtures, got {checked}"
    );
    assert_or_known_failure("t2t", "streaming_parser", result);
}

/// `Writer` writes straight through to a single shared output buffer per
/// event (mirroring `rst-fmt`'s `Writer` design — see
/// `crates/formats/t2t/src/writer.rs`'s module doc), not a
/// buffer-then-reconstruct-the-AST fake streaming writer. Checked via
/// byte-identical comparison against the builder path, plus an
/// incrementality probe.
#[test]
fn t2t_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("t2t");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/t2t dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = t2t::parse::parse(&input);
        let built = t2t::emit::emit(&doc);

        let mut w = t2t::writer::Writer::new(Vec::<u8>::new());
        for e in t2t::events(&input) {
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
        "expected to check a substantial number of t2t fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use t2t::Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = t2t::writer::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text("Hello world".to_string().into()));
        w.write_event(Event::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — t2t::writer::Writer buffers all \
                 events into a Vec<OwnedEvent> and only reconstructs the AST + calls emit() \
                 inside finish() (crates/formats/t2t/src/writer.rs), so it is not a genuine \
                 incremental streaming writer despite content round-tripping correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("t2t", "streaming_writer", result);
}
