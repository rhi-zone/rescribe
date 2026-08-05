//! Streaming-API cross-checks for mediawiki. Split out of the former monolithic
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
// mediawiki-fmt: events() vs parse(), StreamingParser vs events(), streaming
// writer vs build() -- all fully wired
// ---------------------------------------------------------------------------
//
// mediawiki-fmt's `events()` (`EventIter::new`, events.rs) is architecturally
// parse()-then-walk: it calls `crate::parse::parse(input)` and then walks the
// resulting tree with `emit_doc_events`/`emit_block_events`/
// `emit_inline_events`. Unlike html-fmt's `events_from_doc` (a generic,
// structure-free depth-first walk over an html5ever DOM -- see the `html`
// `CAPABILITIES` entry), mediawiki-fmt's walk makes real per-variant semantic
// decisions (one arm per `Block`/`Inline` variant, e.g. `Inline::Link`
// unpacks into `StartLink`/`Text`/`EndLink`), so an independently-derived
// projection from the AST can and does diverge from the walk when the walk
// has a real mapping bug -- it is not guaranteed to pass by construction the
// way html's would be. This mirrors asciidoc's narrower-than-rst "Wired"
// claim (see the comment above `asciidoc_events_check`): it validates the
// AST->event projection layer, not two independent parsers, because both
// `events()` and this check's `mw_ast_to_events` start from the same
// `parse()` output.
mod mediawiki_events_check {
    use super::{assert_or_known_failure, find_input, fixtures_root};
    use mediawiki_fmt::ast::{Block, Inline, MediawikiDoc};
    use mediawiki_fmt::events::OwnedEvent;
    use std::borrow::Cow;
    type Event = OwnedEvent;

