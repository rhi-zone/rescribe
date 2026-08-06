//! Streaming event types for HTML documents.
//!
//! # Genuine incremental delivery, and why events carry `parent`/`node` ids
//!
//! HTML5 tree construction can retroactively rearrange nodes that were
//! already inserted (adoption agency reparents/clones formatting elements;
//! foster parenting decides *where* misplaced content lands, sometimes
//! ahead of content already emitted for the element it's foster-parented
//! around). A verified read of html5ever 0.36.1's `TreeSink` call sites
//! (`html5ever/src/tree_builder/{mod,rules}.rs`) shows every one of these
//! retroactive operations (`remove_from_parent`, `reparent_children`,
//! `append_before_sibling`/`append_based_on_parent_node`) targets only
//! nodes still on the stack of open elements or in the active-formatting
//! list — i.e. still-open nodes. Once `TreeSink::pop()` fires for a node,
//! html5ever never touches it again. So the correction scope is bounded by
//! open-ancestor depth (and, for `reparent_children`, the breadth of that
//! ancestor's own children), not the whole document.
//!
//! Because corrections move whole subtrees rather than flipping one flag,
//! events can't rely on implicit "append after the most recent StartElement"
//! stack ordering the way a plain SAX walk would: a foster-parented node's
//! `StartElement` can arrive *after* the `StartElement` of a sibling it must
//! end up positioned *before* in the final tree (see `<table>foo<div>` in
//! the crate's fixtures). So every content event carries its own stable
//! [`NodeId`] plus an explicit `parent` (and optional `before_sibling`)
//! rather than leaving position to arrival order, and the correction
//! variants (`NodeReparented`, `ChildrenReparented`, `NodeDetached`) name
//! the affected ids directly. This is the same "correction event" pattern
//! used by `commonmark-fmt`'s `Event::ListTightnessResolved` and
//! `rtf-fmt`'s `Event::TableOrderResolved`, generalized from a scalar
//! correction to a structural one.
//!
//! [`EventIter`] (produced by `events()`) and `StreamingParser` (in
//! `batch.rs`) are both driven by [`crate::sink::IncrementalSink`], a
//! custom `html5ever::TreeSink` implementation that emits events as the
//! tokenizer/tree-builder produce them — not after a full tree walk.
//!
//! [`collect_doc`] is the reference consumer: it applies every event,
//! including corrections, to reconstruct an [`HtmlDoc`] and is used to
//! verify the incremental path produces the same tree as [`crate::parse`].

use std::borrow::Cow;
use std::collections::HashMap;

use crate::ast::*;
pub use crate::ids::NodeId;

