//! Stable node identifiers for the incremental HTML5 tree-builder sink.
//!
//! html5ever's `TreeSink::Handle` must be `Clone`; we use `NodeId` (a plain
//! `u64` counter) as the handle type for [`crate::sink::IncrementalSink`].
//! Every node html5ever's tree builder creates (element, comment, processing
//! instruction, doctype, or merged text run) gets one, assigned the moment
//! the sink creates the node object — before it is necessarily attached
//! anywhere in the tree. IDs are never reused and never change: they are
//! what the correction events (see [`crate::events::Event`]) use to refer
//! back to a node that was already emitted.

use std::fmt;

/// Stable identity for a node produced by the incremental HTML5 sink.
///
/// `NodeId(0)` is reserved for the (never emitted) document root — the
/// implicit parent of top-level nodes such as the doctype and the `<html>`
/// element.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub(crate) u64);

impl NodeId {
    /// The implicit document root. Never appears in an `Event` as the
    /// subject of a `StartElement`/`Text`/etc. — only ever as a `parent`.
    pub const DOCUMENT: NodeId = NodeId(0);

    /// Construct an arbitrary `NodeId` for tests and doctests external to
    /// this crate. Real event streams only ever get ids from
    /// [`crate::events::events_from_doc`] or the incremental reader
    /// (`events()`/`StreamingParser`) — this exists so callers building a
    /// synthetic event stream by hand (as in doctests) don't need crate-
    /// internal access to the `u64`.
    pub fn for_doctest(n: u64) -> NodeId {
        NodeId(n)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}
