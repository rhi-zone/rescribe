//! Chunk-driven (batch) HTML parser.
//!
//! # Streaming limitation
//!
//! The HTML5 parsing algorithm requires tree construction for correctness —
//! the spec mandates operations like foster parenting, implied element
//! insertion, and adoption agency that can rearrange previously-seen nodes.
//! This means truly incremental event delivery (events emitted during
//! `feed()`) is not possible without building the full tree first.
//!
//! `StreamingParser` accepts chunks via `feed()` to avoid requiring the
//! full raw input in memory simultaneously, but events are delivered in
//! a batch at `finish()` after the tree is finalized. This matches
//! html5ever's own architecture (the tree builder needs the full parse to
//! produce the final DOM).
//!
//! # Example — AST style
//! ```no_run
//! use html_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"<h1>Hello</h1>");
//! p.feed(b"<p>World</p>");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use html_fmt::batch::{StreamingParser, Handler};
//! use html_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"<h1>Hello</h1>");
//! p.feed(b"<p>World</p>");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, HtmlDoc};
use crate::events::OwnedEvent;

/// Chunk-driven HTML parser that returns the full AST on finish.
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
    pub fn finish(self) -> (HtmlDoc, Vec<Diagnostic>) {
        crate::parse::parse(&self.buf)
    }
}

/// Handler trait for streaming HTML events.
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

/// Chunked streaming HTML parser that delivers events to a [`Handler`].
///
/// Input is accepted in chunks via [`feed()`](StreamingParser::feed).
/// Events are delivered to the handler at [`finish()`](StreamingParser::finish)
/// after the full parse tree is constructed.
///
/// See the [module docs](self) for why incremental event delivery is not
/// possible for HTML5.
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

    /// Feed a chunk of bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish parsing and deliver all events to the handler.
    pub fn finish(mut self) {
        let (doc, _) = crate::parse::parse(&self.buf);
        for event in crate::events::events_from_doc(&doc) {
            self.handler.handle(event);
        }
    }
}
