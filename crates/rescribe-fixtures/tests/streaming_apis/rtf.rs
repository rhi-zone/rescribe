//! Streaming-API cross-checks for rtf. Split out of the former monolithic
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
// rtf-fmt: `rtf_fmt::events()` (`sem_events::events`, src/sem_events.rs) is
// `SemanticEventIter::new(parse(input).0)` — a lazy frame-stack walk of the
// AST `parse()` already built (same pattern as t2t/pod/haddock/fountain/
// asciidoc/man above); per that established precedent it is still `Wired`.
// `sem_events::Event` did not derive `PartialEq` before this pass (only
// `Debug` — crates/formats/rtf-fmt/src/sem_events.rs:29); added here so this
// harness's exact-sequence equivalence check (not a lossy shape comparison)
// is possible, mirroring rst-fmt's `ast_to_events`-vs-`events()` pattern.
// `RtfDoc`/`Block`/`Inline`/`Align`/`TableRow` all already derived
// `PartialEq` beforehand.
//
// `batch::StreamingParser` (batch.rs:107-139) is a confirmed buffer-then-
// finish stub, not a genuine incremental parser: `feed()` only appends to an
// internal `Vec<u8>` (batch.rs:130-132) and `finish()` calls
// `crate::sem_events::events(&self.buf)` exactly once and forwards every
// event (batch.rs:135-139) — the module's own doc block (batch.rs:9-18)
// argues this is an "inherent property of the RTF format" (font/color
// tables must be parsed before body content is meaningful) rather than an
// implementation shortfall, but per this harness's rule a buffer-then-
// finish implementation is always `KnownFailure`, never `NotApplicable`,
// regardless of the format's structural excuse. Directly verified: feeding
// any prefix through `feed()` alone (without calling `finish()`) delivers
// zero events to the handler, for any input.
//
// `streaming_writer` is now fixed by two independent changes, both directly
// verified below:
//
// (1) `sem_writer::Writer` (crates/formats/rtf-fmt/src/sem_writer.rs) is a
// new, genuinely incremental writer consuming the crate's own semantic
// `Event`/`OwnedEvent` type — the gap this entry used to track ("rtf-fmt has
// no writer that consumes its own semantic event stream at all"). It solves
// RTF's real structural constraint (the `\fonttbl`/`\colortbl` header must
// precede any `\f<n>`/`\cf<n>` body reference, but which fonts/colors are
// used can only be known by having walked the whole document) without
// buffering the body: `Event::StartDocument`, always the first event, now
// carries the exact tables (`rtf_fmt::build_font_map`/`build_color_map`),
// computed once from the AST `events()` already has fully parsed before
// yielding anything — so the header can be written and flushed immediately,
// and every subsequent `StartFont`/`StartColor`/`StartBgColor` event is a
// plain lookup into a table already known in full. `out` is flushed to the
// sink whenever the writer's small context stack returns to empty (between
// top-level blocks), so memory is `O(largest top-level block/table-row +
// nesting depth)`, not `O(document size)` — verified by
// `test_writer_peak_memory_bounded` (crate-internal, alloc-probe based) and
// `test_writer_is_incremental` (bytes reach an `ObservableSink` before
// `finish()`).
//
// (2) The low-level `writer::Writer` (writer.rs) had a second, independent
// defect even at the `TokenEvent` level, traced to its true root cause: the
// tokenizer (`events.rs::read_control_word`) silently discarded whether a
// control word's optional trailing-space delimiter was present in the
// source. No re-serialization *policy* keyed off `name`/`param` can recover
// that already-discarded bit (confirmed directly: `\f0 Times` has a
// delimiter space and `\u65?` does not, despite both being param-carrying
// control words — `emit()`'s spacing is a stylistic per-call-site choice,
// not a function of token shape). Fixed by adding
// `TokenEvent::ControlWord::had_delimiter_space`, populated by the
// tokenizer and consumed verbatim by `Writer::write_event` instead of any
// heuristic — see `events.rs::test_events_had_delimiter_space` and
// `writer.rs::test_writer_byte_identical_delimiter_space`.
//
// The check below now exercises (1) directly: `sem_writer::Writer` fed by
// `events()`, compared byte-for-byte against `build()`, over every rtf
// fixture, plus the incrementality probe.
// ---------------------------------------------------------------------------

