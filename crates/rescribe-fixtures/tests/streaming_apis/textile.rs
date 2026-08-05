//! Streaming-API cross-checks for textile. Split out of the former monolithic
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
// textile-fmt: events() vs parse(), real and passing. StreamingParser and
// the streaming Writer are both buffer-until-finish wrappers (see their own
// module docs in crates/formats/textile-fmt/src/batch.rs and writer.rs,
// which self-report "buffers all input" / "buffers all events,
// reconstructs the AST, then emits") rather than genuine incremental
// implementations; tracked as KnownFailure entries.
// ---------------------------------------------------------------------------

fn textile_ast_to_events(doc: &textile_fmt::TextileDoc) -> Vec<textile_fmt::TextileEvent> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        textile_block_events(b, &mut out);
    }
    out
}

fn textile_block_events(b: &textile_fmt::Block, out: &mut Vec<textile_fmt::TextileEvent>) {
    use textile_fmt::Block;
    use textile_fmt::TextileEvent;
    match b {
        Block::Paragraph {
            inlines,
            align,
            attrs,
            ..
        } => {
            out.push(TextileEvent::StartParagraph {
                align: align.clone(),
                attrs: attrs.clone(),
            });
            for i in inlines {
                textile_inline_events(i, out);
            }
            out.push(TextileEvent::EndParagraph);
        }
        Block::Heading {
            level,
            inlines,
            attrs,
            ..
        } => {
            out.push(TextileEvent::StartHeading {
                level: *level,
                attrs: attrs.clone(),
            });
            for i in inlines {
                textile_inline_events(i, out);
            }
            out.push(TextileEvent::EndHeading);
        }
        Block::CodeBlock {
            content, language, ..
        } => {
            out.push(TextileEvent::CodeBlock {
                content: content.clone(),
                language: language.clone(),
            });
        }
        Block::Blockquote { blocks, attrs, .. } => {
            out.push(TextileEvent::StartBlockquote {
                attrs: attrs.clone(),
            });
            for b in blocks {
                textile_block_events(b, out);
            }
            out.push(TextileEvent::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(TextileEvent::StartList { ordered: *ordered });
            for item_blocks in items {
                out.push(TextileEvent::StartListItem);
                for b in item_blocks {
                    textile_block_events(b, out);
                }
                out.push(TextileEvent::EndListItem);
            }
            out.push(TextileEvent::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(TextileEvent::StartTable);
            for row in rows {
                out.push(TextileEvent::StartTableRow {
                    attrs: row.attrs.clone(),
                });
                for cell in &row.cells {
                    out.push(TextileEvent::StartTableCell {
                        is_header: cell.is_header,
                        align: cell.align.clone(),
                    });
                    for i in &cell.inlines {
                        textile_inline_events(i, out);
                    }
                    out.push(TextileEvent::EndTableCell);
                }
                out.push(TextileEvent::EndTableRow);
            }
            out.push(TextileEvent::EndTable);
        }
        Block::HorizontalRule { .. } => out.push(TextileEvent::HorizontalRule),
        Block::FootnoteDef { label, inlines, .. } => {
            out.push(TextileEvent::StartFootnoteDef {
                label: label.clone(),
            });
            for i in inlines {
                textile_inline_events(i, out);
            }
            out.push(TextileEvent::EndFootnoteDef);
        }
        Block::DefinitionList { items, .. } => {
            out.push(TextileEvent::StartDefinitionList);
            for (term, def) in items {
                out.push(TextileEvent::StartDefinitionTerm);
                for i in term {
                    textile_inline_events(i, out);
                }
                out.push(TextileEvent::EndDefinitionTerm);
                out.push(TextileEvent::StartDefinitionDesc);
                for i in def {
                    textile_inline_events(i, out);
                }
                out.push(TextileEvent::EndDefinitionDesc);
            }
            out.push(TextileEvent::EndDefinitionList);
        }
        Block::Raw { content, .. } => out.push(TextileEvent::RawBlock {
            content: content.clone(),
        }),
    }
}

fn textile_inline_events(i: &textile_fmt::Inline, out: &mut Vec<textile_fmt::TextileEvent>) {
    use textile_fmt::Inline;
    use textile_fmt::TextileEvent;
    match i {
        Inline::Text(s, _) => out.push(TextileEvent::Text(s.clone())),
        Inline::Bold(c, _) => {
            out.push(TextileEvent::StartBold);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndBold);
        }
        Inline::Italic(c, _) => {
            out.push(TextileEvent::StartItalic);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndItalic);
        }
        Inline::Underline(c, _) => {
            out.push(TextileEvent::StartUnderline);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndUnderline);
        }
        Inline::Strikethrough(c, _) => {
            out.push(TextileEvent::StartStrikethrough);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndStrikethrough);
        }
        Inline::Code(s, _) => out.push(TextileEvent::InlineCode(s.clone())),
        Inline::Link {
            url,
            title,
            children,
            ..
        } => {
            out.push(TextileEvent::StartLink {
                url: url.clone(),
                title: title.clone(),
            });
            for x in children {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndLink);
        }
        Inline::Image { url, alt, .. } => out.push(TextileEvent::InlineImage {
            url: url.clone(),
            alt: alt.clone(),
        }),
        Inline::Superscript(c, _) => {
            out.push(TextileEvent::StartSuperscript);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndSuperscript);
        }
        Inline::Subscript(c, _) => {
            out.push(TextileEvent::StartSubscript);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndSubscript);
        }
        Inline::FootnoteRef { label, .. } => out.push(TextileEvent::FootnoteRef {
            label: label.clone(),
        }),
        Inline::LineBreak(_) => out.push(TextileEvent::LineBreak),
        Inline::Raw(s, _) => out.push(TextileEvent::RawInline { content: s.clone() }),
        Inline::Citation(c, _) => {
            out.push(TextileEvent::StartCitation);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndCitation);
        }
        Inline::GenericSpan {
            attrs, children, ..
        } => {
            out.push(TextileEvent::StartGenericSpan {
                attrs: attrs.clone(),
            });
            for x in children {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndGenericSpan);
        }
        Inline::Acronym { text, title, .. } => out.push(TextileEvent::Acronym {
            text: text.clone(),
            title: title.clone(),
        }),
    }
}

