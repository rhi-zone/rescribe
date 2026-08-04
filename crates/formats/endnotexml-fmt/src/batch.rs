//! Chunk-driven (batch) EndNote XML parser with **true incremental** event
//! delivery.
//!
//! EndNote XML is well-nested XML, so every markup token is unambiguously
//! complete or incomplete on its own the same way it is for `opml-fmt`/
//! `docbook-fmt`/`jats-fmt`/`tei-fmt` (see those crates' `batch.rs` module
//! docs for the underlying `quick_xml` behavior this relies on):
//! `StreamingParser::feed` drains every event it can prove is complete,
//! dispatches it to the [`Handler`] immediately, and drops the consumed
//! prefix — bounded by the largest *in-progress* token, not the whole
//! document.
//!
//! The one token that can't be resolved the instant `quick_xml` returns it
//! is `Text` content inside a leaf field or `<style>` run — the same "did
//! the text run end at `<` or just at the end of currently-buffered bytes"
//! ambiguity described in the sibling crates' `batch.rs` docs. `drain`
//! handles this the same way they do: a `Text` token that consumes exactly
//! to the end of the currently-buffered bytes is treated as possibly
//! incomplete and held back (not dispatched, not drained from `pending`)
//! until more input arrives or `finish()` is called.

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::ast::{AuthorRole, Diagnostic, Span, UrlRole};
use crate::events::OwnedEvent;

/// Chunk-driven EndNote XML parser that returns the full AST on `finish()`.
///
/// Accumulates bytes and calls [`crate::parse::parse`] once at the end —
/// for the AST-building use case there's no way to avoid holding the whole
/// document anyway. Callers who need bounded memory should use
/// [`StreamingParser`] instead.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        BatchParser { buf: Vec::new() }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    pub fn finish(self) -> (crate::ast::EndNoteDoc, Vec<Diagnostic>) {
        crate::parse::parse(&self.buf)
    }
}

/// Handler trait for streaming EndNote XML events. Implemented
/// automatically for any `FnMut(OwnedEvent)`.
pub trait Handler {
    fn handle(&mut self, event: OwnedEvent);
}

impl<F: FnMut(OwnedEvent)> Handler for F {
    fn handle(&mut self, event: OwnedEvent) {
        self(event);
    }
}

/// Mirrors `events.rs::Frame` — the open-element stack this streaming
/// parser tracks to know how to dispatch the next token and what to emit
/// when the current element closes. Kept as its own copy (rather than
/// reusing `events::Frame`, which is private to that module) since
/// `StreamingParser` is a fully independent implementation, per this
/// crate's own architecture rule: `events()` and `StreamingParser` must not
/// be derived from one another.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Frame {
    Opaque,
    Document,
    Records,
    Record,
    ForeignKeys,
    Contributors,
    AuthorRole(AuthorRole),
    Titles,
    Periodical,
    Urls,
    UrlRole(UrlRole),
    Keywords,
    Dates,
    PubDates,
    InlineElement,
    InlineStyle,
}

