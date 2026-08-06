//! Streaming-API cross-checks for html. Split out of the former monolithic
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
// html-fmt: `events()`/`StreamingParser` are genuinely incremental (2026-08
// rework, replacing the earlier parse()-then-walk implementation)
// ---------------------------------------------------------------------------
//
// html-fmt is html5ever-backed, but unlike the general "third-party-library-
// backed formats are out of scope for the independent-APIs mandate" carve-out
// CLAUDE.md allows, html-fmt does not need it: `events()` and
// `StreamingParser` are driven directly by a custom `html5ever::TreeSink`
// (`html_fmt::sink::IncrementalSink`) that emits events as the
// tokenizer/tree-builder produce them, not after a full tree walk. HTML5's
// retroactive tree-construction operations (foster parenting, adoption
// agency) are handled via bounded correction events
// (`NodeReparented`/`ChildrenReparented`/`NodeDetached`) rather than by
// buffering the whole document — see `html-fmt`'s `sink`/`events` module
// docs for the verified html5ever 0.36.1 call-site trace this rests on.
//
// Because every content event carries its own [`html_fmt::NodeId`] plus an
// explicit parent (assigned at node-*creation* time, not at final-tree-
// position time), a real `events()` run and a hand-written "walk the
// resolved AST in document order" projection do **not** produce identical
// `Vec<Event>` even for a document with zero corrections — creation order
// and final sibling order can differ (foster parenting, absorbed via
// `before_sibling` positioning without any correction event, is enough to
// cause this: a foster-parented node's id is allocated after its later
// sibling's but attaches *before* it). So the meaningful equivalence check
// here is structural, matching the pattern html-fmt's own crate tests use
// (`smoke_events_incremental_matches_parse` in `lib.rs`): apply every event,
// including corrections, via `html_fmt::collect_doc` and compare the
// resulting tree to `parse()`'s, rather than a generic `Event`-list
// projection (`rescribe_fixtures::streaming_harness`'s module docs note this
// generic-projection pattern is the default, not the only valid shape — see
// there for why one size doesn't fit every format's event design).
//
// Four checks below:
//
//  * `collect_doc(events())` matches `parse()` over every fixture — the
//    reader-side genuine-incrementality claim, checked at fixture-suite
//    scale (html-fmt's own crate tests check this on a handful of
//    hand-written cases; this is the same check over the whole corpus);
//  * `StreamingParser` (chunk-fed) reconstructs the same tree as bulk
//    `events()`, under adversarial chunkings including a mid-UTF-8-character
//    split — a structural comparison, not `Vec<OwnedEvent>` equality, since
//    genuine incremental delivery legitimately splits text runs at finer
//    granularity when fed smaller chunks (see the test's own doc comment);
//  * `StreamingParser::feed()` delivers events to the handler before
//    `finish()` is called — the genuine-incrementality probe that would
//    catch a regression back to "buffer everything, dispatch at finish()";
//  * the streaming `Writer` (`writer.rs`) still gets its own byte-identical-
//    to-`emit()` check — independent code, unaffected by the reader-side
//    rework.

/// `collect_doc(events(input))` (i.e. the incremental reader's own output,
/// with every correction event applied) must reconstruct the exact same
/// tree as `parse()` (the non-streaming, html5ever-`RcDom`-backed path),
/// for every html fixture. This is the fixture-suite-scale version of the
/// claim `html-fmt`'s own crate tests
/// (`smoke_events_incremental_matches_parse`,
/// `foster_parenting_incremental_matches_parse_no_corrections`,
/// `adoption_agency_fires_correction_events_and_matches_parse`) check on a
/// handful of hand-written adversarial cases.
#[test]
fn html_events_incremental_matches_parse_over_all_fixtures() {
    let root = fixtures_root().join("html");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/html dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc1, _) = html_fmt::HtmlDoc::parse(&input);
        let evts: Vec<_> = html_fmt::HtmlDoc::events(&input).collect();
        let doc2 = html_fmt::collect_doc(evts);

        checked += 1;
        if doc1.strip_spans() != doc2.strip_spans() && result.is_ok() {
            result = Err(format!(
                "incremental events() did not reconstruct the same tree as parse() for fixture \
                 {}",
                path.display(),
            ));
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of html fixtures, got {checked}"
    );
    assert_or_known_failure("html", "events", result);
}