    fn mw_ast_to_events(doc: &MediawikiDoc) -> Vec<Event> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            mw_block_events(b, &mut out);
        }
        out
    }

    fn mw_block_events(b: &Block, out: &mut Vec<Event>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                mw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                mw_inline_events(inlines, out);
                out.push(Event::EndHeading);
            }
            Block::CodeBlock {
                language, content, ..
            } => {
                out.push(Event::CodeBlock {
                    language: language.clone(),
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item_blocks in items {
                    out.push(Event::StartListItem);
                    for b in item_blocks {
                        mw_block_events(b, out);
                    }
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::DefinitionList { items, .. } => {
                out.push(Event::StartDefinitionList);
                for item in items {
                    out.push(Event::StartDefinitionTerm);
                    mw_inline_events(&item.term, out);
                    out.push(Event::EndDefinitionTerm);
                    out.push(Event::StartDefinitionDesc);
                    mw_inline_events(&item.desc, out);
                    out.push(Event::EndDefinitionDesc);
                }
                out.push(Event::EndDefinitionList);
            }
            Block::HorizontalRule => out.push(Event::HorizontalRule),
            Block::Table { rows, caption, .. } => {
                out.push(Event::StartTable {
                    caption: caption.clone(),
                });
                for row in rows {
                    out.push(Event::StartTableRow);
                    for cell in &row.cells {
                        out.push(Event::StartTableCell {
                            is_header: cell.is_header,
                        });
                        mw_inline_events(&cell.inlines, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::Blockquote { children, .. } => {
                out.push(Event::StartBlockquote);
                for child in children {
                    mw_block_events(child, out);
                }
                out.push(Event::EndBlockquote);
            }
            Block::PreBlock { content, .. } => {
                out.push(Event::PreBlock {
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::RawBlock { content, .. } => {
                out.push(Event::RawBlock {
                    content: Cow::Owned(content.clone()),
                });
            }
        }
    }

    fn mw_inline_events(inlines: &[Inline], out: &mut Vec<Event>) {
        for i in inlines {
            match i {
                Inline::Text(s) => out.push(Event::Text(Cow::Owned(s.clone()))),
                Inline::Bold(children) => {
                    out.push(Event::StartBold);
                    mw_inline_events(children, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(children) => {
                    out.push(Event::StartItalic);
                    mw_inline_events(children, out);
                    out.push(Event::EndItalic);
                }
                Inline::Code(s) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, text } => {
                    out.push(Event::StartLink { url: url.clone() });
                    out.push(Event::Text(Cow::Owned(text.clone())));
                    out.push(Event::EndLink);
                }
                Inline::Image { url, alt } => out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                }),
                Inline::LineBreak => out.push(Event::LineBreak),
                Inline::Strikeout(children) => {
                    out.push(Event::StartStrikethrough);
                    mw_inline_events(children, out);
                    out.push(Event::EndStrikethrough);
                }
                Inline::Underline(children) => {
                    out.push(Event::StartUnderline);
                    mw_inline_events(children, out);
                    out.push(Event::EndUnderline);
                }
                Inline::Subscript(children) => {
                    out.push(Event::StartSubscript);
                    mw_inline_events(children, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Superscript(children) => {
                    out.push(Event::StartSuperscript);
                    mw_inline_events(children, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::FootnoteRef { label, content } => out.push(Event::FootnoteRef {
                    label: label.clone(),
                    content: content.clone(),
                }),
                Inline::MathInline { source } => out.push(Event::MathInline {
                    source: source.clone(),
                }),
                Inline::Template { content } => out.push(Event::Template {
                    content: content.clone(),
                }),
                Inline::Nowiki { content } => out.push(Event::Nowiki {
                    content: content.clone(),
                }),
            }
        }
    }

    #[test]
    fn mediawiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("mediawiki");
        let mut checked = 0;
        let mut result: Result<(), String> = Ok(());
        for entry in std::fs::read_dir(&root).expect("fixtures/mediawiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = mediawiki_fmt::parse::parse_str(&input);
            let expected = mw_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = mediawiki_fmt::events(&input)
                .map(|e| e.into_owned())
                .collect();
            checked += 1;
            if expected != actual && result.is_ok() {
                result = Err(format!(
                    "events() diverged from the AST projection for fixture {name}:\n  \
                     ast-derived: {expected:?}\n  events():    {actual:?}"
                ));
            }
        }
        assert!(
            checked > 20,
            "expected to check a substantial number of mediawiki fixtures, got {checked}"
        );
        assert_or_known_failure("mediawiki", "events", result);
    }
}

/// `StreamingParser` fed a mediawiki fixture under an adversarial chunking
/// must deliver the same event sequence `events()` delivers over the whole
/// input.
///
/// `mediawiki_fmt::batch::StreamingParser::emit_block` re-parses each
/// accumulated block in isolation via `crate::events::events(&text)`
/// (batch.rs) -- the same "re-parse each block" architecture already found to
/// split cross-block constructs for rst/org/asciidoc's `StreamingParser`s.
#[test]
fn mediawiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("mediawiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/mediawiki dir") {
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
        let bulk: Vec<mediawiki_fmt::OwnedEvent> = mediawiki_fmt::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = mediawiki_fmt::StreamingParser::new(|e: mediawiki_fmt::OwnedEvent| {
                streamed.push(e)
            });
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
        checked > 20,
        "expected to check a substantial number of mediawiki fixtures, got {checked}"
    );
    assert_or_known_failure("mediawiki", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// `emit()` produces for the AST `parse(input)` returned. Also probes for
/// genuine incrementality: `Writer::write_event` (writer.rs) only pushes onto
/// an internal `Vec<OwnedEvent>`; `finish()` reconstructs the AST via
/// `events_to_doc` and calls `crate::emit::emit` -- a buffer-then-emit
/// architecture, not incremental streaming, per CLAUDE.md.
#[test]
fn mediawiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("mediawiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/mediawiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = mediawiki_fmt::parse_str(&input);
        let built =
            String::from_utf8(mediawiki_fmt::Emit::emit(&doc)).expect("emit() output is UTF-8");

        let mut w = mediawiki_fmt::Writer::new(Vec::<u8>::new());
        for e in mediawiki_fmt::events(&input) {
            w.write_event(e.into_owned());
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from emit() for fixture {}:\n  emit():  {built:?}\n  \
                 Writer(): {streamed:?}",
                path.display()
            ));
        }
    }
    assert!(
        checked > 20,
        "expected to check a substantial number of mediawiki fixtures, got {checked}"
    );

    if result.is_ok() {
        use mediawiki_fmt::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = mediawiki_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/Text/EndParagraph sequence and before finish() -- \
                 mediawiki_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and \
                 only reconstructs the AST + calls emit() inside finish(), so it is not a \
                 genuine incremental streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("mediawiki", "streaming_writer", result);
}
