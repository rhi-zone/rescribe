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
    /// reason documented in `docs/format-audit.md` (e.g. html-fmt's
    /// `events()`/`StreamingParser` — HTML5 tree construction makes
    /// incremental delivery impossible per the spec, not a library choice;
    /// commonmark-fmt's `StreamingParser` is a sanctioned pulldown-cmark
    /// exemption per CLAUDE.md). This is the *only* path that may be used to
    /// mean "this check will never exist" — it must cite the documented
    /// reason, not be used to dodge writing a check that should exist. A
    /// crate that simply hasn't built the API yet, with no structural
    /// barrier stopping it (e.g. csv-fmt/tsv-fmt/ris/native, all of which are
    /// flat record/line-oriented formats a streaming reader or writer could
    /// trivially be added to), is [`ApiState::NotYetWired`], never this.
    NotApplicable(&'static str),
    /// The API most likely exists in the `-fmt` crate (or its existence
    /// hasn't been confirmed) but this harness does not check it yet. This
    /// is an honest placeholder, not a claim of health — see TODO.md for
    /// the tracked follow-up. Every format must have an explicit line here
    /// rather than simply not appearing in the harness at all. Also used,
    /// per the `odt`/`docx`/`pptx` precedent below, when the type has been
    /// confirmed by reading the source to not exist in the crate *at all*
    /// yet, but nothing about the format makes it impossible to add —
    /// "not a claim of absence" describes the common case (nobody has
    /// looked), not every case; where absence has been confirmed, the entry
    /// says so explicitly.
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
        // Fixed 2026-07-31: djot_fmt::writer::Writer rewritten from
        // buffer-all-events-then-reconstruct-the-AST to a single shared-buffer
        // write-straight-through design (mirroring rst-fmt's Writer), with three
        // deferred per-line re-indent constructs (Blockquote, DefinitionDesc,
        // ListItem/FootnoteDef — see writer.rs's module doc) and Table collecting
        // rows to compute the header separator's column count/alignments.
        // Byte-identical to emit() over all fixtures, including link-reference
        // definitions (Event::LinkDef, fixture link-reference) and table captions
        // (Event::TableCaption); bytes reach the sink before finish().
        streaming_writer: ApiState::Wired,
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
        // streaming_writer was previously content-correct (Event::Metadata carries
        // #+TITLE:/#+AUTHOR:/#+CUSTOM_KEY: lines) but architecturally hollow
        // (buffer-all-events-then-reconstruct-the-AST) with no incrementality probe
        // wired to catch it — CAPABILITIES claimed Wired while the writer.rs module
        // doc admitted otherwise. Fixed 2026-07-31: rewritten to a single
        // shared-buffer write-straight-through design (mirroring rst-fmt's Writer).
        // Document metadata (Event::Metadata) is a documented, deliberate partial
        // divergence from build()'s exact semantics — build() always moves *all*
        // metadata to the document's very top regardless of source position, which
        // a genuinely incremental writer cannot losslessly replicate without
        // unbounded lookahead; this Writer instead emits each Metadata line
        // write-through, wherever it arrives. Unobservable in the current fixture
        // suite (no fixture has metadata after body content starts) — see
        // writer.rs's module doc and TODO.md. Byte-identical to build() over all
        // fixtures; the incrementality probe (previously missing from this
        // harness) now confirms bytes reach the sink before finish().
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
        // Fixed: texinfo::batch::StreamingParser now processes input in logical
        // top-level units (paragraph, heading, or an @directive...@end directive
        // environment), flushing each unit to the handler as soon as its boundary
        // is confirmed, instead of buffering all fed bytes into a Vec<u8> and only
        // parsing + delivering events inside finish(). Confirmed via the
        // matches-events()-under-adversarial-chunking check over all fixtures plus
        // a deterministic pre-finish incrementality probe (see
        // texinfo_streaming_parser_delivers_events_incrementally in
        // crates/rescribe-fixtures/tests/streaming_apis.rs and the crate's own
        // test_streaming_parser_delivers_before_finish in
        // crates/formats/texinfo/src/batch.rs).
        streaming_parser: ApiState::Wired,
        // Fixed: texinfo::writer::Writer now writes straight through to a single shared
        // buffer per event (mirroring rst-fmt's Writer design) instead of buffering all
        // events and reconstructing the AST in finish(). Confirmed via the
        // byte-identical-to-builder check over all fixtures plus a pre-finish
        // incrementality probe.
        streaming_writer: ApiState::Wired,
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
        // Fixed 2026-07-31: StreamingParser rewritten from buffer-all-fed-bytes-then-
        // parse-in-finish() to a true incremental drain, reusing the same SemanticState
        // dispatch machine events()/EventIter already used internally (see events.rs's
        // module doc and StreamingParser::drain's doc comment) — the same
        // "rebuild a Reader over just the unconsumed tail, Err(Syntax) means wait for
        // more bytes" technique docbook-fmt's StreamingParser pioneered. The one
        // XML-generic wrinkle (plain text terminates at either `<` or genuine EOF,
        // indistinguishable from a slice-bounded reader) is handled the same way
        // docbook-fmt handles it: a text token that consumes every currently-buffered
        // byte is held back until more input confirms it's actually complete. Confirmed
        // via a synthetic multi-section document under whole/single-byte/chunks-of-N/
        // mid-UTF-8-character-split adversarial chunking, byte-for-byte equal to
        // events() on the whole input; peak memory measured flat (~3.09KB) across a
        // 10x input-size increase (200 vs 2000 sections), not O(full input).
        streaming_parser: ApiState::Wired,
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
        // Fixed 2026-07-31: StreamingParser rewritten to be genuinely incremental. feed()
        // accumulates lines into a small pending buffer and re-runs the same block-boundary
        // logic parse()/events() use (parse::BlockCursor, extracted from the shared
        // parse_next_block() step function) over just that pending tail; a block is flushed to
        // the handler the moment a later buffered line proves its boundary can't change, and
        // only the still-open block's lines stay buffered. Memory: O(largest block), not
        // O(full input).
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event (shared output
        // buffer, O(nesting depth) frame stack) instead of buffering into a Vec<TextileEvent>
        // and delegating to emit::build() inside finish(). Byte-identical-to-builder confirmed
        // across fixtures. One pre-existing quirk replicated exactly, not "fixed": a Paragraph
        // directly inside a Blockquote or list item silently ignores its own align/attrs (an
        // O(1) parent-frame lookup, ~4.4x faster than build()).
        streaming_writer: ApiState::Wired,
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
             events for the same run (fixtures/commonmark/adv-broken-link); (3) newly found while \
             rewriting the streaming writer (2026-08-03): `EventIter`'s `StartList` always \
             reports `tight: true`, per its own \"Tightness is unknown until we see paragraphs; \
             emit with tight=true tentatively\" comment in events.rs — never corrected once the \
             real tightness becomes known, so any loose list (e.g. \
             fixtures/commonmark/rare-loose-list, integration-loose-list-item, \
             rare-tight-vs-loose) diverges from parse()'s AST. This test's loop only records the \
             *first* fixture that diverges, so this third bug was masked behind (1)/(2) in the \
             test's own failure message even though it was already silently present.",
        ),
        streaming_parser: ApiState::NotApplicable(
            "commonmark-fmt's StreamingParser buffering all input before parsing with \
             pulldown-cmark is the sole documented CLAUDE.md exemption (pulldown-cmark requires \
             the full input as &str); see crates/formats/commonmark-fmt/src/lib.rs's \
             \"Limitations\" doc comment and src/batch.rs's own \"# Limitation\" section",
        ),
        streaming_writer: ApiState::KnownFailure(
            "commonmark_fmt::writer::Writer was rewritten (2026-08-03) to a genuine \
             write-through streaming design — single shared `out: String` buffer, `O(nesting \
             depth)` frame stack, per-top-level-block flush; see the crate's own module doc for \
             the full write-through/deferred construct classification. The buffer-then-emit \
             defect this entry used to track is fixed: byte-identical-to-builder confirmed for \
             every construct in the crate's own tests. One residual divergence remains, and it \
             is not this writer's bug: fixtures/commonmark/image fails as a downstream \
             consequence of the events()/EventIter Text/StartImage ordering bug documented in \
             the \"events\" field above — the leaked Text(alt) arrives before Frame::Image is \
             pushed, so the Writer's discard-inside-image logic never sees it. See the \
             commonmark/streaming_writer KnownFailure entry.",
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
            "shares commonmark-fmt's now-genuinely-streaming Writer with the \"commonmark\" \
             format entry above, including its one residual events()-caused divergence",
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
            "shares commonmark-fmt's now-genuinely-streaming Writer with the \"commonmark\" \
             format entry above, including its one residual events()-caused divergence",
        ),
    },
    // docbook-fmt, jats-fmt, tei-fmt are byte-identical in implementation
    // shape (verified via `diff` across batch.rs/writer.rs: only doc
    // comments and AST/event type names differ), so the same bugs — found by
    // this harness, not present in any prior audit — applied to all three
    // identically. All three ARE genuinely independent/incremental
    // implementations (events() pulls tokens straight off quick_xml::Reader
    // without building an AST; the streaming Writer calls quick_xml::Writer
    // directly per event with no buffering) — these are logic bugs, not
    // architectural hollowness. The streaming_parser mismatched/unmatched-
    // end-tag bug (StreamingParser silently accepting malformed XML that
    // events() correctly rejects) has since been fixed identically in all
    // three via a hand-tracked open-element stack in batch.rs; the events()
    // entity-coalescing gap has also since been fixed (see below); the
    // streaming_writer auto-close-recovery gap remains open.
    FormatCapabilities {
        format: "docbook",
        // Fixed 2026-07-30: events()/EventIter used to emit one Text event per resolved
        // character/predefined/DTD entity (e.g. `&amp;` decoded to its own Text("&") event)
        // instead of merging it into the surrounding text run the way parse()'s AST does
        // (parse.rs's `current_text` accumulator). EventIter::next() now accumulates a run
        // of adjacent text-equivalent tokens via a one-token lookahead (`pending` field) and
        // dispatches one merged Text event, matching parse(); StreamingParser's batch.rs
        // drain() got the equivalent fix (a `pending_text` field persisting across drain()
        // calls). Confirmed via fixture adv-entity-references (a &amp; b &lt; c &gt; d &apos;
        // e&apos; &quot;f&quot; now yields one Text event, not six).
        //
        // events is still KnownFailure, though, because a *different*, already-tracked gap
        // (the streaming_writer entry below: events() lacks parse()'s malformed-XML
        // auto-close recovery) also surfaces through this same events()-vs-
        // events_from_doc(&parse()) equivalence check, via fixture adv-malformed-xml — that
        // fixture has no entities in it at all, so this is not a re-regression of the
        // entity-coalescing bug, just a second, unrelated cause tripping the same check.
        events: ApiState::KnownFailure(
            "parse() auto-closes unclosed elements on malformed XML (synthetic EndElement \
             nodes, see parse.rs's 'unclosed element' diagnostics) but events()/EventIter has \
             no such recovery — it stops at the genuine XML parse error with no synthetic \
             close events — so events() diverges from events_from_doc(&parse()) for fixture \
             adv-malformed-xml specifically (not an entity-coalescing issue: that part is \
             fixed, see the events field's doc comment above). Same root cause as the \
             streaming_writer KnownFailure below.",
        ),
        // StreamingParser's drain() (batch.rs) still sets check_end_names=false and
        // allow_unmatched_ends=true on its per-drain-call quick_xml::Reader — that part is
        // architecturally necessary, since each drain() call only ever sees the unconsumed
        // tail, not the full document. But it now tracks open-element names itself in a
        // `Vec<String>` field that persists across drain() calls (the same "survives
        // multiple feed() calls" shape as entity_resolver), and validates every End event
        // against that stack by hand: a mismatch or an End against an empty stack pushes a
        // Diagnostic and stops draining for good, mirroring parse()'s "fatal diagnostic +
        // stop" behavior on a genuine XML error. Fixed and confirmed passing (fixtures
        // adv-malformed-xml, adv-unmatched-end-tag).
        streaming_parser: ApiState::Wired,
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
        // Fixed alongside docbook-fmt (byte-identical events.rs shape); entity-coalescing is
        // confirmed working. Still KnownFailure for the same unrelated malformed-XML
        // auto-close-recovery reason as docbook's events entry above (jats has its own
        // adv-malformed-xml fixture, a truncated-input case, that trips the same gap).
        events: ApiState::KnownFailure(
            "shares docbook-fmt's implementation (byte-identical events.rs shape): events() \
             lacks parse()'s malformed-XML auto-close recovery, diverging on fixture \
             adv-malformed-xml; see the \"docbook\" events KnownFailure for the root cause",
        ),
        // The mismatched/unmatched-end-tag bug shared with docbook-fmt/tei-fmt (see the
        // "docbook" streaming_parser comment above) is fixed here too (confirmed passing on
        // fixtures adv-mismatched-end-tag, adv-unmatched-end-tag). The remaining failure this
        // entry used to track was a probe-methodology artifact, not a StreamingParser defect:
        // the adversarial-chunking test's incrementality probe fed exactly the first half of
        // fixture adv-malformed-xml (40 of 81 bytes), landing mid-attribute-value inside the
        // still-open root <article ...> start tag, so zero events at that exact split point was
        // the correct, spec-conforming answer — not a bug. Fixed 2026-07-31 by replacing the
        // fixed-50%-byte-split probe with a hand-built synthetic sample with a provably complete
        // prefix (same fix already applied to fb2-fmt/texinfo/xwiki/textile-fmt/pod-fmt); the
        // new probe passes, confirming the implementation itself was already correct.
        streaming_parser: ApiState::Wired,
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
             ast_to_events projection purely from the AST. Separately: events.rs's \
             parse_csi_event 'm' arm used to emit ResetStyle whenever the resulting style was \
             empty, conflating \"style ended up empty\" with \"an explicit reset code was \
             seen\" — an unrecognized/no-op SGR group (e.g. \\x1b[999m) that left style \
             unchanged still emitted a spurious ResetStyle event. Fixed 2026-07-30: \
             apply_sgr_event() now returns whether it actually applied an explicit reset code \
             (`0` or empty), and only that triggers ResetStyle; a no-op/unrecognized code now \
             emits SetStyle(unchanged style) instead, never a spurious reset.",
        ),
        // Two of the three originally-tracked bugs are fixed (2026-07-30): (1)
        // drain_complete() used to build a brand-new EventIter with style: Style::default()
        // on every drain, losing running style state across chunk boundaries — now uses
        // EventIter::new_with_style()/current_style() to carry style forward by hand,
        // mirroring the docbook-fmt entity_resolver pattern. (2) inherited the
        // spurious-ResetStyle bug from events() (see the events field above), also fixed.
        // Fixing (1) uncovered a third, previously-masked, unrelated bug (reproduces even
        // with an unchanging empty style throughout): drain_complete()'s fresh EventIter per
        // call also meant adjacent Text events from separate calls were never merged, so
        // fine-grained (e.g. single-byte) chunking fragmented one text run into one Text
        // event per call — fixed via a `pending_text` accumulator, same shape as the
        // docbook-fmt entity-coalescing fix.
        streaming_parser: ApiState::KnownFailure(
            "after the three fixes above, one genuinely distinct, unfixed bug remains, found \
             via the same adversarial-chunking equivalence check (fixtures hyperlink, \
             rare-hyperlink-uri, both only under fine-grained chunking like single_byte or \
             chunks_of_3 — not under whole-input): EventIter::next() treats an OSC 8 \
             hyperlink as a single atomic token by scanning forward, within one next() call, \
             all the way to its matching closing OSC 8 sequence (`\\x1b]8;;\\x07`) — but \
             find_safe_boundary() (batch.rs) has no concept of this pairing, and calls a \
             complete, well-formed *opening* OSC 8 sequence a safe boundary on its own. Under \
             chunking, drain_complete() then parses just that opening sequence in isolation, \
             finds no closing sequence within the truncated slice, and emits a Hyperlink event \
             with an empty text field immediately instead of buffering through to the close \
             — the link \
             text and the close sequence then get parsed separately (as plain Text and a \
             stray RawEscape) on a later call. Fixing this needs find_safe_boundary() taught \
             to recognize an opening OSC 8 hyperlink sequence and treat everything up to its \
             matching closer as one unsplittable unit (the same kind of \"buffer until a \
             semantic close is seen\" logic html-fmt's StreamingParser already needs for tree \
             construction) — a real architectural extension, not a small bug fix, so left open \
             here rather than guessed at.",
        ),
        streaming_writer: ApiState::KnownFailure(
            "downstream of a *different*, still-open issue than the (now-fixed) spurious- \
             ResetStyle bug: for fixture adv-unknown-sgr, `\\x1b[999m` (no-op, no ResetStyle) \
             followed by `\\x1b[0m` (a genuine explicit reset) means events() now correctly \
             emits exactly one ResetStyle, for the real trailing reset — but parse()'s AST has \
             no node for either SGR group at all (see the \"ansi\" events entry's first \
             paragraph), so build() reconstructs \"Text\\n\" while the streaming Writer, fed \
             by events() and writing every ResetStyle as literal \\x1b[0m bytes, reconstructs \
             \"Text\\x1b[0m\\n\" — a legitimate escape sequence from the source that parse()'s \
             AST silently drops with no raw-preservation fallback, not a StreamingParser or \
             Writer defect",
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
            "the resource-loss defect this entry originally tracked (OdfEvent had no variant \
             carrying mimetype/meta/styles/images) is fixed: OdfEvent now has Mimetype, Meta, \
             AutomaticStyle, NamedStyle, ListStyle, PageLayout, and EmbeddedImage variants, \
             produced by events::extract_events reading mimetype/meta.xml/styles.xml/ \
             content.xml's <office:automatic-styles>/Pictures+media (via parser.rs's \
             read_zip_text/parse_meta_xml/parse_styles_xml/parse_auto_styles_block, now \
             pub(crate)) and consumed by batch::DocBuilder::process. Two directly-adjacent bugs \
             that were blocking verification of that fix were fixed alongside it: StartFrame had \
             no width/height (draw:frame's svg:width/svg:height were silently dropped, found via \
             fixture adv-corrupt-image) and self-closing <office:text/> was not recognized by \
             events.rs's own quick_xml scan (found via fixture adv-empty). However, the \
             byte-identical-to-builder check still fails: it found the OdfEvent vocabulary has \
             many other, unrelated pre-existing gaps for inline/block body content — \
             office:annotation, text:bookmark(-start), field elements (text:date, \
             text:page-number, etc.), text:soft-hyphen, text:soft-page-break, table cell \
             col-span/row-span, footnote/endnote citations, draw:text-box inside a text-body \
             draw:frame (image captions), and at least one heading-only divergence — none of \
             which relate to mimetype/meta/styles/images. 12 of 66 odt fixtures diverge \
             (annotation, bookmark, colspan-rowspan, endnote, footnote, footnote-formatted, \
             heading, image-caption, non-breaking-space, path-deeply-nested-table, soft-hyphen, \
             text-box). Completing OdfEvent's vocabulary to cover all of this is a substantially \
             larger, separate body of work than the resource-loss defect this entry originally \
             tracked; see TODO.md",
        ),
    },
    FormatCapabilities {
        format: "tei",
        // Fixed alongside docbook-fmt (byte-identical events.rs shape); entity-coalescing is
        // confirmed working. Still KnownFailure for the same unrelated malformed-XML
        // auto-close-recovery reason as docbook's events entry above.
        events: ApiState::KnownFailure(
            "shares docbook-fmt's implementation (byte-identical events.rs shape): events() \
             lacks parse()'s malformed-XML auto-close recovery, diverging on fixture \
             adv-malformed-xml; see the \"docbook\" events KnownFailure for the root cause",
        ),
        // Fixed alongside docbook-fmt/jats-fmt (byte-identical batch.rs shape); see the
        // "docbook" streaming_parser comment above. Confirmed passing (fixture
        // adv-malformed-xml, adv-unmatched-end-tag).
        streaming_parser: ApiState::Wired,
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
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to emit() inside finish(). Fully
        // write-through, no deferred constructs found. Byte-identical-to-builder confirmed;
        // ~2.4-4.3x faster and ~1000x lower peak memory than the old buffer-then-emit path.
        streaming_writer: ApiState::Wired,
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
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to build() inside finish(). Fully
        // write-through, no deferred constructs found. Byte-identical-to-builder confirmed;
        // ~2.4-4.3x faster and ~1000x lower peak memory than the old buffer-then-emit path.
        streaming_writer: ApiState::Wired,
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
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to build() inside finish(). Fully
        // write-through, no deferred constructs found. Byte-identical-to-builder confirmed;
        // ~2.4-4.3x faster and ~1000x lower peak memory than the old buffer-then-emit path.
        streaming_writer: ApiState::Wired,
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
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to build() inside finish(). Nearly
        // all write-through; one O(1) deferred piece: a table row's closing ||/| depends on
        // whether its first cell was a header cell, tracked as Option<bool>, not buffered.
        // Byte-identical-to-builder confirmed; ~1.9-2.5x faster, peak memory 176MB -> 4.2KB
        // (~41,700x) on a 50,000-section synthetic doc.
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "mediawiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to emit() inside finish(). Mostly
        // write-through with two genuine deferrals: (1) Link text is only the top-level Text
        // children (nested Bold/Italic contribute nothing per the AST's own build_inline
        // logic), accumulated into a small owned String and rendered once at EndLink; (2) the
        // document-level trailing-whitespace collapse (emit() does trim_end() + "\n" once) is
        // approximated by holding back the trailing-whitespace run at flush() and always
        // emitting exactly one final "\n" at finish(). Byte-identical-to-builder confirmed;
        // ~1.66x faster, peak memory 176MB -> 4.3KB (~40,700x) on a 50,000-section doc.
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "tikiwiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to build() inside finish(). Fully
        // write-through, no deferred constructs found. Byte-identical-to-builder confirmed.
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "twiki",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to build() inside finish(). Fully
        // write-through except Link (plain-text label buffer, since the label strips
        // formatting). The byte-identical test caught a real, previously-latent ordering bug
        // independent of the rewrite itself: build_list_items wrote an item's own newline
        // before recursing into nested lists instead of after; fixed with a wrote_own_line
        // flag. ~3.9x faster (122->483 MB/s).
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "vimwiki",
        events: ApiState::Wired,
        // Fixed 2026-07-30: vimwiki_fmt::parse::Parser::parse_list (parse.rs) had the same
        // "loop condition only checks that some marker matched" defect as zimwiki's/markua's
        // parse_list (independently discovered here, not chunk-boundary-related — it
        // reproduced even under the 'whole input, one feed() call' chunking, fixture
        // 'oracle') — a blank-line-separated unordered list, then ordered list, then
        // unordered checklist (checklists are list items with a checkbox marker prefix,
        // using the same bullet/numbered/hash marker detection) got merged by parse()/
        // events() into ONE Block::List for all 8 items, tagged with the first group's
        // `ordered` value, while batch::StreamingParser's blank-line block-splitter
        // correctly emitted three separately-typed lists. Now that parse_list itself stops
        // at a marker-type change, parse()/events() and StreamingParser agree (fixture
        // vimwiki/int-mixed-list-markers, confirmed passing under adversarial chunking).
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to collect_doc_from_events() +
        // build() inside finish(). Fully write-through except Blockquote (dissolves/merges
        // paragraph children, drops other block kinds — replicated exactly) and Link
        // (equality-based separator). Byte-identical-to-builder confirmed; ~5.4x faster
        // (83->447 MB/s).
        streaming_writer: ApiState::Wired,
    },
    // xwiki's events() is a genuinely lazy pull-iterator over &XwikiDoc
    // (EventIter::next() walks a frame stack on demand, events.rs:168-385),
    // unlike zimwiki/markua/muse-fmt below which eagerly materialize a
    // Vec/VecDeque before iteration begins.
    FormatCapabilities {
        format: "xwiki",
        events: ApiState::Wired,
        // Fixed 2026-07-31 (reader): StreamingParser rewritten from a bare
        // `buf.extend_from_slice`-then-parse-in-finish() wrapper to a genuine
        // block-at-a-time incremental parser. `feed()` accumulates one top-level
        // block (paragraph/heading/list/table/code/macro/quote block — the same
        // boundaries `parse::Parser::parse`'s dispatch loop uses) and flushes it
        // (reparsed in isolation + walked with `events::events`) as soon as its
        // boundary is confirmed, instead of buffering the whole document. Peak
        // memory measured flat across a 10x input-size increase (~1.97 KB
        // regardless of document size) vs. the old implementation's ~9.8x
        // near-linear growth (1.30 MB -> 12.74 MB) for the same synthetic input.
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31 (writer, same day as the streaming_parser fix above but a
        // separate change): Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to emit::build() inside finish().
        // Fully write-through except Link, which needed genuine reordering ([[label>>url]] —
        // label before url, url known first) via holding url on the frame.
        // Byte-identical-to-builder confirmed; ~6.4x faster (59->382 MB/s).
        streaming_writer: ApiState::Wired,
    },
    // zimwiki's events() is parse()+eager-materialize-then-walk (EventIter::new
    // calls parse::parse(input) then walks into a Vec before returning any
    // event, events.rs:94-102) — the same narrower "Wired" claim as asciidoc.
    // StreamingParser, like xwiki's (fixed 2026-07-31) but unlike muse-fmt's,
    // is REAL incremental (feed_line tracks verbatim-block/blank-line
    // boundaries and calls emit_block() during feed(), batch.rs:93-152).
    FormatCapabilities {
        format: "zimwiki",
        events: ApiState::Wired,
        // Fixed 2026-07-30: zimwiki::parse::Parser::parse_list (parse.rs) didn't stop
        // consuming list items when the marker type changed — its loop condition only
        // checked "some marker matched," not that it matched the list's own `ordered`
        // flag, and its blank-line arm skipped blank lines with `continue` instead of
        // breaking — so a blank-line-separated unordered list immediately followed by an
        // ordered list got merged by the *whole-document* parser into ONE `Block::List`
        // tagged with the first item's `ordered` value. `StreamingParser`'s blank-line
        // block-splitter hard-split at that same boundary and did NOT reproduce the
        // merge, so the two disagreed. Now that parse_list itself stops at a marker-type
        // change, parse() and StreamingParser agree (fixture zimwiki/int-mixed-list-markers,
        // confirmed passing under adversarial chunking).
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to emit::build() inside finish().
        // Fully write-through except the link "|" separator (in-place insert).
        // Byte-identical-to-builder confirmed; ~5.7x faster (75->432 MB/s).
        streaming_writer: ApiState::Wired,
    },
    // markua's events() is parse()+eager-tree-build-then-walk (EventIter::new,
    // re-exported from parse.rs not events.rs, runs the full recursive-descent
    // Parser::parse() before any event is returned, parse.rs:969-985) — the
    // same narrower "Wired" claim as asciidoc/zimwiki. StreamingParser, like
    // xwiki's (fixed 2026-07-31) but unlike muse-fmt's, is REAL incremental
    // block-boundary segmentation (fenced-code-aware feed_line, batch.rs:108-152).
    FormatCapabilities {
        format: "markua",
        events: ApiState::Wired,
        // Fixed 2026-07-30, identical shape and cause to zimwiki's fix above:
        // markua::parse::Parser::parse_list (parse.rs) had the same "loop condition only
        // checks that some marker matched, never that it matches this list's `ordered`
        // flag" defect (independently discovered from zimwiki's, but from the same
        // copy-paste lineage), so a blank-line-separated unordered list immediately
        // followed by an ordered list got merged into ONE `Block::List` tagged with the
        // first item's `ordered` value. parse() and StreamingParser now agree (fixture
        // markua/int-mixed-list-markers, confirmed passing under adversarial chunking).
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to the rst-fmt pattern (shared output buffer +
        // frame stack, in-place inserts for figure captions) instead of buffering into a
        // Vec<OwnedMarkuaEvent> and delegating to emit::emit() inside finish(). Found and
        // fixed a real, independent bug in the process: the old writer's EndFigure always
        // built caption: vec![], silently dropping captions — the new writer carries the
        // caption correctly. ~4-7x faster on a 640k-event synthetic doc. Separately (not
        // caught by this harness's fixture loop, since parse() never constructs it):
        // MarkuaDoc::title/author/description are permanently None because parse() never
        // populates them from any Markua syntax — a reader-side gap, out of scope here.
        streaming_writer: ApiState::Wired,
    },
    // muse-fmt's events() takes &MuseDoc (like xwiki) but eagerly materializes
    // a VecDeque in EventIter::new (events.rs:211-220) rather than pulling
    // lazily.
    FormatCapabilities {
        format: "muse",
        events: ApiState::Wired,
        // Fixed 2026-07-31: StreamingParser rewritten to a genuinely incremental
        // line-buffered block splitter (batch.rs) instead of buffer-then-finish. feed()
        // accumulates lines only until a top-level block boundary is confirmed (blank
        // line, a line starting a different block kind, a tag block's own closing tag,
        // or a single-line construct), then immediately re-parses just that block's text
        // via the new crate::parse::parse_blocks (parse.rs — runs Parser::parse_block_loop
        // without the document-header phase, so a '#'-led line mid-document is never
        // misread as a header directive) and forwards its events — before finish() is ever
        // called. Boundary classification reuses pure predicate functions
        // (heading_level/is_over_leveled_heading/is_horizontal_rule/
        // is_unordered_list_start/is_ordered_list_item/is_definition_list_line/
        // is_indented_code_start/is_footnote_def_start/tag_open_close, all now pub(crate)
        // in parse.rs) that Parser::parse_block_loop itself now also calls, so the
        // splitter's boundary decisions cannot drift from the parser's own dispatch order.
        // Memory: O(largest block), confirmed by a thread_local!-allocator peak-memory
        // guard (10x paragraph count -> peak ratio stayed well under 4x, vs the old
        // O(full document) buffering). Muse's own tag blocks do not support nesting in
        // parse() itself (each stops at the *first* occurrence of its own closing tag);
        // StreamingParser intentionally reproduces that, not "fixes" it, to stay aligned
        // with events(). See crates/formats/muse-fmt/src/batch.rs's adversarial
        // (whole/single-byte/chunk-of-N/mid-UTF-8-char) tests.
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedMuseEvent> and delegating to emit::build() inside
        // finish(). Straight-through except Paragraph's O(1) parent-lookup terminator and
        // O(field-count) metadata buffering. The previously-tracked expressiveness gap is
        // also fixed: MuseEvent now has a Metadata variant (events.rs), added because
        // parse() genuinely populates title/author/date/desc/keywords from real Muse syntax,
        // and both batch.rs consumers pick it up.
        // crates/rescribe-fixtures/tests/streaming_apis.rs's hand-rolled muse_ast_to_events()
        // was updated to emit the matching Metadata event, so
        // muse_events_equals_ast_projection_over_all_fixtures already covers it. ~1.8x faster.
        streaming_writer: ApiState::Wired,
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
             via crate::events::events(&text), and events() always wraps its output in its own \
             StartDocument/EndDocument pair — the same \"re-parse each block alone, lose \
             cross-block context\" root cause already tracked for org-fmt/asciidoc/fountain. \
             Bulk events() over the whole document emits exactly one such pair, but \
             StreamingParser emits one per accumulated block, diverging on every fixture with \
             more than one top-level block (heading-h2, horizontal-rule, path-many-sections, \
             comp-heading-list, definition-list, etc. — not limited to definition-list, where \
             the blank line between items, batch.rs's feed_line blank-line branch, \
             batch.rs:143-150, ends the accumulated block, splitting one multi-item \
             DefinitionList into two DefinitionList event pairs). The related \
             document-header-specific defect — an isolated re-parse of the 3-line header block \
             re-triggering try_parse_header() and producing a spurious *empty* \
             StartDocument/EndDocument pair with title/author/date silently dropped — is fixed: \
             Event::Header was added and StreamingParser's try_emit_header() recognizes the \
             first block directly via Parser::try_parse_header instead of falling through to \
             the generic re-parse path; see TODO.md",
        ),
        // Fixed 2026-07-31: t2t::writer::Writer rewritten from
        // buffer-all-events-then-reconstruct-the-AST to a single shared-buffer
        // write-straight-through design (mirroring rst-fmt's Writer). Every t2t
        // construct turned out to be write-straight-through with no generic
        // "blank line between siblings" rule needed at all — each block variant's
        // own emit.rs arm already writes its complete trailing whitespace, so
        // consecutive children just concatenate (see writer.rs's module doc for
        // the full writeup, including the three different framings a Paragraph
        // gets depending on whether its parent is Blockquote/ListItem/
        // DefinitionDesc/anything else). Byte-identical to emit() over all
        // fixtures including document-header (Event::Header), with bytes
        // reaching the sink before finish().
        streaming_writer: ApiState::Wired,
    },
    // pod-fmt's events() is `pod_fmt::events()` (src/lib.rs) — `parse(input)`
    // then an eager `.collect()` of a lazy frame-stack `EventIter` walk of
    // the AST parse() already built, not an independently implemented
    // reader (same pattern as t2t/asciidoc above). See the comment above the
    // check in tests/streaming_apis.rs.
    // Fixed 2026-07-31 (reader side): pod_fmt::batch::StreamingParser rewritten from
    // buffer-all-bytes-then-parse-on-finish() to a genuine line-buffered block-splitting
    // state machine (feed_line/State in batch.rs), mirroring rst-fmt's batch.rs shape:
    // headings/=for/=encoding/stray commands flush as single-line blocks, =over/=back lists
    // are tracked by nesting depth (a =cut at any depth unwinds the whole list, matching
    // parse_list's own unconditional-break-propagates-up behavior), =begin/=end regions
    // accumulate to their literal =end line (not =cut-aware, matching parse_begin_end's raw,
    // un-POD-interpreted content), and ordinary paragraphs/verbatim blocks flush on their
    // parse.rs boundary conditions. The one piece of state carried across flushed blocks is
    // `in_pod` (parse.rs's own cross-block flag, set by =pod/=head/=over, cleared by =cut) —
    // paragraph/verbatim re-parses get a synthetic leading "=pod" line to reproduce it since
    // an isolated re-parse otherwise starts with in_pod=false and would drop the content.
    // Peak memory confirmed flat via a thread_local! allocator probe (crate::alloc_probe,
    // shared with writer.rs's pre-existing one — only one #[global_allocator] per test
    // binary): a synthetic 200-section vs 2000-section (10x) multi-block document holds
    // ~2.25 KB peak either way; a throwaway old-vs-new comparison (not part of the crate)
    // measured the prior buffer-then-finish implementation at ~12-13x the input byte count
    // (328 KB @ 25 KB input, 3.1 MB @ 261 KB input, 35.2 MB @ 2.67 MB input) against the new
    // implementation's flat ~2.25-2.29 KB across the same three sizes — roughly a 15,000x
    // reduction at the largest size. Confirmed correct via adversarial-chunking equivalence
    // tests in batch.rs's own test module (whole/single-byte/chunks-of-N/mid-UTF-8-char) and
    // this harness's fixture-driven equivalence check below, which still passes over all
    // fixtures (the rewrite does not diverge from events(), same as before).
    //
    // The original feed()-before-finish() incrementality probe (a fixed 50%-of-total-bytes
    // split of each real fixture) reported failure on 35 of 36 checked pod fixtures — not
    // from any remaining buffer-then-finish behavior, but because the pod fixture suite is
    // overwhelmingly single-block per fixture by design (fixtures/spec.md's one-focused-
    // construct convention: one heading, one paragraph, or one =over/=back list per file). A
    // single block's events cannot be emitted until that block's own boundary (a blank line,
    // the matching =back, or EOF) is reached, so a fixed byte-count split lands mid-block for
    // such fixtures regardless of implementation quality — the same probe-methodology gap
    // already fixed this session for fb2-fmt/texinfo/xwiki (see
    // `fb2_streaming_parser_matches_events_and_is_incremental` in tests/streaming_apis.rs for
    // the precedent). Replaced with a hand-built probe input with a guaranteed-complete
    // prefix (a full heading + a full paragraph, each provably complete by parse.rs's own
    // boundary rules) followed by deliberately unterminated trailing content — this passes
    // cleanly, confirming real feed()-before-finish() delivery.
    FormatCapabilities {
        format: "pod",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to emit::build() inside finish().
        // Entirely write-straight-through — POD has no computed prefixes and Link/verbatim
        // blocks are self-contained. Byte-identical-to-builder confirmed; ~3.3x faster.
        streaming_writer: ApiState::Wired,
    },
    // haddock-fmt's events() is `parse(input)` then a lazy frame-stack
    // EventIter walk of the AST — not an independently implemented reader
    // (same pattern as t2t/pod/asciidoc above). See the comment above the
    // check in tests/streaming_apis.rs.
    FormatCapabilities {
        format: "haddock",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to emit::build() inside finish().
        // Straight-through except Property's lazy description-separator space (O(1) bool
        // flag). The byte-identical test caught a real, independent bug: events() emits a
        // redundant Text child inside Link that the builder never reads — fixed by
        // suppressing it via a dedicated frame. ~2.9x faster.
        streaming_writer: ApiState::Wired,
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
        // Fixed 2026-07-30 (both bugs, same root fix): emit_block() used to re-parse each
        // accumulated block via crate::events::events(&text) and forward every event it
        // yielded — including that call's own StartDocument/EndDocument pair — with no
        // filtering, so it emitted one such pair PER block instead of one for the whole
        // document (the dominant failure mode, not an edge case). Separately,
        // parse_title_page() ran unconditionally on every re-parse with no "is this really
        // the first block" guard, so a body block matching `key: value` for one of the 9
        // recognized title-page field names got misread as metadata and its content
        // silently swallowed (parse_screenplay() never saw those lines at all — the same
        // defect class as t2t's try_parse_header()).
        //
        // Fix: StreamingParser now owns exactly one StartDocument (dispatched in new())/
        // EndDocument (dispatched in finish()) pair, filtering both out of every per-block
        // re-parse's forwarded events. Only the first accumulated block is parsed via the
        // full crate::events::events() (so real title-page metadata is still recognized
        // there); every later block goes through the new crate::events::events_body(),
        // which calls the new crate::parse::parse_screenplay_only() — skipping title-page
        // detection entirely rather than just filtering its output, since filtering alone
        // can't recover content that parse_title_page() already consumed into metadata.
        streaming_parser: ApiState::Wired,
        // Fixed 2026-07-31: Writer rewritten to emit incrementally per event instead of
        // buffering into a Vec<OwnedEvent> and delegating to emit() inside finish(). Mostly
        // straight-through; Character/Transition need bounded per-block text buffering
        // (uppercase transform), ensure_blank_line needs a persistent O(1) trailing-newline
        // tracker (out gets cleared between flushes), and title-page metadata needs
        // O(field-count) buffering. The byte-identical test caught a real, independent bug:
        // flush_metadata_if_pending fired on StartDocument itself, permanently discarding
        // metadata before it arrived — fixed. ~1.33x faster than the old buffer-then-emit
        // writer, though still slower than the plain builder (789us vs 357us/iter) — noted
        // honestly rather than chased further.
        streaming_writer: ApiState::Wired,
    },
    // man-fmt's events() is `EventIter::new(&parse(input).0)`
    // eagerly-collected — a lazy AST walk, not an independently implemented
    // reader (same pattern as t2t/pod/haddock/fountain/asciidoc above); per
    // that precedent it's still Wired. See tests/streaming_apis.rs's man-fmt
    // section for the full writeup and directly-verified repro commands for
    // the remaining streaming_parser KnownFailure below.
    //
    // Fixed 2026-07-31 (found while wiring this entry): EventIter's handling
    // of every inline container (Bold/Italic/Superscript/Subscript/Link)
    // pushed a synthetic children-walking Frame::Inlines with
    // `close: CloseKind::Paragraph` as a "dummy" — but the dummy was never
    // actually inert: when that children frame ran out of items it
    // unconditionally emitted a real, spurious `EndParagraph` event
    // (regardless of the true enclosing block kind), landing between the
    // container's content and its real close event
    // (`Text, EndParagraph(spurious), EndBold, EndParagraph(real)` instead of
    // `Text, EndBold, EndParagraph`). Caught immediately by this harness's
    // events()-vs-AST-projection check on fixture `bold` — man-fmt's own
    // pre-existing events.rs tests only asserted `.any(...)` membership, not
    // exact event-sequence order, so this shipped undetected. Fixed by
    // adding a real `CloseKind::None` variant (crates/formats/man-fmt/src/
    // events.rs) that pops its frame without emitting any event, replacing
    // the dummy `CloseKind::Paragraph` in all five inline-container sites.
    //
    // streaming_writer fixed 2026-08-03: two stacked defects. (1)
    // man_fmt::writer::Writer buffered all events into a Vec<OwnedManEvent>
    // and only reconstructed the AST + called emit::build() inside
    // finish() — rewritten to a genuine incremental writer (single shared
    // `out: String` buffer + a frame stack of marks/scalars, flushing each
    // completed top-level block to the sink as soon as it closes), the same
    // shape as t2t-fmt's/fountain-fmt's writers. Only two constructs need
    // bounded (not O(document)) buffering: the `.TH` line's five fields
    // (O(field count), via the new `ManEvent::Metadata` below) and a
    // heading's flattened text (O(heading text length) — `emit.rs`'s
    // `Block::Heading` arm uses `extract_text()`, not `build_inlines()`, so
    // all inline markup inside a heading is dropped, not just re-escaped).
    // (2) ManEvent had no variant carrying document metadata
    // (ManDoc::title/section/date/source/manual), so events()-fed writers
    // always dropped a .TH line's title/section/date/source even once (1)
    // was fixed — collect_doc_from_events (events.rs) always built ManDoc {
    // title: None, section: None, date: None, source: None, manual: None,
    // .. }. Fixed by adding `ManEvent::Metadata { title, section, date,
    // source, manual }`, emitted once by EventIter immediately after
    // StartDocument (mirroring t2t-fmt's `Event::Header`), and consumed by
    // collect_doc_from_events. Directly verified on a .TH TEST 1
    // "2024-01-01" "Version 1.0" input: build() and the events()-fed
    // streaming Writer now both emit
    // '.TH TEST 1 "2024-01-01" "Version 1.0" ""'.
    FormatCapabilities {
        format: "man",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "man_fmt::batch::StreamingParser's emit_block() re-parses each accumulated block in \
             isolation via crate::events::events(&text), and events() always wraps its output in \
             its own StartDocument/EndDocument pair (ManEvent has no way to avoid it), so \
             StreamingParser emits one such pair per accumulated block instead of one for the \
             whole document — the same re-parse-each-block-in-isolation root cause already \
             tracked for t2t-fmt/fountain-fmt, but reproducing on every multi-block fixture \
             rather than needing a specific trigger. Directly verified: feeding \
             \".SH NAME\\ntest\\n\\n.SH DESCRIPTION\\nmore text\\n\" through events() yields 1 \
             StartDocument/14 events total; through StreamingParser yields 2 StartDocument/16 \
             events total. See TODO.md",
        ),
        streaming_writer: ApiState::Wired,
    },
    // rtf-fmt's events() is `SemanticEventIter::new(parse(input).0)` — a
    // lazy frame-stack walk of the AST parse() already built, not an
    // independently implemented reader (same pattern as t2t/pod/haddock/
    // fountain/asciidoc/man above); per that precedent it's still Wired.
    // sem_events::Event only derived Debug before this pass (src/
    // sem_events.rs:29) — added PartialEq here so the exact-sequence
    // events()-vs-AST-projection check in tests/streaming_apis.rs is
    // possible (RtfDoc/Block/Inline/Align/TableRow already derived it).
    // events() now also always brackets its output in a new
    // Event::StartDocument{fonts, colors}/EndDocument pair — StartDocument
    // carries the exact font/color tables build()/emit() compute
    // (rtf_fmt::build_font_map/build_color_map, factored into a shared
    // src/tables.rs used by both paths so they can't diverge), which is what
    // made a genuine events()-fed streaming writer possible (see below).
    //
    // batch::StreamingParser (batch.rs:107-139) buffers all fed bytes into a
    // Vec<u8> in feed() and only calls sem_events::events(&self.buf) once,
    // inside finish() — a confirmed buffer-then-finish stub. Its own module
    // doc (batch.rs:9-18) argues this is an inherent RTF structural
    // constraint (font/color tables must be parsed before body content is
    // interpretable) rather than an implementation shortfall, but per this
    // harness's rule a buffer-then-finish implementation is always
    // KnownFailure, never NotApplicable, regardless of the format's
    // structural excuse. Directly verified: feed() alone, without calling
    // finish(), delivers zero events to the handler for any input. This
    // reader-side gap is unrelated to the streaming_writer fix below and
    // remains open — not part of this pass's scope.
    //
    // streaming_writer is now Wired via a new src/sem_writer.rs Writer that
    // consumes Event/OwnedEvent directly (not TokenEvent) — the gap this
    // entry used to track. It solves the same font/color-table-before-body
    // structural constraint the reader has, but without buffering: since
    // Event::StartDocument now carries the finished tables up front (see
    // above), the writer emits and flushes the \fonttbl/\colortbl header
    // immediately, then writes every top-level block straight through,
    // flushing again whenever its small O(nesting depth) context stack
    // returns to empty — never buffering the whole document. Separately, the
    // low-level writer::Writer/TokenEvent path (writer.rs) had a second,
    // deeper bug: the tokenizer (events.rs::read_control_word) silently
    // discarded whether a control word's optional trailing-space delimiter
    // was present in the source, so no re-serialization policy keyed off
    // name/param could have reconstructed it (confirmed: `\f0 Times` has a
    // delimiter space, `\u65?` does not, both are param-carrying — the
    // spacing is a stylistic per-call-site choice in emit.rs, not a function
    // of token shape). Fixed by adding
    // TokenEvent::ControlWord::had_delimiter_space, populated by the
    // tokenizer and consumed verbatim by writer::Writer instead of any
    // heuristic.
    FormatCapabilities {
        format: "rtf",
        events: ApiState::Wired,
        streaming_parser: ApiState::KnownFailure(
            "batch::StreamingParser buffers all fed bytes in feed() and only calls \
             sem_events::events() once inside finish() (batch.rs:107-139) — a confirmed \
             buffer-then-finish stub. feed() alone, without finish(), delivers zero events to \
             the handler for any input, regardless of the module doc's argument that this is an \
             inherent RTF structural constraint (font/color tables must be parsed before body \
             content is interpretable) rather than an implementation shortfall. See TODO.md",
        ),
        streaming_writer: ApiState::Wired,
    },
    // native/csv-fmt/tsv-fmt/ris audited 2026-08-01 (fourth pass on this
    // harness). All four crates were confirmed, by reading every file under
    // `crates/formats/{crate}/src/` in full (native: single lib.rs; csv-fmt/
    // tsv-fmt/ris: ast.rs + parse.rs + emit.rs, no other modules) and
    // grepping each for `StreamingParser`/`EventIter`/`fn events`/`mod
    // events`/`mod batch`/`mod writer`/`impl Iterator`/`fn next(&mut self)`
    // (zero matches in all four) and each Cargo.toml (zero dependencies in
    // all four — hand-rolled, no library could be hiding a streaming mode
    // behind), to have **only** `parse()` (AST reader) and `emit()`/`build()`
    // (AST builder) — no `events()`, no `StreamingParser<H>`, no streaming
    // writer exist anywhere in any of the four crates. This is not the
    // sanctioned commonmark-fmt/html-fmt shape (a real, structural,
    // documented barrier — pulldown-cmark's `&str` requirement, the HTML5
    // spec's tree-construction mandate): CSV/TSV/RIS are flat record- or
    // line-oriented formats (native is a small recursive tree-of-nodes
    // format) with no such barrier — a chunk-driven reader that yields one
    // `Row`/`RisEntry`/`NativeNode` event at a time, and a writer that
    // streams rows/entries straight to a sink, are both straightforward to
    // add; nobody has built them yet. Per the `ApiState::NotApplicable`
    // doc comment, that makes this `NotYetWired`, not `NotApplicable`, for
    // all three APIs across all four formats — building three new APIs from
    // scratch times four crates is a substantial body of work, not a small
    // in-scope defect fix, so left as an honest gap rather than attempted
    // here. This also corrects a stale claim in `ApiState::NotApplicable`'s
    // own doc comment, written before any of these four crates had actually
    // been read, which speculatively cited "csv/tsv/ris/native have no
    // meaningful streaming writer" as a `NotApplicable` example — that
    // comment has been corrected alongside this entry. See TODO.md.
    FormatCapabilities {
        format: "native",
        events: ApiState::NotYetWired(
            "no events()/EventIter exists in crates/formats/native/src/lib.rs (the crate's only \
             source file) — confirmed by reading the whole file (638 lines: NativeError, \
             NativeDoc/NativeNode/NativeResource/NativeValue, a hand-rolled recursive-descent \
             Parser, and a recursive build()/build_node() builder; no events module, no \
             Iterator impl). Nothing in the format (a small recursive tree-of-nodes debug \
             format) structurally prevents a lazy tree-walk events() like man-fmt's/pod-fmt's; \
             it just hasn't been written",
        ),
        streaming_parser: ApiState::NotYetWired(
            "no StreamingParser<H>/feed()/finish() type exists in lib.rs — confirmed by reading \
             the whole file; only the eager, whole-string parse(input: &str) exists. A chunked \
             parser is plausible (native's grammar is a simple recursive Document{...} bracket \
             structure) but has not been attempted",
        ),
        streaming_writer: ApiState::NotYetWired(
            "no event-driven Writer type exists in lib.rs — confirmed by reading the whole file; \
             only the eager build(doc: &NativeDoc) -> String builder exists, which requires the \
             full NativeDoc tree up front",
        ),
    },
    FormatCapabilities {
        format: "csv",
        events: ApiState::NotYetWired(
            "no events()/EventIter exists anywhere in csv-fmt — confirmed by reading ast.rs, \
             parse.rs, and emit.rs in full (lib.rs only re-exports parse/emit and CsvDoc/Row/ \
             Cell/Diagnostic/Span/Severity) and grepping the crate for EventIter/StreamingParser/ \
             'mod events'/'impl Iterator' (zero matches). CSV is a flat row-oriented format with \
             no cross-row state in parse.rs (each row is independently delimited), so a \
             row-at-a-time events() iterator is straightforward to add; it just hasn't been",
        ),
        streaming_parser: ApiState::NotYetWired(
            "no StreamingParser<H> exists — confirmed by reading parse.rs in full: the only \
             entry point is the eager parse(input: &str) -> (CsvDoc, Vec<Diagnostic>), which \
             scans the whole input string in one call. A chunk-driven row splitter is plausible \
             (RFC 4180 CSV has no construct spanning more than one quoted field across a row \
             boundary) but has not been attempted",
        ),
        streaming_writer: ApiState::NotYetWired(
            "no event-driven Writer exists — confirmed by reading emit.rs in full: the only \
             entry point is the eager emit(doc: &CsvDoc) -> String builder, which requires the \
             full CsvDoc up front. A row-at-a-time streaming writer (write header, then each row, \
             to a sink) is straightforward for CSV's flat grammar; it just hasn't been built",
        ),
    },
    FormatCapabilities {
        format: "tsv",
        events: ApiState::NotYetWired(
            "shares csv-fmt's exact module shape (ast.rs/parse.rs/emit.rs, no events module) — \
             confirmed by reading all three files in full and grepping for EventIter/ \
             StreamingParser/'mod events'/'impl Iterator' (zero matches). Same reasoning as csv's \
             events entry: a row-at-a-time events() iterator is plausible, not yet built",
        ),
        streaming_parser: ApiState::NotYetWired(
            "no StreamingParser<H> exists — confirmed by reading parse.rs in full: only the \
             eager parse(input: &str) -> (TsvDoc, Vec<Diagnostic>) entry point exists. Same \
             reasoning as csv's streaming_parser entry",
        ),
        streaming_writer: ApiState::NotYetWired(
            "no event-driven Writer exists — confirmed by reading emit.rs in full: only the \
             eager emit(doc: &TsvDoc) -> String builder exists. Same reasoning as csv's \
             streaming_writer entry",
        ),
    },
    FormatCapabilities {
        format: "ris",
        events: ApiState::NotYetWired(
            "no events()/EventIter exists anywhere in ris — confirmed by reading ast.rs, \
             parse.rs, and emit.rs in full (lib.rs only re-exports parse/emit and RisDoc/ \
             RisEntry/Diagnostic/Span/Severity plus the ris_type_to_bibtex/bibtex_type_to_ris/ \
             csl_type_to_ris helpers) and grepping the crate for EventIter/StreamingParser/'mod \
             events'/'impl Iterator' (zero matches). RIS is a flat, entry-delimited (TY...ER) \
             citation format with each entry self-contained (parse.rs has no cross-entry state), \
             so an entry-at-a-time events() iterator is straightforward to add; it just hasn't \
             been",
        ),
        streaming_parser: ApiState::NotYetWired(
            "no StreamingParser<H> exists — confirmed by reading parse.rs in full: the only \
             entry point is the eager parse(input: &str) -> (RisDoc, Vec<Diagnostic>). A \
             chunk-driven per-entry splitter (flush at each ER  - line) is plausible; not yet \
             attempted",
        ),
        streaming_writer: ApiState::NotYetWired(
            "no event-driven Writer exists — confirmed by reading emit.rs in full: only the \
             eager emit(doc: &RisDoc) -> String builder exists, requiring the full RisDoc up \
             front. An entry-at-a-time streaming writer is straightforward for RIS's flat \
             grammar; it just hasn't been built",
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
    // No {format}-fmt crate exists at all — confirmed by directory listing
    // `crates/formats/` (2026-07-31): only `rescribe-read-{format}`/
    // `rescribe-write-{format}` adapter crates exist for these ten. There is
    // no standalone library to audit against the five-API contract this
    // harness checks. `latex` is the sharpest case: `rescribe-read-latex`
    // contains a 895-line hand-rolled parser (src/handwritten.rs) plus a
    // separate 662-line tree-sitter-backed parser (src/treesitter.rs)
    // directly in the adapter crate — confirmed by reading both files — a
    // CLAUDE.md "adapter layer must never contain parsing or writing logic"
    // violation, tracked in TODO.md, but out of scope to fix here (that
    // requires extracting a real latex-fmt crate, not a harness-wiring task).
    "latex",
    "csl-json",
    "pandoc-json",
    "ipynb",
    "opml",
    "bibtex",
    "biblatex",
    "typst",
    "endnotexml",
    "epub",
    // `multimarkdown` and `pdf` have NO standalone {format}-fmt crate:
    // `rescribe-read-multimarkdown`'s Cargo.toml depends on `pulldown-cmark`
    // directly (not on `commonmark-fmt`), so its parsing logic lives in the
    // adapter crate itself — a further CLAUDE.md "adapter layer must never
    // contain parsing logic" candidate beyond latex, confirmed by reading
    // its Cargo.toml but not further investigated here (out of scope for
    // this pass; see TODO.md). `pdf` likewise has no standalone `pdf-fmt`
    // crate (only `rescribe-read-pdf`) — confirmed by directory listing
    // `crates/formats/`; not further investigated in this pass. `rtf` was
    // audited this pass — see its `CAPABILITIES` entry below.
    "multimarkdown",
    "pdf",
    // Pandoc output-format variants of html/latex/ooxml-* with zero fixtures
    // (`fixtures/{format}/` has no case directories) — not yet audited and
    // not prioritized per CLAUDE.md's "fixture suite is the primary
    // deliverable" ordering (no fixture corpus exists to check against).
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
        format: "fb2",
        api: "events",
        description: "fb2_fmt events()/EventIter silently drops Event::Metadata for input \
                       lacking a literal <description> element, unlike parse()'s AST which \
                       always carries a (possibly-default) description",
    },
    KnownFailure {
        format: "fb2",
        api: "streaming_writer",
        description: "downstream of the fb2/events KnownFailure: the streaming Writer never \
                       receives Metadata for input lacking <description>, so it never emits a \
                       <description> element, while the AST builder path always writes one",
    },
    KnownFailure {
        format: "commonmark",
        api: "events",
        description: "commonmark_fmt events() has three real bugs: (1) for images, Text(alt) is \
                       delivered before StartImage instead of between StartImage/EndImage as \
                       documented (an EventIter::next() loop-ordering bug); (2) consecutive Text \
                       runs are not coalesced the way parse()'s AST does; (3) StartList always \
                       reports tight: true, never corrected once real tightness is known (see the \
                       CAPABILITIES entry above for detail) — found 2026-08-03 while rewriting the \
                       streaming writer",
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
        description: "commonmark_fmt::writer::Writer was rewritten to a genuine write-through \
                       streaming design (2026-08-03) and is byte-identical to build() for every \
                       construct exercised in its own crate-level tests; the buffer-then-emit \
                       defect this entry used to track is fixed. One residual divergence remains \
                       for fixtures/commonmark/image specifically, and it is not this writer's \
                       bug: it is a downstream consequence of the events()/EventIter \
                       Text/StartImage ordering bug (see the commonmark/events KnownFailure \
                       above) — the leaked Text(alt) event arrives before Frame::Image is pushed, \
                       so the Writer's discard-inside-image logic never sees it and it is emitted \
                       as ordinary paragraph text, producing \"Alt text![Alt text](photo.jpg)\" \
                       instead of \"![Alt text](photo.jpg)\". No writer-side fix can distinguish \
                       that leaked event from genuine prose preceding an image.",
    },
    KnownFailure {
        format: "docbook",
        api: "events",
        description: "entity-coalescing is fixed (2026-07-30); still fails on fixture \
                       adv-malformed-xml for the unrelated, still-open reason: events() lacks \
                       parse()'s malformed-XML auto-close recovery (same root cause as the \
                       streaming_writer entry below)",
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
        description: "shares docbook-fmt's implementation; entity-coalescing is fixed, same \
                       remaining malformed-XML auto-close-recovery gap",
    },
    KnownFailure {
        format: "jats",
        api: "streaming_writer",
        description: "shares docbook-fmt's implementation; same malformed-XML recovery gap",
    },
    KnownFailure {
        format: "tei",
        api: "events",
        description: "shares docbook-fmt's implementation; entity-coalescing is fixed, same \
                       remaining malformed-XML auto-close-recovery gap",
    },
    KnownFailure {
        format: "tei",
        api: "streaming_writer",
        description: "shares docbook-fmt's implementation; same malformed-XML recovery gap",
    },
    KnownFailure {
        format: "odt",
        api: "streaming_writer",
        description: "OdfEvent now carries mimetype/meta/styles/images (the originally-tracked \
                       resource-loss gap is fixed), but the byte-identical-to-builder check \
                       still fails on 12 of 66 odt fixtures over unrelated, pre-existing \
                       OdfEvent gaps for inline/block body content: office:annotation, \
                       text:bookmark(-start), field elements (text:date, text:page-number, \
                       etc.), text:soft-hyphen, text:soft-page-break, table cell col-span/ \
                       row-span, footnote/endnote citations, draw:text-box inside a text-body \
                       draw:frame (image captions), and at least one heading-only divergence; \
                       see the matching CAPABILITIES entry for the full fixture list",
    },
    KnownFailure {
        format: "ansi",
        api: "streaming_parser",
        description: "style-loss-across-chunks and inherited spurious-ResetStyle are both fixed \
                       (2026-07-30); a third, previously-masked bug remains: find_safe_boundary \
                       calls a complete OSC 8 hyperlink *opening* sequence a safe boundary on \
                       its own, not knowing EventIter treats the whole open..close span as one \
                       atomic Hyperlink token, so fine-grained chunking splits it into an \
                       empty-text Hyperlink plus stray Text/RawEscape (fixtures hyperlink, \
                       rare-hyperlink-uri) — needs find_safe_boundary taught to await a \
                       matching OSC 8 closer, a real architectural extension",
    },
    KnownFailure {
        format: "ansi",
        api: "streaming_writer",
        description: "the spurious-ResetStyle bug it depended on is fixed (2026-07-30); still \
                       fails on fixture adv-unknown-sgr, but now for an unrelated, genuine \
                       reason: parse()'s AST has no node for an SGR group that changes nothing, \
                       so build() silently drops a real trailing \\x1b[0m from the source while \
                       the streaming Writer (fed by events(), which does emit a ResetStyle for \
                       it) faithfully re-emits it — a parse()/build() fidelity gap, not a \
                       streaming-API defect",
    },
    KnownFailure {
        format: "man",
        api: "streaming_parser",
        description: "man_fmt::batch::StreamingParser's emit_block() re-parses each accumulated \
                       block in isolation via events(), which always wraps its output in its own \
                       StartDocument/EndDocument pair, so StreamingParser emits one such pair per \
                       block instead of one for the whole document",
    },
    KnownFailure {
        format: "t2t",
        api: "streaming_parser",
        description: "t2t::batch::StreamingParser's emit_block() re-parses each accumulated \
                       block in isolation, and events() always wraps its output in its own \
                       StartDocument/EndDocument pair, so StreamingParser emits one such pair \
                       per block instead of one for the whole document — reproduces on any \
                       fixture with more than one top-level block, e.g. a blank line splitting a \
                       multi-item DefinitionList into one StartDefinitionList/EndDefinitionList \
                       pair per item. (The narrower document-header instance of this — an \
                       isolated re-parse of the header's own 3 lines mis-triggering \
                       try_parse_header() and silently dropping title/author/date — is fixed via \
                       the new Event::Header variant.)",
    },
    KnownFailure {
        format: "rtf",
        api: "streaming_parser",
        description: "rtf_fmt::batch::StreamingParser buffers all fed bytes in feed() and only \
                       calls sem_events::events() once inside finish() (batch.rs:107-139) — a \
                       confirmed buffer-then-finish stub; feed() alone (no finish()) delivers \
                       zero events to the handler for any input",
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
