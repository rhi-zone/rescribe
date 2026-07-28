//! Document type - the root container for content and resources.

use crate::{Node, Properties, Resource, ResourceId, ResourceMap};

/// A document with content and embedded resources.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Document {
    /// Root content node.
    pub content: Node,
    /// Embedded resources (images, fonts, etc.).
    pub resources: ResourceMap,
    /// Document-level metadata.
    pub metadata: Properties,
    /// Source format information (for roundtrip fidelity).
    pub source: Option<SourceInfo>,
}

/// Information about the source format, for better roundtrip fidelity.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceInfo {
    /// Source format identifier (e.g., "markdown", "html", "docx").
    pub format: String,
    /// Format-specific metadata preserved for roundtrip.
    pub metadata: Properties,
}

impl Document {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self {
            content: Node::new("document"),
            resources: ResourceMap::new(),
            metadata: Properties::new(),
            source: None,
        }
    }

    /// Set the root content node.
    pub fn with_content(mut self, content: Node) -> Self {
        self.content = content;
        self
    }

    /// Set document metadata.
    pub fn with_metadata(mut self, metadata: Properties) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set source format info.
    pub fn with_source(mut self, source: SourceInfo) -> Self {
        self.source = Some(source);
        self
    }

    /// Embed a resource and return its ID.
    pub fn embed(&mut self, resource: Resource) -> ResourceId {
        let id = ResourceId::new();
        self.resources.insert(id.clone(), resource);
        id
    }

    /// Get a resource by ID.
    pub fn resource(&self, id: &ResourceId) -> Option<&Resource> {
        self.resources.get(id)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use crate::{Node, PropValue};

    #[test]
    fn document_serializes_full_tree() {
        let mut doc = Document::new();
        doc.content =
            Node::new("document").child(Node::new("paragraph").prop("style:align", "center"));
        doc.metadata.set("title", "Test");
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["content"]["kind"], serde_json::json!("document"));
        assert_eq!(
            json["content"]["children"][0]["kind"],
            serde_json::json!("paragraph")
        );
        assert_eq!(
            json["content"]["children"][0]["props"]["style:align"],
            serde_json::json!("center")
        );
        assert_eq!(json["metadata"]["title"], serde_json::json!("Test"));
    }

    #[test]
    fn node_kind_serializes_as_plain_string_not_wrapped() {
        let node = Node::new("heading").prop("level", PropValue::Int(2));
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["kind"], serde_json::json!("heading"));
        assert_eq!(json["props"]["level"], serde_json::json!(2));
    }
}
