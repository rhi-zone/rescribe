//! Chunk-driven (batch) Textile parser.
//!
//! Feed input in arbitrarily-sized chunks with [`BatchParser::feed`], then
//! call [`BatchParser::finish`] to get the full AST.
//!
//! For event-driven use, see [`StreamingParser`] and [`Handler`].
//!
//! # Memory model
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//! Use it when you need the complete AST.
//!
//! [`StreamingParser`] delivers events to a [`Handler`] incrementally as
//! `feed()` is called: it accumulates lines into a small pending buffer and,
//! on every new line, re-runs [`crate::parse::BlockCursor`] (the same
//! block-boundary logic [`crate::parse::parse`] and [`crate::events::events`]
//! use) over just that pending tail. A block is confirmed complete — and its
//! events flushed to the handler — the moment the cursor's parse stops short
//! of the buffered tail's end (proof that no future input can change the
//! boundary decision, since every block-parsing arm only ever inspects lines
//! up to and including the one that ends it). Only the still-open block's
//! lines stay buffered, so memory is O(largest block), not O(full input).
//!
//! # Example — AST style
//! ```no_run
//! use textile_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"h1. Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use textile_fmt::batch::{StreamingParser, Handler};
//! use textile_fmt::TextileEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: TextileEvent| events.push(ev));
//! p.feed(b"h1. Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, TextileDoc};
use crate::events::{TextileEvent, push_block_events};
use crate::parse::BlockCursor;

/// Chunk-driven Textile parser that returns the full AST on finish.
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
    pub fn finish(self) -> (TextileDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Textile events.
///
/// Implemented automatically for any `FnMut(TextileEvent)`.
pub trait Handler {
    fn handle(&mut self, event: TextileEvent);
}

impl<F: FnMut(TextileEvent)> Handler for F {
    fn handle(&mut self, event: TextileEvent) {
        self(event);
    }
}

/// Chunk-driven Textile parser that delivers events to a [`Handler`]
/// incrementally, as soon as each top-level block is confirmed complete.
///
/// Memory: O(largest block), not O(full input) — see the module docs.
/// Split tokens (partial lines, mid-UTF-8-character byte splits) at chunk
/// boundaries are buffered internally, not the caller's concern.
pub struct StreamingParser<H: Handler> {
    handler: H,
    /// Bytes of the current in-progress line (no `\n` yet). Handles chunk
    /// boundaries landing mid-line or mid-UTF-8-character: `String::from_utf8_lossy`
    /// is only ever applied to a complete line once its trailing `\n` arrives.
    line_buf: Vec<u8>,
    /// Lines belonging to the block(s) not yet confirmed complete.
    pending_lines: Vec<String>,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            pending_lines: Vec::new(),
        }
    }

    /// Feed a chunk of bytes. May call `handler.handle()` zero or more times.
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
        self.pending_lines.push(line);
        self.try_flush();
    }

    /// Parse as many confirmed-complete top-level blocks as possible out of
    /// `pending_lines`, deliver their events to the handler, and trim the
    /// consumed lines off. Leaves the current still-open block (if any)
    /// buffered in `pending_lines`.
    fn try_flush(&mut self) {
        if self.pending_lines.is_empty() {
            return;
        }
        let text = self.pending_lines.join("\n");
        let mut cursor = BlockCursor::new(&text);
        let mut confirmed = Vec::new();
        let mut consumed_lines = 0usize;
        loop {
            match cursor.next_block() {
                None => {
                    // Only blank lines (or nothing) remain: consumed, no block.
                    consumed_lines = cursor.pos();
                    break;
                }
                Some(block) => {
                    if cursor.pos() < cursor.line_count() {
                        // More buffered lines exist past this block, so its
                        // boundary decision can never change — confirmed.
                        consumed_lines = cursor.pos();
                        confirmed.push(block);
                    } else {
                        // Ran up to the buffered tail's end: might still
                        // grow with more input. Leave it (and any lines
                        // already accounted for by it) pending.
                        break;
                    }
                }
            }
        }
        for block in confirmed {
            let mut evs = Vec::new();
            push_block_events(&block, &mut evs);
            for ev in evs {
                self.handler.handle(ev);
            }
        }
        if consumed_lines > 0 {
            self.pending_lines.drain(0..consumed_lines);
        }
    }

    /// Flush any remaining buffered input and deliver the final events.
    pub fn finish(mut self) {
        if !self.line_buf.is_empty() {
            if self.line_buf.last() == Some(&b'\r') {
                self.line_buf.pop();
            }
            let line = String::from_utf8_lossy(&self.line_buf).into_owned();
            self.line_buf.clear();
            self.pending_lines.push(line);
        }
        if !self.pending_lines.is_empty() {
            let text = self.pending_lines.join("\n");
            let (doc, _diags) = crate::parse::parse(&text);
            for block in &doc.blocks {
                let mut evs = Vec::new();
                push_block_events(block, &mut evs);
                for ev in evs {
                    self.handler.handle(ev);
                }
            }
        }
    }
}

