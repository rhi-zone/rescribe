//! Streaming-API cross-checks for markua. Split out of the former monolithic
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
// markua: events() is parse()+eager-tree-build-then-walk. `EventIter::new`
// (re-exported from parse.rs, not events.rs) runs the full recursive-descent
// `Parser::parse()` before any event is returned (parse.rs:969-985); only the
// subsequent `Iterator::next()` pull over `expand_block`/`expand_inline` is
// lazy. So this is the same narrower "Wired" claim as asciidoc/zimwiki: the
// check validates the AST->event expansion layer.
// ---------------------------------------------------------------------------
mod markua_events_check {
    use super::{find_input, fixtures_root};
    use markua::{Block, Inline, MarkuaDoc, OwnedMarkuaEvent};
    use std::borrow::Cow;

    /// Reconstruct the exact [`markua::OwnedMarkuaEvent`] sequence `events()`
    /// must produce for `doc`.
    ///
    /// `Block::Figure`/`Inline::FootnoteRef` etc. are handled for
    /// completeness (an exhaustive match, so a new AST variant breaks the
    /// build), but `Block::Figure` is never constructed by `parse()` —
    /// confirmed by reading `crates/formats/markua/src/parse.rs` and
    /// `emit.rs`, neither of which has any code path building a `Figure`
    /// from Markua syntax — so it is unreachable via any fixture in this
    /// check and its ordering (caption before body, settled from
    /// `expand_block`'s `Block::Figure` arm, parse.rs:1071-1082) is untested
    /// here.
    fn markua_ast_to_events(doc: &MarkuaDoc) -> Vec<OwnedMarkuaEvent> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            markua_block_events(b, &mut out);
        }
        out
    }

    fn markua_block_events(b: &Block, out: &mut Vec<OwnedMarkuaEvent>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(OwnedMarkuaEvent::StartParagraph);
                markua_inline_events(inlines, out);
                out.push(OwnedMarkuaEvent::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(OwnedMarkuaEvent::StartHeading { level: *level });
                markua_inline_events(inlines, out);
                out.push(OwnedMarkuaEvent::EndHeading);
            }
            Block::CodeBlock {
                content, language, ..
            } => out.push(OwnedMarkuaEvent::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            }),
            Block::Blockquote { children, .. } => {
                out.push(OwnedMarkuaEvent::StartBlockquote);
                for c in children {
                    markua_block_events(c, out);
                }
                out.push(OwnedMarkuaEvent::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(OwnedMarkuaEvent::StartList { ordered: *ordered });
                for item in items {
                    out.push(OwnedMarkuaEvent::StartListItem);
                    for c in item {
                        markua_block_events(c, out);
                    }
                    out.push(OwnedMarkuaEvent::EndListItem);
                }
                out.push(OwnedMarkuaEvent::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(OwnedMarkuaEvent::StartTable);
                for row in rows {
                    out.push(OwnedMarkuaEvent::StartTableRow);
                    for cell in &row.cells {
                        out.push(OwnedMarkuaEvent::StartTableCell);
                        markua_inline_events(cell, out);
                        out.push(OwnedMarkuaEvent::EndTableCell);
                    }
                    out.push(OwnedMarkuaEvent::EndTableRow);
                }
                out.push(OwnedMarkuaEvent::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(OwnedMarkuaEvent::HorizontalRule),
            Block::SpecialBlock {
                block_type,
                children,
                ..
            } => {
                out.push(OwnedMarkuaEvent::StartSpecialBlock {
                    kind: block_type.clone(),
                });
                for c in children {
                    markua_block_events(c, out);
                }
                out.push(OwnedMarkuaEvent::EndSpecialBlock);
            }
            Block::DefinitionList { items, .. } => {
                out.push(OwnedMarkuaEvent::StartDefinitionList);
                for (term, desc) in items {
                    out.push(OwnedMarkuaEvent::StartDefinitionTerm);
                    markua_inline_events(term, out);
                    out.push(OwnedMarkuaEvent::EndDefinitionTerm);
                    out.push(OwnedMarkuaEvent::StartDefinitionDesc);
                    for b in desc {
                        markua_block_events(b, out);
                    }
                    out.push(OwnedMarkuaEvent::EndDefinitionDesc);
                }
                out.push(OwnedMarkuaEvent::EndDefinitionList);
            }
            Block::PageBreak { .. } => out.push(OwnedMarkuaEvent::PageBreak),
            Block::Figure { caption, body, .. } => {
                out.push(OwnedMarkuaEvent::StartFigure);
                if !caption.is_empty() {
                    out.push(OwnedMarkuaEvent::StartCaption);
                    markua_inline_events(caption, out);
                    out.push(OwnedMarkuaEvent::EndCaption);
                }
                markua_block_events(body, out);
                out.push(OwnedMarkuaEvent::EndFigure);
            }
        }
    }

    fn markua_inline_events(inlines: &[Inline], out: &mut Vec<OwnedMarkuaEvent>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(OwnedMarkuaEvent::Text(Cow::Owned(s.clone()))),
                Inline::Strong(c, _) => {
                    out.push(OwnedMarkuaEvent::StartStrong);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndStrong);
                }
                Inline::Emphasis(c, _) => {
                    out.push(OwnedMarkuaEvent::StartEmphasis);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndEmphasis);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(OwnedMarkuaEvent::StartStrikethrough);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndStrikethrough);
                }
                Inline::Subscript(c, _) => {
                    out.push(OwnedMarkuaEvent::StartSubscript);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndSubscript);
                }
                Inline::Superscript(c, _) => {
                    out.push(OwnedMarkuaEvent::StartSuperscript);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndSuperscript);
                }
                Inline::Underline(c, _) => {
                    out.push(OwnedMarkuaEvent::StartUnderline);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndUnderline);
                }
                Inline::SmallCaps(c, _) => {
                    out.push(OwnedMarkuaEvent::StartSmallCaps);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndSmallCaps);
                }
                Inline::Code(s, _) => out.push(OwnedMarkuaEvent::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(OwnedMarkuaEvent::StartLink { url: url.clone() });
                    markua_inline_events(children, out);
                    out.push(OwnedMarkuaEvent::EndLink);
                }
                Inline::Image { url, alt, .. } => out.push(OwnedMarkuaEvent::Image {
                    url: url.clone(),
                    alt: alt.clone(),
                }),
                Inline::LineBreak(_) => out.push(OwnedMarkuaEvent::LineBreak),
                Inline::SoftBreak(_) => out.push(OwnedMarkuaEvent::SoftBreak),
                Inline::FootnoteRef { content, .. } => {
                    out.push(OwnedMarkuaEvent::StartFootnoteRef);
                    markua_inline_events(content, out);
                    out.push(OwnedMarkuaEvent::EndFootnoteRef);
                }
                Inline::IndexTerm { term, .. } => {
                    out.push(OwnedMarkuaEvent::IndexTerm { term: term.clone() })
                }
                Inline::MathInline { content, .. } => out.push(OwnedMarkuaEvent::MathInline {
                    content: content.clone(),
                }),
            }
        }
    }

    #[test]
    fn markua_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("markua");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/markua dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = markua::parse(&input);
            let expected = markua_ast_to_events(&doc);
            let actual: Vec<OwnedMarkuaEvent> =
                markua::events(&input).map(|e| e.into_owned()).collect();
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
            "expected to check a substantial number of markua fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} markua fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `StreamingParser` fed a markua fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
