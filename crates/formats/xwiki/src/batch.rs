//! Chunk-driven (batch) XWiki parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] accumulates one top-level block at a time — a
//! paragraph, heading, list, table, code block, or macro/quote block — the
//! same boundaries [`crate::parse::parse`]'s dispatch loop uses. As soon as a
//! block's boundary is confirmed it is flushed: reparsed in isolation via
//! [`crate::parse::parse`] and walked with [`crate::events::events`], and
//! every event reaches the handler before the next block starts
//! accumulating. Memory is O(largest block), not O(full input).
//! [`BatchParser`] buffers all input and is O(full input) — it materializes
//! the whole [`crate::ast::XwikiDoc`], which needs the complete document
//! anyway.
//!
//! Macro/code/quote blocks (`{{code}}...{{/code}}`, `{{quote}}...{{/quote}}`,
//! `{{name}}...{{/name}}`) can span many lines, including blank lines
//! internally — [`StreamingParser`] tracks this with an explicit state (not
//! just a flag) so it isn't fooled by a blank line into flushing early.
//! XWiki's own recursive-descent parser does not track macro-block nesting
//! depth either (it closes on the first line containing the matching
//! `{{/name}}` end tag, regardless of nested same-name macros) — the
//! streaming parser matches that behavior exactly rather than introducing a
//! depth counter that would diverge from `parse()`/`events()`.
//!
//! # Example — AST style
//! ```no_run
//! use xwiki::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"= Hello =\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use xwiki::batch::{StreamingParser, Handler};
//! use xwiki::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"= Hello =\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, XwikiDoc};
use crate::events::OwnedEvent;
use crate::parse::{try_parse_block_macro_start, try_parse_self_closing_macro};

