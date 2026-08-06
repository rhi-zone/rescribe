//! `IncrementalSink`: a custom `html5ever::TreeSink` that emits
//! [`crate::events::Event`]s as the tokenizer/tree-builder produce them,
//! instead of building a full tree first.
//!
//! # Design
//!
//! html5ever's `TreeSink::Handle` must be `Clone`; we use [`NodeId`] (a
//! plain counter) as the handle. A node's id is assigned the moment the
//! sink creates the node object (`create_element`/`create_comment`/
//! `create_pi`, or inline for text — see below) — before it is necessarily
//! attached anywhere.
//!
//! The key design choice: **a node's content event (`StartElement`/`Text`/
//! `Comment`/`Raw`/`Doctype`) is emitted the first time it is attached to a
//! parent (`append`/`append_before_sibling`/`append_based_on_parent_node`),
//! not when it is created.** This absorbs foster parenting for free — by
//! the time html5ever calls `append_based_on_parent_node`, it has already
//! decided the correct final location, so the first-attach event is simply
//! emitted there instead of at the (irrelevant) `create_*` point.
//!
//! A *second* attach of a node that is already attached is, by definition,
//! a structural correction — html5ever only does this from the adoption
//! agency algorithm, always targeting a node still on the stack of open
//! elements (see `crate::events` module docs for the verified call-site
//! trace). That second attach is emitted as [`Event::NodeReparented`]
//! instead of a duplicate content event.
//!
//! `TreeSink::reparent_children` and a standalone `TreeSink::remove_from_parent`
//! (with no follow-up attach — the legacy `<frameset>` reset) map to
//! [`Event::ChildrenReparented`] and [`Event::NodeDetached`] respectively.
//!
//! Adjacent text merging (HTML5 merges sibling text nodes) is replicated by
//! checking, at each text insertion, whether the sibling immediately before
//! the insertion point is a text node created by this sink; if so the new
//! chunk is emitted as [`Event::TextAppended`] against that node's existing
//! id instead of allocating a new one — this keeps `collect_doc`'s
//! reconstruction identical in shape to `parse()`'s (which uses html5ever's
//! own RcDom-style merging).
//!
//! # Memory bound
//!
//! `children_of`/`parent_of` — the structural bookkeeping that exists only
//! to resolve insertion points and corrections — are dropped for a node
//! the moment it pops (`TreeSink::pop`), since (per the same verified
//! trace) nothing ever targets a popped node as an insertion or correction
//! target again. The `nodes` record itself (just the node's kind and tag
//! name — no children, no accumulated text beyond one still-open run) has
//! to outlive `pop`: html5ever calls `elem_name` on a node immediately
//! after popping it in some cases (e.g. `assert_named` in
//! `tree_builder/rules.rs`'s `</tr>` handling), so a per-node record is
//! kept for the life of the parse. That is one small fixed-size record per
//! element the document contains — no text content, no accumulated
//! subtree — a world away from the old tree-then-walk implementation
//! holding the fully materialized `Document` (every text run, every
//! attribute, the works) for the whole parse. The *content* bookkeeping
//! (`children_of`) is what's actually bounded by open-ancestor depth (plus
//! breadth for whichever still-open ancestor is currently accumulating
//! children, since `reparent_children` must be able to name that
//! ancestor's entire current child list) rather than the whole document.

use std::cell::{Ref, RefCell};
use std::collections::HashMap;

use html5ever::driver::{ParseOpts, Parser, parse_document};
use html5ever::interface::{ElemName, ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::stream::{TendrilSink, Utf8LossyDecoder};
use html5ever::tendril::{SliceExt, StrTendril};
use html5ever::{Attribute, LocalName, Namespace, QualName};

use crate::ast::is_void_element;
use crate::events::{Event, OwnedEvent};
use crate::ids::NodeId;

/// `TreeSink::ElemName` wrapper. `Ref<'a, QualName>` can't implement the
/// foreign `ElemName` trait directly (orphan rules — both are foreign to
/// this crate), so this newtype does it instead.
#[derive(Debug)]
pub struct ElemNameRef<'a>(Ref<'a, QualName>);

impl ElemName for ElemNameRef<'_> {
    fn ns(&self) -> &Namespace {
        &self.0.ns
    }
    fn local_name(&self) -> &LocalName {
        &self.0.local
    }
}

