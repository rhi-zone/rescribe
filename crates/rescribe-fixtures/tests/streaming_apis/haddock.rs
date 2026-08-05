//! Streaming-API cross-checks for haddock. Split out of the former monolithic
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
// haddock-fmt: `haddock_fmt::events()` (src/lib.rs) is `events::events(input)`
// which is `parse(input)` then a lazy frame-stack `EventIter::expand_block`
// walk of the AST parse() already built — the same events()-is-parse()+
// AST-walk pattern already documented for t2t/pod/asciidoc above, not an
// independently-implemented reader. The ast_to_events-vs-events() check
// below is real and passes, but validates the AST->event expansion layer.
//
// StreamingParser (batch.rs) genuinely flushes events per accumulated block
// as fed (blank line or EOF triggers emit_block(), which re-parses just
// that block's text via crate::events::events()) — architecturally the same
// "re-parse each block alone" shape as t2t/org/asciidoc's StreamingParsers.
// Unlike those, every haddock block-termination rule in parse.rs (heading,
// paragraph, code block, @-code block, doctest, lists, definition list,
// property) depends only on the content of lines within the block being
// scanned — never on cross-block state or document position (no
// document-start-only special case the way t2t's 3-line header lookahead
// is) — so re-parsing an isolated block's text from scratch recovers
// exactly the same block boundaries parse() would find inline. This harness
// found no fixture where StreamingParser disagrees with events() under
// adversarial chunking, so streaming_parser is Wired, not KnownFailure.
//
// Writer buffers all fed events into a Vec<OwnedEvent> and only
// reconstructs the AST + calls emit::build() inside finish() (writer.rs's
// own module doc: "This implementation buffers all events, reconstructs the
// AST, then emits") — the same fake-streaming-writer pattern as
// t2t/pod/textile/commonmark/org/texinfo.
// ---------------------------------------------------------------------------

fn haddock_ast_to_events(doc: &haddock_fmt::HaddockDoc) -> Vec<haddock_fmt::OwnedEvent> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        haddock_block_events(b, &mut out);
    }
    out
}

fn haddock_block_events(b: &haddock_fmt::Block, out: &mut Vec<haddock_fmt::OwnedEvent>) {
    use haddock_fmt::{Block, Event};
    match b {
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            for i in inlines {
                haddock_inline_events(i, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                haddock_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock { content, .. } => out.push(Event::CodeBlock {
            content: content.clone().into(),
        }),
        Block::AtCodeBlock { content, .. } => out.push(Event::AtCodeBlock {
            content: content.clone().into(),
        }),
        Block::UnorderedList { items, .. } => {
            out.push(Event::StartUnorderedList);
            for item in items {
                out.push(Event::StartListItem);
                for i in item {
                    haddock_inline_events(i, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndUnorderedList);
        }
        Block::OrderedList { items, .. } => {
            out.push(Event::StartOrderedList);
            for item in items {
                out.push(Event::StartListItem);
                for i in item {
                    haddock_inline_events(i, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndOrderedList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for (term, desc) in items {
                out.push(Event::StartDefinitionTerm);
                for i in term {
                    haddock_inline_events(i, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for i in desc {
                    haddock_inline_events(i, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::DocTest {
            expression, result, ..
        } => out.push(Event::DocTest {
            expression: expression.clone().into(),
            result: result.clone().map(Into::into),
        }),
        Block::Blockquote { inlines, .. } => {
            out.push(Event::StartBlockquote);
            for i in inlines {
                haddock_inline_events(i, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::Property {
            key,
            name,
            description,
            ..
        } => {
            out.push(Event::Property {
                key: key.clone().into(),
                name: name.clone().map(Into::into),
            });
            for i in description {
                haddock_inline_events(i, out);
            }
            out.push(Event::EndProperty);
        }
    }
}

fn haddock_inline_events(i: &haddock_fmt::Inline, out: &mut Vec<haddock_fmt::OwnedEvent>) {
    use haddock_fmt::{Event, Inline};
    match i {
        Inline::Text(s, _) => out.push(Event::Text(s.clone().into())),
        Inline::Code(s, _) => out.push(Event::InlineCode(s.clone().into())),
        Inline::Strong(children, _) => {
            out.push(Event::StartStrong);
            for c in children {
                haddock_inline_events(c, out);
            }
            out.push(Event::EndStrong);
        }
        Inline::Emphasis(children, _) => {
            out.push(Event::StartEmphasis);
            for c in children {
                haddock_inline_events(c, out);
            }
            out.push(Event::EndEmphasis);
        }
        Inline::Link { url, text, .. } => {
            out.push(Event::StartLink {
                url: url.clone(),
                text: text.clone(),
            });
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndLink);
        }
        Inline::ModuleLink { module, .. } => out.push(Event::ModuleLink {
            module: module.clone(),
        }),
    }
}

#[test]
fn haddock_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("haddock");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/haddock dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = haddock_fmt::parse(&input);
        let expected = haddock_ast_to_events(&doc);
        let actual: Vec<_> = haddock_fmt::events(&input)
            .map(|e| e.into_owned())
            .collect();
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
        "expected to check a substantial number of haddock fixtures, got {checked}"
    );
}

/// `StreamingParser` flushes events per accumulated block as fed, re-parsing
/// each block's text in isolation via `crate::events::events()`. Checked via
/// adversarial-chunking equivalence against `events()` over the whole input.
#[test]
fn haddock_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("haddock");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/haddock dir") {
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
        let bulk: Vec<haddock_fmt::OwnedEvent> = haddock_fmt::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                haddock_fmt::batch::StreamingParser::new(|e: haddock_fmt::OwnedEvent| {
                    streamed.push(e);
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
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of haddock fixtures, got {checked}"
    );
    assert_or_known_failure("haddock", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<OwnedEvent>` and only
/// reconstructs the AST + calls `emit::build()` inside `finish()` (see
/// `crates/formats/haddock-fmt/src/writer.rs`'s own module doc). Checked via
/// byte-identical comparison against the builder path, plus an
/// incrementality probe.
#[test]
fn haddock_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("haddock");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/haddock dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = haddock_fmt::parse(&input);
        let built = haddock_fmt::build(&doc);

        let mut w = haddock_fmt::Writer::new(Vec::<u8>::new());
        for e in haddock_fmt::events(&input) {
            w.write_event(e.into_owned());
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
        "expected to check a substantial number of haddock fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use haddock_fmt::Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = haddock_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text("Hello world".to_string().into()));
        w.write_event(Event::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — haddock_fmt::writer::Writer \
                 buffers all events into a Vec<OwnedEvent> and only reconstructs the AST + \
                 calls emit::build() inside finish() (crates/formats/haddock-fmt/src/writer.rs, \
                 self-admitted in its own module doc), so it is not a genuine incremental \
                 streaming writer despite content round-tripping correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("haddock", "streaming_writer", result);
}
