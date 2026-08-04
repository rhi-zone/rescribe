//! Streaming EndNote XML writer — converts [`Event`]s to bytes
//! incrementally, using `quick_xml::Writer` under the hood for correct
//! escaping. An independent implementation from [`crate::emit::emit`] (not
//! routed through it): each event maps directly to output bytes, with a
//! small explicit stack tracking which element name each `Start*`/`End*`
//! domain event pair should open/close (needed since several domain events
//! share the same underlying tag name depending on context — e.g.
//! `Event::StartUrlRole` always writes `<related-urls>` or `<pdf-urls>`,
//! never looked up dynamically, so no buffering beyond that stack is
//! required).

use std::io::Write;

use quick_xml::Writer as XmlWriter;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event as XmlEvent};

use crate::ast::{AuthorRole, UrlRole};
use crate::events::Event;

/// Streaming EndNote XML writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush and recover the sink.
pub struct Writer<W: Write> {
    inner: XmlWriter<W>,
    /// Tag names for currently-open elements written by container/leaf
    /// `Start*` events, popped by the matching `End*` event so `EndElement`
    /// (used by every leaf field regardless of name) knows what to close.
    open: Vec<String>,
}

impl<W: Write> Writer<W> {
    /// Not pretty-printed — see [`crate::emit::emit`]'s doc comment for why:
    /// an auto-indenting writer would corrupt meaningful whitespace between
    /// `<style>` runs in field content.
    pub fn new(sink: W) -> Self {
        Writer {
            inner: XmlWriter::new(sink),
            open: Vec::new(),
        }
    }

    /// Write one event to the sink.
    pub fn write_event(&mut self, event: Event<'_>) {
        let _ = self.write_event_inner(event);
    }

    fn open_tag(
        &mut self,
        name: &str,
        attrs: &[(String, std::borrow::Cow<'_, str>)],
    ) -> std::io::Result<()> {
        let mut start = BytesStart::new(name);
        for (k, v) in attrs {
            start.push_attribute((k.as_str(), v.as_ref()));
        }
        self.open.push(name.to_string());
        self.inner.write_event(XmlEvent::Start(start))
    }

    fn open_tag_plain(&mut self, name: &str) -> std::io::Result<()> {
        self.open.push(name.to_string());
        self.inner
            .write_event(XmlEvent::Start(BytesStart::new(name)))
    }

    fn close_tag(&mut self) -> std::io::Result<()> {
        let name = self.open.pop().unwrap_or_default();
        self.inner.write_event(XmlEvent::End(BytesEnd::new(name)))
    }

    fn write_event_inner(&mut self, event: Event<'_>) -> std::io::Result<()> {
        match event {
            Event::Decl {
                version,
                encoding,
                standalone,
            } => self
                .inner
                .write_event(XmlEvent::Decl(quick_xml::events::BytesDecl::new(
                    &version,
                    encoding.as_deref(),
                    standalone.as_deref(),
                ))),
            Event::StartDocument => self.open_tag_plain("xml"),
            Event::EndDocument => self.close_tag(),
            Event::StartRecords => self.open_tag_plain("records"),
            Event::EndRecords => self.close_tag(),
            Event::StartRecord => self.open_tag_plain("record"),
            Event::EndRecord => self.close_tag(),
            Event::StartForeignKeys => self.open_tag_plain("foreign-keys"),
            Event::EndForeignKeys => self.close_tag(),
            Event::StartContributors => self.open_tag_plain("contributors"),
            Event::EndContributors => self.close_tag(),
            Event::StartAuthorRole(r) => self.open_tag_plain(author_role_tag(r)),
            Event::EndAuthorRole(_) => self.close_tag(),
            Event::StartTitles => self.open_tag_plain("titles"),
            Event::EndTitles => self.close_tag(),
            Event::StartPeriodical => self.open_tag_plain("periodical"),
            Event::EndPeriodical => self.close_tag(),
            Event::StartUrls => self.open_tag_plain("urls"),
            Event::EndUrls => self.close_tag(),
            Event::StartUrlRole(r) => self.open_tag_plain(url_role_tag(r)),
            Event::EndUrlRole(_) => self.close_tag(),
            Event::StartKeywords => self.open_tag_plain("keywords"),
            Event::EndKeywords => self.close_tag(),
            Event::StartDates => self.open_tag_plain("dates"),
            Event::EndDates => self.close_tag(),
            Event::StartPubDates => self.open_tag_plain("pub-dates"),
            Event::EndPubDates => self.close_tag(),
            Event::StartElement { name, attrs } => self.open_tag(&name, &attrs),
            Event::EndElement => self.close_tag(),
            Event::Text(t) => self.inner.write_event(XmlEvent::Text(BytesText::new(&t))),
            Event::StartStyle { face } => {
                let mut start = BytesStart::new("style");
                start.push_attribute(("face", face.as_ref()));
                self.open.push("style".to_string());
                self.inner.write_event(XmlEvent::Start(start))
            }
            Event::EndStyle => self.close_tag(),
        }
        .map_err(|e| std::io::Error::other(format!("XML write error: {e}")))
    }

    /// Flush and return the underlying sink.
    pub fn finish(self) -> W {
        self.inner.into_inner()
    }
}

fn author_role_tag(r: AuthorRole) -> &'static str {
    match r {
        AuthorRole::Authors => "authors",
        AuthorRole::SecondaryAuthors => "secondary-authors",
        AuthorRole::TertiaryAuthors => "tertiary-authors",
        AuthorRole::SubsidiaryAuthors => "subsidiary-authors",
    }
}

fn url_role_tag(r: UrlRole) -> &'static str {
    match r {
        UrlRole::RelatedUrls => "related-urls",
        UrlRole::PdfUrls => "pdf-urls",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_simple_document() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartDocument);
        w.write_event(Event::StartRecord);
        w.write_event(Event::StartElement {
            name: "ref-type".into(),
            attrs: vec![("name".to_string(), "Journal Article".into())],
        });
        w.write_event(Event::Text("17".into()));
        w.write_event(Event::EndElement);
        w.write_event(Event::StartTitles);
        w.write_event(Event::StartElement {
            name: "title".into(),
            attrs: vec![],
        });
        w.write_event(Event::Text("T".into()));
        w.write_event(Event::EndElement);
        w.write_event(Event::EndTitles);
        w.write_event(Event::EndRecord);
        w.write_event(Event::EndDocument);
        let bytes = w.finish();
        let xml = String::from_utf8(bytes).unwrap();
        assert!(xml.contains(r#"<ref-type name="Journal Article">17</ref-type>"#));
        assert!(xml.contains("<title>T</title>"));
    }
}
