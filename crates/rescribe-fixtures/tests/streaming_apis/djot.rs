//! Streaming-API cross-checks for djot. Split out of the former monolithic
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
// djot-fmt: events() vs parse(), fully wired
// ---------------------------------------------------------------------------
//
// This check carries real signal for djot-fmt specifically, because the two
// paths are genuinely independent implementations: `parse()` is direct
// recursive descent (`parse_blocks_direct`/`parse_next_block_direct` in
// parse.rs), while `events()` is a line-driven frame-stack state machine
// (`EventIter::next`/`push_next_block_frames`, with `Frame::SubParser` for
// compound-block content). Neither is derived from the other.
//
// Note the doc comment on `events()` in djot-fmt/src/lib.rs claims it "parses
// the input first, then walks the AST yielding owned events" — that is stale
// and describes the hollow pattern CLAUDE.md rejects, not what the code does.
// `EventIter::next` pulls one top-level block at a time straight off the
// source lines. The doc comment should be corrected; tracked in TODO.md.
mod djot_events_check {
    use super::{find_input, fixtures_root};
    use djot_fmt::{Attr, Block, DjotDoc, Inline, OwnedEvent};
    use std::path::PathBuf;

    /// Unpack djot's `Attr` struct into the flattened `(id, classes, kv)` triple
    /// every attribute-carrying `Event` variant uses.
    fn dj_unpack(attr: &Attr) -> (Option<String>, Vec<String>, Vec<(String, String)>) {
        (attr.id.clone(), attr.classes.clone(), attr.kv.clone())
    }

