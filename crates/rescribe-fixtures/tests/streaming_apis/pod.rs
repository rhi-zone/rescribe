//! Streaming-API cross-checks for pod. Split out of the former monolithic
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
// pod-fmt: top-level `pod_fmt::events()` (src/lib.rs) is `parse(input)` then
// an eager `.collect()` of `events::EventIter::new(&doc)` — a lazy
// frame-stack walk of the AST parse() already built (the same
// events()-is-parse()+AST-walk pattern already documented for t2t/asciidoc
// above), not an independently-implemented reader. The ast_to_events-vs-
// events() check below is real and passes, but validates the AST->event
// expansion layer, not two independent parsers.
//
// StreamingParser is explicitly self-documented buffer-then-finish: its own
// doc comment reads "POD documents are always small enough to buffer fully,
// so this implementation accumulates all input and parses on finish(). The
// chunk API is provided for interface consistency with other format
// crates." feed() only does `self.buf.extend_from_slice(chunk)`; all
// parsing and event delivery happens in finish() via
// `crate::events::EventIter::new(&doc)` over the *whole* buffered input (not
// per-isolated-block), so — unlike t2t/org/asciidoc — there is no
// re-parse-in-isolation divergence from events(): the adversarial-chunking
// equivalence check is expected to (and does) pass. The real defect is
// purely architectural non-incrementality, per CLAUDE.md's "the 'buffer all
// input until finish()' stub is explicitly rejected for hand-rolled
// parsers" — pod-fmt's own docstring rationale does not make this a
// sanctioned exemption (only commonmark-fmt's pulldown-cmark wrapping is);
// pinned via the incrementality probe.
//
// Writer buffers all fed events into a Vec<OwnedEvent> and only reconstructs
// the AST + calls emit::build() inside finish() (writer.rs's `finish()`:
// `events_to_doc(...)` then `crate::emit::build(&doc)`) — the same
// fake-streaming-writer pattern as t2t/textile/commonmark/org/texinfo. Since
// PodDoc has no document-level metadata field pod::Event could plausibly be
// missing (unlike t2t's title/author/date), the byte-identical-to-builder
// check is expected to (and does) pass; only the incrementality probe fails.
// ---------------------------------------------------------------------------

fn pod_ast_to_events(doc: &pod_fmt::PodDoc) -> Vec<pod_fmt::OwnedEvent> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        pod_block_events(b, &mut out);
    }
    out
}

