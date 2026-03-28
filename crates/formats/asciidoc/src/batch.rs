//! Chunk-driven (batch) AsciiDoc parser.
//!
//! # Memory model
//!
//! [`StreamingParser`] processes input in logical blocks. Memory usage is
//! O(largest block), not O(full input). [`BatchParser`] buffers all input.
//!
//! # Example — AST style
//! ```no_run
//! use asciidoc::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"== Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use asciidoc::batch::{StreamingParser, Handler};
//! use asciidoc::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"== Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{AsciiDoc, Diagnostic};
use crate::events::OwnedEvent;

/// Chunk-driven AsciiDoc parser that returns the full AST on finish.
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
    pub fn finish(self) -> (AsciiDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming AsciiDoc events.
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
    Between,
    Accumulating,
    /// Inside a delimited block (e.g. `----`…`----`).
    /// The delimiter is stored to detect the closing line.
    InDelimitedBlock { delim: String },
}

/// Chunked streaming AsciiDoc parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block). Delimited blocks (`----`…`----`, `====`…`====`,
/// etc.) are buffered until the closing delimiter. All other content is
/// buffered until the next blank line.
pub struct StreamingParser<H: Handler> {
    handler: H,
    line_buf: Vec<u8>,
    block_lines: Vec<String>,
    state: BlockState,
    /// Whether `StartDocument` has been emitted.
    started: bool,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            state: BlockState::Between,
            started: false,
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
        let is_end_delim: Option<bool> =
            if let BlockState::InDelimitedBlock { ref delim } = self.state {
                Some(line.trim() == delim.as_str())
            } else {
                None
            };

        if let Some(is_end) = is_end_delim {
            self.block_lines.push(line);
            if is_end {
                self.emit_block();
                self.state = BlockState::Between;
            }
            return;
        }

        if line.trim().is_empty() {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::Between;
            return;
        }

        let trimmed = line.trim();
        // AsciiDoc delimited block markers: 4+ of the same delimiter character
        if is_delimited_block_marker(trimmed) {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            let delim = trimmed.to_owned();
            self.state = BlockState::InDelimitedBlock { delim };
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
        // AsciiDoc events() wraps each sub-document in StartDocument/EndDocument.
        // We emit those at the document level instead, stripping the per-block wrappers.
        for event in crate::events(&text) {
            let owned = event.into_owned();
            match owned {
                OwnedEvent::StartDocument => {
                    if !self.started {
                        self.started = true;
                        self.handler.handle(OwnedEvent::StartDocument);
                    }
                }
                OwnedEvent::EndDocument => {} // deferred to finish()
                ev => self.handler.handle(ev),
            }
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
        if self.started {
            self.handler.handle(OwnedEvent::EndDocument);
        }
    }
}

/// Returns true if `line` looks like an AsciiDoc delimited block marker.
/// Markers are 4+ consecutive identical delimiter characters: `-`, `=`, `.`, `_`, `*`, `+`, `/`, `|`.
fn is_delimited_block_marker(line: &str) -> bool {
    if line.len() < 4 {
        return false;
    }
    let ch = line.chars().next().unwrap();
    matches!(ch, '-' | '=' | '.' | '_' | '*' | '+' | '/' | '|')
        && line.chars().all(|c| c == ch)
}

/// Chunk-driven AsciiDoc parser that delivers events to a callback on finish.
///
/// Prefer [`StreamingParser`] for new code; `BatchSink` is kept for
/// backwards compatibility.
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
        p.feed(b"== Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"== Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"== Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 2, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"== Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { .. })));
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"== Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events(&s).map(|e| e.into_owned()).collect()
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
        sink.feed(b"== Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 2, .. })));
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
    }
}