/// `StreamingParser` (chunk-fed) must reconstruct the exact same tree as
/// bulk `events()` over the whole input, under every adversarial chunking —
/// including a split landing inside a multi-byte UTF-8 character.
///
/// This is a structural comparison (`collect_doc`), not a `Vec<OwnedEvent>`
/// equality check: genuine incremental delivery means text-run granularity
/// tracks *feed()* granularity, not just tokenizer-internal chunking — a
/// text node fed one byte at a time legitimately arrives as one `Text` plus
/// several `TextAppended` events rather than a single merged `Text` (see
/// `IncrementalEventIter`'s own `DEFAULT_CHUNK_SIZE`-driven feeding in
/// `crate::sink`, which is coarser than `single_byte` chunking and so
/// produces fewer, larger events for the same content). That's the correct
/// behavior for a real streaming reader — a `Vec<OwnedEvent>` identity check
/// would be the wrong invariant and was replaced with this one after
/// `adv-deeply-nested` demonstrated the legitimate split under
/// `single_byte` chunking. Node ids, however, are stable regardless of
/// chunking (allocation is a function of token processing order only), so
/// `collect_doc`'s parent/child reconstruction is unaffected either way.
#[test]
fn html_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("html");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/html dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<html_fmt::OwnedEvent> = html_fmt::HtmlDoc::events(&input).collect();
        let bulk_doc = html_fmt::collect_doc(bulk);
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                html_fmt::StreamingParser::new(|e: html_fmt::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            let streamed_doc = html_fmt::collect_doc(streamed);
            assert_eq!(
                bulk_doc.strip_spans(),
                streamed_doc.strip_spans(),
                "StreamingParser reconstructed a different tree than events() for fixture {} \
                 under chunking {chunking_name}",
                path.display()
            );
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of html fixtures, got {checked}"
    );
}

/// `StreamingParser::feed()` must deliver events to the handler as soon as
/// they're available, not buffer everything until `finish()`. Feeds a
/// complete-prefix-then-incomplete-trailing-content input (an open `<p>`
/// with no closing tag or end-of-document) and checks the handler already
/// received events before `finish()` is ever called — the probe that would
/// catch a regression back to the old "parse the whole buffer at
/// `finish()`" implementation.
#[test]
fn html_streaming_parser_feed_is_incremental() {
    let delivered = std::rc::Rc::new(std::cell::RefCell::new(Vec::<html_fmt::OwnedEvent>::new()));
    let delivered_for_handler = delivered.clone();
    let mut parser = html_fmt::StreamingParser::new(move |e: html_fmt::OwnedEvent| {
        delivered_for_handler.borrow_mut().push(e)
    });
    parser.feed(b"<html><body><p>hello world, this element is deliberately left unclosed");
    let delivered_before_finish = !delivered.borrow().is_empty();
    parser.finish();
    assert_streaming_parser_is_incremental("html", delivered_before_finish).unwrap();
}

/// The streaming `Writer` must produce byte-identical output to builder
/// `emit()` over every html fixture.
///
/// `emit()`'s default `EmitOptions` is non-pretty, which is the mode `Writer`
/// implements (it has no pretty-printing path at all), so this compares the
/// two independent serializers on equal terms.
#[test]
fn html_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("html");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/html dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _) = html_fmt::HtmlDoc::parse(&input);
        let built = doc.emit();

        let mut w = html_fmt::Writer::new(Vec::<u8>::new());
        for e in html_fmt::HtmlDoc::events(&input) {
            w.write_event(e);
        }
        let streamed = w.finish();

        // Count as checked as soon as both serializations exist, not only when
        // they match — `checked` is a coverage floor, not a pass counter (see
        // the rst StreamingParser test for why gating it on success makes the
        // floor depend on `read_dir` ordering).
        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from emit() for fixture {}:\n  emit():  {}\n  \
                 Writer: {}",
                path.display(),
                String::from_utf8_lossy(&built),
                String::from_utf8_lossy(&streamed),
            ));
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of html fixtures, got {checked}"
    );
    assert_or_known_failure("html", "streaming_writer", result);
}
