//! Chunk-driven (batch) Djot parser.
//!
//! # Memory model
//!
//! [`StreamingParser`] processes input in logical blocks. Memory usage is
//! O(largest block), not O(full input). [`BatchParser`] buffers all input.
//!
//! # Example — AST style
//! ```no_run
//! use djot_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"# Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use djot_fmt::batch::{StreamingParser, Handler};
//! use djot_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"# Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, DjotDoc};
use crate::events::OwnedEvent;

/// Chunk-driven Djot parser that returns the full AST on finish.
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
    pub fn finish(self) -> (DjotDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse_str(&s)
    }
}

/// Handler trait for streaming Djot events — the shared
/// [`rescribe_format_api::Handler`], not a locally declared trait; see that
/// crate's docs for why bounding `H` by one shared trait (instead of each
/// format crate declaring its own concrete `Handler`) is required for a
/// common `StreamingParse` trait to exist at all. Implemented automatically
/// for any `FnMut(OwnedEvent)`.
pub use rescribe_format_api::Handler;

/// Block accumulation state for the streaming parser.
enum BlockState {
    Between,
    Accumulating,
    /// Inside a fenced code block.  `fence` is the opening fence string
    /// (e.g. "```" or "````") used to detect the closing fence.
    InFencedCode {
        fence: String,
    },
    /// Inside a div block (`:::` … `:::`), possibly nested. `depth` counts
    /// unclosed `:::`-openers seen so far (starts at 1 for the outermost
    /// div). A bare `:::` line (no trailing class text) decrements `depth`;
    /// the block ends when `depth` reaches 0. A `::: class` line (trailing
    /// text after the colons) is a *nested* opener and increments `depth` —
    /// mirrors `find_div_close_generic`'s depth tracking in parse.rs, which
    /// is the ground truth for how `events()`/`parse()` match div closers.
    InDiv {
        depth: usize,
    },
}

/// Chunked streaming Djot parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block) for the common case. Fenced code blocks and div
/// blocks (including nested divs) are buffered until their closing
/// fence/marker. All other content is buffered until the next blank line
/// that doesn't continue a list/definition-list (see [`BlockState`] and
/// `feed_line`'s blank-line handling).
///
/// One exception: a block containing an explicit reference-style link
/// (`[text][label]`) can't be resolved against a same-block `pre_scan` alone
/// — the `[label]: url` definition may live in an earlier *or later* block.
/// Once such a block is seen, this parser switches into a document-buffering
/// mode for the rest of the input (`deferred`), so `link_defs` accumulated
/// from every block fed so far — and every block still to come — are all
/// available by the time deferred blocks are actually emitted, at
/// [`finish`](StreamingParser::finish). This trades O(largest block) memory
/// for O(remaining document) memory, but only for documents that use
/// explicit reference-style links; documents without them never enter this
/// mode. Bare shortcut references (`[label]` with no second bracket pair)
/// are not detected by this heuristic and are not deferred — a caller
/// relying on forward-declared shortcut references may still see an
/// unresolved link from `StreamingParser` where `events()` would have
/// resolved it. See TODO.md.
pub struct StreamingParser<H: Handler<OwnedEvent>> {
    handler: H,
    line_buf: Vec<u8>,
    block_lines: Vec<String>,
    /// Blank lines seen since the last content line, held back (not yet
    /// merged into `block_lines` or flushed) until the next non-blank line
    /// reveals whether they end the block or are a loose-list separator.
    held_blanks: Vec<String>,
    state: BlockState,
    /// Link definitions collected from every block fed so far (see the
    /// struct doc's note on `deferred`).
    link_defs: Vec<crate::ast::LinkDef>,
    /// Once true, every subsequent completed block is pushed to `deferred`
    /// instead of being emitted immediately, preserving document order
    /// until `finish()` flushes them with the final `link_defs`.
    deferred_mode: bool,
    deferred: Vec<String>,
}

