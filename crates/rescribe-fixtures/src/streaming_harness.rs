//! Cross-API test infrastructure: reusable pieces every format's
//! `events()`/`StreamingParser`/streaming-writer fixture test instantiates.
//!
//! Historically `tests/run.rs` only ever drove `parse()` (reader) and
//! `emit()` (writer) — the rescribe adapter's two entry points. That left the
//! other three `{format}-fmt` APIs (`events()`, `StreamingParser<H>`, the
//! streaming writer) completely unexercised by the fixture suite, which let
//! real bugs (orphaned modules that never compiled, an `events()` that
//! silently drops content, a streaming writer that drops attributes) ship
//! for months, caught only when someone happened to hand-write the first-ever
//! test for that specific API. See `fixtures/spec.md` ("Cross-API harness")
//! for the full contract this module implements.
//!
//! This module provides three pieces of reusable machinery:
//!
//! 1. [`adversarial_chunkings`] — chunk splits for `StreamingParser` vs.
//!    `events()` equivalence tests.
//! 2. [`FormatCapabilities`] / [`ApiState`] / [`CAPABILITIES`] — the explicit,
//!    reviewable "does this format have API X" declaration.
//! 3. [`KnownFailure`] / [`KNOWN_FAILURES`] / [`assert_or_known_failure`] —
//!    the "this check is wired and currently fails, and that failure is
//!    tracked" mechanism, so the suite stays green without silently masking
//!    anything.
//!
//! What this module does **not** provide is a single generic "AST -> events
//! projection" type. Each format's AST and Event types are independent Rust
//! types with independent shapes (that's the whole point of the "-fmt crates
//! are not rescribe internals" architecture — see repository CLAUDE.md), so
//! there is no one projection type that fits all of them. The reusable
//! *pattern* is: construct the format's own `Event` sequence directly from
//! its own AST (a hand-written `ast_to_events` next to each fixture test),
//! then compare it against the real `events()` output with that format
//! crate's own `PartialEq` on `Event`. That gives *exact* equivalence, not
//! merely a lossy shape comparison — see `tests/streaming_apis.rs` for the
//! rst-fmt instantiation of this pattern.

// ---------------------------------------------------------------------------
// 1. Adversarial chunking
// ---------------------------------------------------------------------------

/// Adversarial ways to split `input` into chunks for feeding a
/// `StreamingParser`/`feed()`-style API, each paired with a short name for
/// failure messages.
///
/// Generalizes rst-fmt's original "6 chunk-splitting cases"
/// (`crates/formats/rst-fmt/src/batch.rs`) into reusable infrastructure any
/// format's fixture test can call. The adversarial intent: split mid-token,
/// mid-tag, mid-UTF-8-character, and one byte at a time — the ways a network
/// socket or a naive line-buffered reader would actually hand a parser its
/// input.
pub fn adversarial_chunkings(input: &[u8]) -> Vec<(&'static str, Vec<Vec<u8>>)> {
    let mut out = vec![
        ("whole", vec![input.to_vec()]),
        (
            "single_byte",
            input.iter().map(|b| vec![*b]).collect::<Vec<_>>(),
        ),
    ];
    for (name, n) in [
        ("chunks_of_3", 3usize),
        ("chunks_of_7", 7),
        ("chunks_of_13", 13),
    ] {
        if input.len() > n {
            out.push((name, input.chunks(n).map(|c| c.to_vec()).collect()));
        }
    }
    if let Some(pos) = mid_utf8_char_split_point(input) {
        out.push((
            "mid_utf8_char",
            vec![input[..pos].to_vec(), input[pos..].to_vec()],
        ));
    }
    out
}

/// Find a byte offset that lands strictly inside a multi-byte UTF-8
/// character's encoding (a continuation byte), so a chunk boundary there
/// tears the character in half. Returns `None` if `input` is pure ASCII.
fn mid_utf8_char_split_point(input: &[u8]) -> Option<usize> {
    input
        .iter()
        .position(|&b| (0b1000_0000..0b1100_0000).contains(&b))
}

// ---------------------------------------------------------------------------
// 2. Capability declaration
// ---------------------------------------------------------------------------

