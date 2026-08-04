//! Chunk-driven (batch) RTF parser.
//!
//! Feed input in arbitrarily-sized chunks with [`BatchParser::feed`], then
//! call [`BatchParser::finish`] to obtain the parsed AST.
//!
//! For event-callback style with low-level token events use [`BatchSink`].
//! For event-callback style with semantic document events use [`StreamingParser`].
//!
//! # `StreamingParser`'s memory shape
//!
//! RTF requires font tables and color tables (declared in the document
//! header) to be fully parsed before any body content can be semantically
//! interpreted: `\f<N>`/`\cf<N>` body control words are indices into those
//! tables. This is a *bounded* requirement, not an unbounded one — the RTF
//! 1.9.1 grammar (`<file> ::= '{' <header> <document> '}'`) places the header
//! strictly before the document body, and the spec text is explicit that the
//! header groups "must precede the first plain-text character in the
//! document." [`StreamingParser::feed`] exploits exactly that: it buffers
//! only until the header is confirmed complete
//! ([`crate::incremental::find_header_boundary`], driven off the same
//! byte-level tokenizer [`crate::parse::Parser`] itself uses — see
//! `src/incremental.rs`'s module doc), parses *just* that buffered slice with
//! the existing `parse_font_table`/`parse_color_table`, emits
//! `Event::StartDocument` (see below for what its `fonts`/`colors` fields
//! carry here), and discards the header bytes — retaining only the small
//! font/color tables from then on.
//!
//! After the header, `feed()` buffers only up to the next safely-parseable
//! increment: the position right after the next top-level `\par`/`\pard`
//! control word ([`crate::incremental::find_next_par_cut`]), found the same
//! way regardless of whether that position arrives in this `feed()` call or
//! a later one (a single byte at a time is fine — see
//! `test_streaming_parser_byte_at_a_time` below). That increment is hand to
//! [`crate::parse::Parser::run_body_step`] — the *exact* method the
//! whole-document `parse()` path uses, just called once per increment
//! instead of once for the whole document — carrying inherited
//! character/paragraph formatting state
//! ([`crate::parse::BodyCarry`]) across increments, so a table or list that
//! spans several paragraphs still comes out as one `Block::Table`/
//! `Block::List`, split across however many `\par`s it took to arrive.
//! Resulting blocks are converted to events immediately
//! (`SemanticEventIter::from_blocks`) and delivered to the handler — no
//! whole-document buffering at any point.
//!
//! **Memory bound**: `O(header size + largest single paragraph or
//! still-open table/list + nesting depth)`. Not `O(full document)`. This is
//! *not* the finer-grained `O(largest token)` bound `StreamingReader`'s
//! target design describes in the top-level `CLAUDE.md` — RTF's grammar
//! offers no signal cheaper than "the next `\par`" that a paragraph (or an
//! in-progress table/list) is complete, so that is this reader's real
//! increment granularity. A document with one pathologically large paragraph
//! (or an unbroken run of table rows) buffers that whole paragraph/table
//! before flushing it — see `test_streaming_parser_memory_bounded_by_header_not_document`
//! for what *is* verified: a many-thousand-paragraph document does not scale
//! with paragraph *count*.
//!
//! **`\binN` exception**: a `\binN` control word's `N` raw payload bytes
//! (embedded pictures/objects) are treated as one atomic token by the
//! boundary scanner (matching `handle_control_word`'s `"bin"` arm), so a
//! chunk boundary landing inside one is buffered until the whole payload
//! arrives — bounded by that embedded blob's own size, not the document's,
//! but real for documents with very large embedded binary objects.
//!
//! **`StartDocument.fonts`/`colors` can diverge from `events()`'s, but the
//! true values are always recoverable via `Event::TableOrderResolved` —
//! fixed 2026-08-04, see below for what was structural and what wasn't.**
//! [`crate::sem_events::events`] computes `StartDocument`'s `fonts`/`colors`
//! by walking the *entire* parsed document
//! ([`crate::tables::build_font_map`]/[`build_color_map`]: the font/color
//! table an emitter should treat as canonical, deduplicated, in
//! first-*use* order, with an implicit "Times New Roman" default always at
//! index 0) — information that is by definition unavailable until the last
//! font/color reference anywhere in the body has been seen. A genuinely
//! incremental reader cannot compute that before emitting `StartDocument`
//! (which must be the first event) without buffering the whole document
//! first, which is exactly what this rewrite exists to avoid. This part is a
//! genuine, structural limit — not fixable without either buffering the
//! whole document (defeating the point of this rewrite) or delaying
//! `StartDocument` past the point RTF requires it (also not an option: the
//! header must precede the body).
//!
//! So `StreamingParser` emits the document's own *declared* header tables in
//! `StartDocument` (`parse_font_table`/`parse_color_table`'s output — the
//! same tables `Parser` itself resolves body `\f<n>`/`\cf<n>` references
//! against, and the same convention `RtfDoc.color_table` uses), available
//! after `O(header size)` bytes. Every font/color a body `StartFont`/
//! `StartColor` event carries is guaranteed to already be present in this
//! table (that's where the reader resolved its name/RGB value from in the
//! first place), so the output is always valid RTF regardless — but the
//! *order* (declaration order vs. first-use order) and *set* (the header may
//! declare fonts/colors the body never uses) can differ from what
//! `events()` reports.
//!
//! What *was* fixable, and is: two independent things used to compound this
//! into a false "this is unrecoverable" impression.
//!
//! 1. **The true first-use table is never lost — only delayed.**
//!    `used_fonts`/`used_colors` on [`BodyState`] accumulate first-occurrence
//!    order incrementally, one increment at a time
//!    ([`crate::tables::collect_used_fonts_incremental`]/
//!    [`collect_used_colors_incremental`]) — bounded state, proportional to
//!    the number of distinct fonts/colors actually used, not document size.
//!    By `finish()`, the whole body has been seen, so this accumulator is
//!    byte-identical to what `build_font_map`/`build_color_map` would
//!    compute over the whole parsed document. `finish()` reports it via
//!    [`crate::sem_events::Event::TableOrderResolved`], emitted right before
//!    `EndDocument` — a consumer that needs the true order (e.g. to
//!    reproduce `events()`'s exact header) gets it there, just later than
//!    `StartDocument` rather than never.
//! 2. **The "no `\fonttbl` at all" case was a plain convention mismatch, not
//!    a structural one, and has been fixed directly.** `parse_font_table`
//!    used to default to `[""]` for a document with no `\fonttbl` group,
//!    while `build_font_map` always defaults to `["Times New Roman"]`. That
//!    default is never observable in parsed `Inline::Font` output (see
//!    `parse_font_table`'s doc comment), so it was safe to change
//!    `parse_font_table`'s fallback to match `build_font_map`'s convention
//!    directly — eliminating this divergence rather than working around it.
//!
//! After both fixes, the only *remaining* `StartDocument` divergence is a
//! document whose header genuinely declares fonts/colors in a different
//! order, or a different set, than the body actually uses — a real
//! document shape RTF permits, not present in any fixture in this
//! repository's `fixtures/rtf/` corpus, but not guaranteed to be absent from
//! a document produced by some other tool. For such a document,
//! `StartDocument` still reports the declared table; `TableOrderResolved`
//! still recovers the true one.
//!
//! **Degenerate fallback**: if the header is never confirmed complete even
//! at `finish()` (malformed/truncated input — e.g. an `\fonttbl` or `\binN`
//! payload that never closes) — `finish()` falls back to a full
//! whole-buffer `events()` call for that input only, exactly reproducing
//! the old buffer-then-finish behavior for this one pathological case
//! (see `test_streaming_parser_malformed_header_falls_back`).
//!
//! # Example — AST style
//! ```no_run
//! use rtf_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"{\\rtf1\\ansi ");
//! p.feed(b"Hello}");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — callback style
//! ```no_run
//! use rtf_fmt::batch::BatchSink;
//! use rtf_fmt::TokenEvent;
//!
//! let mut evs = Vec::new();
//! let mut sink = BatchSink::new(|ev: TokenEvent| evs.push(ev));
//! sink.feed(b"{\\rtf1 Hello}");
//! sink.finish();
//! ```

