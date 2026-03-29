//! Chunk-driven (batch) MediaWiki parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Example -- AST style
//! ```no_run
//! use mediawiki_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"== Heading ==\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example -- event callback style
//! ```no_run
//! use mediawiki_fmt::batch::{StreamingParser, Handler};
//! use mediawiki_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"== Heading ==\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, MediawikiDoc};
use crate::events::OwnedEvent;

/// Chunk-driven MediaWiki parser that returns the full AST on finish.
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
    pub fn finish(self) -> (MediawikiDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming MediaWiki events.
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
    /// Inside a block-level HTML tag (blockquote, pre, syntaxhighlight, etc.).
    InHtmlBlock { end_tag: String },
    /// Inside a wiki table `{| ... |}`.
    InTable,
}

/// Chunked streaming MediaWiki parser that delivers events to a [`Handler`].
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
        // Inside an HTML block tag
        let is_end_of_html: Option<bool> =
            if let BlockState::InHtmlBlock { ref end_tag } = self.state {
                Some(line.trim().to_lowercase().contains(&end_tag.to_lowercase()))
            } else {
                None
            };

        if let Some(is_end) = is_end_of_html {
            self.block_lines.push(line);
            if is_end {
                self.emit_block();
                self.state = BlockState::Between;
            }
            return;
        }

        // Inside a table
        if matches!(self.state, BlockState::InTable) {
            self.block_lines.push(line.clone());
            if line.trim() == "|}" {
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

        // Block-level HTML tag start
        let lower_trimmed = line.trim().to_lowercase();
        if lower_trimmed.starts_with("<blockquote") {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InHtmlBlock { end_tag: "</blockquote>".to_string() };
            self.block_lines.push(line);
            // Check if self-closing on same line
            if lower_trimmed.contains("</blockquote>") {
                self.emit_block();
                self.state = BlockState::Between;
            }
            return;
        }

        if lower_trimmed.starts_with("<pre") {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InHtmlBlock { end_tag: "</pre>".to_string() };
            self.block_lines.push(line);
            if lower_trimmed.contains("</pre>") {
                self.emit_block();
                self.state = BlockState::Between;
            }
            return;
        }

        if lower_trimmed.starts_with("<syntaxhighlight") || lower_trimmed.starts_with("<source") {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            let end_tag = if lower_trimmed.starts_with("<syntaxhighlight") {
                "</syntaxhighlight>"
            } else {
                "</source>"
            };
            self.state = BlockState::InHtmlBlock { end_tag: end_tag.to_string() };
            self.block_lines.push(line);
            if lower_trimmed.contains(end_tag) {
                self.emit_block();
                self.state = BlockState::Between;
            }
            return;
        }

        // Table start
        if line.trim().starts_with("{|") {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InTable;
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

/// Chunk-driven parser that delivers events to a callback on finish.
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
        p.feed(b"== Hello ==\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"== Title ==\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"== Hello ==\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 2 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"== Title ==\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_table() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"{|\n! Header\n|-\n| Cell\n|}\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTable { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTable)));
    }

    #[test]
    fn test_streaming_parser_blockquote() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"<blockquote>\nQuoted text.\n</blockquote>\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBlockquote)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBlockquote)));
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"== Hello ==\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartHeading { level: 2 })));
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }
}