/// `markua::batch::StreamingParser::feed()` is REAL incremental
/// block-boundary segmentation (fenced-code-aware `feed_line`, batch.rs:108-
/// 152), like xwiki's and muse-fmt's (both fixed 2026-07-31) —
/// `emit_block()` re-parses each accumulated block via `crate::events::events()`,
/// the same architecture (and bug class) already tracked for
/// org/rst/asciidoc/zimwiki.
#[test]
fn markua_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("markua");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/markua dir") {
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
        let bulk: Vec<markua::OwnedMarkuaEvent> =
            markua::events(input_str).map(|e| e.into_owned()).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                markua::batch::StreamingParser::new(|e: markua::OwnedMarkuaEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():         {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 20,
        "expected to check a substantial number of markua fixtures, got {checked}"
    );
    assert_or_known_failure("markua", "streaming_parser", result);
}

/// `markua::writer::Writer::write_event()` only pushes to a `Vec`
/// (crates/formats/markua/src/writer.rs:40-42); `finish()` reconstructs the
/// AST via `events_to_doc`/`DocBuilder` and calls `emit::emit` once
/// (writer.rs:45-50). Content round-trips correctly for every fixture
/// (checked below — `MarkuaDoc::title`/`author`/`description` are always
/// `None` regardless of path: `parse()` itself never populates them from any
/// Markua syntax, confirmed by reading `parse.rs`'s `pub fn parse`, which
/// hardcodes `title: None, author: None, description: None` unconditionally
/// — so the `DocBuilder::finish` hardcoding the same is not a reachable
/// divergence via any fixture). An incrementality probe shows zero bytes
/// reach the sink before `finish()`.
#[test]
fn markua_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("markua");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/markua dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = markua::parse(&input);
        let built = markua::build(&doc);

        let mut w = markua::Writer::new(Vec::<u8>::new());
        for e in markua::events(&input) {
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
        "expected to check a substantial number of markua fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = markua::Writer::new(ObservableSink(observed.clone()));
        w.write_event(markua::OwnedMarkuaEvent::StartHeading { level: 1 });
        w.write_event(markua::OwnedMarkuaEvent::Text("Hello".to_string().into()));
        w.write_event(markua::OwnedMarkuaEvent::EndHeading);
        w.write_event(markua::OwnedMarkuaEvent::StartParagraph);
        w.write_event(markua::OwnedMarkuaEvent::Text("World".to_string().into()));
        w.write_event(markua::OwnedMarkuaEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — markua::writer::Writer buffers \
                 all events into a Vec<OwnedMarkuaEvent> and only reconstructs the AST + calls \
                 emit::emit() inside finish() (crates/formats/markua/src/writer.rs:40-50), so it \
                 is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("markua", "streaming_writer", result);
}