/// A streaming event from an HTML document.
///
/// Every content-producing variant carries the [`NodeId`] assigned to the
/// node it introduces, plus (where the node can be attached anywhere other
/// than "last child of `parent`") an explicit `parent` and optional
/// `before_sibling`. See the [module docs](self) for why.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    Doctype {
        node: NodeId,
        name: Cow<'a, str>,
        public_id: Cow<'a, str>,
        system_id: Cow<'a, str>,
    },
    StartElement {
        node: NodeId,
        parent: NodeId,
        before_sibling: Option<NodeId>,
        tag: Cow<'a, str>,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    /// Fired when html5ever pops `node` off its internal stack of open
    /// elements (`TreeSink::pop`).
    ///
    /// **Not guaranteed for every `StartElement`.** Verified against
    /// html5ever 0.36.1: `TreeBuilder::adoption_agency`'s "no furthest
    /// block" branch (`tree_builder/mod.rs`, step 10 of the inner loop) and
    /// its 13.5/13.6 stack-cleanup steps drop a node from the open-elements
    /// stack via `Vec::truncate`/`Vec::remove` without ever calling
    /// `sink.pop()` — and once dropped, the end-of-input drain in
    /// `TokenSink::end()` cannot pop it either, since it is already gone
    /// from the stack. This is reachable even without an active-formatting
    /// clone: `<table><div>foo</div></table>` never emits `EndElement` for
    /// either `div` or `table`, even though both are explicitly closed in
    /// the source, because closing them while foster-parenting is active
    /// routes through this path. The node's position in the tree is never
    /// in doubt — it was already fixed by its `StartElement`'s `parent`/
    /// `before_sibling` (or a later correction event) — only the "this
    /// node will accept no more children" signal is missing for these
    /// cases. A consumer that needs that signal cannot rely on
    /// `EndElement`; [`collect_doc`], which builds structure purely from
    /// parent/child links, is unaffected.
    EndElement { node: NodeId, tag: Cow<'a, str> },
    Text {
        node: NodeId,
        parent: NodeId,
        before_sibling: Option<NodeId>,
        content: Cow<'a, str>,
    },
    /// More text merged into the still-open run started by a prior `Text`
    /// event at the same `node` (HTML5 merges adjacent text nodes). Bounded:
    /// a consumer just appends `content` to the node it already knows about
    /// — it never needs to buffer the whole run to apply this.
    TextAppended { node: NodeId, content: Cow<'a, str> },
    Comment {
        node: NodeId,
        parent: NodeId,
        before_sibling: Option<NodeId>,
        content: Cow<'a, str>,
    },
    /// Raw HTML content to be emitted verbatim.
    Raw {
        node: NodeId,
        parent: NodeId,
        before_sibling: Option<NodeId>,
        content: Cow<'a, str>,
    },
    /// Correction: `node` (already emitted, still open) is detached from
    /// wherever it is currently attached and reattached under `new_parent`
    /// (optionally before `before_sibling`). Mirrors the
    /// `remove_from_parent` + `append`/`append_based_on_parent_node` pairs
    /// html5ever's adoption agency algorithm uses to move a misnested
    /// formatting element's already-open descendant under a fresh clone.
    NodeReparented {
        node: NodeId,
        new_parent: NodeId,
        before_sibling: Option<NodeId>,
    },
    /// Correction: every child currently attached under `from` moves, in
    /// order, to become children of `to`. Mirrors
    /// `TreeSink::reparent_children`, which adoption agency uses to hand a
    /// formatting element's misnested children over to its clone.
    ChildrenReparented { from: NodeId, to: NodeId },
    /// Correction: `node` is detached from its current parent with no
    /// replacement attachment. Mirrors a standalone
    /// `TreeSink::remove_from_parent` — the one call site in html5ever
    /// 0.36.1 is the legacy `<frameset>` reset, which discards `<body>`
    /// outright. Rare in real documents.
    NodeDetached { node: NodeId },
}

/// Owned event (all `Cow` fields are `Cow::Owned`).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Doctype {
                node,
                name,
                public_id,
                system_id,
            } => Event::Doctype {
                node,
                name: Cow::Owned(name.into_owned()),
                public_id: Cow::Owned(public_id.into_owned()),
                system_id: Cow::Owned(system_id.into_owned()),
            },
            Event::StartElement {
                node,
                parent,
                before_sibling,
                tag,
                attrs,
                self_closing,
            } => Event::StartElement {
                node,
                parent,
                before_sibling,
                tag: Cow::Owned(tag.into_owned()),
                attrs,
                self_closing,
            },
            Event::EndElement { node, tag } => Event::EndElement {
                node,
                tag: Cow::Owned(tag.into_owned()),
            },
            Event::Text {
                node,
                parent,
                before_sibling,
                content,
            } => Event::Text {
                node,
                parent,
                before_sibling,
                content: Cow::Owned(content.into_owned()),
            },
            Event::TextAppended { node, content } => Event::TextAppended {
                node,
                content: Cow::Owned(content.into_owned()),
            },
            Event::Comment {
                node,
                parent,
                before_sibling,
                content,
            } => Event::Comment {
                node,
                parent,
                before_sibling,
                content: Cow::Owned(content.into_owned()),
            },
            Event::Raw {
                node,
                parent,
                before_sibling,
                content,
            } => Event::Raw {
                node,
                parent,
                before_sibling,
                content: Cow::Owned(content.into_owned()),
            },
            Event::NodeReparented {
                node,
                new_parent,
                before_sibling,
            } => Event::NodeReparented {
                node,
                new_parent,
                before_sibling,
            },
            Event::ChildrenReparented { from, to } => Event::ChildrenReparented { from, to },
            Event::NodeDetached { node } => Event::NodeDetached { node },
        }
    }
}

/// Iterator over HTML events, produced by [`crate::events()`].
///
/// Genuinely incremental: this holds the html5ever tokenizer/tree-builder
/// state directly (via [`crate::sink::IncrementalSink`]) plus a small queue
/// of events already produced but not yet consumed. `next()` drains the
/// queue, refilling it by feeding the next slice of input into the parser
/// only when the queue runs dry — it does not parse the whole document up
/// front. Memory is bounded by open-ancestor depth (and open-ancestor
/// breadth for the rare `reparent_children` case — see the module docs),
/// not by the size of the document.
pub struct EventIter(crate::sink::IncrementalEventIter);

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        self.0.next()
    }
}

