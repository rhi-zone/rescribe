//! Chunk-driven (batch) DocBook/XML parser with **true incremental**
//! event delivery.
//!
//! Unlike `html-fmt`'s `StreamingParser` (which must buffer all input
//! because HTML5 tree construction can rearrange already-seen nodes), XML
//! is well-nested by construction, so every markup token (`<tag>`, `</tag>`,
//! comments, CDATA, PIs, the XML declaration) is unambiguously complete or
//! incomplete on its own: `quick_xml` reports an `Err(Syntax(_))` for a
//! truncated tag/comment/CDATA/decl rather than silently treating it as
//! finished. `StreamingParser::feed` uses that to drain every event it can
//! prove is complete, dispatch it to the [`Handler`] immediately, and drop
//! the consumed prefix from its internal buffer — so buffered memory is
//! bounded by the largest *in-progress* token, not the whole document.
//!
//! The one non-obvious case is plain text: quick-xml terminates a `Text`
//! token at the next `<` *or* at end-of-input, and those two situations are
//! indistinguishable from the reader's return value alone (both look like
//! "reached the end successfully"). So a `Text` event is only dispatched
//! immediately when it was terminated by an actual `<` boundary (i.e. it
//! did not consume all currently-buffered bytes); a `Text` run that reaches
//! the end of the buffered bytes is held back until more input arrives (or
//! `finish()` confirms there is no more).
//!
//! # Example — AST style
//! ```
//! use docbook_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"<article><title>Hi</title>");
//! p.feed(b"<para>World</para></article>");
//! let (doc, diags) = p.finish();
//! assert!(diags.is_empty());
//! ```
//!
//! # Example — event callback style
//! ```
//! use docbook_fmt::batch::{StreamingParser, Handler};
//! use docbook_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"<article><para>Hello</para>");
//! p.feed(b"</article>");
//! p.finish();
//! assert!(!events.is_empty());
//! ```

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;
use xml_entities::{DtdEntities, EntityResolver};

use crate::ast::{Diagnostic, DocBookDoc, Span};
use crate::events::OwnedEvent;

/// Chunk-driven DocBook/XML parser that returns the full AST on finish.
///
/// Internally just accumulates bytes and calls [`crate::parse::parse`] at
/// `finish()` — for the AST-building use case there is no way to avoid
/// holding the whole document in memory anyway (the AST *is* the whole
/// document), so there is nothing to gain from incremental draining here.
/// Callers who need bounded memory should use [`StreamingParser`] instead.
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
    pub fn finish(self) -> (DocBookDoc, Vec<Diagnostic>) {
        crate::parse::parse(&self.buf)
    }
}

/// Handler trait for streaming DocBook/XML events.
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

