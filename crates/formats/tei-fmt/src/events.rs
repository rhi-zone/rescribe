//! Streaming event types and iterator for TEI/XML documents.
//!
//! Unlike `html-fmt` (which must build the full tree first because HTML5
//! tree construction can rearrange nodes already seen), XML has no such
//! requirement: it is well-nested by construction. `EventIter` therefore
//! wraps `quick_xml::Reader` *directly* and pulls one token at a time from
//! the input slice — it never materializes a `TeiDoc`. This is a true
//! independent implementation of the reader, not a walk over `parse()`'s
//! output, and it is genuinely `O(largest token)` memory beyond the input
//! slice itself.

use std::borrow::Cow;

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;
use xml_entities::{DtdEntities, EntityResolver, Resolution};

/// A streaming event from a TEI/XML document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    Decl {
        version: Cow<'a, str>,
        encoding: Option<Cow<'a, str>>,
        standalone: Option<Cow<'a, str>>,
    },
    Doctype(Cow<'a, str>),
    ProcessingInstruction {
        target: Cow<'a, str>,
        data: Cow<'a, str>,
    },
    StartElement {
        name: Cow<'a, str>,
        attrs: Vec<(String, String)>,
    },
    EndElement {
        name: Cow<'a, str>,
    },
    /// A self-closing element (`<foo/>`) — equivalent to `StartElement`
    /// immediately followed by `EndElement`, delivered as one event so a
    /// streaming writer can emit `<foo/>` rather than `<foo></foo>`.
    EmptyElement {
        name: Cow<'a, str>,
        attrs: Vec<(String, String)>,
    },
    Text(Cow<'a, str>),
    Cdata(Cow<'a, str>),
    Comment(Cow<'a, str>),
    /// An unresolvable named entity reference (`&custom;`); see
    /// [`crate::ast::Node::EntityRef`]. Predefined XML entities and numeric
    /// character references are resolved to a `Text` event instead.
    EntityRef(Cow<'a, str>),
    /// Raw XML content emitted verbatim; see [`crate::ast::Node::Raw`].
    /// Never produced by [`EventIter`] itself.
    Raw(Cow<'a, str>),
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
            Event::Doctype(d) => Event::Doctype(Cow::Owned(d.into_owned())),
            Event::ProcessingInstruction { target, data } => Event::ProcessingInstruction {
                target: Cow::Owned(target.into_owned()),
                data: Cow::Owned(data.into_owned()),
            },
            Event::StartElement { name, attrs } => Event::StartElement {
                name: Cow::Owned(name.into_owned()),
                attrs,
            },
            Event::EndElement { name } => Event::EndElement {
                name: Cow::Owned(name.into_owned()),
            },
            Event::EmptyElement { name, attrs } => Event::EmptyElement {
                name: Cow::Owned(name.into_owned()),
                attrs,
            },
            Event::Text(t) => Event::Text(Cow::Owned(t.into_owned())),
            Event::Cdata(t) => Event::Cdata(Cow::Owned(t.into_owned())),
            Event::Comment(t) => Event::Comment(Cow::Owned(t.into_owned())),
            Event::EntityRef(t) => Event::EntityRef(Cow::Owned(t.into_owned())),
            Event::Raw(t) => Event::Raw(Cow::Owned(t.into_owned())),
        }
    }
}

/// A streaming iterator over TEI/XML events, produced by [`crate::events()`].
///
/// Holds the `quick_xml::Reader` directly: `next()` advances the reader by
/// exactly one token per call. Diagnostics for malformed input are available
/// via [`EventIter::diagnostics`] after iteration completes (an `Err` token
/// stops iteration but does not panic).
pub struct EventIter<'a> {
    reader: Reader<&'a [u8]>,
    buf: Vec<u8>,
    done: bool,
    diagnostics: Vec<crate::ast::Diagnostic>,
    /// Entities declared in this document's own DOCTYPE internal subset (if
    /// any); replaced once a `Doctype` event is produced. See `parse.rs`'s
    /// module docs for why a single forward pass is sufficient.
    entity_resolver: EntityResolver,
    /// Names of currently-open elements, pushed on `StartElement` and popped
    /// on the matching `EndElement` — mirrors `parse.rs`'s `stack: Vec<
    /// ElementFrame>`, but only the name is needed here (there is no tree to
    /// build). `quick_xml::Reader`'s default `check_end_names = true` means
    /// an `Ok(XmlEvent::End(..))` is only ever produced for a tag that
    /// matches the innermost open element, so this stays in sync without
    /// EventIter needing its own mismatch detection; a *mismatched* closing
    /// tag surfaces as `Err` from the reader instead (see the `Eof`/`Err`
    /// arms below). Used to synthesize the same "unclosed element" recovery
    /// `parse()` performs at EOF/error (see parse.rs's post-loop cleanup).
    open_stack: Vec<String>,
    /// A FIFO of events to return before reading more input. Two producers
    /// feed it: (1) one-token lookahead — when `next()` merges a run of
    /// adjacent text-equivalent tokens (`Text`, resolved char/predefined/DTD
    /// entities) into a single `Event::Text`, it inevitably reads one token
    /// *past* the end of that run to discover the run is over, and that
    /// token can't be "unread" from `quick_xml::Reader`, so it's queued here
    /// instead of being read again; (2) synthetic `EndElement` events for
    /// any names left on `open_stack` when `Eof`/`Err` is reached, matching
    /// `parse()`'s auto-close recovery (see `finalize`).
    pending: std::collections::VecDeque<Event<'a>>,
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
            entity_resolver: EntityResolver::new(DtdEntities::empty()),
            open_stack: Vec::new(),
            pending: std::collections::VecDeque::new(),
        }
    }

    /// Called once the reader reaches `Eof` or `Err`: closes out any
    /// still-open elements by pushing an "unclosed element" diagnostic and a
    /// synthetic `EndElement` event per name remaining on `open_stack`
    /// (innermost first), exactly mirroring parse.rs's post-loop cleanup —
    /// so `events()` produces the same recovered structure as
    /// `events_from_doc(&parse())` for malformed/truncated input instead of
    /// just stopping dead. `merged_text` is any in-progress text run that
    /// was about to be flushed (matching parse()'s `flush_text!` call before
    /// its own `break`); it is returned first, ahead of the synthetic
    /// closes, matching parse()'s ordering (text flushed before the
    /// post-loop cleanup runs).
    fn finalize(&mut self, merged_text: Option<String>) -> Option<Event<'a>> {
        self.done = true;
        while let Some(name) = self.open_stack.pop() {
            self.diagnostics.push(crate::ast::Diagnostic {
                severity: crate::ast::Severity::Warning,
                code: "",
                message: format!("unclosed element <{name}>"),
                span: crate::ast::Span::NONE,
            });
            self.pending.push_back(Event::EndElement {
                name: Cow::Owned(name),
            });
        }
        if let Some(text) = merged_text {
            return Some(Event::Text(Cow::Owned(text)));
        }
        self.pending.pop_front()
    }

    /// Diagnostics accumulated so far (populated as iteration proceeds).
    pub fn diagnostics(&self) -> &[crate::ast::Diagnostic] {
        &self.diagnostics
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        // Queued events from a previous call (see `pending`'s doc comment —
        // either stashed lookahead or synthetic closes from `finalize`)
        // always take priority over reading more input, and must be drained
        // even after `self.done` is set (that's exactly how `finalize`'s
        // synthetic closes get delivered one per `next()` call).
        if let Some(ev) = self.pending.pop_front() {
            return Some(ev);
        }
        if self.done {
            return None;
        }

        // Accumulates a run of adjacent text-equivalent tokens (`Text`,
        // resolved char refs, resolved predefined/DTD entity refs) into one
        // merged run, matching `parse()`'s `current_text` coalescing
        // (documented in parse.rs's module doc) instead of emitting one
        // `Text` event per resolved entity.
        let mut merged_text: Option<String> = None;

        macro_rules! push_text {
            ($s:expr) => {
                match &mut merged_text {
                    Some(acc) => acc.push_str($s),
                    None => merged_text = Some($s.to_string()),
                }
            };
        }

        // Return `ev` now if no text run is pending; otherwise stash `ev`
        // as lookahead and return the merged text run first.
        macro_rules! emit_or_stash {
            ($ev:expr) => {{
                let ev = $ev;
                if let Some(text) = merged_text.take() {
                    self.pending.push_back(ev);
                    return Some(Event::Text(Cow::Owned(text)));
                }
                return Some(ev);
            }};
        }

        loop {
            self.buf.clear();
            let pos = self.reader.buffer_position() as usize;
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
                    emit_or_stash!(Event::Decl {
                        version: Cow::Owned(version),
                        encoding: encoding.map(Cow::Owned),
                        standalone: standalone.map(Cow::Owned),
                    });
                }
                Ok(XmlEvent::DocType(dt)) => {
                    let content = String::from_utf8_lossy(dt.as_ref()).into_owned();
                    let (declared, entity_diagnostics) = DtdEntities::parse_doctype(&content);
                    for d in entity_diagnostics {
                        self.diagnostics.push(crate::ast::Diagnostic {
                            severity: crate::ast::Severity::Warning,
                            code: "",
                            message: format!("DOCTYPE internal subset: {d}"),
                            span: crate::ast::Span {
                                start: pos,
                                end: pos,
                            },
                        });
                    }
                    self.entity_resolver = EntityResolver::new(declared);
                    emit_or_stash!(Event::Doctype(Cow::Owned(content)));
                }
                Ok(XmlEvent::PI(pi)) => {
                    let raw = String::from_utf8_lossy(pi.as_ref()).into_owned();
                    let (target, data) = crate::ast::split_pi(&raw);
                    emit_or_stash!(Event::ProcessingInstruction {
                        target: Cow::Owned(target),
                        data: Cow::Owned(data),
                    });
                }
                Ok(XmlEvent::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    let attrs = read_attrs_owned(&e);
                    self.open_stack.push(name.clone());
                    emit_or_stash!(Event::StartElement {
                        name: Cow::Owned(name),
                        attrs,
                    });
                }
                Ok(XmlEvent::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    let attrs = read_attrs_owned(&e);
                    emit_or_stash!(Event::EmptyElement {
                        name: Cow::Owned(name),
                        attrs,
                    });
                }
                Ok(XmlEvent::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    // `check_end_names = true` (the reader's default, set in
                    // `new()`) guarantees quick_xml only ever hands back an
                    // `Ok(End)` for a tag matching the innermost open
                    // element — a genuine mismatch surfaces as `Err` instead
                    // (see the `Err` arm below) — so this always pops the
                    // element `Start` just pushed.
                    self.open_stack.pop();
                    emit_or_stash!(Event::EndElement {
                        name: Cow::Owned(name),
                    });
                }
                Ok(XmlEvent::Text(t)) => {
                    let content = t
                        .decode()
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                    if content.is_empty() {
                        continue;
                    }
                    push_text!(&content);
                }
                Ok(XmlEvent::GeneralRef(r)) => {
                    if let Ok(Some(ch)) = r.resolve_char_ref() {
                        push_text!(ch.encode_utf8(&mut [0u8; 4]));
                        continue;
                    }
                    let name = r
                        .decode()
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(r.as_ref()).into_owned());
                    if let Some(ch) = crate::ast::resolve_predefined_entity(&name) {
                        push_text!(ch.encode_utf8(&mut [0u8; 4]));
                        continue;
                    }
                    match self.entity_resolver.resolve(&name) {
                        Resolution::Resolved { text, .. } => {
                            push_text!(&text);
                        }
                        Resolution::ExternalUnresolved { .. } | Resolution::Unknown => {
                            emit_or_stash!(Event::EntityRef(Cow::Owned(name)));
                        }
                    }
                }
                Ok(XmlEvent::CData(c)) => {
                    emit_or_stash!(Event::Cdata(Cow::Owned(
                        String::from_utf8_lossy(c.as_ref()).into_owned(),
                    )));
                }
                Ok(XmlEvent::Comment(c)) => {
                    emit_or_stash!(Event::Comment(Cow::Owned(
                        String::from_utf8_lossy(c.as_ref()).into_owned(),
                    )));
                }
                Ok(XmlEvent::Eof) => {
                    return self.finalize(merged_text.take());
                }
                Err(e) => {
                    self.diagnostics.push(crate::ast::Diagnostic {
                        severity: crate::ast::Severity::Warning,
                        code: "",
                        message: format!("XML parse error: {e}"),
                        span: crate::ast::Span {
                            start: pos,
                            end: pos,
                        },
                    });
                    return self.finalize(merged_text.take());
                }
            }
        }
    }
}

