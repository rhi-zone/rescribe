//! Streaming-API cross-checks for vimwiki. Split out of the former monolithic
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
// vimwiki-fmt: events() vs parse(), StreamingParser vs events(), streaming
// writer vs build() -- all fully wired
// ---------------------------------------------------------------------------
//
// Same architecture and narrower-Wired-claim caveat as mediawiki-fmt/
// tikiwiki/twiki above: `vimwiki_fmt::events::EventIter::new` calls
// `crate::parse::parse(input)` then walks the tree with `emit_doc_events`/
// `emit_block_events`/`emit_inline_events`.
mod vimwiki_events_check {
    use super::{assert_or_known_failure, find_input, fixtures_root};
    use std::borrow::Cow;
    use vimwiki_fmt::ast::{Block, Inline, VimwikiDoc};
    use vimwiki_fmt::events::OwnedEvent;
    type Event = OwnedEvent;

    fn vw_ast_to_events(doc: &VimwikiDoc) -> Vec<Event> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            vw_block_events(b, &mut out);
        }
        out
    }

    fn vw_block_events(b: &Block, out: &mut Vec<Event>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                vw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                vw_inline_events(inlines, out);
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
            // events.rs's `emit_block_events` wraps a vimwiki blockquote's flat
            // `inlines` in a synthetic StartParagraph/EndParagraph pair inside
            // the blockquote (blockquotes hold inlines directly in the AST, not
            // a nested paragraph block) -- mirrored here, not simplified away,
            // since the projection must match what events() actually emits.
            Block::Blockquote { inlines, .. } => {
                out.push(Event::StartBlockquote);
                out.push(Event::StartParagraph);
                vw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
                out.push(Event::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item in items {
                    out.push(Event::StartListItem {
                        checked: item.checked,
                    });
                    vw_inline_events(&item.inlines, out);
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(Event::StartTable);
                for row in rows {
                    out.push(Event::StartTableRow);
                    for cell in &row.cells {
                        out.push(Event::StartTableCell);
                        vw_inline_events(cell, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
            Block::DefinitionList { items, .. } => {
                out.push(Event::StartDefinitionList);
                for item in items {
                    out.push(Event::StartDefinitionTerm);
                    vw_inline_events(&item.term, out);
                    out.push(Event::EndDefinitionTerm);
                    out.push(Event::StartDefinitionDesc);
                    vw_inline_events(&item.desc, out);
                    out.push(Event::EndDefinitionDesc);
                }
                out.push(Event::EndDefinitionList);
            }
        }
    }

    fn vw_inline_events(inlines: &[Inline], out: &mut Vec<Event>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(Event::StartBold);
                    vw_inline_events(c, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(Event::StartItalic);
                    vw_inline_events(c, out);
                    out.push(Event::EndItalic);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(Event::StartStrikethrough);
                    vw_inline_events(c, out);
                    out.push(Event::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(Event::StartSuperscript);
                    vw_inline_events(c, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(Event::StartSubscript);
                    vw_inline_events(c, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, label, .. } => {
                    out.push(Event::StartLink { url: url.clone() });
                    out.push(Event::Text(Cow::Owned(label.clone())));
                    out.push(Event::EndLink);
                }
                Inline::Image {
                    url, alt, style, ..
                } => out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                    style: style.clone(),
                }),
            }
        }
    }

    #[test]
    fn vimwiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("vimwiki");
        let mut checked = 0;
        let mut result: Result<(), String> = Ok(());
        for entry in std::fs::read_dir(&root).expect("fixtures/vimwiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = vimwiki_fmt::parse::parse(&input);
            let expected = vw_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = vimwiki_fmt::events(&input)
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
            "expected to check a substantial number of vimwiki fixtures, got {checked}"
        );
        assert_or_known_failure("vimwiki", "events", result);
    }
}

/// `StreamingParser` fed a vimwiki fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
/// `vimwiki_fmt::batch::StreamingParser::emit_block` re-parses each
/// accumulated block in isolation via `crate::events::events(&text)`. The
/// crate's own `test_streaming_matches_bulk` (batch.rs) already exercises
/// one hand-picked heading+2-paragraph input under 7-byte chunking; this
/// generalizes that self-check to the full adversarial-chunking suite (whole
/// input, single-byte, 3/7/13-byte chunks, mid-UTF-8-char split) over every
/// `fixtures/vimwiki/` fixture.
#[test]
fn vimwiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("vimwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/vimwiki dir") {
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
        let bulk: Vec<vimwiki_fmt::OwnedEvent> = vimwiki_fmt::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                vimwiki_fmt::StreamingParser::new(|e: vimwiki_fmt::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of vimwiki fixtures, got {checked}"
    );
    assert_or_known_failure("vimwiki", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// `build()` produces for the AST `parse(input)` returned, plus an
/// incrementality probe. `Writer::write_event` (writer.rs) only pushes onto
/// an internal `Vec<OwnedEvent>`; `finish()` calls
/// `crate::events::collect_doc_from_events` then `crate::emit::build`.
#[test]
fn vimwiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("vimwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/vimwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = vimwiki_fmt::parse(&input);
        let built = vimwiki_fmt::build(&doc);

        let mut w = vimwiki_fmt::Writer::new(Vec::<u8>::new());
        for e in vimwiki_fmt::events(&input) {
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
        "expected to check a substantial number of vimwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        use vimwiki_fmt::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = vimwiki_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/Text/EndParagraph sequence and before finish() -- \
                 vimwiki_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                 reconstructs the AST + calls build() inside finish(), so it is not a genuine \
                 incremental streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("vimwiki", "streaming_writer", result);
}