use crate::ast::{Diagnostic, RtfDoc};
use crate::events::TokenEvent;
use crate::sem_events::OwnedEvent;

/// Chunk-driven RTF parser that returns the full AST on finish.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        BatchParser { buf: Vec::new() }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish parsing and return the AST.
    pub fn finish(self) -> (RtfDoc, Vec<Diagnostic>) {
        crate::parse::parse(&self.buf)
    }
}

/// Chunk-driven RTF tokenizer that delivers low-level token events to a callback on finish.
pub struct BatchSink<F: FnMut(TokenEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(TokenEvent)> BatchSink<F> {
    pub fn new(callback: F) -> Self {
        BatchSink {
            buf: Vec::new(),
            callback,
        }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish and deliver all RTF token events to the callback.
    pub fn finish(mut self) {
        for event in crate::events::token_events(&self.buf) {
            (self.callback)(event);
        }
    }
}

/// Handler trait for semantic RTF events.
///
/// Implemented automatically for any `FnMut(OwnedEvent)`.
pub trait Handler {
    fn handle(&mut self, event: OwnedEvent);
}

impl<F: FnMut(OwnedEvent)> Handler for F {
    fn handle(&mut self, event: OwnedEvent) {
        self(event);
    }
}

/// Internal state machine driving [`StreamingParser`]. See the module doc's
/// "`StreamingParser`'s memory shape" section for the full picture.
enum Phase {
    /// Buffering until the RTF header (`\fonttbl`/`\colortbl`/`\stylesheet`/
    /// `\info`/`\*`-destination groups) is confirmed complete. `scan_pos` is
    /// the resume point [`crate::incremental::find_header_boundary`] gave us
    /// last time (avoids re-scanning already-checked bytes on every `feed()`).
    Header { buf: Vec<u8>, scan_pos: usize },
    /// Header resolved; `font_table`/`color_table`/`codepage` are fixed for
    /// the rest of the document. `buf` holds only the body bytes not yet
    /// consumed into a completed increment; `carry` is the inherited
    /// character/paragraph formatting state a table/list/paragraph spanning
    /// multiple increments needs.
    Body(Box<BodyState>),
}

/// Boxed payload of `Phase::Body` — pulled out to a separate struct (and
/// boxed at the enum level) purely to keep `Phase` itself small;
/// `BodyCarry`'s inherited-formatting fields make the unboxed variant large
/// enough to trip `clippy::large_enum_variant`.
struct BodyState {
    font_table: Vec<String>,
    color_table: Vec<(u8, u8, u8)>,
    codepage: u16,
    buf: Vec<u8>,
    scan_pos: usize,
    carry: crate::parse::BodyCarry,
    /// First-occurrence-order fonts actually referenced by the body so far,
    /// accumulated one increment at a time via
    /// [`crate::tables::collect_used_fonts_incremental`] as each increment's
    /// blocks are produced — the same accumulator threaded across every
    /// `pump()` iteration and `finish()`, so its final state (after the last
    /// increment) is byte-identical to `crate::tables::collect_used_fonts`
    /// run once over the whole parsed document. Reported (with the implicit
    /// `"Times New Roman"` default prepended, matching `build_font_map`) via
    /// `Event::TableOrderResolved` at `finish()`. Bounded by the number of
    /// distinct fonts actually used, not document size.
    used_fonts: Vec<String>,
    /// Same accumulation as `used_fonts`, for colors — see
    /// [`crate::tables::collect_used_colors_incremental`].
    used_colors: Vec<(u8, u8, u8)>,
    /// True only for the very first increment carved out of this document's
    /// `buf`. That increment still has the leading `{\rtf<N>...` bytes in
    /// front of it (the header phase only buffers long enough to compute the
    /// font/color tables — it does not slice them off, precisely because a
    /// bare control word between `{\rtf1` and the first recognized header
    /// group, e.g. `\ql` in `{\rtf1 \ql left\par...}`, has a real semantic
    /// effect `Parser::handle_control_word` must still apply; only the
    /// header's *destination groups* — `\fonttbl`/`\colortbl`/etc. — are
    /// skipped, and `Parser::run_body_step` already knows how to skip those
    /// itself via `is_skip_group`, same as the whole-document parse). So the
    /// first increment needs `Parser::skip_rtf_header` run before it, the
    /// exact same one call `Parser::run` makes once at the very start —
    /// every later increment is genuinely mid-document and must not repeat it.
    first_increment: bool,
}

/// Strip the parser's internal index-0 auto/default sentinel from a raw
/// `parse_color_table` result, matching `RtfDoc.color_table`'s convention
/// (see `parse::parse`'s doc comment for why: index 0 is a body-reference
/// sentinel, not a real declared color).
fn strip_color_sentinel(mut colors: Vec<(u8, u8, u8)>) -> Vec<(u8, u8, u8)> {
    if colors.len() > 1 {
        colors.remove(0);
        colors
    } else {
        colors.clear();
        colors
    }
}

/// Chunked streaming RTF parser delivering semantic document events to a
/// [`Handler`] — genuinely incrementally; see the module doc's
/// "`StreamingParser`'s memory shape" section for the full design and its
/// one documented, structural divergence from [`crate::sem_events::events`]
/// (`StartDocument`'s `fonts`/`colors` fields).
pub struct StreamingParser<H: Handler> {
    phase: Phase,
    handler: H,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            phase: Phase::Header {
                buf: Vec::new(),
                scan_pos: 0,
            },
            handler,
        }
    }

    /// Feed a chunk of bytes, advancing the parser and delivering every
    /// event that becomes available as a result.
    pub fn feed(&mut self, chunk: &[u8]) {
        match &mut self.phase {
            Phase::Header { buf, .. } => buf.extend_from_slice(chunk),
            Phase::Body(state) => state.buf.extend_from_slice(chunk),
        }
        self.pump();
    }

    /// Drive the state machine forward as far as currently-buffered bytes
    /// allow, delivering events to the handler as increments complete.
    /// Returns (without delivering anything further) once neither phase can
    /// make progress without more bytes.
    fn pump(&mut self) {
        loop {
            // Swap in a cheap placeholder so we can match on `self.phase` by
            // value (needed to move `buf`/`carry` out for `Parser::with_tables`,
            // which borrows them) and still assign a new `self.phase` from
            // inside the match arm.
            let phase = std::mem::replace(
                &mut self.phase,
                Phase::Header {
                    buf: Vec::new(),
                    scan_pos: 0,
                },
            );
            match phase {
                Phase::Header { buf, scan_pos } => {
                    match crate::incremental::find_header_boundary(&buf, scan_pos) {
                        Ok(boundary) => {
                            let font_table = crate::parse::parse_font_table(&buf[..boundary]);
                            // `color_table` here is the *raw* table (index 0
                            // = the auto/default sentinel, matching
                            // `Parser::color_table`'s own convention every
                            // `\cf<n>`/`\cb<n>` lookup in `make_inline`
                            // depends on) — it must be what `Parser::with_tables`
                            // receives. `StartDocument.colors` reports the
                            // *stripped* form (matching `RtfDoc.color_table`'s
                            // public convention), computed separately below,
                            // never stored.
                            let color_table = crate::parse::parse_color_table(&buf[..boundary]);
                            let codepage = crate::parse::parse_ansicpg(&buf[..boundary]);
                            self.handler.handle(OwnedEvent::StartDocument {
                                fonts: font_table.clone(),
                                colors: strip_color_sentinel(color_table.clone()),
                            });
                            // `buf` is kept whole (not sliced at `boundary`) —
                            // see `BodyState::first_increment`'s doc comment
                            // for why: a bare control word before the first
                            // header destination group can be semantically
                            // real (`\ql` etc.), so it must still reach
                            // `Parser::run_body_step`, not be discarded.
                            // `boundary` was only ever needed to know how
                            // much to buffer before the tables were
                            // computable.
                            self.phase = Phase::Body(Box::new(BodyState {
                                font_table,
                                color_table,
                                codepage,
                                buf,
                                scan_pos: 0,
                                carry: crate::parse::BodyCarry::new(0),
                                used_fonts: Vec::new(),
                                used_colors: Vec::new(),
                                first_increment: true,
                            }));
                            // loop again — body increments may already be available
                        }
                        Err(resume) => {
                            self.phase = Phase::Header {
                                buf,
                                scan_pos: resume,
                            };
                            return;
                        }
                    }
                }
                Phase::Body(mut state) => {
                    match crate::incremental::find_next_par_cut(&state.buf, state.scan_pos) {
                        Ok(cut) => {
                            let increment: Vec<u8> = state.buf.drain(..cut).collect();
                            let mut sub = crate::parse::Parser::with_tables(
                                &increment,
                                state.color_table.clone(),
                                state.font_table.clone(),
                                state.codepage,
                            );
                            if state.first_increment {
                                sub.skip_rtf_header();
                                state.first_increment = false;
                            }
                            let blocks =
                                crate::parse::normalize_blocks(sub.run_body_step(&mut state.carry));
                            crate::tables::collect_used_fonts_incremental(
                                &blocks,
                                &mut state.used_fonts,
                            );
                            crate::tables::collect_used_colors_incremental(
                                &blocks,
                                &mut state.used_colors,
                            );
                            for ev in crate::sem_events::SemanticEventIter::from_blocks(blocks) {
                                self.handler.handle(ev);
                            }
                            state.scan_pos = 0;
                            self.phase = Phase::Body(state);
                            // loop again — another cut may already be available
                        }
                        Err(resume) => {
                            state.scan_pos = resume;
                            self.phase = Phase::Body(state);
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Finish parsing: flush whatever is still buffered/in-progress and
    /// deliver the final events (including `EndDocument`) to the handler.
    pub fn finish(mut self) {
        match self.phase {
            Phase::Header { buf, .. } => {
                // Header never confirmed complete even at end of input:
                // malformed/truncated (e.g. an unclosed `\fonttbl` or a
                // `\binN` payload that never fully arrived). Fall back to a
                // full whole-buffer parse for this input only — exactly
                // `events()`'s output, so still correct, just not O(header)
                // for this one pathological case. See the module doc.
                for ev in crate::sem_events::events(&buf) {
                    self.handler.handle(ev);
                }
            }
            Phase::Body(mut state) => {
                // `state.buf` holds whatever body bytes never reached a
                // `\par`/`\pard` (e.g. a final paragraph with no trailing
                // `\par`) — process it exactly like any other increment,
                // then flush whatever `finalize_body` still owes
                // (unterminated table row/table, pending list, the final
                // partial paragraph).
                let mut sub = crate::parse::Parser::with_tables(
                    &state.buf,
                    state.color_table,
                    state.font_table,
                    state.codepage,
                );
                if state.first_increment {
                    sub.skip_rtf_header();
                }
                let mut blocks = sub.run_body_step(&mut state.carry);
                blocks.extend(sub.finalize_body(state.carry));
                let blocks = crate::parse::normalize_blocks(blocks);
                crate::tables::collect_used_fonts_incremental(&blocks, &mut state.used_fonts);
                crate::tables::collect_used_colors_incremental(&blocks, &mut state.used_colors);
                for ev in crate::sem_events::SemanticEventIter::from_blocks(blocks) {
                    self.handler.handle(ev);
                }
                // The whole body has now been seen: `used_fonts`/`used_colors`
                // are complete and byte-identical to what
                // `crate::tables::build_font_map`/`build_color_map` would
                // compute over the whole parsed document — see
                // `Event::TableOrderResolved`'s doc comment.
                let mut fonts = vec!["Times New Roman".to_string()];
                fonts.extend(state.used_fonts);
                self.handler.handle(OwnedEvent::TableOrderResolved {
                    fonts,
                    colors: state.used_colors,
                });
                self.handler.handle(OwnedEvent::EndDocument);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TokenEvent;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"{\\rtf1\\ansi Hello}");
        let (doc, _diags) = p.finish();
        // Should have at least one block
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"{\\rtf1\\ansi Hello}" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"{\\rtf1 Hello}");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, TokenEvent::GroupStart { .. }))
        );
    }

    // ── StreamingParser: adversarial chunk-splitting equivalence ───────────

    /// A reasonably rich RTF document: a heading-style run (bold + font
    /// size), plain bold/italic runs, a nested nested-group nested-group
    /// {bold {italic}} case (via the "Normal paragraph" run), a font table
    /// with two fonts, a color table, an explicit `\cf`/`\f` character run,
    /// a table, and a bulleted list (`{\*\pn\pnlvlblt...}`) — covering every
    /// construct category the task asked for except footnotes (already
    /// covered by `fixtures/rtf/footnote`, exercised by the fixture-corpus
    /// loop in `rescribe-fixtures`' `tests/streaming_apis.rs`).
    const RICH_SAMPLE: &[u8] = br#"{\rtf1\ansi\deff0
{\fonttbl{\f0 Times New Roman;}{\f1 Arial;}}
{\colortbl ;\red255\green0\blue0;\blue255;}
\pard\qc{\b\fs28 Heading}\par
\pard\ql Normal paragraph with \b bold\b0  and \i italic\i0  text.\par
{\cf1 Red }{\f1 Arial font}\par
\pard{\*\pn\pnlvlblt\pnf2\pnindent0{\pntxtb\'95}}\fi-360\li720 Bullet item one\par
\pard{\*\pn\pnlvlblt\pnf2\pnindent0{\pntxtb\'95}}\fi-360\li720 Bullet item two\par
\pard\trowd\intbl Cell1\cell Cell2\cell\row
\pard\par
}"#;

    /// Drive a `StreamingParser` over `input` split into `chunks` and return
    /// the collected events.
    fn stream_events(chunks: &[&[u8]]) -> Vec<OwnedEvent> {
        let mut collected = Vec::new();
        let mut parser = StreamingParser::new(|e| collected.push(e));
        for chunk in chunks {
            parser.feed(chunk);
        }
        parser.finish();
        collected
    }

    fn fixed_size_chunks(input: &[u8], n: usize) -> Vec<&[u8]> {
        input.chunks(n).collect()
    }

    fn single_byte_chunks(input: &[u8]) -> Vec<&[u8]> {
        input.iter().map(std::slice::from_ref).collect()
    }

    /// The one documented, structural divergence from `events()`:
    /// `StartDocument.fonts`/`colors` (see the module doc's
    /// "`StartDocument.fonts`/`colors` diverge..." section). Every other
    /// event must match exactly.
    fn assert_matches_events_except_start_document(input: &[u8], chunks: &[&[u8]], case: &str) {
        let bulk: Vec<_> = crate::sem_events::events(input).collect();
        let streamed = stream_events(chunks);
        assert_eq!(
            bulk.len(),
            streamed.len(),
            "{case}: event count differs (bulk={}, streamed={})",
            bulk.len(),
            streamed.len()
        );
        assert_eq!(
            &bulk[1..],
            &streamed[1..],
            "{case}: body events (everything after StartDocument) diverged from events()"
        );
        assert!(
            matches!(&bulk[0], OwnedEvent::StartDocument { .. })
                && matches!(&streamed[0], OwnedEvent::StartDocument { .. }),
            "{case}: first event must be StartDocument in both"
        );
    }

    #[test]
    fn test_streaming_parser_whole_input() {
        assert_matches_events_except_start_document(RICH_SAMPLE, &[RICH_SAMPLE], "whole");
    }

    #[test]
    fn test_streaming_parser_byte_at_a_time() {
        let chunks = single_byte_chunks(RICH_SAMPLE);
        assert_matches_events_except_start_document(RICH_SAMPLE, &chunks, "single_byte");
    }

    #[test]
    fn test_streaming_parser_chunks_of_3_7_13() {
        for n in [3usize, 7, 13] {
            let chunks = fixed_size_chunks(RICH_SAMPLE, n);
            assert_matches_events_except_start_document(
                RICH_SAMPLE,
                &chunks,
                &format!("chunks_of_{n}"),
            );
        }
    }

    #[test]
    fn test_streaming_parser_mid_control_word_split() {
        // Split right inside the control word `\colortbl` and right inside
        // `\pnlvlblt` (a control word nested inside a header/skip-style and
        // a body destination group, respectively).
        let colortbl_pos = RICH_SAMPLE
            .windows(9)
            .position(|w| w == b"\\colortbl")
            .unwrap()
            + 5; // split mid-word, after "\colo"
        let pnlvl_pos = RICH_SAMPLE
            .windows(9)
            .position(|w| w == b"\\pnlvlblt")
            .unwrap()
            + 4; // split mid-word, after "\pnlv"
        for split in [colortbl_pos, pnlvl_pos] {
            let chunks = vec![&RICH_SAMPLE[..split], &RICH_SAMPLE[split..]];
            assert_matches_events_except_start_document(
                RICH_SAMPLE,
                &chunks,
                &format!("mid_control_word@{split}"),
            );
        }
    }

    #[test]
    fn test_streaming_parser_mid_group_boundary_split() {
        // Split exactly on a `{`/`}` byte itself (the boundary between two
        // tokens), at both a header-group boundary and a body-group boundary.
        let fonttbl_close = RICH_SAMPLE.windows(2).position(|w| w == b";}").unwrap() + 2; // right after the first `\fonttbl` entry's closing `}`
        let bold_group_open = RICH_SAMPLE.windows(4).position(|w| w == b"{\\b\\").unwrap() + 1; // right after the `{` opening the heading's bold+fontsize group
        for split in [fonttbl_close, bold_group_open] {
            let chunks = vec![&RICH_SAMPLE[..split], &RICH_SAMPLE[split..]];
            assert_matches_events_except_start_document(
                RICH_SAMPLE,
                &chunks,
                &format!("mid_group_boundary@{split}"),
            );
        }
    }

    #[test]
    fn test_streaming_parser_fixture_like_matches_exactly() {
        // For a document with no `\fonttbl`/`\colortbl` at all,
        // `parse_font_table`'s "no \fonttbl" default now matches
        // `build_font_map`'s hardcoded `["Times New Roman"]` default (see the
        // module doc's item 2) — so `StartDocument` itself now matches
        // exactly here too, not just the body events. This test exists to
        // pin that down, since it used to be the concrete example of the
        // (now-fixed) mismatch.
        let input = br"{\rtf1 {\b bold}\par}";
        let bulk: Vec<_> = crate::sem_events::events(input).collect();
        let streamed = stream_events(&[input]);
        assert_eq!(
            bulk, streamed,
            "fonttbl-less document should now match events() exactly, including StartDocument"
        );
    }

    #[test]
    fn test_streaming_parser_incremental_before_finish() {
        // feed() alone (no finish()) must deliver events for a document with
        // a complete prefix, proving `feed()` is not a buffer-then-finish
        // stub. This is the harness's own incrementality probe
        // (`assert_streaming_parser_is_incremental` in
        // rescribe-fixtures/tests/streaming_apis.rs, using this exact probe
        // input), reproduced locally so this crate's own test suite (which
        // has no dependency on rescribe-fixtures — rtf-fmt is a standalone
        // crate) proves it too. Note the probe deliberately omits the
        // trailing space/closing brace after `\par`, so `\par` itself is
        // *not* guaranteed to be disambiguated yet (a control word ending
        // exactly at the fed prefix is genuinely ambiguous — see
        // `incremental::tests::par_cut_needs_more_at_trailing_word_boundary`)
        // — only `StartDocument` is guaranteed here, which is already enough
        // to prove `feed()` isn't buffering everything until `finish()`.
        let mut delivered = Vec::new();
        let mut parser = StreamingParser::new(|e| delivered.push(e));
        parser.feed(br"{\rtf1\ansi\deff0 Hello world\par");
        assert!(
            !delivered.is_empty(),
            "feed() with a complete prefix (deliberately missing the closing `}}`) must deliver \
             events before finish() is ever called"
        );
        assert!(
            delivered
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartDocument { .. })),
            "delivered: {delivered:?}"
        );

        // A prefix that unambiguously completes a paragraph (trailing space
        // after `\par`) must deliver the paragraph's own events too, still
        // before `finish()`.
        let mut delivered2 = Vec::new();
        let mut parser2 = StreamingParser::new(|e| delivered2.push(e));
        parser2.feed(br"{\rtf1\ansi\deff0 Hello world\par ");
        assert!(
            delivered2
                .iter()
                .any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello world")),
            "delivered2: {delivered2:?}"
        );
    }

    #[test]
    fn test_streaming_parser_malformed_header_falls_back() {
        // An `\fonttbl` group that never closes: the header boundary is
        // never confirmed even at finish(). Must still produce correct
        // (byte-for-byte identical to events()) output via the documented
        // fallback, not panic or drop content.
        let input = br"{\rtf1{\fonttbl{\f0 Times New Roman";
        let bulk: Vec<_> = crate::sem_events::events(input).collect();
        let streamed = stream_events(&[input]);
        assert_eq!(bulk, streamed);
    }

    #[test]
    fn test_streaming_parser_memory_bounded_by_header_not_document() {
        use crate::alloc_probe::{CURRENT_BYTES, PEAK_BYTES};

        // Build a synthetic document with `n` short paragraphs, fed in small
        // (32-byte) chunks — the shape a socket/pipe reader would actually
        // hand a parser.
        fn make_doc(n: usize) -> Vec<u8> {
            let mut doc = Vec::new();
            doc.extend_from_slice(br"{\rtf1\ansi\deff0{\fonttbl{\f0 Times New Roman;}}");
            for i in 0..n {
                doc.extend_from_slice(
                    format!("\\pard paragraph number {i} with some filler text\\par").as_bytes(),
                );
            }
            doc.push(b'}');
            doc
        }

        fn run_peak(n: usize) -> usize {
            // Build the input *before* resetting the peak tracker: `doc`
            // itself is necessarily O(document) (it's the test's stand-in
            // for "bytes already sitting in a buffer somewhere" — a socket
            // or file reader wouldn't have this cost at all), and must not
            // be attributed to the parser's own memory use. Only bytes
            // allocated *during* the feed loop below count toward peak.
            let doc = make_doc(n);
            PEAK_BYTES.with(|p| p.set(0));
            let before = CURRENT_BYTES.with(|c| c.get());
            // Count events rather than collecting them into a `Vec` that
            // outlives the parser — accumulating every event would itself
            // grow linearly with document size and swamp the measurement
            // this test exists to take: `StreamingParser`'s *own* peak
            // memory, not a test-harness artifact.
            let mut count = 0usize;
            let mut parser = StreamingParser::new(|_e| count += 1);
            for chunk in doc.chunks(32) {
                parser.feed(chunk);
            }
            parser.finish();
            assert!(count > 0);
            PEAK_BYTES.with(|p| p.get()).saturating_sub(before)
        }

        let small = run_peak(200).max(1);
        let large = run_peak(20_000); // 100x the paragraphs

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 20.0,
            "peak memory did not stay roughly constant across document sizes: \
             {small} bytes peak @200 paragraphs -> {large} bytes peak @20000 paragraphs \
             (ratio {ratio:.2}); this suggests StreamingParser is buffering O(document) \
             instead of O(header + single paragraph + nesting depth)"
        );
    }
}
