//! Streaming HTML writer — converts events to HTML bytes incrementally.
//!
//! Unlike many format writers, the HTML streaming writer can emit each event
//! immediately without buffering, because HTML syntax maps directly from
//! events to bytes (no lookahead or context needed).
//!
//! # Example
//! ```no_run
//! use html_fmt::writer::Writer;
//! use html_fmt::Event;
//! use std::borrow::Cow;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(Event::StartElement {
//!     tag: Cow::Borrowed("p"),
//!     attrs: vec![],
//!     self_closing: false,
//! });
//! w.write_event(Event::Text(Cow::Borrowed("Hello")));
//! w.write_event(Event::EndElement { tag: Cow::Borrowed("p") });
//! let bytes = w.finish();
//! assert_eq!(&bytes, b"<p>Hello</p>");
//! ```

use std::io::Write;

use crate::emit::{escape_attr, escape_html};
use crate::events::Event;

/// Streaming HTML writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush and recover the sink.
pub struct Writer<W: Write> {
    sink: W,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink }
    }

    /// Write one event to the sink.
    pub fn write_event(&mut self, event: Event<'_>) {
        // Ignore I/O errors during streaming; callers check the sink.
        let _ = self.write_event_inner(event);
    }

    fn write_event_inner(&mut self, event: Event<'_>) -> std::io::Result<()> {
        match event {
            Event::Doctype {
                name,
                public_id,
                system_id,
            } => {
                write!(self.sink, "<!DOCTYPE {}", name)?;
                if !public_id.is_empty() {
                    write!(self.sink, " PUBLIC \"{}\"", public_id)?;
                    if !system_id.is_empty() {
                        write!(self.sink, " \"{}\"", system_id)?;
                    }
                } else if !system_id.is_empty() {
                    write!(self.sink, " SYSTEM \"{}\"", system_id)?;
                }
                write!(self.sink, ">")?;
            }
            Event::StartElement {
                tag,
                attrs,
                self_closing: _,
            } => {
                write!(self.sink, "<{}", tag)?;
                for (name, value) in &attrs {
                    write!(self.sink, " {}=\"{}\"", name, escape_attr(value))?;
                }
                write!(self.sink, ">")?;
            }
            Event::EndElement { tag } => {
                write!(self.sink, "</{}>", tag)?;
            }
            Event::Text(text) => {
                write!(self.sink, "{}", escape_html(&text))?;
            }
            Event::Comment(text) => {
                write!(self.sink, "<!--{}-->", text)?;
            }
            Event::Raw(text) => {
                write!(self.sink, "{}", text)?;
            }
        }
        Ok(())
    }

    /// Flush and return the underlying sink.
    pub fn finish(self) -> W {
        self.sink
    }
}