#[derive(Clone)]
enum Pending {
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
    },
    Comment {
        text: String,
    },
    Pi,
    /// A `<template>` element's content container. Content appended here is
    /// never attached to the real tree — matching the pre-existing
    /// non-streaming `parse.rs` behavior, where `RcDom` also keeps template
    /// contents in a side slot that `convert_children` never walks. This is
    /// a known, pre-existing gap (not introduced by the incremental sink)
    /// and out of scope for this change; see `fixtures/html/COVERAGE.md`.
    TemplateContents,
    /// Marks a text-node id (text nodes are created inline at insertion
    /// time, not via a `create_*` call, so this exists purely so
    /// `is_open_text` can distinguish "a text node" from "an
    /// element/comment/pi whose content hasn't been emitted yet").
    Text,
}

struct NodeRecord {
    qualname: Option<QualName>,
    pending: Option<Pending>,
    attached: bool,
}

/// Custom `TreeSink` that emits [`Event`]s via a callback as html5ever's
/// tokenizer/tree-builder produce them. See the [module docs](self).
pub(crate) struct IncrementalSink<'e> {
    next_id: RefCell<u64>,
    nodes: RefCell<HashMap<NodeId, NodeRecord>>,
    children_of: RefCell<HashMap<NodeId, Vec<NodeId>>>,
    parent_of: RefCell<HashMap<NodeId, NodeId>>,
    template_contents: RefCell<HashMap<NodeId, NodeId>>,
    quirks_mode: RefCell<QuirksMode>,
    errors: RefCell<Vec<String>>,
    emit: RefCell<Box<dyn FnMut(OwnedEvent) + 'e>>,
}

impl<'e> IncrementalSink<'e> {
    fn new(emit: impl FnMut(OwnedEvent) + 'e) -> Self {
        IncrementalSink {
            next_id: RefCell::new(1),
            nodes: RefCell::new(HashMap::new()),
            children_of: RefCell::new(HashMap::new()),
            parent_of: RefCell::new(HashMap::new()),
            template_contents: RefCell::new(HashMap::new()),
            quirks_mode: RefCell::new(QuirksMode::NoQuirks),
            errors: RefCell::new(Vec::new()),
            emit: RefCell::new(Box::new(emit)),
        }
    }

    fn alloc(&self) -> NodeId {
        let mut n = self.next_id.borrow_mut();
        let id = NodeId(*n);
        *n += 1;
        id
    }

    fn record(&self, id: NodeId, qualname: Option<QualName>, pending: Option<Pending>) {
        self.nodes.borrow_mut().insert(
            id,
            NodeRecord {
                qualname,
                pending,
                attached: false,
            },
        );
    }

    /// The sibling immediately before the given insertion point under
    /// `parent`, if any — used for adjacent-text-merge detection.
    fn sibling_before(&self, parent: NodeId, before_sibling: Option<NodeId>) -> Option<NodeId> {
        let children = self.children_of.borrow();
        let list = children.get(&parent)?;
        match before_sibling {
            None => list.last().copied(),
            Some(sib) => {
                let idx = list.iter().position(|&x| x == sib)?;
                if idx == 0 { None } else { Some(list[idx - 1]) }
            }
        }
    }

    fn is_open_text(&self, id: NodeId) -> bool {
        matches!(
            self.nodes.borrow().get(&id),
            Some(NodeRecord {
                pending: Some(Pending::Text),
                ..
            })
        )
    }

    /// Attach `node` under `parent` (optionally before `before_sibling`),
    /// updating sibling-order bookkeeping. Detaches from any prior parent
    /// first (a no-op for a first attach).
    fn place(&self, node: NodeId, parent: NodeId, before_sibling: Option<NodeId>) {
        self.detach_bookkeeping(node);
        let mut children = self.children_of.borrow_mut();
        let siblings = children.entry(parent).or_default();
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
        drop(children);
        self.parent_of.borrow_mut().insert(node, parent);
    }

    fn detach_bookkeeping(&self, node: NodeId) {
        if let Some(old_parent) = self.parent_of.borrow_mut().remove(&node)
            && let Some(siblings) = self.children_of.borrow_mut().get_mut(&old_parent)
        {
            siblings.retain(|&x| x != node);
        }
    }

    fn is_attached(&self, node: NodeId) -> bool {
        self.nodes
            .borrow()
            .get(&node)
            .map(|r| r.attached)
            .unwrap_or(false)
    }

