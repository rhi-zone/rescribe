//! Streaming-API cross-checks for commonmark. Split out of the former monolithic
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
// commonmark-fmt: shared by the "commonmark", "gfm", and "markdown" fixture
// formats (all three are wrappers over the same commonmark-fmt crate). This
// section wires each API exactly once against the "commonmark" fixture set
// (the superset — gfm/markdown fixtures exercise the same crate code paths)
// and the CAPABILITIES/KNOWN_FAILURES entries for gfm/markdown just cite
// these findings, per the task's "shares one crate audit" instruction.
//
// events() wraps pulldown-cmark's OffsetIter and is genuinely lazy/pull-based
// (real, passing). StreamingParser buffering all input before parsing is the
// sole CLAUDE.md-sanctioned pulldown-cmark exemption (documented in
// crates/formats/commonmark-fmt/src/lib.rs and src/batch.rs) — NotApplicable,
// no check needed. The streaming Writer, by contrast, self-admits in its own
// module doc that it "buffers all events, reconstructs the AST, then emits"
// — unrelated to the sanctioned reader exemption (the writer never touches
// pulldown-cmark) and a fake streaming writer per CLAUDE.md; tracked as a
// KnownFailure via the same incrementality-probe pattern used for
// texinfo/textile above.
// ---------------------------------------------------------------------------

fn commonmark_ast_to_events(
    doc: &commonmark_fmt::CmDoc,
) -> Vec<commonmark_fmt::events::Event<'static>> {
    use commonmark_fmt::events::Event;
    let mut out = vec![Event::StartDocument];
    if let Some(fm) = &doc.frontmatter {
        out.push(Event::FrontMatter {
            kind: fm.kind,
            content: std::borrow::Cow::Owned(fm.content.clone()),
        });
    }
    for b in &doc.blocks {
        commonmark_block_events(b, &mut out);
    }
    // link_defs land after the body, before EndDocument — see
    // commonmark_fmt::events::Event::LinkDef's doc comment.
    for def in &doc.link_defs {
        out.push(Event::LinkDef {
            label: std::borrow::Cow::Owned(def.label.clone()),
            url: std::borrow::Cow::Owned(def.url.clone()),
            title: def.title.clone().map(std::borrow::Cow::Owned),
        });
    }
    out.push(Event::EndDocument);
    out
}

