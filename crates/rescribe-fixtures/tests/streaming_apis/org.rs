//! Streaming-API cross-checks for org. Split out of the former monolithic
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
// org-fmt: events() vs parse(), fully wired
// ---------------------------------------------------------------------------
//
// org-fmt's `events()` is a genuinely independent implementation, not a
// `parse()`-then-walk wrapper — the dependency runs the other way. `EventIter`
// (defined in `parse.rs`, re-exported by `events.rs`) is a lazy pull parser
// over the source lines, and `parse()` is built on top of it by repeatedly
// calling `parse_next_block()`. So this equivalence check compares two real
// code paths: the AST `parse()` assembles, and the events `expand_block`/
// `expand_inline` unfold.
mod org_events_check {
    use super::{find_input, fixtures_root};
    use org_fmt::{Block, Inline, ListItemContent, OrgDoc, OwnedEvent};
    use std::borrow::Cow;

    /// Reconstruct the exact `OwnedEvent` sequence `events()` must produce for
    /// `doc`.
    ///
    /// `OrgDoc::metadata` deliberately produces no events: the `Event` enum has
    /// no metadata variant, so document metadata is out-of-band for the
    /// streaming API.
    fn org_ast_to_events(doc: &OrgDoc) -> Vec<OwnedEvent> {
        let mut out = Vec::new();
        // All fixtures that carry `#+KEY: value` metadata lines have them at
        // the very top of the document, before any block content, so a flat
        // prefix of `Metadata` events matches `events()`'s actual output.
        // (`OrgDoc.metadata` itself carries no per-entry source position, so
        // this projection can't reconstruct interleaving if a future fixture
        // put metadata lines between blocks — see `Event::Metadata`'s doc
        // comment for where `events()` actually places them.)
        for (key, value) in &doc.metadata {
            out.push(OwnedEvent::Metadata {
                key: key.clone(),
                value: value.clone(),
            });
        }
        for b in &doc.blocks {
            org_block_events(b, &mut out);
        }
        out
    }

    fn org_block_events(b: &Block, out: &mut Vec<OwnedEvent>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(OwnedEvent::StartParagraph);
                org_inline_events(inlines, out);
                out.push(OwnedEvent::EndParagraph);
            }
            Block::Heading {
                level,
                todo,
                priority,
                tags,
                properties,
                scheduled,
                deadline,
                inlines,
                ..
            } => {
                // Heading attributes all ride on StartHeading; the heading's own
                // title text is the inline run between Start/EndHeading. Nested
                // headings are siblings in `OrgDoc::blocks`, not children, so
                // EndHeading immediately follows the title inlines.
                out.push(OwnedEvent::StartHeading {
                    level: *level,
                    todo: todo.clone(),
                    priority: priority.clone(),
                    tags: tags.clone(),
                    properties: properties.clone(),
                    scheduled: scheduled.clone(),
                    deadline: deadline.clone(),
                });
                org_inline_events(inlines, out);
                out.push(OwnedEvent::EndHeading);
            }
            Block::CodeBlock {
                language,
                header_args,
                name,
                content,
                ..
            } => out.push(OwnedEvent::CodeBlock {
                language: language.clone(),
                header_args: header_args.clone(),
                name: name.clone(),
                content: Cow::Owned(content.clone()),
            }),
            Block::Blockquote { children, .. } => {
                out.push(OwnedEvent::StartBlockquote);
                for c in children {
                    org_block_events(c, out);
                }
                out.push(OwnedEvent::EndBlockquote);
            }
            Block::List {
                ordered,
                start,
                items,
                ..
            } => {
                out.push(OwnedEvent::StartList {
                    ordered: *ordered,
                    start: *start,
                });
                for item in items {
                    out.push(OwnedEvent::StartListItem {
                        checkbox: item.checkbox,
                    });
                    for child in &item.children {
                        match child {
                            // `ListItemContent::Inline` is a bare inline run — the
                            // tree builder (`events::handle_event`, the
                            // `BlockFrame::ListItem { inline_buf, .. }` arm)
                            // accumulates inlines seen directly inside a list item
                            // with no enclosing paragraph, so the projection emits
                            // them unwrapped.
                            ListItemContent::Inline(inlines) => org_inline_events(inlines, out),
                            ListItemContent::Block(block) => org_block_events(block, out),
                        }
                    }
                    out.push(OwnedEvent::EndListItem);
                }
                out.push(OwnedEvent::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(OwnedEvent::StartTable);
                for row in rows {
                    out.push(OwnedEvent::StartTableRow {
                        is_header: row.is_header,
                    });
                    for cell in &row.cells {
                        out.push(OwnedEvent::StartTableCell);
                        org_inline_events(cell, out);
                        out.push(OwnedEvent::EndTableCell);
                    }
                    out.push(OwnedEvent::EndTableRow);
                }
                out.push(OwnedEvent::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(OwnedEvent::HorizontalRule),
            Block::DefinitionList { items, .. } => {
                out.push(OwnedEvent::StartDefinitionList);
                for item in items {
                    // Term then desc, per item: settled from `handle_event`'s
                    // EndDefinitionTerm arm (pushes a partial `DefinitionItem`)
                    // and EndDefinitionDesc arm (fills in `items.last_mut()`).
                    out.push(OwnedEvent::StartDefinitionTerm);
                    org_inline_events(&item.term, out);
                    out.push(OwnedEvent::EndDefinitionTerm);
                    out.push(OwnedEvent::StartDefinitionDesc);
                    org_inline_events(&item.desc, out);
                    out.push(OwnedEvent::EndDefinitionDesc);
                }
                out.push(OwnedEvent::EndDefinitionList);
            }
            Block::Div { inlines, .. } => {
                out.push(OwnedEvent::StartDiv);
                org_inline_events(inlines, out);
                out.push(OwnedEvent::EndDiv);
            }
            Block::RawBlock {
                format, content, ..
            } => out.push(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            }),
            Block::Figure { name, children, .. } => {
                out.push(OwnedEvent::StartFigure { name: name.clone() });
                for c in children {
                    org_block_events(c, out);
                }
                out.push(OwnedEvent::EndFigure);
            }
            Block::Caption { inlines, .. } => {
                out.push(OwnedEvent::StartCaption);
                org_inline_events(inlines, out);
                out.push(OwnedEvent::EndCaption);
            }
            Block::FootnoteDef { label, content, .. } => {
                // Block-level footnote definitions map to the Block* pair; the
                // inline `Inline::FootnoteDefinition` maps to
                // Start/EndFootnoteDefinition. Both pairs exist and are distinct.
                out.push(OwnedEvent::StartBlockFootnoteDef {
                    label: label.clone(),
                });
                org_inline_events(content, out);
                out.push(OwnedEvent::EndBlockFootnoteDef);
            }
            Block::Unknown { kind, .. } => {
                out.push(OwnedEvent::UnknownBlock { kind: kind.clone() })
            }
        }
    }

