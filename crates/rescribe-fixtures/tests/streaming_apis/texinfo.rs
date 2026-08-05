//! Streaming-API cross-checks for texinfo. Split out of the former monolithic
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
// texinfo: events() vs parse(), real and passing (including Event::Title for
// @settitle — see crates/formats/texinfo/src/events.rs). StreamingParser and
// the streaming Writer are both buffer-until-finish wrappers (see their own
// module docs in crates/formats/texinfo/src/batch.rs and writer.rs, which
// self-report "Memory usage is O(full input)" / "buffers all events,
// reconstructs the AST, then emits") rather than genuine incremental
// implementations; tracked as KnownFailure entries. (The @settitle-dropping
// bug that used to additionally afflict the streaming writer is fixed:
// Event::Title now carries the title through events()/StreamingParser/
// Writer; only the buffer-until-finish incrementality gap remains open.)
// ---------------------------------------------------------------------------

fn texinfo_ast_to_events(doc: &texinfo::TexinfoDoc) -> Vec<texinfo::events::Event<'static>> {
    let mut out = Vec::new();
    if let Some(title) = &doc.title {
        out.push(texinfo::events::Event::Title(title.clone()));
    }
    for b in &doc.blocks {
        texinfo_block_events(b, &mut out);
    }
    out
}

