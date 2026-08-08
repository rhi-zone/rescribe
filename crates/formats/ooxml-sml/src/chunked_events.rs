//! Chunk-resumable SAX-style tokenizer for a single worksheet part's XML
//! (`xl/worksheets/sheetN.xml`), producing the same [`OwnedSmlEvent`]
//! vocabulary as [`crate::events::events`] but without requiring the part's
//! bytes to be buffered in full first.
//!
//! # Relationship to `SmlEventIter`
//!
//! This is a separate, independent implementation from [`crate::events::SmlEventIter`]
//! — not derived from it — per this repo's rule that `parse()`, `events()`,
//! and a chunk-fed streaming reader each get their own optimal
//! implementation, sharing only pure state-transition helper functions. The
//! two implementations share [`crate::events::build_container_start_event`],
//! [`crate::events::end_event_for`], [`crate::events::start_event_no_props`],
//! [`crate::events::text_leaf_event`], [`crate::events::local_name_owned`],
//! and [`crate::generated_events::dispatch_start`]/[`is_text_element`]/
//! [`props_strategy`] — none of which touch a `Reader`'s position beyond a
//! single already-complete tag's own bytes, so all of them are safe to call
//! from a resumable driver as well as `SmlEventIter`'s single-pass one.
//!
//! # The chunk-resumability technique
//!
//! Same technique as `docbook-fmt`'s `StreamingParser` (see that crate's
//! `batch.rs` module docs for the general argument): `quick_xml`'s slice
//! `Reader` reports a truncated tag/comment/CDATA/PI/decl as
//! `Err(Syntax(Unclosed*))` rather than silently treating it as finished, so
//! those tokens are unambiguous — safe to dispatch the instant they parse
//! successfully. The one ambiguous token is plain text: `quick_xml`
//! terminates a `Text` token either at the next `<` or at end-of-buffer, and
//! the two cases are indistinguishable from the return value alone. Each
//! [`ChunkedSmlEvents::drain`] call builds a **fresh** `Reader` over the
//! current unconsumed tail (`self.pending`), reads one token, and either
//! commits it (drains the consumed prefix out of `pending`, dispatches or
//! records state, loops again) or — for an ambiguous `Text` token, or a
//! `Syntax` error while more input may still arrive — leaves `pending`
//! untouched and returns, waiting for the next [`ChunkedSmlEvents::feed`].
//!
//! `check_end_names`/`allow_unmatched_ends` are disabled on each per-call
//! `Reader`, same reasoning as `docbook-fmt`: a fresh reader has no memory of
//! a `Start` tag consumed by an earlier `drain()` call, so it cannot itself
//! validate that a later `End` tag matches. This crate's target document
//! (well-formed worksheet XML) never exercises that validation regardless —
//! see "Malformed input" below for what changes on truly invalid XML.
//!
//! # Container element props: no lookahead needed
//!
//! Unlike WordprocessingML, SpreadsheetML row/cell properties (`<row r="2"
//! ht="15">`, `<c r="A1" t="s">`) come entirely from the *opening tag's own
//! attributes*, never from a following properties child element — this is
//! exactly the "Memory model" fact [`crate::events`]'s module docs already
//! establish for the whole-buffer iterator, and it is what makes a `Start`
//! container event safe to dispatch the instant its opening tag is fully
//! read, with no lookahead into the tag's children required at all. Empty
//! containers (`<row/>`, `<c/>`) are similarly atomic: `Start` + `End` are
//! dispatched back to back from the single `Empty` token.
//!
//! # Sub-states
//!
//! Besides top-level scanning for container/leaf/unknown elements, two
//! resumable sub-loops are tracked by hand across `drain()` calls (mirroring
//! `SmlEventIter`'s own two special-cased helpers, `skip_element` and
//! `read_text_content`, but made resumable):
//!
//! - **`Skipping { depth }`** — inside an element `SmlEventIter` doesn't
//!   track (e.g. `<phoneticPr>`), counting nested `Start`/`End` tokens by
//!   hand until the matching `End` is found. Content is discarded either
//!   way, so unlike the text case, an ambiguous token boundary inside a
//!   skip is harmless — it is always safe to commit and keep skipping.
//! - **`ReadingText { local, acc }`** — inside a text-content leaf (`v`, `t`,
//!   `f`): accumulates `Text`/`CData` runs (this is where the ambiguous-Text
//!   handling above actually matters, since here content correctness is at
//!   stake) until the matching `End`, then emits one [`SmlEvent::CellValue`]/
//!   [`SmlEvent::StringFragment`]/[`SmlEvent::Formula`] — mirrors
//!   `read_text_content`'s exact behavior, including that a stray non-text
//!   token (e.g. a comment) inside the leaf is silently skipped rather than
//!   ending the run, and that a text leaf never nests (no depth counter is
//!   needed — the first `End` token always closes it, matching
//!   `read_text_content`'s unconditional `break` on `End`).
//!
//! # Malformed input
//!
//! `SmlEventIter` relies on its single continuous `Reader`'s own
//! `check_end_names` validation to guarantee every `End` token it sees
//! already matches the correct open `Start` — it does not itself check tag
//! names (its top-level `End` handling just pops whatever kind is on top of
//! its context stack; `skip_element` just counts `Start`/`End`, not names).
//! Disabling that validation here (required — see above) means a chunked
//! reader trusts the same well-formedness a `Reader` would otherwise
//! enforce, rather than re-deriving it. For well-formed worksheet XML —
//! the only case this module claims byte-for-byte parity with `events()`
//! for — this makes no observable difference, since the validation would
//! never have fired. For malformed input, behavior is only guaranteed to
//! not panic (see this module's own `chunked_no_panic_on_arbitrary_bytes`
//! and `chunked_no_panic_on_empty_and_truncated_input` tests, plus
//! `batch.rs`'s equivalent full-pipeline tests), not to reproduce
//! `SmlEventIter`'s exact (already best-effort, silently-truncating)
//! recovery.
//!
//! [`SmlEvent::CellValue`]: crate::generated_events::SmlEvent::CellValue
//! [`SmlEvent::StringFragment`]: crate::generated_events::SmlEvent::StringFragment
//! [`SmlEvent::Formula`]: crate::generated_events::SmlEvent::Formula
//! [`is_text_element`]: crate::generated_events::is_text_element
//! [`props_strategy`]: crate::generated_events::props_strategy

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::events::{
    build_container_start_event, end_event_for, local_name_owned, text_leaf_event,
};
use crate::generated_events::{OwnedSmlEvent, SmlStartKind, dispatch_start, is_text_element};