impl<H: Handler<OwnedEvent>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            held_blanks: Vec::new(),
            state: BlockState::Between,
            link_defs: Vec::new(),
            deferred_mode: false,
            deferred: Vec::new(),
        }
    }

    /// Feed a chunk of bytes.  May call `handler.handle()` zero or more times.
    pub fn feed(&mut self, chunk: &[u8]) {
        for &byte in chunk {
            if byte == b'\n' {
                if self.line_buf.last() == Some(&b'\r') {
                    self.line_buf.pop();
                }
                let line = String::from_utf8_lossy(&self.line_buf).into_owned();
                self.line_buf.clear();
                self.feed_line(line);
            } else {
                self.line_buf.push(byte);
            }
        }
    }

    fn feed_line(&mut self, line: String) {
        let trimmed = line.trim().to_owned();

        // Inside fenced code: accumulate until closing fence
        let close_fence: Option<bool> = if let BlockState::InFencedCode { ref fence } = self.state {
            Some(trimmed == *fence)
        } else {
            None
        };
        if let Some(is_close) = close_fence {
            self.block_lines.push(line);
            if is_close {
                self.emit_block();
                self.state = BlockState::Between;
            }
            return;
        }

        // Inside div: accumulate, tracking nesting depth (see BlockState::InDiv).
        if let BlockState::InDiv { depth } = self.state {
            self.block_lines.push(line);
            if let Some(stripped) = trimmed.strip_prefix(":::") {
                let rest = stripped.trim();
                if rest.is_empty() {
                    // Bare `:::` — closes one level of nesting.
                    let new_depth = depth - 1;
                    if new_depth == 0 {
                        self.emit_block();
                        self.state = BlockState::Between;
                    } else {
                        self.state = BlockState::InDiv { depth: new_depth };
                    }
                } else {
                    // `::: class` — a nested opener.
                    self.state = BlockState::InDiv { depth: depth + 1 };
                }
            }
            return;
        }

        if trimmed.is_empty() {
            if self.block_lines.is_empty() {
                self.state = BlockState::Between;
                return;
            }
            // Hold the blank line: whether it ends the current block or is
            // a loose-list separator is only decidable once the next
            // non-blank line arrives (see below).
            self.held_blanks.push(line);
            return;
        }

        // A non-blank line arrived with blank lines pending. If both the
        // held block and this new line are list/definition-list starts,
        // the blank run is a loose-list separator, not a block boundary —
        // merge it in and keep accumulating. Otherwise the blank run really
        // did end the block: flush now, then fall through to classify this
        // line as usual.
        if !self.held_blanks.is_empty() {
            let continues_list =
                is_list_start_line(&trimmed) && block_starts_with_list(&self.block_lines);
            if continues_list {
                self.block_lines.append(&mut self.held_blanks);
            } else {
                self.emit_block();
                self.held_blanks.clear();
                self.state = BlockState::Between;
            }
        }

        // Fenced code block open: line is 3+ backticks or tildes
        if let Some(fence) = detect_fence(&trimmed) {
            if !self.block_lines.is_empty() && !block_is_only_pending_attrs(&self.block_lines) {
                self.emit_block();
            }
            self.state = BlockState::InFencedCode { fence };
            self.block_lines.push(line);
            return;
        }

        // Div block open: line starting with `:::`
        if trimmed.starts_with(":::") && trimmed.len() >= 3 {
            if !self.block_lines.is_empty() && !block_is_only_pending_attrs(&self.block_lines) {
                self.emit_block();
            }
            self.state = BlockState::InDiv { depth: 1 };
            self.block_lines.push(line);
            return;
        }

        self.state = BlockState::Accumulating;
        self.block_lines.push(line);
    }

    fn emit_block(&mut self) {
        if self.block_lines.is_empty() {
            return;
        }
        let text = self.block_lines.join("\n");
        self.block_lines.clear();

        // Collect any `[label]: url` definitions in this block so blocks
        // elsewhere (earlier — via `deferred` — or later) can resolve
        // references against them; see the struct doc.
        for candidate in text.split('\n') {
            let t = candidate.trim();
            if t.starts_with('[')
                && !t.starts_with("[^")
                && let Some(ld) = crate::parse::parse_link_def(t)
            {
                self.link_defs.push(ld);
            }
        }

        if !self.deferred_mode && contains_explicit_ref_link(&text) {
            self.deferred_mode = true;
        }

        if self.deferred_mode {
            self.deferred.push(text);
        } else {
            self.emit_text_block(&text);
        }
    }

    fn emit_text_block(&mut self, text: &str) {
        for event in
            crate::events::EventIter::new_with_extra_link_defs(text, self.link_defs.clone())
        {
            self.handler.handle(event.into_owned());
        }
    }

    /// Flush any remaining input and deliver final events.
    pub fn finish(mut self) {
        if !self.line_buf.is_empty() {
            if self.line_buf.last() == Some(&b'\r') {
                self.line_buf.pop();
            }
            let line = String::from_utf8_lossy(&self.line_buf).into_owned();
            self.feed_line(line);
        }
        if !self.held_blanks.is_empty() {
            self.held_blanks.clear();
        }
        self.emit_block();
        let deferred = std::mem::take(&mut self.deferred);
        for text in deferred {
            self.emit_text_block(&text);
        }
    }
}