/// Convert a raw `quick_xml` event into our owned [`Event`], for every
/// variant except `Text` and `Eof` (callers that need chunk-boundary-aware
/// text handling, like [`crate::batch::StreamingParser`], handle `Text`
/// themselves; `Eof` carries no data). Returns `None` for those two.
///
/// `entity_resolver` resolves non-predefined named entities against the
/// document's DOCTYPE-declared entities plus the standard table; this
/// function is stateless, so it's the caller's job to notice a returned
/// `Event::Doctype` and rebuild the resolver it passes on subsequent calls
/// (see [`crate::batch::StreamingParser::drain`]).
pub(crate) fn owned_event_from_xml(
    event: XmlEvent<'_>,
    entity_resolver: &EntityResolver,
) -> Option<OwnedEvent> {
    match event {
        XmlEvent::Decl(decl) => {
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
            Some(Event::Decl {
                version: Cow::Owned(version),
                encoding: encoding.map(Cow::Owned),
                standalone: standalone.map(Cow::Owned),
            })
        }
        XmlEvent::DocType(dt) => Some(Event::Doctype(Cow::Owned(
            String::from_utf8_lossy(dt.as_ref()).into_owned(),
        ))),
        XmlEvent::PI(pi) => {
            let raw = String::from_utf8_lossy(pi.as_ref()).into_owned();
            let (target, data) = crate::ast::split_pi(&raw);
            Some(Event::ProcessingInstruction {
                target: Cow::Owned(target),
                data: Cow::Owned(data),
            })
        }
        XmlEvent::Start(e) => Some(Event::StartElement {
            name: Cow::Owned(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
            attrs: read_attrs_owned(&e),
        }),
        XmlEvent::Empty(e) => Some(Event::EmptyElement {
            name: Cow::Owned(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
            attrs: read_attrs_owned(&e),
        }),
        XmlEvent::End(e) => Some(Event::EndElement {
            name: Cow::Owned(String::from_utf8_lossy(e.name().as_ref()).into_owned()),
        }),
        XmlEvent::CData(c) => Some(Event::Cdata(Cow::Owned(
            String::from_utf8_lossy(c.as_ref()).into_owned(),
        ))),
        XmlEvent::Comment(c) => Some(Event::Comment(Cow::Owned(
            String::from_utf8_lossy(c.as_ref()).into_owned(),
        ))),
        XmlEvent::GeneralRef(r) => {
            if let Ok(Some(ch)) = r.resolve_char_ref() {
                return Some(Event::Text(Cow::Owned(ch.to_string())));
            }
            let name = r
                .decode()
                .map(|c| c.into_owned())
                .unwrap_or_else(|_| String::from_utf8_lossy(r.as_ref()).into_owned());
            if let Some(ch) = crate::ast::resolve_predefined_entity(&name) {
                return Some(Event::Text(Cow::Owned(ch.to_string())));
            }
            match entity_resolver.resolve(&name) {
                Resolution::Resolved { text, .. } => {
                    Some(Event::Text(Cow::Owned(text.into_owned())))
                }
                Resolution::ExternalUnresolved { .. } | Resolution::Unknown => {
                    Some(Event::EntityRef(Cow::Owned(name)))
                }
            }
        }
        XmlEvent::Text(_) | XmlEvent::Eof => None,
    }
}

fn read_attrs_owned(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
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

/// Build an owned event stream from an already-parsed [`crate::ast::TeiDoc`].
///
/// Used by the builder writer and by round-trip tests; not used by
/// [`crate::events()`] itself (which streams directly from bytes).
pub fn events_from_doc(doc: &crate::ast::TeiDoc) -> Vec<OwnedEvent> {
    let mut events = Vec::new();
    if let Some(decl) = &doc.xml_decl {
        events.push(Event::Decl {
            version: Cow::Owned(decl.version.clone()),
            encoding: decl.encoding.clone().map(Cow::Owned),
            standalone: decl.standalone.clone().map(Cow::Owned),
        });
    }
    for node in &doc.nodes {
        walk_node(node, &mut events);
    }
    events
}

fn walk_node(node: &crate::ast::Node, events: &mut Vec<OwnedEvent>) {
    use crate::ast::Node;
    match node {
        Node::Element {
            name,
            attrs,
            children,
            ..
        } => {
            if children.is_empty() {
                events.push(Event::EmptyElement {
                    name: Cow::Owned(name.clone()),
                    attrs: attrs.clone(),
                });
            } else {
                events.push(Event::StartElement {
                    name: Cow::Owned(name.clone()),
                    attrs: attrs.clone(),
                });
                for child in children {
                    walk_node(child, events);
                }
                events.push(Event::EndElement {
                    name: Cow::Owned(name.clone()),
                });
            }
        }
        Node::Text { content, .. } => events.push(Event::Text(Cow::Owned(content.clone()))),
        Node::Cdata { content, .. } => events.push(Event::Cdata(Cow::Owned(content.clone()))),
        Node::Comment { content, .. } => events.push(Event::Comment(Cow::Owned(content.clone()))),
        Node::ProcessingInstruction { target, data, .. } => {
            events.push(Event::ProcessingInstruction {
                target: Cow::Owned(target.clone()),
                data: Cow::Owned(data.clone()),
            })
        }
        Node::Doctype { content, .. } => events.push(Event::Doctype(Cow::Owned(content.clone()))),
        Node::EntityRef { name, .. } => events.push(Event::EntityRef(Cow::Owned(name.clone()))),
        Node::Raw { content, .. } => events.push(Event::Raw(Cow::Owned(content.clone()))),
    }
}

/// Reconstruct a `TeiDoc` from an event stream.
///
/// Used by the streaming writer's AST fallback and for round-trip testing.
/// One in-progress element on the [`collect_doc`] build stack: name,
/// attributes, and children accumulated so far.
type BuildFrame = (String, Vec<(String, String)>, Vec<crate::ast::Node>);

pub fn collect_doc(events: impl IntoIterator<Item = OwnedEvent>) -> crate::ast::TeiDoc {
    use crate::ast::{Node, Span, XmlDecl};

    let mut xml_decl = None;
    let mut roots: Vec<Node> = Vec::new();
    let mut stack: Vec<BuildFrame> = Vec::new();

    fn push(node: Node, stack: &mut [BuildFrame], roots: &mut Vec<Node>) {
        if let Some((_, _, children)) = stack.last_mut() {
            children.push(node);
        } else {
            roots.push(node);
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
            Event::Doctype(content) => push(
                Node::Doctype {
                    content: content.into_owned(),
                    span: Span::NONE,
                },
                &mut stack,
                &mut roots,
            ),
            Event::ProcessingInstruction { target, data } => push(
                Node::ProcessingInstruction {
                    target: target.into_owned(),
                    data: data.into_owned(),
                    span: Span::NONE,
                },
                &mut stack,
                &mut roots,
            ),
            Event::StartElement { name, attrs } => {
                stack.push((name.into_owned(), attrs, Vec::new()));
            }
            Event::EmptyElement { name, attrs } => push(
                Node::Element {
                    name: name.into_owned(),
                    attrs,
                    children: Vec::new(),
                    span: Span::NONE,
                },
                &mut stack,
                &mut roots,
            ),
            Event::EndElement { .. } => {
                if let Some((name, attrs, children)) = stack.pop() {
                    push(
                        Node::Element {
                            name,
                            attrs,
                            children,
                            span: Span::NONE,
                        },
                        &mut stack,
                        &mut roots,
                    );
                }
            }
            Event::Text(t) => push(
                Node::Text {
                    content: t.into_owned(),
                    span: Span::NONE,
                },
                &mut stack,
                &mut roots,
            ),
            Event::Cdata(t) => push(
                Node::Cdata {
                    content: t.into_owned(),
                    span: Span::NONE,
                },
                &mut stack,
                &mut roots,
            ),
            Event::Comment(t) => push(
                Node::Comment {
                    content: t.into_owned(),
                    span: Span::NONE,
                },
                &mut stack,
                &mut roots,
            ),
            Event::EntityRef(name) => push(
                Node::EntityRef {
                    name: name.into_owned(),
                    span: Span::NONE,
                },
                &mut stack,
                &mut roots,
            ),
            Event::Raw(content) => push(
                Node::Raw {
                    content: content.into_owned(),
                    span: Span::NONE,
                },
                &mut stack,
                &mut roots,
            ),
        }
    }

    // Close any unclosed elements.
    while let Some((name, attrs, children)) = stack.pop() {
        push(
            Node::Element {
                name,
                attrs,
                children,
                span: Span::NONE,
            },
            &mut stack,
            &mut roots,
        );
    }

    crate::ast::TeiDoc {
        xml_decl,
        nodes: roots,
    }
}