/// Chunk-driven XWiki parser that returns the full AST on finish.
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
    pub fn finish(self) -> (XwikiDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming XWiki events.
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

/// Block-accumulation state for the streaming parser.
///
/// Each variant mirrors one of the block dispatch arms in
/// [`crate::parse::Parser::parse`]. `MacroBlock` carries the macro name so
/// the matching `{{/name}}` end tag can be recognized.
enum BlockState {
    /// Not currently accumulating a block.
    Between,
    /// Accumulating a paragraph; ends at a blank line or any other block's
    /// start line (mirrors `collect_paragraph`'s break condition).
    Paragraph,
    /// Inside `{{code ...}} ... {{/code}}`; ends at a line containing
    /// `{{/code}}`. Blank lines inside are part of the code body.
    CodeBlock,
    /// Inside `{{quote}} ... {{/quote}}`; ends at a line containing
    /// `{{/quote}}`.
    QuoteBlock,
    /// Inside `{{name ...}} ... {{/name}}` (any other block macro); ends at
    /// a line containing `{{/name}}`.
    MacroBlock(String),
    /// Accumulating contiguous `|`-prefixed table rows; ends at the first
    /// line that doesn't start with `|`.
    Table,
    /// Accumulating contiguous list-item lines sharing the same marker
    /// (`* ` or `1.`); ends at the first line that doesn't start with it.
    List(bool),
}

/// Chunked streaming XWiki parser that delivers events to a [`Handler`].
///
/// Memory: O(largest top-level block). Split tokens at chunk boundaries are
/// handled correctly — partial lines are buffered until the next `\n`, so a
/// multi-byte UTF-8 character or a `feed()` call landing mid-line never
/// produces incorrect output.
pub struct StreamingParser<H: Handler> {
    handler: H,
    line_buf: Vec<u8>,
    block_lines: Vec<String>,
    state: BlockState,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            state: BlockState::Between,
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
        match &self.state {
            BlockState::CodeBlock => {
                let ended = line.trim().contains("{{/code}}");
                self.block_lines.push(line);
                if ended {
                    self.emit_block();
                    self.state = BlockState::Between;
                }
                return;
            }
            BlockState::QuoteBlock => {
                let ended = line.trim().contains("{{/quote}}");
                self.block_lines.push(line);
                if ended {
                    self.emit_block();
                    self.state = BlockState::Between;
                }
                return;
            }
            BlockState::MacroBlock(name) => {
                let end_tag = format!("{{{{/{name}}}}}");
                let ended = line.trim().contains(&end_tag);
                self.block_lines.push(line);
                if ended {
                    self.emit_block();
                    self.state = BlockState::Between;
                }
                return;
            }
            BlockState::Table => {
                if line.trim().starts_with('|') {
                    self.block_lines.push(line);
                    return;
                }
                self.emit_block();
                self.state = BlockState::Between;
                // Fall through: this line starts the next block.
            }
            BlockState::List(ordered) => {
                let marker = if *ordered { "1." } else { "* " };
                if line.starts_with(marker) {
                    self.block_lines.push(line);
                    return;
                }
                self.emit_block();
                self.state = BlockState::Between;
                // Fall through: this line starts the next block.
            }
            BlockState::Paragraph => {
                if is_paragraph_boundary(&line) {
                    self.emit_block();
                    self.state = BlockState::Between;
                    if line.trim().is_empty() {
                        // Blank line: consumed as the paragraph separator,
                        // nothing more to do with it.
                        return;
                    }
                    // Fall through: this line starts the next block.
                } else {
                    self.block_lines.push(line);
                    return;
                }
            }
            BlockState::Between => {}
        }

        self.start_block(line);
    }

    /// Classify `line` as the start of a new top-level block (called only
    /// from [`BlockState::Between`], including immediately after a prior
    /// block's boundary line falls through). Mirrors the dispatch order in
    /// [`crate::parse::Parser::parse`] exactly.
    fn start_block(&mut self, line: String) {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        // Heading: = text = or == text == etc.
        if line.starts_with('=') && trimmed.ends_with('=') {
            self.block_lines.push(line);
            self.emit_block();
            return;
        }

        // Horizontal rule: ----
        if trimmed == "----" {
            self.block_lines.push(line);
            self.emit_block();
            return;
        }

        // Code block: {{code}}...{{/code}}
        if trimmed.starts_with("{{code") {
            self.block_lines.push(line);
            self.state = BlockState::CodeBlock;
            return;
        }

        // Block macros: {{quote}}, {{info}}, {{warning}}, etc.
        if let Some(macro_info) = try_parse_block_macro_start(&trimmed) {
            self.block_lines.push(line);
            self.state = if macro_info.name == "quote" {
                BlockState::QuoteBlock
            } else {
                BlockState::MacroBlock(macro_info.name)
            };
            return;
        }

        // Self-closing macros: {{toc/}}, {{include .../}}
        if try_parse_self_closing_macro(&trimmed).is_some() {
            self.block_lines.push(line);
            self.emit_block();
            return;
        }

        // Table
        if trimmed.starts_with('|') {
            self.block_lines.push(line);
            self.state = BlockState::Table;
            return;
        }

        // Lists: `* item` (asterisk + space) or `1. item`
        if line.starts_with("* ") {
            self.block_lines.push(line);
            self.state = BlockState::List(false);
            return;
        }
        if line.starts_with("1.") {
            self.block_lines.push(line);
            self.state = BlockState::List(true);
            return;
        }

        // Regular paragraph
        self.block_lines.push(line);
        self.state = BlockState::Paragraph;
    }

    /// Flush the current `block_lines` buffer (if any) by reparsing it in
    /// isolation and forwarding every resulting event to the handler.
    ///
    /// XWiki blocks carry no cross-block state (no reference-style link
    /// definitions or similar forward-referencing constructs), so reparsing
    /// one block at a time produces the same events a whole-document parse
    /// would produce for that block.
    fn emit_block(&mut self) {
        if self.block_lines.is_empty() {
            return;
        }
        let text = self.block_lines.join("\n");
        self.block_lines.clear();
        let (doc, _) = crate::parse::parse(&text);
        for event in crate::events::events(&doc) {
            self.handler.handle(event.into_owned());
        }
    }

    /// Flush any remaining input and deliver final events.
    pub fn finish(mut self) {
        if !self.line_buf.is_empty() {
            let bytes = std::mem::take(&mut self.line_buf);
            let line = String::from_utf8_lossy(&bytes).into_owned();
            self.feed_line(line);
        }
        self.emit_block();
    }
}

