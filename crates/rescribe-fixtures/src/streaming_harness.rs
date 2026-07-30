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
// 1b. Observable sink — probing streaming-writer incrementality
// ---------------------------------------------------------------------------

/// A `Write` sink that exposes the bytes written so far via a shared handle.
///
/// Byte-identical-to-builder comparisons (the primary streaming-writer
/// equivalence check; see [`adversarial_chunkings`]'s sibling checks in
/// `tests/streaming_apis.rs`) only prove the *final* output is correct — a
/// writer that buffers every event internally and does all real work inside
/// `finish()` can still pass that check while being architecturally a fake
/// streaming writer per CLAUDE.md ("a wrapper that funnels everything
/// through the tree builder is a fake streaming API"). `ObservableSink` lets
/// a test additionally check *when* bytes reach the sink: write several
/// events, inspect `pre_finish` bytes before calling `finish()`, and compare
/// to the post-finish length. A genuine incremental writer emits some bytes
/// before `finish()`; a buffer-then-emit wrapper emits zero.
pub struct ObservableSink(pub std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

impl std::io::Write for ObservableSink {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
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
    // djot-fmt's events() and parse() are genuinely independent implementations
    // (direct recursive descent vs a line-driven frame-stack state machine), so
    // the equivalence check compares two real code paths.
    FormatCapabilities {
        format: "djot",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "djot-fmt StreamingParser diverges from events() on 6 of 79 fixtures, via four \
             distinct bugs in batch.rs, none sanctioned by any doc comment. (a) BlockState::InDiv \
             is a flag, not a counter: the InDiv arm tests `trimmed == \":::\"` for the close, so \
             a nested `::: level2` opener is accumulated as content while the first bare `:::` \
             ends the whole block — and because the Between arm's opener test is \
             `starts_with(\":::\")`, every leftover closer is then misread as a NEW div opener, \
             so emit_block() re-parses `\":::\\n:::\"` as a spurious empty div. adv-nested-divs \
             (20 openers/closers) gains exactly 20 events, path-deep-divs (100/100) exactly 100. \
             This also falsifies the StreamingParser doc comment at batch.rs:80-84 claiming div \
             blocks are buffered until their closing marker. (b) link-reference: emit_block() \
             re-parses each block in isolation via crate::events(), so EventIter::new's \
             pre_scan() (parse.rs:1048) never sees a [label]: url definition living in a \
             different block and the reference resolves to url: \"\". (c) block-attr-on-code: the \
             fence-open branch of feed_line flushes a pending `{.python}` block-attribute line as \
             its own block before starting InFencedCode, so it becomes a pending_attr dropped at \
             EOF and the fence gets classes: []. The same flush exists in the `:::` branch. \
             (d) definition-list, e2e-rich: the blank-line arm of feed_line unconditionally calls \
             emit_block(), splitting a multi-item definition list into one \
             StartDefinitionList/EndDefinitionList pair per item — the same bug class already \
             tracked for rst-fmt, though here only the top-level blank-line boundary is at fault \
             (parse_definition_list_direct itself is fine). See TODO.md",
        ),
        streaming_writer: ApiState::KnownFailure(
            "djot-fmt's Writer drops link-reference definitions (fixture link-reference). Two \
             confirmed halves: the Event enum has no variant corresponding to LinkDef — \
             events()/EventIter keeps link defs in iter.link_defs and only surfaces them via \
             collect_doc_from_iter (events.rs:268), reaching into the iterator's field, a channel \
             Writer does not have since write_event only receives Event values — and in \
             writer.rs DocBuilder.link_defs is declared (line 180), initialized to vec![] (188) \
             and moved into the reconstructed DjotDoc (781) but never pushed to anywhere in the \
             file, so events_to_doc always returns link_defs: []. Footnotes do NOT have this \
             problem: StartFootnoteDef/EndFootnoteDef exist and DocBuilder::process handles them \
             at writer.rs:477-491. Note Writer is also not incrementally streaming — writer.rs's \
             module docs admit it buffers all events, reconstructs the AST, then emits. See \
             TODO.md",
        ),
    },
    // asciidoc's `events = Wired` is a narrower claim than rst's: `parse()`
    // (parse.rs:15) drives the same `try_parse_block()` loop `events()` does, so
    // the equivalence check validates the AST->event *expansion* layer
    // (expand_block/expand_inline/Frame unwinding), not two independent parsers.
    // See the comment above the check in tests/streaming_apis.rs.
    FormatCapabilities {
        format: "asciidoc",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "asciidoc StreamingParser diverges from events() on 8 of 85 fixtures, via three \
             distinct bugs in batch.rs — none sanctioned by any doc comment (batch.rs and lib.rs \
             make only an O(largest block) memory claim, and the crate's own \
             test_streaming_matches_bulk asserts exact parity with events(), so parity is the \
             crate's own stated contract). (a) feed_line treats a delimited-block marker as a \
             hard block boundary and flushes accumulated lines first, so a preceding \
             [source,...]/[verse]/[stem]/[EXAMPLE] attribute line or .Block Title line is \
             re-parsed by emit_block() as an isolated chunk; parse_block_with_attributes then \
             hits EOF and falls back to an empty collect_paragraph_content() (emitting \
             CodeBlockContent(\"\"), MathBlock{content:\"\"} or an empty paragraph), while \
             parse_block_title returns None so the title is dropped and the following block gets \
             title: None — 6 fixtures. Note the delimited-block open/close detection itself is \
             correct; this is not a delimiter-matching bug. (b) is_delimited_block_marker \
             (batch.rs:204) only matches runs of identical characters, so the table delimiter \
             |=== is unrecognized, StreamingParser never enters InDelimitedBlock for a table, \
             and the blank line between header and body splits it — parse_table sees one row \
             group and emits is_header: false while the rest degrades to a paragraph \
             (table-header). (c) StartDocument is only emitted from inside emit_block(), which \
             early-returns on empty block_lines, and finish() gates EndDocument on `started`, so \
             empty input yields zero events instead of the StartDocument/EndDocument pair \
             events(\"\") produces (adv-empty). See TODO.md",
        ),
        streaming_writer: ApiState::Wired,
    },
    // org-fmt's events() is genuinely independent of parse() — the dependency
    // runs the other way (`parse()` drives `EventIter::parse_next_block()`), so
    // the events-vs-AST-projection check compares two real code paths.
    FormatCapabilities {
        format: "org",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "org-fmt StreamingParser diverges from events() on 3 of 89 org fixtures, via three \
             distinct previously-unknown bugs in batch.rs's feed_line/BlockState machine — none \
             covered by the two exceptions batch.rs's module docs sanction (loose lists, drawers \
             containing blank lines), as none of the three fixtures has a loose list or a \
             drawer. (a) blockquote-nested: BlockState::InSpecialBlock stores only a single \
             expected end keyword with no nesting depth, so a nested #+BEGIN_QUOTE's #+END_QUOTE \
             closes the outer block early — parse.rs:521 tracks begin_marker depth for exactly \
             this reason and the streaming path simply lacks it. (b) code-block-name: feed_line \
             calls emit_block() unconditionally before entering a #+BEGIN_ block, so a preceding \
             affiliated #+NAME: line is re-parsed alone (setting pending_name with no following \
             block) and the code block emits name: None. (c) integration-list-code: feed_line \
             trims the line before its #+BEGIN_ test, so an indented code block inside a list \
             item reads as a top-level block start and the item is split from its child. All \
             three are downstream of emit_block() (batch.rs:190) re-parsing each accumulated \
             block in isolation; see TODO.md",
        ),
        // streaming_writer was KnownFailure: Event had no document-metadata
        // variant, so events() couldn't carry #+TITLE:/#+AUTHOR:/#+CUSTOM_KEY:
        // lines and writer.rs's DocBuilder::finish hardcoded `metadata:
        // vec![]`, dropping every leading keyword line (fixtures metadata,
        // keyword-line). Fixed by adding `Event::Metadata { key, value }`,
        // emitted by EventIter::next() (parse.rs) alongside the block it
        // precedes, and handled by DocBuilder::process/finish (writer.rs).
        // Note Writer is still not incrementally streaming — writer.rs's
        // module docs state it buffers all events, reconstructs the AST,
        // then calls emit::build.
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
    FormatCapabilities {
        format: "texinfo",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "texinfo::batch::StreamingParser buffers all fed bytes into a Vec<u8> and only \
             parses + delivers events inside finish() (see crates/formats/texinfo/src/batch.rs's \
             own module doc, \"Memory usage is O(full input)\"); feed() never advances real \
             parser state, so no events reach the handler until finish() is called — not \
             incremental streaming despite implementing the feed/finish contract; found while \
             wiring this harness's incrementality probe",
        ),
        streaming_writer: ApiState::KnownFailure(
            "texinfo::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls emit() inside finish() (see crates/formats/texinfo/src/\
             writer.rs's own module doc, \"buffers all events, reconstructs the AST, then \
             emits\"); zero bytes reach the sink before finish() despite content round-tripping \
             correctly (including @settitle, now carried via events::Event::Title — see \
             fixtures/texinfo/settitle-header)",
        ),
    },
    FormatCapabilities {
        format: "fb2",
        events: ApiState::KnownFailure(
            "fb2_fmt events()/EventIter silently drops the Event::Metadata event entirely \
             whenever the input has no literal <description> element — finalize_description() \
             (crates/formats/fb2-fmt/src/events.rs) only fires on a </description> close tag, \
             while parse()'s AST always carries a (possibly-default) FictionBook.description; \
             affects the majority of single-construct fb2 fixtures, which omit <description> \
             for brevity — found via this harness's ast_to_events projection check",
        ),
        streaming_parser: ApiState::KnownFailure(
            "fb2_fmt::StreamingParser buffers all fed bytes into a Vec<u8> and only parses + \
             delivers events inside finish() (see crates/formats/fb2-fmt/src/events.rs's \
             StreamingParser::finish, which calls events(&self.buf) — feed() itself just \
             extends the buffer); despite the crate's own events()/EventIter being a genuine \
             incremental quick_xml pull parser, StreamingParser does not reuse that \
             incrementality — feed() delivers zero events to the handler before finish()",
        ),
        streaming_writer: ApiState::KnownFailure(
            "the streaming Writer itself is genuinely incremental (write_event() writes \
             straight to the underlying quick_xml::Writer<W>), but it is fed by events(), which \
             (see the events KnownFailure above) never delivers Metadata for input lacking a \
             literal <description>; the Writer then never emits a <description> element, while \
             the AST builder path (emit()) always writes one — a downstream consequence of the \
             events() gap, not an independent Writer defect",
        ),
    },
    FormatCapabilities {
        format: "textile",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "textile_fmt::batch::StreamingParser buffers all fed bytes into a Vec<u8> and only \
             parses + delivers events inside finish() (see crates/formats/textile-fmt/src/\
             batch.rs's own module doc, \"It also buffers all input ... so memory is likewise \
             O(full input)\"); feed() never advances real parser state, so no events reach the \
             handler until finish() is called",
        ),
        streaming_writer: ApiState::KnownFailure(
            "textile_fmt::writer::Writer buffers all fed events into a Vec<TextileEvent> and \
             only reconstructs the AST + calls emit() inside finish() (see \
             crates/formats/textile-fmt/src/writer.rs's own module doc, \"buffers all events, \
             reconstructs the AST, then emits\") — a fake streaming writer per CLAUDE.md, not an \
             independent incremental implementation",
        ),
    },
    FormatCapabilities {
        format: "commonmark",
        events: ApiState::KnownFailure(
            "commonmark_fmt events()/EventIter has two distinct, real divergences from \
             parse()'s AST, both found via this harness's ast_to_events projection check: (1) \
             for images, EventIter's \"drain pending first\" loop structure \
             (crates/formats/commonmark-fmt/src/events.rs's Iterator::next) returns the \
             buffered alt Text event on the very next next() call issued while still inside the \
             PdEvent::Text match arm — i.e. before the matching TagEnd::Image is even reached — \
             so the real event order is Text(alt), StartImage, EndImage instead of the \
             documented StartImage, Text(alt), EndImage (reproduced with \
             fixtures/commonmark/image: real output is \"Alt text![Alt text](photo.jpg)\", \
             duplicating the alt text outside the image markup); (2) events() forwards \
             pulldown-cmark's raw Text events unmerged, while parse() deliberately coalesces \
             consecutive Inline::Text nodes (src/parse.rs's push_inline, comment \"pulldown-cmark \
             can split a single logical text run into multiple Text events\") — e.g. a broken \
             link like \"[text\" parses to one Text node but events() yields three separate Text \
             events for the same run (fixtures/commonmark/adv-broken-link)",
        ),
        streaming_parser: ApiState::NotApplicable(
            "commonmark-fmt's StreamingParser buffering all input before parsing with \
             pulldown-cmark is the sole documented CLAUDE.md exemption (pulldown-cmark requires \
             the full input as &str); see crates/formats/commonmark-fmt/src/lib.rs's \
             \"Limitations\" doc comment and src/batch.rs's own \"# Limitation\" section",
        ),
        streaming_writer: ApiState::KnownFailure(
            "two independent findings: (1) commonmark_fmt::writer::Writer buffers all fed events \
             into a Vec<OwnedEvent> and only reconstructs the AST + calls emit() inside finish() \
             — explicitly self-admitted in crates/formats/commonmark-fmt/src/writer.rs's own \
             module doc: \"the internal implementation is buffer-then-emit for correctness \
             (reuses the proven emit() path)\" — unrelated to the sanctioned pulldown-cmark \
             StreamingParser exemption (the writer never touches pulldown-cmark) and a fake \
             streaming writer per CLAUDE.md; (2) for fixtures/commonmark/image specifically, the \
             byte-identical-to-builder check fails as a downstream consequence of the events() \
             Text/StartImage ordering bug (see the commonmark/events KnownFailure) — the leaked \
             Text(alt) arrives before Frame::Image is pushed, so the Writer's discard-inside-\
             image logic never sees it and it is emitted as ordinary paragraph text, producing \
             \"Alt text![Alt text](photo.jpg)\" instead of \"![Alt text](photo.jpg)\"",
        ),
    },
    FormatCapabilities {
        format: "gfm",
        events: ApiState::KnownFailure(
            "shares commonmark-fmt with the \"commonmark\" format entry above; same \
             unmerged-Text-events-vs-coalesced-AST defect applies",
        ),
        streaming_parser: ApiState::NotApplicable(
            "shares commonmark-fmt with the \"commonmark\" format entry above; same sanctioned \
             pulldown-cmark StreamingParser exemption applies",
        ),
        streaming_writer: ApiState::KnownFailure(
            "shares commonmark-fmt with the \"commonmark\" format entry above; same \
             buffer-then-emit streaming writer defect applies",
        ),
    },
    FormatCapabilities {
        format: "markdown",
        events: ApiState::KnownFailure(
            "shares commonmark-fmt with the \"commonmark\" format entry above; same \
             unmerged-Text-events-vs-coalesced-AST defect applies",
        ),
        streaming_parser: ApiState::NotApplicable(
            "shares commonmark-fmt with the \"commonmark\" format entry above; same sanctioned \
             pulldown-cmark StreamingParser exemption applies",
        ),
        streaming_writer: ApiState::KnownFailure(
            "shares commonmark-fmt with the \"commonmark\" format entry above; same \
             buffer-then-emit streaming writer defect applies",
        ),
    },
    // docbook-fmt, jats-fmt, tei-fmt are byte-identical in implementation
    // shape (verified via `diff` across batch.rs/writer.rs: only doc
    // comments and AST/event type names differ), so the same three real bugs
    // — found by this harness, not present in any prior audit — apply to all
    // three identically. All three ARE genuinely independent/incremental
    // implementations (events() pulls tokens straight off quick_xml::Reader
    // without building an AST; the streaming Writer calls quick_xml::Writer
    // directly per event with no buffering) — these are logic bugs, not
    // architectural hollowness.
    FormatCapabilities {
        format: "docbook",
        events: ApiState::KnownFailure(
            "events()/EventIter emits one Text event per resolved character/predefined entity \
             (e.g. `&amp;` decodes to its own Text(\"&\") event) instead of merging it into the \
             surrounding text run the way parse()'s AST does (parse.rs's `current_text` \
             accumulator, documented in parse.rs's module doc as deliberate coalescing) — found \
             via this harness's events()-vs-events_from_doc(&parse()) equivalence check \
             (fixture adv-entity-references, 3/110 docbook fixtures contain entities)",
        ),
        streaming_parser: ApiState::KnownFailure(
            "StreamingParser's drain() (batch.rs) sets check_end_names=false and \
             allow_unmatched_ends=true on its per-drain-call quick_xml::Reader — an \
             architecturally necessary consequence of only ever seeing the unconsumed tail, \
             not the full document, on each call (documented in batch.rs's drain() comment) — \
             so it silently accepts a mismatched end tag (e.g. an unclosed <para> closed by \
             </article>) that events()'s single continuous Reader (default \
             check_end_names=true) correctly rejects as a parse error; the two diverge on \
             malformed input with zero diagnostics from StreamingParser (fixture \
             adv-malformed-xml) instead of matching events()'s error-and-stop behavior",
        ),
        streaming_writer: ApiState::KnownFailure(
            "downstream of the events() gap above: parse() has explicit malformed-XML recovery \
             (auto-closes unclosed elements with synthetic EndElement nodes, see parse.rs's \
             'unclosed element' diagnostics) so build() always emits well-formed output, but \
             events()/EventIter has no such recovery — it just stops at the same parse error \
             with no synthetic close events — so the streaming Writer (fed by events()) emits \
             truncated, unclosed XML for the same malformed input build() recovers from \
             (fixture adv-malformed-xml)",
        ),
    },
    FormatCapabilities {
        format: "jats",
        events: ApiState::KnownFailure(
            "shares docbook-fmt's implementation (byte-identical batch.rs/events.rs shape): \
             events()/EventIter does not coalesce consecutive resolved-entity Text events into \
             one Text run the way parse()'s AST does; see the \"docbook\" events KnownFailure \
             for the root cause",
        ),
        streaming_parser: ApiState::KnownFailure(
            "shares docbook-fmt's implementation: StreamingParser's per-drain-call Reader \
             disables check_end_names/allow_unmatched_ends and so does not detect mismatched \
             end tags events() catches; see the \"docbook\" streaming_parser KnownFailure",
        ),
        streaming_writer: ApiState::KnownFailure(
            "shares docbook-fmt's implementation: the streaming Writer inherits events()'s lack \
             of parse()'s malformed-XML auto-close recovery; see the \"docbook\" \
             streaming_writer KnownFailure",
        ),
    },
    // ansi-fmt: events()/StreamingParser/Writer are all genuinely
    // independent, incremental implementations (EventIter advances its own
    // position per next() call; StreamingParser::drain_complete only
    // re-parses the safe, provably-complete prefix of its buffer; Writer
    // writes straight to the sink per event) — not architecturally hollow.
    // But this harness's own checks found two real, previously-undocumented
    // logic bugs while wiring streaming_parser/streaming_writer (fixture
    // adv-unknown-sgr).
    FormatCapabilities {
        format: "ansi",
        events: ApiState::NotYetWired(
            "no events()-vs-AST-projection check is wired: parse()'s AnsiNode has no variant \
             for a bare SGR sequence at all — apply_sgr() (parse.rs) folds SGR codes into a \
             running `style` variable and returns (None, pos), so a run of SGR codes not \
             immediately followed by text produces zero AST nodes, while events()'s EventIter \
             unconditionally emits one SetStyle/ResetStyle event per 'm'-terminated CSI group \
             regardless of what follows — the reverse of the usual defect shape (here parse()'s \
             own AST is the lossier side), so there is no way to reconstruct a faithful \
             ast_to_events projection purely from the AST. Separately, and found via the \
             streaming_parser/streaming_writer checks below without needing that projection: \
             events.rs's parse_csi_event 'm' arm emits ResetStyle whenever the resulting style \
             is empty (`self.style.is_empty() && !params.is_empty()`), conflating \"style ended \
             up empty\" with \"an explicit reset code was seen\" — an unrecognized/no-op SGR \
             group (e.g. \\x1b[999m) that leaves style unchanged still emits a spurious \
             ResetStyle event",
        ),
        streaming_parser: ApiState::KnownFailure(
            "two compounding bugs, found via adversarial-chunking equivalence against events(): \
             (1) StreamingParser::drain_complete (batch.rs) calls crate::events::events(&to_parse) \
             — a brand-new EventIter with style: Style::default() — on every drain, so the \
             running style state does not persist across multiple drain_complete() calls; a \
             chunk boundary between an SGR sequence and the text it colors loses that style \
             under fine-grained chunking (fixture adv-unknown-sgr fails under \"single_byte\" \
             chunking specifically, not \"whole\", pinning the cross-call state loss). (2) \
             inherits events()'s spurious-ResetStyle-on-unrecognized-SGR bug (see the \"ansi\" \
             events entry)",
        ),
        streaming_writer: ApiState::KnownFailure(
            "downstream of events()'s spurious-ResetStyle-on-unrecognized-SGR bug (see the \
             \"ansi\" events entry): Writer's ResetStyle handler unconditionally writes the \
             literal \\x1b[0m bytes for every ResetStyle event it receives, so the spurious \
             event becomes spurious visible output — build() emits \"Text\\n\" for fixture \
             adv-unknown-sgr (parse()'s AST has no node for the unrecognized SGR at all) while \
             the streaming Writer emits \"\\x1b[0mText\\x1b[0m\\n\"",
        ),
    },
    // odf-fmt backs odt/ods/odp. events() is a genuine independent
    // implementation (direct quick_xml scan of content.xml, not a
    // parse()-then-walk fake — correcting a prior assessment) but is
    // eagerly, fully buffered before the first next() call (self-documented
    // in events.rs), so it is not memory-bounded; no StreamingParser<H>
    // exists yet (batch.rs module doc calls it "future" work). The streaming
    // Writer genuinely builds its AST incrementally per event (same
    // sanctioned shape as ooxml-sml's SmlWriter, deferring only ZIP byte
    // packaging to finish()) but OdfEvent has no variant for mimetype/
    // meta/styles/images, so round-tripping through it always drops them.
    FormatCapabilities {
        format: "odt",
        events: ApiState::NotYetWired(
            "odf-fmt's events()/EventIter (crates/formats/odf-fmt/src/events.rs) is a real, \
             independent quick_xml scan of content.xml, not a parse()-then-walk fake — but it \
             eagerly buffers every event into a VecDeque inside EventIter::new before the first \
             next() call (self-documented: \"Events are pre-buffered... For large files \
             consider... a future StreamingParser\"), so it is not memory-bounded. This harness \
             does not yet have a fixture-driven events()-vs-parse() equivalence check for it: \
             OdfEvent spans three sibling document-body shapes (text/spreadsheet/presentation) \
             and a faithful hand-written ast_to_events projection is substantial follow-up work",
        ),
        streaming_parser: ApiState::NotYetWired(
            "no StreamingParser<H> type exists in odf-fmt at all yet — batch.rs only has \
             BatchParser (a legitimate buffer-until-finish AST builder, since ODF's ZIP central \
             directory lives at the end of the file) and Writer; batch.rs's own module doc \
             calls a true chunked event-delivering parser \"a future StreamingParser\"",
        ),
        streaming_writer: ApiState::KnownFailure(
            "odf_fmt::batch::Writer::write_event() genuinely builds an OdfDocument \
             incrementally per event via DocBuilder::process (same sanctioned shape as \
             ooxml-sml's SmlWriter — real per-event work, only ZIP byte packaging deferred to \
             finish(), which is inherent to ZIP's central-directory-at-end layout) — but \
             OdfEvent has no variant carrying the document's mimetype, meta (title/author/\
             date), styles.xml, or embedded image bytes; events() only covers body content \
             (paragraphs, tables, slides), so the reconstructed OdfDocument always has \
             mimetype: \"\" (never set anywhere in batch.rs) and default/empty meta/styles/\
             images, unlike parse()'s AST which reads all of them from the ZIP's other parts — \
             an Event-enum expressiveness gap, the same defect class as org-fmt's missing- \
             metadata-variant gap, found via this harness's byte-identical-to-builder check \
             (fixture adv-corrupt-image: built 1676 bytes vs streamed 1440 bytes)",
        ),
    },
    FormatCapabilities {
        format: "tei",
        events: ApiState::KnownFailure(
            "shares docbook-fmt's implementation (byte-identical batch.rs/events.rs shape): \
             events()/EventIter does not coalesce consecutive resolved-entity Text events into \
             one Text run the way parse()'s AST does; see the \"docbook\" events KnownFailure \
             for the root cause",
        ),
        streaming_parser: ApiState::KnownFailure(
            "shares docbook-fmt's implementation: StreamingParser's per-drain-call Reader \
             disables check_end_names/allow_unmatched_ends and so does not detect mismatched \
             end tags events() catches; see the \"docbook\" streaming_parser KnownFailure",
        ),
        streaming_writer: ApiState::KnownFailure(
            "shares docbook-fmt's implementation: the streaming Writer inherits events()'s lack \
             of parse()'s malformed-XML auto-close recovery; see the \"docbook\" \
             streaming_writer KnownFailure",
        ),
    },
    // bbcode-fmt: events() is `parse::parse(input)` followed by a tree walk
    // (events.rs's `events()` literally calls `crate::parse::parse(input)`
    // then walks the resulting `BbcodeDoc`) — the same non-independent shape
    // as html-fmt's `events_from_doc`, but with no format-spec reason
    // forcing it, so (per the asciidoc precedent) it's still wired rather
    // than NotApplicable. StreamingParser (batch.rs) is a genuine
    // incremental line-buffered state machine — real Wired, confirmed by an
    // incrementality probe and an adversarial-chunking equivalence check
    // that holds over all 53 bbcode fixtures plus several hand-built
    // adversarial cases (nested same-tag quotes, a same-line-closed block
    // tag immediately followed by more text, a blank line inside an
    // InBlock quote) tried while auditing it. The streaming Writer is the
    // one real gap: hollow buffer-then-finish() (writer.rs's own module
    // doc), so content matches build() but the incrementality probe fails.
    FormatCapabilities {
        format: "bbcode",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::KnownFailure(
            "bbcode_fmt::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST (events_to_doc) + calls emit() inside finish() (see \
             crates/formats/bbcode-fmt/src/writer.rs's own module doc, \"This implementation \
             buffers all events, reconstructs the AST, then emits\" — write_event() at \
             writer.rs:42-44 only pushes onto self.events); content is still byte-identical to \
             build() over all fixtures since finish() ultimately drives the same emit() path, \
             but zero bytes reach the sink before finish() is called — not a genuine \
             incremental streaming writer",
        ),
    },
    // creole's events() is `EventIter::new` calling `crate::parse::parse(input)`
    // then `collect_events(&doc)`, a depth-first AST walk (events.rs:123-127) —
    // the same non-independent shape as bbcode-fmt's and html-fmt's events(),
    // but with no format-spec reason forcing it, so per the bbcode/asciidoc
    // precedent it's still wired rather than NotApplicable. StreamingParser
    // (batch.rs) is a genuine incremental line-buffered state machine — real
    // Wired, confirmed by an incrementality probe and an adversarial-chunking
    // equivalence check that holds over all 35 creole fixtures (one inspected
    // edge case, a nowiki block closed by a line with trailing content after
    // "}}}", degrades incrementality but not correctness — see the doc
    // comment on the streaming_parser test). The streaming Writer is the one
    // real gap: hollow buffer-then-finish() (writer.rs's own module doc,
    // write_event() only pushes onto a Vec), so content matches build() but
    // the incrementality probe fails.
    FormatCapabilities {
        format: "creole",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::KnownFailure(
            "creole::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST (events_to_doc) + calls crate::emit::build inside finish() \
             (write_event() at writer.rs:38-40 only pushes onto self.events, all real work \
             happens in finish() at writer.rs:43-48); content is still byte-identical to \
             build() over all fixtures since finish() ultimately drives the same build() path, \
             but zero bytes reach the sink before finish() is called — not a genuine \
             incremental streaming writer",
        ),
    },
    // dokuwiki's events() is `InputEventIter::new`, which calls
    // `crate::parse::parse(input)` then walks the resulting `DokuwikiDoc` with
    // the crate's own lazy `EventIter` (events.rs:705-731) — the same "parse()
    // then walk the tree" shape as bbcode-fmt's and creole's events(), not two
    // independent implementations, but per that precedent it's still wired
    // rather than NotApplicable (nothing in the DokuWiki format forces this
    // shape). StreamingParser (batch.rs) is a genuine incremental
    // line-buffered state machine — real Wired, confirmed by an
    // incrementality probe and an adversarial-chunking equivalence check that
    // holds over every dokuwiki fixture with no coarser-boundary caveat
    // needed (unlike bbcode/creole): parse.rs's Parser has no cross-block
    // state at all (no loose-list joining, no reference resolution), so every
    // boundary StreamingParser::feed_line flushes on is one parse.rs's own
    // dispatch loop would also treat as a valid split point. The streaming
    // Writer is the one real gap: hollow buffer-then-finish() (writer.rs's
    // own module doc, write_event() only pushes onto a Vec), so content
    // matches build() but the incrementality probe fails.
    FormatCapabilities {
        format: "dokuwiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::KnownFailure(
            "dokuwiki::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST (events_to_doc) + calls crate::emit::build inside finish() \
             (write_event() at writer.rs:27-29 only pushes onto self.events, all real work \
             happens in finish() at writer.rs:32-37, self-admitted in the module doc \
             \"Buffers all events, reconstructs the AST, then emits\"); content is still \
             byte-identical to build() over all fixtures since finish() ultimately drives the \
             same build() path, but zero bytes reach the sink before finish() is called — not a \
             genuine incremental streaming writer",
        ),
    },
    // jira-fmt's events() is `crate::parse::parse(input)` followed by a full
    // walk of the resulting `JiraDoc` (events.rs's `events()`/
    // `emit_doc_events`) — the same "parse() then walk the tree" shape as
    // bbcode-fmt's, creole's, and dokuwiki's events(), not two independent
    // implementations, but per that precedent it's still wired rather than
    // NotApplicable (nothing in the Jira wiki markup grammar forces this
    // shape). StreamingParser (batch.rs) is a genuine incremental
    // line-buffered state machine — real Wired, confirmed by an
    // incrementality probe and an adversarial-chunking equivalence check
    // that holds over every jira fixture with no coarser-boundary caveat
    // needed (unlike bbcode/creole): parse.rs's Parser has no cross-block
    // state (no loose-list joining, no reference resolution, and no
    // decorator-line-preceding-a-fence construct — {code:lang} and
    // {panel:title=...} both encode their parameters on the fence line
    // itself), so every boundary StreamingParser::feed_line flushes on is
    // one parse.rs's own block-stop conditions would also treat as a valid
    // split point. The streaming Writer is the one real gap: hollow
    // buffer-then-finish() (writer.rs's own module doc, write_event() only
    // pushes onto a Vec), so content matches build() but the incrementality
    // probe fails.
    FormatCapabilities {
        format: "jira",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::KnownFailure(
            "jira_fmt::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST (events_to_doc) + calls crate::emit::build inside finish() \
             (write_event() at writer.rs:40-42 only pushes onto self.events, all real work \
             happens in finish() at writer.rs:45-50, self-admitted in the module doc \"this \
             implementation buffers all events, reconstructs the AST, then emits\"); content is \
             still byte-identical to build() over all fixtures since finish() ultimately drives \
             the same build() path, but zero bytes reach the sink before finish() is called — \
             not a genuine incremental streaming writer",
        ),
    },
    FormatCapabilities {
        format: "mediawiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::KnownFailure(
            "mediawiki_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls emit() inside finish() (writer.rs's Writer::finish); \
             content round-trips correctly on every fixture, but the incrementality probe (a \
             complete StartParagraph/Text/EndParagraph sequence, checked for any bytes reaching \
             an ObservableSink before finish()) writes zero bytes, confirming this is a fake \
             streaming writer per CLAUDE.md, not just architecturally described as one",
        ),
    },
    FormatCapabilities {
        format: "tikiwiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::KnownFailure(
            "tikiwiki::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls build() inside finish() (writer.rs's Writer::finish); \
             content round-trips correctly on every fixture, but the incrementality probe \
             writes zero bytes before finish(), confirming this is a fake streaming writer per \
             CLAUDE.md",
        ),
    },
    FormatCapabilities {
        format: "twiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::KnownFailure(
            "twiki::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls build() inside finish() (writer.rs's Writer::finish); \
             content round-trips correctly on every fixture, but the incrementality probe \
             writes zero bytes before finish(), confirming this is a fake streaming writer per \
             CLAUDE.md",
        ),
    },
    FormatCapabilities {
        format: "vimwiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "vimwiki_fmt's StreamingParser and events() disagree even under the 'whole input, \
             one feed() call' chunking (fixture 'oracle'), so this is not a chunk-boundary bug: \
             parse()/events() treat a blank-line-separated run of an unordered list, then an \
             ordered list, then an unordered checklist (no other content between them) as ONE \
             Block::List with a single ordered flag for all 8 items -- silently losing the \
             ordered/unordered distinction for the second and third groups, since Block::List \
             has one `ordered: bool` for the whole list, not per-item -- while \
             batch::StreamingParser's emit_block hard-splits on every blank line (batch.rs's \
             feed_line unconditionally treats a blank line as a block boundary), so it emits \
             three separate, correctly-typed StartList/EndList pairs. The two implementations \
             disagree about where one list ends and the next begins, not just about \
             representing the disagreement identically; see TODO.md",
        ),
        streaming_writer: ApiState::KnownFailure(
            "vimwiki_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
             calls collect_doc_from_events() + build() inside finish() (writer.rs's \
             Writer::finish); content round-trips correctly on every fixture, but the \
             incrementality probe writes zero bytes before finish(), confirming this is a fake \
             streaming writer per CLAUDE.md",
        ),
    },
    // xwiki's events() is a genuinely lazy pull-iterator over &XwikiDoc
    // (EventIter::next() walks a frame stack on demand, events.rs:168-385),
    // unlike zimwiki/markua/muse-fmt below which eagerly materialize a
    // Vec/VecDeque before iteration begins.
    FormatCapabilities {
        format: "xwiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "xwiki::batch::StreamingParser buffers all fed bytes into a Vec<u8> and only \
             parses + delivers events inside finish() (batch.rs:61-72); feed() never advances \
             real parser state, so no events reach the handler until finish() is called — \
             found while wiring this harness's incrementality probe",
        ),
        streaming_writer: ApiState::KnownFailure(
            "xwiki::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls emit::build() inside finish() (writer.rs:39-49); \
             content round-trips correctly but zero bytes reach the sink before finish() — \
             found while wiring this harness's incrementality probe",
        ),
    },
    // zimwiki's events() is parse()+eager-materialize-then-walk (EventIter::new
    // calls parse::parse(input) then walks into a Vec before returning any
    // event, events.rs:94-102) — the same narrower "Wired" claim as asciidoc.
    // StreamingParser, unlike xwiki/muse-fmt, is REAL incremental (feed_line
    // tracks verbatim-block/blank-line boundaries and calls emit_block()
    // during feed(), batch.rs:93-152).
    FormatCapabilities {
        format: "zimwiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "zimwiki::parse::Parser::parse_list (parse.rs:190-241) doesn't stop consuming list \
             items when the marker type changes: its loop condition is `is_bullet || \
             is_numbered` (line 211-213) with no check that a run of numbered items follows a \
             run of bulleted ones, and its blank-line arm (line 199-202) skips blank lines \
             with `continue` instead of breaking — so a blank-line-separated unordered list \
             immediately followed by an ordered list (fixtures/zimwiki/oracle: '* First item' \
             .. blank .. '1. Ordered one') gets merged by the *whole-document* parser into ONE \
             `Block::List` tagged with the FIRST item's `ordered` value, silently discarding \
             that items 4-5 were numbered. `StreamingParser`'s blank-line block-splitter \
             (batch.rs) hard-splits at that same blank line BEFORE re-parsing each half in \
             isolation, so it does NOT reproduce this merge and instead emits two correctly-typed \
             lists — the two diverge because they disagree on where the block boundary is, not \
             because StreamingParser lost information the bulk parser had. This is a pre-existing \
             parse()-level bug independent of streaming, found while wiring this harness's \
             adversarial-chunking equivalence check; see TODO.md",
        ),
        streaming_writer: ApiState::KnownFailure(
            "zimwiki::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls emit::build() inside finish() (writer.rs:24-34); \
             content round-trips correctly but zero bytes reach the sink before finish() — \
             found while wiring this harness's incrementality probe",
        ),
    },
    // markua's events() is parse()+eager-tree-build-then-walk (EventIter::new,
    // re-exported from parse.rs not events.rs, runs the full recursive-descent
    // Parser::parse() before any event is returned, parse.rs:969-985) — the
    // same narrower "Wired" claim as asciidoc/zimwiki. StreamingParser, unlike
    // xwiki/muse-fmt, is REAL incremental block-boundary segmentation
    // (fenced-code-aware feed_line, batch.rs:108-152).
    FormatCapabilities {
        format: "markua",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "markua::parse::Parser::parse_list (parse.rs:379-432) has the identical structural \
             bug already found in zimwiki's parse_list: its loop condition accepts either \
             `is_bullet` or `is_numbered` with no check for a marker-type change, so a \
             blank-line-separated unordered list immediately followed by an ordered list \
             (fixtures/markua/oracle: '- item one' / '- item two' .. blank .. '1. first' / \
             '2. second') gets merged by the whole-document parser into ONE `Block::List` \
             tagged with the first item's `ordered` value, silently mislabeling the numbered \
             items. `StreamingParser`'s blank-line block-splitter hard-splits at that blank \
             line before re-parsing each half in isolation, so it correctly emits two \
             separately-typed lists instead — same root cause and same 'the two block-splitting \
             passes disagree' shape as zimwiki, found while wiring this harness's \
             adversarial-chunking equivalence check; see TODO.md",
        ),
        streaming_writer: ApiState::KnownFailure(
            "markua::writer::Writer buffers all fed events into a Vec<OwnedMarkuaEvent> and \
             only reconstructs the AST + calls emit::emit() inside finish() (writer.rs:40-50); \
             content round-trips correctly but zero bytes reach the sink before finish() — \
             found while wiring this harness's incrementality probe. Separately (not caught by \
             this harness's fixture loop, since parse() never constructs it): MarkuaDoc::title/ \
             author/description are permanently None because parse() never populates them from \
             any Markua syntax, and Block::Figure is never constructed by parse() either, so the \
             Writer's own Figure/Caption reconstruction bug (EndFigure takes the wrong child as \
             body and drops the caption, writer.rs:315-330) is unreachable via fixtures",
        ),
    },
    // muse-fmt's events() takes &MuseDoc (like xwiki) but eagerly materializes
    // a VecDeque in EventIter::new (events.rs:211-220) rather than pulling
    // lazily.
    FormatCapabilities {
        format: "muse",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "muse_fmt::batch::StreamingParser buffers all fed bytes into a Vec<u8> and only \
             parses + delivers events inside finish() (batch.rs:94-105); the crate's own module \
             docs admit this outright (\"Muse's block-level structure makes true incremental \
             parsing difficult without a dedicated state machine\", batch.rs:11-13) — found \
             while wiring this harness's incrementality probe",
        ),
        streaming_writer: ApiState::KnownFailure(
            "muse_fmt::writer::Writer buffers all fed events into a Vec<OwnedMuseEvent> and \
             only reconstructs the AST + calls emit::build() inside finish() (writer.rs:47-52) \
             — a fake streaming writer per CLAUDE.md, found via this harness's incrementality \
             probe. Also a genuine expressiveness gap independent of the buffering: MuseEvent \
             has no variant carrying document metadata at all (events.rs:27-114), so \
             DocBuilder::finish always reconstructs `MuseDoc { ..Default::default() }` \
             (writer.rs:499-504), permanently dropping #title/#author/#date/#desc/#keywords — \
             reachable via the document-header fixture, unlike markua's equivalent gap, since \
             muse-fmt's parse() genuinely populates these fields (parse.rs:240-249)",
        ),
    },
    // t2t's events() is `EventIter::new(parse(input).0)` (src/events.rs) — a
    // lazy frame-stack walk of the AST parse() already built, not an
    // independently implemented reader. Per the asciidoc precedent above,
    // the ast_to_events-vs-events() check validates the AST->event expansion
    // layer, not two independent parsers. See the comment above the check in
    // tests/streaming_apis.rs.
    FormatCapabilities {
        format: "t2t",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "t2t::batch::StreamingParser genuinely flushes events per accumulated block as fed \
             (not only at finish()), but emit_block() re-parses each block's text in isolation \
             via crate::events::events(&text) — the same \"re-parse each block alone, lose \
             cross-block context\" root cause already tracked for org-fmt/asciidoc. Two \
             distinct fixtures expose it: definition-list, where the blank line between items \
             (batch.rs's feed_line blank-line branch, batch.rs:143-150) ends the accumulated \
             block, splitting one multi-item DefinitionList into two DefinitionList event pairs \
             (the same bug class already tracked for rst/org); and document-header, where the \
             isolated re-parse of the 3-line header block re-triggers try_parse_header() \
             (parse.rs:70, requires >=3 lines and a non-heading/list/table/comment first line), \
             which any 3+ line block satisfies purely by looking like a header out of context, \
             producing a spurious extra StartDocument/EndDocument pair (Event has no metadata \
             variant to carry the consumed header text) that events() over the whole document \
             never produces; see TODO.md",
        ),
        streaming_writer: ApiState::KnownFailure(
            "t2t::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls emit() inside finish() (writer.rs's own module doc: \
             \"This implementation buffers all events, reconstructs the AST, then emits\") — the \
             same fake-streaming-writer pattern as textile/commonmark/org/texinfo. It also drops \
             doc.title/author/date on every fixture with a document header: emit::emit() always \
             writes the 3-line header verbatim from T2tDoc.title/author/date (emit.rs:9-16), but \
             t2t::Event has no variant carrying those fields, so writer.rs's DocBuilder::finish \
             (writer.rs:400-404) always reconstructs title: None/author: None/date: None — an \
             Event-enum expressiveness gap, not a one-line logic bug, exposed by the \
             document-header fixture; see TODO.md",
        ),
    },
    // pod-fmt's events() is `pod_fmt::events()` (src/lib.rs) — `parse(input)`
    // then an eager `.collect()` of a lazy frame-stack `EventIter` walk of
    // the AST parse() already built, not an independently implemented
    // reader (same pattern as t2t/asciidoc above). See the comment above the
    // check in tests/streaming_apis.rs.
    FormatCapabilities {
        format: "pod",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "pod_fmt::batch::StreamingParser is explicitly self-documented buffer-then-finish \
             (batch.rs's own module doc: \"POD documents are always small enough to buffer \
             fully, so this implementation accumulates all input and parses on finish()\"); \
             feed() only extends an internal Vec<u8>, all parsing and event delivery happen in \
             finish(). Unlike t2t/org/asciidoc this does NOT diverge from events() under \
             adversarial chunking (finish() parses the whole buffered input the same way bulk \
             events() does, no per-block re-parse-in-isolation to disagree with) — the defect is \
             purely architectural non-incrementality, pinned via the feed()-before-finish() \
             probe. pod-fmt's own docstring rationale is not a CLAUDE.md-sanctioned exemption \
             (only commonmark-fmt's pulldown-cmark wrapping is); see TODO.md",
        ),
        streaming_writer: ApiState::KnownFailure(
            "pod_fmt::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls emit::build() inside finish() (writer.rs's finish(): \
             events_to_doc(...) then crate::emit::build(&doc)) — the same fake-streaming-writer \
             pattern as t2t/textile/commonmark/org/texinfo. Content is not lost (PodDoc has no \
             document-level metadata field pod::Event could be missing, unlike t2t), so the \
             byte-identical-to-builder check passes; only the incrementality probe fails; see \
             TODO.md",
        ),
    },
    // haddock-fmt's events() is `parse(input)` then a lazy frame-stack
    // EventIter walk of the AST — not an independently implemented reader
    // (same pattern as t2t/pod/asciidoc above). See the comment above the
    // check in tests/streaming_apis.rs.
    FormatCapabilities {
        format: "haddock",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::KnownFailure(
            "haddock_fmt::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls emit::build() inside finish() (writer.rs's own module \
             doc: \"This implementation buffers all events, reconstructs the AST, then emits\") \
             — the same fake-streaming-writer pattern as t2t/pod/textile/commonmark/org/texinfo; \
             see TODO.md",
        ),
    },
    // fountain-fmt's events() is `parse(input)` then a lazy AST walk via
    // events::OwnedEventIter — not an independently implemented reader
    // (same pattern as t2t/pod/haddock/asciidoc above). See the comment
    // above the check in tests/streaming_apis.rs, which also notes a
    // separate, out-of-scope bug in the crate's un-exported borrowed
    // `EventIter<'a>` type (not what `events()` returns).
    FormatCapabilities {
        format: "fountain",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "fountain_fmt::batch::StreamingParser's emit_block() re-parses each accumulated \
             block via crate::events::events(&text) and forwards every event it yields — \
             including that call's own StartDocument/EndDocument pair — straight to the handler \
             with no filtering (batch.rs's emit_block(): \"for event in \
             crate::events::events(&text) { self.handler.handle(event); }\"). Bulk events() over \
             the whole input emits exactly one StartDocument/EndDocument pair; StreamingParser \
             emits one pair PER accumulated block, diverging on every fixture with more than one \
             blank-line-separated block — not an edge case, the dominant failure mode. A second, \
             narrower defect shares the same re-parse-in-isolation root cause: \
             parse_title_page() (parse.rs:81) runs unconditionally at the start of every parse() \
             call with no \"is this really the first block\" guard, so a body block matching \
             `key: value` for one of the 9 recognized title-page field names is misread as \
             metadata when re-parsed in isolation — the same class already tracked for t2t's \
             try_parse_header(); see TODO.md",
        ),
        streaming_writer: ApiState::KnownFailure(
            "fountain_fmt::writer::Writer buffers all fed events into a Vec<OwnedEvent> and only \
             reconstructs the AST + calls emit() inside finish() (writer.rs's own module doc: \
             \"This implementation buffers all events, reconstructs the AST, then emits\") — the \
             same fake-streaming-writer pattern as t2t/pod/haddock/textile/commonmark/org/\
             texinfo; see TODO.md",
        ),
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
    "latex",
    "man",
    "csl-json",
    "native",
    "pandoc-json",
    "ipynb",
    "csv",
    "tsv",
    "opml",
    "ris",
    "bibtex",
    "biblatex",
    "typst",
    "endnotexml",
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
        format: "mediawiki",
        api: "streaming_writer",
        description: "mediawiki_fmt::writer::Writer buffers all events and only reconstructs \
                       the AST + emits inside finish(); zero bytes reach the sink before \
                       finish() despite content round-tripping correctly",
    },
    KnownFailure {
        format: "tikiwiki",
        api: "streaming_writer",
        description: "tikiwiki::writer::Writer buffers all events and only reconstructs the \
                       AST + calls build() inside finish(); zero bytes reach the sink before \
                       finish() despite content round-tripping correctly",
    },
    KnownFailure {
        format: "twiki",
        api: "streaming_writer",
        description: "twiki::writer::Writer buffers all events and only reconstructs the AST + \
                       calls build() inside finish(); zero bytes reach the sink before finish() \
                       despite content round-tripping correctly",
    },
    KnownFailure {
        format: "vimwiki",
        api: "streaming_parser",
        description: "vimwiki_fmt's StreamingParser and events() disagree on where a list ends: \
                       parse()/events() merge an unordered list, ordered list, and unordered \
                       checklist (separated only by blank lines, no other content) into one \
                       Block::List with a single ordered flag, losing the type distinction for \
                       later groups, while StreamingParser hard-splits on every blank line and \
                       emits three separately-typed lists",
    },
    KnownFailure {
        format: "vimwiki",
        api: "streaming_writer",
        description: "vimwiki_fmt::writer::Writer buffers all events and only calls \
                       collect_doc_from_events() + build() inside finish(); zero bytes reach \
                       the sink before finish() despite content round-tripping correctly",
    },
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
        format: "djot",
        api: "streaming_parser",
        description: "djot-fmt StreamingParser: four distinct batch.rs bugs — BlockState::InDiv \
                       is a flag not a counter (nested ::: divs unbalance and leftover closers \
                       are misread as new openers), out-of-block link-reference definitions are \
                       invisible to the per-block pre_scan, a pending {.attr} line is flushed \
                       away from the fence it decorates, and the blank-line boundary splits a \
                       multi-item definition list",
    },
    KnownFailure {
        format: "djot",
        api: "streaming_writer",
        description: "djot-fmt Event enum has no LinkDef variant and writer.rs's \
                       DocBuilder.link_defs is never pushed to, so events_to_doc always returns \
                       link_defs: [] and Writer drops link-reference definitions",
    },
    KnownFailure {
        format: "asciidoc",
        api: "streaming_parser",
        description: "asciidoc StreamingParser: three distinct batch.rs bugs — feed_line \
                       flushing an attribute/.title line away from the delimited block it \
                       modifies, is_delimited_block_marker not recognizing the |=== table \
                       delimiter, and empty input producing no StartDocument/EndDocument pair",
    },
    KnownFailure {
        format: "org",
        api: "streaming_parser",
        description: "org-fmt StreamingParser: three distinct feed_line/BlockState bugs — no \
                       nesting depth in InSpecialBlock, emit_block() flushing an affiliated \
                       #+NAME: line away from its block, and the #+BEGIN_ test trimming so an \
                       indented list-item code block reads as top-level",
    },
    KnownFailure {
        format: "rst",
        api: "streaming_parser",
        description: "rst-fmt StreamingParser splits a multi-item DefinitionList into one \
                       StartDefinitionList/EndDefinitionList pair per item instead of one list \
                       spanning all items",
    },
    KnownFailure {
        format: "texinfo",
        api: "streaming_parser",
        description: "texinfo::batch::StreamingParser buffers all fed bytes and only parses + \
                       delivers events inside finish(); feed() delivers zero events before \
                       finish() is called",
    },
    KnownFailure {
        format: "texinfo",
        api: "streaming_writer",
        description: "texinfo::writer::Writer buffers all events and only emits inside finish(); \
                       zero bytes reach the sink before finish() despite content \
                       round-tripping correctly (including @settitle, now carried via \
                       events::Event::Title)",
    },
    KnownFailure {
        format: "fb2",
        api: "events",
        description: "fb2_fmt events()/EventIter silently drops Event::Metadata for input \
                       lacking a literal <description> element, unlike parse()'s AST which \
                       always carries a (possibly-default) description",
    },
    KnownFailure {
        format: "fb2",
        api: "streaming_parser",
        description: "fb2_fmt::StreamingParser buffers all fed bytes and only parses + delivers \
                       events inside finish(), even though the crate's own events()/EventIter is \
                       a genuine incremental quick_xml pull parser — StreamingParser does not \
                       reuse that incrementality",
    },
    KnownFailure {
        format: "fb2",
        api: "streaming_writer",
        description: "downstream of the fb2/events KnownFailure: the streaming Writer never \
                       receives Metadata for input lacking <description>, so it never emits a \
                       <description> element, while the AST builder path always writes one",
    },
    KnownFailure {
        format: "textile",
        api: "streaming_parser",
        description: "textile_fmt::batch::StreamingParser buffers all fed bytes and only parses \
                       + delivers events inside finish(); feed() delivers zero events before \
                       finish() is called",
    },
    KnownFailure {
        format: "textile",
        api: "streaming_writer",
        description: "textile_fmt::writer::Writer buffers all events and only reconstructs the \
                       AST + emits inside finish() — a fake streaming writer per CLAUDE.md",
    },
    KnownFailure {
        format: "commonmark",
        api: "events",
        description: "commonmark_fmt events() has two real bugs: (1) for images, Text(alt) is \
                       delivered before StartImage instead of between StartImage/EndImage as \
                       documented (an EventIter::next() loop-ordering bug); (2) consecutive Text \
                       runs are not coalesced the way parse()'s AST does",
    },
    KnownFailure {
        format: "gfm",
        api: "events",
        description: "shares commonmark-fmt's unmerged-Text-events defect (see the \
                       \"commonmark\" KnownFailure entry above)",
    },
    KnownFailure {
        format: "markdown",
        api: "events",
        description: "shares commonmark-fmt's unmerged-Text-events defect (see the \
                       \"commonmark\" KnownFailure entry above)",
    },
    KnownFailure {
        format: "commonmark",
        api: "streaming_writer",
        description: "commonmark_fmt::writer::Writer buffers all events and only reconstructs \
                       the AST + emits inside finish(), self-admitted in its own module doc; \
                       unrelated to the sanctioned pulldown-cmark StreamingParser exemption",
    },
    KnownFailure {
        format: "gfm",
        api: "streaming_writer",
        description: "shares commonmark-fmt's buffer-then-emit streaming Writer defect (see the \
                       \"commonmark\" KnownFailure entry above)",
    },
    KnownFailure {
        format: "markdown",
        api: "streaming_writer",
        description: "shares commonmark-fmt's buffer-then-emit streaming Writer defect (see the \
                       \"commonmark\" KnownFailure entry above)",
    },
    KnownFailure {
        format: "docbook",
        api: "events",
        description: "events()/EventIter emits one Text event per resolved character/predefined \
                       entity instead of merging it into the surrounding text run the way \
                       parse()'s AST does",
    },
    KnownFailure {
        format: "docbook",
        api: "streaming_parser",
        description: "StreamingParser's per-drain-call Reader disables \
                       check_end_names/allow_unmatched_ends (architecturally necessary for only \
                       seeing the unconsumed tail per call) and so silently accepts mismatched \
                       end tags that events()'s continuous Reader correctly rejects",
    },
    KnownFailure {
        format: "docbook",
        api: "streaming_writer",
        description: "downstream of the events() gap: parse() auto-closes unclosed elements on \
                       malformed XML but events() has no such recovery, so the streaming Writer \
                       emits truncated/unclosed output where build() recovers",
    },
    KnownFailure {
        format: "jats",
        api: "events",
        description: "shares docbook-fmt's implementation; same entity-Text-coalescing gap",
    },
    KnownFailure {
        format: "jats",
        api: "streaming_parser",
        description: "shares docbook-fmt's implementation; same check_end_names-disabled gap",
    },
    KnownFailure {
        format: "jats",
        api: "streaming_writer",
        description: "shares docbook-fmt's implementation; same malformed-XML recovery gap",
    },
    KnownFailure {
        format: "tei",
        api: "events",
        description: "shares docbook-fmt's implementation; same entity-Text-coalescing gap",
    },
    KnownFailure {
        format: "tei",
        api: "streaming_parser",
        description: "shares docbook-fmt's implementation; same check_end_names-disabled gap",
    },
    KnownFailure {
        format: "tei",
        api: "streaming_writer",
        description: "shares docbook-fmt's implementation; same malformed-XML recovery gap",
    },
    KnownFailure {
        format: "odt",
        api: "streaming_writer",
        description: "OdfEvent has no variant carrying mimetype/meta/styles/images, so the \
                       streaming Writer's reconstructed OdfDocument always drops them even \
                       though it genuinely builds incrementally per event",
    },
    KnownFailure {
        format: "ansi",
        api: "streaming_parser",
        description: "StreamingParser::drain_complete constructs a fresh EventIter (style: \
                       default) on every drain, losing running style state across chunk \
                       boundaries under fine-grained chunking; also inherits events()'s \
                       spurious-ResetStyle-on-unrecognized-SGR bug",
    },
    KnownFailure {
        format: "ansi",
        api: "streaming_writer",
        description: "downstream of events()'s spurious ResetStyle event on an unrecognized SGR \
                       group: Writer unconditionally writes literal reset bytes for every \
                       ResetStyle event, turning the spurious event into spurious output",
    },
    KnownFailure {
        format: "bbcode",
        api: "streaming_writer",
        description: "bbcode_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and \
                       only reconstructs the AST + calls emit() inside finish() (self-admitted \
                       in its own module doc); content matches build() exactly but the writer \
                       is not incrementally streaming",
    },
    KnownFailure {
        format: "creole",
        api: "streaming_writer",
        description: "creole::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                       reconstructs the AST + calls build() inside finish() (write_event() only \
                       pushes onto self.events); content matches build() exactly but the writer \
                       is not incrementally streaming",
    },
    KnownFailure {
        format: "dokuwiki",
        api: "streaming_writer",
        description: "dokuwiki::writer::Writer buffers all events into a Vec<OwnedEvent> and \
                       only reconstructs the AST + calls crate::emit::build inside finish() \
                       (write_event() at writer.rs:27-29 only pushes onto self.events); content \
                       matches build() exactly but the writer is not incrementally streaming",
    },
    KnownFailure {
        format: "jira",
        api: "streaming_writer",
        description: "jira_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and \
                       only reconstructs the AST (events_to_doc) + calls crate::emit::build \
                       inside finish() (write_event() at writer.rs:40-42 only pushes onto \
                       self.events); content matches build() exactly but the writer is not \
                       incrementally streaming",
    },
    KnownFailure {
        format: "xwiki",
        api: "streaming_parser",
        description: "xwiki::batch::StreamingParser buffers all fed bytes and only parses + \
                       delivers events inside finish(); feed() delivers zero events before \
                       finish() is called",
    },
    KnownFailure {
        format: "xwiki",
        api: "streaming_writer",
        description: "xwiki::writer::Writer buffers all events and only reconstructs the AST + \
                       emits inside finish(); content round-trips but zero bytes reach the sink \
                       before finish()",
    },
    KnownFailure {
        format: "zimwiki",
        api: "streaming_parser",
        description: "zimwiki::parse::Parser::parse_list merges a blank-line-separated \
                       unordered list immediately followed by an ordered list into one \
                       Block::List tagged with the first item's `ordered` value (a \
                       whole-document parse()-level bug); StreamingParser's blank-line block \
                       splitter hard-splits at that boundary and does not reproduce the merge",
    },
    KnownFailure {
        format: "zimwiki",
        api: "streaming_writer",
        description: "zimwiki::writer::Writer buffers all events and only reconstructs the AST \
                       + emits inside finish(); content round-trips but zero bytes reach the \
                       sink before finish()",
    },
    KnownFailure {
        format: "markua",
        api: "streaming_parser",
        description: "markua::parse::Parser::parse_list has the identical structural bug as \
                       zimwiki's: it merges a blank-line-separated unordered list immediately \
                       followed by an ordered list into one Block::List tagged with the first \
                       item's `ordered` value; StreamingParser's blank-line block splitter \
                       hard-splits at that boundary and does not reproduce the merge",
    },
    KnownFailure {
        format: "markua",
        api: "streaming_writer",
        description: "markua::writer::Writer buffers all events and only reconstructs the AST + \
                       emits inside finish(); content round-trips but zero bytes reach the sink \
                       before finish(). Separately, the Writer's Figure/Caption reconstruction \
                       (EndFigure takes the wrong child as body and drops the caption) is a real \
                       code bug but unreachable via fixtures since parse() never constructs \
                       Block::Figure",
    },
    KnownFailure {
        format: "muse",
        api: "streaming_parser",
        description: "muse_fmt::batch::StreamingParser buffers all fed bytes and only parses + \
                       delivers events inside finish(); the crate's own module docs admit this \
                       outright",
    },
    KnownFailure {
        format: "muse",
        api: "streaming_writer",
        description: "muse_fmt::writer::Writer buffers all events and only reconstructs the AST \
                       + emits inside finish() (a fake streaming writer per CLAUDE.md); also \
                       MuseEvent has no variant for document metadata, so #title/#author/#date/ \
                       #desc/#keywords are always dropped on round-trip through the streaming \
                       writer (reachable via the document-header fixture)",
    },
    KnownFailure {
        format: "t2t",
        api: "streaming_parser",
        description: "t2t::batch::StreamingParser's emit_block() re-parses each accumulated \
                       block in isolation: a blank line splits a multi-item DefinitionList into \
                       one StartDefinitionList/EndDefinitionList pair per item, and an isolated \
                       3+ line block that looks like a document header re-triggers \
                       try_parse_header(), producing a spurious extra StartDocument/EndDocument \
                       pair events() over the whole document never produces",
    },
    KnownFailure {
        format: "t2t",
        api: "streaming_writer",
        description: "t2t::writer::Writer buffers all events and only reconstructs the AST + \
                       emits inside finish() — a fake streaming writer per CLAUDE.md; it also \
                       always drops doc.title/author/date since t2t::Event has no variant \
                       carrying them",
    },
    KnownFailure {
        format: "pod",
        api: "streaming_parser",
        description: "pod_fmt::batch::StreamingParser self-documents as buffer-then-finish; \
                       feed() only extends a Vec<u8>, all parsing and event delivery happen in \
                       finish() — not incremental despite implementing the feed/finish contract",
    },
    KnownFailure {
        format: "pod",
        api: "streaming_writer",
        description: "pod_fmt::writer::Writer buffers all events and only reconstructs the AST \
                       + calls emit::build() inside finish() — a fake streaming writer per \
                       CLAUDE.md",
    },
    KnownFailure {
        format: "haddock",
        api: "streaming_writer",
        description: "haddock_fmt::writer::Writer buffers all events and only reconstructs the \
                       AST + calls emit::build() inside finish() — a fake streaming writer per \
                       CLAUDE.md",
    },
    KnownFailure {
        format: "fountain",
        api: "streaming_parser",
        description: "fountain_fmt::batch::StreamingParser's emit_block() forwards every event \
                       from its own re-parse of each block, including that call's \
                       StartDocument/EndDocument pair, so it emits one such pair per block \
                       instead of one for the whole document; also parse_title_page() has no \
                       document-position guard, so a body line matching a title-page field name \
                       is misread as metadata when re-parsed in isolation",
    },
    KnownFailure {
        format: "fountain",
        api: "streaming_writer",
        description: "fountain_fmt::writer::Writer buffers all events and only reconstructs the \
                       AST + calls emit() inside finish() — a fake streaming writer per \
                       CLAUDE.md",
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