#[test]
fn textile_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("textile");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/textile dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = textile_fmt::parse::parse(&input);
        let expected = textile_ast_to_events(&doc);
        let actual: Vec<_> = textile_fmt::events(&input).collect();
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
        "expected to check a substantial number of textile fixtures, got {checked}"
    );
}

/// `StreamingParser` is genuinely incremental (see
/// `crates/formats/textile-fmt/src/batch.rs`): it flushes a top-level block
/// to the handler as soon as a later buffered line proves the block's
/// boundary decision can't change, rather than buffering all input until
/// `finish()`. Checks (1) equivalence with `events()` under adversarial
/// chunking, per fixture, and (2) incremental delivery via a hand-built
/// probe below.
#[test]
fn textile_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("textile");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/textile dir") {
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
        let bulk: Vec<textile_fmt::TextileEvent> = textile_fmt::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                textile_fmt::batch::StreamingParser::new(|e: textile_fmt::TextileEvent| {
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

        // Deliberately NOT probed here: an arbitrary 50%-byte split of each
        // real fixture. textile's streaming granularity is one top-level
        // block, so for a single-block fixture whose first line alone
        // exceeds half the file (e.g. `acronym`: a 66-byte first line of 124
        // total bytes) not even one complete line exists at the halfway
        // point — zero events delivered there is the correct answer for a
        // line-oriented block parser, not a buffer-then-finish defect. See
        // the hand-built probe below, which guarantees an unambiguous
        // complete-block boundary.
    }
    assert!(
        checked > 5,
        "expected to check several textile fixtures, got {checked}"
    );

    // Incrementality probe: a hand-built input with a definite complete
    // prefix (an `h1.` heading and an `h2.` heading, each a single-line block
    // that flushes on its own newline, plus a full ordinary paragraph that
    // flushes on the blank line after it) followed by deliberately
    // unterminated trailing content (a partial line with no trailing newline
    // at all, so it never reaches feed_line and stays buffered). Confirms
    // events reach the handler after feed() alone, before finish() is ever
    // called — the property that failed when feed() only extended a Vec<u8>.
    if result.is_ok() {
        let probe_input = b"h1. Title\n\nFirst paragraph.\n\nh2. Sub\n\n\
                             Unterminated trailing paragraph with no closing newline";
        let mut delivered: Vec<textile_fmt::TextileEvent> = Vec::new();
        let mut parser = textile_fmt::batch::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("textile", !delivered.is_empty());
        // `parser` intentionally dropped without calling finish(): this probe
        // only needs to observe pre-finish handler state.
    }

    assert_or_known_failure("textile", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<TextileEvent>` and only
/// reconstructs the AST + calls `emit()` inside `finish()` (see
/// `crates/formats/textile-fmt/src/writer.rs`'s own module doc). Checked via
/// byte-identical comparison against the builder path across all fixtures.
#[test]
fn textile_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("textile");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/textile dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = textile_fmt::parse::parse(&input);
        let built = textile_fmt::emit::emit(&doc);

        let mut w = textile_fmt::writer::Writer::new(Vec::<u8>::new());
        for e in textile_fmt::events(&input) {
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
        "expected to check a substantial number of textile fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming. Feed a full paragraph
    // (well short of finish()) and check whether any bytes already reached
    // the sink.
    if result.is_ok() {
        use textile_fmt::TextileEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = textile_fmt::writer::Writer::new(ObservableSink(observed.clone()));
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: Default::default(),
        });
        w.write_event(TextileEvent::Text("Hello world".to_string()));
        w.write_event(TextileEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — textile_fmt::writer::Writer \
                 buffers all events into a Vec<TextileEvent> and only reconstructs the AST + \
                 calls emit() inside finish() (crates/formats/textile-fmt/src/writer.rs), so it \
                 is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("textile", "streaming_writer", result);
}
