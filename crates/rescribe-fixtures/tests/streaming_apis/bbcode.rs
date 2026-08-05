//! Streaming-API cross-checks for bbcode. Split out of the former monolithic
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
// bbcode-fmt: events() is `parse::parse(input)` followed by a tree walk (see
// crates/formats/bbcode-fmt/src/events.rs's `events()`, which literally
// calls `crate::parse::parse(input)` before walking the resulting
// `BbcodeDoc`) — the same "walk the tree parse() already built" shape as
// html-fmt's `events_from_doc(&parse(input).0)`, not an independent
// incremental reader. Unlike html-fmt, there is no format-spec reason
// (foster parenting, adoption agency, etc.) forcing that shape here — it's
// an implementation choice, not a structural absence — so per this task's
// brief the check is wired (like asciidoc's honestly-scoped entry) rather
// than declared `NotApplicable`: it still pins the current AST<->Event
// correspondence and would catch a field silently dropped or reordered by
// the walk, even though it cannot demonstrate two independent parsers.
// `StreamingParser` (batch.rs), by contrast, *is* a genuine incremental
// line-buffered state machine — feed() advances real parser state and calls
// emit_block() (and therefore the handler) as soon as a block boundary
// (blank line, or a recognized block tag's close line) is recognized, not
// only inside finish(). Both the incrementality probe and the
// adversarial-chunking equivalence check against events() pass for real,
// over all 53 bbcode fixtures plus several hand-built adversarial cases
// tried while auditing this (same-line-closed block tag immediately
// followed by more content with no blank line; a blank line inside an
// InBlock quote; nested same-tag quotes) — every case converges because
// StreamingParser's own block-boundary detection only ever needs to be
// *coarser than or equal to* parse()'s, never finer: whatever text it
// accumulates into one flushed chunk gets handed to `crate::events::events()`
// (i.e. a fresh `parse::parse()` call), which re-derives the exact same
// fine-grained block/inline structure the bulk parser would have for that
// span. The streaming `Writer` self-admits (module doc, writer.rs:3) it
// buffers all events and only reconstructs the AST + calls emit() inside
// finish() — the same hollow pattern as texinfo/commonmark's writers; its
// *content* still matches build() exactly (same reason: finish() ends up
// calling the same emit()), so only the incrementality probe fails.
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`bbcode_fmt::events::Event`] sequence `events()`
/// must produce for `doc`, directly from the AST `parse()` returned. Mirrors
/// `bbcode_fmt::events::{emit_block_events, emit_inline_events}` structurally
/// (unavoidable, since bbcode-fmt's `Event` enum is a direct 1:1 mirror of
/// `Block`/`Inline` — see the module comment above for why that means this
/// check pins the AST<->Event correspondence rather than proving two
/// independent implementations agree).
fn bbcode_ast_to_events(doc: &bbcode_fmt::BbcodeDoc) -> Vec<bbcode_fmt::OwnedEvent> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        bbcode_block_events(block, &mut out);
    }
    out
}

