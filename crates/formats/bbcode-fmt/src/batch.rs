//! Chunk-driven (batch) BBCode parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] accumulates blocks separated by blank lines or BBCode
//! block tags. Memory usage is O(largest block in the document).
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//! Use it when you need the complete AST.
//!
//! # Example — AST style
//! ```no_run
//! use bbcode_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"[b]Hello[/b]\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use bbcode_fmt::batch::{StreamingParser, Handler};
//! use bbcode_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"[b]Hello[/b]\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{BbcodeDoc, Diagnostic};
use crate::events::OwnedEvent;

/// Chunk-driven BBCode parser that returns the full AST on finish.
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
    pub fn finish(self) -> (BbcodeDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming BBCode events.
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
    /// Accumulating normal content (paragraph, etc.).
    /// Ends on the next blank line.
    Accumulating,
    /// Inside a block tag (`[code]`, `[quote]`, etc.) that must be closed.
    /// `close_tag` is the lowercase closing tag (e.g. `[/code]`).
    InBlock { close_tag: String },
}

/// Chunked streaming BBCode parser that delivers events to a [`Handler`].
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
                // Strip trailing \r (Windows line endings)
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
        // ── Inside a block tag ──────────────────────────────────────────
        let is_end_of_block: Option<bool> =
            if let BlockState::InBlock { ref close_tag } = self.state {
                Some(line.to_lowercase().contains(close_tag.as_str()))
            } else {
                None
            };

        if let Some(is_end) = is_end_of_block {
            self.block_lines.push(line);
            if is_end {
                self.emit_block();
                self.state = BlockState::Between;
            }
            return;
        }

        // ── Blank line: end of current block ─────────────────────────────
        if line.trim().is_empty() {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::Between;
            return;
        }

        // ── Block tag start ─────────────────────────────────────────────
        let lower_trimmed = line.trim().to_lowercase();
        let block_start = detect_block_tag(&lower_trimmed);
        if let Some(close_tag) = block_start {
            if !self.block_lines.is_empty() {
                self.emit_block();
            }
            self.state = BlockState::InBlock { close_tag };
            self.block_lines.push(line);
            return;
        }

        // ── Regular line ─────────────────────────────────────────────────
        self.state = BlockState::Accumulating;
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

/// Detect if a trimmed lowercase line starts a block-level BBCode tag.
/// Returns the closing tag if so.
fn detect_block_tag(lower: &str) -> Option<String> {
    static BLOCK_TAGS: &[(&str, &str)] = &[
        ("[code]", "[/code]"),
        ("[code=", "[/code]"),
        ("[pre]", "[/pre]"),
        ("[quote", "[/quote]"),
        ("[list", "[/list]"),
        ("[table]", "[/table]"),
        ("[spoiler]", "[/spoiler]"),
        ("[indent]", "[/indent]"),
        ("[center]", "[/center]"),
        ("[left]", "[/left]"),
        ("[right]", "[/right]"),
    ];

    for &(open, close) in BLOCK_TAGS {
        if lower.starts_with(open) {
            // If the close tag is already on this line, don't enter InBlock state
            if lower.contains(close) {
                return None;
            }
            return Some(close.to_string());
        }
    }
    None
}

/// Chunk-driven BBCode parser that delivers events to a callback on finish.
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
        p.feed(b"[b]Hello[/b]\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"[b]bold[/b]\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev: OwnedEvent| evs.push(ev));
        p.feed(b"[b]Hello[/b]\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"[b]bold[/b]\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_code_block() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"[code]\nlet x = 1;\n[/code]\n");
        p.finish();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"[b]Bold text[/b]\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s)
                .map(|e| e.into_owned())
                .collect()
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
        let mut sink = BatchSink::new(|ev: OwnedEvent| events.push(ev));
        sink.feed(b"[b]Hello[/b]\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(events.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }
}
