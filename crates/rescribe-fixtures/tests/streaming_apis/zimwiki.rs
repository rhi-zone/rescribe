//! Streaming-API cross-checks for zimwiki. Split out of the former monolithic
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
// zimwiki: events() is parse()+eager-materialize-then-walk (EventIter::new
// calls parse::parse(input), then walks the resulting tree into a Vec before
// any event is returned — see events.rs:94-102) — a narrower claim than
// xwiki's genuinely lazy walker, in the same spirit as asciidoc's narrower
// "Wired" claim: the equivalence check validates the AST->event expansion
// layer (emit_block/emit_inline), not two independent parsers.
// StreamingParser here, like xwiki's and muse-fmt's (both fixed 2026-07-31),
// is REAL incremental: feed_line tracks verbatim-block boundaries and
// blank-line block termination and calls emit_block() during feed(), not
// deferred to finish() (batch.rs:93-152).
// ---------------------------------------------------------------------------
mod zimwiki_events_check {
    use super::{find_input, fixtures_root};
    use std::borrow::Cow;
    use zimwiki::{Block, Inline, OwnedEvent, ZimwikiDoc};

    /// Reconstruct the exact [`zimwiki::OwnedEvent`] sequence `events()` must
    /// produce for `doc`.
    fn zimwiki_ast_to_events(doc: &ZimwikiDoc) -> Vec<OwnedEvent> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            zimwiki_block_events(b, &mut out);
        }
        out
    }

    fn zimwiki_block_events(b: &Block, out: &mut Vec<OwnedEvent>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(OwnedEvent::StartParagraph);
                zimwiki_inline_events(inlines, out);
                out.push(OwnedEvent::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(OwnedEvent::StartHeading { level: *level });
                zimwiki_inline_events(inlines, out);
                out.push(OwnedEvent::EndHeading);
            }
            Block::CodeBlock { content, .. } => out.push(OwnedEvent::CodeBlock {
                content: Cow::Owned(content.clone()),
            }),
            Block::Blockquote { children, .. } => {
                out.push(OwnedEvent::StartBlockquote);
                for c in children {
                    zimwiki_block_events(c, out);
                }
                out.push(OwnedEvent::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(OwnedEvent::StartList { ordered: *ordered });
                for item in items {
                    out.push(OwnedEvent::StartListItem {
                        checked: item.checked,
                    });
                    for c in &item.children {
                        zimwiki_block_events(c, out);
                    }
                    out.push(OwnedEvent::EndListItem);
                }
                out.push(OwnedEvent::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(OwnedEvent::StartTable);
                for row in rows {
                    out.push(OwnedEvent::StartTableRow);
                    for cell in &row.cells {
                        out.push(OwnedEvent::StartTableCell);
                        zimwiki_inline_events(cell, out);
                        out.push(OwnedEvent::EndTableCell);
                    }
                    out.push(OwnedEvent::EndTableRow);
                }
                out.push(OwnedEvent::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(OwnedEvent::HorizontalRule),
        }
    }

    fn zimwiki_inline_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(OwnedEvent::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(OwnedEvent::StartBold);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(OwnedEvent::StartItalic);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndItalic);
                }
                Inline::Underline(c, _) => {
                    out.push(OwnedEvent::StartUnderline);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndUnderline);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(OwnedEvent::StartStrikethrough);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndStrikethrough);
                }
                Inline::Subscript(c, _) => {
                    out.push(OwnedEvent::StartSubscript);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndSubscript);
                }
                Inline::Superscript(c, _) => {
                    out.push(OwnedEvent::StartSuperscript);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndSuperscript);
                }
                Inline::Code(s, _) => out.push(OwnedEvent::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(OwnedEvent::StartLink { url: url.clone() });
                    zimwiki_inline_events(children, out);
                    out.push(OwnedEvent::EndLink);
                }
                Inline::Image { url, .. } => out.push(OwnedEvent::InlineImage { url: url.clone() }),
                Inline::LineBreak { .. } => out.push(OwnedEvent::LineBreak),
                Inline::SoftBreak { .. } => out.push(OwnedEvent::SoftBreak),
            }
        }
    }

    #[test]
    fn zimwiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("zimwiki");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/zimwiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = zimwiki::parse::parse(&input);
            let expected = zimwiki_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = zimwiki::events::events(&input).collect();
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
            "expected to check a substantial number of zimwiki fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} zimwiki fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `StreamingParser` fed a zimwiki fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
/// Like xwiki's and muse-fmt's (both fixed 2026-07-31),
/// `zimwiki::batch::StreamingParser::feed()` really is
/// incremental — it tracks verbatim-block (`'''`) boundaries and blank-line
/// block termination line-by-line and calls `emit_block()` during `feed()`
/// (batch.rs:93-152) — so divergences found here are genuine block-boundary
/// bugs, the same bug class already tracked for org/rst/asciidoc
/// (`emit_block()` re-parses each accumulated block in isolation via
/// `crate::events::events()`, so cross-block context such as a loose list's
/// blank-line-separated items is lost).
#[test]
fn zimwiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("zimwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/zimwiki dir") {
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
        let bulk: Vec<zimwiki::OwnedEvent> = zimwiki::events::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                zimwiki::batch::StreamingParser::new(|e: zimwiki::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of zimwiki fixtures, got {checked}"
    );
    assert_or_known_failure("zimwiki", "streaming_parser", result);
}

/// `zimwiki::writer::Writer::write_event()` only pushes to a `Vec`
/// (crates/formats/zimwiki/src/writer.rs:24-26); `finish()` reconstructs the
/// AST via `collect_doc_from_events` and calls `emit::build` once
/// (writer.rs:29-34). Content round-trips correctly (checked below), but an
/// incrementality probe shows zero bytes reach the sink before `finish()`.
#[test]
fn zimwiki_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("zimwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/zimwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = zimwiki::parse::parse(&input);
        let built = String::from_utf8(doc.emit()).expect("zimwiki emit output is UTF-8");

        let mut w = zimwiki::Writer::new(Vec::<u8>::new());
        for e in zimwiki::events::events(&input) {
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
        checked > 20,
        "expected to check a substantial number of zimwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = zimwiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(zimwiki::OwnedEvent::StartHeading { level: 1 });
        w.write_event(zimwiki::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(zimwiki::OwnedEvent::EndHeading);
        w.write_event(zimwiki::OwnedEvent::StartParagraph);
        w.write_event(zimwiki::OwnedEvent::Text("World".to_string().into()));
        w.write_event(zimwiki::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — zimwiki::writer::Writer buffers \
                 all events into a Vec<OwnedEvent> and only reconstructs the AST + calls \
                 emit::build() inside finish() (crates/formats/zimwiki/src/writer.rs:24-34), so \
                 it is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("zimwiki", "streaming_writer", result);
}