fn commonmark_block_events(
    b: &commonmark_fmt::ast::Block,
    out: &mut Vec<commonmark_fmt::events::Event<'static>>,
) {
    use commonmark_fmt::ast::Block;
    use commonmark_fmt::events::Event;
    use std::borrow::Cow;
    match b {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                commonmark_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            for i in inlines {
                commonmark_inline_events(i, out);
            }
            out.push(Event::EndHeading { level: *level });
        }
        Block::CodeBlock {
            language, content, ..
        } => {
            out.push(Event::CodeBlock {
                language: language.clone().map(Cow::Owned),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::HtmlBlock { content, .. } => {
            out.push(Event::HtmlBlock(Cow::Owned(content.clone())));
        }
        Block::Blockquote { blocks, .. } => {
            out.push(Event::StartBlockquote);
            for c in blocks {
                commonmark_block_events(c, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List {
            kind, items, tight, ..
        } => {
            use commonmark_fmt::ast::ListKind;
            let (ordered, start) = match kind {
                ListKind::Unordered { .. } => (false, 1),
                ListKind::Ordered { start, .. } => (true, *start),
            };
            // EventIter always emits StartList { tight: true } optimistically,
            // regardless of the AST's real tight value — see
            // commonmark_fmt::events::Event::ListTightnessResolved's doc comment.
            out.push(Event::StartList {
                ordered,
                start,
                tight: true,
            });
            for item in items {
                out.push(Event::StartItem {
                    checked: item.checked,
                });
                for c in &item.blocks {
                    commonmark_block_events(c, out);
                }
                out.push(Event::EndItem);
            }
            // EventIter always emits StartList { tight: true } optimistically
            // and, only for a loose list, corrects it once right before
            // EndList — see commonmark_fmt::events::Event::ListTightnessResolved's
            // doc comment.
            if !*tight {
                out.push(Event::ListTightnessResolved { tight: false });
            }
            out.push(Event::EndList);
        }
        Block::ThematicBreak { .. } => out.push(Event::ThematicBreak),
        Block::Table {
            alignments,
            head,
            rows,
            ..
        } => {
            out.push(Event::StartTable {
                alignments: alignments.clone(),
            });
            out.push(Event::StartTableHead);
            out.push(Event::StartTableRow);
            for cell in &head.cells {
                out.push(Event::StartTableCell);
                for i in &cell.inlines {
                    commonmark_inline_events(i, out);
                }
                out.push(Event::EndTableCell);
            }
            out.push(Event::EndTableRow);
            out.push(Event::EndTableHead);
            for row in rows {
                out.push(Event::StartTableRow);
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    for i in &cell.inlines {
                        commonmark_inline_events(i, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::FootnoteDefinition { label, blocks, .. } => {
            out.push(Event::StartFootnoteDefinition {
                label: Cow::Owned(label.clone()),
            });
            for c in blocks {
                commonmark_block_events(c, out);
            }
            out.push(Event::EndFootnoteDefinition);
        }
        Block::DefinitionList { items, tight, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionListTitle);
                for i in &item.term {
                    commonmark_inline_events(i, out);
                }
                out.push(Event::EndDefinitionListTitle);
                for def_blocks in &item.definitions {
                    out.push(Event::StartDefinitionListDefinition);
                    for c in def_blocks {
                        commonmark_block_events(c, out);
                    }
                    out.push(Event::EndDefinitionListDefinition);
                }
            }
            // EventIter always emits StartDefinitionList optimistically and,
            // only for a loose list, corrects it once right before
            // EndDefinitionList — see
            // commonmark_fmt::events::Event::DefinitionListTightnessResolved's
            // doc comment (mirrors ListTightnessResolved).
            if !*tight {
                out.push(Event::DefinitionListTightnessResolved { tight: false });
            }
            out.push(Event::EndDefinitionList);
        }
    }
}

fn commonmark_inline_events(
    i: &commonmark_fmt::ast::Inline,
    out: &mut Vec<commonmark_fmt::events::Event<'static>>,
) {
    use commonmark_fmt::ast::Inline;
    use commonmark_fmt::events::Event;
    use std::borrow::Cow;
    match i {
        Inline::Text { content, .. } => out.push(Event::Text(Cow::Owned(content.clone()))),
        Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
        Inline::HardBreak { .. } => out.push(Event::HardBreak),
        Inline::Emphasis { inlines, .. } => {
            out.push(Event::StartEmphasis);
            for c in inlines {
                commonmark_inline_events(c, out);
            }
            out.push(Event::EndEmphasis);
        }
        Inline::Strong { inlines, .. } => {
            out.push(Event::StartStrong);
            for c in inlines {
                commonmark_inline_events(c, out);
            }
            out.push(Event::EndStrong);
        }
        Inline::Strikethrough { inlines, .. } => {
            out.push(Event::StartStrikethrough);
            for c in inlines {
                commonmark_inline_events(c, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Code { content, .. } => out.push(Event::Code(Cow::Owned(content.clone()))),
        Inline::HtmlInline { content, .. } => {
            out.push(Event::HtmlInline(Cow::Owned(content.clone())));
        }
        Inline::Link {
            inlines,
            url,
            title,
            ..
        } => {
            out.push(Event::StartLink {
                url: Cow::Owned(url.clone()),
                title: title.clone().map(Cow::Owned),
            });
            for c in inlines {
                commonmark_inline_events(c, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image {
            alt, url, title, ..
        } => {
            out.push(Event::StartImage {
                url: Cow::Owned(url.clone()),
                title: title.clone().map(Cow::Owned),
                alt: Cow::Owned(alt.clone()),
            });
            if !alt.is_empty() {
                out.push(Event::Text(Cow::Owned(alt.clone())));
            }
            out.push(Event::EndImage);
        }
        Inline::FootnoteReference { label, .. } => out.push(Event::FootnoteReference {
            label: Cow::Owned(label.clone()),
        }),
        Inline::InlineMath { source, .. } => {
            out.push(Event::InlineMath(Cow::Owned(source.clone())))
        }
        Inline::DisplayMath { source, .. } => {
            out.push(Event::DisplayMath(Cow::Owned(source.clone())));
        }
    }
}

#[test]
fn commonmark_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("commonmark");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/commonmark dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = commonmark_fmt::CmDoc::parse(&input);
        let expected = commonmark_ast_to_events(&doc);
        let Some(actual_iter) = commonmark_fmt::events::events(&input) else {
            continue; // non-UTF8 input: events() returns None, not this check's concern
        };
        let actual: Vec<_> = actual_iter.map(|e| e.into_owned()).collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  ast-derived: \
                 {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of commonmark fixtures, got {checked}"
    );
    assert_or_known_failure("commonmark", "events", result);
}

/// `commonmark_fmt::writer::Writer` self-admits (see its own module doc) that
/// it buffers all fed events into a `Vec<OwnedEvent>` and only reconstructs
/// the AST + calls `emit()` inside `finish()`. Checked the same way as
/// texinfo/textile above: byte-identical-to-builder content correctness,
/// plus an incrementality probe (write a full paragraph, check whether any
/// bytes reached the sink before `finish()`).
#[test]
fn commonmark_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("commonmark");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/commonmark dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = commonmark_fmt::CmDoc::parse(&input);
        let built = doc.emit();

        let Some(events_iter) = commonmark_fmt::events::events(&input) else {
            continue;
        };
        let mut w = commonmark_fmt::Writer::new(Vec::<u8>::new());
        for e in events_iter {
            w.write_event(e);
        }
        let streamed = w.finish().expect("Writer::finish");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of commonmark fixtures, got {checked}"
    );

    if result.is_ok() {
        use commonmark_fmt::events::Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = commonmark_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartDocument);
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text("Hello world".into()));
        w.write_event(Event::EndParagraph);
        w.write_event(Event::EndDocument);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartDocument/StartParagraph/Text/EndParagraph/EndDocument sequence and before \
                 finish() — commonmark_fmt::writer::Writer buffers all events into a \
                 Vec<OwnedEvent> and only reconstructs the AST + calls emit() inside finish() \
                 (crates/formats/commonmark-fmt/src/writer.rs, self-admitted in its own module \
                 doc), so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("commonmark", "streaming_writer", result);
}
