//! Streaming-API cross-checks for tikiwiki. Split out of the former monolithic
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
// tikiwiki: events() vs parse(), StreamingParser vs events(), streaming
// writer vs build() -- all fully wired
// ---------------------------------------------------------------------------
//
// Same architecture and same narrower-Wired-claim caveat as mediawiki-fmt
// above: `tikiwiki::tikiwiki_events` (`EventIter::new`, events.rs) calls
// `crate::parse::parse(input)` then walks the tree with `emit_block`/
// `emit_inlines`, so this check validates the AST->event projection layer,
// not two independent parsers.
mod tikiwiki_events_check {
    use super::{assert_or_known_failure, find_input, fixtures_root};
    use std::borrow::Cow;
    use tikiwiki::ast::{Block, Inline, TikiwikiDoc};
    use tikiwiki::events::OwnedEvent;
    type Event = OwnedEvent;

    fn tw_ast_to_events(doc: &TikiwikiDoc) -> Vec<Event> {
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
            Block::CodeBlock {
                content, language, ..
            } => {
                out.push(Event::CodeBlock {
                    language: language.clone(),
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::Blockquote { blocks, .. } => {
                out.push(Event::StartBlockquote);
                for b in blocks {
                    tw_block_events(b, out);
                }
                out.push(Event::EndBlockquote);
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
                    out.push(Event::StartTableRow {
                        is_header: row.is_header,
                    });
                    for cell in &row.cells {
                        out.push(Event::StartTableCell);
                        tw_inline_events(&cell.inlines, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
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
                Inline::Underline(c, _) => {
                    out.push(Event::StartUnderline);
                    tw_inline_events(c, out);
                    out.push(Event::EndUnderline);
                }
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
                Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
                Inline::Nowiki(s, _) => out.push(Event::Nowiki(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(Event::StartLink { url: url.clone() });
                    tw_inline_events(children, out);
                    out.push(Event::EndLink);
                }
                Inline::WikiLink { page, children, .. } => {
                    out.push(Event::StartWikiLink { page: page.clone() });
                    tw_inline_events(children, out);
                    out.push(Event::EndWikiLink);
                }
                Inline::Image { url, alt, .. } => out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                }),
                Inline::LineBreak { .. } => out.push(Event::LineBreak),
            }
        }
    }

    #[test]
    fn tikiwiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("tikiwiki");
        let mut checked = 0;
        let mut result: Result<(), String> = Ok(());
        for entry in std::fs::read_dir(&root).expect("fixtures/tikiwiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = tikiwiki::parse::parse(&input);
            let expected = tw_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = tikiwiki::tikiwiki_events(&input)
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
            "expected to check a substantial number of tikiwiki fixtures, got {checked}"
        );
        assert_or_known_failure("tikiwiki", "events", result);
    }
}

/// `StreamingParser` fed a tikiwiki fixture under an adversarial chunking
/// must deliver the same event sequence `events()` delivers over the whole
/// input. `tikiwiki::batch::StreamingParser::emit_block` re-parses each
/// accumulated block in isolation via `crate::events::events(&text)`.
#[test]
fn tikiwiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("tikiwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/tikiwiki dir") {
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
        let bulk: Vec<tikiwiki::OwnedEvent> = tikiwiki::tikiwiki_events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                tikiwiki::StreamingParser::new(|e: tikiwiki::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of tikiwiki fixtures, got {checked}"
    );
    assert_or_known_failure("tikiwiki", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// `build()` produces for the AST `parse(input)` returned, plus an
/// incrementality probe (`Writer::write_event` only pushes onto an internal
/// `Vec`; `finish()` reconstructs the AST and calls `crate::emit::build`).
#[test]
fn tikiwiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("tikiwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/tikiwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = tikiwiki::parse::parse(&input);
        let built = String::from_utf8(Emit::emit(&doc)).expect("emit() output is UTF-8");

        let mut w = tikiwiki::Writer::new(Vec::<u8>::new());
        for e in tikiwiki::tikiwiki_events(&input) {
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
        "expected to check a substantial number of tikiwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        use tikiwiki::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = tikiwiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/Text/EndParagraph sequence and before finish() -- \
                 tikiwiki::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                 reconstructs the AST + calls build() inside finish(), so it is not a genuine \
                 incremental streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("tikiwiki", "streaming_writer", result);
}
