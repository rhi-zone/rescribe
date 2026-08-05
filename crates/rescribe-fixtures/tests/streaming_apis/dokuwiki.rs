//! Streaming-API cross-checks for dokuwiki. Split out of the former monolithic
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
// dokuwiki: events() vs. an AST projection, StreamingParser adversarial
// chunking, streaming writer vs. builder
// ---------------------------------------------------------------------------

// dokuwiki's `events()` (`crate::events::events`, re-exported at the crate
// root, `lib.rs:33-35`) is `InputEventIter::new`, which calls
// `crate::parse::parse(input)` to build a `DokuwikiDoc`, walks it with the
// crate's own lazy `EventIter` (a genuine O(depth) stack-machine walker over
// the already-owned tree — see `events.rs`'s `Frame`/`Iterator::next`), and
// eagerly collects the result into an owned `Vec` before returning
// (`events.rs:705-731`, self-documented: "not a genuine streaming parser...
// Memory use is therefore O(full document), not O(depth)"). That is the same
// "parse() then walk the tree" shape as bbcode-fmt's and creole's `events()`
// (and html-fmt's `events_from_doc`), not two independent implementations —
// but as with those two, nothing in the DokuWiki format forces this shape,
// so per the bbcode/creole/asciidoc precedent it is wired here rather than
// declared `NotApplicable`: the check still pins the exact AST<->Event
// correspondence and would catch a field silently dropped or reordered by
// the walk. `StreamingParser` (`batch.rs`), by contrast, is a genuine
// incremental line-buffered state machine: `feed()` advances real per-line
// state (`BlockState::{Between,Accumulating,InSpecialBlock}`) and calls
// `emit_block()` (which re-parses just the accumulated block text via
// `crate::events::events()`, i.e. a fresh `parse::parse()` call) as soon as
// a blank line or a recognized `<code>/<file>/<html>/<php>` block boundary
// is seen — not only inside `finish()`. Unlike org-fmt/rst-fmt/djot-fmt's
// batch parsers, dokuwiki's `Parser` (`parse.rs`) has *no* cross-block
// state at all (no loose-list joining across blank lines — `parse_list_items`
// already stops at a non-`"  "`-prefixed line, which includes blank lines —
// and no forward/backward reference resolution), so every block boundary
// `StreamingParser` can pick is one `parse.rs`'s own top-level dispatch loop
// would also treat as a valid block split point; re-parsing each flushed
// chunk in isolation re-derives the identical block/inline structure a bulk
// `parse()` over the whole input would. Confirmed here over every dokuwiki
// fixture under every chunking in [`adversarial_chunkings`] plus the
// incrementality probe below. The streaming `Writer` (`writer.rs`)
// self-admits (module doc, `writer.rs:3`, "Buffers all events, reconstructs
// the AST, then emits") that `write_event()` only pushes onto an internal
// `Vec<OwnedEvent>` (`writer.rs:27-29`) and all real work happens inside
// `finish()` (`writer.rs:32-37`) — the same hollow pattern as
// bbcode/creole/texinfo/commonmark's writers. Its *content* still matches
// `build()` exactly (same reason: `finish()` ends up calling the same
// `crate::emit::build`), so only the incrementality probe fails.
//
// Event-enum expressiveness: every `Block`/`Inline` variant and field in
// `ast.rs` has a corresponding `Event` variant/field in `events.rs` (block
// metadata such as `FileBlock`'s `filename`, `RawBlock`'s `format`,
// `Macro`'s `name`, and `Image`'s `alt` all round-trip) — no expressiveness
// gap was found for this crate, unlike org-fmt (no metadata variant) or
// djot-fmt (no `LinkDef` variant).
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`dokuwiki::Event`] sequence `events()` must produce
/// for `doc`, directly from the AST `parse()` returned. Mirrors
/// `dokuwiki::events::{EventIter, emit_inline}` structurally (unavoidable:
/// `events()` is `parse()` + a walk over the resulting `DokuwikiDoc` — see
/// the module comment above for why that means this check pins the
/// AST<->Event correspondence rather than proving two independent
/// implementations agree).
fn dokuwiki_ast_to_events(doc: &dokuwiki::DokuwikiDoc) -> Vec<dokuwiki::OwnedEvent> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        dokuwiki_block_events(block, &mut out);
    }
    out
}

