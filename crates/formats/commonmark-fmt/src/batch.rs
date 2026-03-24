//! Chunk-driven (batch) CommonMark parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Limitation
//!
//! This implementation buffers all input until [`finish`](StreamingParser::finish)
//! is called, then parses the complete input with pulldown-cmark. True
//! incremental chunked streaming is not possible without a native CommonMark
//! parser. For large-file use cases, prefer loading the full input and using
//! [`events`](crate::events::events) directly.
//!
//! # Example
//!
//! ```no_run
//! use commonmark_fmt::batch::StreamingParser;
//! use commonmark_fmt::events::OwnedEvent;
//!
//! let mut collected = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| collected.push(ev));
//! p.feed(b"# Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::events::OwnedEvent;

/// Handler closure or type for batch-mode CommonMark events.
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

/// Chunked streaming parser for CommonMark.
///
/// # Limitation
///
/// This implementation buffers all input until [`finish`](StreamingParser::finish)
/// is called, then parses the complete input with pulldown-cmark. True
/// incremental chunked streaming is not possible without a native CommonMark
/// parser. For large-file use cases, prefer loading the full input and using
/// [`events`](crate::events::events) instead.
pub struct StreamingParser<H: Handler> {
    buf: Vec<u8>,
    handler: H,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser { buf: Vec::new(), handler }
    }

    /// Append a chunk of bytes to the internal buffer.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Parse all buffered input and deliver events to the handler.
    ///
    /// If the buffer is not valid UTF-8, no events are delivered (the input
    /// is silently skipped, consistent with [`events`](crate::events::events)
    /// returning `None`).
    pub fn finish(mut self) {
        if let Some(iter) = crate::events::events(&self.buf) {
            for event in iter {
                self.handler.handle(event.into_owned());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{events_str, OwnedEvent};

    #[test]
    fn test_batch_parser_basic() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"# Hello\n\nA paragraph.\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"# Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_batch_matches_events_iter() {
        let input = b"# Hello\n\nA paragraph.\n";

        let direct: Vec<OwnedEvent> =
            events_str(std::str::from_utf8(input).unwrap()).map(|e| e.into_owned()).collect();

        let mut batch: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| batch.push(ev));
        p.feed(&input[..9]);
        p.feed(&input[9..]);
        p.finish();

        assert_eq!(direct, batch);
    }

    #[test]
    fn test_batch_invalid_utf8_no_panic() {
        // Should not panic; handler never called.
        let mut called = false;
        let mut p = StreamingParser::new(|_ev: OwnedEvent| { called = true; });
        p.feed(b"\xff\xfe");
        p.finish();
        assert!(!called);
    }
}