fn dispatch(
    top: Option<Frame>,
    name: &str,
    attrs: Vec<(String, String)>,
) -> (Frame, Option<OwnedEvent>) {
    let attrs_cow: Vec<(String, std::borrow::Cow<'static, str>)> = attrs
        .into_iter()
        .map(|(k, v)| (k, std::borrow::Cow::Owned(v)))
        .collect();
    match top {
        None => {
            if name == "xml" {
                (Frame::Document, Some(OwnedEvent::StartDocument))
            } else {
                (Frame::Opaque, None)
            }
        }
        Some(Frame::Document) => match name {
            "records" => (Frame::Records, Some(OwnedEvent::StartRecords)),
            "record" => (Frame::Record, Some(OwnedEvent::StartRecord)),
            _ => (Frame::Opaque, None),
        },
        Some(Frame::Records) | Some(Frame::Opaque) => {
            if name == "record" {
                (Frame::Record, Some(OwnedEvent::StartRecord))
            } else {
                (Frame::Opaque, None)
            }
        }
        Some(Frame::Record) => match name {
            "contributors" => (Frame::Contributors, Some(OwnedEvent::StartContributors)),
            "titles" => (Frame::Titles, Some(OwnedEvent::StartTitles)),
            "periodical" => (Frame::Periodical, Some(OwnedEvent::StartPeriodical)),
            "urls" => (Frame::Urls, Some(OwnedEvent::StartUrls)),
            "foreign-keys" => (Frame::ForeignKeys, Some(OwnedEvent::StartForeignKeys)),
            "keywords" => (Frame::Keywords, Some(OwnedEvent::StartKeywords)),
            "dates" => (Frame::Dates, Some(OwnedEvent::StartDates)),
            _ => (
                Frame::InlineElement,
                Some(OwnedEvent::StartElement {
                    name: name.to_string().into(),
                    attrs: attrs_cow,
                }),
            ),
        },
        Some(Frame::Contributors) => match name {
            "authors" => (
                Frame::AuthorRole(AuthorRole::Authors),
                Some(OwnedEvent::StartAuthorRole(AuthorRole::Authors)),
            ),
            "secondary-authors" => (
                Frame::AuthorRole(AuthorRole::SecondaryAuthors),
                Some(OwnedEvent::StartAuthorRole(AuthorRole::SecondaryAuthors)),
            ),
            "tertiary-authors" => (
                Frame::AuthorRole(AuthorRole::TertiaryAuthors),
                Some(OwnedEvent::StartAuthorRole(AuthorRole::TertiaryAuthors)),
            ),
            "subsidiary-authors" => (
                Frame::AuthorRole(AuthorRole::SubsidiaryAuthors),
                Some(OwnedEvent::StartAuthorRole(AuthorRole::SubsidiaryAuthors)),
            ),
            _ => (
                Frame::InlineElement,
                Some(OwnedEvent::StartElement {
                    name: name.to_string().into(),
                    attrs: attrs_cow,
                }),
            ),
        },
        Some(Frame::Urls) => match name {
            "related-urls" => (
                Frame::UrlRole(UrlRole::RelatedUrls),
                Some(OwnedEvent::StartUrlRole(UrlRole::RelatedUrls)),
            ),
            "pdf-urls" => (
                Frame::UrlRole(UrlRole::PdfUrls),
                Some(OwnedEvent::StartUrlRole(UrlRole::PdfUrls)),
            ),
            _ => (
                Frame::InlineElement,
                Some(OwnedEvent::StartElement {
                    name: name.to_string().into(),
                    attrs: attrs_cow,
                }),
            ),
        },
        Some(Frame::Dates) => match name {
            "pub-dates" => (Frame::PubDates, Some(OwnedEvent::StartPubDates)),
            _ => (
                Frame::InlineElement,
                Some(OwnedEvent::StartElement {
                    name: name.to_string().into(),
                    attrs: attrs_cow,
                }),
            ),
        },
        Some(Frame::InlineElement) | Some(Frame::InlineStyle) => {
            if name == "style" {
                let face = attrs_cow
                    .into_iter()
                    .find(|(k, _)| k == "face")
                    .map(|(_, v)| v)
                    .unwrap_or_default();
                (Frame::InlineStyle, Some(OwnedEvent::StartStyle { face }))
            } else {
                (
                    Frame::InlineElement,
                    Some(OwnedEvent::StartElement {
                        name: name.to_string().into(),
                        attrs: attrs_cow,
                    }),
                )
            }
        }
        Some(
            Frame::ForeignKeys
            | Frame::AuthorRole(_)
            | Frame::Titles
            | Frame::Periodical
            | Frame::UrlRole(_)
            | Frame::Keywords
            | Frame::PubDates,
        ) => (
            Frame::InlineElement,
            Some(OwnedEvent::StartElement {
                name: name.to_string().into(),
                attrs: attrs_cow,
            }),
        ),
    }
}

fn end_event(frame: Frame) -> Option<OwnedEvent> {
    match frame {
        Frame::Opaque => None,
        Frame::Document => Some(OwnedEvent::EndDocument),
        Frame::Records => Some(OwnedEvent::EndRecords),
        Frame::Record => Some(OwnedEvent::EndRecord),
        Frame::ForeignKeys => Some(OwnedEvent::EndForeignKeys),
        Frame::Contributors => Some(OwnedEvent::EndContributors),
        Frame::AuthorRole(r) => Some(OwnedEvent::EndAuthorRole(r)),
        Frame::Titles => Some(OwnedEvent::EndTitles),
        Frame::Periodical => Some(OwnedEvent::EndPeriodical),
        Frame::Urls => Some(OwnedEvent::EndUrls),
        Frame::UrlRole(r) => Some(OwnedEvent::EndUrlRole(r)),
        Frame::Keywords => Some(OwnedEvent::EndKeywords),
        Frame::Dates => Some(OwnedEvent::EndDates),
        Frame::PubDates => Some(OwnedEvent::EndPubDates),
        Frame::InlineElement => Some(OwnedEvent::EndElement),
        Frame::InlineStyle => Some(OwnedEvent::EndStyle),
    }
}

/// Chunked streaming EndNote XML parser that delivers events to a
/// [`Handler`] as soon as they are provably complete.
pub struct StreamingParser<H: Handler> {
    handler: H,
    pending: Vec<u8>,
    diagnostics: Vec<Diagnostic>,
    /// Names of currently-open XML elements (generic; XML well-nestedness
    /// makes this sufficient to validate End tags).
    open_stack: Vec<String>,
    stack: Vec<Frame>,
    failed: bool,
}

impl<H: Handler> StreamingParser<H> {
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            pending: Vec::new(),
            diagnostics: Vec::new(),
            open_stack: Vec::new(),
            stack: Vec::new(),
            failed: false,
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.pending.extend_from_slice(chunk);
        self.drain(false);
    }

    pub fn finish(mut self) -> Vec<Diagnostic> {
        self.drain(true);
        self.diagnostics
    }

    fn drain(&mut self, is_final: bool) {
        if self.failed {
            return;
        }
        loop {
            if self.pending.is_empty() {
                if is_final {
                    self.close_out();
                }
                return;
            }

            let mut reader = Reader::from_reader(&self.pending[..]);
            reader.config_mut().trim_text(false);
            reader.config_mut().check_end_names = false;
            reader.config_mut().allow_unmatched_ends = true;
            let mut buf = Vec::new();
            let total_len = self.pending.len();

            match reader.read_event_into(&mut buf) {
                Ok(XmlEvent::Eof) => {
                    if is_final {
                        self.pending.clear();
                        self.close_out();
                    }
                    return;
                }
                Ok(XmlEvent::Text(t)) => {
                    let consumed = reader.buffer_position() as usize;
                    let ambiguous_eof = consumed == total_len;
                    if ambiguous_eof && !is_final {
                        return;
                    }
                    self.pending.drain(0..consumed);
                    let top = self.stack.last().copied();
                    if matches!(top, Some(Frame::InlineElement) | Some(Frame::InlineStyle)) {
                        let content = t
                            .decode()
                            .map(|c| c.into_owned())
                            .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                        if !content.is_empty() {
                            self.handler.handle(OwnedEvent::Text(content.into()));
                        }
                    }
                }
                Ok(XmlEvent::Decl(decl)) => {
                    let consumed = reader.buffer_position() as usize;
                    let version = decl
                        .version()
                        .map(|v| String::from_utf8_lossy(&v).into_owned())
                        .unwrap_or_else(|_| "1.0".to_string());
                    let encoding = decl
                        .encoding()
                        .and_then(|e| e.ok())
                        .map(|e| String::from_utf8_lossy(&e).into_owned());
                    let standalone = decl
                        .standalone()
                        .and_then(|s| s.ok())
                        .map(|s| String::from_utf8_lossy(&s).into_owned());
                    self.pending.drain(0..consumed);
                    self.handler.handle(OwnedEvent::Decl {
                        version: version.into(),
                        encoding: encoding.map(Into::into),
                        standalone: standalone.map(Into::into),
                    });
                }
                Ok(XmlEvent::Start(e)) => {
                    let consumed = reader.buffer_position() as usize;
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                    let attrs = read_attrs(&e);
                    self.open_stack.push(name.clone());
                    self.pending.drain(0..consumed);
                    let top = self.stack.last().copied();
                    let (frame, ev) = dispatch(top, &name, attrs);
                    self.stack.push(frame);
                    if let Some(ev) = ev {
                        self.handler.handle(ev);
                    }
                }
                Ok(XmlEvent::Empty(e)) => {
                    let consumed = reader.buffer_position() as usize;
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                    let attrs = read_attrs(&e);
                    self.pending.drain(0..consumed);
                    let top = self.stack.last().copied();
                    let (frame, start_ev) = dispatch(top, &name, attrs);
                    if let Some(ev) = start_ev {
                        self.handler.handle(ev);
                    }
                    if let Some(ev) = end_event(frame) {
                        self.handler.handle(ev);
                    }
                }
                Ok(XmlEvent::End(e)) => {
                    let consumed = reader.buffer_position() as usize;
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();

                    match self.open_stack.pop() {
                        Some(expected) if expected == name => {}
                        Some(expected) => {
                            self.diagnostics.push(Diagnostic {
                                message: format!(
                                    "XML parse error: expected `</{expected}>`, but `</{name}>` was found"
                                ),
                                span: Span::NONE,
                            });
                            self.open_stack.push(expected);
                            self.pending.clear();
                            self.failed = true;
                            self.close_out();
                            return;
                        }
                        None => {
                            self.diagnostics.push(Diagnostic {
                                message: format!(
                                    "XML parse error: close tag `</{name}>` does not match any open tag"
                                ),
                                span: Span::NONE,
                            });
                            self.pending.clear();
                            self.failed = true;
                            self.close_out();
                            return;
                        }
                    }

                    self.pending.drain(0..consumed);
                    let frame = self.stack.pop().unwrap_or(Frame::Opaque);
                    if let Some(ev) = end_event(frame) {
                        self.handler.handle(ev);
                    }
                }
                Ok(_) => {
                    let consumed = reader.buffer_position() as usize;
                    self.pending.drain(0..consumed);
                }
                Err(e) => {
                    if is_final {
                        self.diagnostics.push(Diagnostic {
                            message: format!("XML parse error: {e}"),
                            span: Span::NONE,
                        });
                        self.pending.clear();
                        self.close_out();
                    }
                    return;
                }
            }
        }
    }

    fn close_out(&mut self) {
        while let Some(frame) = self.stack.pop() {
            if let Some(ev) = end_event(frame) {
                self.handler.handle(ev);
            }
        }
    }
}