/// One tracked container frame on the hand-maintained context stack —
/// mirrors `SmlEventIter`'s `ContextFrame`.
struct ContextFrame {
    kind: SmlStartKind,
}

/// What the tokenizer is currently doing, persisted across `drain()` calls.
enum SubState {
    /// Looking for the next container/leaf/unknown element at the current
    /// nesting depth (tracked via `stack`, not depth-counted here).
    Scanning,
    /// Inside an untracked element's subtree, discarding content until the
    /// matching `End` (`depth` counts nested `Start`s seen so far).
    Skipping { depth: u32 },
    /// Inside a text-content leaf (`v`/`t`/`f`), accumulating text until the
    /// matching `End`. `local` is the element's local name, used to pick the
    /// right `SmlEvent` variant on close.
    ReadingText { local: Vec<u8>, acc: String },
}

/// Chunk-resumable tokenizer for one worksheet part's XML. See the
/// [module docs](self) for the technique and its scope.
pub(crate) struct ChunkedSmlEvents {
    pending: Vec<u8>,
    stack: Vec<ContextFrame>,
    sub: SubState,
    started: bool,
    done: bool,
}

impl ChunkedSmlEvents {
    pub(crate) fn new() -> Self {
        ChunkedSmlEvents {
            pending: Vec::new(),
            stack: Vec::new(),
            sub: SubState::Scanning,
            started: false,
            done: false,
        }
    }

    /// Feed the next chunk of this worksheet part's XML bytes, dispatching
    /// every event that can be proven complete from the bytes seen so far.
    pub(crate) fn feed(&mut self, chunk: &[u8], emit: &mut dyn FnMut(OwnedSmlEvent)) {
        self.pending.extend_from_slice(chunk);
        self.drain(false, emit);
    }

    /// Signal that this part's bytes are complete: drain any remaining
    /// buffered state, resolving ambiguous trailing text and delivering the
    /// final synthetic `EndWorkbook` event exactly once — mirrors
    /// `SmlEventIter`'s own single unconditional `EndWorkbook` on real EOF.
    pub(crate) fn finish(mut self, emit: &mut dyn FnMut(OwnedSmlEvent)) {
        self.drain(true, emit);
    }

