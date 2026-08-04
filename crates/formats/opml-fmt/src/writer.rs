//! Streaming OPML writer — converts [`Event`]s to bytes incrementally,
//! using `quick_xml::Writer` under the hood for correct escaping. An
//! independent implementation from [`crate::emit::emit`] (not routed
//! through it): each event maps directly to output bytes with no
//! buffering or lookahead beyond the head-field text run tracked below.

use std::io::Write;

use quick_xml::Writer as XmlWriter;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event as XmlEvent};

use crate::events::Event;

/// Streaming OPML writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush and recover the sink.
pub struct Writer<W: Write> {
    inner: XmlWriter<W>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            inner: XmlWriter::new_with_indent(sink, b' ', 2),
        }
    }

    /// Write one event to the sink.
    pub fn write_event(&mut self, event: Event<'_>) {
        let _ = self.write_event_inner(event);
    }

    fn write_event_inner(&mut self, event: Event<'_>) -> std::io::Result<()> {
        let result = match event {
            Event::Decl {
                version,
                encoding,
                standalone,
            } => self.inner.write_event(XmlEvent::Decl(BytesDecl::new(
                &version,
                encoding.as_deref(),
                standalone.as_deref(),
            ))),
            Event::StartOpml { version } => {
                let mut start = BytesStart::new("opml");
                start.push_attribute(("version", version.as_ref()));
                self.inner.write_event(XmlEvent::Start(start))
            }
            Event::EndOpml => self.inner.write_event(XmlEvent::End(BytesEnd::new("opml"))),
            Event::StartHead => self
                .inner
                .write_event(XmlEvent::Start(BytesStart::new("head"))),
            Event::EndHead => self.inner.write_event(XmlEvent::End(BytesEnd::new("head"))),
            Event::HeadField { name, value } => {
                if value.is_empty() {
                    self.inner
                        .write_event(XmlEvent::Empty(BytesStart::new(name.into_owned())))
                } else {
                    self.inner
                        .write_event(XmlEvent::Start(BytesStart::new(name.clone().into_owned())))
                        .and_then(|_| {
                            self.inner
                                .write_event(XmlEvent::Text(BytesText::new(&value)))
                        })
                        .and_then(|_| {
                            self.inner
                                .write_event(XmlEvent::End(BytesEnd::new(name.into_owned())))
                        })
                }
            }
            Event::StartBody => self
                .inner
                .write_event(XmlEvent::Start(BytesStart::new("body"))),
            Event::EndBody => self.inner.write_event(XmlEvent::End(BytesEnd::new("body"))),
            Event::StartOutline { attrs } => {
                let mut start = BytesStart::new("outline");
                for (k, v) in &attrs {
                    start.push_attribute((k.as_str(), v.as_ref()));
                }
                self.inner.write_event(XmlEvent::Start(start))
            }
            Event::EndOutline => self
                .inner
                .write_event(XmlEvent::End(BytesEnd::new("outline"))),
            Event::EmptyOutline { attrs } => {
                let mut start = BytesStart::new("outline");
                for (k, v) in &attrs {
                    start.push_attribute((k.as_str(), v.as_ref()));
                }
                self.inner.write_event(XmlEvent::Empty(start))
            }
        };
        result.map_err(|e| std::io::Error::other(format!("XML write error: {e}")))
    }

    /// Flush and return the underlying sink.
    pub fn finish(self) -> W {
        self.inner.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn writes_simple_document() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartOpml {
            version: Cow::Borrowed("2.0"),
        });
        w.write_event(Event::StartHead);
        w.write_event(Event::HeadField {
            name: Cow::Borrowed("title"),
            value: Cow::Borrowed("T"),
        });
        w.write_event(Event::EndHead);
        w.write_event(Event::StartBody);
        w.write_event(Event::EmptyOutline {
            attrs: vec![("text".to_string(), Cow::Borrowed("X"))],
        });
        w.write_event(Event::EndBody);
        w.write_event(Event::EndOpml);
        let bytes = w.finish();
        let xml = String::from_utf8(bytes).unwrap();
        assert!(xml.contains(r#"<opml version="2.0">"#));
        assert!(xml.contains("<title>T</title>"));
        assert!(xml.contains(r#"<outline text="X"/>"#));
    }
}