fn pod_block_events(b: &pod_fmt::Block, out: &mut Vec<pod_fmt::OwnedEvent>) {
    use pod_fmt::{Block, Event};
    match b {
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            for i in inlines {
                pod_inline_events(i, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                pod_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock { content, .. } => out.push(Event::CodeBlock {
            content: content.clone().into(),
        }),
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item_blocks in items {
                out.push(Event::StartListItem);
                for b in item_blocks {
                    pod_block_events(b, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                for i in &item.term {
                    pod_inline_events(i, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for b in &item.desc {
                    pod_block_events(b, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::RawBlock {
            format, content, ..
        } => out.push(Event::RawBlock {
            format: format.clone(),
            content: content.clone(),
        }),
        Block::ForBlock {
            format, content, ..
        } => out.push(Event::ForBlock {
            format: format.clone(),
            content: content.clone(),
        }),
        Block::Encoding { encoding, .. } => out.push(Event::Encoding {
            encoding: encoding.clone(),
        }),
    }
}

fn pod_inline_events(i: &pod_fmt::Inline, out: &mut Vec<pod_fmt::OwnedEvent>) {
    use pod_fmt::{Event, Inline};
    match i {
        Inline::Text(s, _) => out.push(Event::Text(s.clone().into())),
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Code(s, _) => out.push(Event::InlineCode(s.clone().into())),
        Inline::Link { url, label, .. } => {
            out.push(Event::StartLink {
                url: url.clone(),
                label: label.clone(),
            });
            out.push(Event::EndLink);
        }
        Inline::Filename(children, _) => {
            out.push(Event::StartFilename);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndFilename);
        }
        Inline::NonBreaking(children, _) => {
            out.push(Event::StartNonBreaking);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndNonBreaking);
        }
        Inline::IndexEntry(s, _) => out.push(Event::IndexEntry(s.clone())),
        Inline::Null(_) => out.push(Event::Null),
        Inline::Entity(s, _) => out.push(Event::Entity(s.clone())),
    }
}

#[test]
fn pod_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("pod");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/pod dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = pod_fmt::parse(&input);
        let expected = pod_ast_to_events(&doc);
        let actual: Vec<_> = pod_fmt::events(&input).collect();
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
        "expected to check a substantial number of pod fixtures, got {checked}"
    );
}

/// `StreamingParser` was rewritten 2026-07-31 from buffer-all-bytes-then-parse-on-finish() to
/// a genuine line-buffered incremental block parser (see
/// `crates/formats/pod-fmt/src/batch.rs` and the `CAPABILITIES` entry in `streaming_harness.rs`
/// for the full design and measured peak-memory numbers). Checks (1) equivalence with
/// `events()` under adversarial chunking and (2) incremental delivery via a hand-built probe
/// with a guaranteed-complete prefix (see below) rather than an arbitrary byte-count split of
/// a real fixture — the pod fixture suite is overwhelmingly one block per fixture (one
/// heading, paragraph, or list per `fixtures/spec.md`'s single-construct convention), so a
/// fixed 50%-byte split of a real fixture routinely lands mid-block with zero events to
/// deliver, which says nothing about whether the implementation is incremental (the same
/// probe-methodology gap already fixed this session for fb2-fmt/texinfo/xwiki — see
/// `fb2_streaming_parser_matches_events_and_is_incremental`'s hand-built probe for the
/// precedent this one follows).
#[test]
fn pod_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("pod");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/pod dir") {
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
        let bulk: Vec<pod_fmt::OwnedEvent> = pod_fmt::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = pod_fmt::batch::StreamingParser::new(|e: pod_fmt::OwnedEvent| {
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

        // Deliberately NOT probed here: an arbitrary 50%-byte split of each real
        // fixture. That was this check's original design and it is
        // fixture-shape-unaware in exactly the way jats-fmt's own
        // streaming_parser KnownFailure documents: a 50% split of most pod
        // fixtures (single heading/paragraph/list documents) lands mid-block,
        // so zero events delivered at that exact split point is the correct,
        // spec-conforming answer, not a StreamingParser defect. See the
        // hand-built probe below instead, which guarantees an unambiguous
        // complete-prefix boundary.
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of pod fixtures, got {checked}"
    );

    // Incrementality probe: a hand-built input with a definite complete prefix
    // (a full `=head1` heading — a single-line block that flushes on its own
    // newline — followed by a full ordinary paragraph, which flushes on the
    // blank line after it) followed by deliberately unterminated trailing
    // content (a partial line with no trailing newline at all, so it never
    // reaches feed_line and stays buffered). Confirms events reach the handler
    // after feed() alone, before finish() is ever called — the property a
    // fixed 50%-byte split of a real fixture can't reliably demonstrate for
    // this format's fixture corpus.
    if result.is_ok() {
        let probe_input = b"=head1 Title\n\nComplete paragraph text.\n\n\
                             Unterminated trailing paragraph with no closing blank line";
        let mut delivered: Vec<pod_fmt::OwnedEvent> = Vec::new();
        let mut parser = pod_fmt::batch::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("pod", !delivered.is_empty());
    }
    assert_or_known_failure("pod", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<OwnedEvent>` and only
/// reconstructs the AST + calls `emit::build()` inside `finish()` (see
/// `crates/formats/pod-fmt/src/writer.rs`). Checked via byte-identical
/// comparison against the builder path, plus an incrementality probe.
#[test]
fn pod_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("pod");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/pod dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = pod_fmt::parse(&input);
        let built = pod_fmt::build(&doc);

        let mut w = pod_fmt::Writer::new(Vec::<u8>::new());
        for e in pod_fmt::events(&input) {
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
        "expected to check a substantial number of pod fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use pod_fmt::Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = pod_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text("Hello world".to_string().into()));
        w.write_event(Event::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — pod_fmt::writer::Writer buffers all \
                 events into a Vec<OwnedEvent> and only reconstructs the AST + calls \
                 emit::build() inside finish() (crates/formats/pod-fmt/src/writer.rs), so it is \
                 not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("pod", "streaming_writer", result);
}