    /// The exact event sequence `djot_fmt::events()` must produce for `doc`.
    ///
    /// CONTRACT (document level): `DjotDoc` carries `blocks`, `footnotes` and
    /// `link_defs` side by side, and the type definitions alone do not say where
    /// the latter two land in the stream. Resolved against `EventIter::next`'s
    /// `None` arm: once top-level blocks are exhausted, `link_defs` are pushed
    /// first as `Event::LinkDef` (one per entry, document order), then footnote
    /// defs as `StartFootnoteDef`/blocks/`EndFootnoteDef` — i.e. link defs land
    /// right after the body and footnotes trail everything.
    fn dj_ast_to_events(doc: &DjotDoc) -> Vec<OwnedEvent> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            dj_block_events(b, &mut out);
        }
        for ld in &doc.link_defs {
            let (id, classes, kv) = dj_unpack(&ld.attr);
            out.push(OwnedEvent::LinkDef {
                label: ld.label.clone(),
                url: ld.url.clone(),
                title: ld.title.clone(),
                id,
                classes,
                kv,
            });
        }
        for f in &doc.footnotes {
            out.push(OwnedEvent::StartFootnoteDef {
                label: f.label.clone(),
            });
            for b in &f.blocks {
                dj_block_events(b, &mut out);
            }
            out.push(OwnedEvent::EndFootnoteDef);
        }
        out
    }

    fn dj_block_events(b: &Block, out: &mut Vec<OwnedEvent>) {
        match b {
            Block::Paragraph { inlines, attr, .. } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartParagraph { id, classes, kv });
                dj_inline_events(inlines, out);
                out.push(OwnedEvent::EndParagraph);
            }
            Block::Heading {
                level,
                inlines,
                attr,
                ..
            } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartHeading {
                    level: *level,
                    id,
                    classes,
                    kv,
                });
                dj_inline_events(inlines, out);
                out.push(OwnedEvent::EndHeading);
            }
            Block::Blockquote { blocks, attr, .. } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartBlockquote { id, classes, kv });
                for c in blocks {
                    dj_block_events(c, out);
                }
                out.push(OwnedEvent::EndBlockquote);
            }
            Block::List {
                kind,
                items,
                tight,
                attr,
                ..
            } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartList {
                    kind: kind.clone(),
                    tight: *tight,
                    id,
                    classes,
                    kv,
                });
                for item in items {
                    out.push(OwnedEvent::StartListItem {
                        checked: item.checked,
                    });
                    for c in &item.blocks {
                        dj_block_events(c, out);
                    }
                    out.push(OwnedEvent::EndListItem);
                }
                out.push(OwnedEvent::EndList);
            }
            Block::CodeBlock {
                language,
                content,
                attr,
                ..
            } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartCodeBlock {
                    language: language.clone(),
                    id,
                    classes,
                    kv,
                });
                // CONTRACT: `CodeBlockContent` is emitted unconditionally, even for
                // an empty body — `handle_event`'s `StartCodeBlock` arm seeds the
                // frame with `String::new()` and only `CodeBlockContent` ever writes
                // it, so an omitted event and an empty one are indistinguishable to
                // the tree builder. Confirmed unconditional in `expand_block_frames`.
                out.push(OwnedEvent::CodeBlockContent(content.clone().into()));
                out.push(OwnedEvent::EndCodeBlock);
            }
            Block::RawBlock {
                format, content, ..
            } => {
                // `Event::RawBlock` has no attribute fields, so `Block::RawBlock`'s
                // `attr` has no representation in the stream. That is a lossy point
                // in the Event type itself, not a projection choice.
                out.push(OwnedEvent::RawBlock {
                    format: format.clone(),
                    content: content.clone(),
                });
            }
            Block::Div {
                class,
                blocks,
                attr,
                ..
            } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartDiv {
                    class: class.clone(),
                    id,
                    classes,
                    kv,
                });
                for c in blocks {
                    dj_block_events(c, out);
                }
                out.push(OwnedEvent::EndDiv);
            }
            Block::Table { caption, rows, .. } => {
                // CONTRACT: the caption is carried as a single `TableCaption(Vec<Inline>)`
                // event that *precedes* `StartTable`, not as inline events inside the
                // table. Pinned by `handle_event`: `TableCaption` pushes a
                // `TablePendingCaption` frame which the following `StartTable` pops.
                if let Some(cap) = caption {
                    out.push(OwnedEvent::TableCaption(cap.clone()));
                }
                out.push(OwnedEvent::StartTable);
                for row in rows {
                    out.push(OwnedEvent::StartTableRow {
                        is_header: row.is_header,
                    });
                    for cell in &row.cells {
                        out.push(OwnedEvent::StartTableCell {
                            alignment: cell.alignment.clone(),
                        });
                        dj_inline_events(&cell.inlines, out);
                        out.push(OwnedEvent::EndTableCell);
                    }
                    out.push(OwnedEvent::EndTableRow);
                }
                out.push(OwnedEvent::EndTable);
            }
            Block::ThematicBreak { attr, .. } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::ThematicBreak { id, classes, kv });
            }
            Block::DefinitionList { items, attr, .. } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartDefinitionList { id, classes, kv });
                for item in items {
                    out.push(OwnedEvent::StartDefinitionTerm);
                    dj_inline_events(&item.term, out);
                    out.push(OwnedEvent::EndDefinitionTerm);
                    out.push(OwnedEvent::StartDefinitionDesc);
                    // `DefItem::definitions` is `Vec<Block>` (unlike rst's inline
                    // desc), so the desc body is a block sequence.
                    for c in &item.definitions {
                        dj_block_events(c, out);
                    }
                    out.push(OwnedEvent::EndDefinitionDesc);
                }
                out.push(OwnedEvent::EndDefinitionList);
            }
        }
    }

    fn dj_inline_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
        for i in inlines {
            match i {
                Inline::Text { content, .. } => out.push(OwnedEvent::Text(content.clone().into())),
                Inline::SoftBreak { .. } => out.push(OwnedEvent::SoftBreak),
                Inline::HardBreak { .. } => out.push(OwnedEvent::HardBreak),
                Inline::Emphasis { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartEmphasis { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndEmphasis);
                }
                Inline::Strong { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartStrong { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndStrong);
                }
                Inline::Delete { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartDelete { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndDelete);
                }
                Inline::Insert { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartInsert { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndInsert);
                }
                Inline::Highlight { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartHighlight { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndHighlight);
                }
                Inline::Subscript { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartSubscript { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndSubscript);
                }
                Inline::Superscript { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartSuperscript { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndSuperscript);
                }
                Inline::Verbatim { content, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::Verbatim {
                        content: content.clone().into(),
                        id,
                        classes,
                        kv,
                    });
                }
                Inline::MathInline { content, .. } => {
                    out.push(OwnedEvent::MathInline(content.clone().into()))
                }
                Inline::MathDisplay { content, .. } => {
                    out.push(OwnedEvent::MathDisplay(content.clone().into()))
                }
                Inline::RawInline {
                    format, content, ..
                } => out.push(OwnedEvent::RawInline {
                    format: format.clone(),
                    content: content.clone(),
                }),
                Inline::Link {
                    inlines,
                    url,
                    title,
                    attr,
                    ..
                } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartLink {
                        url: url.clone(),
                        title: title.clone(),
                        id,
                        classes,
                        kv,
                    });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndLink);
                }
                Inline::Image {
                    inlines,
                    url,
                    title,
                    attr,
                    ..
                } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    // Unlike rst's leaf `InlineImage`, djot's image is a container
                    // pair: the alt text is the child inline sequence.
                    out.push(OwnedEvent::StartImage {
                        url: url.clone(),
                        title: title.clone(),
                        id,
                        classes,
                        kv,
                    });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndImage);
                }
                Inline::Span { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartSpan { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndSpan);
                }
                Inline::FootnoteRef { label, .. } => {
                    out.push(OwnedEvent::FootnoteRef(label.clone()))
                }
                Inline::Symbol { name, .. } => out.push(OwnedEvent::Symbol(name.clone())),
                Inline::Autolink { url, is_email, .. } => out.push(OwnedEvent::Autolink {
                    url: url.clone(),
                    is_email: *is_email,
                }),
            }
        }
    }

    /// `events()` must equal the hand-written AST projection exactly, over
    /// every fixture in `fixtures/djot/`.
    #[test]
    fn djot_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("djot");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
            .expect("fixtures/djot dir")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();

        for path in dirs {
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            // `parse()` is infallible and returns diagnostics alongside the doc;
            // diagnostics are not this check's concern.
            let (doc, _diags) = djot_fmt::parse(&input);
            let expected = dj_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> =
                djot_fmt::events(&input).map(|e| e.into_owned()).collect();
            checked += 1;

            if expected != actual {
                let at = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual.len()));
                let lo = at.saturating_sub(2);
                failures.push(format!(
                    "{}: first divergence at event #{at} (expected {} events, got {})\n  \
                     expected[{lo}..]: {:?}\n  actual[{lo}..]:   {:?}",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                    &expected[lo..expected.len().min(at + 4)],
                    &actual[lo..actual.len().min(at + 4)],
                ));
            }
        }

        assert!(
            checked > 50,
            "expected to check a substantial number of djot fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} djot fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}
/// `StreamingParser` fed a djot fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
///
/// No sanctioned exception applies: `batch.rs`'s doc comments make only a
/// memory claim (`O(largest block)`) plus a nesting claim that one of the
/// bugs below actually violates, and CLAUDE.md names commonmark-fmt as the
/// only streaming exemption.
#[test]
fn djot_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("djot");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/djot dir") {
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
        let bulk: Vec<djot_fmt::OwnedEvent> = djot_fmt::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        // Coverage floor, not a pass counter — see the rst equivalent.
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                djot_fmt::StreamingParser::new(|e: djot_fmt::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of djot fixtures, got {checked}"
    );
    assert_or_known_failure("djot", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// builder `emit()` produces for the AST `parse(input)` returned.
///
/// `djot_fmt::writer::Writer` writes straight through to a single shared
/// output buffer per event (mirroring `rst-fmt`'s `Writer` design — see
/// `crates/formats/djot-fmt/src/writer.rs`'s module doc), not a
/// buffer-then-reconstruct-the-AST fake streaming writer. This checks
/// content is byte-identical to `emit()` over all fixtures, including
/// link-reference definitions (`Event::LinkDef`) and table captions
/// (`Event::TableCaption`), and that bytes reach the sink before `finish()`.
#[test]
fn djot_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("djot");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/djot dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = djot_fmt::parse(&input);
        let built = djot_fmt::emit(&doc);

        let mut w = djot_fmt::Writer::new(Vec::<u8>::new());
        for e in djot_fmt::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from emit() for fixture {}:\n  emit():   \
                 {built:?}\n  Writer(): {streamed:?}",
                path.display()
            ));
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of djot fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves the *content* is right, not that the writer is genuinely
    // streaming. Feed a complete heading + paragraph (well short of
    // finish()) and check whether any bytes have already reached the sink.
    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = djot_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(djot_fmt::OwnedEvent::StartHeading {
            level: 1,
            id: None,
            classes: vec![],
            kv: vec![],
        });
        w.write_event(djot_fmt::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(djot_fmt::OwnedEvent::EndHeading);
        w.write_event(djot_fmt::OwnedEvent::StartParagraph {
            id: None,
            classes: vec![],
            kv: vec![],
        });
        w.write_event(djot_fmt::OwnedEvent::Text("World".to_string().into()));
        w.write_event(djot_fmt::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — djot_fmt::writer::Writer \
                 buffers all events into a Vec<OwnedEvent> and only reconstructs the AST + \
                 calls emit() inside finish() (crates/formats/djot-fmt/src/writer.rs), so it \
                 is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("djot", "streaming_writer", result);
}
