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
        streaming_writer: ApiState::KnownFailure(
            "org-fmt's Writer loses content vs build() on 3 of 89 org fixtures. The dominant \
             cause is an expressiveness gap in the Event enum, not a logic error: Event \
             (events.rs:14-133) has no document-metadata variant at all — metadata is delivered \
             out-of-band via EventIter::take_metadata() (parse.rs:87) — so events() cannot carry \
             #+TITLE:/#+AUTHOR:/#+CUSTOM_KEY: lines and writer.rs's DocBuilder::finish \
             (writer.rs:616) has no choice but to hardcode `metadata: vec![]`, dropping every \
             leading keyword line (fixtures metadata, keyword-line). The third fixture, \
             dynamic-block, stacks on a separate pre-existing parse/emit bug: parse.rs has no \
             #+BEGIN:/#+END: support, so parse_metadata_line absorbs a bare #+END: as document \
             metadata key `end`, which build() re-emits as a stray leading `#+END: ` before all \
             blocks and DocBuilder::finish then discards. Note Writer is also not incrementally \
             streaming — writer.rs's module docs state it buffers all events, reconstructs the \
             AST, then calls emit::build; see TODO.md",
        ),
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
             emits\"); additionally, texinfo::events::Event has no variant carrying \
             TexinfoDoc::title, so events_to_doc() always reconstructs title: None, silently \
             dropping @settitle (see fixtures/texinfo/settitle-header) when content round-trips \
             through the streaming writer",
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
    "mediawiki",
    "latex",
    "creole",
    "muse",
    "tikiwiki",
    "twiki",
    "vimwiki",
    "dokuwiki",
    "jira",
    "man",
    "xwiki",
    "zimwiki",
    "bbcode",
    "markua",
    "ansi",
    "csl-json",
    "native",
    "pandoc-json",
    "docbook",
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
        format: "org",
        api: "streaming_writer",
        description: "org-fmt Event enum has no document-metadata variant, so events() cannot \
                       carry #+KEY: lines and writer.rs's DocBuilder::finish hardcodes \
                       metadata: vec![], dropping every leading keyword line",
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
                       also, Event has no variant for TexinfoDoc::title, so @settitle is always \
                       dropped when round-tripped through the streaming writer",
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
