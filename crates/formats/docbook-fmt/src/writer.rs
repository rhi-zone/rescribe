//! Streaming DocBook/XML writer — converts events to bytes incrementally,
//! using `quick_xml::Writer` under the hood for correct escaping.

use std::io::Write;

use quick_xml::Writer as XmlWriter;
use quick_xml::events::{
    BytesCData, BytesDecl, BytesPI, BytesRef, BytesStart, BytesText, Event as XmlEvent,
};

use crate::events::Event;

/// Streaming DocBook/XML writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush and recover the sink. Each event
/// maps directly to bytes with no buffering or lookahead — XML syntax
/// (unlike HTML pretty-printing) needs none.
pub struct Writer<W: Write> {
    inner: XmlWriter<W>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            inner: XmlWriter::new(sink),
        }
    }

    /// Write one event to the sink.
    pub fn write_event(&mut self, event: Event<'_>) {
        let _ = self.write_event_inner(event);
    }

    fn write_event_inner(&mut self, event: Event<'_>) -> std::io::Result<()> {
        let xml_result = match event {
            Event::Decl {
                version,
                encoding,
                standalone,
            } => self.inner.write_event(XmlEvent::Decl(BytesDecl::new(
                &version,
                encoding.as_deref(),
                standalone.as_deref(),
            ))),
            Event::Doctype(content) => self
                .inner
                .write_event(XmlEvent::DocType(BytesText::new(&content))),
            Event::ProcessingInstruction { target, data } => {
                let content = if data.is_empty() {
                    target.into_owned()
                } else {
                    format!("{target} {data}")
                };
                self.inner.write_event(XmlEvent::PI(BytesPI::new(content)))
            }
            Event::StartElement { name, attrs } => {
                let mut start = BytesStart::new(name.into_owned());
                for (k, v) in &attrs {
                    start.push_attribute((k.as_str(), v.as_str()));
                }
                self.inner.write_event(XmlEvent::Start(start))
            }
            Event::EmptyElement { name, attrs } => {
                let mut start = BytesStart::new(name.into_owned());
                for (k, v) in &attrs {
                    start.push_attribute((k.as_str(), v.as_str()));
                }
                self.inner.write_event(XmlEvent::Empty(start))
            }
            Event::EndElement { name } => {
                self.inner
                    .write_event(XmlEvent::End(quick_xml::events::BytesEnd::new(
                        name.into_owned(),
                    )))
            }
            Event::Text(t) => self
                .inner
                .write_event(XmlEvent::Text(BytesText::new(t.as_ref()))),
            Event::Cdata(t) => self
                .inner
                .write_event(XmlEvent::CData(BytesCData::new(t.as_ref()))),
            Event::Comment(t) => self
                .inner
                .write_event(XmlEvent::Comment(BytesText::new(t.as_ref()))),
            Event::EntityRef(name) => self
                .inner
                .write_event(XmlEvent::GeneralRef(BytesRef::new(name.as_ref()))),
            Event::Raw(content) => {
                return self.inner.get_mut().write_all(content.as_bytes());
            }
        };
        xml_result.map_err(|e| std::io::Error::other(format!("XML write error: {e}")))
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
    fn writes_simple_element() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartElement {
            name: Cow::Borrowed("para"),
            attrs: vec![],
        });
        w.write_event(Event::Text(Cow::Borrowed("Hello")));
        w.write_event(Event::EndElement {
            name: Cow::Borrowed("para"),
        });
        let bytes = w.finish();
        assert_eq!(String::from_utf8(bytes).unwrap(), "<para>Hello</para>");
    }

    #[test]
    fn writes_empty_element() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::EmptyElement {
            name: Cow::Borrowed("br"),
            attrs: vec![],
        });
        let bytes = w.finish();
        assert_eq!(String::from_utf8(bytes).unwrap(), "<br/>");
    }
}