/// Mirrors `Parser::collect_paragraph`'s break condition: a line that starts
/// a different kind of block, or a blank line, ends paragraph accumulation.
fn is_paragraph_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty()
        || line.starts_with('=')
        || line.starts_with("* ")
        || line.starts_with("1.")
        || trimmed.starts_with('|')
        || trimmed.starts_with("{{code")
        || trimmed == "----"
        || try_parse_block_macro_start(trimmed).is_some()
        || try_parse_self_closing_macro(trimmed).is_some()
}

/// Chunk-driven XWiki parser that delivers events to a callback on finish.
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
        let s = String::from_utf8_lossy(&self.buf);
        let (doc, _) = crate::parse::parse(&s);
        for event in crate::events::events(&doc) {
            (self.callback)(event.into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"= Hello =\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"= Title =\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"= Hello =\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"= Title =\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { .. }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"= Hello =\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartParagraph))
        );
    }

    /// Direct incrementality probe: feeding half of a multi-block document
    /// (well past the first block's boundary) must deliver events to the
    /// handler *before* `finish()` is called — the old implementation
    /// buffered everything into a `Vec<u8>` and only parsed inside
    /// `finish()`, so this would fail with zero events delivered.
    #[test]
    fn test_streaming_parser_delivers_events_before_finish() {
        let input = b"= Title =\n\nFirst paragraph.\n\n== Sub ==\n\nSecond paragraph.\n";
        let mid = input.len() / 2;
        let mut delivered: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| delivered.push(ev));
        p.feed(&input[..mid]);
        assert!(
            !delivered.is_empty(),
            "expected at least one event delivered to the handler after feeding half the \
             input and before finish() — StreamingParser must parse block-by-block, not \
             buffer-then-finish"
        );
        // Intentionally dropped without calling finish(): this probe only
        // needs to observe pre-finish handler state.
    }

    fn bulk_events(input: &[u8]) -> Vec<OwnedEvent> {
        let s = String::from_utf8_lossy(input);
        let (doc, _) = crate::parse::parse(&s);
        crate::events::events(&doc)
            .map(|e| e.into_owned())
            .collect()
    }

    fn streamed_events(input: &[u8], chunk_size: usize) -> Vec<OwnedEvent> {
        let mut streamed = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        if chunk_size == 0 {
            p.feed(input);
        } else {
            for chunk in input.chunks(chunk_size) {
                p.feed(chunk);
            }
        }
        p.finish();
        streamed
    }

    const ADVERSARIAL_INPUT: &str = "\
= Title =

Intro paragraph with **bold** and //italic// text.

== Section ==

* item one
* item two
* item three

1. ordered one
1. ordered two

