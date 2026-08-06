//! Streaming HTML writer — converts events to HTML bytes incrementally.
//!
//! # Buffering, and why
//!
//! The incremental reader (`events()`/`StreamingParser`, see
//! `crate::sink`/`crate::events`) can emit structural correction events
//! (`NodeReparented`, `ChildrenReparented`, `NodeDetached`) that move a
//! subtree whose `StartElement`/text/etc. bytes may already have been
//! produced. A byte-stream writer cannot un-write bytes already flushed to
//! its sink, so `Writer` cannot serialize the old "emit every event
//! straight to the sink" way and still be correct against a stream that may
//! contain corrections.
//!
//! Instead, `Writer` keeps a small in-memory tree keyed by the event
//! stream's `NodeId`s — the same shape `events::collect_doc` builds — and
//! only converts a node to its final rendered bytes once the whole document
//! is known (at [`finish()`](Writer::finish)). This keeps `Writer` correct
//! for any valid event stream, including one produced by this crate's own
//! incremental reader, at the cost of buffering the (already-bounded, since
//! it's just ids/strings, not a second full parse) node tree until
//! `finish()`.
//!
//! **This is a known regression from the writer's pre-correction-event
//! behavior**, where every event was serialized straight to the sink with
//! no buffering at all (HTML syntax maps directly from event to bytes with
//! no lookahead). Streams that never contain correction events — e.g.
//! events derived by walking an already-resolved `HtmlDoc` via
//! `events::events_from_doc`, which `rescribe-write-html` uses — now pay
//! the buffering cost too, even though nothing about them requires it. A
//! `Writer` that streams bytes immediately and only falls back to buffering
//! once (if ever) a correction event actually arrives would restore the
//! fast path for the common case while keeping the corrected-stream case
//! correct; that rework is not done here — this change is scoped to the
//! *reader* side's incrementality — and is an open follow-up.
//!
//! # Example
//! ```no_run
//! use html_fmt::writer::Writer;
//! use html_fmt::{Event, NodeId};
//! use std::borrow::Cow;
//!
//! // Real event streams get their ids from `events()`/`StreamingParser`
//! // (or `events_from_doc`); ids just need to be unique and nonzero here.
//! let p = NodeId::for_doctest(1);
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(Event::StartElement {
//!     node: p,
//!     parent: NodeId::DOCUMENT,
//!     before_sibling: None,
//!     tag: Cow::Borrowed("p"),
//!     attrs: vec![],
//!     self_closing: false,
//! });
//! w.write_event(Event::Text {
//!     node: NodeId::for_doctest(2),
//!     parent: p,
//!     before_sibling: None,
//!     content: Cow::Borrowed("Hello"),
//! });
//! w.write_event(Event::EndElement { node: p, tag: Cow::Borrowed("p") });
//! let bytes = w.finish();
//! assert_eq!(&bytes, b"<p>Hello</p>");
//! ```

use std::io::Write;

use crate::emit::{escape_attr, escape_html};
use crate::events::{Event, OwnedEvent, collect_doc};

/// Streaming HTML writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to render and flush the sink. See the
/// [module docs](self) for why rendering happens at `finish()` rather than
/// per event.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            events: Vec::new(),
        }
    }

    /// Write one event to the sink.
    pub fn write_event(&mut self, event: Event<'_>) {
        self.events.push(event.into_owned());
    }

    /// Render every event received so far (applying any corrections) and
    /// flush the resulting bytes to the sink.
    pub fn finish(mut self) -> W {
        let doc = collect_doc(std::mem::take(&mut self.events));
        let _ = self.sink.write_all(&render(&doc));
        self.sink
    }
}

fn render(doc: &crate::ast::HtmlDoc) -> Vec<u8> {
    let mut out = Vec::new();
    for node in &doc.nodes {
        render_node(node, &mut out);
    }
    out
}

fn render_node(node: &crate::ast::Node, out: &mut Vec<u8>) {
    use crate::ast::Node;
    match node {
        Node::Doctype {
            name,
            public_id,
            system_id,
            ..
        } => {
            let _ = write!(out, "<!DOCTYPE {}", name);
            if !public_id.is_empty() {
                let _ = write!(out, " PUBLIC \"{}\"", public_id);
                if !system_id.is_empty() {
                    let _ = write!(out, " \"{}\"", system_id);
                }
            } else if !system_id.is_empty() {
                let _ = write!(out, " SYSTEM \"{}\"", system_id);
            }
            let _ = write!(out, ">");
        }
        Node::Element {
            tag,
            attrs,
            children,
            self_closing,
            ..
        } => {
            let _ = write!(out, "<{}", tag);
            for (name, value) in attrs {
                let _ = write!(out, " {}=\"{}\"", name, escape_attr(value));
            }
            let _ = write!(out, ">");
            if !*self_closing {
                for child in children {
                    render_node(child, out);
                }
                let _ = write!(out, "</{}>", tag);
            }
        }
        Node::Text { content, .. } => {
            let _ = write!(out, "{}", escape_html(content));
        }
        Node::Comment { content, .. } => {
            let _ = write!(out, "<!--{}-->", content);
        }
        Node::Raw { content, .. } => {
            let _ = write!(out, "{}", content);
        }
    }
}
