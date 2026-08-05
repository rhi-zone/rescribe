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
// html-fmt: events()/StreamingParser are documented tree-walk projections;
// the streaming Writer is independent code and gets a real byte-identical check
// ---------------------------------------------------------------------------
//
// html-fmt is html5ever-backed, and CLAUDE.md puts third-party-library-backed
// formats (pulldown-cmark, html5ever) out of scope for the "three independently
// optimal reader APIs" mandate. The crate does not merely fail that mandate
// silently — it documents the reason at module and crate level:
//
//   crates/formats/html-fmt/src/batch.rs (module docs):
//     "The HTML5 parsing algorithm requires tree construction for correctness —
//      the spec mandates operations like foster parenting, implied element
//      insertion, and adoption agency that can rearrange previously-seen nodes.
//      This means truly incremental event delivery (events emitted during
//      `feed()`) is not possible without building the full tree first."
//
//   crates/formats/html-fmt/src/lib.rs (crate docs):
//     "All three reader APIs build the full parse tree internally. `events()`
//      and `StreamingParser` walk the tree to produce events after
//      construction. This is a fundamental limitation of the HTML5 spec, not a
//      library choice."
//
// Concretely: `html_fmt::events()` is `events_from_doc(&parse(input).0)` — a
// depth-first walk of the finished tree into a `Vec<OwnedEvent>` — and
// `StreamingParser::feed()` is a bare `buf.extend_from_slice(chunk)` with all
// parsing and handler dispatch deferred to `finish()`. An
// "events() == ast_to_events(parse())" equivalence check would therefore be
// tautological (both sides are literally the same tree walk) and carry zero
// signal, which is why those two APIs are declared `NotApplicable` with the
// citations above rather than given a check that would pass by construction and
// misrepresent html-fmt as having independent streaming readers.
//
// Two checks below still carry real signal:
//
//  * the streaming `Writer` (`writer.rs`) writes bytes to its sink directly and
//    shares nothing with `emit.rs`'s `Emitter` except the two escaping helpers,
//    so byte-identity against `emit()` is a genuine cross-implementation check;
//  * `StreamingParser`'s chunk buffering is checked for byte-boundary
//    correctness (a mid-UTF-8-character split must not corrupt the buffer),
//    which is the one property `feed()` can actually get wrong.

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
        let (doc, _) = html_fmt::parse(&input);
        let built = html_fmt::emit(&doc);

        let mut w = html_fmt::Writer::new(Vec::<u8>::new());
        for e in html_fmt::events(&input) {
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

/// `StreamingParser` buffers all input and dispatches at `finish()` (see the
/// module comment above). The one property that buffering can still get wrong
/// is chunk-boundary handling, so this feeds every html fixture under the
/// adversarial chunkings — including a split landing inside a multi-byte UTF-8
/// character — and requires the delivered event sequence to equal `events()`
/// over the whole input at once.
///
/// This is deliberately *not* claimed as a `Wired` `streaming_parser`
/// capability: it verifies buffering integrity, not the incremental event
/// delivery html-fmt documents it cannot provide.
#[test]
fn html_streaming_parser_buffering_survives_adversarial_chunking() {
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
        let bulk: Vec<html_fmt::OwnedEvent> = html_fmt::events(&input).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                html_fmt::StreamingParser::new(|e: html_fmt::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            assert_eq!(
                bulk,
                streamed,
                "StreamingParser chunk buffering corrupted input for fixture {} under \
                 chunking {chunking_name}",
                path.display()
            );
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of html fixtures, got {checked}"
    );
}