    /// Only present for the memory-bound test — the whole point of this
    /// module is that this stays small regardless of the part's total size.
    #[cfg(test)]
    pub(crate) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    fn drain(&mut self, is_final: bool, emit: &mut dyn FnMut(OwnedSmlEvent)) {
        if !self.started {
            self.started = true;
            emit(OwnedSmlEvent::StartWorkbook);
        }
        if self.done {
            return;
        }
        loop {
            if self.pending.is_empty() {
                if is_final {
                    self.finalize_at_eof(emit);
                }
                return;
            }

            let mut reader = Reader::from_reader(&self.pending[..]);
            reader.config_mut().trim_text(false);
            // See the module docs' "the chunk-resumability technique"
            // section for why these must be disabled on a fresh
            // per-`drain()` reader.
            reader.config_mut().check_end_names = false;
            reader.config_mut().allow_unmatched_ends = true;
            let total_len = self.pending.len();
            let mut buf = Vec::new();

            match std::mem::replace(&mut self.sub, SubState::Scanning) {
                SubState::Skipping { mut depth } => {
                    match reader.read_event_into(&mut buf) {
                        Ok(XmlEvent::Start(_)) => {
                            let consumed = reader.buffer_position() as usize;
                            self.pending.drain(0..consumed);
                            depth += 1;
                            self.sub = SubState::Skipping { depth };
                        }
                        Ok(XmlEvent::End(_)) => {
                            let consumed = reader.buffer_position() as usize;
                            self.pending.drain(0..consumed);
                            depth -= 1;
                            self.sub = if depth == 0 {
                                SubState::Scanning
                            } else {
                                SubState::Skipping { depth }
                            };
                        }
                        Ok(XmlEvent::Eof) => {
                            self.sub = SubState::Skipping { depth };
                            if !is_final {
                                return;
                            }
                            // Truncated mid-skip: silently abandon the
                            // subtree, mirroring `skip_element`'s own
                            // `Eof | Err(_) => break`.
                            self.sub = SubState::Scanning;
                        }
                        Err(_) => {
                            self.sub = SubState::Skipping { depth };
                            if !is_final {
                                return;
                            }
                            self.sub = SubState::Scanning;
                        }
                        Ok(_) => {
                            // Empty/Text/CData/Comment/PI/Decl/Doctype
                            // inside a skipped subtree: discarded, depth
                            // unchanged — matches `skip_element`'s `_ => {}`.
                            let consumed = reader.buffer_position() as usize;
                            self.pending.drain(0..consumed);
                            self.sub = SubState::Skipping { depth };
                        }
                    }
                }
                SubState::ReadingText { local, mut acc } => {
                    match reader.read_event_into(&mut buf) {
                        Ok(XmlEvent::Text(t)) => {
                            let consumed = reader.buffer_position() as usize;
                            let ambiguous_eof = consumed == total_len;
                            if ambiguous_eof && !is_final {
                                self.sub = SubState::ReadingText { local, acc };
                                return;
                            }
                            let content = t.decode().map(|c| c.into_owned()).unwrap_or_default();
                            self.pending.drain(0..consumed);
                            acc.push_str(&content);
                            self.sub = SubState::ReadingText { local, acc };
                        }
                        Ok(XmlEvent::CData(t)) => {
                            // CDATA is never ambiguous — a truncated CDATA
                            // section is a `Syntax` error, not a partial Ok.
                            let consumed = reader.buffer_position() as usize;
                            let content = t.decode().map(|c| c.into_owned()).unwrap_or_default();
                            self.pending.drain(0..consumed);
                            acc.push_str(&content);
                            self.sub = SubState::ReadingText { local, acc };
                        }
                        Ok(XmlEvent::End(_)) => {
                            let consumed = reader.buffer_position() as usize;
                            self.pending.drain(0..consumed);
                            emit(text_leaf_event(&local, acc));
                            self.sub = SubState::Scanning;
                        }
                        Ok(XmlEvent::Eof) => {
                            if !is_final {
                                self.sub = SubState::ReadingText { local, acc };
                                return;
                            }
                            // No closing tag ever arrived: emit whatever was
                            // accumulated, matching `read_text_content`'s own
                            // `Eof | Err(_) => break`.
                            emit(text_leaf_event(&local, acc));
                            self.sub = SubState::Scanning;
                        }
                        Err(_) => {
                            if !is_final {
                                self.sub = SubState::ReadingText { local, acc };
                                return;
                            }
                            emit(text_leaf_event(&local, acc));
                            self.sub = SubState::Scanning;
                        }
                        Ok(_) => {
                            // A stray non-text token inside the leaf (should
                            // not occur in valid worksheet XML): skipped,
                            // matching `read_text_content`'s `_ => {}`.
                            let consumed = reader.buffer_position() as usize;
                            self.pending.drain(0..consumed);
                            self.sub = SubState::ReadingText { local, acc };
                        }
                    }
                }
                SubState::Scanning => {
                    match reader.read_event_into(&mut buf) {
                        Ok(XmlEvent::Start(ref e)) => {
                            let local = local_name_owned(e.local_name().as_ref());
                            let consumed = reader.buffer_position() as usize;
                            if let Some(kind) = dispatch_start(&local) {
                                let tag_bytes = buf.clone();
                                let start_event =
                                    build_container_start_event(&mut reader, kind, &tag_bytes);
                                self.pending.drain(0..consumed);
                                self.stack.push(ContextFrame { kind });
                                self.sub = SubState::Scanning;
                                emit(start_event);
                            } else if is_text_element(&local) {
                                self.pending.drain(0..consumed);
                                self.sub = SubState::ReadingText {
                                    local,
                                    acc: String::new(),
                                };
                            } else {
                                self.pending.drain(0..consumed);
                                self.sub = SubState::Skipping { depth: 1 };
                            }
                        }
                        Ok(XmlEvent::Empty(ref e)) => {
                            let local = local_name_owned(e.local_name().as_ref());
                            let consumed = reader.buffer_position() as usize;
                            if let Some(kind) = dispatch_start(&local) {
                                let tag_bytes = buf.clone();
                                let start_event =
                                    build_container_start_event(&mut reader, kind, &tag_bytes);
                                let end_event = end_event_for(kind);
                                self.pending.drain(0..consumed);
                                self.sub = SubState::Scanning;
                                emit(start_event);
                                emit(end_event);
                            } else {
                                self.pending.drain(0..consumed);
                                self.sub = SubState::Scanning;
                            }
                        }
                        Ok(XmlEvent::End(_)) => {
                            let consumed = reader.buffer_position() as usize;
                            self.pending.drain(0..consumed);
                            self.sub = SubState::Scanning;
                            if let Some(frame) = self.stack.pop() {
                                emit(end_event_for(frame.kind));
                            }
                        }
                        Ok(XmlEvent::Eof) => {
                            self.sub = SubState::Scanning;
                            self.done = true;
                            self.pending.clear();
                            emit(OwnedSmlEvent::EndWorkbook);
                            return;
                        }
                        Err(_) => {
                            self.sub = SubState::Scanning;
                            if !is_final {
                                return;
                            }
                            self.done = true;
                            self.pending.clear();
                            emit(OwnedSmlEvent::EndWorkbook);
                            return;
                        }
                        Ok(_) => {
                            // Stray top-level Text/CData/Comment/PI/Decl/
                            // Doctype (e.g. whitespace between elements):
                            // discarded, matches `read_xml_info`'s catch-all
                            // `Ok(_) => XmlInfo::Other`. Content is never
                            // used, so an ambiguous Text boundary here is
                            // harmless — always safe to commit.
                            let consumed = reader.buffer_position() as usize;
                            self.pending.drain(0..consumed);
                            self.sub = SubState::Scanning;
                        }
                    }
                }
            }
        }
    }

