//! Chunk-driven (batch) MultiMarkdown parser.
//!
//! # Limitation
//!
//! Like `commonmark_fmt::batch::StreamingParser`, this implementation
//! buffers all input until [`finish`](StreamingParser::finish) is called.
//! This is not a limitation multimarkdown-fmt introduces on top of an
//! otherwise-incremental base — it *inherits* commonmark-fmt's own
//! documented pulldown-cmark exemption (pulldown-cmark requires the full
//! input as `&str`; see `commonmark_fmt`'s crate docs and
//! `commonmark_fmt::batch`'s own "Limitation" section) for the CommonMark
//! body content, and multimarkdown-fmt's own metadata-block extraction adds
//! no further requirement to buffer beyond that — but since the CommonMark
//! layer beneath it already must, there is no incremental path available
//! here either. For large-file use cases, prefer loading the full input and
//! using [`crate::events::events`] directly (a true streaming iterator —
//! see its module docs for exactly how it stays incremental despite this
//! limitation).
//!
//! # Example
//!
//! ```no_run
//! use multimarkdown_fmt::batch::StreamingParser;
//! use multimarkdown_fmt::events::OwnedEvent;
//!
//! let mut collected = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| collected.push(ev));
//! p.feed(b"Title: Doc\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::events::OwnedEvent;

/// Handler closure or type for batch-mode MultiMarkdown events.
pub trait Handler {
    fn handle(&mut self, event: OwnedEvent);
}

impl<F: FnMut(OwnedEvent)> Handler for F {
    fn handle(&mut self, event: OwnedEvent) {
        self(event);
    }
}

/// Chunked streaming parser for MultiMarkdown. See the module docs for the
/// buffering limitation this inherits from commonmark-fmt.
pub struct StreamingParser<H: Handler> {
    buf: Vec<u8>,
    handler: H,
}

impl<H: Handler> StreamingParser<H> {
    pub fn new(handler: H) -> Self {
        StreamingParser {
            buf: Vec::new(),
            handler,
        }
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
    use crate::events::events_str;

    #[test]
    fn batch_parser_basic() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"Title: Doc\n\nA paragraph[p. 1][#Doe:2020].\n");
        p.finish();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Metadata { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Citation { .. })));
    }

    #[test]
    fn batch_matches_events_iter_split_mid_token() {
        let input = b"Title: Doc\n\nSee[p. 23][#Doe:2006] here.\n";
        let direct: Vec<OwnedEvent> = events_str(std::str::from_utf8(input).unwrap())
            .map(|e| e.into_owned())
            .collect();

        let mut batch: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| batch.push(ev));
        p.feed(&input[..15]);
        p.feed(&input[15..]);
        p.finish();

        assert_eq!(direct, batch);
    }

    #[test]
    fn batch_invalid_utf8_no_panic() {
        let mut called = false;
        let mut p = StreamingParser::new(|_ev: OwnedEvent| {
            called = true;
        });
        p.feed(b"\xff\xfe");
        p.finish();
        assert!(!called);
    }
}
