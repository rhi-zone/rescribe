//! Chunk-driven (batch) txt2tags parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] processes input in logical blocks (content between blank
//! lines, or delimited by fenced markers). Memory usage is O(largest block),
//! which for typical documents means O(longest paragraph or code block).
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//! Use it when you need the complete AST.
//!
//! # Example — AST style
//! ```no_run
//! use t2t::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"= Hello =\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use t2t::batch::{StreamingParser, Handler};
//! use t2t::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"= Hello =\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, T2tDoc};
use crate::events::OwnedEvent;

/// Chunk-driven txt2tags parser that returns the full AST on finish.
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
    pub fn finish(self) -> (T2tDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming txt2tags events.
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

/// Block accumulation state for the streaming parser.
enum BlockState {
    /// Between blocks — waiting for the first non-blank line.
    Between,
    /// Accumulating normal content.
    Accumulating,
    /// Inside a fenced block (``` or """).
    InFenced { end_marker: &'static str },
}

/// Chunked streaming txt2tags parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block). See the [module-level docs](self) for details.
pub struct StreamingParser<H: Handler> {
    handler: H,
    /// Bytes of the current incomplete line (not yet terminated by `\n`).
    line_buf: Vec<u8>,
    /// Complete lines of the block currently being accumulated.
    block_lines: Vec<String>,
    state: BlockState,
    /// True until the first block has been emitted. The txt2tags document
    /// header (title/author/date) can only ever be the first block of the
    /// whole stream, so header detection is only attempted once.
    at_document_start: bool,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    ///
    /// Dispatches `StartDocument` immediately: bulk `events()` always wraps
    /// the whole document in exactly one `StartDocument`/`EndDocument` pair
    /// (even for empty input), and `StreamingParser` owns that single pair
    /// itself now rather than forwarding one per re-parsed block (see
    /// `emit_block`'s doc comment).
    pub fn new(handler: H) -> Self {
        let mut parser = StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            state: BlockState::Between,
            at_document_start: true,
        };
        parser.handler.handle(OwnedEvent::StartDocument);
        parser
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
        // Inside a fenced block
        let is_end_of_fenced: Option<bool> =
            if let BlockState::InFenced { end_marker } = &self.state {
                Some(line.trim() == *end_marker)
            } else {
                None
            };

        if let Some(is_end) = is_end_of_fenced {
            self.block_lines.push(line);
            if is_end {
                self.emit_block();
                self.state = BlockState::Between;
            }
            return;
        }

        // Blank line: end of current block
        if line.trim().is_empty() {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::Between;
            return;
        }

        // Fenced block start
        let trimmed = line.trim();
        if trimmed == "```" {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InFenced { end_marker: "```" };
            self.block_lines.push(line);
            return;
        }
        if trimmed == "\"\"\"" {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InFenced {
                end_marker: "\"\"\"",
            };
            self.block_lines.push(line);
            return;
        }

        // Regular line
        self.state = BlockState::Accumulating;
        self.block_lines.push(line);
    }

    /// Parse the accumulated block lines and deliver events to the handler.
    ///
    /// Each accumulated block is re-parsed in isolation via
    /// `crate::events::events()`, which always wraps its output in its own
    /// `StartDocument`/`EndDocument` pair. `StreamingParser` owns exactly
    /// one such pair for the whole stream (`StartDocument` dispatched in
    /// `new()`, `EndDocument` in `finish()`), so every re-parsed block's own
    /// wrapper pair is filtered out here before forwarding the rest.
    fn emit_block(&mut self) {
        if self.block_lines.is_empty() {
            return;
        }
        let text = self.block_lines.join("\n");
        self.block_lines.clear();

        let was_at_document_start = self.at_document_start;
        self.at_document_start = false;

        if was_at_document_start && self.try_emit_header(&text) {
            return;
        }

        for event in crate::events::events(&text) {
            if matches!(
                event,
                crate::events::Event::StartDocument | crate::events::Event::EndDocument
            ) {
                continue;
            }
            self.handler.handle(event.into_owned());
        }
    }

    /// If `text` (the first accumulated block of the stream) is a txt2tags
    /// document header, deliver an `Event::Header` for it — plus events for
    /// any trailing lines beyond the 3-line header, in the rare case the
    /// header isn't immediately followed by a blank line — and return
    /// `true`. Returns `false` (delivering nothing) if `text` is not a
    /// header, so the caller falls back to parsing it as a normal block.
    ///
    /// This mirrors `crate::parse::Parser::try_parse_header` directly rather
    /// than routing through `crate::events::events()`, which would re-run
    /// full document parsing on the isolated block and spuriously wrap it in
    /// its own `StartDocument`/`EndDocument` pair.
    fn try_emit_header(&mut self, text: &str) -> bool {
        let mut p = crate::parse::Parser::new(text);
        let (title, author, date) = p.try_parse_header();
        if title.is_none() {
            return false;
        }
        self.handler.handle(OwnedEvent::Header {
            title,
            author,
            date,
        });
        // try_parse_header() only ever consumes exactly 3 lines (p.pos == 3
        // here). Anything after that in this block is body content that
        // normally would have been split off by a blank line; handle it in
        // case the caller didn't include one.
        let remaining = &p.lines[p.pos..];
        if remaining.iter().any(|line| !line.trim().is_empty()) {
            let remaining_text = remaining.join("\n");
            for event in crate::events::events(&remaining_text) {
                if matches!(
                    event,
                    crate::events::Event::StartDocument | crate::events::Event::EndDocument
                ) {
                    continue;
                }
                self.handler.handle(event.into_owned());
            }
        }
        true
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
        self.emit_block();
        self.handler.handle(OwnedEvent::EndDocument);
    }
}

/// Chunk-driven parser that delivers events to a callback on finish.
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
        for event in crate::events::events(&s) {
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
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_code_block() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"```\nlet x = 1;\n```\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::CodeBlock { .. }))
        );
    }

    #[test]
    fn test_streaming_parser_code_block_with_blank_lines() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"```\nline 1\n\nline 2\n```\n");
        p.finish();
        let code_blocks: Vec<_> = evs
            .iter()
            .filter(|e| matches!(e, OwnedEvent::CodeBlock { .. }))
            .collect();
        assert_eq!(
            code_blocks.len(),
            1,
            "should be exactly one code block event"
        );
        if let OwnedEvent::CodeBlock { content, .. } = &code_blocks[0] {
            assert!(content.contains("line 1"));
            assert!(content.contains("line 2"));
        }
    }

    #[test]
    fn test_streaming_parser_document_header() {
        let input = "My Document Title\nJohn Doe\n2024-01-15\n\nThis is the body text.\n";
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|e: OwnedEvent| evs.push(e));
        p.feed(input.as_bytes());
        p.finish();
        assert_eq!(
            evs.first(),
            Some(&OwnedEvent::StartDocument),
            "StreamingParser owns exactly one StartDocument for the whole stream"
        );
        assert_eq!(
            evs.get(1),
            Some(&OwnedEvent::Header {
                title: Some("My Document Title".to_string()),
                author: Some("John Doe".to_string()),
                date: Some("2024-01-15".to_string()),
            }),
            "header should be recognized directly, not lost to an isolated re-parse"
        );
        assert!(
            evs.iter()
                .filter(|e| matches!(e, OwnedEvent::Header { .. }))
                .count()
                == 1,
            "header should be emitted exactly once"
        );
    }

    fn bulk_events(input: &[u8]) -> Vec<OwnedEvent> {
        let s = String::from_utf8_lossy(input);
        crate::events::events(&s).map(|e| e.into_owned()).collect()
    }

    fn streamed_events_chunked(input: &[u8], chunk_size: usize) -> Vec<OwnedEvent> {
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

    /// Construct mix covering a document header, multiple headings,
    /// paragraphs, a code block, a list, a table, a definition list, and a
    /// horizontal rule — several top-level blocks, each separated by a
    /// blank line, so `StreamingParser`'s block-accumulation boundaries
    /// (`feed_line`'s blank-line/fence branches) fire repeatedly across the
    /// stream. Regression input for the "StreamingParser emitted one
    /// StartDocument/EndDocument pair per accumulated block instead of one
    /// for the whole document" bug.
    fn adversarial_input() -> &'static [u8] {
        b"Streaming Test\n\
          Jane Doe\n\
          2024-01-01\n\
          \n\
          = Heading one =\n\
          \n\
          A paragraph with **bold** and //em// text.\n\
          \n\
          == Sub heading ==\n\
          \n\
          ```\n\
          code line one\n\
          code line two\n\
          ```\n\
          \n\
          - item one\n\
          - item two\n\
          \n\
          || Name | Age |\n\
          | Alice | 30 |\n\
          \n\
          : Term\n\
          Definition text\n\
          \n\
          --------------------\n\
          \n\
          After the rule.\n\
          "
    }

    #[test]
    fn test_streaming_matches_events_adversarial_whole() {
        let input = adversarial_input();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 0));
    }

    #[test]
    fn test_streaming_matches_events_adversarial_single_byte() {
        let input = adversarial_input();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 1));
    }

    #[test]
    fn test_streaming_matches_events_adversarial_chunks_of_7() {
        let input = adversarial_input();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 7));
    }

    #[test]
    fn test_streaming_matches_events_adversarial_chunks_of_37() {
        let input = adversarial_input();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 37));
    }

    #[test]
    fn test_streaming_matches_events_single_start_end_document() {
        let input = adversarial_input();
        let streamed = streamed_events_chunked(input, 5);
        assert_eq!(
            streamed
                .iter()
                .filter(|e| **e == OwnedEvent::StartDocument)
                .count(),
            1,
            "StreamingParser must emit exactly one StartDocument for the whole document, not \
             one per accumulated block"
        );
        assert_eq!(
            streamed
                .iter()
                .filter(|e| **e == OwnedEvent::EndDocument)
                .count(),
            1,
            "StreamingParser must emit exactly one EndDocument for the whole document, not one \
             per accumulated block"
        );
        assert_eq!(streamed.first(), Some(&OwnedEvent::StartDocument));
        assert_eq!(streamed.last(), Some(&OwnedEvent::EndDocument));
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
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartParagraph))
        );
    }
}