fn rtf_ast_to_events(doc: &rtf_fmt::RtfDoc) -> Vec<rtf_fmt::OwnedEvent> {
    let mut out = Vec::new();
    // events() always brackets the body in StartDocument/EndDocument;
    // StartDocument carries the exact font/color tables emit() computes
    // (rtf_fmt::build_font_map/build_color_map — the same functions, not a
    // re-derivation), so a byte-identical-to-build() streaming writer can
    // write the \fonttbl/\colortbl header immediately instead of buffering
    // the whole body to discover it. See sem_writer.rs's module doc.
    out.push(rtf_fmt::Event::StartDocument {
        fonts: rtf_fmt::build_font_map(doc),
        colors: rtf_fmt::build_color_map(doc),
    });
    for b in &doc.blocks {
        rtf_block_events(b, &mut out);
    }
    // events() emits TableOrderResolved (carrying the same already-known
    // tables as StartDocument, since the whole-document path has them up
    // front either way) right before EndDocument — see
    // rtf_fmt::sem_events::Event::TableOrderResolved's doc comment for why
    // this event exists (StreamingParser's incremental path needs it to
    // report the true first-use table once the whole body has been seen;
    // events() reports the same value twice for a uniform contract).
    out.push(rtf_fmt::Event::TableOrderResolved {
        fonts: rtf_fmt::build_font_map(doc),
        colors: rtf_fmt::build_color_map(doc),
    });
    out.push(rtf_fmt::Event::EndDocument);
    out
}