    /// Called once, when `pending` is empty and no more input is coming.
    /// Resolves whatever sub-state was in progress and delivers the final
    /// `EndWorkbook`.
    fn finalize_at_eof(&mut self, emit: &mut dyn FnMut(OwnedSmlEvent)) {
        match std::mem::replace(&mut self.sub, SubState::Scanning) {
            SubState::ReadingText { local, acc } => {
                emit(text_leaf_event(&local, acc));
            }
            SubState::Skipping { .. } | SubState::Scanning => {}
        }
        self.done = true;
        emit(OwnedSmlEvent::EndWorkbook);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::events as whole_buffer_events;

    fn run_chunked(bytes: &[u8], chunk_size: usize) -> Vec<OwnedSmlEvent> {
        let mut out = Vec::new();
        let mut c = ChunkedSmlEvents::new();
        {
            let mut emit = |e: OwnedSmlEvent| out.push(e);
            if chunk_size == 0 {
                c.feed(bytes, &mut emit);
            } else {
                for chunk in bytes.chunks(chunk_size) {
                    c.feed(chunk, &mut emit);
                }
            }
            c.finish(&mut emit);
        }
        out
    }

    fn debug_seq(events: &[OwnedSmlEvent]) -> Vec<String> {
        events.iter().map(|e| format!("{e:?}")).collect()
    }

    const SAMPLE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1" spans="1:3">
      <c r="A1" t="s"><v>0</v></c>
      <c r="B1" t="s"><v>1</v></c>
      <c r="C1"><v>42</v></c>
    </row>
    <row r="2">
      <c r="A2" t="b"><v>1</v></c>
      <c r="B2"><f>SUM(A1:A1)</f><v>0</v></c>
      <c r="C2" t="inlineStr"><is><t>Hi &amp; bye</t></is></c>
    </row>
    <row r="3"/>
  </sheetData>
</worksheet>
"#;

    #[test]
    fn chunked_matches_whole_buffer_events_at_various_chunk_sizes() {
        let expected: Vec<OwnedSmlEvent> = whole_buffer_events(SAMPLE)
            .map(|e| e.into_owned())
            .collect();
        let expected_debug = debug_seq(&expected);

        for chunk_size in [0, 1, 2, 3, 7, 16, 64, 4096] {
            let actual = run_chunked(SAMPLE, chunk_size);
            assert_eq!(
                debug_seq(&actual),
                expected_debug,
                "chunk_size={chunk_size}"
            );
        }
    }

    #[test]
    fn chunked_no_panic_on_empty_and_truncated_input() {
        let mut out = Vec::new();
        {
            let c = ChunkedSmlEvents::new();
            let mut emit = |e: OwnedSmlEvent| out.push(e);
            c.finish(&mut emit);
        }
        assert_eq!(out.len(), 2); // StartWorkbook, EndWorkbook

        for cut in [0, 1, SAMPLE.len() / 3, SAMPLE.len() / 2, SAMPLE.len() - 1] {
            let mut out = Vec::new();
            let mut c = ChunkedSmlEvents::new();
            let mut emit = |e: OwnedSmlEvent| out.push(e);
            c.feed(&SAMPLE[..cut], &mut emit);
            c.finish(&mut emit);
            // Must not panic regardless of where input was cut off.
        }
    }

    #[test]
    fn chunked_no_panic_on_arbitrary_bytes() {
        for seed in [0u8, 1, 42, 255] {
            let data: Vec<u8> = (0..2000).map(|i| (i as u8).wrapping_add(seed)).collect();
            let mut out = Vec::new();
            let mut c = ChunkedSmlEvents::new();
            let mut emit = |e: OwnedSmlEvent| out.push(e);
            for chunk in data.chunks(13) {
                c.feed(chunk, &mut emit);
            }
            c.finish(&mut emit);
        }
    }

    #[test]
    fn pending_buffer_stays_small_for_a_large_worksheet() {
        // Tens of thousands of rows: total XML size is large, but the
        // internal `pending` buffer should never grow anywhere near it —
        // that is the structural evidence the fix landed, not just that
        // output is correct (see `batch.rs`'s equivalent full-pipeline
        // test for the end-to-end version of this assertion).
        let mut xml = String::from(
            "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
             <sheetData>",
        );
        for r in 1..=50_000u32 {
            xml.push_str(&format!(
                "<row r=\"{r}\"><c r=\"A{r}\"><v>{r}</v></c><c r=\"B{r}\" t=\"str\"><v>row-{r}-value</v></c></row>"
            ));
        }
        xml.push_str("</sheetData></worksheet>");
        let bytes = xml.into_bytes();
        assert!(
            bytes.len() > 2_000_000,
            "expected a multi-MB worksheet, got {} bytes",
            bytes.len()
        );

        let mut c = ChunkedSmlEvents::new();
        let mut event_count = 0usize;
        let mut max_pending = 0usize;
        {
            let mut emit = |_e: OwnedSmlEvent| event_count += 1;
            for chunk in bytes.chunks(256) {
                c.feed(chunk, &mut emit);
                max_pending = max_pending.max(c.pending_len());
            }
            c.finish(&mut emit);
        }

        assert!(event_count > 50_000 * 4, "got {event_count} events");
        assert!(
            max_pending < 4096,
            "internal pending buffer grew to {max_pending} bytes while streaming a \
             {}-byte worksheet — expected it to stay bounded by nesting depth + \
             largest token, not the whole part",
            bytes.len()
        );
    }
}
