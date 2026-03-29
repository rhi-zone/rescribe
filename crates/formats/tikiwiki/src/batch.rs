//! Chunk-driven (batch) TikiWiki parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] processes input in logical blocks (content between blank
//! lines, or delimited by `{CODE}`/`{QUOTE}` markers). Memory usage is
//! O(largest block in the document).
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//!
//! # Example — AST style
//! ```no_run
//! use tikiwiki::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"! Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use tikiwiki::batch::{StreamingParser, Handler};
//! use tikiwiki::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"! Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, TikiwikiDoc};
use crate::events::OwnedEvent;

/// Chunk-driven TikiWiki parser that returns the full AST on finish.
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
    pub fn finish(self) -> (TikiwikiDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming TikiWiki events.
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
    /// Inside a `{CODE(...)}`…`{CODE}` or `{QUOTE(...)}`…`{QUOTE}` block.
    InPlugin { end_tag: String },
}

/// Chunked streaming TikiWiki parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block). See the [module-level docs](self) for details.
pub struct StreamingParser<H: Handler> {
    handler: H,
    /// Bytes of the current incomplete line (not yet terminated by `\n`).
    line_buf: Vec<u8>,
    /// Complete lines of the block currently being accumulated.
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
        // Inside a plugin block
        let is_end_of_plugin: Option<bool> =
            if let BlockState::InPlugin { ref end_tag } = self.state {
                Some(line.trim() == end_tag.as_str())
            } else {
                None
            };

        if let Some(is_end) = is_end_of_plugin {
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

        // Plugin block start
        let trimmed = line.trim();
        if trimmed.starts_with("{CODE") && trimmed.contains('}') {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InPlugin { end_tag: "{CODE}".to_string() };
            self.block_lines.push(line);
            return;
        }
        if trimmed.starts_with("{QUOTE") && trimmed.contains('}') {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InPlugin { end_tag: "{QUOTE}".to_string() };
            self.block_lines.push(line);
            return;
        }

        // Regular line
        self.state = BlockState::Accumulating;
        self.block_lines.push(line);
    }

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
        if !self.line_buf.is_empty() {
            if self.line_buf.last() == Some(&b'\r') {
                self.line_buf.pop();
            }
            let line = String::from_utf8_lossy(&self.line_buf).into_owned();
            self.feed_line(line);
        }
        self.emit_block();
    }
}

/// Chunk-driven TikiWiki parser that delivers events to a callback on finish.
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
        p.feed(b"! Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"! Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"! Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_code_block() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"{CODE()}\nlet x = 1;\n{CODE}\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"! Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }
}