    /// Insert `child` (text or an existing node handle) at the point
    /// described by `parent`/`before_sibling`. This is the single place
    /// that decides: first attach (emit content event), text merge (emit
    /// `TextAppended`), or structural correction (emit `NodeReparented`).
    fn insert_at(&self, parent: NodeId, before_sibling: Option<NodeId>, child: NodeOrText<NodeId>) {
        match child {
            NodeOrText::AppendText(tendril) => {
                let text: String = tendril.chars().collect();
                if let Some(prev) = self.sibling_before(parent, before_sibling)
                    && self.is_open_text(prev)
                {
                    (self.emit.borrow_mut())(Event::TextAppended {
                        node: prev,
                        content: text.into(),
                    });
                    return;
                }
                let id = self.alloc();
                self.record(id, None, Some(Pending::Text));
                self.place(id, parent, before_sibling);
                self.nodes.borrow_mut().get_mut(&id).unwrap().attached = true;
                (self.emit.borrow_mut())(Event::Text {
                    node: id,
                    parent,
                    before_sibling,
                    content: text.into(),
                });
            }
            NodeOrText::AppendNode(id) => {
                if self.is_attached(id) {
                    self.place(id, parent, before_sibling);
                    (self.emit.borrow_mut())(Event::NodeReparented {
                        node: id,
                        new_parent: parent,
                        before_sibling,
                    });
                } else {
                    self.place(id, parent, before_sibling);
                    self.nodes.borrow_mut().get_mut(&id).unwrap().attached = true;
                    self.emit_first_attach(id, parent, before_sibling);
                }
            }
        }
    }

    fn emit_first_attach(&self, id: NodeId, parent: NodeId, before_sibling: Option<NodeId>) {
        let pending = self
            .nodes
            .borrow_mut()
            .get_mut(&id)
            .and_then(|r| r.pending.take());
        match pending {
            Some(Pending::Element { tag, attrs }) => {
                let self_closing = is_void_element(&tag);
                (self.emit.borrow_mut())(Event::StartElement {
                    node: id,
                    parent,
                    before_sibling,
                    tag: tag.into(),
                    attrs,
                    self_closing,
                });
            }
            Some(Pending::Comment { text }) => {
                (self.emit.borrow_mut())(Event::Comment {
                    node: id,
                    parent,
                    before_sibling,
                    content: text.into(),
                });
            }
            Some(Pending::Pi) | Some(Pending::TemplateContents) | Some(Pending::Text) | None => {
                // Processing instructions have no cross-format modeling in
                // the existing AST either (parse.rs maps them to an empty
                // comment) — preserve that, not a new gap. `Text`/`None`
                // are unreachable here in practice (text nodes are emitted
                // inline in `insert_at`, never through this path) but are
                // handled rather than panicking, per the no-guessing /
                // no-silent-drop discipline.
                (self.emit.borrow_mut())(Event::Comment {
                    node: id,
                    parent,
                    before_sibling,
                    content: String::new().into(),
                });
            }
        }
    }
}

