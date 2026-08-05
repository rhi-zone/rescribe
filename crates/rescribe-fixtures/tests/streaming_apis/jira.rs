//! Streaming-API cross-checks for jira. Split out of the former monolithic
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
// jira-fmt: events() vs AST projection, StreamingParser vs events(),
// streaming Writer vs build()
// ---------------------------------------------------------------------------
//
// jira-fmt's `events()` (`crates/formats/jira-fmt/src/events.rs::events`) is
// `crate::parse::parse(input)` followed by a full walk of the resulting
// `JiraDoc` into a `Vec<OwnedEvent>` (`emit_doc_events`/`emit_block_events`/
// `emit_inline_events`) — the same "parse() then walk the tree" shape as
// bbcode-fmt's, creole's, and dokuwiki's `events()`, not two independent
// implementations. Nothing in the Jira wiki markup format forces this shape,
// but per the bbcode/creole/dokuwiki precedent established earlier in this
// file it is still wired as `Wired` rather than `NotApplicable` — the check
// below still pins the real AST<->Event correspondence (a hand-written
// projection built independently from `ast.rs`, not by calling the crate's
// own private `emit_*_events` helpers). `jira_fmt::Event` has a variant
// carrying every field every `Block`/`Inline` variant holds (checked by
// exhaustive match below) — no expressiveness gap was found for this crate.
//
// `jira_fmt::batch::StreamingParser` (`batch.rs`) is a genuine incremental
// line-buffered state machine, not a hollow buffer-then-`finish()` stub:
// `feed_line` dispatches per line into `{code:.../{quote}/{noformat}/{panel`
// delimited-block accumulation or blank-line-terminated block accumulation,
// and `emit_block()` re-parses just the accumulated block text via
// `crate::events::events()` as soon as a boundary is seen — real `Wired`,
// confirmed below by an adversarial-chunking equivalence check against
// `events()` over every jira fixture plus an incrementality probe. This
// holds with no coarser-boundary caveat (unlike bbcode/creole's
// `detect_block_tag`): `parse.rs`'s `Parser` has no state that spans a blank
// line or a delimited-block boundary (no loose-list joining, no reference
// resolution, no title/attribute line preceding a fence — the `{code:lang}`
// language and `{panel:title=...}` title are both encoded on the fence line
// itself, so there is no "flush a decorator line away from its target"
// construct for this format's grammar to trigger the class of bug found in
// org-fmt/asciidoc/djot-fmt), so every boundary `feed_line` flushes on is
// one `parse.rs`'s own `parse_paragraph`/`parse_list_at_depth`/`parse_table`
// stop conditions would also treat as a block boundary — re-parsing a
// flushed chunk in isolation always reproduces the identical block/inline
// structure a bulk `parse()` over that span would.
//
// `jira_fmt::writer::Writer` self-admits (module doc, `writer.rs:1-3`)
// "This implementation buffers all events, reconstructs the AST, then
// emits" — `write_event()` (`writer.rs:40-42`) only pushes onto an internal
// `Vec<OwnedEvent>`, and all real work (`events_to_doc` + `crate::emit::
// build`) happens inside `finish()` (`writer.rs:45-50`). Checked the same
// way as bbcode/creole/dokuwiki's writers: byte-identical-to-builder content
// correctness (expected to pass, since `finish()` ultimately drives the same
// `build()` path the builder uses) plus an incrementality probe that is
// expected to fail (zero bytes reach the sink before `finish()`).
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`jira_fmt::OwnedEvent`] sequence `events()` must
/// produce for `doc`, directly from the AST `parse()` returned. Mirrors
/// `jira_fmt::events::{emit_block_events, emit_inline_events}` structurally
/// (unavoidable: `events()` is `parse()` + a walk over the resulting
/// `JiraDoc` — see the module comment above for why that means this check
/// pins the AST<->Event correspondence rather than proving two independent
/// implementations agree), but built independently from `jira_fmt::ast`
/// rather than by calling those private crate-internal helpers.
fn jira_ast_to_events(doc: &jira_fmt::JiraDoc) -> Vec<jira_fmt::OwnedEvent> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        jira_block_events(block, &mut out);
    }
    out
}

