//! Streaming man page writer -- converts a stream of events to man page text.
//!
//! This implementation buffers all events, reconstructs the AST, then emits.
//!
//! # Example
//! ```no_run
//! use man_fmt::writer::Writer;
//! use man_fmt::OwnedManEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedManEvent::StartHeading { level: 2 });
//! w.write_event(OwnedManEvent::Text("NAME".to_string().into()));
//! w.write_event(OwnedManEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedManEvent;
use std::io::Write;

/// Streaming man page writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush man page text to the underlying sink
/// and recover it.
pub struct Writer<W: Write> {
    sink: W,
    events: Vec<OwnedManEvent>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            events: Vec::new(),
        }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OwnedManEvent) {
        self.events.push(event);
    }

    /// Flush all buffered events as man page text and return the sink.
    pub fn finish(mut self) -> W {
        let doc = crate::events::collect_doc_from_events(std::mem::take(&mut self.events).into_iter());
        let text = crate::emit::build(&doc);
        let _ = self.sink.write_all(text.as_bytes());
        self.sink
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedManEvent;
    use std::borrow::Cow;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedManEvent::StartHeading { level: 2 });
        w.write_event(OwnedManEvent::Text(Cow::Owned("NAME".to_string())));
        w.write_event(OwnedManEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains(".SH NAME"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedManEvent::StartParagraph);
        w.write_event(OwnedManEvent::Text(Cow::Owned("Hello world".to_string())));
        w.write_event(OwnedManEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("Hello world"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = ".SH NAME\ntest \\- a test program\n.SH SYNOPSIS\ntest [options]\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        // The emitted text should re-parse without panicking and contain
        // the key content.
        let (doc_emit, _) = crate::parse::parse(&emitted_text);
        assert!(
            !doc_emit.blocks.is_empty(),
            "writer roundtrip should produce blocks"
        );
        assert!(
            emitted_text.contains("NAME"),
            "emitted text should contain NAME"
        );
        assert!(
            emitted_text.contains("SYNOPSIS"),
            "emitted text should contain SYNOPSIS"
        );
    }
}