fn dokuwiki_block_events(block: &dokuwiki::Block, out: &mut Vec<dokuwiki::OwnedEvent>) {
    use dokuwiki::Block;
    use dokuwiki::Event;
    use std::borrow::Cow;
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for inline in inlines {
                dokuwiki_inline_events(inline, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            for inline in inlines {
                dokuwiki_inline_events(inline, out);
            }
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
        Block::FileBlock {
            language,
            filename,
            content,
            ..
        } => {
            out.push(Event::FileBlock {
                language: language.clone(),
                filename: filename.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                dokuwiki_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for inline in &item.inlines {
                    dokuwiki_inline_events(inline, out);
                }
                for child in &item.children {
                    dokuwiki_block_events(child, out);
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
                    for inline in &cell.inlines {
                        dokuwiki_inline_events(inline, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                for inline in &item.term {
                    dokuwiki_inline_events(inline, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for inline in &item.desc {
                    dokuwiki_inline_events(inline, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::HorizontalRule(_) => {
            out.push(Event::HorizontalRule);
        }
        Block::RawBlock {
            format, content, ..
        } => {
            out.push(Event::RawBlock {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Block::Macro { name, .. } => {
            out.push(Event::Macro { name: name.clone() });
        }
    }
}

fn dokuwiki_inline_events(inline: &dokuwiki::Inline, out: &mut Vec<dokuwiki::OwnedEvent>) {
    use dokuwiki::Event;
    use dokuwiki::Inline;
    use std::borrow::Cow;
    match inline {
        Inline::Text(s, _) => {
            out.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndSubscript);
        }
        Inline::Code(s, _) => {
            out.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::Nowiki(s, _) => {
            out.push(Event::Nowiki(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image { url, alt, .. } => {
            out.push(Event::InlineImage {
                url: url.clone(),
                alt: alt.clone(),
            });
        }
        Inline::FootnoteRef { content, .. } => {
            out.push(Event::FootnoteRef {
                content: content.clone(),
            });
        }
        Inline::LineBreak(_) => {
            out.push(Event::LineBreak);
        }
        Inline::SoftBreak(_) => {
            out.push(Event::SoftBreak);
        }
    }
}

#[test]
fn dokuwiki_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("dokuwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/dokuwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = dokuwiki::parse(&input);
        let expected = dokuwiki_ast_to_events(&doc);
        let actual: Vec<_> = dokuwiki::events(&input)
            .map(dokuwiki::Event::into_owned)
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
        checked > 10,
        "expected to check a substantial number of dokuwiki fixtures, got {checked}"
    );
    assert_or_known_failure("dokuwiki", "events", result);
}

/// `dokuwiki::StreamingParser` accumulates lines into blocks and calls
/// `emit_block()` (which re-parses just the accumulated block text via
/// `crate::events::events()`) as soon as a blank line or a recognized
/// `<code>/<file>/<html>/<php>` block boundary is seen — see `batch.rs`'s
/// `feed_line`/`emit_block` — so unlike texinfo/fb2/textile's
/// `StreamingParser` it is not a hollow buffer-then-`finish()` stub. Both
/// halves of this check pass for real: the adversarial-chunking equivalence
/// check against `events()` holds over every dokuwiki fixture, and the
/// incrementality probe below confirms `feed()` alone delivers events before
/// `finish()` is ever called. This holds cleanly (no coarser-boundary caveat
/// needed, unlike bbcode/creole's `detect_block_tag`) because `parse.rs`'s
/// `Parser` has no cross-block state: every block type's own consumption
/// loop (`parse_list_items`, `parse_table`, `parse_definition_list`,
/// `parse_blockquote`, `parse_paragraph`) already stops at the same
/// boundaries `StreamingParser::feed_line` flushes on (a blank line, or a
/// `<code>/<file>/<html>/<php>` tag), so re-parsing a flushed chunk in
/// isolation always reproduces the identical block/inline structure a bulk
/// `parse()` over that span would.
#[test]
fn dokuwiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("dokuwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/dokuwiki dir") {
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
        let bulk: Vec<dokuwiki::OwnedEvent> = dokuwiki::events(input_str)
            .map(dokuwiki::Event::into_owned)
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                dokuwiki::StreamingParser::new(|e: dokuwiki::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of dokuwiki fixtures, got {checked}"
    );

    // Incrementality probe: confirm a completed block's events reach the
    // handler as soon as its terminating blank line arrives, before
    // finish() is ever called.
    if result.is_ok() {
        let probe_input = b"**Hello**\n\nUnterminated tail with no blank line after it";
        let mut delivered: Vec<dokuwiki::OwnedEvent> = Vec::new();
        let mut parser = dokuwiki::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        if delivered.is_empty() {
            result = Err(
                "StreamingParser delivered zero events to the handler after feed() with a \
                 complete `**Hello**` paragraph followed by a blank line and unterminated \
                 trailing text, and before finish() was called — expected the completed first \
                 block to have been flushed as soon as its terminating blank line arrived"
                    .to_string(),
            );
        }
        // `parser` intentionally dropped without `finish()`: this probe only
        // needs to observe pre-finish handler state.
    }
    assert_or_known_failure("dokuwiki", "streaming_parser", result);
}

/// `dokuwiki::writer::Writer` self-admits (module doc, `writer.rs:3`) that
/// "Buffers all events, reconstructs the AST, then emits" — `write_event()`
/// (`writer.rs:27-29`) only pushes onto an internal `Vec<OwnedEvent>`, and
/// all real work (`events_to_doc` + `crate::emit::build`) happens inside
/// `finish()` (`writer.rs:32-37`). Checked the same way as
/// bbcode/creole/texinfo/commonmark's writers: byte-identical-to-builder
/// content correctness (expected to pass, since `finish()` ultimately drives
/// the same `build()` the builder path uses) plus an incrementality probe.
#[test]
fn dokuwiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("dokuwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/dokuwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = dokuwiki::parse(&input);
        let built = dokuwiki::build(&doc);

        let mut w = dokuwiki::Writer::new(Vec::<u8>::new());
        for e in dokuwiki::events(&input).map(dokuwiki::Event::into_owned) {
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
        "expected to check a substantial number of dokuwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = dokuwiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(dokuwiki::OwnedEvent::StartParagraph);
        w.write_event(dokuwiki::OwnedEvent::StartBold);
        w.write_event(dokuwiki::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(dokuwiki::OwnedEvent::EndBold);
        w.write_event(dokuwiki::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/StartBold/Text/EndBold/EndParagraph sequence and before \
                 finish() — dokuwiki::writer::Writer buffers all events into a Vec<OwnedEvent> \
                 and only reconstructs the AST + calls crate::emit::build inside finish() \
                 (crates/formats/dokuwiki/src/writer.rs, self-admitted in its own module doc), \
                 so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("dokuwiki", "streaming_writer", result);
}
