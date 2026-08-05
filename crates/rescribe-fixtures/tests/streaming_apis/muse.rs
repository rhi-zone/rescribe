//! Streaming-API cross-checks for muse. Split out of the former monolithic
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
// muse: events() takes &MuseDoc (like xwiki), but eagerly materializes a
// VecDeque in EventIter::new (events.rs:211-220) rather than pulling lazily —
// still a real, independently hand-checkable walk. StreamingParser and
// Writer were both confirmed-fake buffer-then-finish wrappers as of
// 2026-07-31's harness wiring; both have since been rewritten to genuinely
// incremental implementations (batch.rs's line-buffered block splitter and
// writer.rs's per-event streaming writer, respectively).
// ---------------------------------------------------------------------------
mod muse_events_check {
    use super::{find_input, fixtures_root};
    use muse_fmt::{Block, Inline, MuseDoc, OwnedMuseEvent};
    use rescribe_format_api::Parse;
    use std::borrow::Cow;

    /// Reconstruct the exact [`muse_fmt::OwnedMuseEvent`] sequence `events()`
    /// must produce for `doc`, including the `StartDocument`/`EndDocument`
    /// wrapper pair `EventIter::new` always emits (events.rs:213-219).
    fn muse_ast_to_events(doc: &MuseDoc) -> Vec<OwnedMuseEvent> {
        let mut out = vec![
            OwnedMuseEvent::StartDocument,
            OwnedMuseEvent::Metadata {
                title: doc.title.clone().map(Cow::Owned),
                author: doc.author.clone().map(Cow::Owned),
                date: doc.date.clone().map(Cow::Owned),
                description: doc.description.clone().map(Cow::Owned),
                keywords: doc.keywords.clone().map(Cow::Owned),
            },
        ];
        for b in &doc.blocks {
            muse_block_events(b, &mut out);
        }
        out.push(OwnedMuseEvent::EndDocument);
        out
    }

    fn muse_block_events(b: &Block, out: &mut Vec<OwnedMuseEvent>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(OwnedMuseEvent::StartParagraph);
                muse_inline_events(inlines, out);
                out.push(OwnedMuseEvent::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(OwnedMuseEvent::StartHeading { level: *level });
                muse_inline_events(inlines, out);
                out.push(OwnedMuseEvent::EndHeading);
            }
            Block::CodeBlock { content, .. } => out.push(OwnedMuseEvent::CodeBlock {
                content: Cow::Owned(content.clone()),
            }),
            Block::Blockquote { children, .. } => {
                out.push(OwnedMuseEvent::StartBlockquote);
                for c in children {
                    muse_block_events(c, out);
                }
                out.push(OwnedMuseEvent::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(OwnedMuseEvent::StartList { ordered: *ordered });
                for item in items {
                    out.push(OwnedMuseEvent::StartListItem);
                    for c in item {
                        muse_block_events(c, out);
                    }
                    out.push(OwnedMuseEvent::EndListItem);
                }
                out.push(OwnedMuseEvent::EndList);
            }
            Block::DefinitionList { items, .. } => {
                out.push(OwnedMuseEvent::StartDefinitionList);
                for (term, desc) in items {
                    out.push(OwnedMuseEvent::StartDefinitionTerm);
                    muse_inline_events(term, out);
                    out.push(OwnedMuseEvent::EndDefinitionTerm);
                    out.push(OwnedMuseEvent::StartDefinitionDesc);
                    for b in desc {
                        muse_block_events(b, out);
                    }
                    out.push(OwnedMuseEvent::EndDefinitionDesc);
                }
                out.push(OwnedMuseEvent::EndDefinitionList);
            }
            Block::HorizontalRule { .. } => out.push(OwnedMuseEvent::HorizontalRule),
            Block::Verse { children, .. } => {
                out.push(OwnedMuseEvent::StartVerse);
                for c in children {
                    muse_block_events(c, out);
                }
                out.push(OwnedMuseEvent::EndVerse);
            }
            Block::CenteredBlock { children, .. } => {
                out.push(OwnedMuseEvent::StartCenteredBlock);
                for c in children {
                    muse_block_events(c, out);
                }
                out.push(OwnedMuseEvent::EndCenteredBlock);
            }
            Block::RightBlock { children, .. } => {
                out.push(OwnedMuseEvent::StartRightBlock);
                for c in children {
                    muse_block_events(c, out);
                }
                out.push(OwnedMuseEvent::EndRightBlock);
            }
            Block::LiteralBlock { content, .. } => out.push(OwnedMuseEvent::LiteralBlock {
                content: Cow::Owned(content.clone()),
            }),
            Block::SrcBlock { lang, content, .. } => out.push(OwnedMuseEvent::SrcBlock {
                lang: lang.clone().map(Cow::Owned),
                content: Cow::Owned(content.clone()),
            }),
            Block::Comment { content, .. } => out.push(OwnedMuseEvent::Comment {
                content: Cow::Owned(content.clone()),
            }),
            Block::Table { rows, .. } => {
                out.push(OwnedMuseEvent::StartTable);
                for row in rows {
                    out.push(OwnedMuseEvent::StartTableRow { header: row.header });
                    for cell in &row.cells {
                        out.push(OwnedMuseEvent::StartTableCell);
                        muse_inline_events(cell, out);
                        out.push(OwnedMuseEvent::EndTableCell);
                    }
                    out.push(OwnedMuseEvent::EndTableRow);
                }
                out.push(OwnedMuseEvent::EndTable);
            }
            Block::FootnoteDef { label, content, .. } => {
                out.push(OwnedMuseEvent::StartFootnoteDef {
                    label: Cow::Owned(label.clone()),
                });
                muse_inline_events(content, out);
                out.push(OwnedMuseEvent::EndFootnoteDef);
            }
        }
    }

