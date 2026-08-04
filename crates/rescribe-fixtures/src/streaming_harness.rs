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
        // Fixed 2026-08-03: StreamingParser::feed_line used to treat every blank line as
        // ending the accumulated block, splitting a multi-item DefinitionList into one
        // StartDefinitionList/EndDefinitionList pair per item. It now defers the flush
        // decision with a one-line lookahead mirroring parse_definition_list's own
        // peek_line() check (lib.rs): a blank line whose preceding content ends in an
        // indented definition body holds the flush until the next line confirms (non-indented
        // then indented => another item) or denies (anything else => block really ended) a
        // continuation. Fixing it surfaced a second, previously-masked bug (this check only
        // reports the first divergence per run, and definition-list sorted first
        // alphabetically): each flushed block is re-parsed via a fresh EventIter in isolation,
        // which reset heading-level numbering per block instead of carrying it across the
        // whole document (heading-h2 fixture) — fixed via the new
        // EventIter::with_heading_levels/heading_levels, threaded through StreamingParser's
        // emit_block(). Both confirmed via fixtures/rst/definition-list and
        // fixtures/rst/heading-h2 directly, and via the adversarial-chunking harness.
        //
        // Also fixed 2026-08-03 (same session): the same emit_block()-blank-line-flush root
        // cause also split ordinary bullet/numbered lists spanning blank lines — including
        // blank-line-separated nested sub-lists — into multiple StartList/EndList pairs
        // instead of one. feed_line now applies the same one-line-lookahead deferral used for
        // DefinitionLists, but with a list-specific confirmation test: when the block's last
        // accumulated line is a bullet/numbered list-item marker (last_line_is_list_item(), at
        // *any* indentation — this check now runs before the indented-definition-body check,
        // since a nested sub-list item like "  - Nested item" is both), a following blank line
        // no longer flushes immediately. The next line confirms continuation (don't flush) if
        // it is indented (nested sub-list) or itself a list-item marker (any bullet character
        // or numeral/`#.`, at any indentation), otherwise the list genuinely ended and the
        // block flushes normally. This is deliberately permissive rather than replicating
        // parse_bullet_list/parse_numbered_list's full continuation grammar (matching bullet
        // character, indent-relative sub-list detection, etc.): emit_block() always re-parses
        // the whole merged block text through a fresh EventIter, which is the real
        // recursive-descent grammar, so over-merging two adjacent blocks into one emit_block()
        // call is safe — that call still emits however many List blocks/items the real parser
        // decides on for that text, matching what parse()/events() would produce for the same
        // substring. Confirmed via the nested-list and path-deep-list fixtures directly and via
        // the adversarial-chunking harness (rst_streaming_parser_matches_events_under_adversarial_chunking,
        // all fixtures/chunkings green).
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::Wired,
    },
    // djot-fmt's events() and parse() are genuinely independent implementations
    // (direct recursive descent vs a line-driven frame-stack state machine), so
    // the equivalence check compares two real code paths.
    FormatCapabilities {
        format: "djot",
        events: ApiState::Wired,
        // Fixed 2026-08-03: four distinct batch.rs bugs, all in
        // crates/formats/djot-fmt/src/batch.rs unless noted.
        // (a) BlockState::InDiv was a flag, not a counter, so a nested
        // `::: level2` opener was accumulated as content while the first
        // bare `:::` ended the whole block, and every leftover closer was
        // then misread as a new div opener. `InDiv` now carries `depth:
        // usize`, incremented on `::: class` (nested opener) and
        // decremented on bare `:::` (closer), mirroring parse.rs's
        // `find_div_close_generic` (the ground truth `events()`/`parse()`
        // already use for closer matching).
        // (b) link-reference: emit_block() re-parses each block in
        // isolation, so a block's own pre_scan never saw a `[label]: url`
        // definition living in a sibling block. StreamingParser now keeps a
        // persistent `link_defs` table accumulated across every block fed,
        // plus a `deferred`/`deferred_mode` queue: once a block contains an
        // explicit reference-style link (`][`), that block and everything
        // after it is held until `finish()`, then emitted against the fully
        // -accumulated `link_defs` via a new `EventIter::
        // new_with_extra_link_defs` constructor (parse.rs). That
        // constructor initially caused a second bug — each per-block
        // EventIter's end-of-document logic re-emits every entry in
        // `link_defs` as a trailing `LinkDef` event, so injected
        // resolution-only extras were being re-emitted by every block that
        // needed them, producing duplicate `LinkDef` events. Fixed by
        // adding `local_link_def_count` to `EventIter` so only defs found
        // by that input's own pre_scan are drained into trailing events;
        // externally-supplied extras are resolution data only. Bare
        // shortcut references (`[label]` with no second bracket) are not
        // detected by the `][` heuristic and are not deferred — see the
        // caveat on `StreamingParser`'s doc comment in batch.rs.
        // (c) block-attr-on-code: the fence-open and `:::`-open branches of
        // feed_line used to flush a pending `{.python}` block-attribute
        // line as its own throwaway block before starting the new state, so
        // it became a `pending_attr` on a discarded EventIter and was
        // dropped. Both branches now check `block_is_only_pending_attrs()`
        // and, when true, carry the attribute line(s) into the new block
        // instead of flushing them away.
        // (d) definition-list, e2e-rich: the blank-line arm of feed_line
        // unconditionally called emit_block(), splitting a multi-item
        // definition list into one StartDefinitionList/EndDefinitionList
        // pair per item (same bug class as rst-fmt's). Blank lines are now
        // held (`held_blanks`) until the next non-blank line reveals
        // whether they're a real block boundary or a loose-list separator
        // (both sides are list/definition-list starts, via
        // `is_list_start_line`/`block_starts_with_list`).
        // Verified via djot_streaming_parser_matches_events_under_
        // adversarial_chunking (tests/streaming_apis.rs) over all 79
        // fixtures under every adversarial chunking.
        streaming_parser: ApiState::Wired,
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
    // Fixed 2026-08-03: batch.rs's StreamingParser had three distinct bugs, all now fixed.
    // (a) feed_line flushed accumulated lines as soon as it saw a delimited-block marker,
    // so a preceding [source,...]/[verse]/[stem]/[EXAMPLE] attribute line or .Block Title
    // line was re-parsed by emit_block() in isolation and never reached the delimiter (6
    // fixtures: block-title, code-block-source, callout-code, integration-example-code,
    // verse-block, math). feed_line now holds back the trailing run of attribute/title
    // lines and flushes them together with the delimited block as one unit. (b)
    // is_delimited_block_marker only matched runs of identical characters, so the table
    // delimiter |=== was never recognized as a delimited-block opener/closer and the blank
    // line between header and body rows split the table mid-parse (table-header).
    // is_delimited_block_marker now special-cases the `|===` literal (AsciiDoc's only
    // non-identical-run delimiter — parse.rs has no CSV/DSV table variants). (c)
    // StartDocument was only emitted from inside emit_block(), which early-returns on empty
    // block_lines, so empty input yielded zero events instead of the StartDocument/
    // EndDocument pair events("") produces (adv-empty). StartDocument is now emitted
    // unconditionally in new(), and finish() always emits the matching EndDocument. All 85
    // fixtures now pass under adversarial chunking (asciidoc_streaming_parser_matches_
    // events_under_adversarial_chunking), and the crate's own test_streaming_matches_bulk
    // still holds.
    FormatCapabilities {
        format: "asciidoc",
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::Wired,
    },
    // org-fmt's events() is genuinely independent of parse() — the dependency
    // runs the other way (`parse()` drives `EventIter::parse_next_block()`), so
    // the events-vs-AST-projection check compares two real code paths.
    FormatCapabilities {
        format: "org",
        events: ApiState::Wired,
        // Fixed 2026-08-03: batch.rs's feed_line/BlockState machine had three distinct
        // previously-unknown bugs, all downstream of emit_block() (batch.rs) re-parsing each
        // accumulated block in isolation, none covered by the two exceptions batch.rs's module
        // docs sanction (loose lists, drawers containing blank lines). (a) blockquote-nested:
        // BlockState::InSpecialBlock stored only a single expected end keyword with no nesting
        // depth, so a nested #+BEGIN_QUOTE's #+END_QUOTE closed the outer block early —
        // InSpecialBlock now carries a `depth: usize` counter incremented/decremented on
        // same-keyword BEGIN/END lines, mirroring the nesting counter
        // parse::EventIter::parse_block already tracks (parse.rs:521). (b) code-block-name:
        // feed_line called emit_block() unconditionally before entering a #+BEGIN_ block, so a
        // preceding affiliated #+NAME: line was re-parsed alone (setting pending_name with no
        // following block) and the code block emitted name: None — feed_line now recognizes a
        // trailing run of affiliated-keyword lines (`#+` but not `#+BEGIN`) immediately before a
        // BEGIN_ line and keeps them attached to the same accumulated block, so a single
        // events() call over the combined text threads pending_name to the block the way
        // parse::EventIter::parse_next_block does. (c) integration-list-code: feed_line trimmed
        // the line before its #+BEGIN_ test, so an indented code block inside a list item read
        // as a top-level block start and the item was split from its child — the check now
        // matches the untrimmed line (`line.to_uppercase().starts_with("#+BEGIN_")`, no trim),
        // mirroring parse::EventIter::parse_next_block (parse.rs:252), so an indented #+BEGIN_
        // falls through to the "Regular line" branch and stays continuation text of the list
        // item, matching parse()'s own documented limitation for this fixture. Confirmed via
        // org_streaming_parser_matches_events_under_adversarial_chunking passing over all 89 org
        // fixtures (previously 86/89).
        streaming_parser: ApiState::Wired,
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
        events: ApiState::Wired,
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
        events: ApiState::Wired,
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
        events: ApiState::Wired,
        streaming_parser: ApiState::NotApplicable(
            "commonmark-fmt's StreamingParser buffering all input before parsing with \
             pulldown-cmark is the sole documented CLAUDE.md exemption (pulldown-cmark requires \
             the full input as &str); see crates/formats/commonmark-fmt/src/lib.rs's \
             \"Limitations\" doc comment and src/batch.rs's own \"# Limitation\" section",
        ),
        // Fixed 2026-08-04: the last real divergence — `EventIter`'s `StartList` always
        // reporting `tight: true` — is closed. `Event` gained a new variant,
        // `ListTightnessResolved { tight: bool }` (a breaking, semver-relevant change to
        // commonmark-fmt's public `events::Event` enum, noted in its own module doc and
        // CHANGELOG.md): `EventIter` still emits `StartList { tight: true, .. }`
        // optimistically (list tightness is a whole-list property that can require seeing
        // every item to determine), but if the list turns out loose it now emits
        // `ListTightnessResolved { tight: false }` exactly once, immediately before the
        // matching `EndList` — the one point in the stream where the answer is always known,
        // without ever buffering the whole list (the signal is a real, direct-child
        // `Paragraph` tag on any item, tracked with O(nesting depth) state; see
        // events.rs's `ListState`/`ItemFrame` doc comments). `Writer` was updated in
        // lockstep: it retroactively splices in the missing blank-line separators (both
        // between items and between two blank-line-separated blocks inside the same item)
        // using a small parallel `Vec<Vec<usize>>` of skipped-separator byte offsets per
        // open list — see `Writer::list_correction_marks`'s doc comment — so it stays
        // byte-identical to the tree-based `emit()` regardless of exactly where in a list
        // the correction fires, including the case where the first item revealing the
        // signal isn't the one whose own separator needed fixing. Verified against every
        // fixtures/commonmark fixture, including the three that exercise this exactly
        // (rare-loose-list, integration-loose-list-item, rare-tight-vs-loose).
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "gfm",
        events: ApiState::Wired,
        streaming_parser: ApiState::NotApplicable(
            "shares commonmark-fmt with the \"commonmark\" format entry above; same sanctioned \
             pulldown-cmark StreamingParser exemption applies",
        ),
        // Fixed 2026-08-04: shares commonmark-fmt's now-corrected list-tightness handling
        // (ListTightnessResolved) with the "commonmark" format entry above — gfm is a thin
        // wrapper over the same commonmark-fmt crate.
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "markdown",
        events: ApiState::Wired,
        streaming_parser: ApiState::NotApplicable(
            "shares commonmark-fmt with the \"commonmark\" format entry above; same sanctioned \
             pulldown-cmark StreamingParser exemption applies",
        ),
        // Fixed 2026-08-04: shares commonmark-fmt's now-corrected list-tightness handling
        // (ListTightnessResolved) with the "commonmark" format entry above — markdown is a
        // thin wrapper over the same commonmark-fmt crate.
        streaming_writer: ApiState::Wired,
    },
    // docbook-fmt, jats-fmt, tei-fmt are byte-identical in implementation
    // shape (verified via `diff` across batch.rs/events.rs/writer.rs: only
    // doc comments and AST/event type names differ), so the same bugs —
    // found by this harness, not present in any prior audit — applied to all
    // three identically. All three ARE genuinely independent/incremental
    // implementations (events() pulls tokens straight off quick_xml::Reader
    // without building an AST; the streaming Writer calls quick_xml::Writer
    // directly per event with no buffering) — these are logic bugs, not
    // architectural hollowness. The streaming_parser mismatched/unmatched-
    // end-tag bug (StreamingParser silently accepting malformed XML that
    // events() correctly rejects) has since been fixed identically in all
    // three via a hand-tracked open-element stack in batch.rs; the events()
    // entity-coalescing gap has also since been fixed (see below).
    //
    // Fixed 2026-08-03 (all three, identically): the malformed-XML
    // auto-close-recovery gap. `EventIter` (events.rs) now tracks open
    // element names in its own `open_stack` (mirroring parse.rs's `stack:
    // Vec<ElementFrame>`) and, on EOF/`Err`, synthesizes an `EndElement`
    // event per still-open name (innermost first) via a new `finalize()`
    // method — the same recovery parse.rs's post-loop cleanup already
    // performed for `parse()`, now genuinely ported to `events()` (not
    // delegated to `parse()`) rather than just stopping dead. This fixed
    // `events()`/`events_from_doc(&parse())` equivalence on fixture
    // adv-malformed-xml for all three formats, and the streaming Writer
    // (fed by `events()`) picked up the same recovery for free with no
    // writer-side change. `StreamingParser` (batch.rs) needed its own,
    // separate port of the identical recovery (it already had its own
    // hand-tracked `open_stack` for the mismatched/unmatched-end-tag fix
    // above; a new `close_unclosed_elements()` method dispatches the same
    // synthetic `EndElement`s to the handler at every point `drain()` can
    // terminate for good — plain EOF, a fatal XML `Err`, and the two
    // manual tag-mismatch branches) — without it, `StreamingParser` would
    // have silently regressed relative to the now-recovering `events()`.
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
        // events is now Wired: the malformed-XML auto-close gap (fixture adv-malformed-xml)
        // that used to surface here is fixed (see the comment above this FormatCapabilities
        // block); fixing it uncovered a second, previously-masked defect on fixture line-break
        // (`<sbr></sbr>`, an explicitly-empty non-self-closing tag), also now fixed (2026-08-03):
        // `Node::Element` (ast.rs) gained a `self_closing: bool` field — parse.rs's
        // `XmlEvent::Empty` arm sets it `true`, the `XmlEvent::End` arm (built from a real
        // Start+End pair, even with zero children) sets it `false`. `events_from_doc`'s
        // `walk_node` now checks `self_closing` (not just `children.is_empty()`) before emitting
        // `EmptyElement` vs. `StartElement`+`EndElement`, so it faithfully distinguishes
        // `<sbr/>` from `<sbr></sbr>` the same way `EventIter`'s token-at-a-time pass already
        // did — no architectural change to `EventIter` itself was needed; the AST was the lossy
        // side, not the streaming reader. `collect_doc` (used by the streaming writer's AST
        // fallback) sets the field the same way from `Event::EmptyElement`/`Event::EndElement`.
        events: ApiState::Wired,
        // StreamingParser's drain() (batch.rs) still sets check_end_names=false and
        // allow_unmatched_ends=true on its per-drain-call quick_xml::Reader — that part is
        // architecturally necessary, since each drain() call only ever sees the unconsumed
        // tail, not the full document. But it now tracks open-element names itself in a
        // `Vec<String>` field that persists across drain() calls (the same "survives
        // multiple feed() calls" shape as entity_resolver), and validates every End event
        // against that stack by hand: a mismatch or an End against an empty stack pushes a
        // Diagnostic and stops draining for good, mirroring parse()'s "fatal diagnostic +
        // stop" behavior on a genuine XML error. Fixed and confirmed passing (fixtures
        // adv-malformed-xml, adv-unmatched-end-tag). Also now performs the same
        // malformed-XML auto-close recovery as events() (close_unclosed_elements(), see the
        // comment above this FormatCapabilities block), keeping it in sync with events()'s fix.
        streaming_parser: ApiState::Wired,
        // Fixed 2026-08-03: the malformed-XML auto-close gap (fixture adv-malformed-xml) that
        // used to surface here is fixed alongside events() (see the comment above this
        // FormatCapabilities block) — the streaming Writer is fed by events(), so events()'s
        // new synthetic closes flow through for free, no writer-side change needed. The other,
        // newly-surfaced line-break (`<sbr></sbr>`) gap is also now fixed, alongside events()'s
        // self_closing fix above: build() (via parse()'s AST) now also respects
        // `Node::Element.self_closing` in `emit.rs`'s `emit_node` (only writes the self-closing
        // `XmlEvent::Empty` form when `self_closing` is true, not merely `children.is_empty()`),
        // so it is no longer the lossy side either — both build() and the streaming Writer now
        // faithfully reproduce `<sbr></sbr>` vs. `<sbr/>` from the source.
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "jats",
        // Fixed 2026-08-03 alongside docbook-fmt (byte-identical events.rs shape); the
        // malformed-XML auto-close-recovery gap (jats's own adv-malformed-xml fixture, a
        // truncated-input case) is fixed the same way. Unlike docbook-fmt, no jats fixture
        // exercises an explicitly-empty non-self-closing tag, so the events()-vs-
        // events_from_doc(&parse()) check now passes cleanly over the whole fixture suite —
        // genuinely Wired, not narrowed.
        events: ApiState::Wired,
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
        // Fixed 2026-08-03 alongside docbook-fmt: the streaming Writer is fed by events(), so
        // events()'s new malformed-XML auto-close recovery (see the comment above the
        // "docbook" FormatCapabilities entry) flows through for free. Unlike docbook-fmt, no
        // jats fixture hits the separate explicitly-empty-tag defect either, so this is
        // genuinely Wired.
        streaming_writer: ApiState::Wired,
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
            "no events()-vs-AST-projection check is wired for ansi. Previously this also \
             couldn't be wired at all: parse()'s AnsiNode had no variant for a bare SGR \
             sequence, so a run of SGR codes not immediately followed by text produced zero AST \
             nodes while events()'s EventIter unconditionally emits one SetStyle/ResetStyle \
             event per 'm'-terminated CSI group. Fixed 2026-08-04 alongside the streaming_writer \
             entry below: AnsiNode gained SetStyle{style,span}/ResetStyle{span} variants, \
             emitted unconditionally per source SGR group (see ast.rs's SetStyle/ResetStyle doc \
             comments), so parse()'s AST now has a 1:1 node for every events() SetStyle/ \
             ResetStyle event — a faithful ast_to_events projection is possible. Still \
             NotYetWired only because no one has written the projection function/fixture-driven \
             check itself, not because of any remaining AST gap. Separately: events.rs's \
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
        // Fixed 2026-08-03: the fourth, previously-open bug (fixtures hyperlink,
        // rare-hyperlink-uri, only under fine-grained chunking like single_byte or
        // chunks_of_3 — not whole-input) is now fixed. find_safe_boundary() (batch.rs) gained a
        // second pass, truncate_before_unclosed_osc8_hyperlink(), run after the existing
        // last-ESC-completeness check: it scans the naive-safe prefix for a complete OSC 8
        // *opening* sequence (`ESC ]8;;<url><BEL|ST>`, non-empty url) with no matching closer
        // (any later complete `ESC ]8;...` sequence, open or close — mirroring
        // EventIter::parse_osc_event's own forward-scan termination rule exactly) within that
        // same prefix, and if found, moves the safe boundary back to that opening sequence's own
        // ESC byte. This defers the whole open..close span to a later drain_complete()/finish()
        // call that sees it all at once, instead of exposing the isolated opener to EventIter
        // and getting a premature empty-text Hyperlink. Confirmed via
        // ansi_streaming_parser_matches_events_under_adversarial_chunking (previously panicked
        // "check now PASSES, but is still listed in KNOWN_FAILURES" once the fix landed,
        // confirming it now holds over every fixture and chunking).
        streaming_parser: ApiState::Wired,
        // Fixed 2026-08-04, in two stages. Stage 1 fixed the adv-unknown-sgr divergence
        // (`\x1b[999m` no-op followed by a genuine trailing `\x1b[0m` with nothing left to
        // reset) via a narrow AnsiNode::ResetStyle node, emitted only when an explicit reset was
        // itself a no-op — but verifying beyond that one fixture found the fix was far too
        // narrow: 24 of 46 ansi fixtures diverged for two distinct, much larger reasons
        // (trailing-reset-vs-newline ordering, e.g. fixture bold: build() deferred its
        // "reset at end if style non-empty" epilogue past a trailing Newline node, giving
        // "...Hello\n\x1b[0m" for source "...Hello\x1b[0m\n"; SGR grouping not preserved across
        // multiple source escape sequences, e.g. fixture rare-bold-italic: build() re-derived
        // one merged \x1b[1;3m from the resulting style instead of the source's two separate
        // \x1b[1m/\x1b[3m groups, which events()/Writer reproduce verbatim). Both traced to the
        // same root cause: parse()'s AST only ever attached a resulting style to the next
        // Text/Hyperlink node, discarding both SGR grouping and position relative to
        // intervening non-text nodes.
        //
        // Stage 2 fixed that root cause directly: AnsiNode gained SetStyle{style,span}
        // (non-resetting SGR groups) alongside the existing ResetStyle{span}, both now emitted
        // unconditionally — one node per source SGR escape group, in source order, mirroring
        // events()'s SetStyle/ResetStyle exactly (parse.rs's 'm' arm is now a byte-for-byte
        // match of events.rs's parse_csi_event 'm' arm). build()'s emit_node transitions style
        // at each SetStyle/ResetStyle node's own source position instead of only at Text/
        // Hyperlink nodes, so it now replays the same one-escape-per-source-group shape
        // events()/Writer already produce. This single AST change covered both previously-named
        // root causes (they were the same gap, not two separate ones): source order now fixes
        // trailing-reset-vs-newline ordering, and per-group nodes now fix SGR-grouping
        // preservation. A third, unrelated, previously-undiscovered bug in writer.rs's
        // `transition_style`/`append_all_style_codes` (missing double_underline, rapid_blink,
        // and underline_color entirely — SGR 21/6/58 were silently dropped by the streaming
        // Writer) was also fixed, needed to close fixture double-underline specifically.
        // rescribe-read-ansi's build_document_nodes was updated to match the new AST shape:
        // SetStyle nodes are dropped (their style is already captured on the next Text/
        // Hyperlink node), and ResetStyle nodes are kept as a raw_inline `\x1b[0m` only when the
        // adapter's own running-style tracking shows the reset was otherwise unobservable —
        // reproducing the exact same IR output as before this change for every existing fixture
        // (verified: rescribe-read-ansi's and rescribe-fixtures' full test suites pass
        // unchanged). Confirmed via direct measurement (crates/formats/ansi-fmt/examples/
        // divergence_check.rs, run against all 46 ansi fixtures): 24/46 diverging before this
        // fix, 0/46 after.
        streaming_writer: ApiState::Wired,
    },
    // odf-fmt backs odt/ods/odp. events() is a genuine independent
    // implementation (direct quick_xml scan of content.xml, not a
    // parse()-then-walk fake — correcting a prior assessment) but is
    // eagerly, fully buffered before the first next() call (self-documented
    // in events.rs), so it is not memory-bounded; no StreamingParser<H>
    // exists yet (batch.rs module doc calls it "future" work). The streaming
    // Writer genuinely builds its AST incrementally per event (same
    // sanctioned shape as ooxml-sml's SmlWriter, deferring only ZIP byte
    // packaging to finish()) and, as of 2026-08-03, is byte-identical to
    // build() across all 66 odt fixtures.
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
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "tei",
        // Fixed 2026-08-03 alongside docbook-fmt (byte-identical events.rs shape); the
        // malformed-XML auto-close-recovery gap (tei's own adv-malformed-xml fixture, which
        // combines mismatched *and* unclosed end tags — see the "docbook" FormatCapabilities
        // comment above) is fixed the same way. Unlike docbook-fmt, no tei fixture exercises
        // an explicitly-empty non-self-closing tag, so the events()-vs-
        // events_from_doc(&parse()) check now passes cleanly over the whole fixture suite —
        // genuinely Wired, not narrowed.
        events: ApiState::Wired,
        // Fixed alongside docbook-fmt/jats-fmt (byte-identical batch.rs shape); see the
        // "docbook" streaming_parser comment above. Confirmed passing (fixture
        // adv-malformed-xml, adv-unmatched-end-tag).
        streaming_parser: ApiState::Wired,
        // Fixed 2026-08-03 alongside docbook-fmt: the streaming Writer is fed by events(), so
        // events()'s new malformed-XML auto-close recovery flows through for free. Unlike
        // docbook-fmt, no tei fixture hits the separate explicitly-empty-tag defect either, so
        // this is genuinely Wired.
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "opml",
        // opml-fmt is a from-scratch crate (2026-08-04), not an audit of a pre-existing
        // implementation, so this entry documents design rather than a fix. Unlike
        // docbook/jats/tei's generic-element-tree AST, opml-fmt models OPML's small fixed
        // grammar as a domain-typed AST/event vocabulary (OpmlDoc/Head/Body/Outline,
        // StartOutline/HeadField/EmptyOutline/...) — the same "well-nested XML, three
        // genuinely independent readers" architecture, just with OPML-specific event names
        // instead of generic StartElement/EndElement. `events()` (`EventIter`) wraps
        // `quick_xml::Reader` directly, pulling a `<head>` child's Start/Text/End run in one
        // `next()` call (bounded lookahead, not tree-building) so the returned `HeadField`
        // event carries the complete text value. `events::events_from_doc(&OpmlDoc)` is the
        // crate's own AST->events projection, used as the equivalence oracle here exactly as
        // for docbook/jats/tei.
        //
        // Verified via this harness's own checks over all 26 opml fixtures, not asserted from
        // design alone: initial runs found two real defects, both now fixed. (1) An empty
        // `Head` (all fields unset) is indistinguishable, once parsed, from "no `<head>`
        // element in the source" (`Head::is_empty()`, ast.rs) — `events_from_doc` and `emit`
        // both used to unconditionally synthesize a `<head></head>` pair regardless, while
        // `events()` (reading real bytes) only emits one when the source actually had a
        // `<head>` tag; fixture adv-minimal (no head at all) caught this. Fixed by having both
        // `events_from_doc` and `emit` skip the Start/EndHead pair when `head.is_empty()`,
        // matching what `events()` produces for a headless source. (2) On a fatal XML error
        // (fixture adv-malformed-xml: an `<outline>` left unclosed when `</body>` is hit),
        // `EventIter::finalize()`/`StreamingParser::close_out()` auto-closed any open
        // `<outline>`/`<head>`/`<body>` but had no equivalent tracking for `<opml>` itself, so
        // no synthetic `EndOpml` was ever produced on that recovery path — while
        // `events_from_doc` always appends `EndOpml` unconditionally at the end, since the AST
        // has no way to represent "the document was truncated before `</opml>`". Fixed by
        // adding an `in_opml` flag (mirroring `in_head`/`in_body`) to both `EventIter` and
        // `StreamingParser`, synthesizing `EndOpml` in `finalize()`/`close_out()` when still
        // open, the same recovery shape as the docbook/jats/tei "unclosed element" fix above.
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "endnotexml",
        // endnotexml-fmt is a from-scratch crate (2026-08-04), extracted from
        // rescribe-read-endnotexml/rescribe-write-endnotexml (which called quick_xml directly
        // in production code, a pre-existing CLAUDE.md violation logged in TODO.md), following
        // the opml-fmt template. EndNote XML's schema is larger and deeper than OPML's (nested
        // contributor role lists, multiple title variants, `<style>` markup runs inside field
        // content), so the AST/event vocabulary is domain-typed at the container level
        // (Record/Contributors/Titles/Periodical/Urls/Dates get their own
        // Start*/End* event pairs) but generic at the leaf level: any leaf field or
        // unrecognized element becomes `StartElement{name,attrs}`/inline-content/`EndElement`,
        // keyed by its exact source tag name — see ast.rs's and events.rs's module docs for the
        // full rationale, including how this streams `<style>` runs incrementally (Text/
        // StartStyle/EndStyle/nested StartElement events) rather than buffering a whole field's
        // content into one aggregate event.
        //
        // Verified via this harness's own checks over all fixtures, not asserted from design
        // alone. Two defects surfaced and were fixed before this entry was written: (1) using a
        // pretty-printing `quick_xml::Writer::new_with_indent` in `emit()`/the streaming
        // `Writer` corrupted meaningful whitespace between `<style>` runs (the auto-indenter
        // injects its own whitespace around every tag) — fixed by switching both to the
        // non-indenting `Writer::new`, matching the pre-existing rescribe-write-endnotexml
        // writer's identical choice. (2) `events_from_doc`'s per-record field emission order
        // was an arbitrary struct-field order that didn't match real EndNote export order
        // (observed directly from the existing fixture files: `ref-type`, `contributors`,
        // `titles`, `dates`, then `volume`/`pages`/`urls`/etc., with `rec-number`/
        // `foreign-keys` last) — fixed by reordering `events_from_doc`'s and `emit()`'s field
        // emission to match, both consistently (this is a known, documented limitation shared
        // with opml-fmt's identical `Head` design: record field order is canonicalized on
        // write, not preserved verbatim for a source file with a nonstandard order — content
        // fidelity is exact regardless, only field order is not byte-for-byte guaranteed for
        // files exported by tools that reorder these particular fields).
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::Wired,
    },
    FormatCapabilities {
        format: "typst",
        // typst-fmt is a from-scratch crate (2026-08-04) extracted from
        // rescribe-read-typst/rescribe-write-typst (which called
        // typst_syntax::parse and hand-emitted Typst markup text directly in
        // production code, a pre-existing CLAUDE.md violation logged in
        // TODO.md). Unlike opml-fmt/endnotexml-fmt (hand-rolled quick_xml-
        // backed parsers with a genuinely independent events() pull
        // iterator), typst-fmt wraps `typst-syntax`, which has no native
        // event/SAX parsing mode at all — there is no way to produce
        // IR-shaped events without parsing to a tree first. `events()`
        // (EventIter) is therefore a cursor-based walk over the already-
        // parsed structure (an explicit `Vec<Task>` work stack, each
        // `next()` call doing O(1) work — a legitimate streaming
        // *iteration* API, honest about being a post-parse walk, not a fake
        // one) — the same "events() = parse() + tree walk" shape as
        // bbcode-fmt below, wired for the identical reason: no format-spec
        // restriction forces it, it is what the upstream parser makes
        // possible. `StreamingParser`/`BatchParser` (batch.rs) buffer all
        // input until `finish()` for the same underlying reason (no
        // chunk-fed from-scratch parse API exists in typst-syntax — only
        // edit-based *re*parsing of an already-built tree, an editor use
        // case) — the second sanctioned "buffer all input" exemption
        // alongside commonmark-fmt's pulldown-cmark, documented in
        // CLAUDE.md and in batch.rs's own module docs.
        //
        // Verified via this harness's own checks over all `fixtures/typst`
        // fixtures (rescribe-fixtures' typst() test, tests/run.rs), plus
        // typst-fmt's own in-crate smoke tests (events() == events_from_doc
        // projection, StreamingParser == events(), streaming Writer ==
        // builder emit() over a construct sample) and its two fuzz targets
        // (no-panic, native-AST roundtrip). Two genuine bugs surfaced and
        // were fixed while adding the roundtrip test, both inherited
        // unnoticed from the pre-extraction rescribe-write-typst emitter
        // (never roundtrip-tested before): a `Block::DefinitionList` was
        // emitted wrapped in `#terms(...)`, which is not valid Typst syntax
        // at all; and equation/unknown-function-call source text was
        // extracted via `SyntaxNode::text()`, which is empty for any
        // composite node (text only lives on leaf tokens) — both fixed, see
        // typst-fmt's own commit history for the full detail.
        events: ApiState::Wired,
        streaming_parser: ApiState::Wired,
        streaming_writer: ApiState::Wired,
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
        // Fixed (StartDocument/EndDocument duplication): StreamingParser used to re-parse each
        // accumulated block in isolation via crate::events::events(&text), and events() always
        // wraps its output in its own StartDocument/EndDocument pair, so StreamingParser used to
        // emit one such pair per accumulated block instead of one for the whole document,
        // diverging on every fixture with more than one top-level block. StreamingParser now
        // dispatches its own single StartDocument in new() and EndDocument in finish() (mirroring
        // fountain-fmt's batch.rs), filtering the wrapper pair out of every per-block re-parse's
        // output (emit_block() and try_emit_header()'s trailing-content path both filter
        // Event::StartDocument/EndDocument). Confirmed via adversarial-chunking equivalence
        // against events() over every fixture, plus a hand-built synthetic-sample adversarial
        // test added to t2t's own batch.rs (whole/single-byte/chunks-of-7/chunks-of-37) asserting
        // exactly one StartDocument/EndDocument pair. The related, already-fixed
        // document-header-specific defect (Event::Header / try_emit_header) is unaffected by
        // this change.
        // Fixed 2026-08-03 (three distinct root causes, all confirmed via
        // t2t_streaming_parser_matches_events_under_adversarial_chunking, now passing over the
        // whole fixture suite): (1) definition-list — parse_definition_list (parse.rs) skips
        // any number of blank lines between consecutive ': '-prefixed items and only stops once
        // a following non-blank line fails to start with ": ", merging them into one
        // DefinitionList at the whole-document level. StreamingParser's feed_line now defers
        // the flush decision on a blank line the same way rst-fmt's/djot-fmt's DefinitionList
        // fixes do: when the accumulating block is a definition list
        // (block_is_definition_list(), first line starts with ": "), a blank line is held
        // (pending_deflist_blank) rather than flushing immediately, and the next non-blank line
        // decides — another ": "-prefixed line continues the same block, anything else flushes
        // it as already-ended. (2) adv-heading-no-close / adv-link-no-close — this was actually
        // a bug in the whole-document parse()/events() reference itself, not in StreamingParser:
        // Parser::try_parse_header (parse.rs) treated an *unclosed* heading opener ("=" not
        // closed by a matching "=") or unclosed link opener ("[" with no closing "]") as
        // ordinary header title text, then blindly consumed the next two lines as author/date —
        // even when those lines actually belonged to an unrelated block across a blank-line
        // boundary, silently discarding that block's content. try_parse_header now also rejects
        // lines starting with 1-5 '='s or with '[' (an unambiguous attempted heading/link opener,
        // closed or not), matching the existing rejection checks for closed headings and other
        // block markers. Both parse()/events() and StreamingParser now agree, and no content is
        // silently dropped. (3) adv-unclosed-code — an EOF-terminated code/raw fence with no
        // closing marker: the whole-document parser's Parser::new splits the entire input on
        // '\n' up front, so a trailing newline in the original input always produces one extra,
        // synthetic empty trailing "line" that parse_verbatim_block/parse_raw_block include in
        // an unterminated block's content (since they never see a closing marker to stop
        // early), producing a trailing '\n' that isn't really source text.
        // StreamingParser::finish() now replicates that artifact: when EOF is reached while
        // still BlockState::InFenced (no closer ever seen) and the original input had a genuine
        // trailing newline (line_buf was empty before draining any leftover partial line), an
        // extra empty line is appended to block_lines before the final emit_block() re-parse, so
        // its content trailing-newline matches events()'s.
        streaming_parser: ApiState::Wired,
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
    // the streaming_parser fix below (was a KnownFailure, fixed 2026-08-03).
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
        // Fixed 2026-08-03: StreamingParser used to re-parse each accumulated block in
        // isolation via crate::events::events(&text), and events() always wraps its output
        // in its own StartDocument/EndDocument pair (ManEvent has no way to avoid it), so
        // StreamingParser emitted one such pair per accumulated block instead of one for
        // the whole document — the same re-parse-each-block-in-isolation root cause already
        // fixed for t2t-fmt/fountain-fmt. Fixed the same way: StreamingParser now owns
        // exactly one StartDocument/EndDocument pair itself (StartDocument dispatched in
        // `new()`, EndDocument in `finish()`), and `emit_block()` filters out the
        // re-parsed block's own StartDocument/EndDocument before forwarding the rest.
        // man-fmt's events() has no title-page-style "only the first block can mean X"
        // wrinkle (unlike fountain's title page), so no `events_body()`-style second entry
        // point was needed here — every block re-parses through the same `events()`.
        // Confirmed byte-for-byte equal to events() on the exact repro from this entry's
        // former description (".SH NAME\ntest\n\n.SH DESCRIPTION\nmore text\n") and across
        // all man fixtures under adversarial chunking.
        streaming_parser: ApiState::Wired,
        // streaming_writer fixed 2026-08-03 (see the module-doc-referencing comment above
        // this struct entry for the two-stacked-defect writeup): now Wired, independently
        // of the streaming_parser fix directly above.
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
    // Fixed 2026-08-04: batch::StreamingParser (src/batch.rs) is now a genuinely incremental
    // reader, not a buffer-then-finish stub. `feed()` buffers only until the header
    // (`\fonttbl`/`\colortbl`/`\stylesheet`/`\info`/`\*`-destination groups) is confirmed
    // complete (`incremental::find_header_boundary`, a byte-level tokenizer shared with the
    // body-cut scanner — see `src/incremental.rs`'s module doc), computes the font/color
    // tables from just that buffered slice, emits `Event::StartDocument`, then buffers only up
    // to the next top-level `\par`/`\pard` (`incremental::find_next_par_cut`) and hands that one
    // bounded increment to `parse::Parser::run_body_step` — the *same* method the whole-document
    // `parse()` path uses (not a reimplementation), carrying inherited character/paragraph/
    // table/list state (`parse::BodyCarry`) across increments. Verified: `feed()` alone (no
    // `finish()`) delivers events for a complete-prefix input; a 100x paragraph-count increase
    // (200 -> 20,000 paragraphs, fed in 32-byte chunks) shows peak allocator bytes going from
    // 1372 to 1376 (ratio 1.00) via this crate's `alloc_probe` tracking allocator, not scaling
    // with document size; adversarial-chunking equivalence against `events()` (whole-input,
    // single-byte, 3/7/13-byte chunks, and hand-constructed mid-control-word and
    // mid-group-boundary splits) passes over a rich hand-built sample (headings, bold/italic,
    // nested groups, color/font changes, a table, a bulleted list) in `src/batch.rs`'s own test
    // suite, and over every fixture in `fixtures/rtf/` via this file's own adversarial-chunking
    // test below.
    //
    // Fixed 2026-08-04 (second pass, same day): `Event::StartDocument`'s divergence from
    // `events()` used to be described as structural and not-further-fixable. It wasn't — two
    // independent things were compounded together:
    //
    // 1. `events()`'s `StartDocument.fonts`/`colors` are computed by `build_font_map`/
    //    `build_color_map` walking the *entire already-parsed* document (first-*use* order,
    //    deduplicated) — information that by definition doesn't exist until the last font/color
    //    reference anywhere in the body has been seen, which conflicts directly with
    //    `StartDocument` needing to be the *first* event a genuinely incremental reader emits.
    //    This part *is* structural: `StreamingParser` still reports the header's own *declared*
    //    `\fonttbl`/`\colortbl` tables in `StartDocument` (available after O(header size) bytes),
    //    which can differ in order/set from the body's actual usage. But the true, first-use-order
    //    table is no longer *unrecoverable* — a new `Event::TableOrderResolved { fonts, colors }`
    //    (sem_events.rs) is emitted right before `EndDocument`, carrying the same value
    //    `events()`'s `StartDocument` carries, computed by accumulating first-use order
    //    incrementally across body increments (`tables::collect_used_fonts_incremental`/
    //    `collect_used_colors_incremental`, threaded through `BodyState` — bounded by the number
    //    of distinct fonts/colors used, not document size). `events()` now also emits
    //    `TableOrderResolved` (with the same value as its own `StartDocument`, trivially, since it
    //    has the whole document up front either way) for a uniform two-path contract.
    // 2. The "no `\fonttbl` at all" case (`parse_font_table` defaulting to `[""]` vs.
    //    `build_font_map`'s hardcoded `["Times New Roman"]`) was a plain convention mismatch, not
    //    a structural one — that default string is never read into any `Inline::Font` output
    //    (`make_inline` only consults `font_table[idx]` when `idx != 0`), so `parse_font_table`'s
    //    fallback was changed to `["Times New Roman"]` directly, eliminating the mismatch instead
    //    of working around it.
    //
    // Net effect, confirmed empirically across all 38 `fixtures/rtf/` fixtures under adversarial
    // chunking (`rtf_streaming_parser_matches_events_under_adversarial_chunking`): `events()` and
    // `StreamingParser` now produce **exactly identical** event vectors, including
    // `StartDocument`, for every fixture in this repository's corpus. A document whose header
    // declares fonts/colors in a different order or set than its body actually uses (not present
    // in this corpus, but not excluded by the RTF grammar) would still show a `StartDocument`-only
    // divergence — recoverable via `TableOrderResolved`, same as before. `streaming_parser` moves
    // from `KnownFailure` to `Wired` below; this KNOWN_FAILURES-table entry has been removed
    // accordingly (a fixed bug must not keep masking future regressions).
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
        // Wired 2026-08-04 (was KnownFailure): see the module-doc-referencing comment above this
        // struct entry for the full writeup of both fixes (the TableOrderResolved correction
        // event plus the parse_font_table fontless-default fix). `events()` and `StreamingParser`
        // now produce byte-for-byte identical event vectors, including StartDocument, for every
        // fixture in fixtures/rtf/, under adversarial chunking.
        streaming_parser: ApiState::Wired,
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
    "bibtex",
    "biblatex",
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
    // Ten Pandoc output-format variants (beamer, revealjs, slidy, s5,
    // dzslides, slideous, context, ms, icml, chunkedhtml) — write-only
    // presentation/rendering targets with no reader, analogous to how pandoc
    // treats html/latex/ooxml variants. Each now has a fixture suite under
    // `fixtures/writers/{format}/` (added 2026-08-04: slide/frame-boundary +
    // paragraph/list/code-block/emphasis coverage for the six HTML- and
    // LaTeX-slideshow-family writers, table coverage for context, troff
    // macro coverage for ms, InCopy story/text-run coverage for icml,
    // multi-file coverage for chunkedhtml — see fixtures/writers/{format}/
    // and TODO.md for exact constructs and two confirmed content-drop bugs
    // found along the way: revealjs/slidy/s5/dzslides all silently drop
    // inline `Image` nodes appearing inside a paragraph, ms and icml drop
    // `Image` entirely). They remain in this list, not `CAPABILITIES`,
    // because — independent of fixture coverage — none of the ten has a
    // standalone `{format}-fmt` crate for this harness to audit: all writer
    // logic lives directly in `rescribe-write-{format}` (confirmed by
    // reading each `src/lib.rs`; no `crates/formats/{format}-fmt` exists
    // for any of them), the same "no `-fmt` crate exists at all" situation
    // as `latex`/`csl-json`/etc. above, and being write-only they have no
    // `events()`/`StreamingParser` (reader) surface to audit in the first
    // place; only a streaming writer could conceivably be checked, and
    // there is no `{format}-fmt` crate to expose one independently of the
    // `rescribe-write-{format}` builder `emit()` fixture tests already
    // covering these adapters.
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
    // fb2's "events" and "streaming_writer" KnownFailure entries (the book's own
    // <description><title-info><annotation> — with <p>/<poem>/<cite>/<subtitle>/<table>/
    // empty-line sub-content — silently discarded by events()/EventIter, leaving
    // TitleInfo.annotation permanently None while parse()'s AST modeled it correctly) were
    // removed 2026-08-04. Fixed in events.rs by adding a dedicated `ann_stack: Vec<AnnoItem>`
    // content-builder, separate from the top-level `stack`: `AnnoItem` mirrors the subset of
    // parse.rs's `StackItem` reachable from `Annotation`'s content model (Paragraph/Subtitle/
    // Poem/PoemTitle/Stanza/VerseLine/Cite/Epigraph/TextAuthor/Table/TableRow/TableCell/
    // InlineWrapper/Link/FootnoteRef), assembling owned AST values bottom-up exactly like
    // parse.rs's Parser does — not streamed as top-level events, since the whole thing must
    // fold into a single Event::Metadata rather than emit incrementally the way identical-
    // looking <poem>/<cite> content in a <section> does. `handle_start`/`handle_empty`/
    // `handle_end`/`flush_inline_text`/`finalize_open_elements` were all given an ann_stack
    // priority check (before the pre-existing `desc.in_description` branch, since
    // `in_description` stays true the whole time inside the annotation too) that routes to the
    // new `handle_ann_start`/`handle_ann_empty`/`handle_ann_end`/`close_ann_item`/
    // `ann_push_block_content`/`ann_in_inline_context`/`ann_push_text_to_inline_context`/
    // `ann_push_inline` methods, which mirror parse.rs's `handle_start`/`handle_empty`/
    // `handle_end`/`push_block_content`/`in_inline_context`/`push_text_to_inline_context`/
    // `push_inline` respectively, scoped to `ann_stack`. `finalize_open_elements` also drains
    // any still-open `ann_stack` (folding a truncated annotation into TitleInfo.annotation)
    // before its pre-existing `self.stack` unwind, for the same malformed/truncated-input
    // robustness the rest of this file already established. The `#[cfg(test)]`-only
    // `collect_poem_events`/`collect_cite_events` (AST→events, the opposite direction) were
    // read first per the task brief but turned out not to be directly invertible — events()
    // needs owned AST values assembled bottom-up, not a flat event sequence — though they did
    // confirm the exact target shape. Four new fixtures were added (fixtures/fb2/annotation-
    // poem, -cite, -table, -subtitle) alongside the existing fixtures/fb2/annotation (a bare
    // <p>), covering every AnnotationContent variant (Para/Poem/Cite/Subtitle/Table/EmptyLine)
    // and Poem's own nested Epigraph. Confirmed via
    // fb2_events_equals_ast_projection_over_all_fixtures and
    // fb2_streaming_writer_byte_identical_to_builder_over_all_fixtures both passing over the
    // full (now 61-fixture) suite, and fb2_streaming_parser_matches_events_and_is_incremental
    // (adversarial chunking) continuing to pass. Deliberately NOT touched: the separate,
    // still-open, pre-existing gap where a *section-level* `<section><annotation>` (Section's
    // own `annotation` field, distinct from TitleInfo's) is parsed into `ParseState::Annotation`
    // as a bare marker whose `close_item` arm is a no-op (`ParseState::Description |
    // ParseState::Annotation => {}`) — content nested inside it currently leaks out as
    // top-level events instead of being dropped or folded anywhere, since events() has no
    // `Event::Annotation` variant to carry an owned Section-level annotation value the way
    // Event::Metadata carries TitleInfo's. This is out of the confirmed scope for this fix (no
    // fixture currently exercises it, and fixing it would need a new public Event variant plus
    // rescribe-read-fb2/rescribe-write-fb2/oracle-harness changes) — left as a genuinely open,
    // separate gap; see TODO.md.
    // commonmark/gfm/markdown's "events" and "streaming_writer" KnownFailure entries
    // (StartList-tight-always-true, and the Writer's downstream blank-line omission) were
    // removed 2026-08-04 — both are fixed via a new Event::ListTightnessResolved correction
    // event; see the CAPABILITIES entries above for the full explanation.
    // docbook/events and docbook/streaming_writer were both KnownFailure entries here (the
    // `<sbr></sbr>`-vs-`<sbr/>` explicitly-empty-tag ambiguity: parse()'s AST couldn't
    // distinguish them, so events_from_doc/build() collapsed both to the self-closing form).
    // Both are now fixed — `Node::Element` gained a `self_closing: bool` field threaded through
    // parse.rs/emit.rs/events.rs — and the FormatCapabilities entry above reflects both as
    // Wired — no entries left here per assert_or_known_failure's own rule against masking a
    // fixed bug.
    // odt/streaming_writer was a KnownFailure entry here (12 of 66 fixtures
    // diverging over OdfEvent body-content gaps); it is now fixed and the
    // FormatCapabilities entry above is Wired — no entry left here per
    // assert_or_known_failure's own rule against masking a fixed bug. See the
    // comment above the check in tests/streaming_apis.rs for the seven root
    // causes that were found and closed.
    // ansi/streaming_parser was a KnownFailure entry here (find_safe_boundary treating a
    // complete OSC 8 hyperlink *opening* sequence as a safe boundary on its own, splitting the
    // atomic open..close Hyperlink token under fine-grained chunking); it is now fixed and the
    // FormatCapabilities entry above reflects it as Wired — no entry left here per
    // assert_or_known_failure's own rule against masking a fixed bug.
    // ansi/streaming_writer was a KnownFailure entry here, in two stages (adv-unknown-sgr's
    // trailing-reset divergence, then a much larger 24-of-46-fixtures divergence it surfaced:
    // trailing-reset-vs-newline ordering and SGR grouping not preserved across source escape
    // sequences — both traced to the same root cause, parse()'s AST only ever attaching a
    // resulting style to the next Text/Hyperlink node instead of modeling SGR groups as their
    // own nodes in source order). All of it is now fixed (2026-08-04): AnsiNode gained
    // SetStyle{style,span}/ResetStyle{span} variants, emitted unconditionally, one per source
    // SGR escape group, mirroring events()'s SetStyle/ResetStyle exactly; build() now
    // transitions style at each such node's own source position instead of only at Text/
    // Hyperlink nodes. A third, separate bug (writer.rs's transition_style/
    // append_all_style_codes silently dropped double_underline/rapid_blink/underline_color
    // entirely) was fixed alongside it. The FormatCapabilities entry above reflects this as
    // Wired (24/46 diverging -> 0/46, confirmed via
    // crates/formats/ansi-fmt/examples/divergence_check.rs) — no entry left here per
    // assert_or_known_failure's own rule against masking a fixed bug.
    // man/streaming_parser and man/streaming_writer were both KnownFailure entries here;
    // both are now fixed (2026-08-03, two independent commits) and the FormatCapabilities
    // entry above reflects both as Wired — no entry left here per assert_or_known_failure's
    // own rule against masking a fixed bug.
    // t2t/streaming_parser was a KnownFailure entry here (three distinct root causes: a
    // definition-list continuation gap, a try_parse_header contiguity bug shared with
    // parse()/events() themselves, and an EOF-fence trailing-newline mismatch); all three are
    // now fixed and the FormatCapabilities entry above reflects it as Wired — no entry left
    // here per assert_or_known_failure's own rule against masking a fixed bug.
    // rtf/streaming_parser was a KnownFailure entry here (StartDocument.fonts/colors reported the
    // header's declared table instead of events()'s usage-based, first-use-order table). Fixed
    // 2026-08-04 via a new Event::TableOrderResolved correction event plus a parse_font_table
    // convention fix (see the FormatCapabilities entry above for the full writeup) — no entry
    // left here per assert_or_known_failure's own rule against masking a fixed bug.
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
