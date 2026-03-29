//! Chunk-driven (batch) DokuWiki parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.

use crate::ast::{Diagnostic, DokuwikiDoc};
use crate::events::OwnedEvent;

/// Chunk-driven DokuWiki parser that returns the full AST on finish.
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
    pub fn finish(self) -> (DokuwikiDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming DokuWiki events.
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
    /// Between blocks -- waiting for the first non-blank line.
    Between,
    /// Accumulating normal content.
    Accumulating,
    /// Inside a `<code>`/`<file>`/`<html>`/`<php>` block.
    InSpecialBlock { end: String },
}

/// Chunked streaming DokuWiki parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block).
pub struct StreamingParser<H: Handler> {
    handler: H,
    line_buf: Vec<u8>,
    block_lines: Vec<String>,
    state: BlockState,
}

impl<H: Handler> StreamingParser<H> {
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
        // Check if we're inside a special block
        let is_end_of_special: Option<bool> =
            if let BlockState::InSpecialBlock { ref end } = self.state {
                Some(line.trim() == end.as_str())
            } else {
                None
            };

        if let Some(is_end) = is_end_of_special {
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

        // Special block start: <code>, <file>, <html>, <php>
        let trimmed = line.trim();
        if let Some(end_tag) = detect_special_block_start(trimmed) {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InSpecialBlock { end: end_tag };
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
            self.handler.handle(event);
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

/// Detect if a line starts a special block and return the expected end tag.
fn detect_special_block_start(trimmed: &str) -> Option<String> {
    for tag in &["code", "file", "html", "php"] {
        let prefix = format!("<{}", tag);
        if trimmed.starts_with(&prefix) {
            let rest = &trimmed[prefix.len()..];
            if rest.is_empty() || rest.starts_with('>') || rest.starts_with(' ') {
                return Some(format!("</{}>", tag));
            }
        }
    }
    None
}

/// Chunk-driven DokuWiki parser that delivers events to a callback on finish.
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
            (self.callback)(event);
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
        p.feed(b"====== Hello ======\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev: OwnedEvent| evs.push(ev));
        p.feed(b"====== Hello ======\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_code_block() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"<code rust>\nlet x = 1;\n\nlet y = 2;\n</code>\n");
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
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev: OwnedEvent| events.push(ev));
        sink.feed(b"====== Hello ======\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }
}