    fn muse_inline_events(inlines: &[Inline], out: &mut Vec<OwnedMuseEvent>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(OwnedMuseEvent::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(OwnedMuseEvent::StartBold);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(OwnedMuseEvent::StartItalic);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndItalic);
                }
                Inline::Code(s, _) => out.push(OwnedMuseEvent::Code(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(OwnedMuseEvent::StartLink {
                        url: Cow::Owned(url.clone()),
                    });
                    muse_inline_events(children, out);
                    out.push(OwnedMuseEvent::EndLink);
                }
                Inline::Underline(c, _) => {
                    out.push(OwnedMuseEvent::StartUnderline);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndUnderline);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(OwnedMuseEvent::StartStrikethrough);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(OwnedMuseEvent::StartSuperscript);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(OwnedMuseEvent::StartSubscript);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndSubscript);
                }
                Inline::FootnoteRef { label, .. } => out.push(OwnedMuseEvent::FootnoteRef {
                    label: Cow::Owned(label.clone()),
                }),
                Inline::LineBreak(_) => out.push(OwnedMuseEvent::LineBreak),
                Inline::Anchor { name, .. } => out.push(OwnedMuseEvent::Anchor {
                    name: Cow::Owned(name.clone()),
                }),
                Inline::Image { src, alt, .. } => out.push(OwnedMuseEvent::Image {
                    src: Cow::Owned(src.clone()),
                    alt: alt.clone().map(Cow::Owned),
                }),
            }
        }
    }

    #[test]
    fn muse_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("muse");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/muse dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = muse_fmt::MuseDoc::parse(input.as_bytes());
            let expected = muse_ast_to_events(&doc);
            let actual: Vec<OwnedMuseEvent> = muse_fmt::events::events(&doc)
                .map(|e| e.into_owned())
                .collect();
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
            "expected to check a substantial number of muse fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} muse fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `muse_fmt::batch::StreamingParser::feed()` is a genuinely incremental