/// Chunk-driven Textile parser that delivers events to a callback on finish.
///
/// Prefer [`StreamingParser`] for new code; `BatchSink` is kept for
/// API symmetry with other format crates.
pub struct BatchSink<F: FnMut(TextileEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(TextileEvent)> BatchSink<F> {
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
        let s = String::from_utf8_lossy(&self.buf);
        for event in crate::events::events(&s) {
            (self.callback)(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"h1. Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"h1. Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_matches_parse() {
        let input = "h1. Title\n\nA paragraph.\n";
        let (expected_doc, _) = crate::parse::parse(input);

        let mut p = BatchParser::new();
        p.feed(input.as_bytes());
        let (actual_doc, _) = p.finish();

        assert_eq!(expected_doc.blocks.len(), actual_doc.blocks.len());
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"h1. Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, TextileEvent::StartHeading { level: 1, .. }))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, TextileEvent::StartParagraph { .. }))
        );
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<TextileEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"h1. Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, TextileEvent::StartHeading { .. }))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, TextileEvent::StartParagraph { .. }))
        );
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"h1. Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<TextileEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s).collect()
        };

        let mut streamed: Vec<TextileEvent> = Vec::new();
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
        sink.feed(b"h1. Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, TextileEvent::StartHeading { level: 1, .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, TextileEvent::StartParagraph { .. }))
        );
    }

    // ── Adversarial chunking ─────────────────────────────────────────────

    fn bulk_events(input: &[u8]) -> Vec<TextileEvent> {
        let s = String::from_utf8_lossy(input);
        crate::events::events(&s).collect()
    }

    fn streamed_with_chunks(input: &[u8], chunk_size: usize) -> Vec<TextileEvent> {
        let mut streamed = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        for chunk in input.chunks(chunk_size.max(1)) {
            p.feed(chunk);
        }
        p.finish();
        streamed
    }

    fn streamed_whole(input: &[u8]) -> Vec<TextileEvent> {
        let mut streamed = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        p.feed(input);
        p.finish();
        streamed
    }

    /// Feed `input` byte-by-byte — every keyword, block-prefix, and blank-line
    /// boundary torn apart across `feed()` calls.
    fn streamed_single_byte(input: &[u8]) -> Vec<TextileEvent> {
        streamed_with_chunks(input, 1)
    }

    const ADVERSARIAL_INPUT: &[u8] = b"h1. Title\n\nA paragraph with *bold* and _italic_.\n\n\
* item one\n* item two\n** nested\n\nbq.. A blockquote paragraph.\n\nSecond quoted paragraph.\n\n\
p. After the blockquote.\n\nbc.. fn main() {\n    println!(\"hi\");\n}\n\n\
|_. Name|_. Age|\n|Alice|30|\n|Bob|25|\n\nfn1. A footnote.\n\n---\n\n\
;Term\n:Definition\n\ncaf\xc3\xa9 society with an accented word.\n";

    #[test]
    fn test_adversarial_whole_matches_bulk() {
        let bulk = bulk_events(ADVERSARIAL_INPUT);
        assert_eq!(bulk, streamed_whole(ADVERSARIAL_INPUT));
    }

    #[test]
    fn test_adversarial_single_byte_matches_bulk() {
        let bulk = bulk_events(ADVERSARIAL_INPUT);
        assert_eq!(bulk, streamed_single_byte(ADVERSARIAL_INPUT));
    }

    #[test]
    fn test_adversarial_chunks_of_n_match_bulk() {
        let bulk = bulk_events(ADVERSARIAL_INPUT);
        for n in [2, 3, 5, 7, 11, 13, 17, 31, 64] {
            assert_eq!(
                bulk,
                streamed_with_chunks(ADVERSARIAL_INPUT, n),
                "diverged at chunk size {n}"
            );
        }
    }

    #[test]
    fn test_adversarial_mid_utf8_char_split() {
        // "café" — 'é' is 2 UTF-8 bytes (0xC3 0xA9). Split it across two
        // feed() calls, and also split mid multi-byte word elsewhere.
        let input = "h1. caf\u{e9} title\n\nAnother caf\u{e9} paragraph with na\u{ef}ve.\n"
            .as_bytes()
            .to_vec();
        let bulk = bulk_events(&input);
        // Byte-at-a-time exercises every possible mid-character split point.
        assert_eq!(bulk, streamed_single_byte(&input));
        // A handful of fixed chunk sizes chosen to land mid-character for
        // this specific input (the string's multi-byte sequences start at
        // varying offsets).
        for n in [1, 2, 3, 4, 5, 6] {
            assert_eq!(
                bulk,
                streamed_with_chunks(&input, n),
                "diverged at chunk size {n}"
            );
        }
    }

    #[test]
    fn test_adversarial_extended_blockquote_spanning_blank_lines() {
        // bq.. spans multiple blank-line-separated paragraphs until an
        // explicit block-start line — the one construct whose boundary
        // isn't decidable from blank lines alone.
        let input = b"bq.. First quoted paragraph.\n\nSecond quoted paragraph.\n\n\
Third quoted paragraph.\n\np. Not part of the quote.\n";
        let bulk = bulk_events(input);
        assert_eq!(bulk, streamed_single_byte(input));
        assert_eq!(bulk, streamed_with_chunks(input, 9));
    }

    /// Regression guard against reintroducing "buffer everything, parse only
    /// in finish()": `feed()` alone (no `finish()`) must deliver events for
    /// multi-block input, and peak memory must stay roughly flat (not grow
    /// linearly with document size) across a 10x input-size increase.
    ///
    /// `thread_local!` counters, not a shared `AtomicUsize`: the allocator is
    /// process-wide and `cargo test` runs other tests concurrently on other
    /// threads by default, so a shared counter lets unrelated tests inflate
    /// this measurement — confirmed as a real flake in this batch's `pod-fmt`
    /// sibling crate (a spurious 407x ratio under full-workspace `cargo test
    /// -q`, passing cleanly under `--test-threads=1`).
    #[test]
    fn test_streaming_parser_feed_alone_delivers_events_and_peak_memory_flat() {
        use crate::alloc_probe;

        /// A synthetic multi-block document: `n` independent
        /// heading+paragraph+list groups, each closed by a blank line, so
        /// every earlier block is confirmed-complete well before the end of
        /// the document — the scenario a buffer-everything implementation
        /// would fail (peak memory growing with document size) and a
        /// genuinely incremental one should pass (peak memory flat).
        fn synthetic_doc(n: usize) -> Vec<u8> {
            let mut s = String::new();
            for i in 0..n {
                s.push_str(&format!("h2. Section {i}\n\n"));
                s.push_str(&format!(
                    "A paragraph with *bold text* and _italic text_ in section {i}.\n\n"
                ));
                s.push_str("* item one\n* item two\n* item three\n\n");
            }
            s.into_bytes()
        }

        // Regression guard: feed() alone (well under half the document,
        // never calling finish()) must deliver events for large multi-block
        // input — the exact defect this rewrite fixes.
        {
            let doc = synthetic_doc(50);
            let mut delivered: Vec<TextileEvent> = Vec::new();
            let mut p = StreamingParser::new(|e| delivered.push(e));
            p.feed(&doc[..doc.len() / 4]);
            assert!(
                !delivered.is_empty(),
                "feed() with a quarter of a 50-section synthetic document delivered zero \
                 events before finish() — StreamingParser is not genuinely incremental"
            );
            // `p` intentionally dropped without finish(): this probe only
            // needs to observe pre-finish handler state.
        }

        fn run_peak(n: usize) -> usize {
            let before = alloc_probe::reset_peak();
            let doc = synthetic_doc(n);
            let mut count = 0usize;
            {
                let mut p = StreamingParser::new(|_e| count += 1);
                // Feed in small chunks to exercise real incremental
                // accumulation rather than one giant feed() call.
                for chunk in doc.chunks(64) {
                    p.feed(chunk);
                }
                p.finish();
            }
            std::hint::black_box(count);
            alloc_probe::peak_since_reset(before)
        }

        let small_peak = run_peak(20).max(1);
        let large_peak = run_peak(200);
        let ratio = large_peak as f64 / small_peak as f64;
        assert!(
            ratio < 20.0,
            "peak memory did not stay roughly constant across a 10x document-size increase: \
             {small_peak} bytes peak @20 sections -> {large_peak} bytes peak @200 sections \
             (ratio {ratio:.2}); this suggests StreamingParser is buffering O(document) \
             instead of O(largest block)"
        );
    }
}