/// True if `trimmed` starts a bullet/ordered/definition list item.
fn is_list_start_line(trimmed: &str) -> bool {
    trimmed == ":"
        || trimmed.starts_with(": ")
        || crate::parse::detect_list_marker(trimmed).is_some()
}

/// True if the first line of an accumulated block is itself a list start —
/// used to decide whether a blank line inside `block_lines` is a loose-list
/// separator rather than a block boundary.
fn block_starts_with_list(block_lines: &[String]) -> bool {
    block_lines
        .first()
        .is_some_and(|l| is_list_start_line(l.trim()))
}

/// True if every line accumulated so far is a pending block-attribute line
/// (`{.python}`, `{#id}`, …). Used to decide whether a fence/div opener
/// should absorb a preceding attribute line into the same block (so the
/// attribute reaches the fence/div it decorates) instead of flushing it away
/// as its own block, where it would set `pending_attr` on a throwaway
/// `EventIter` and never be read.
fn block_is_only_pending_attrs(block_lines: &[String]) -> bool {
    !block_lines.is_empty()
        && block_lines.iter().all(|l| {
            let t = l.trim();
            t.starts_with('{') && crate::parse::looks_like_attr_line(t)
        })
}

/// Heuristic for "this block might contain an explicit reference-style link
/// (`[text][label]`) whose `[label]: url` definition lives in a different
/// block". Deliberately conservative (a false positive only costs memory,
/// via `deferred_mode`); a `][` substring is unambiguous for this syntax and
/// can't appear inside a footnote reference (`[^label]`) or an inline link
/// with an explicit URL (`[text](url)`).
fn contains_explicit_ref_link(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(2).any(|w| w == b"][")
}

/// If `line` is a fenced code opener (3+ backticks or 3+ tildes), return the
/// fence string (backticks/tildes only, no info string).
fn detect_fence(line: &str) -> Option<String> {
    let ch = line.chars().next()?;
    if !matches!(ch, '`' | '~') {
        return None;
    }
    let fence_len = line.chars().take_while(|&c| c == ch).count();
    if fence_len >= 3 {
        Some(std::iter::repeat_n(ch, fence_len).collect())
    } else {
        None
    }
}

/// Chunk-driven Djot parser that delivers events to a callback on finish.
///
/// Prefer [`StreamingParser`] for new code; `BatchSink` is kept for
/// backwards compatibility.
pub struct BatchSink<F: FnMut(OwnedEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(OwnedEvent)> BatchSink<F> {
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

    /// Finish parsing and deliver all events to the callback.
    pub fn finish(mut self) {
        let s = String::from_utf8_lossy(&self.buf).into_owned();
        for event in crate::events::EventIter::new(&s) {
            (self.callback)(event.into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"# Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"# Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"# Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. }))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartParagraph { .. }))
        );
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"# Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { .. }))
        );
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"# Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events_str(&s).map(|e| e.into_owned()).collect()
        };

        let mut streamed: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        for chunk in input.chunks(7) {
            p.feed(chunk);
        }
        p.finish();

        assert_eq!(bulk, streamed);
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"# Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartParagraph { .. }))
        );
    }
}