fn rtf_block_events(b: &rtf_fmt::Block, out: &mut Vec<rtf_fmt::OwnedEvent>) {
    use rtf_fmt::Block;
    use rtf_fmt::Event;
    match b {
        Block::Paragraph {
            inlines,
            align,
            para_props,
            ..
        } => {
            out.push(Event::StartParagraph {
                align: *align,
                para_props: para_props.clone().into(),
            });
            rtf_inline_events(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            rtf_inline_events(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { content, .. } => {
            out.push(Event::StartCodeBlock);
            out.push(Event::CodeBlockContent(content.clone().into()));
            out.push(Event::EndCodeBlock);
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for c in children {
                rtf_block_events(c, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for c in item {
                    rtf_block_events(c, out);
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
                    out.push(Event::StartTableCell);
                    rtf_inline_events(cell, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
    }
}

fn rtf_inline_events(inlines: &[rtf_fmt::Inline], out: &mut Vec<rtf_fmt::OwnedEvent>) {
    use rtf_fmt::Event;
    use rtf_fmt::Inline;
    for i in inlines {
        match i {
            Inline::Text { text, .. } => out.push(Event::Text(text.clone().into())),
            Inline::LineBreak { .. } => out.push(Event::LineBreak),
            Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
            Inline::Bold { children, .. } => {
                out.push(Event::StartBold);
                rtf_inline_events(children, out);
                out.push(Event::EndBold);
            }
            Inline::Italic { children, .. } => {
                out.push(Event::StartItalic);
                rtf_inline_events(children, out);
                out.push(Event::EndItalic);
            }
            Inline::Underline { children, .. } => {
                out.push(Event::StartUnderline);
                rtf_inline_events(children, out);
                out.push(Event::EndUnderline);
            }
            Inline::Strikethrough { children, .. } => {
                out.push(Event::StartStrikethrough);
                rtf_inline_events(children, out);
                out.push(Event::EndStrikethrough);
            }
            Inline::Code { text, .. } => out.push(Event::Code(text.clone().into())),
            Inline::Link { url, children, .. } => {
                out.push(Event::StartLink { url: url.clone() });
                rtf_inline_events(children, out);
                out.push(Event::EndLink);
            }
            Inline::Image { url, alt, .. } => out.push(Event::Image {
                url: url.clone(),
                alt: alt.clone(),
            }),
            Inline::Superscript { children, .. } => {
                out.push(Event::StartSuperscript);
                rtf_inline_events(children, out);
                out.push(Event::EndSuperscript);
            }
            Inline::Subscript { children, .. } => {
                out.push(Event::StartSubscript);
                rtf_inline_events(children, out);
                out.push(Event::EndSubscript);
            }
            Inline::FontSize { size, children, .. } => {
                out.push(Event::StartFontSize { size: *size });
                rtf_inline_events(children, out);
                out.push(Event::EndFontSize);
            }
            Inline::Color {
                r, g, b, children, ..
            } => {
                out.push(Event::StartColor {
                    r: *r,
                    g: *g,
                    b: *b,
                });
                rtf_inline_events(children, out);
                out.push(Event::EndColor);
            }
            Inline::AllCaps { children, .. } => {
                out.push(Event::StartAllCaps);
                rtf_inline_events(children, out);
                out.push(Event::EndAllCaps);
            }
            Inline::SmallCaps { children, .. } => {
                out.push(Event::StartSmallCaps);
                rtf_inline_events(children, out);
                out.push(Event::EndSmallCaps);
            }
            Inline::Hidden { children, .. } => {
                out.push(Event::StartHidden);
                rtf_inline_events(children, out);
                out.push(Event::EndHidden);
            }
            Inline::CharSpan {
                char_props,
                children,
                ..
            } => {
                out.push(Event::StartCharSpan {
                    char_props: char_props.clone(),
                });
                rtf_inline_events(children, out);
                out.push(Event::EndCharSpan);
            }
            Inline::Font { name, children, .. } => {
                out.push(Event::StartFont { name: name.clone() });
                rtf_inline_events(children, out);
                out.push(Event::EndFont);
            }
            Inline::BgColor {
                r, g, b, children, ..
            } => {
                out.push(Event::StartBgColor {
                    r: *r,
                    g: *g,
                    b: *b,
                });
                rtf_inline_events(children, out);
                out.push(Event::EndBgColor);
            }
            Inline::Lang { lcid, children, .. } => {
                out.push(Event::StartLang { lcid: *lcid });
                rtf_inline_events(children, out);
                out.push(Event::EndLang);
            }
            Inline::Footnote { content, .. } => {
                out.push(Event::StartFootnote);
                for c in content {
                    rtf_block_events(c, out);
                }
                out.push(Event::EndFootnote);
            }
        }
    }
}

#[test]
fn rtf_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("rtf");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/rtf dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = rtf_fmt::parse(&input);
        let expected = rtf_ast_to_events(&doc);
        let actual: Vec<_> = rtf_fmt::events(&input).collect();
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
        "expected to check a substantial number of rtf fixtures, got {checked}"
    );
}

/// `StreamingParser` buffers all fed bytes and only calls `sem_events::events()`
/// once, inside `finish()` (batch.rs:107-139) — so its output is byte-for-byte
/// identical to bulk `events()` regardless of chunking (the adversarial-chunking
/// equivalence check below always passes), but it is not a genuine incremental
/// parser: `feed()` alone (without `finish()`) never delivers anything to the
/// handler, which the incrementality probe below catches.
#[test]
fn rtf_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("rtf");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/rtf dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<rtf_fmt::OwnedEvent> = rtf_fmt::events(&input).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                rtf_fmt::batch::StreamingParser::new(|e: rtf_fmt::OwnedEvent| streamed.push(e));
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
        "expected to check a substantial number of rtf fixtures, got {checked}"
    );

    if result.is_ok() {
        let probe_input = br"{\rtf1\ansi\deff0 Hello world\par";
        let mut delivered: Vec<rtf_fmt::OwnedEvent> = Vec::new();
        let mut parser = rtf_fmt::batch::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("rtf", !delivered.is_empty());
    }
    assert_or_known_failure("rtf", "streaming_parser", result);
}

/// `sem_writer::Writer` is fed by `events()` directly (not a token
/// re-tokenization stand-in) and compared byte-for-byte against `build()`,
/// over every rtf fixture, plus an incrementality probe.
#[test]
fn rtf_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("rtf");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/rtf dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = rtf_fmt::parse(&input);
        let built = rtf_fmt::emit(&doc);

        let mut w = rtf_fmt::sem_writer::Writer::new(Vec::<u8>::new());
        for e in rtf_fmt::events(&input) {
            w.write_event(e);
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
        "expected to check a substantial number of rtf fixtures, got {checked}"
    );

    // Incrementality probe: the header (from StartDocument) is written and
    // flushed to the sink immediately, well before finish().
    if result.is_ok() {
        use rtf_fmt::OwnedEvent as Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = rtf_fmt::sem_writer::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartDocument {
            fonts: vec!["Times New Roman".to_string()],
            colors: vec![],
        });
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after StartDocument and before finish() \
                 — expected genuine incremental writes"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("rtf", "streaming_writer", result);
}
