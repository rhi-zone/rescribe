//! Streaming event types and iterator for HTML documents.
//!
//! `EventIter` walks a parsed `HtmlDoc` AST and yields events in document
//! order. For container elements, a `StartElement`/`EndElement` pair is
//! emitted; void (self-closing) elements emit only `StartElement` with
//! `self_closing: true`.
//!
//! All text fields use `Cow<'a, str>`. Since html5ever always produces
//! owned strings, events from `events()` are always `Cow::Owned`.
//! The `Cow` API is preserved so callers constructing events for the
//! streaming writer can use `Cow::Borrowed` for static strings.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from an HTML document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    Doctype {
        name: Cow<'a, str>,
        public_id: Cow<'a, str>,
        system_id: Cow<'a, str>,
    },
    StartElement {
        tag: Cow<'a, str>,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    EndElement {
        tag: Cow<'a, str>,
    },
    Text(Cow<'a, str>),
    Comment(Cow<'a, str>),
    /// Raw HTML content to be emitted verbatim.
    Raw(Cow<'a, str>),
}

/// Owned event (all `Cow` fields are `Cow::Owned`).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Doctype {
                name,
                public_id,
                system_id,
            } => Event::Doctype {
                name: Cow::Owned(name.into_owned()),
                public_id: Cow::Owned(public_id.into_owned()),
                system_id: Cow::Owned(system_id.into_owned()),
            },
            Event::StartElement {
                tag,
                attrs,
                self_closing,
            } => Event::StartElement {
                tag: Cow::Owned(tag.into_owned()),
                attrs,
                self_closing,
            },
            Event::EndElement { tag } => Event::EndElement {
                tag: Cow::Owned(tag.into_owned()),
            },
            Event::Text(t) => Event::Text(Cow::Owned(t.into_owned())),
            Event::Comment(t) => Event::Comment(Cow::Owned(t.into_owned())),
            Event::Raw(t) => Event::Raw(Cow::Owned(t.into_owned())),
        }
    }
}

/// Iterator over HTML events, produced by [`crate::events()`].
pub struct EventIter(std::vec::IntoIter<OwnedEvent>);

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for EventIter {}

/// Create an `EventIter` from a parsed document.
pub(crate) fn events_from_doc(doc: &HtmlDoc) -> EventIter {
    let mut events = Vec::new();
    walk_nodes(&doc.nodes, &mut events);
    EventIter(events.into_iter())
}

/// Walk AST nodes depth-first, appending events.
fn walk_nodes(nodes: &[Node], events: &mut Vec<OwnedEvent>) {
    for node in nodes {
        walk_node(node, events);
    }
}

fn walk_node(node: &Node, events: &mut Vec<OwnedEvent>) {
    match node {
        Node::Doctype {
            name,
            public_id,
            system_id,
            ..
        } => {
            events.push(Event::Doctype {
                name: Cow::Owned(name.clone()),
                public_id: Cow::Owned(public_id.clone()),
                system_id: Cow::Owned(system_id.clone()),
            });
        }
        Node::Element {
            tag,
            attrs,
            children,
            self_closing,
            ..
        } => {
            events.push(Event::StartElement {
                tag: Cow::Owned(tag.clone()),
                attrs: attrs.clone(),
                self_closing: *self_closing,
            });
            if !*self_closing {
                walk_nodes(children, events);
                events.push(Event::EndElement {
                    tag: Cow::Owned(tag.clone()),
                });
            }
        }
        Node::Text { content, .. } => {
            events.push(Event::Text(Cow::Owned(content.clone())));
        }
        Node::Comment { content, .. } => {
            events.push(Event::Comment(Cow::Owned(content.clone())));
        }
        Node::Raw { content, .. } => {
            events.push(Event::Raw(Cow::Owned(content.clone())));
        }
    }
}

/// Reconstruct an `HtmlDoc` from an event stream.
///
/// Used by the streaming writer to convert events back to AST for
/// emit, and useful for testing round-trip correctness.
pub fn collect_doc(events: impl IntoIterator<Item = OwnedEvent>) -> HtmlDoc {
    let mut stack: Vec<ElementFrame> = Vec::new();
    let mut roots: Vec<Node> = Vec::new();

    for event in events {
        match event {
            Event::Doctype {
                name,
                public_id,
                system_id,
            } => {
                let node = Node::Doctype {
                    name: name.into_owned(),
                    public_id: public_id.into_owned(),
                    system_id: system_id.into_owned(),
                    span: Span::NONE,
                };
                push_node(node, &mut stack, &mut roots);
            }
            Event::StartElement {
                tag,
                attrs,
                self_closing,
            } => {
                if self_closing {
                    let node = Node::Element {
                        tag: tag.into_owned(),
                        attrs,
                        children: Vec::new(),
                        self_closing: true,
                        span: Span::NONE,
                    };
                    push_node(node, &mut stack, &mut roots);
                } else {
                    stack.push(ElementFrame {
                        tag: tag.into_owned(),
                        attrs,
                        children: Vec::new(),
                    });
                }
            }
            Event::EndElement { .. } => {
                if let Some(frame) = stack.pop() {
                    let node = Node::Element {
                        tag: frame.tag,
                        attrs: frame.attrs,
                        children: frame.children,
                        self_closing: false,
                        span: Span::NONE,
                    };
                    push_node(node, &mut stack, &mut roots);
                }
            }
            Event::Text(t) => {
                let node = Node::Text {
                    content: t.into_owned(),
                    span: Span::NONE,
                };
                push_node(node, &mut stack, &mut roots);
            }
            Event::Comment(t) => {
                let node = Node::Comment {
                    content: t.into_owned(),
                    span: Span::NONE,
                };
                push_node(node, &mut stack, &mut roots);
            }
            Event::Raw(t) => {
                let node = Node::Raw {
                    content: t.into_owned(),
                    span: Span::NONE,
                };
                push_node(node, &mut stack, &mut roots);
            }
        }
    }

    // Close any unclosed elements.
    while let Some(frame) = stack.pop() {
        let node = Node::Element {
            tag: frame.tag,
            attrs: frame.attrs,
            children: frame.children,
            self_closing: false,
            span: Span::NONE,
        };
        push_node(node, &mut stack, &mut roots);
    }

    HtmlDoc { nodes: roots }
}

struct ElementFrame {
    tag: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

fn push_node(node: Node, stack: &mut [ElementFrame], roots: &mut Vec<Node>) {
    if let Some(frame) = stack.last_mut() {
        frame.children.push(node);
    } else {
        roots.push(node);
    }
}
