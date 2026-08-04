//! Chunk-driven (batch) OPML parser with **true incremental** event
//! delivery.
//!
//! OPML is well-nested XML, so every markup token is unambiguously complete
//! or incomplete on its own the same way it is for `tei-fmt`/`docbook-fmt`/
//! `jats-fmt` (see those crates' `batch.rs` module docs for the underlying
//! `quick_xml` behavior this relies on): `StreamingParser::feed` drains
//! every event it can prove is complete, dispatches it to the [`Handler`]
//! immediately, and drops the consumed prefix — bounded by the largest
//! *in-progress* token, not the whole document.
//!
//! The one token that can't be resolved the instant `quick_xml` returns it
//! is the text content of a `<head>` child element (`<title>`, `ownerName`,
//! ...): the same "did the text run end at `<` or just at the end of
//! currently-buffered bytes" ambiguity described in the sibling crates'
//! `batch.rs` docs. That text is held in `pending_head_text` until the
//! field's closing tag is unambiguously seen, then dispatched as one
//! `HeadField` event.

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::ast::{Diagnostic, Span};
use crate::events::OwnedEvent;

/// Chunk-driven OPML parser that returns the full AST on `finish()`.
///
/// Accumulates bytes and calls [`crate::parse::parse`] once at the end —
/// for the AST-building use case there's no way to avoid holding the whole
/// document anyway (the AST *is* the whole document). Callers who need
/// bounded memory should use [`StreamingParser`] instead.
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

    pub fn finish(self) -> (crate::ast::OpmlDoc, Vec<Diagnostic>) {
        crate::parse::parse(&self.buf)
    }
}

/// Handler trait for streaming OPML events. Implemented automatically for
/// any `FnMut(OwnedEvent)`.
pub trait Handler {
    fn handle(&mut self, event: OwnedEvent);
}

impl<F: FnMut(OwnedEvent)> Handler for F {
    fn handle(&mut self, event: OwnedEvent) {
        self(event);
    }
}

/// Chunked streaming OPML parser that delivers events to a [`Handler`] as
/// soon as they are provably complete.
pub struct StreamingParser<H: Handler> {
    handler: H,
    pending: Vec<u8>,
    diagnostics: Vec<Diagnostic>,
    /// Names of currently-open elements (generic; XML well-nestedness makes
    /// this sufficient to validate End tags without a fresh reader
    /// remembering earlier Start tags — see the module docs on the sibling
    /// XML crates' identical pattern).
    open_stack: Vec<String>,
    in_opml: bool,
    in_head: bool,
    in_body: bool,
    outline_depth: u32,
    /// Local element name of a `<head>` child currently being read, and its
    /// accumulated text so far.
    pending_head_field: Option<(String, String)>,
    failed: bool,
}