impl TreeSink for IncrementalSink<'_> {
    type Handle = NodeId;
    type Output = ();
    type ElemName<'a>
        = ElemNameRef<'a>
    where
        Self: 'a;

    fn finish(self) {}

    fn parse_error(&self, msg: std::borrow::Cow<'static, str>) {
        self.errors.borrow_mut().push(msg.into_owned());
    }

    fn get_document(&self) -> NodeId {
        NodeId::DOCUMENT
    }

    fn elem_name<'a>(&'a self, target: &'a NodeId) -> ElemNameRef<'a> {
        ElemNameRef(Ref::map(self.nodes.borrow(), |m| {
            m.get(target)
                .and_then(|r| r.qualname.as_ref())
                .expect("elem_name called on a non-element handle")
        }))
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> NodeId {
        let id = self.alloc();
        let tag = name.local.to_string();
        let attrs = attrs
            .into_iter()
            .map(|a| (a.name.local.to_string(), a.value.to_string()))
            .collect();
        self.record(id, Some(name), Some(Pending::Element { tag, attrs }));
        if flags.template {
            let contents_id = self.alloc();
            self.record(contents_id, None, Some(Pending::TemplateContents));
            self.template_contents.borrow_mut().insert(id, contents_id);
        }
        id
    }

    fn create_comment(&self, text: StrTendril) -> NodeId {
        let id = self.alloc();
        self.record(
            id,
            None,
            Some(Pending::Comment {
                text: text.chars().collect(),
            }),
        );
        id
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> NodeId {
        let id = self.alloc();
        self.record(id, None, Some(Pending::Pi));
        id
    }

    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        self.insert_at(*parent, None, child);
    }

    fn append_based_on_parent_node(
        &self,
        element: &NodeId,
        prev_element: &NodeId,
        child: NodeOrText<NodeId>,
    ) {
        // html5ever's own TreeBuilder never constructs
        // `InsertionPoint::BeforeSibling` in 0.36.1 (verified: no call site
        // in tree_builder/mod.rs builds that variant) — the only source of
        // "insert before an existing element" semantics is how a TreeSink
        // chooses to implement *this* method, for the foster-parenting
        // case. We implement it directly instead of round-tripping through
        // a separate `append_before_sibling` call: if `element` (the table)
        // currently has a parent, `child` lands immediately before it under
        // that same parent; otherwise `element` never actually made it into
        // the tree and `child` just becomes the last child of
        // `prev_element`, matching RcDom's own fallback.
        // Bound to a `let` statement (not the `if let` condition directly)
        // so the `Ref` borrow guard drops before `insert_at` runs — an
        // `if let`'s condition temporaries are otherwise kept alive for the
        // whole body, and `insert_at` transitively needs `self.parent_of`
        // mutably (via `place`/`detach_bookkeeping`).
        let parent = self.parent_of.borrow().get(element).copied();
        if let Some(parent) = parent {
            self.insert_at(parent, Some(*element), child);
        } else {
            self.insert_at(*prev_element, None, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        let id = self.alloc();
        self.nodes.borrow_mut().insert(
            id,
            NodeRecord {
                qualname: None,
                pending: None,
                attached: true,
            },
        );
        self.place(id, NodeId::DOCUMENT, None);
        (self.emit.borrow_mut())(Event::Doctype {
            node: id,
            name: name.chars().collect::<String>().into(),
            public_id: public_id.chars().collect::<String>().into(),
            system_id: system_id.chars().collect::<String>().into(),
        });
    }

    fn pop(&self, node: &NodeId) {
        // Not called for every node that stops being an open element: see
        // `Event::EndElement`'s doc comment for the verified html5ever
        // adoption-agency code path that removes a node from the open-
        // elements stack directly, bypassing this method entirely.
        let tag = self
            .nodes
            .borrow()
            .get(node)
            .and_then(|r| r.qualname.as_ref())
            .map(|qn| qn.local.to_string());
        if let Some(tag) = tag {
            (self.emit.borrow_mut())(Event::EndElement {
                node: *node,
                tag: tag.into(),
            });
        }
        // Once popped, nothing will ever target this node as an insertion
        // or correction point again (verified call-site trace in the
        // module docs) — free the structural bookkeeping (children order,
        // parent pointer) that exists only to support that. The small
        // `nodes` record (just the tag name) has to stay: html5ever calls
        // `elem_name` on a node immediately after popping it (e.g.
        // `assert_named` in `rules.rs`'s `</tr>` handling), so dropping the
        // qualname here would panic on the very next tree-builder step.
        self.children_of.borrow_mut().remove(node);
        self.parent_of.borrow_mut().remove(node);
    }

    fn get_template_contents(&self, target: &NodeId) -> NodeId {
        *self
            .template_contents
            .borrow()
            .get(target)
            .expect("get_template_contents called on a non-template element")
    }

    fn same_node(&self, x: &NodeId, y: &NodeId) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        *self.quirks_mode.borrow_mut() = mode;
    }

    fn append_before_sibling(&self, sibling: &NodeId, new_node: NodeOrText<NodeId>) {
        // Not called by html5ever's own TreeBuilder in 0.36.1 (see the note
        // in `append_based_on_parent_node`), but implemented faithfully in
        // case a future html5ever version starts constructing
        // `InsertionPoint::BeforeSibling` directly.
        let parent = self
            .parent_of
            .borrow()
            .get(sibling)
            .copied()
            .expect("append_before_sibling called on a node without a parent");
        self.insert_at(parent, Some(*sibling), new_node);
    }

    fn add_attrs_if_missing(&self, target: &NodeId, attrs: Vec<Attribute>) {
        let mut nodes = self.nodes.borrow_mut();
        if let Some(NodeRecord {
            pending: Some(Pending::Element {
                attrs: existing, ..
            }),
            ..
        }) = nodes.get_mut(target)
        {
            for attr in attrs {
                let name = attr.name.local.to_string();
                if !existing.iter().any(|(n, _)| n == &name) {
                    existing.push((name, attr.value.to_string()));
                }
            }
        }
        // If the element was already attached (its StartElement already
        // emitted with its original attribute set), html5ever only calls
        // this for the synthetic `<html>`/`<body>` re-insertion cases where
        // no content has been observed for the target yet in practice; if
        // it ever does fire post-attach there is no attribute-only
        // correction event, since real documents changing already-emitted
        // attributes have never been observed from this call site.
    }

    fn remove_from_parent(&self, target: &NodeId) {
        self.detach_bookkeeping(*target);
        (self.emit.borrow_mut())(Event::NodeDetached { node: *target });
    }

    fn reparent_children(&self, node: &NodeId, new_parent: &NodeId) {
        let moving = self
            .children_of
            .borrow_mut()
            .remove(node)
            .unwrap_or_default();
        {
            let mut parent_of = self.parent_of.borrow_mut();
            let mut children = self.children_of.borrow_mut();
            let dest = children.entry(*new_parent).or_default();
            for child in &moving {
                parent_of.insert(*child, *new_parent);
                dest.push(*child);
            }
        }
        (self.emit.borrow_mut())(Event::ChildrenReparented {
            from: *node,
            to: *new_parent,
        });
    }
}

/// `'static`-lifetime sink handle used by [`crate::batch::StreamingParser`],
/// which (unlike [`IncrementalEventIter`]) must hold the `Parser` across
/// separate `feed()` calls without borrowing a per-call closure.
pub(crate) type IncrementalSinkHandle = IncrementalSink<'static>;

/// Build a `Utf8LossyDecoder<Parser<IncrementalSink>>` whose sink pushes
/// every event it produces onto `queue`, for [`crate::batch::StreamingParser`].
/// Feeding this decoder chunk-by-chunk (via `TendrilSink::process`) drives
/// html5ever's tokenizer/tree-builder incrementally, exactly like
/// [`IncrementalEventIter`] but pushed to a queue the caller drains after
/// every `feed()` instead of pulled lazily from `next()`.
pub(crate) fn new_streaming_decoder(
    queue: std::rc::Rc<RefCell<std::collections::VecDeque<OwnedEvent>>>,
) -> (
    Utf8LossyDecoder<Parser<IncrementalSinkHandle>>,
    std::rc::Rc<RefCell<std::collections::VecDeque<OwnedEvent>>>,
) {
    let q = queue.clone();
    let sink = IncrementalSink::new(move |e| q.borrow_mut().push_back(e));
    let decoder = parse_document(sink, ParseOpts::default()).from_utf8();
    (decoder, queue)
}

/// Pull-based [`crate::events::EventIter`] backing type. Holds the whole
/// input (already fully in memory per the `events()` contract — see
/// `crate::events` module docs) plus a small queue of events already
/// produced. `next()` feeds another chunk into the parser only when the
/// queue is empty, so no more than a few events are ever buffered at once.
pub(crate) struct IncrementalEventIter {
    input: Vec<u8>,
    offset: usize,
    chunk_size: usize,
    queue: std::rc::Rc<RefCell<std::collections::VecDeque<OwnedEvent>>>,
    decoder: Option<Utf8LossyDecoder<Parser<IncrementalSink<'static>>>>,
}

/// Default chunk size for `events()`'s internal incremental feed. Small
/// enough to keep the "only a handful of events queued" property honest,
/// large enough to avoid pathological per-byte tokenizer overhead.
const DEFAULT_CHUNK_SIZE: usize = 4096;

impl IncrementalEventIter {
    pub(crate) fn new(input: &[u8]) -> Self {
        let queue: std::rc::Rc<RefCell<std::collections::VecDeque<OwnedEvent>>> =
            std::rc::Rc::default();
        let q = queue.clone();
        let sink = IncrementalSink::new(move |e| q.borrow_mut().push_back(e));
        let decoder = parse_document(sink, ParseOpts::default()).from_utf8();
        IncrementalEventIter {
            input: input.to_vec(),
            offset: 0,
            chunk_size: DEFAULT_CHUNK_SIZE,
            queue,
            decoder: Some(decoder),
        }
    }

    fn drive_until_event_or_eof(&mut self) {
        while self.queue.borrow().is_empty() {
            let Some(decoder) = self.decoder.as_mut() else {
                break;
            };
            if self.offset < self.input.len() {
                let end = (self.offset + self.chunk_size).min(self.input.len());
                decoder.process(self.input[self.offset..end].to_tendril());
                self.offset = end;
            } else {
                self.decoder.take().unwrap().finish();
                break;
            }
        }
    }
}

impl Iterator for IncrementalEventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        if let Some(ev) = self.queue.borrow_mut().pop_front() {
            return Some(ev);
        }
        self.drive_until_event_or_eof();
        self.queue.borrow_mut().pop_front()
    }
}
