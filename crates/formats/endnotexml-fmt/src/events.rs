//! Streaming event types and iterator for EndNote XML documents.
//!
//! EndNote XML is well-nested XML (no HTML5-style tree-construction
//! quirks), so `EventIter` wraps `quick_xml::Reader` *directly* and pulls
//! one token at a time from the input slice — it never materializes an
//! `EndNoteDoc`. This is a true independent implementation of the reader,
//! not a walk over `parse()`'s output.
//!
//! Every known EndNote container (`<record>`, `<contributors>`, an
//! author-role list, `<titles>`, `<periodical>`, `<urls>`, a url-role list,
//! `<keywords>`, `<dates>`, `<pub-dates>`, `<foreign-keys>`) gets its own
//! `Start*`/`End*` event pair. A leaf field or any element this crate does
//! not otherwise recognize (`rec-number`, `title`, `author`, `key`,
//! `custom1`, ...) becomes a `StartElement { name, attrs }` /
//! `Text`/`StartStyle`/`EndStyle`/nested-`StartElement` / `EndElement`
//! sequence — so `<style>` markup nested inside a field streams
//! incrementally rather than being buffered into one aggregate value, and
//! any element name this crate doesn't know by name still round-trips
//! (see `ast.rs`'s `Element`/`Inline::Other`, which this event vocabulary
//! mirrors).

use std::borrow::Cow;
use std::collections::VecDeque;

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::ast::{AuthorRole, Diagnostic, Span, UrlRole};

/// A streaming EndNote XML event.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    Decl {
        version: Cow<'a, str>,
        encoding: Option<Cow<'a, str>>,
        standalone: Option<Cow<'a, str>>,
    },
    /// `<xml>` — start of the document.
    StartDocument,
    EndDocument,
    /// `<record>`.
    /// `<records>`.
    StartRecords,
    EndRecords,
    StartRecord,
    EndRecord,
    StartForeignKeys,
    EndForeignKeys,
    StartContributors,
    EndContributors,
    StartAuthorRole(AuthorRole),
    EndAuthorRole(AuthorRole),
    StartTitles,
    EndTitles,
    StartPeriodical,
    EndPeriodical,
    StartUrls,
    EndUrls,
    StartUrlRole(UrlRole),
    EndUrlRole(UrlRole),
    StartKeywords,
    EndKeywords,
    StartDates,
    EndDates,
    StartPubDates,
    EndPubDates,
    /// A leaf field or any other element, by its exact source tag name
    /// (`"ref-type"`, `"rec-number"`, `"label"`, `"key"`, `"author"`,
    /// `"title"`, `"secondary-title"`, `"tertiary-title"`, `"full-title"`,
    /// `"volume"`, `"number"`, `"pages"`, `"publisher"`, `"pub-location"`,
    /// `"isbn"`, `"issn"`, `"electronic-resource-num"`, `"url"`,
    /// `"abstract"`, `"notes"`, `"keyword"`, `"year"`, `"date"`, or any
    /// other element name EndNote's exporter-dependent schema produces),
    /// plus its attributes (`name` on `ref-type`, `app`/`db-id` on `key`).
    /// Delimited by a matching [`Event::EndElement`].
    StartElement {
        name: Cow<'a, str>,
        attrs: Vec<(String, Cow<'a, str>)>,
    },
    EndElement,
    Text(Cow<'a, str>),
    /// `<style face="...">` — `face` preserved verbatim.
    StartStyle {
        face: Cow<'a, str>,
    },
    EndStyle,
}

/// Owned event (all `Cow` fields are `Cow::Owned`).
pub type OwnedEvent = Event<'static>;

impl Event<'_> {
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Decl {
                version,
                encoding,
                standalone,
            } => Event::Decl {
                version: Cow::Owned(version.into_owned()),
                encoding: encoding.map(|e| Cow::Owned(e.into_owned())),
                standalone: standalone.map(|s| Cow::Owned(s.into_owned())),
            },
            Event::StartDocument => Event::StartDocument,
            Event::EndDocument => Event::EndDocument,
            Event::StartRecords => Event::StartRecords,
            Event::EndRecords => Event::EndRecords,
            Event::StartRecord => Event::StartRecord,
            Event::EndRecord => Event::EndRecord,
            Event::StartForeignKeys => Event::StartForeignKeys,
            Event::EndForeignKeys => Event::EndForeignKeys,
            Event::StartContributors => Event::StartContributors,
            Event::EndContributors => Event::EndContributors,
            Event::StartAuthorRole(r) => Event::StartAuthorRole(r),
            Event::EndAuthorRole(r) => Event::EndAuthorRole(r),
            Event::StartTitles => Event::StartTitles,
            Event::EndTitles => Event::EndTitles,
            Event::StartPeriodical => Event::StartPeriodical,
            Event::EndPeriodical => Event::EndPeriodical,
            Event::StartUrls => Event::StartUrls,
            Event::EndUrls => Event::EndUrls,
            Event::StartUrlRole(r) => Event::StartUrlRole(r),
            Event::EndUrlRole(r) => Event::EndUrlRole(r),
            Event::StartKeywords => Event::StartKeywords,
            Event::EndKeywords => Event::EndKeywords,
            Event::StartDates => Event::StartDates,
            Event::EndDates => Event::EndDates,
            Event::StartPubDates => Event::StartPubDates,
            Event::EndPubDates => Event::EndPubDates,
            Event::StartElement { name, attrs } => Event::StartElement {
                name: Cow::Owned(name.into_owned()),
                attrs: attrs
                    .into_iter()
                    .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
                    .collect(),
            },
            Event::EndElement => Event::EndElement,
            Event::Text(t) => Event::Text(Cow::Owned(t.into_owned())),
            Event::StartStyle { face } => Event::StartStyle {
                face: Cow::Owned(face.into_owned()),
            },
            Event::EndStyle => Event::EndStyle,
        }
    }
}

