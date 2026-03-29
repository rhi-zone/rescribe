//! Chunk-driven (batch) Creole parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Example — AST style
//! ```no_run
//! use creole::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"= Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use creole::batch::{StreamingParser, Handler};
//! use creole::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"= Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{CreoleDoc, Diagnostic};
use crate::events::OwnedEvent;

/// Chunk-driven Creole parser that returns the full AST on finish.
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
    pub fn finish(self) -> (CreoleDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Creole events.
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

/// Chunked streaming Creole parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block). Accumulates lines between blank lines, then
/// parses and emits events for each block.
pub struct StreamingParser<H: Handler> {
    handler: H,
    /// Bytes of the current incomplete line (not yet terminated by `\n`).
    line_buf: Vec<u8>,
    /// Complete lines of the block currently being accumulated.
    block_lines: Vec<String>,
    /// Whether we are inside a `{{{`...`}}}` nowiki block.
    in_nowiki: bool,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            in_nowiki: false,
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
        // Inside nowiki block: accumulate until }}}
        if self.in_nowiki {
            let is_end = line.trim() == "}}}";
            self.block_lines.push(line);
            if is_end {
                self.in_nowiki = false;
                self.emit_block();
            }
            return;
        }

        // Blank line: end of current block
        if line.trim().is_empty() {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            return;
        }

        // Nowiki block start
        if line.trim_start().starts_with("{{{") && !line[line.find("{{{").unwrap() + 3..].contains("}}}") {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.in_nowiki = true;
            self.block_lines.push(line);
            return;
        }

        self.block_lines.push(line);
    }

    /// Parse accumulated block lines and deliver events to handler.
    fn emit_block(&mut self) {
        if self.block_lines.is_empty() {
            return;
        }
        let text = self.block_lines.join("\n");
        self.block_lines.clear();
        for event in crate::events::events(&text) {
            self.handler.handle(event);
        }
    }

    /// Flush any remaining input and deliver final events.
    pub fn finish(mut self) {
        // Flush any bytes that did not end with \n
        if !self.line_buf.is_empty() {
            if self.line_buf.last() == Some(&b'\r') {
                self.line_buf.pop();
            }
            let line = String::from_utf8_lossy(&self.line_buf).into_owned();
            self.feed_line(line);
        }
        // Flush any pending block
        self.emit_block();
    }
}

/// Chunk-driven Creole parser that delivers events to a callback on finish.
pub struct BatchSink<F: FnMut(OwnedEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(OwnedEvent)> BatchSink<F> {
    pub fn new(callback: F) -> Self {
        BatchSink { buf: Vec::new(), callback }
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
        p.feed(b"= Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"= Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"= Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"= Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_code_block() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"{{{\nlet x = 1;\n}}}\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"= Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s).collect()
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
        sink.feed(b"= Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }
}
