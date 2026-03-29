//! Chunk-driven (batch) Markua parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Example — AST style
//! ```no_run
//! use markua::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"# Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use markua::batch::{StreamingParser, Handler};
//! use markua::OwnedMarkuaEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedMarkuaEvent| events.push(ev));
//! p.feed(b"# Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, MarkuaDoc};
use crate::events::OwnedMarkuaEvent;

/// Chunk-driven Markua parser that returns the full AST on finish.
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
    pub fn finish(self) -> (MarkuaDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Markua events.
///
/// Implemented automatically for any `FnMut(OwnedMarkuaEvent)`.
pub trait Handler {
    fn handle(&mut self, event: OwnedMarkuaEvent);
}

impl<F: FnMut(OwnedMarkuaEvent)> Handler for F {
    fn handle(&mut self, event: OwnedMarkuaEvent) {
        self(event);
    }
}

/// Chunked streaming Markua parser that delivers events to a [`Handler`].
pub struct StreamingParser<H: Handler> {
    handler: H,
    /// Bytes of the current incomplete line (not yet terminated by `\n`).
    line_buf: Vec<u8>,
    /// Complete lines of the block currently being accumulated.
    block_lines: Vec<String>,
    /// Whether we are inside a fenced code block.
    in_code_block: bool,
    /// The fence string (e.g. "```" or "~~~") when inside a code block.
    code_fence: String,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            in_code_block: false,
            code_fence: String::new(),
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
        // Inside a code block: accumulate until closing fence
        if self.in_code_block {
            let trimmed = line.trim();
            if trimmed.starts_with(&self.code_fence)
                && trimmed
                    .chars()
                    .take_while(|&c| c == self.code_fence.chars().next().unwrap_or('`'))
                    .count()
                    >= self.code_fence.len()
            {
                self.block_lines.push(line);
                self.in_code_block = false;
                self.emit_block();
            } else {
                self.block_lines.push(line);
            }
            return;
        }

        // Check for code block start
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            let fence_char = trimmed.chars().next().unwrap_or('`');
            let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
            self.code_fence = fence_char.to_string().repeat(fence_len);
            self.in_code_block = true;
            self.block_lines.push(line);
            return;
        }

        // Blank line: end of current block
        if line.trim().is_empty() {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            return;
        }

        // Regular line
        self.block_lines.push(line);
    }

    /// Parse the accumulated block lines and deliver events to the handler.
    fn emit_block(&mut self) {
        if self.block_lines.is_empty() {
            return;
        }
        let text = self.block_lines.join("\n");
        self.block_lines.clear();
        for event in crate::events::events(&text) {
            self.handler.handle(event.into_owned());
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

/// Chunk-driven Markua parser that delivers events to a callback on finish.
///
/// Prefer [`StreamingParser`] for new code; `BatchSink` is kept for
/// backwards compatibility.
pub struct BatchSink<F: FnMut(OwnedMarkuaEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(OwnedMarkuaEvent)> BatchSink<F> {
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
            (self.callback)(event.into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedMarkuaEvent;

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
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedMarkuaEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"# Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartHeading { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_code_block() {
        let mut evs: Vec<OwnedMarkuaEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"```rust\nlet x = 1;\n```\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedMarkuaEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_streaming_parser_code_block_with_blank_lines() {
        let mut evs: Vec<OwnedMarkuaEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"```rust\nlet x = 1;\n\nlet y = 2;\n```\n");
        p.finish();
        let code_blocks: Vec<_> = evs
            .iter()
            .filter(|e| matches!(e, OwnedMarkuaEvent::CodeBlock { .. }))
            .collect();
        assert_eq!(code_blocks.len(), 1, "should be exactly one code block event");
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"# Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedMarkuaEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s).map(|e| e.into_owned()).collect()
        };

        let mut streamed: Vec<OwnedMarkuaEvent> = Vec::new();
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
        assert!(events.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartHeading { level: 1 })));
        assert!(events.iter().any(|e| matches!(e, OwnedMarkuaEvent::StartParagraph)));
    }
}