impl<H: Handler> StreamingParser<H> {
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            pending: Vec::new(),
            diagnostics: Vec::new(),
            open_stack: Vec::new(),
            in_opml: false,
            in_head: false,
            in_body: false,
            outline_depth: 0,
            pending_head_field: None,
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
            reader.config_mut().trim_text(true);
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
                    if let Some((_, acc)) = &mut self.pending_head_field {
                        let content = t
                            .decode()
                            .map(|c| c.into_owned())
                            .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                        acc.push_str(&content);
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
                    self.open_stack.push(name.clone());
                    self.pending.drain(0..consumed);
                    match name.as_str() {
                        "opml" => {
                            self.in_opml = true;
                            let version = attr_value(&e, "version").unwrap_or_else(|| "2.0".into());
                            self.handler.handle(OwnedEvent::StartOpml {
                                version: version.into(),
                            });
                        }
                        "head" if !self.in_head => {
                            self.in_head = true;
                            self.handler.handle(OwnedEvent::StartHead);
                        }
                        "body" if !self.in_body => {
                            self.in_body = true;
                            self.handler.handle(OwnedEvent::StartBody);
                        }
                        "outline" if self.in_body => {
                            self.outline_depth += 1;
                            let attrs = read_attrs_owned(&e);
                            self.handler.handle(OwnedEvent::StartOutline { attrs });
                        }
                        other if self.in_head => {
                            self.pending_head_field = Some((other.to_string(), String::new()));
                        }
                        _ => {}
                    }
                }
                Ok(XmlEvent::Empty(e)) => {
                    let consumed = reader.buffer_position() as usize;
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                    self.pending.drain(0..consumed);
                    match name.as_str() {
                        "outline" if self.in_body => {
                            let attrs = read_attrs_owned(&e);
                            self.handler.handle(OwnedEvent::EmptyOutline { attrs });
                        }
                        other if self.in_head => {
                            self.handler.handle(OwnedEvent::HeadField {
                                name: other.to_string().into(),
                                value: String::new().into(),
                            });
                        }
                        _ => {}
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
                    match name.as_str() {
                        "opml" => {
                            self.in_opml = false;
                            self.handler.handle(OwnedEvent::EndOpml);
                        }
                        "head" => {
                            self.in_head = false;
                            self.handler.handle(OwnedEvent::EndHead);
                        }
                        "body" => {
                            self.in_body = false;
                            self.handler.handle(OwnedEvent::EndBody);
                        }
                        "outline" => {
                            if self.outline_depth > 0 {
                                self.outline_depth -= 1;
                            }
                            self.handler.handle(OwnedEvent::EndOutline);
                        }
                        _ => {
                            if let Some((field_name, text)) = self.pending_head_field.take()
                                && field_name == name
                            {
                                self.handler.handle(OwnedEvent::HeadField {
                                    name: field_name.into(),
                                    value: text.into(),
                                });
                            }
                        }
                    }
                }
                Ok(_) => {
                    // Comments/PIs/CData/DocType: not modeled; drop the token.
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
                    // Otherwise: assume the error is a token split across the
                    // chunk boundary and wait for more input.
                    return;
                }
            }
        }
    }

    fn close_out(&mut self) {
        while self.outline_depth > 0 {
            self.outline_depth -= 1;
            self.diagnostics.push(Diagnostic {
                message: "unclosed element <outline>".to_string(),
                span: Span::NONE,
            });
            self.handler.handle(OwnedEvent::EndOutline);
        }
        if self.in_head {
            self.in_head = false;
            self.handler.handle(OwnedEvent::EndHead);
        }
        if self.in_body {
            self.in_body = false;
            self.handler.handle(OwnedEvent::EndBody);
        }
        if self.in_opml {
            self.in_opml = false;
            self.handler.handle(OwnedEvent::EndOpml);
        }
    }
}

fn attr_value(e: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        if a.key.local_name().as_ref() == name.as_bytes() {
            Some(
                a.unescape_value()
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&a.value).into_owned()),
            )
        } else {
            None
        }
    })
}

fn read_attrs_owned(
    e: &quick_xml::events::BytesStart<'_>,
) -> Vec<(String, std::borrow::Cow<'static, str>)> {
    let mut attrs = Vec::new();
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .unescape_value()
            .map(|v| v.into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned());
        attrs.push((key, std::borrow::Cow::Owned(value)));
    }
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_parser_finishes_across_chunks() {
        let mut p = BatchParser::new();
        p.feed(b"<opml version=\"2.0\"><head><title>T");
        p.feed(b"est</title></head><body><outline text=\"X\"/></body></opml>");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.head.title.as_deref(), Some("Test"));
        assert_eq!(doc.body.outlines.len(), 1);
    }

    #[test]
    fn streaming_parser_delivers_events_incrementally() {
        let mut events = Vec::new();
        let mut p = StreamingParser::new(|e| events.push(e));
        p.feed(b"<opml version=\"2.0\"><head><title>Hi");
        p.feed(b"</title></head><body><outline text=\"A\"/></body></opml>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(events.iter().any(
            |e| matches!(e, OwnedEvent::HeadField { name, value } if name == "title" && value == "Hi")
        ));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::EmptyOutline { .. }))
        );
    }

    #[test]
    fn streaming_parser_splits_tag_across_chunks() {
        let mut events = Vec::new();
        let mut p = StreamingParser::new(|e| events.push(e));
        p.feed(b"<opml version=\"2.0\"><body><out");
        p.feed(b"line text=\"A\"/></body></opml>");
        let diags = p.finish();
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::EmptyOutline { .. }))
        );
    }
}