fn read_attrs(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .unescape_value()
            .map(|v| v.into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned());
        attrs.push((key, value));
    }
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_parser_finishes_across_chunks() {
        let mut p = BatchParser::new();
        p.feed(b"<xml><records><record><ref-type name=\"Journal Article\">17</ref-type><titles><title>Te");
        p.feed(b"st</title></titles></record></records></xml>");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.records.len(), 1);
        assert_eq!(
            crate::parse::flatten_inline_text(
                doc.records[0]
                    .titles
                    .as_ref()
                    .unwrap()
                    .title
                    .as_ref()
                    .unwrap()
            ),
            "Test"
        );
    }

    #[test]
    fn streaming_parser_delivers_events_incrementally() {
        let mut events = Vec::new();
        let mut p = StreamingParser::new(|e| events.push(e));
        p.feed(b"<xml><records><record><ref-type>17</ref-type><titles><title>Hi");
        p.feed(b"</title></titles></record></records></xml>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(events.contains(&OwnedEvent::StartRecord));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hi"))
        );
    }

    #[test]
    fn streaming_parser_splits_tag_across_chunks() {
        let mut events = Vec::new();
        let mut p = StreamingParser::new(|e| events.push(e));
        p.feed(b"<xml><records><rec");
        p.feed(b"ord><ref-type>17</ref-type></record></records></xml>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(events.contains(&OwnedEvent::StartRecord));
        assert!(events.contains(&OwnedEvent::EndRecord));
    }
}