    fn org_inline_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
        for i in inlines {
            match i {
                Inline::Text { text, .. } => out.push(OwnedEvent::Text(Cow::Owned(text.clone()))),
                Inline::Bold(c, _) => {
                    out.push(OwnedEvent::StartBold);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(OwnedEvent::StartItalic);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndItalic);
                }
                Inline::Underline(c, _) => {
                    out.push(OwnedEvent::StartUnderline);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndUnderline);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(OwnedEvent::StartStrikethrough);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(OwnedEvent::StartSuperscript);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(OwnedEvent::StartSubscript);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndSubscript);
                }
                Inline::Code(s, _) => out.push(OwnedEvent::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(OwnedEvent::StartLink { url: url.clone() });
                    org_inline_events(children, out);
                    out.push(OwnedEvent::EndLink);
                }
                Inline::Image { url, .. } => out.push(OwnedEvent::InlineImage { url: url.clone() }),
                Inline::LineBreak { .. } => out.push(OwnedEvent::LineBreak),
                Inline::SoftBreak { .. } => out.push(OwnedEvent::SoftBreak),
                Inline::FootnoteRef { label, .. } => out.push(OwnedEvent::FootnoteRef {
                    label: label.clone(),
                }),
                Inline::FootnoteDefinition {
                    label, children, ..
                } => {
                    out.push(OwnedEvent::StartFootnoteDefinition {
                        label: label.clone(),
                    });
                    org_inline_events(children, out);
                    out.push(OwnedEvent::EndFootnoteDefinition);
                }
                Inline::MathInline { source, .. } => out.push(OwnedEvent::MathInline {
                    source: source.clone(),
                }),
                Inline::Timestamp { active, value, .. } => out.push(OwnedEvent::Timestamp {
                    active: *active,
                    value: value.clone(),
                }),
                Inline::ExportSnippet { backend, value, .. } => {
                    out.push(OwnedEvent::ExportSnippet {
                        backend: backend.clone(),
                        value: value.clone(),
                    })
                }
            }
        }
    }

    /// `events()` must equal the hand-written AST projection exactly, over
    /// every fixture in `fixtures/org/`.
    ///
    /// Coverage note (measured while writing this, not assumed): across the
    /// 89 `fixtures/org/` directories this exercises 58 of org-fmt's 59
    /// `Event` variants, so nearly every projection arm is load-bearing
    /// rather than a dead arm agreeing trivially. The one variant never
    /// produced is `Event::UnknownBlock`: `parse.rs` never constructs
    /// `Block::Unknown` (its only mention, at parse.rs:961, is a *match* arm
    /// in `expand_block`), so an unknown `#+BEGIN_FOO` block becomes a
    /// `Block::Div` and the block-kind string is silently dropped. That is a
    /// reader losslessness gap, tracked in TODO.md — not an events()/parse()
    /// divergence, so it does not belong in KNOWN_FAILURES for this check.
    #[test]
    fn org_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("org");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/org dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = org_fmt::parse(&input);
            let expected = org_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = org_fmt::events(&input)
                .map(org_fmt::Event::into_owned)
                .collect();
            // Counted once both sequences exist, not once they match — this is
            // a coverage floor, not a pass counter.
            checked += 1;
            if expected != actual {
                let first_div = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(e, a)| e != a)
                    .unwrap_or(expected.len().min(actual.len()));
                let lo = first_div.saturating_sub(3);
                failures.push(format!(
                    "{}: first divergence at event #{first_div} \
                     (expected len {}, actual len {})\n  expected: {:?}\n  actual:   {:?}",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                    &expected[lo..expected.len().min(first_div + 4)],
                    &actual[lo..actual.len().min(first_div + 4)],
                ));
            }
        }
        assert!(
            checked > 50,
            "expected to check a substantial number of org fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} org fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `StreamingParser` fed an org fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
///
/// org-fmt's `batch.rs` module docs sanction exactly two behavioural
/// exceptions — loose lists emitted as separate single-item lists, and
/// drawers containing blank lines being split. Three previously-unknown bugs
/// outside those exceptions used to make this fail on 3 of 89 fixtures
/// (nested `#+BEGIN_QUOTE` closing early, an affiliated `#+NAME:` line
/// dropped from its block, an indented list-item code block misread as
/// top-level) — see the `streaming_harness::CAPABILITIES` "org" entry's
/// `streaming_parser` doc comment for the fix. Now passes over all 89.
#[test]
fn org_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("org");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/org dir") {
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
        let bulk: Vec<org_fmt::OwnedEvent> = org_fmt::events(input_str)
            .map(org_fmt::Event::into_owned)
            .collect();
        // Coverage floor, not a pass counter — see the rst equivalent.
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                org_fmt::StreamingParser::new(|e: org_fmt::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            if bulk != streamed {
                if result.is_ok() {
                    result = Err(format!(
                        "StreamingParser diverged from events() for fixture {name} under \
                         chunking {chunking_name}:\n  events():         {bulk:?}\n  \
                         StreamingParser: {streamed:?}"
                    ));
                }
                break;
            }
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of org fixtures, got {checked}"
    );
    assert_or_known_failure("org", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// builder `build()` produces for the AST `parse(input)` returned.
///
/// `org_fmt::writer::Writer` writes straight through to a single shared
/// output buffer per event (mirroring `rst-fmt`'s `Writer` design — see
/// `crates/formats/org-fmt/src/writer.rs`'s module doc), not a
/// buffer-then-reconstruct-the-AST fake streaming writer. This checks
/// content is byte-identical to `build()` over all fixtures, and that bytes
/// reach the sink before `finish()`.
#[test]
fn org_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("org");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/org dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = org_fmt::parse(&input);
        let built = org_fmt::build(&doc);

        let mut w = org_fmt::Writer::new(Vec::<u8>::new());
        for e in org_fmt::events(&input) {
            w.write_event(e.into_owned());
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {}:\n  build():  \
                 {built:?}\n  Writer(): {streamed:?}",
                path.display()
            ));
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of org fixtures, got {checked}"
    );

    // Incrementality probe: a byte-identical final result (checked above)
    // only proves the *content* is right, not that the writer is genuinely
    // streaming. Feed several complete events (well short of finish()) and
    // check whether any bytes have already reached the sink.
    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = org_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(org_fmt::OwnedEvent::StartHeading {
            level: 1,
            todo: None,
            priority: None,
            tags: vec![],
            properties: vec![],
            scheduled: None,
            deadline: None,
        });
        w.write_event(org_fmt::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(org_fmt::OwnedEvent::EndHeading);
        w.write_event(org_fmt::OwnedEvent::StartParagraph);
        w.write_event(org_fmt::OwnedEvent::Text("World".to_string().into()));
        w.write_event(org_fmt::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — org_fmt::writer::Writer is not \
                 a genuine incremental streaming writer despite content round-tripping correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("org", "streaming_writer", result);
}
