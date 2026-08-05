//! Streaming-API cross-checks for twiki. Split out of the former monolithic
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
// twiki: events() vs parse(), StreamingParser vs events(), streaming
// writer vs build() -- all fully wired
// ---------------------------------------------------------------------------
//
// twiki's `events()` (`twiki::events::events`) has a narrower signature than
// every other format checked in this file: `fn events(doc: &TwikiDoc) ->
// EventIter<'_>` takes an already-parsed AST, not raw input -- a caller must
// call `parse()` first. This is a real deviation from the vertical
// completion checklist's `events(input: &[u8]) -> impl Iterator<Item =
// Event>` contract (CLAUDE.md), tracked as a follow-up in TODO.md, not fixed
// here. It does not block wiring this check: `EventIter::new(doc)` still
// walks the tree with `emit_block`/`emit_inlines`, making real per-variant
// mapping decisions, so an independently-derived projection can and does
// diverge from the walk on a genuine bug (same narrower-Wired-claim caveat
// as mediawiki-fmt/tikiwiki above).
mod twiki_events_check {
    use super::{assert_or_known_failure, find_input, fixtures_root};
    use std::borrow::Cow;
    use twiki::ast::{Block, Inline, TwikiDoc};
    use twiki::events::OwnedEvent;
    type Event = OwnedEvent;

    fn tw_ast_to_events(doc: &TwikiDoc) -> Vec<Event> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            tw_block_events(b, &mut out);
        }
        out
    }

    fn tw_block_events(b: &Block, out: &mut Vec<Event>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                tw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                tw_inline_events(inlines, out);
                out.push(Event::EndHeading);
            }
            Block::CodeBlock { content, .. } => {
                out.push(Event::CodeBlock {
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item in items {
                    out.push(Event::StartListItem);
                    tw_inline_events(&item.inlines, out);
                    for child in &item.children {
                        tw_block_events(child, out);
                    }
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(Event::StartTable);
                for row in rows {
                    out.push(Event::StartTableRow);
                    for cell in &row.cells {
                        out.push(Event::StartTableCell {
                            is_header: cell.is_header,
                        });
                        tw_inline_events(&cell.inlines, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
            Block::RawBlock { content, .. } => {
                out.push(Event::RawBlock {
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::DefinitionList { items, .. } => {
                out.push(Event::StartDefinitionList);
                for item in items {
                    out.push(Event::StartDefinitionTerm);
                    tw_inline_events(&item.term, out);
                    out.push(Event::EndDefinitionTerm);
                    out.push(Event::StartDefinitionDesc);
                    tw_inline_events(&item.desc, out);
                    out.push(Event::EndDefinitionDesc);
                }
                out.push(Event::EndDefinitionList);
            }
            Block::Blockquote { children, .. } => {
                out.push(Event::StartBlockquote);
                for child in children {
                    tw_block_events(child, out);
                }
                out.push(Event::EndBlockquote);
            }
        }
    }

    fn tw_inline_events(inlines: &[Inline], out: &mut Vec<Event>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(Event::StartBold);
                    tw_inline_events(c, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(Event::StartItalic);
                    tw_inline_events(c, out);
                    out.push(Event::EndItalic);
                }
                Inline::BoldItalic(c, _) => {
                    out.push(Event::StartBoldItalic);
                    tw_inline_events(c, out);
                    out.push(Event::EndBoldItalic);
                }
                Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
                Inline::BoldCode(c, _) => {
                    out.push(Event::StartBoldCode);
                    tw_inline_events(c, out);
                    out.push(Event::EndBoldCode);
                }
                Inline::Link { url, label, .. } => {
                    out.push(Event::StartLink { url: url.clone() });
                    out.push(Event::Text(Cow::Owned(label.clone())));
                    out.push(Event::EndLink);
                }
                Inline::LineBreak { .. } => out.push(Event::LineBreak),
                Inline::Strikethrough(c, _) => {
                    out.push(Event::StartStrikethrough);
                    tw_inline_events(c, out);
                    out.push(Event::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(Event::StartSuperscript);
                    tw_inline_events(c, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(Event::StartSubscript);
                    tw_inline_events(c, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Underline(c, _) => {
                    out.push(Event::StartUnderline);
                    tw_inline_events(c, out);
                    out.push(Event::EndUnderline);
                }
                Inline::Image { url, alt, .. } => out.push(Event::Image {
                    url: url.clone(),
                    alt: alt.clone(),
                }),
                Inline::RawInline { content, .. } => out.push(Event::RawInline {
                    content: content.clone(),
                }),
                Inline::WikiWord { word, .. } => out.push(Event::WikiWord { word: word.clone() }),
            }
        }
    }

    #[test]
    fn twiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("twiki");
        let mut checked = 0;
        let mut result: Result<(), String> = Ok(());
        for entry in std::fs::read_dir(&root).expect("fixtures/twiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = twiki::parse::parse(&input);
            let expected = tw_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = twiki::events::events(&doc)
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
            checked > 15,
            "expected to check a substantial number of twiki fixtures, got {checked}"
        );
        assert_or_known_failure("twiki", "events", result);
    }
}

/// `StreamingParser` fed a twiki fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
/// `twiki::batch::StreamingParser::emit_block` re-parses each accumulated
/// block in isolation via `crate::parse::parse(&text)` followed by
/// `crate::events::events(&doc)`.
#[test]
fn twiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("twiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/twiki dir") {
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
        let (bulk_doc, _) = twiki::parse::parse(input_str);
        let bulk: Vec<twiki::OwnedEvent> = twiki::events::events(&bulk_doc)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = twiki::StreamingParser::new(|e: twiki::OwnedEvent| streamed.push(e));
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
        checked > 15,
        "expected to check a substantial number of twiki fixtures, got {checked}"
    );
    assert_or_known_failure("twiki", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(&doc)` must reproduce what
/// `build()` produces for the same AST, plus an incrementality probe.
#[test]
fn twiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("twiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/twiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = twiki::parse::parse(&input);
        let built = String::from_utf8(doc.emit()).expect("twiki emit output is UTF-8");

        let mut w = twiki::Writer::new(Vec::<u8>::new());
        for e in twiki::events::events(&doc) {
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
        checked > 15,
        "expected to check a substantial number of twiki fixtures, got {checked}"
    );

    if result.is_ok() {
        use twiki::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = twiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/Text/EndParagraph sequence and before finish() -- \
                 twiki::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                 reconstructs the AST + calls build() inside finish(), so it is not a genuine \
                 incremental streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("twiki", "streaming_writer", result);
}
