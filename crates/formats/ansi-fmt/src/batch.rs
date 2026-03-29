//! Chunk-driven (batch) ANSI parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! ANSI escape sequences are self-contained within a few bytes, so the
//! streaming parser only needs to buffer an incomplete escape sequence at
//! chunk boundaries.  Memory usage is O(longest escape sequence), which is
//! effectively O(1) for well-formed input.
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//!
//! # Example — AST style
//! ```no_run
//! use ansi_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"\x1b[1mHello\x1b[0m");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use ansi_fmt::batch::{StreamingParser, Handler};
//! use ansi_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"\x1b[1mHello\x1b[0m");
//! p.finish();
//! ```

use crate::ast::{AnsiDoc, Diagnostic};
use crate::events::OwnedEvent;

/// Chunk-driven ANSI parser that returns the full AST on finish.
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
    pub fn finish(self) -> (AnsiDoc, Vec<Diagnostic>) {
        crate::parse::parse(&self.buf)
    }
}

/// Handler trait for streaming ANSI events.
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

/// Chunked streaming ANSI parser that delivers events to a [`Handler`].
///
/// Memory: O(largest escape sequence) for well-formed input.
pub struct StreamingParser<H: Handler> {
    handler: H,
    buf: Vec<u8>,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            buf: Vec::new(),
        }
    }

    /// Feed a chunk of bytes.  May call `handler.handle()` zero or more times.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
        self.drain_complete();
    }

    /// Try to emit events for all complete sequences in `self.buf`.
    fn drain_complete(&mut self) {
        // Find the last position that might be the start of an incomplete
        // escape sequence.  Everything before it is safe to parse.
        let safe_end = find_safe_boundary(&self.buf);
        if safe_end == 0 {
            return;
        }

        let to_parse: Vec<u8> = self.buf.drain(..safe_end).collect();
        for event in crate::events::events(&to_parse) {
            self.handler.handle(event.into_owned());
        }
    }

    /// Flush any remaining input and deliver final events.
    pub fn finish(mut self) {
        if !self.buf.is_empty() {
            let remaining = std::mem::take(&mut self.buf);
            for event in crate::events::events(&remaining) {
                self.handler.handle(event.into_owned());
            }
        }
    }
}

/// Find the byte offset up to which the buffer can be safely parsed.
/// Returns 0 if the entire buffer might be an incomplete escape sequence.
fn find_safe_boundary(buf: &[u8]) -> usize {
    if buf.is_empty() {
        return 0;
    }

    // If the last byte is ESC, it might be the start of an escape sequence.
    // Walk backwards to find the last ESC.
    let mut i = buf.len();
    while i > 0 {
        i -= 1;
        if buf[i] == 0x1b {
            // Check if this ESC starts a potentially incomplete sequence.
            // If there are enough bytes after it to form a complete sequence,
            // include it.  Otherwise, this is the boundary.
            if is_complete_escape(&buf[i..]) {
                return buf.len();
            } else {
                return i;
            }
        }
    }

    // No ESC found — everything is safe.
    buf.len()
}

/// Check if an escape sequence starting at `data[0]` is complete.
fn is_complete_escape(data: &[u8]) -> bool {
    if data.is_empty() || data[0] != 0x1b {
        return true;
    }
    if data.len() < 2 {
        return false;
    }
    match data[1] {
        b'[' => {
            // CSI: need digits/semicolons then an alpha terminator.
            let mut j = 2;
            // Skip '?' prefix.
            if j < data.len() && data[j] == b'?' {
                j += 1;
            }
            while j < data.len()
                && (data[j].is_ascii_digit() || data[j] == b';' || data[j] == b':')
            {
                j += 1;
            }
            // Need a terminator byte.
            j < data.len() && data[j].is_ascii_alphabetic()
        }
        b']' => {
            // OSC: need BEL or ST terminator.
            for j in 2..data.len() {
                if data[j] == 0x07 {
                    return true;
                }
                if data[j] == 0x1b && j + 1 < data.len() && data[j + 1] == b'\\' {
                    return true;
                }
            }
            false
        }
        b'(' | b')' => data.len() >= 3,
        b'7' | b'8' => true,
        _ => true, // Unknown — treat as complete.
    }
}

/// Chunk-driven ANSI parser that delivers events to a callback on finish.
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
        for event in crate::events::events(&self.buf) {
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
        p.feed(b"\x1b[1mHello\x1b[0m");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert!(!doc.nodes.is_empty());
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"\x1b[1mHello\x1b[0m" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert!(!doc.nodes.is_empty());
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"\x1b[1mHello\x1b[0m");
        p.finish();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::Text { text, .. } if text == "Hello")));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"\x1b[1mHello\x1b[0m" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        // When fed byte-by-byte, text may arrive in multiple fragments.
        // Concatenate all text events and check the result.
        let text: String = evs
            .iter()
            .filter_map(|e| match e {
                OwnedEvent::Text { text, .. } => Some(text.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"\x1b[1mHello\x1b[0m");
        sink.finish();
        assert!(events
            .iter()
            .any(|e| matches!(e, OwnedEvent::Text { text, .. } if text == "Hello")));
    }
}