/// line-buffered block splitter (crates/formats/muse-fmt/src/batch.rs):
/// it accumulates lines only until a top-level block boundary is confirmed,
/// then immediately re-parses just that block's text via the crate's new
/// `parse::parse_blocks` and forwards its events to the handler — before
/// `finish()` is ever called. Both the content-equivalence check and the
/// incrementality probe below are expected to pass.
#[test]
fn muse_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("muse");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/muse dir") {
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
        let (doc, _diags) = muse_fmt::MuseDoc::parse(input_str.as_bytes());
        let bulk: Vec<muse_fmt::OwnedMuseEvent> = muse_fmt::events::events(&doc)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                muse_fmt::batch::StreamingParser::new(|e: muse_fmt::OwnedMuseEvent| {
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
        checked > 20,
        "expected to check a substantial number of muse fixtures, got {checked}"
    );

    // Deliberately NOT probed here: an arbitrary 50%-byte split of each real
    // fixture. That was this check's original design and it is fixture-shape-
    // unaware: fixtures/spec.md's "one focused construct per fixture"
    // convention means most muse fixtures are a single block, so a fixed
    // byte-count split usually lands mid-block regardless of implementation
    // quality (the same probe-methodology gap already fixed for
    // fb2-fmt/texinfo/xwiki/textile-fmt/jats-fmt/pod-fmt). See the hand-built
    // probe below instead, which guarantees an unambiguous complete-prefix
    // boundary: a single-line heading (a complete top-level block on its
    // own) followed by a blank line and a paragraph deliberately left
    // unterminated (no trailing blank line/EOF), so the heading is provably
    // flushable while the paragraph is provably not yet complete.
    if result.is_ok() {
        let probe_input = b"* Heading\n\nUnterminated paragraph text with no closing blank line";
        let mut delivered: Vec<muse_fmt::OwnedMuseEvent> = Vec::new();
        let mut parser = muse_fmt::batch::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("muse", !delivered.is_empty());
        // `parser` intentionally dropped without calling finish(): this probe
        // only needs to observe pre-finish handler state.
    }
    assert_or_known_failure("muse", "streaming_parser", result);
}

/// `muse_fmt::writer::Writer::write_event()` only pushes to a `Vec`
/// (crates/formats/muse-fmt/src/writer.rs:42-44); `finish()` reconstructs the
/// AST via `events_to_doc`/`DocBuilder` and calls `emit::build` once
/// (writer.rs:47-52). Unlike xwiki/zimwiki/markua, this is NOT purely an
/// architectural finding: `DocBuilder::finish` builds `MuseDoc { blocks,
/// span: Span::NONE, ..Default::default() }` (writer.rs:494-504), so
/// `title`/`author`/`date`/`description`/`keywords` always come back `None`
/// — and unlike markua, muse-fmt's `parse()` genuinely does populate these
/// fields from `#title`/`#author`/`#date`/`#desc`/`#keywords` directives
/// (parse.rs:240-249), reachable via the `document-header` fixture. The
/// `MuseEvent` enum has no variant carrying document metadata at all
/// (confirmed by reading the full enum, events.rs:27-114), so this is the
/// same expressiveness-gap bug class already tracked for org-fmt/texinfo.
#[test]
fn muse_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("muse");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/muse dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = muse_fmt::MuseDoc::parse(input.as_bytes());
        let built = String::from_utf8(doc.emit()).expect("emit produces valid UTF-8");

        let mut w = muse_fmt::Writer::new(Vec::<u8>::new());
        for e in muse_fmt::events::events(&doc) {
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
        "expected to check a substantial number of muse fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = muse_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(muse_fmt::OwnedMuseEvent::StartDocument);
        w.write_event(muse_fmt::OwnedMuseEvent::StartHeading { level: 1 });
        w.write_event(muse_fmt::OwnedMuseEvent::Text("Hello".to_string().into()));
        w.write_event(muse_fmt::OwnedMuseEvent::EndHeading);
        w.write_event(muse_fmt::OwnedMuseEvent::StartParagraph);
        w.write_event(muse_fmt::OwnedMuseEvent::Text("World".to_string().into()));
        w.write_event(muse_fmt::OwnedMuseEvent::EndParagraph);
        w.write_event(muse_fmt::OwnedMuseEvent::EndDocument);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 8 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — muse_fmt::writer::Writer \
                 buffers all events into a Vec<OwnedMuseEvent> and only reconstructs the AST + \
                 calls emit::build() inside finish() (crates/formats/muse-fmt/src/writer.rs:42- \
                 52), so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly for fixtures without document metadata"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("muse", "streaming_writer", result);
}