fn bbcode_block_events(block: &bbcode_fmt::ast::Block, out: &mut Vec<bbcode_fmt::OwnedEvent>) {
    use bbcode_fmt::Event;
    use bbcode_fmt::ast::Block;
    use std::borrow::Cow;
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for inline in inlines {
                bbcode_inline_events(inline, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock {
            language, content, ..
        } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote {
            author, children, ..
        } => {
            out.push(Event::StartBlockquote {
                author: author.clone(),
            });
            for child in children {
                bbcode_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for inline in item {
                    bbcode_inline_events(inline, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow);
                for (is_header, inlines) in &row.cells {
                    out.push(Event::StartTableCell {
                        is_header: *is_header,
                    });
                    for inline in inlines {
                        bbcode_inline_events(inline, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
        Block::Heading {
            level, children, ..
        } => {
            out.push(Event::StartHeading { level: *level });
            for inline in children {
                bbcode_inline_events(inline, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Alignment { kind, children, .. } => {
            out.push(Event::StartAlignment { kind: *kind });
            for child in children {
                bbcode_block_events(child, out);
            }
            out.push(Event::EndAlignment);
        }
        Block::Spoiler { children, .. } => {
            out.push(Event::StartSpoiler);
            for child in children {
                bbcode_block_events(child, out);
            }
            out.push(Event::EndSpoiler);
        }
        Block::Preformatted { content, .. } => {
            out.push(Event::Preformatted {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Indent { children, .. } => {
            out.push(Event::StartIndent);
            for child in children {
                bbcode_block_events(child, out);
            }
            out.push(Event::EndIndent);
        }
    }
}

fn bbcode_inline_events(inline: &bbcode_fmt::ast::Inline, out: &mut Vec<bbcode_fmt::OwnedEvent>) {
    use bbcode_fmt::Event;
    use bbcode_fmt::ast::Inline;
    use std::borrow::Cow;
    match inline {
        Inline::Text(s, _) => {
            out.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Code(s, _) => {
            out.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image {
            url, width, height, ..
        } => {
            out.push(Event::InlineImage {
                url: url.clone(),
                width: *width,
                height: *height,
            });
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndSubscript);
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndSuperscript);
        }
        Inline::Color {
            value, children, ..
        } => {
            out.push(Event::StartColor {
                value: value.clone(),
            });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndColor);
        }
        Inline::Size {
            value, children, ..
        } => {
            out.push(Event::StartSize {
                value: value.clone(),
            });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndSize);
        }
        Inline::Font { name, children, .. } => {
            out.push(Event::StartFont { name: name.clone() });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndFont);
        }
        Inline::Email { addr, children, .. } => {
            out.push(Event::StartEmail { addr: addr.clone() });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndEmail);
        }
        Inline::Noparse(s, _) => {
            out.push(Event::Noparse(Cow::Owned(s.clone())));
        }
        Inline::Span {
            attr,
            value,
            children,
            ..
        } => {
            out.push(Event::StartSpan {
                attr: attr.clone(),
                value: value.clone(),
            });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndSpan);
        }
    }
}

#[test]
fn bbcode_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("bbcode");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/bbcode dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = bbcode_fmt::parse(&input);
        let expected = bbcode_ast_to_events(&doc);
        let actual: Vec<_> = bbcode_fmt::events(&input)
            .map(bbcode_fmt::Event::into_owned)
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
        "expected to check a substantial number of bbcode fixtures, got {checked}"
    );
    assert_or_known_failure("bbcode", "events", result);
}

/// `bbcode_fmt::batch::StreamingParser` accumulates lines into blocks and
/// calls `emit_block()` (which re-parses just the accumulated block text via
/// `crate::events::events()`) as soon as a block boundary is recognized —
/// see `crates/formats/bbcode-fmt/src/batch.rs`'s `feed_line`/`emit_block` —
/// so unlike texinfo/fb2/textile's `StreamingParser` it is not a hollow
/// buffer-then-`finish()` stub. Both halves of this check pass for real:
/// the adversarial-chunking equivalence check against `events()` holds over
/// every bbcode fixture, and the incrementality probe below confirms
/// `feed()` alone delivers events before `finish()` is ever called.
/// `detect_block_tag` (batch.rs:200-224) is coarser than `parse.rs`'s
/// `is_block_start` — it is missing heading/`[hr]` tags entirely and
/// returns `None` (no boundary at all) whenever a recognized block tag's
/// close appears on the same line (batch.rs:217-219) — but this never
/// causes a *visible* divergence: everywhere the streaming splitter is
/// coarser, it only accumulates more text into one flushed chunk, and that
/// chunk is handed to a fresh `crate::events::events()` call, which
/// re-derives the identical fine-grained block/inline structure a bulk
/// `parse()` over that span would have produced. Confirmed by hand against
/// several adversarial cases beyond the fixture suite (same-line-closed
/// tag immediately followed by more content, a blank line inside an
/// `InBlock` quote, nested same-tag quotes) in addition to all 53 fixtures
/// under every chunking in [`adversarial_chunkings`].
#[test]
fn bbcode_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("bbcode");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/bbcode dir") {
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
        let bulk: Vec<bbcode_fmt::OwnedEvent> = bbcode_fmt::events(input_str)
            .map(bbcode_fmt::Event::into_owned)
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                bbcode_fmt::StreamingParser::new(|e: bbcode_fmt::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of bbcode fixtures, got {checked}"
    );

    // Incrementality probe: most individual fixtures are a single block (no
    // internal blank-line boundary), so nothing would legitimately flush
    // before finish() even under a fully incremental implementation — that
    // is not evidence of hollowness (see fixture adv-deeply-nested-unclosed,
    // one unterminated paragraph, found while first drafting this probe as
    // a per-fixture check). Use one hand-built input with an internal block
    // boundary (a completed bold paragraph, a blank line, then unterminated
    // trailing content) instead, and confirm the completed block's events
    // reach the handler before finish() is ever called.
    if result.is_ok() {
        let probe_input = b"[b]Hello[/b]\n\nUnterminated tail with no blank line after it";
        let mut delivered: Vec<bbcode_fmt::OwnedEvent> = Vec::new();
        let mut parser = bbcode_fmt::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        if delivered.is_empty() {
            result = Err(
                "StreamingParser delivered zero events to the handler after feed() with a \
                 complete `[b]Hello[/b]` paragraph followed by a blank line and unterminated \
                 trailing text, and before finish() was called — expected the completed first \
                 block to have been flushed as soon as its terminating blank line arrived"
                    .to_string(),
            );
        }
        // `parser` intentionally dropped without `finish()`: this probe only
        // needs to observe pre-finish handler state.
    }
    assert_or_known_failure("bbcode", "streaming_parser", result);
}

/// `bbcode_fmt::writer::Writer` self-admits (module doc, writer.rs:3) that
/// "this implementation buffers all events, reconstructs the AST, then
/// emits" — `write_event()` (writer.rs:42-44) only pushes onto an internal
/// `Vec<OwnedEvent>`, and all real work (`events_to_doc` + `emit::emit`)
/// happens inside `finish()`. Checked the same way as texinfo/commonmark's
/// writers: byte-identical-to-builder content correctness (expected to
/// pass, since `finish()` ultimately drives the same `emit()` the builder
/// path uses) plus an incrementality probe.
#[test]
fn bbcode_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("bbcode");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/bbcode dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = bbcode_fmt::parse(&input);
        let built = bbcode_fmt::emit(&doc);

        let mut w = bbcode_fmt::Writer::new(Vec::<u8>::new());
        for e in bbcode_fmt::events(&input).map(bbcode_fmt::Event::into_owned) {
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
        "expected to check a substantial number of bbcode fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = bbcode_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(bbcode_fmt::OwnedEvent::StartParagraph);
        w.write_event(bbcode_fmt::OwnedEvent::StartBold);
        w.write_event(bbcode_fmt::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(bbcode_fmt::OwnedEvent::EndBold);
        w.write_event(bbcode_fmt::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/StartBold/Text/EndBold/EndParagraph sequence and before \
                 finish() — bbcode_fmt::writer::Writer buffers all events into a \
                 Vec<OwnedEvent> and only reconstructs the AST + calls emit() inside finish() \
                 (crates/formats/bbcode-fmt/src/writer.rs, self-admitted in its own module doc), \
                 so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("bbcode", "streaming_writer", result);
}