fn jira_block_events(block: &jira_fmt::Block, out: &mut Vec<jira_fmt::OwnedEvent>) {
    use jira_fmt::Block;
    use jira_fmt::Event;
    use jira_fmt::ListItemContent;
    use std::borrow::Cow;
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            jira_inline_events_all(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            jira_inline_events_all(inlines, out);
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
        Block::Noformat { content, .. } => {
            out.push(Event::Noformat {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                jira_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::Panel {
            title, children, ..
        } => {
            out.push(Event::StartPanel {
                title: title.clone(),
            });
            for child in children {
                jira_block_events(child, out);
            }
            out.push(Event::EndPanel);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for content in &item.children {
                    match content {
                        ListItemContent::Inline(inlines) => {
                            out.push(Event::StartParagraph);
                            jira_inline_events_all(inlines, out);
                            out.push(Event::EndParagraph);
                        }
                        ListItemContent::NestedList(nested) => {
                            jira_block_events(nested, out);
                        }
                    }
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
                    jira_inline_events_all(&cell.inlines, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
    }
}

fn jira_inline_events_all(inlines: &[jira_fmt::Inline], out: &mut Vec<jira_fmt::OwnedEvent>) {
    for inline in inlines {
        jira_inline_events(inline, out);
    }
}

fn jira_inline_events(inline: &jira_fmt::Inline, out: &mut Vec<jira_fmt::OwnedEvent>) {
    use jira_fmt::Event;
    use jira_fmt::Inline;
    use std::borrow::Cow;
    match inline {
        Inline::Text(s, _) => {
            out.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            jira_inline_events_all(children, out);
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            jira_inline_events_all(children, out);
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            jira_inline_events_all(children, out);
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            jira_inline_events_all(children, out);
            out.push(Event::EndStrikethrough);
        }
        Inline::Code(s, _) => {
            out.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            jira_inline_events_all(children, out);
            out.push(Event::EndLink);
        }
        Inline::Image { url, alt, .. } => {
            out.push(Event::InlineImage {
                url: url.clone(),
                alt: alt.clone(),
            });
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            jira_inline_events_all(children, out);
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            jira_inline_events_all(children, out);
            out.push(Event::EndSubscript);
        }
        Inline::ColorSpan {
            color, children, ..
        } => {
            out.push(Event::StartColorSpan {
                color: color.clone(),
            });
            jira_inline_events_all(children, out);
            out.push(Event::EndColorSpan);
        }
        Inline::Mention(name, _) => {
            out.push(Event::Mention(Cow::Owned(name.clone())));
        }
    }
}

#[test]
fn jira_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("jira");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jira dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = jira_fmt::JiraDoc::parse(input.as_bytes());
        let expected = jira_ast_to_events(&doc);
        let actual: Vec<_> = jira_fmt::events_str(&input).collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  \
                 ast-derived: {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of jira fixtures, got {checked}"
    );
    assert_or_known_failure("jira", "events", result);
}

#[test]
fn jira_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("jira");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jira dir") {
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
        let bulk: Vec<jira_fmt::OwnedEvent> = jira_fmt::events_str(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                jira_fmt::StreamingParser::new(|e: jira_fmt::OwnedEvent| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():        {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of jira fixtures, got {checked}"
    );

    // Incrementality probe: confirm a completed block's events reach the
    // handler as soon as its terminating blank line arrives, before
    // finish() is ever called.
    if result.is_ok() {
        let probe_input = b"*Hello*\n\nUnterminated tail with no blank line after it";
        let mut delivered: Vec<jira_fmt::OwnedEvent> = Vec::new();
        let mut parser = jira_fmt::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        if delivered.is_empty() {
            result = Err(
                "StreamingParser delivered zero events to the handler after feed() with a \
                 complete `*Hello*` paragraph followed by a blank line and unterminated \
                 trailing text, and before finish() was called — expected the completed first \
                 block to have been flushed as soon as its terminating blank line arrived"
                    .to_string(),
            );
        }
        // `parser` intentionally dropped without `finish()`: this probe only
        // needs to observe pre-finish handler state.
    }
    assert_or_known_failure("jira", "streaming_parser", result);
}

/// `jira_fmt::writer::Writer` self-admits (module doc, `writer.rs:1-3`) that
/// "this implementation buffers all events, reconstructs the AST, then
/// emits" — `write_event()` (`writer.rs:40-42`) only pushes onto an internal
/// `Vec<OwnedEvent>`, and all real work (`events_to_doc` + `crate::emit::
/// build`) happens inside `finish()` (`writer.rs:45-50`). Checked the same
/// way as bbcode/creole/dokuwiki's writers: byte-identical-to-builder
/// content correctness (expected to pass, since `finish()` ultimately drives
/// the same `build()` path the builder uses) plus an incrementality probe.
#[test]
fn jira_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("jira");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jira dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = jira_fmt::JiraDoc::parse(input.as_bytes());
        let built = String::from_utf8(doc.emit()).expect("emit produces UTF-8");

        let mut w = jira_fmt::Writer::new(Vec::<u8>::new());
        for e in jira_fmt::events_str(&input) {
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
        "expected to check a substantial number of jira fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = jira_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(jira_fmt::OwnedEvent::StartParagraph);
        w.write_event(jira_fmt::OwnedEvent::StartBold);
        w.write_event(jira_fmt::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(jira_fmt::OwnedEvent::EndBold);
        w.write_event(jira_fmt::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/StartBold/Text/EndBold/EndParagraph sequence and before \
                 finish() — jira_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> \
                 and only reconstructs the AST + calls crate::emit::build inside finish() \
                 (crates/formats/jira-fmt/src/writer.rs, self-admitted in its own module doc), \
                 so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("jira", "streaming_writer", result);
}
