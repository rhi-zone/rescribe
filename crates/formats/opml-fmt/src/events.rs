//! Streaming event types and iterator for OPML documents.
//!
//! OPML is well-nested XML (no HTML5-style tree-construction quirks), so
//! `EventIter` wraps `quick_xml::Reader` *directly* and pulls one token at a
//! time from the input slice — it never materializes an `OpmlDoc`. This is a
//! true independent implementation of the reader, not a walk over `parse()`'s
//! output.
//!
//! Unlike a generic-XML event stream, these events carry OPML's own
//! vocabulary (`StartOutline`/`EndOutline`, `HeadField`, ...) rather than
//! bare `StartElement`/`EndElement` — a consumer that only cares about "give
//! me the outline tree as it streams by" (e.g. an incremental feed-list
//! importer) never has to know OPML is XML at all.

use std::borrow::Cow;

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::ast::{Diagnostic, Span};

/// A streaming OPML event.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    Decl {
        version: Cow<'a, str>,
        encoding: Option<Cow<'a, str>>,
        standalone: Option<Cow<'a, str>>,
    },
    /// `<opml version="...">` — start of the document.
    StartOpml {
        version: Cow<'a, str>,
    },
    EndOpml,
    StartHead,
    /// One `<head>` child element, delivered as a single event once its
    /// closing tag is seen (`name` is the local element name — `title`,
    /// `ownerName`, or any element outside the fixed OPML 2.0 set).
    HeadField {
        name: Cow<'a, str>,
        value: Cow<'a, str>,
    },
    EndHead,
    StartBody,
    /// `<outline ...>` with children (or an explicit empty `<outline>
    /// </outline>` pair) — always paired with a later `EndOutline`.
    /// Attribute values are `Cow::Borrowed` when the source had no
    /// character references to unescape, `Cow::Owned` otherwise (same
    /// zero-copy-where-possible rule `quick_xml`'s own `unescape_value()`
    /// follows).
    StartOutline {
        attrs: Vec<(String, Cow<'a, str>)>,
    },
    EndOutline,
    /// A self-closing `<outline .../>` with no children — delivered as one
    /// event (not a `StartOutline`/`EndOutline` pair) so a streaming writer
    /// can reproduce the exact self-closing source form. See
    /// [`crate::ast::Outline::self_closing`].
    EmptyOutline {
        attrs: Vec<(String, Cow<'a, str>)>,
    },
    EndBody,
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
            Event::StartOpml { version } => Event::StartOpml {
                version: Cow::Owned(version.into_owned()),
            },
            Event::EndOpml => Event::EndOpml,
            Event::StartHead => Event::StartHead,
            Event::HeadField { name, value } => Event::HeadField {
                name: Cow::Owned(name.into_owned()),
                value: Cow::Owned(value.into_owned()),
            },
            Event::EndHead => Event::EndHead,
            Event::StartBody => Event::StartBody,
            Event::StartOutline { attrs } => Event::StartOutline {
                attrs: attrs
                    .into_iter()
                    .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
                    .collect(),
            },
            Event::EndOutline => Event::EndOutline,
            Event::EmptyOutline { attrs } => Event::EmptyOutline {
                attrs: attrs
                    .into_iter()
                    .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
                    .collect(),
            },
            Event::EndBody => Event::EndBody,
        }
    }
}

