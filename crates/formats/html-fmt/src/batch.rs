//! Chunk-driven (batch) HTML parser.
//!
//! # Streaming
//!
//! `StreamingParser` is genuinely incremental: `feed()` drives html5ever's
//! tokenizer/tree-builder through a custom `TreeSink`
//! ([`crate::sink::IncrementalSink`]) that calls the handler as soon as
//! each event is available — not after a full tree is built. See the
//! [`crate::events`] module docs for the correction-event design this uses
//! to handle HTML5's retroactive tree-construction operations (adoption
//! agency, foster parenting) without buffering the whole document.
//!
//! `BatchParser` (the chunked-input, full-`HtmlDoc`-at-`finish()` reader)
//! still buffers raw input bytes and parses once at `finish()` — that is
//! inherent to returning a *complete* `HtmlDoc`, not a streaming
//! limitation: producing a full materialized AST always needs the whole
//! input, in any format.
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

/// Handler trait for streaming HTML events — the shared
/// [`rescribe_format_api::Handler`], not a locally declared trait.
pub use rescribe_format_api::Handler;

/// Chunked streaming HTML parser that delivers events to a [`Handler`] as
/// they are produced.
///
/// Input is accepted in chunks via [`feed()`](StreamingParser::feed), and
/// each `feed()` call drives html5ever's tokenizer/tree-builder over that
/// chunk immediately, dispatching every event the chunk makes available to
/// the handler before returning — genuinely incremental, not buffered
/// until [`finish()`](StreamingParser::finish). The handler may also
/// receive correction events (`NodeReparented`, `ChildrenReparented`,
/// `NodeDetached`) — see the [`crate::events`] module docs.
pub struct StreamingParser<H: Handler<OwnedEvent>> {
    handler: H,
    decoder: html5ever::tendril::stream::Utf8LossyDecoder<
        html5ever::driver::Parser<crate::sink::IncrementalSinkHandle>,
    >,
    // Events land here via the sink's callback, then get drained into the
    // handler after each `feed()` call.
    queue: std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<OwnedEvent>>>,
}

impl<H: Handler<OwnedEvent>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        let queue: std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<OwnedEvent>>> =
            std::rc::Rc::default();
        let (decoder, queue) = crate::sink::new_streaming_decoder(queue);
        StreamingParser {
            handler,
            decoder,
            queue,
        }
    }

    fn drain(&mut self) {
        loop {
            let ev = self.queue.borrow_mut().pop_front();
            match ev {
                Some(ev) => self.handler.handle(ev),
                None => break,
            }
        }
    }

    /// Feed a chunk of bytes. Drives the parser over `chunk` immediately
    /// and dispatches every resulting event to the handler before
    /// returning.
    pub fn feed(&mut self, chunk: &[u8]) {
        use html5ever::tendril::SliceExt;
        use html5ever::tendril::stream::TendrilSink;
        self.decoder.process(chunk.to_tendril());
        self.drain();
    }

    /// Finish parsing: flush the tokenizer/tree-builder's end-of-input
    /// handling (which closes every still-open element) and dispatch the
    /// resulting events to the handler.
    pub fn finish(self) {
        use html5ever::tendril::stream::TendrilSink;
        let StreamingParser {
            mut handler,
            decoder,
            queue,
        } = self;
        decoder.finish();
        loop {
            let ev = queue.borrow_mut().pop_front();
            match ev {
                Some(ev) => handler.handle(ev),
                None => break,
            }
        }
    }
}