/// Chunked streaming DocBook/XML parser that delivers events to a
/// [`Handler`] as soon as they are provably complete.
///
/// See the [module docs](self) for the incremental-draining strategy and
/// why plain text is the one token that must sometimes wait for more input.
pub struct StreamingParser<H: Handler> {
    handler: H,
    pending: Vec<u8>,
    diagnostics: Vec<Diagnostic>,
    /// Entities declared in this document's own DOCTYPE internal subset (if
    /// any), rebuilt whenever a `Doctype` event is drained. See
    /// `parse.rs`'s module docs for why a single forward pass is
    /// sufficient.
    entity_resolver: EntityResolver,
    /// Names of elements currently open, tracked by hand across `drain()`
    /// calls (same "persists across calls" shape as `entity_resolver`).
    /// Each `drain()` call builds a *fresh* `quick_xml::Reader` over just
    /// the unconsumed tail, so quick-xml's own `check_end_names`/
    /// `allow_unmatched_ends` validation has no memory of Start tags
    /// consumed by a previous call and must stay disabled on that reader
    /// (see the comment below); this stack is what actually enforces tag
    /// balance, mirroring `parse()`'s default `check_end_names = true` /
    /// `allow_unmatched_ends = false` convention by hand.
    open_stack: Vec<String>,
    /// Set once a mismatched/unmatched end tag has been reported and
    /// draining has stopped for good, mirroring `parse()`'s "diagnostic +
    /// stop" behavior on a fatal XML error. Once true, `drain()` is a no-op
    /// so a caller that keeps calling `feed()` after the fatal error
    /// doesn't get any further (possibly-confusing) partial dispatch.
    failed: bool,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            pending: Vec::new(),
            diagnostics: Vec::new(),
            entity_resolver: EntityResolver::new(DtdEntities::empty()),
            open_stack: Vec::new(),
            failed: false,
        }
    }

    /// Feed a chunk of bytes, dispatching every event that can be proven
    /// complete from the bytes seen so far.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.pending.extend_from_slice(chunk);
        self.drain(false);
    }

    /// Signal that no more input is coming: drain any remaining buffered
    /// bytes, resolving ambiguous trailing text and reporting genuine
    /// syntax errors (unterminated tags/comments/etc.) as diagnostics.
    pub fn finish(mut self) -> Vec<Diagnostic> {
        self.drain(true);
        self.diagnostics
    }

    /// Attempt to drain as many complete events as possible from
    /// `self.pending`, dispatching each to the handler and shrinking the
    /// buffer as tokens are confirmed consumed.
    fn drain(&mut self, is_final: bool) {
        if self.failed {
            return;
        }
        loop {
            if self.pending.is_empty() {
                return;
            }

            let mut reader = Reader::from_reader(&self.pending[..]);
            reader.config_mut().trim_text(false);
            // Each drain() call constructs a fresh `Reader` over just the
            // unconsumed tail (already-consumed prefixes are dropped to keep
            // memory bounded — see module docs). That means this reader
            // never sees the `Start` tag matching an `End` tag that was
            // consumed by a *previous* drain() call, so quick-xml's own
            // start/end name validation must be disabled here. Tag balance
            // is instead enforced by hand via `self.open_stack` below, which
            // *does* persist across calls — see the field's doc comment.
            reader.config_mut().check_end_names = false;
            reader.config_mut().allow_unmatched_ends = true;
            let mut buf = Vec::new();
            let total_len = self.pending.len();

            match reader.read_event_into(&mut buf) {
                Ok(XmlEvent::Eof) => {
                    // Nothing left to parse from the buffered bytes.
                    if is_final && !self.pending.is_empty() {
                        // Leftover bytes that parsed as nothing (e.g. only
                        // whitespace) — nothing to report, just drop them.
                        self.pending.clear();
                    }
                    return;
                }
                Ok(XmlEvent::Text(t)) => {
                    let consumed = reader.buffer_position() as usize;
                    let ambiguous_eof = consumed == total_len;
                    if ambiguous_eof && !is_final {
                        // Could still be continued by the next chunk — wait.
                        return;
                    }
                    let content = t
                        .decode()
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                    self.pending.drain(0..consumed);
                    if !content.is_empty() {
                        self.handler.handle(OwnedEvent::Text(content.into()));
                    }
                }
                Ok(event) => {
                    let consumed = reader.buffer_position() as usize;
                    let owned = crate::events::owned_event_from_xml(event, &self.entity_resolver);

                    // Manual open/close tag-balance check, replacing the
                    // quick-xml check_end_names/allow_unmatched_ends
                    // validation disabled above (see open_stack's doc
                    // comment). Only fires on a fully-formed Start/End event
                    // — reaching this arm at all already guarantees the
                    // token is complete, so this can't misfire on a token
                    // split across a chunk boundary.
                    if let Some(owned) = &owned {
                        match owned {
                            OwnedEvent::StartElement { name, .. } => {
                                self.open_stack.push(name.to_string());
                            }
                            OwnedEvent::EndElement { name } => match self.open_stack.pop() {
                                Some(expected) if expected == name.as_ref() => {}
                                Some(expected) => {
                                    self.diagnostics.push(Diagnostic {
                                        message: format!(
                                            "XML parse error: expected `</{expected}>`, but \
                                             `</{name}>` was found"
                                        ),
                                        span: Span::NONE,
                                    });
                                    self.pending.clear();
                                    self.failed = true;
                                    return;
                                }
                                None => {
                                    self.diagnostics.push(Diagnostic {
                                        message: format!(
                                            "XML parse error: close tag `</{name}>` does not \
                                             match any open tag"
                                        ),
                                        span: Span::NONE,
                                    });
                                    self.pending.clear();
                                    self.failed = true;
                                    return;
                                }
                            },
                            _ => {}
                        }
                    }

                    self.pending.drain(0..consumed);
                    if let Some(owned) = &owned
                        && let OwnedEvent::Doctype(content) = owned
                    {
                        let (declared, entity_diagnostics) = DtdEntities::parse_doctype(content);
                        for d in entity_diagnostics {
                            self.diagnostics.push(Diagnostic {
                                message: format!("DOCTYPE internal subset: {d}"),
                                span: Span::NONE,
                            });
                        }
                        self.entity_resolver = EntityResolver::new(declared);
                    }
                    if let Some(owned) = owned {
                        self.handler.handle(owned);
                    }
                }
                Err(e) => {
                    if is_final {
                        self.diagnostics.push(Diagnostic {
                            message: format!("XML parse error: {e}"),
                            span: Span::NONE,
                        });
                        self.pending.clear();
                    }
                    // Otherwise: assume the error is due to a token spanning
                    // the chunk boundary (unterminated tag/comment/CDATA/
                    // decl) and wait for more data.
                    return;
                }
            }
        }
    }
}