/// A streaming iterator over OPML events, produced by [`crate::events()`].
///
/// Holds the `quick_xml::Reader` directly: `next()` advances the reader by
/// one or more raw XML tokens per call (a `<head>` child element consumes
/// its Start/Text/End run in one `next()` call so the domain event carries
/// its complete text value — this is still O(1) tokens of lookahead, not
/// tree-building).
pub struct EventIter<'a> {
    reader: Reader<&'a [u8]>,
    buf: Vec<u8>,
    done: bool,
    diagnostics: Vec<Diagnostic>,
    in_opml: bool,
    in_head: bool,
    in_body: bool,
    outline_depth: u32,
    pending: std::collections::VecDeque<OwnedEvent>,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        let mut reader = Reader::from_reader(input);
        reader.config_mut().trim_text(true);
        EventIter {
            reader,
            buf: Vec::new(),
            done: false,
            diagnostics: Vec::new(),
            in_opml: false,
            in_head: false,
            in_body: false,
            outline_depth: 0,
            pending: std::collections::VecDeque::new(),
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    fn finalize(&mut self) -> Option<OwnedEvent> {
        self.done = true;
        while self.outline_depth > 0 {
            self.outline_depth -= 1;
            self.diagnostics.push(Diagnostic {
                message: "unclosed element <outline>".to_string(),
                span: Span::NONE,
            });
            self.pending.push_back(Event::EndOutline);
        }
        if self.in_head {
            self.in_head = false;
            self.pending.push_back(Event::EndHead);
        }
        if self.in_body {
            self.in_body = false;
            self.pending.push_back(Event::EndBody);
        }
        if self.in_opml {
            self.in_opml = false;
            self.pending.push_back(Event::EndOpml);
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
                    match name.as_str() {
                        "opml" => {
                            self.in_opml = true;
                            let version =
                                attr_value(&e, "version").unwrap_or_else(|| "2.0".to_string());
                            return Some(Event::StartOpml {
                                version: Cow::Owned(version),
                            });
                        }
                        "head" if !self.in_head => {
                            self.in_head = true;
                            return Some(Event::StartHead);
                        }
                        "body" if !self.in_body => {
                            self.in_body = true;
                            return Some(Event::StartBody);
                        }
                        "outline" if self.in_body => {
                            self.outline_depth += 1;
                            let attrs = read_attrs_owned(&e);
                            return Some(Event::StartOutline { attrs });
                        }
                        other if self.in_head => {
                            // Pull this head child's complete text content in
                            // one extra reader call — bounded lookahead, not
                            // tree building.
                            let field_name = other.to_string();
                            let value = match self.reader.read_text(e.to_end().name()) {
                                Ok(t) => t.into_owned(),
                                Err(err) => {
                                    self.diagnostics.push(Diagnostic {
                                        message: format!(
                                            "XML parse error reading <{field_name}>: {err}"
                                        ),
                                        span: Span::NONE,
                                    });
                                    String::new()
                                }
                            };
                            return Some(Event::HeadField {
                                name: Cow::Owned(field_name),
                                value: Cow::Owned(value),
                            });
                        }
                        _ => continue,
                    }
                }
                Ok(XmlEvent::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                    match name.as_str() {
                        "outline" if self.in_body => {
                            let attrs = read_attrs_owned(&e);
                            return Some(Event::EmptyOutline { attrs });
                        }
                        other if self.in_head => {
                            return Some(Event::HeadField {
                                name: Cow::Owned(other.to_string()),
                                value: Cow::Owned(String::new()),
                            });
                        }
                        _ => continue,
                    }
                }
                Ok(XmlEvent::End(e)) => {
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                    match name.as_str() {
                        "opml" => {
                            self.in_opml = false;
                            return Some(Event::EndOpml);
                        }
                        "head" => {
                            self.in_head = false;
                            return Some(Event::EndHead);
                        }
                        "body" => {
                            self.in_body = false;
                            return Some(Event::EndBody);
                        }
                        "outline" => {
                            if self.outline_depth > 0 {
                                self.outline_depth -= 1;
                            }
                            return Some(Event::EndOutline);
                        }
                        _ => continue,
                    }
                }
                Ok(XmlEvent::Text(_)) => continue,
                Ok(XmlEvent::Eof) => return self.finalize(),
                Ok(_) => continue, // Comments/PIs/CData/DocType: not modeled; see ast.rs docs.
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

fn read_attrs_owned(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, Cow<'static, str>)> {
    let mut attrs = Vec::new();
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .unescape_value()
            .map(|v| Cow::Owned(v.into_owned()))
            .unwrap_or_else(|_| Cow::Owned(String::from_utf8_lossy(&attr.value).into_owned()));
        attrs.push((key, value));
    }
    attrs
}

/// Project an already-parsed [`crate::ast::OpmlDoc`] into the same event
/// shape [`crate::events()`] would stream — used by the streaming writer's
/// builder path and by round-trip/equivalence tests, not by `events()`
/// itself (which streams directly from bytes).
pub fn events_from_doc(doc: &crate::ast::OpmlDoc) -> Vec<OwnedEvent> {
    let mut events = Vec::new();
    if let Some(decl) = &doc.xml_decl {
        events.push(Event::Decl {
            version: Cow::Owned(decl.version.clone()),
            encoding: decl.encoding.clone().map(Cow::Owned),
            standalone: decl.standalone.clone().map(Cow::Owned),
        });
    }
    events.push(Event::StartOpml {
        version: Cow::Owned(doc.version.clone()),
    });
    // See `Head::is_empty`'s doc comment (ast.rs): an empty `Head` is
    // indistinguishable from a source with no `<head>` element at all, so
    // this omits the Start/EndHead pair in that case — matching `emit`
    // (emit.rs) and, in turn, what `events()` actually produces when
    // streaming a headless document from bytes.
    if !doc.head.is_empty() {
        events.push(Event::StartHead);
        push_head_fields(&doc.head, &mut events);
        events.push(Event::EndHead);
    }
    events.push(Event::StartBody);
    for o in &doc.body.outlines {
        walk_outline(o, &mut events);
    }
    events.push(Event::EndBody);
    events.push(Event::EndOpml);
    events
}

fn push_head_fields(head: &crate::ast::Head, events: &mut Vec<OwnedEvent>) {
    macro_rules! field {
        ($opt:expr, $name:literal) => {
            if let Some(v) = &$opt {
                events.push(Event::HeadField {
                    name: Cow::Borrowed($name),
                    value: Cow::Owned(v.clone()),
                });
            }
        };
    }
    field!(head.title, "title");
    field!(head.date_created, "dateCreated");
    field!(head.date_modified, "dateModified");
    field!(head.owner_name, "ownerName");
    field!(head.owner_email, "ownerEmail");
    field!(head.owner_id, "ownerId");
    field!(head.docs, "docs");
    field!(head.expansion_state, "expansionState");
    field!(head.vert_scroll_state, "vertScrollState");
    field!(head.window_top, "windowTop");
    field!(head.window_left, "windowLeft");
    field!(head.window_bottom, "windowBottom");
    field!(head.window_right, "windowRight");
    for (name, value) in &head.extra {
        events.push(Event::HeadField {
            name: Cow::Owned(name.clone()),
            value: Cow::Owned(value.clone()),
        });
    }
}

fn walk_outline(o: &crate::ast::Outline, events: &mut Vec<OwnedEvent>) {
    let attrs = || -> Vec<(String, Cow<'static, str>)> {
        o.attrs
            .iter()
            .map(|(k, v)| (k.clone(), Cow::Owned(v.clone())))
            .collect()
    };
    if o.children.is_empty() && o.self_closing {
        events.push(Event::EmptyOutline { attrs: attrs() });
        return;
    }
    events.push(Event::StartOutline { attrs: attrs() });
    for child in &o.children {
        walk_outline(child, events);
    }
    events.push(Event::EndOutline);
}

/// Reconstruct an [`crate::ast::OpmlDoc`] from an event stream. Used by the
/// streaming writer's AST fallback and round-trip tests.
pub fn collect_doc(events: impl IntoIterator<Item = OwnedEvent>) -> crate::ast::OpmlDoc {
    use crate::ast::{Body, Head, OpmlDoc, Outline, Span};

    let mut xml_decl = None;
    let mut version = "2.0".to_string();
    let mut head = Head::default();
    let mut body = Body::default();
    type StackFrame = (Vec<(String, String)>, Vec<Outline>);
    let mut outline_stack: Vec<StackFrame> = Vec::new();

    fn assign(head: &mut Head, name: &str, value: String) {
        match name {
            "title" => head.title = Some(value),
            "dateCreated" => head.date_created = Some(value),
            "dateModified" => head.date_modified = Some(value),
            "ownerName" => head.owner_name = Some(value),
            "ownerEmail" => head.owner_email = Some(value),
            "ownerId" => head.owner_id = Some(value),
            "docs" => head.docs = Some(value),
            "expansionState" => head.expansion_state = Some(value),
            "vertScrollState" => head.vert_scroll_state = Some(value),
            "windowTop" => head.window_top = Some(value),
            "windowLeft" => head.window_left = Some(value),
            "windowBottom" => head.window_bottom = Some(value),
            "windowRight" => head.window_right = Some(value),
            other => head.extra.push((other.to_string(), value)),
        }
    }

    for event in events {
        match event {
            Event::Decl {
                version: v,
                encoding,
                standalone,
            } => {
                xml_decl = Some(crate::ast::XmlDecl {
                    version: v.into_owned(),
                    encoding: encoding.map(|e| e.into_owned()),
                    standalone: standalone.map(|s| s.into_owned()),
                });
            }
            Event::StartOpml { version: v } => version = v.into_owned(),
            Event::EndOpml | Event::StartHead | Event::EndHead | Event::StartBody => {}
            Event::HeadField { name, value } => assign(&mut head, &name, value.into_owned()),
            Event::StartOutline { attrs } => {
                let owned: Vec<(String, String)> = attrs
                    .into_iter()
                    .map(|(k, v)| (k, v.into_owned()))
                    .collect();
                outline_stack.push((owned, Vec::new()));
            }
            Event::EndOutline => {
                if let Some((attrs, children)) = outline_stack.pop() {
                    let o = Outline {
                        attrs,
                        children,
                        self_closing: false,
                        span: Span::NONE,
                    };
                    if let Some((_, parent_children)) = outline_stack.last_mut() {
                        parent_children.push(o);
                    } else {
                        body.outlines.push(o);
                    }
                }
            }
            Event::EmptyOutline { attrs } => {
                let owned: Vec<(String, String)> = attrs
                    .into_iter()
                    .map(|(k, v)| (k, v.into_owned()))
                    .collect();
                let o = Outline {
                    attrs: owned,
                    children: Vec::new(),
                    self_closing: true,
                    span: Span::NONE,
                };
                if let Some((_, parent_children)) = outline_stack.last_mut() {
                    parent_children.push(o);
                } else {
                    body.outlines.push(o);
                }
            }
            Event::EndBody => {}
        }
    }

    OpmlDoc {
        xml_decl,
        version,
        head,
        body,
        span: Span::NONE,
    }
}