/// Status of one non-`parse()`/`emit()` API for one format, as declared by a
/// human reviewing the format's actual `-fmt` crate source.
///
/// `parse()` and builder `emit()` are excluded from this table: those are
/// already exercised by the pre-existing `run_format_fixtures`/
/// `run_format_writer_fixtures` tests in `tests/run.rs`. This table only
/// covers the three APIs that were previously never touched by the fixture
/// suite: `events()`, `StreamingParser<H>`, and the streaming writer.
#[derive(Debug, Clone, Copy)]
pub enum ApiState {
    /// The fixture suite has a real, passing, fixture-driven check for this
    /// API (not a stub, not a `parse()`-then-wrap fake).
    Wired,
    /// The API exists and is wired into a real check, but the check
    /// currently fails against a specific tracked bug. See
    /// [`KNOWN_FAILURES`] for the matching entry and `TODO.md` for the
    /// fuller writeup.
    KnownFailure(&'static str),
    /// The `{format}-fmt` crate structurally does not have this API, for a
    /// reason documented in `docs/format-audit.md` (e.g. csv/tsv/ris/native
    /// have no meaningful streaming writer; commonmark-fmt's
    /// `StreamingParser` is a sanctioned pulldown-cmark exemption per
    /// CLAUDE.md). This is the *only* path that may be used to mean "this
    /// check will never exist" — it must cite the documented reason, not be
    /// used to dodge writing a check that should exist.
    NotApplicable(&'static str),
    /// The API most likely exists in the `-fmt` crate (or its existence
    /// hasn't been confirmed) but this harness does not check it yet. This
    /// is an honest placeholder, not a claim of absence — see TODO.md for
    /// the tracked follow-up. Every format must have an explicit line here
    /// rather than simply not appearing in the harness at all.
    NotYetWired(&'static str),
}

/// Declared API status for one format, for the three APIs the harness adds
/// checks for beyond the pre-existing `parse()`/`emit()` tests.
#[derive(Debug, Clone, Copy)]
pub struct FormatCapabilities {
    pub format: &'static str,
    pub events: ApiState,
    pub streaming_parser: ApiState,
    pub streaming_writer: ApiState,
}

/// One entry per format tested in `tests/run.rs`. Absence of a format here
/// is a bug in the harness (checked by
/// `tests/streaming_apis.rs::every_run_rs_format_has_a_capability_entry`) —
/// the point of this table is that "not checked" must always be a line of
/// code someone wrote and can review, never silence.
pub const CAPABILITIES: &[FormatCapabilities] = &[
    FormatCapabilities {
        format: "rst",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "rst-fmt StreamingParser closes and reopens a multi-item DefinitionList as one \
             StartDefinitionList/EndDefinitionList pair per item, instead of one list spanning \
             all items the way events() produces — found while wiring this harness's \
             adversarial-chunking equivalence check across all rst fixtures (rst-fmt's own \
             pre-existing streaming tests only covered 6 hand-picked cases, none a multi-item \
             definition list); see TODO.md",
        ),
        streaming_writer: ApiState::Wired,
    },
    // html-fmt is html5ever-backed. CLAUDE.md puts third-party-library-backed
    // formats (pulldown-cmark, html5ever) out of scope for the "three
    // independently optimal reader APIs" mandate, and html-fmt does not fail
    // that mandate silently — it documents the reason in `batch.rs`'s module
    // docs and `lib.rs`'s crate docs (quoted in tests/streaming_apis.rs above
    // the html checks): the HTML5 spec mandates tree construction (foster
    // parenting, implied elements, adoption agency), so incremental event
    // delivery during `feed()` is not possible. The streaming writer is
    // independent code and is fully checked.
    FormatCapabilities {
        format: "html",
        events: ApiState::NotApplicable(
            "html-fmt's events() is `events_from_doc(&parse(input).0)` — a depth-first walk of \
             the html5ever-built tree into a Vec<OwnedEvent> (lib.rs:55, events.rs:92). An \
             events()-vs-AST-projection equivalence check would compare that walk against \
             itself and pass by construction, so wiring one would misrepresent html-fmt as \
             having an independent streaming reader. The derivation is documented, not \
             accidental: lib.rs's crate docs state \"All three reader APIs build the full parse \
             tree internally... This is a fundamental limitation of the HTML5 spec, not a \
             library choice\", and CLAUDE.md puts html5ever-backed formats out of scope",
        ),
        streaming_parser: ApiState::NotApplicable(
            "html-fmt's StreamingParser::feed() is a bare `buf.extend_from_slice(chunk)`; all \
             parsing and handler dispatch happen in finish() (batch.rs:100-110). batch.rs's \
             module docs state incremental event delivery \"is not possible without building \
             the full tree first\" because the HTML5 algorithm can rearrange previously-seen \
             nodes. The one property buffering can still get wrong — chunk-boundary integrity, \
             including mid-UTF-8-character splits — IS checked over every html fixture by \
             html_streaming_parser_buffering_survives_adversarial_chunking; that check is \
             deliberately not claimed as a Wired streaming_parser capability, because it \
             verifies buffering, not the incremental delivery html-fmt documents it cannot do",
        ),
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "docx",
        events: ApiState::KnownFailure(
            "ooxml-wml events() drops the Text event and reverses End-tag \
             order for the common <w:p><w:r><w:t> shape (no <w:pPr> before \
             the run) — a read_props()/queue() clobber bug found while \
             wiring this harness; see TODO.md",
        ),
        streaming_parser: ApiState::NotYetWired(
            "ooxml-wml::batch::BatchParser exists (crates/formats/ooxml-wml/src/batch.rs) \
             but is not yet exercised by this harness; a fixture-driven chunking check \
             over real docx document.xml parts is tracked follow-up work",
        ),
        streaming_writer: ApiState::NotYetWired(
            "ooxml-wml::streaming::WmlWriter exists and ooxml-pml's sibling writer has a \
             fixed, crate-level-tested fidelity bug (see the ooxml-sml precedent below), \
             but this harness does not yet drive WmlWriter against a byte-identical-to- \
             builder check — docx's builder path packages a full zip (content types, \
             rels, etc.), which needs a purpose-built minimal-package harness helper; \
             tracked follow-up work",
        ),
    },
    FormatCapabilities {
        format: "pptx",
        events: ApiState::KnownFailure(
            "ooxml-pml events() cannot reach slide text at all: dispatch_start() \
             has no entry for <p:txBody> (and nvSpPr/style), so it falls into \
             skip_element() and the whole subtree is dropped — open, documented \
             bug, see TODO.md. It also shares ooxml-wml's Text-drop / \
             End-tag-reversal queue() clobber bug once a txBody-reaching fix \
             lets it get as far as a paragraph/run.",
        ),
        streaming_parser: ApiState::NotYetWired(
            "ooxml-pml::batch exists but not yet exercised by this harness",
        ),
        streaming_writer: ApiState::NotYetWired(
            "ooxml-pml::streaming::PmlWriter's non-rectangular-shape fidelity bug was \
             already fixed and is pinned by a crate-level test, but this harness does not \
             yet drive it against a byte-identical-to-builder check for the same \
             full-zip-package reason as docx; tracked follow-up work",
        ),
    },
    FormatCapabilities {
        format: "xlsx",
        events: ApiState::Wired,
        streaming_parser: ApiState::NotYetWired(
            "ooxml-sml::batch exists but not yet exercised by this harness",
        ),
        streaming_writer: ApiState::Wired,
    },
];

/// Formats declared with an honest "not yet audited" placeholder: the
/// `-fmt` crate's `events()`/`StreamingParser`/streaming-writer status has
/// not been individually verified for these by this harness. This is
/// deliberately distinct from [`ApiState::NotApplicable`] (a verified,
/// documented structural absence) — it says only "nobody has looked yet",
/// which is itself a fact worth a reviewable line rather than silent
/// absence from the table. See the task report / TODO.md for the plan to
/// retire entries from this list into real `CAPABILITIES` rows.
pub const NOT_YET_AUDITED: &[&str] = &[
    "markdown",
    "commonmark",
    "gfm",
    "asciidoc",
    "mediawiki",
    "latex",
    "org",
    "creole",
    "djot",
    "textile",
    "muse",
    "t2t",
    "tikiwiki",
    "twiki",
    "vimwiki",
    "dokuwiki",
    "jira",
    "haddock",
    "pod",
    "man",
    "xwiki",
    "zimwiki",
    "bbcode",
    "texinfo",
    "markua",
    "fountain",
    "ansi",
    "csl-json",
    "native",
    "pandoc-json",
    "docbook",
    "fb2",
    "ipynb",
    "csv",
    "tsv",
    "opml",
    "ris",
    "bibtex",
    "biblatex",
    "typst",
    "jats",
    "endnotexml",
    "tei",
    "odt",
    "epub",
    "pdf",
    "rtf",
    "multimarkdown",
    "beamer",
    "revealjs",
    "slidy",
    "s5",
    "dzslides",
    "slideous",
    "context",
    "ms",
    "icml",
    "chunkedhtml",
];

// ---------------------------------------------------------------------------
// 3. Known-failures mechanism
// ---------------------------------------------------------------------------

/// A tracked, acknowledged failure of a specific format+API check.
///
/// Every entry here must correspond to a real, currently-reproducing bug —
/// never a blanket suppression. Each check wired against a `KNOWN_FAILURES`
/// entry runs for real; if it starts passing, `assert_or_known_failure`
/// panics telling the maintainer to delete the now-stale entry. That's the
/// anti-regression property: this list can only shrink by someone
/// confirming the fix, never grow silently.
#[derive(Debug, Clone, Copy)]
pub struct KnownFailure {
    pub format: &'static str,
    pub api: &'static str,
    pub description: &'static str,
}

pub const KNOWN_FAILURES: &[KnownFailure] = &[
    KnownFailure {
        format: "docx",
        api: "events",
        description: "ooxml-wml events() Text-drop / End-tag-reversal queue() clobber bug",
    },
    KnownFailure {
        format: "pptx",
        api: "events",
        description: "ooxml-pml events() cannot reach slide text (no txBody in dispatch_start) \
                       + shares the wml Text-drop/reversal bug",
    },
    KnownFailure {
        format: "rst",
        api: "streaming_parser",
        description: "rst-fmt StreamingParser splits a multi-item DefinitionList into one \
                       StartDefinitionList/EndDefinitionList pair per item instead of one list \
                       spanning all items",
    },
];

/// Run a check's `result` against the [`KNOWN_FAILURES`] table for
/// `format`/`api`.
///
/// - No matching entry, `Ok`: check passed cleanly, nothing to report.
/// - No matching entry, `Err`: a real, un-tracked failure — panics with the
///   error, since every failure must be either fixed or explicitly
///   acknowledged in `KNOWN_FAILURES`.
/// - Matching entry, `Err`: an acknowledged, tracked failure — prints a
///   visible "ACKNOWLEDGED KNOWN FAILURE" line (so `cargo test` output still
///   shows it) and returns without panicking.
/// - Matching entry, `Ok`: the bug no longer reproduces — panics telling the
///   maintainer to remove the stale entry, so fixed bugs can't silently keep
///   masking a check forever.
pub fn assert_or_known_failure(format: &str, api: &str, result: Result<(), String>) {
    let known = KNOWN_FAILURES
        .iter()
        .find(|k| k.format == format && k.api == api);
    match (known, result) {
        (Some(k), Err(e)) => {
            eprintln!(
                "ACKNOWLEDGED KNOWN FAILURE [{format}/{api}]: {}\n  (current failure: {e})",
                k.description
            );
        }
        (Some(k), Ok(())) => {
            panic!(
                "{format}/{api} check now PASSES, but is still listed in KNOWN_FAILURES as \
                 {:?}. Remove this entry from streaming_harness::KNOWN_FAILURES — a fixed bug \
                 must not keep masking future regressions.",
                k.description
            );
        }
        (None, Err(e)) => {
            panic!(
                "{format}/{api} check failed and is not in KNOWN_FAILURES: {e}\n\
                 Either fix the underlying bug, or add a KnownFailure entry describing it \
                 (see streaming_harness::KNOWN_FAILURES) — a failing check may never be \
                 silently ignored."
            );
        }
        (None, Ok(())) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunkings_cover_whole_and_single_byte() {
        let names: Vec<_> = adversarial_chunkings(b"hello world")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert!(names.contains(&"whole"));
        assert!(names.contains(&"single_byte"));
    }

    #[test]
    fn chunkings_reassemble_to_original_input() {
        let input = "hello \u{2192} world, \u{1F980} crab".as_bytes();
        for (name, chunks) in adversarial_chunkings(input) {
            let reassembled: Vec<u8> = chunks.into_iter().flatten().collect();
            assert_eq!(
                reassembled, input,
                "chunking {name} lost or reordered bytes"
            );
        }
    }

    #[test]
    fn mid_utf8_char_split_found_for_multibyte_input() {
        let input = "\u{1F980}".as_bytes(); // crab emoji, 4-byte UTF-8
        let names: Vec<_> = adversarial_chunkings(input)
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert!(names.contains(&"mid_utf8_char"));
    }

    #[test]
    fn known_failures_reference_declared_capabilities() {
        for kf in KNOWN_FAILURES {
            let cap = CAPABILITIES
                .iter()
                .find(|c| c.format == kf.format)
                .unwrap_or_else(|| {
                    panic!(
                        "KNOWN_FAILURES entry for {} has no CAPABILITIES row",
                        kf.format
                    )
                });
            let matches = matches!(
                (
                    kf.api,
                    cap.events,
                    cap.streaming_parser,
                    cap.streaming_writer
                ),
                ("events", ApiState::KnownFailure(_), _, _)
                    | ("streaming_parser", _, ApiState::KnownFailure(_), _)
                    | ("streaming_writer", _, _, ApiState::KnownFailure(_))
            );
            assert!(
                matches,
                "KNOWN_FAILURES entry {}/{} has no matching ApiState::KnownFailure in CAPABILITIES",
                kf.format, kf.api
            );
        }
    }
}