fn texinfo_block_events(b: &texinfo::Block, out: &mut Vec<texinfo::events::Event<'static>>) {
    use std::borrow::Cow;
    use texinfo::Block;
    use texinfo::events::Event;
    match b {
        Block::Heading {
            level,
            kind,
            inlines,
            ..
        } => {
            out.push(Event::StartHeading {
                level: *level,
                kind: kind.clone(),
            });
            for i in inlines {
                texinfo_inline_events(i, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                texinfo_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock {
            variant, content, ..
        } => {
            out.push(Event::CodeBlock {
                variant: variant.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for c in children {
                texinfo_block_events(c, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for i in item {
                    texinfo_inline_events(i, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for (term, desc) in items {
                out.push(Event::StartDefinitionTerm);
                for i in term {
                    texinfo_inline_events(i, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for db in desc {
                    texinfo_block_events(db, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow {
                    is_header: row.is_header,
                });
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    for i in cell {
                        texinfo_inline_events(i, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::Menu { entries, .. } => {
            out.push(Event::StartMenu);
            for e in entries {
                out.push(Event::MenuEntry {
                    node: e.node.clone(),
                    description: e.description.clone(),
                });
            }
            out.push(Event::EndMenu);
        }
        Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
        Block::RawBlock {
            environment,
            content,
            ..
        } => out.push(Event::RawBlock {
            environment: environment.clone(),
            content: content.clone(),
        }),
        Block::Float {
            float_type,
            label,
            children,
            ..
        } => {
            out.push(Event::StartFloat {
                float_type: float_type.clone(),
                label: label.clone(),
            });
            for c in children {
                texinfo_block_events(c, out);
            }
            out.push(Event::EndFloat);
        }
        Block::NoIndent { .. } => out.push(Event::NoIndent),
    }
}

fn texinfo_inline_events(i: &texinfo::Inline, out: &mut Vec<texinfo::events::Event<'static>>) {
    use std::borrow::Cow;
    use texinfo::Inline;
    use texinfo::events::Event;
    match i {
        Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
        Inline::Strong(c, _) => {
            out.push(Event::StartStrong);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndStrong);
        }
        Inline::Emphasis(c, _) => {
            out.push(Event::StartEmphasis);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndEmphasis);
        }
        Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
        Inline::Var(c, _) => {
            out.push(Event::StartVar);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndVar);
        }
        Inline::File(s, _) => out.push(Event::File(Cow::Owned(s.clone()))),
        Inline::Command(s, _) => out.push(Event::Command(Cow::Owned(s.clone()))),
        Inline::Option(s, _) => out.push(Event::Option(Cow::Owned(s.clone()))),
        Inline::Env(s, _) => out.push(Event::Env(Cow::Owned(s.clone()))),
        Inline::Samp(s, _) => out.push(Event::Samp(Cow::Owned(s.clone()))),
        Inline::Kbd(s, _) => out.push(Event::Kbd(Cow::Owned(s.clone()))),
        Inline::Key(s, _) => out.push(Event::Key(Cow::Owned(s.clone()))),
        Inline::Dfn(c, _) => {
            out.push(Event::StartDfn);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndDfn);
        }
        Inline::Cite(s, _) => out.push(Event::Cite(Cow::Owned(s.clone()))),
        Inline::Acronym {
            abbrev, expansion, ..
        } => out.push(Event::Acronym {
            abbrev: abbrev.clone(),
            expansion: expansion.clone(),
        }),
        Inline::Abbr {
            abbrev, expansion, ..
        } => out.push(Event::Abbr {
            abbrev: abbrev.clone(),
            expansion: expansion.clone(),
        }),
        Inline::Roman(s, _) => out.push(Event::Roman(Cow::Owned(s.clone()))),
        Inline::SmallCaps(s, _) => out.push(Event::SmallCaps(Cow::Owned(s.clone()))),
        Inline::DirectItalic(c, _) => {
            out.push(Event::StartDirectItalic);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndDirectItalic);
        }
        Inline::DirectBold(c, _) => {
            out.push(Event::StartDirectBold);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndDirectBold);
        }
        Inline::DirectTypewriter(s, _) => {
            out.push(Event::DirectTypewriter(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            for x in children {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image {
            file,
            width,
            height,
            alt,
            extension,
            ..
        } => out.push(Event::Image {
            file: file.clone(),
            width: width.clone(),
            height: height.clone(),
            alt: alt.clone(),
            extension: extension.clone(),
        }),
        Inline::Superscript(c, _) => {
            out.push(Event::StartSuperscript);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(c, _) => {
            out.push(Event::StartSubscript);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndSubscript);
        }
        Inline::LineBreak { .. } => out.push(Event::LineBreak),
        Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
        Inline::FootnoteDef { content, .. } => {
            out.push(Event::StartFootnoteDef);
            for x in content {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndFootnoteDef);
        }
        Inline::CrossRef {
            kind, node, text, ..
        } => out.push(Event::CrossRef {
            kind: kind.clone(),
            node: node.clone(),
            text: text.clone(),
        }),
        Inline::Anchor { name, .. } => out.push(Event::Anchor { name: name.clone() }),
        Inline::NoBreak(s, _) => out.push(Event::NoBreak(Cow::Owned(s.clone()))),
        Inline::Email { address, text, .. } => out.push(Event::Email {
            address: address.clone(),
            text: text.clone(),
        }),
        Inline::Symbol(kind, _) => out.push(Event::Symbol(kind.clone())),
    }
}

#[test]
fn texinfo_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("texinfo");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/texinfo dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = texinfo::parse(&input);
        let expected = texinfo_ast_to_events(&doc);
        let actual: Vec<_> = texinfo::events::events(&input).collect();
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
        "expected to check a substantial number of texinfo fixtures, got {checked}"
    );
}

/// `StreamingParser` now processes input in logical top-level units (a
/// paragraph, heading, or `@directive ... @end directive` environment —
/// see `crates/formats/texinfo/src/batch.rs`'s module docs), flushing each
/// unit to the handler as soon as its boundary is confirmed, rather than
/// buffering the whole document until `finish()`. This check verifies the
/// final event sequence still matches `events()` under adversarial chunking
/// over the whole fixture suite.
///
/// The separate "does `feed()` alone deliver anything before `finish()`"
/// incrementality probe that used to live in this function was removed: it
/// asserted non-empty delivery after feeding half of *every* fixture's
/// bytes, which is not actually a valid general property — a fixture that
/// is a single short paragraph (no blank line, no directive) legitimately
/// delivers nothing until its one unit completes, same as `rst-fmt` and
/// `org-fmt`'s equivalent per-block streaming parsers. That probe is
/// replaced by `texinfo_streaming_parser_delivers_events_incrementally`
/// below, a deterministic synthetic-input check (not fixture-dependent, same
/// pattern as the crate's own `test_streaming_parser_delivers_before_finish`
/// unit test) that feeds a *complete* leading unit and checks it is
/// delivered before `finish()`.
#[test]
fn texinfo_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("texinfo");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/texinfo dir") {
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
        let bulk: Vec<texinfo::OwnedEvent> = texinfo::events::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                texinfo::StreamingParser::new(|e: texinfo::OwnedEvent| streamed.push(e));
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
        checked > 5,
        "expected to check several texinfo fixtures, got {checked}"
    );
    assert_or_known_failure("texinfo", "streaming_parser", result);
}

/// Deterministic incrementality probe: feeding a complete leading unit (a
/// heading followed by its terminating blank line) must deliver its events
/// to the handler before `finish()` is ever called, and before the second,
/// much larger paragraph that follows is fed at all. This proves `feed()`
/// advances real per-unit parser state rather than buffering the whole
/// input — the defect the pre-fix `texinfo::batch::StreamingParser` had
/// (buffering into a `Vec<u8>` and only calling `events()` inside
/// `finish()`).
#[test]
fn texinfo_streaming_parser_delivers_events_incrementally() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let delivered: Rc<RefCell<Vec<texinfo::OwnedEvent>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&delivered);
    let mut parser = texinfo::StreamingParser::new(move |e| sink.borrow_mut().push(e));

    parser.feed(b"@chapter Hello\n\n");
    if let Err(e) =
        assert_streaming_parser_is_incremental("texinfo", !delivered.borrow().is_empty())
    {
        panic!("{e}");
    }
    assert!(
        delivered
            .borrow()
            .iter()
            .any(|e| matches!(e, texinfo::OwnedEvent::StartHeading { .. }))
    );

    // A second, much larger unit fed afterward — but not yet completed by a
    // terminating blank line — must not retroactively change what was
    // already delivered for the first.
    let before = delivered.borrow().len();
    parser.feed(&"word ".repeat(10_000).into_bytes());
    assert_eq!(
        delivered.borrow().len(),
        before,
        "feeding more bytes of a still-incomplete second paragraph must not change what was \
         already delivered for the completed first unit"
    );
    parser.feed(b"\n\n");
    parser.finish();
    assert!(delivered.borrow().len() > before);
}

/// `Writer` writes straight through to a single shared output buffer per
/// event (mirroring `rst-fmt`'s `Writer` design — see
/// `crates/formats/texinfo/src/writer.rs`'s module doc), not a
/// buffer-then-reconstruct-the-AST fake streaming writer. This checks
/// content is byte-identical to `emit()` over all fixtures, including
/// `@settitle` (carried via `Event::Title`), and that bytes reach the sink
/// before `finish()` is called.
#[test]
fn texinfo_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("texinfo");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/texinfo dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = texinfo::parse(&input);
        let built = texinfo::emit(&doc);

        let mut w = texinfo::Writer::new(Vec::<u8>::new());
        for e in texinfo::events::events(&input) {
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
        "expected to check a substantial number of texinfo fixtures, got {checked}"
    );

    // Incrementality probe: a byte-identical final result (checked above)
    // only proves the *content* is right, not that the writer is genuinely
    // streaming. Feed several complete events (well short of finish()) and
    // check whether any bytes have already reached the sink.
    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = texinfo::Writer::new(ObservableSink(observed.clone()));
        w.write_event(texinfo::OwnedEvent::StartHeading {
            level: 1,
            kind: texinfo::HeadingKind::Numbered,
        });
        w.write_event(texinfo::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(texinfo::OwnedEvent::EndHeading);
        w.write_event(texinfo::OwnedEvent::StartParagraph);
        w.write_event(texinfo::OwnedEvent::Text("World".to_string().into()));
        w.write_event(texinfo::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — texinfo::writer::Writer \
                 buffers all events into a Vec<OwnedEvent> and only reconstructs the AST + \
                 calls emit() inside finish() (crates/formats/texinfo/src/writer.rs), so it is \
                 not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("texinfo", "streaming_writer", result);
}