{{code language=\"rust\"}}
fn main() {
    println!(\"hi\");
}
{{/code}}

{{quote}}
A quoted paragraph.

A second quoted paragraph after a blank line.
{{/quote}}

{{info}}
Some info text.

More info after a blank line.
{{/info}}

|=Name|=Age|
|Alice|30|
|Bob|40|

----

A [[label>>http://example.com]] link and a [[image:photo.png||alt=\"A photo\"]] image.

{{toc/}}

Final paragraph with a line\\\\ break.
";

    /// `StreamingParser` fed under an adversarial chunking (whole input,
    /// single-byte, small chunk sizes, and a mid-UTF-8-character split) must
    /// deliver exactly the same event sequence as `events()` over the whole
    /// input, across every top-level construct: headings, paragraphs,
    /// lists (ordered/unordered), a code block, a quote block (with an
    /// internal blank line), a generic macro block (with an internal blank
    /// line), a table, a horizontal rule, a link, an image, a self-closing
    /// macro, and a line break.
    #[test]
    fn test_streaming_matches_events_under_adversarial_chunking() {
        let input = ADVERSARIAL_INPUT.as_bytes();
        let bulk = bulk_events(input);
        assert!(!bulk.is_empty());

        for chunk_size in [0, 1, 3, 7, 16, 64] {
            let streamed = streamed_events(input, chunk_size);
            assert_eq!(
                bulk, streamed,
                "StreamingParser diverged from events() under chunk_size={chunk_size}"
            );
        }
    }

    #[test]
    fn test_streaming_split_mid_utf8_char() {
        // "café" — the 'é' is 2 UTF-8 bytes, split across separate feed()
        // calls when chunked one byte at a time.
        let input = "café society\n\nAnother caf\u{e9} paragraph.\n".as_bytes();
        assert_eq!(bulk_events(input), streamed_events(input, 1));
    }

    #[test]
    fn test_streaming_split_mid_directive_keyword() {
        // Splits land inside "{{code" and inside the "{{/code}}" end tag.
        let input = b"{{code language=\"rust\"}}\n\nlet x = 1;\nlet y = 2;\n{{/code}}\n";
        assert_eq!(bulk_events(input), streamed_events(input, 1));
    }

    #[test]
    fn test_streaming_split_mid_quote_with_blank_line() {
        // The quote body has an internal blank line; single-byte chunking
        // tears the "{{/quote}}" end tag apart too.
        let input = b"{{quote}}\nFirst.\n\nSecond after blank.\n{{/quote}}\n";
        assert_eq!(bulk_events(input), streamed_events(input, 1));
    }

    #[test]
    fn test_streaming_split_mid_table_and_list() {
        let input = b"|=A|=B|\n|1|2|\n\n* one\n* two\n\n1. first\n1. second\n";
        assert_eq!(bulk_events(input), streamed_events(input, 1));
    }

    // The instrumented global allocator lives in `crate::writer::alloc_guard`
    // — only one `#[global_allocator]` may exist per test binary, and all
    // `#[cfg(test)]` items across this crate's modules compile into the same
    // binary, so both writer.rs's and this module's memory-guard tests share
    // it and its thread-local `CURRENT`/`PEAK` counters.

    /// Peak-memory guard: feeding a large synthetic multi-block document
    /// through `StreamingParser` in small chunks must keep peak allocated
    /// bytes within a small constant multiple of a single top-level block's
    /// size, not grow with the full document. The old implementation
    /// buffered the entire input into a `Vec<u8>` and only parsed inside
    /// `finish()` — `O(full document)`.
    ///
    /// Measured as a **scaling comparison** (peak at 10x the section count
    /// vs peak at baseline), not an absolute byte threshold: the allocator
    /// is process-wide, so an absolute threshold is at the mercy of
    /// unrelated tests allocating concurrently on other threads. A true
    /// `O(full document)` parser would show peak growing proportionally
    /// with N (~10x); a bounded parser's peak stays flat regardless of N.
    #[test]
    fn test_streaming_parser_peak_memory_bounded() {
        use crate::writer::alloc_guard::{CURRENT, PEAK};

        fn synthetic_doc(n: usize) -> String {
            let mut s = String::new();
            for i in 0..n {
                s.push_str(&format!("== Section {i} ==\n\n"));
                s.push_str("word ".repeat(20).as_str());
                s.push_str(&format!("{i}\n\n"));
                s.push_str("* item a\n* item b\n\n");
            }
            s
        }

        // Peak-above-baseline for feeding `n` synthetic sections through
        // StreamingParser in small chunks.
        fn run(n: usize) -> usize {
            let doc = synthetic_doc(n);
            let baseline_current = CURRENT.with(|c| c.get());
            PEAK.with(|p| p.set(baseline_current));

            let mut p = StreamingParser::new(|ev: OwnedEvent| {
                std::hint::black_box(&ev);
            });
            for chunk in doc.as_bytes().chunks(64) {
                p.feed(chunk);
            }
            p.finish();

            PEAK.with(|p| p.get()).saturating_sub(baseline_current)
        }

        let small = run(500).max(1);
        let large = run(5000); // 10x the sections

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 6.0,
            "peak allocated bytes above baseline did not stay flat under 10x the sections: \
             {small} bytes @500 sections -> {large} bytes @5000 sections (ratio {ratio:.2}); \
             this suggests the parser is retaining O(full document) memory instead of \
             flushing each top-level block"
        );
    }
}