/// Create an `EventIter` over `input`, driven incrementally.
pub(crate) fn events_from_input(input: &[u8]) -> EventIter {
    EventIter(crate::sink::IncrementalEventIter::new(input))
}

/// Walk a resolved `HtmlDoc` AST and produce the equivalent event sequence
/// (assigning fresh ids and explicit parent links as it goes). A resolved
/// tree never needs correction events — only the incremental sink emits
/// those. Used as a cross-check: [`events_from_input`]'s output, once
/// [`collect_doc`] applies its corrections, must reconstruct the same tree
/// this produces from `parse()`'s own output.
pub fn events_from_doc(doc: &HtmlDoc) -> Vec<OwnedEvent> {
    let mut events = Vec::new();
    let mut next_id = 1u64;
    let mut alloc = || {
        let id = NodeId(next_id);
        next_id += 1;
        id
    };
    walk_nodes(&doc.nodes, NodeId::DOCUMENT, &mut alloc, &mut events);
    events
}

fn walk_nodes(
    nodes: &[Node],
    parent: NodeId,
    alloc: &mut impl FnMut() -> NodeId,
    events: &mut Vec<OwnedEvent>,
) {
    for node in nodes {
        walk_node(node, parent, alloc, events);
    }
}

fn walk_node(
    node: &Node,
    parent: NodeId,
    alloc: &mut impl FnMut() -> NodeId,
    events: &mut Vec<OwnedEvent>,
) {
    match node {
        Node::Doctype {
            name,
            public_id,
            system_id,
            ..
        } => {
            events.push(Event::Doctype {
                node: alloc(),
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
            let id = alloc();
            events.push(Event::StartElement {
                node: id,
                parent,
                before_sibling: None,
                tag: Cow::Owned(tag.clone()),
                attrs: attrs.clone(),
                self_closing: *self_closing,
            });
            if !*self_closing {
                walk_nodes(children, id, alloc, events);
                events.push(Event::EndElement {
                    node: id,
                    tag: Cow::Owned(tag.clone()),
                });
            }
        }
        Node::Text { content, .. } => {
            events.push(Event::Text {
                node: alloc(),
                parent,
                before_sibling: None,
                content: Cow::Owned(content.clone()),
            });
        }
        Node::Comment { content, .. } => {
            events.push(Event::Comment {
                node: alloc(),
                parent,
                before_sibling: None,
                content: Cow::Owned(content.clone()),
            });
        }
        Node::Raw { content, .. } => {
            events.push(Event::Raw {
                node: alloc(),
                parent,
                before_sibling: None,
                content: Cow::Owned(content.clone()),
            });
        }
    }
}

/// Reconstruct an `HtmlDoc` from an event stream, applying corrections.
///
/// This is the reference consumer for the correction-event pattern: it
/// keeps a map from [`NodeId`] to an in-progress node plus a children list
/// per parent, applies `NodeReparented`/`ChildrenReparented`/`NodeDetached`
/// exactly as they arrive, and only assembles the final nested [`Node`]
/// tree once all events have been consumed (since a node's final parent
/// isn't settled until then). Used by the streaming writer and by tests
/// that check the incremental reader produces the same tree as `parse()`.
pub fn collect_doc(events: impl IntoIterator<Item = OwnedEvent>) -> HtmlDoc {
    let mut builder = TreeBuilder::default();
    for event in events {
        builder.apply(event);
    }
    builder.finish()
}

#[derive(Clone)]
enum PendingNode {
    Doctype {
        name: String,
        public_id: String,
        system_id: String,
    },
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    Text {
        content: String,
    },
    Comment {
        content: String,
    },
    Raw {
        content: String,
    },
}

#[derive(Default)]
struct TreeBuilder {
    nodes: HashMap<NodeId, PendingNode>,
    parent_of: HashMap<NodeId, NodeId>,
    children_of: HashMap<NodeId, Vec<NodeId>>,
}

impl TreeBuilder {
    fn attach(&mut self, node: NodeId, parent: NodeId, before_sibling: Option<NodeId>) {
        self.detach(node);
        let siblings = self.children_of.entry(parent).or_default();
        match before_sibling {
            Some(sib) => {
                let idx = siblings
                    .iter()
                    .position(|&x| x == sib)
                    .unwrap_or(siblings.len());
                siblings.insert(idx, node);
            }
            None => siblings.push(node),
        }
        self.parent_of.insert(node, parent);
    }

    fn detach(&mut self, node: NodeId) {
        if let Some(old_parent) = self.parent_of.remove(&node)
            && let Some(siblings) = self.children_of.get_mut(&old_parent)
        {
            siblings.retain(|&x| x != node);
        }
    }

    fn apply(&mut self, event: OwnedEvent) {
        match event {
            Event::Doctype {
                node,
                name,
                public_id,
                system_id,
            } => {
                self.nodes.insert(
                    node,
                    PendingNode::Doctype {
                        name: name.into_owned(),
                        public_id: public_id.into_owned(),
                        system_id: system_id.into_owned(),
                    },
                );
                self.attach(node, NodeId::DOCUMENT, None);
            }
            Event::StartElement {
                node,
                parent,
                before_sibling,
                tag,
                attrs,
                self_closing,
            } => {
                self.nodes.insert(
                    node,
                    PendingNode::Element {
                        tag: tag.into_owned(),
                        attrs,
                        self_closing,
                    },
                );
                self.attach(node, parent, before_sibling);
            }
            Event::EndElement { .. } => {}
            Event::Text {
                node,
                parent,
                before_sibling,
                content,
            } => {
                self.nodes.insert(
                    node,
                    PendingNode::Text {
                        content: content.into_owned(),
                    },
                );
                self.attach(node, parent, before_sibling);
            }
            Event::TextAppended { node, content } => {
                if let Some(PendingNode::Text { content: existing }) = self.nodes.get_mut(&node) {
                    existing.push_str(&content);
                }
            }
            Event::Comment {
                node,
                parent,
                before_sibling,
                content,
            } => {
                self.nodes.insert(
                    node,
                    PendingNode::Comment {
                        content: content.into_owned(),
                    },
                );
                self.attach(node, parent, before_sibling);
            }
            Event::Raw {
                node,
                parent,
                before_sibling,
                content,
            } => {
                self.nodes.insert(
                    node,
                    PendingNode::Raw {
                        content: content.into_owned(),
                    },
                );
                self.attach(node, parent, before_sibling);
            }
            Event::NodeReparented {
                node,
                new_parent,
                before_sibling,
            } => {
                self.attach(node, new_parent, before_sibling);
            }
            Event::ChildrenReparented { from, to } => {
                let moving = self.children_of.remove(&from).unwrap_or_default();
                for child in moving {
                    self.parent_of.insert(child, to);
                    self.children_of.entry(to).or_default().push(child);
                }
            }
            Event::NodeDetached { node } => {
                self.detach(node);
            }
        }
    }

    fn finish(mut self) -> HtmlDoc {
        let roots = self
            .children_of
            .remove(&NodeId::DOCUMENT)
            .unwrap_or_default();
        let nodes = roots.into_iter().map(|id| self.build_node(id)).collect();
        HtmlDoc { nodes }
    }

    fn build_node(&mut self, id: NodeId) -> Node {
        match self.nodes.remove(&id) {
            Some(PendingNode::Doctype {
                name,
                public_id,
                system_id,
            }) => Node::Doctype {
                name,
                public_id,
                system_id,
                span: Span::NONE,
            },
            Some(PendingNode::Element {
                tag,
                attrs,
                self_closing,
            }) => {
                let child_ids = self.children_of.remove(&id).unwrap_or_default();
                let children: Vec<Node> = child_ids
                    .into_iter()
                    .map(|cid| self.build_node(cid))
                    .collect();
                let self_closing = self_closing && children.is_empty();
                Node::Element {
                    tag,
                    attrs,
                    children,
                    self_closing,
                    span: Span::NONE,
                }
            }
            Some(PendingNode::Text { content }) => Node::Text {
                content,
                span: Span::NONE,
            },
            Some(PendingNode::Comment { content }) => Node::Comment {
                content,
                span: Span::NONE,
            },
            Some(PendingNode::Raw { content }) => Node::Raw {
                content,
                span: Span::NONE,
            },
            None => Node::Comment {
                content: String::new(),
                span: Span::NONE,
            },
        }
    }
}