/// Open-element stack frame: what kind of element we're currently inside,
/// which determines both how a nested `Start`/`Empty` token is dispatched
/// and which domain event (if any) its matching `End` token emits.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Frame {
    /// Before `<xml>`, or any element this crate doesn't recognize at that
    /// nesting point (a stray unknown root/wrapper) — no domain event on
    /// close.
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

/// A streaming iterator over EndNote XML events, produced by
/// [`crate::events()`]. Holds the `quick_xml::Reader` directly.
pub struct EventIter<'a> {
    reader: Reader<&'a [u8]>,
    buf: Vec<u8>,
    done: bool,
    diagnostics: Vec<Diagnostic>,
    stack: Vec<Frame>,
    pending: VecDeque<OwnedEvent>,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        let mut reader = Reader::from_reader(input);
        reader.config_mut().trim_text(false);
        EventIter {
            reader,
            buf: Vec::new(),
            done: false,
            diagnostics: Vec::new(),
            stack: Vec::new(),
            pending: VecDeque::new(),
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Compute the frame a `name` opened underneath `top` pushes, and the
    /// domain event (if any) its Start emits. Shared by the `Start` and
    /// `Empty` handling below (an `Empty` token synthesizes push+pop
    /// immediately).
    fn dispatch(
        top: Option<Frame>,
        name: &str,
        attrs: Vec<(String, Cow<'static, str>)>,
    ) -> (Frame, Option<OwnedEvent>) {
        match top {
            None => {
                if name == "xml" {
                    (Frame::Document, Some(Event::StartDocument))
                } else {
                    (Frame::Opaque, None)
                }
            }
            Some(Frame::Document) => match name {
                "records" => (Frame::Records, Some(Event::StartRecords)),
                // Defensive: a `<record>` directly under `<xml>` with no
                // `<records>` wrapper is not the documented schema, but is
                // still handled rather than silently ignored.
                "record" => (Frame::Record, Some(Event::StartRecord)),
                _ => (Frame::Opaque, None),
            },
            Some(Frame::Records) | Some(Frame::Opaque) => {
                if name == "record" {
                    (Frame::Record, Some(Event::StartRecord))
                } else {
                    (Frame::Opaque, None)
                }
            }
            Some(Frame::Record) => match name {
                "contributors" => (Frame::Contributors, Some(Event::StartContributors)),
                "titles" => (Frame::Titles, Some(Event::StartTitles)),
                "periodical" => (Frame::Periodical, Some(Event::StartPeriodical)),
                "urls" => (Frame::Urls, Some(Event::StartUrls)),
                "foreign-keys" => (Frame::ForeignKeys, Some(Event::StartForeignKeys)),
                "keywords" => (Frame::Keywords, Some(Event::StartKeywords)),
                "dates" => (Frame::Dates, Some(Event::StartDates)),
                _ => (
                    Frame::InlineElement,
                    Some(Event::StartElement {
                        name: Cow::Owned(name.to_string()),
                        attrs,
                    }),
                ),
            },
            Some(Frame::Contributors) => match name {
                "authors" => (
                    Frame::AuthorRole(AuthorRole::Authors),
                    Some(Event::StartAuthorRole(AuthorRole::Authors)),
                ),
                "secondary-authors" => (
                    Frame::AuthorRole(AuthorRole::SecondaryAuthors),
                    Some(Event::StartAuthorRole(AuthorRole::SecondaryAuthors)),
                ),
                "tertiary-authors" => (
                    Frame::AuthorRole(AuthorRole::TertiaryAuthors),
                    Some(Event::StartAuthorRole(AuthorRole::TertiaryAuthors)),
                ),
                "subsidiary-authors" => (
                    Frame::AuthorRole(AuthorRole::SubsidiaryAuthors),
                    Some(Event::StartAuthorRole(AuthorRole::SubsidiaryAuthors)),
                ),
                _ => (
                    Frame::InlineElement,
                    Some(Event::StartElement {
                        name: Cow::Owned(name.to_string()),
                        attrs,
                    }),
                ),
            },
            Some(Frame::Urls) => match name {
                "related-urls" => (
                    Frame::UrlRole(UrlRole::RelatedUrls),
                    Some(Event::StartUrlRole(UrlRole::RelatedUrls)),
                ),
                "pdf-urls" => (
                    Frame::UrlRole(UrlRole::PdfUrls),
                    Some(Event::StartUrlRole(UrlRole::PdfUrls)),
                ),
                _ => (
                    Frame::InlineElement,
                    Some(Event::StartElement {
                        name: Cow::Owned(name.to_string()),
                        attrs,
                    }),
                ),
            },
            Some(Frame::Dates) => match name {
                "pub-dates" => (Frame::PubDates, Some(Event::StartPubDates)),
                _ => (
                    Frame::InlineElement,
                    Some(Event::StartElement {
                        name: Cow::Owned(name.to_string()),
                        attrs,
                    }),
                ),
            },
            Some(Frame::InlineElement) | Some(Frame::InlineStyle) => {
                if name == "style" {
                    let face = attrs
                        .into_iter()
                        .find(|(k, _)| k == "face")
                        .map(|(_, v)| v)
                        .unwrap_or(Cow::Borrowed(""));
                    (Frame::InlineStyle, Some(Event::StartStyle { face }))
                } else {
                    (
                        Frame::InlineElement,
                        Some(Event::StartElement {
                            name: Cow::Owned(name.to_string()),
                            attrs,
                        }),
                    )
                }
            }
            // ForeignKeys / AuthorRole / Titles / Periodical / UrlRole /
            // Keywords / PubDates: every child is a leaf field, identified
            // generically by name (`key`, `author`, `title`,
            // `secondary-title`, `full-title`, `url`, `keyword`, `date`, or
            // any unexpected element).
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
                Some(Event::StartElement {
                    name: Cow::Owned(name.to_string()),
                    attrs,
                }),
            ),
        }
    }

    fn end_event(frame: Frame) -> Option<OwnedEvent> {
        match frame {
            Frame::Opaque => None,
            Frame::Document => Some(Event::EndDocument),
            Frame::Records => Some(Event::EndRecords),
            Frame::Record => Some(Event::EndRecord),
            Frame::ForeignKeys => Some(Event::EndForeignKeys),
            Frame::Contributors => Some(Event::EndContributors),
            Frame::AuthorRole(r) => Some(Event::EndAuthorRole(r)),
            Frame::Titles => Some(Event::EndTitles),
            Frame::Periodical => Some(Event::EndPeriodical),
            Frame::Urls => Some(Event::EndUrls),
            Frame::UrlRole(r) => Some(Event::EndUrlRole(r)),
            Frame::Keywords => Some(Event::EndKeywords),
            Frame::Dates => Some(Event::EndDates),
            Frame::PubDates => Some(Event::EndPubDates),
            Frame::InlineElement => Some(Event::EndElement),
            Frame::InlineStyle => Some(Event::EndStyle),
        }
    }

    fn finalize(&mut self) -> Option<OwnedEvent> {
        self.done = true;
        while let Some(frame) = self.stack.pop() {
            if !matches!(frame, Frame::Opaque) {
                self.diagnostics.push(Diagnostic {
                    message: "unclosed element at end of input".to_string(),
                    span: Span::NONE,
                });
            }
            if let Some(ev) = Self::end_event(frame) {
                self.pending.push_back(ev);
            }
        }
        self.pending.pop_front()
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        if let Some(ev) = self.pending.pop_front() {
            return Some(ev.into_owned());
        }
        if self.done {
            return None;
        }

        loop {
            self.buf.clear();
            match self.reader.read_event_into(&mut self.buf) {
                Ok(XmlEvent::Decl(decl)) => {
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
                    return Some(Event::Decl {
                        version: Cow::Owned(version),
                        encoding: encoding.map(Cow::Owned),
                        standalone: standalone.map(Cow::Owned),
                    });
                }
                Ok(XmlEvent::Start(e)) => {
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                    let attrs = read_attrs_owned(&e);
                    let top = self.stack.last().copied();
                    let (frame, ev) = Self::dispatch(top, &name, attrs);
                    self.stack.push(frame);
                    if let Some(ev) = ev {
                        return Some(ev);
                    }
                    continue;
                }
                Ok(XmlEvent::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                    let attrs = read_attrs_owned(&e);
                    let top = self.stack.last().copied();
                    let (frame, start_ev) = Self::dispatch(top, &name, attrs);
                    if let Some(end_ev) = Self::end_event(frame) {
                        self.pending.push_back(end_ev);
                    }
                    if let Some(ev) = start_ev {
                        return Some(ev);
                    }
                    continue;
                }
                Ok(XmlEvent::End(_)) => {
                    let frame = self.stack.pop().unwrap_or(Frame::Opaque);
                    if let Some(ev) = Self::end_event(frame) {
                        return Some(ev);
                    }
                    continue;
                }
                Ok(XmlEvent::Text(t)) => {
                    let top = self.stack.last().copied();
                    if !matches!(top, Some(Frame::InlineElement) | Some(Frame::InlineStyle)) {
                        continue;
                    }
                    let content = t
                        .decode()
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                    if content.is_empty() {
                        continue;
                    }
                    return Some(Event::Text(Cow::Owned(content)));
                }
                Ok(XmlEvent::CData(t)) => {
                    let top = self.stack.last().copied();
                    if !matches!(top, Some(Frame::InlineElement) | Some(Frame::InlineStyle)) {
                        continue;
                    }
                    let content = String::from_utf8_lossy(t.as_ref()).into_owned();
                    if content.is_empty() {
                        continue;
                    }
                    return Some(Event::Text(Cow::Owned(content)));
                }
                Ok(XmlEvent::Eof) => return self.finalize(),
                Ok(_) => continue,
                Err(e) => {
                    self.diagnostics.push(Diagnostic {
                        message: format!("XML parse error: {e}"),
                        span: Span::NONE,
                    });
                    return self.finalize();
                }
            }
        }
    }
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

// ---------------------------------------------------------------------------
// AST <-> event projections, used as the equivalence oracle by the fixture
// harness and by the streaming writer's AST-fallback path.
// ---------------------------------------------------------------------------

/// Project an already-parsed [`crate::ast::EndNoteDoc`] into the same event
/// shape [`crate::events()`] would stream — used by round-trip/equivalence
/// tests, not by `events()` itself (which streams directly from bytes).
pub fn events_from_doc(doc: &crate::ast::EndNoteDoc) -> Vec<OwnedEvent> {
    let mut events = Vec::new();
    if let Some(decl) = &doc.xml_decl {
        events.push(Event::Decl {
            version: Cow::Owned(decl.version.clone()),
            encoding: decl.encoding.clone().map(Cow::Owned),
            standalone: decl.standalone.clone().map(Cow::Owned),
        });
    }
    events.push(Event::StartDocument);
    // A zero-record document is indistinguishable, once parsed, from a
    // source with no `<records>` element at all — see `emit()`'s matching
    // doc comment. Omitting the pair here keeps this projection consistent
    // with what `events()` produces for such a source.
    if !doc.records.is_empty() {
        events.push(Event::StartRecords);
        for record in &doc.records {
            walk_record(record, &mut events);
        }
        events.push(Event::EndRecords);
    }
    events.push(Event::EndDocument);
    events
}

fn start_element(name: &str, attrs: Vec<(String, String)>) -> OwnedEvent {
    Event::StartElement {
        name: Cow::Owned(name.to_string()),
        attrs: attrs.into_iter().map(|(k, v)| (k, Cow::Owned(v))).collect(),
    }
}

fn push_inline(inline: &[crate::ast::Inline], events: &mut Vec<OwnedEvent>) {
    for item in inline {
        push_inline_one(item, events);
    }
}

fn push_inline_one(item: &crate::ast::Inline, events: &mut Vec<OwnedEvent>) {
    use crate::ast::Inline;
    match item {
        Inline::Text(t) => events.push(Event::Text(Cow::Owned(t.clone()))),
        Inline::Style { face, children } => {
            events.push(Event::StartStyle {
                face: Cow::Owned(face.clone()),
            });
            push_inline(children, events);
            events.push(Event::EndStyle);
        }
        Inline::Other {
            name,
            attrs,
            children,
        } => {
            events.push(start_element(name, attrs.clone()));
            push_inline(children, events);
            events.push(Event::EndElement);
        }
    }
}

/// Emit `StartElement{name}`/inline-content/`EndElement` for a leaf field,
/// only if present (`None` fields are simply omitted, matching what
/// `events()` produces for a source document that never had the tag).
fn push_field(name: &str, content: &Option<Vec<crate::ast::Inline>>, events: &mut Vec<OwnedEvent>) {
    if let Some(inline) = content {
        events.push(start_element(name, Vec::new()));
        push_inline(inline, events);
        events.push(Event::EndElement);
    }
}

fn push_text_field(name: &str, content: &Option<String>, events: &mut Vec<OwnedEvent>) {
    if let Some(text) = content {
        events.push(start_element(name, Vec::new()));
        if !text.is_empty() {
            events.push(Event::Text(Cow::Owned(text.clone())));
        }
        events.push(Event::EndElement);
    }
}

fn push_element(el: &crate::ast::Element, events: &mut Vec<OwnedEvent>) {
    events.push(start_element(&el.name, el.attrs.clone()));
    push_inline(&el.children, events);
    events.push(Event::EndElement);
}

fn walk_record(record: &crate::ast::Record, events: &mut Vec<OwnedEvent>) {
    use crate::ast::AuthorRole;

    events.push(Event::StartRecord);

    let ref_type_attrs = match &record.ref_type.name {
        Some(n) => vec![("name".to_string(), n.clone())],
        None => Vec::new(),
    };
    events.push(start_element("ref-type", ref_type_attrs));
    if !record.ref_type.code.is_empty() {
        events.push(Event::Text(Cow::Owned(record.ref_type.code.clone())));
    }
    events.push(Event::EndElement);

    if let Some(c) = &record.contributors {
        if c.is_empty() {
            events.push(Event::StartContributors);
            events.push(Event::EndContributors);
        } else {
            events.push(Event::StartContributors);
            push_author_role(AuthorRole::Authors, &c.authors, events);
            push_author_role(AuthorRole::SecondaryAuthors, &c.secondary_authors, events);
            push_author_role(AuthorRole::TertiaryAuthors, &c.tertiary_authors, events);
            push_author_role(AuthorRole::SubsidiaryAuthors, &c.subsidiary_authors, events);
            for el in &c.extra {
                push_element(el, events);
            }
            events.push(Event::EndContributors);
        }
    }

    if let Some(t) = &record.titles {
        if t.is_empty() {
            events.push(Event::StartTitles);
            events.push(Event::EndTitles);
        } else {
            events.push(Event::StartTitles);
            push_field("title", &t.title, events);
            push_field("secondary-title", &t.secondary_title, events);
            push_field("tertiary-title", &t.tertiary_title, events);
            for el in &t.extra {
                push_element(el, events);
            }
            events.push(Event::EndTitles);
        }
    }

    if let Some(p) = &record.periodical {
        events.push(Event::StartPeriodical);
        push_field("full-title", &p.full_title, events);
        for el in &p.extra {
            push_element(el, events);
        }
        events.push(Event::EndPeriodical);
    }

    if let Some(d) = &record.dates {
        if d.is_empty() {
            events.push(Event::StartDates);
            events.push(Event::EndDates);
        } else {
            events.push(Event::StartDates);
            push_field("year", &d.year, events);
            if d.pub_date.is_some() {
                events.push(Event::StartPubDates);
                push_field("date", &d.pub_date, events);
                events.push(Event::EndPubDates);
            }
            for el in &d.extra {
                push_element(el, events);
            }
            events.push(Event::EndDates);
        }
    }

    push_field("volume", &record.volume, events);
    push_field("number", &record.number, events);
    push_field("pages", &record.pages, events);
    push_field("publisher", &record.publisher, events);
    push_field("pub-location", &record.pub_location, events);
    push_text_field("isbn", &record.isbn, events);
    push_text_field("issn", &record.issn, events);
    push_text_field(
        "electronic-resource-num",
        &record.electronic_resource_num,
        events,
    );

    if let Some(u) = &record.urls {
        if u.is_empty() {
            events.push(Event::StartUrls);
            events.push(Event::EndUrls);
        } else {
            events.push(Event::StartUrls);
            push_url_role(crate::ast::UrlRole::RelatedUrls, &u.related_urls, events);
            push_url_role(crate::ast::UrlRole::PdfUrls, &u.pdf_urls, events);
            for el in &u.extra {
                push_element(el, events);
            }
            events.push(Event::EndUrls);
        }
    }
    push_text_field("url", &record.bare_url, events);

    push_field("abstract", &record.abstract_, events);
    push_field("notes", &record.notes, events);

    if !record.keywords.is_empty() {
        events.push(Event::StartKeywords);
        for kw in &record.keywords {
            events.push(start_element("keyword", Vec::new()));
            push_inline(kw, events);
            events.push(Event::EndElement);
        }
        events.push(Event::EndKeywords);
    }

    push_text_field("rec-number", &record.rec_number, events);
    push_text_field("label", &record.label, events);

    if let Some(fk) = &record.foreign_keys
        && !(fk.keys.is_empty() && fk.extra.is_empty())
    {
        events.push(Event::StartForeignKeys);
        for key in &fk.keys {
            let mut attrs = Vec::new();
            if let Some(app) = &key.app {
                attrs.push(("app".to_string(), app.clone()));
            }
            if let Some(db_id) = &key.db_id {
                attrs.push(("db-id".to_string(), db_id.clone()));
            }
            events.push(start_element("key", attrs));
            if !key.text.is_empty() {
                events.push(Event::Text(Cow::Owned(key.text.clone())));
            }
            events.push(Event::EndElement);
        }
        for el in &fk.extra {
            push_element(el, events);
        }
        events.push(Event::EndForeignKeys);
    } else if record.foreign_keys.is_some() {
        events.push(Event::StartForeignKeys);
        events.push(Event::EndForeignKeys);
    }

    for el in &record.extra {
        push_element(el, events);
    }

    events.push(Event::EndRecord);
}

fn push_author_role(
    role: crate::ast::AuthorRole,
    people: &[Vec<crate::ast::Inline>],
    events: &mut Vec<OwnedEvent>,
) {
    if people.is_empty() {
        return;
    }
    events.push(Event::StartAuthorRole(role));
    for person in people {
        events.push(start_element("author", Vec::new()));
        push_inline(person, events);
        events.push(Event::EndElement);
    }
    events.push(Event::EndAuthorRole(role));
}

fn push_url_role(role: crate::ast::UrlRole, urls: &[String], events: &mut Vec<OwnedEvent>) {
    if urls.is_empty() {
        return;
    }
    events.push(Event::StartUrlRole(role));
    for url in urls {
        events.push(start_element("url", Vec::new()));
        if !url.is_empty() {
            events.push(Event::Text(Cow::Owned(url.clone())));
        }
        events.push(Event::EndElement);
    }
    events.push(Event::EndUrlRole(role));
}

/// Reconstruct an [`crate::ast::EndNoteDoc`] from an event stream. Used by
/// the streaming writer's AST fallback and round-trip tests.
pub fn collect_doc(events: impl IntoIterator<Item = OwnedEvent>) -> crate::ast::EndNoteDoc {
    use crate::ast::*;

    let mut xml_decl = None;
    let mut records = Vec::new();

    // A small stack of in-progress builders, one frame per open domain
    // container. `InlineBuilder` accumulates `Inline` content (shared by
    // every leaf-field / style frame); the outer frames accumulate their
    // own typed pieces directly.
    enum Builder {
        Record(Box<Record>),
        ForeignKeys(ForeignKeys),
        Contributors(Contributors),
        AuthorRole(AuthorRole, Vec<Vec<Inline>>),
        Titles(Titles),
        Periodical(Periodical),
        Urls(Urls),
        UrlRole(UrlRole, Vec<String>),
        Keywords(Vec<Vec<Inline>>),
        Dates(Dates),
        PubDates(Option<Vec<Inline>>),
        /// A leaf element (`StartElement`) or a `<style>` run: accumulates
        /// `Inline` children plus (for elements) name/attrs.
        Element {
            name: String,
            attrs: Vec<(String, String)>,
            children: Vec<Inline>,
        },
        Style {
            face: String,
            children: Vec<Inline>,
        },
    }

    let mut stack: Vec<Builder> = Vec::new();

    /// Append a completed `Inline` value to whichever builder is now on
    /// top (an `Element`/`Style` frame accumulates it as a child; any
    /// container frame ignores stray inline content, which never happens
    /// for well-formed event sequences).
    fn push_inline_to_parent(stack: &mut [Builder], value: Inline) {
        if let Some(Builder::Element { children, .. } | Builder::Style { children, .. }) =
            stack.last_mut()
        {
            children.push(value);
        }
    }

    for event in events {
        match event {
            Event::Decl {
                version,
                encoding,
                standalone,
            } => {
                xml_decl = Some(XmlDecl {
                    version: version.into_owned(),
                    encoding: encoding.map(|e| e.into_owned()),
                    standalone: standalone.map(|s| s.into_owned()),
                });
            }
            Event::StartDocument | Event::EndDocument | Event::StartRecords | Event::EndRecords => {
            }
            Event::StartRecord => stack.push(Builder::Record(Box::default())),
            Event::EndRecord => {
                if let Some(Builder::Record(rec)) = stack.pop() {
                    records.push(*rec);
                }
            }
            Event::StartForeignKeys => stack.push(Builder::ForeignKeys(ForeignKeys::default())),
            Event::EndForeignKeys => {
                if let Some(Builder::ForeignKeys(fk)) = stack.pop()
                    && let Some(Builder::Record(rec)) = stack.last_mut()
                {
                    rec.foreign_keys = Some(fk);
                }
            }
            Event::StartContributors => stack.push(Builder::Contributors(Contributors::default())),
            Event::EndContributors => {
                if let Some(Builder::Contributors(c)) = stack.pop()
                    && let Some(Builder::Record(rec)) = stack.last_mut()
                {
                    rec.contributors = Some(c);
                }
            }
            Event::StartAuthorRole(r) => stack.push(Builder::AuthorRole(r, Vec::new())),
            Event::EndAuthorRole(_) => {
                if let Some(Builder::AuthorRole(r, people)) = stack.pop()
                    && let Some(Builder::Contributors(c)) = stack.last_mut()
                {
                    match r {
                        AuthorRole::Authors => c.authors = people,
                        AuthorRole::SecondaryAuthors => c.secondary_authors = people,
                        AuthorRole::TertiaryAuthors => c.tertiary_authors = people,
                        AuthorRole::SubsidiaryAuthors => c.subsidiary_authors = people,
                    }
                }
            }
            Event::StartTitles => stack.push(Builder::Titles(Titles::default())),
            Event::EndTitles => {
                if let Some(Builder::Titles(t)) = stack.pop()
                    && let Some(Builder::Record(rec)) = stack.last_mut()
                {
                    rec.titles = Some(t);
                }
            }
            Event::StartPeriodical => stack.push(Builder::Periodical(Periodical::default())),
            Event::EndPeriodical => {
                if let Some(Builder::Periodical(p)) = stack.pop()
                    && let Some(Builder::Record(rec)) = stack.last_mut()
                {
                    rec.periodical = Some(p);
                }
            }
            Event::StartUrls => stack.push(Builder::Urls(Urls::default())),
            Event::EndUrls => {
                if let Some(Builder::Urls(u)) = stack.pop()
                    && let Some(Builder::Record(rec)) = stack.last_mut()
                {
                    rec.urls = Some(u);
                }
            }
            Event::StartUrlRole(r) => stack.push(Builder::UrlRole(r, Vec::new())),
            Event::EndUrlRole(_) => {
                if let Some(Builder::UrlRole(r, urls)) = stack.pop()
                    && let Some(Builder::Urls(u)) = stack.last_mut()
                {
                    match r {
                        UrlRole::RelatedUrls => u.related_urls = urls,
                        UrlRole::PdfUrls => u.pdf_urls = urls,
                    }
                }
            }
            Event::StartKeywords => stack.push(Builder::Keywords(Vec::new())),
            Event::EndKeywords => {
                if let Some(Builder::Keywords(kws)) = stack.pop()
                    && let Some(Builder::Record(rec)) = stack.last_mut()
                {
                    rec.keywords = kws;
                }
            }
            Event::StartDates => stack.push(Builder::Dates(Dates::default())),
            Event::EndDates => {
                if let Some(Builder::Dates(d)) = stack.pop()
                    && let Some(Builder::Record(rec)) = stack.last_mut()
                {
                    rec.dates = Some(d);
                }
            }
            Event::StartPubDates => stack.push(Builder::PubDates(None)),
            Event::EndPubDates => {
                if let Some(Builder::PubDates(date)) = stack.pop()
                    && let Some(Builder::Dates(d)) = stack.last_mut()
                {
                    d.pub_date = date;
                }
            }
            Event::StartElement { name, attrs } => {
                stack.push(Builder::Element {
                    name: name.into_owned(),
                    attrs: attrs
                        .into_iter()
                        .map(|(k, v)| (k, v.into_owned()))
                        .collect(),
                    children: Vec::new(),
                });
            }
            Event::EndElement => {
                if let Some(Builder::Element {
                    name,
                    attrs,
                    children,
                }) = stack.pop()
                {
                    attach_element(&mut stack, name, attrs, children);
                }
            }
            Event::Text(t) => push_inline_to_parent(&mut stack, Inline::Text(t.into_owned())),
            Event::StartStyle { face } => stack.push(Builder::Style {
                face: face.into_owned(),
                children: Vec::new(),
            }),
            Event::EndStyle => {
                if let Some(Builder::Style { face, children }) = stack.pop() {
                    push_inline_to_parent(&mut stack, Inline::Style { face, children });
                }
            }
        }
    }

    /// Route a completed leaf element into its parent frame's typed field,
    /// by name — mirrors `parse.rs`'s dispatch tables, just operating on
    /// already-built values instead of raw XML tokens.
    fn attach_element(
        stack: &mut [Builder],
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<Inline>,
    ) {
        let make_element = || Element {
            name: name.clone(),
            attrs: attrs.clone(),
            children: children.clone(),
        };
        match stack.last_mut() {
            Some(Builder::Record(rec)) => match name.as_str() {
                "ref-type" => {
                    rec.ref_type = RefType {
                        code: crate::parse::flatten_inline_text(&children),
                        name: attrs.into_iter().find(|(k, _)| k == "name").map(|(_, v)| v),
                    };
                }
                "rec-number" => rec.rec_number = Some(crate::parse::flatten_inline_text(&children)),
                "label" => rec.label = Some(crate::parse::flatten_inline_text(&children)),
                "volume" => rec.volume = Some(children),
                "number" => rec.number = Some(children),
                "pages" => rec.pages = Some(children),
                "publisher" => rec.publisher = Some(children),
                "pub-location" => rec.pub_location = Some(children),
                "isbn" => rec.isbn = Some(crate::parse::flatten_inline_text(&children)),
                "issn" => rec.issn = Some(crate::parse::flatten_inline_text(&children)),
                "electronic-resource-num" => {
                    rec.electronic_resource_num =
                        Some(crate::parse::flatten_inline_text(&children));
                }
                "url" => rec.bare_url = Some(crate::parse::flatten_inline_text(&children)),
                "abstract" => rec.abstract_ = Some(children),
                "notes" => rec.notes = Some(children),
                _ => rec.extra.push(make_element()),
            },
            Some(Builder::ForeignKeys(fk)) => {
                if name == "key" {
                    fk.keys.push(ForeignKey {
                        app: attrs
                            .iter()
                            .find(|(k, _)| k == "app")
                            .map(|(_, v)| v.clone()),
                        db_id: attrs
                            .iter()
                            .find(|(k, _)| k == "db-id")
                            .map(|(_, v)| v.clone()),
                        text: crate::parse::flatten_inline_text(&children),
                    });
                } else {
                    fk.extra.push(make_element());
                }
            }
            Some(Builder::AuthorRole(_, people)) => {
                if name == "author" {
                    people.push(children);
                }
                // Any other name under an author-role list is dropped by
                // this reconstruction path only if it never reaches here —
                // in practice `events_from_doc` never emits anything but
                // `author` there, and `Contributors::extra` catches
                // anything nonstandard at the `<contributors>` level
                // instead (see `EndContributors`'s dispatch above and
                // `parse.rs::read_author_role_list`, which routes unknown
                // children the same way).
            }
            Some(Builder::Titles(t)) => match name.as_str() {
                "title" => t.title = Some(children),
                "secondary-title" => t.secondary_title = Some(children),
                "tertiary-title" => t.tertiary_title = Some(children),
                _ => t.extra.push(make_element()),
            },
            Some(Builder::Periodical(p)) => {
                if name == "full-title" {
                    p.full_title = Some(children);
                } else {
                    p.extra.push(make_element());
                }
            }
            Some(Builder::Urls(u)) => u.extra.push(make_element()),
            Some(Builder::UrlRole(_, urls)) => {
                if name == "url" {
                    urls.push(crate::parse::flatten_inline_text(&children));
                }
            }
            Some(Builder::Keywords(kws)) => {
                if name == "keyword" {
                    kws.push(children);
                }
            }
            Some(Builder::Dates(d)) => match name.as_str() {
                "year" => d.year = Some(children),
                _ => d.extra.push(make_element()),
            },
            Some(Builder::PubDates(date)) => {
                if name == "date" && date.is_none() {
                    *date = Some(children);
                }
            }
            Some(Builder::Element {
                children: parent_children,
                ..
            }) => {
                parent_children.push(Inline::Other {
                    name,
                    attrs,
                    children,
                });
            }
            _ => {}
        }
    }

    EndNoteDoc {
        xml_decl,
        records,
        span: Span::NONE,
    }
}
