//! OPML reader for rescribe.
//!
//! Thin AST→IR adapter over [`opml_fmt`] (the standalone OPML parser/AST/
//! emitter crate — see its docs for the format grammar). All XML parsing
//! lives in `opml-fmt`; this crate only translates `opml_fmt::OpmlDoc` into
//! rescribe's `Document`. Per CLAUDE.md's adapter-layer rule, no
//! `quick_xml` (or any XML tokenizer) appears in this crate's production
//! code.
//!
//! # Mapping
//!
//! - `<head>` metadata: `title` → `Document::metadata["title"]`, `ownerName`
//!   → `metadata["author"]` (the two rescribe treats as cross-format
//!   metadata), every other head field → a namespaced `opml:` metadata key
//!   (e.g. `opml:date_created`), so nothing from `<head>` is silently
//!   dropped even though only two fields have cross-format meaning.
//! - Each `<outline>` becomes a `paragraph` (no children) or a `list` of
//!   one `list_item` (has children) wrapping a `paragraph` plus a nested
//!   `list` of the same shape — proper nesting, not flattened, so an
//!   outline's depth is preserved exactly.
//! - The paragraph's content is `text` (or, wrapped in a `link`, when
//!   `xmlUrl`/`htmlUrl` is present) — a semantic rendering useful to any
//!   *other* format writer. Separately, **every** OPML attribute on the
//!   outline is raw-preserved verbatim as an `opml:attr:{name}` property on
//!   that same node — the writer reconstructs `Outline::attrs` from these,
//!   not by re-deriving them from the rendered text/link, so round-tripping
//!   through rescribe's IR is exact regardless of what the semantic
//!   rendering does with them.
//!
//! # Example
//!
//! ```
//! use rescribe_read_opml::parse;
//!
//! let opml = r#"<?xml version="1.0"?>
//! <opml version="2.0">
//!   <head><title>Example</title></head>
//!   <body>
//!     <outline text="Item 1"/>
//!     <outline text="Item 2"/>
//!   </body>
//! </opml>"#;
//!
//! let result = parse(opml).unwrap();
//! let doc = result.value;
//! ```

use opml_fmt::{Head, OpmlDoc, Outline};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Severity,
    WarningKind,
};
use rescribe_format_api::Parse as _;
use rescribe_std::{node, prop};

/// Parse OPML text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diagnostics) = OpmlDoc::parse(input.as_bytes());

    let mut warnings: Vec<FidelityWarning> = diagnostics
        .iter()
        .map(|d| {
            FidelityWarning::new(
                Severity::Major,
                WarningKind::FeatureLost("opml-syntax".to_string()),
                format!("OPML parse error: {}", d.message),
            )
        })
        .collect();

    let mut metadata = Properties::new();
    apply_head_metadata(&doc.head, &mut metadata);
    metadata.set("opml:version", doc.version.clone());

    let children: Vec<Node> = doc
        .body
        .outlines
        .iter()
        .map(|o| outline_to_top_node(o, &mut warnings))
        .collect();

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

fn apply_head_metadata(head: &Head, metadata: &mut Properties) {
    if let Some(v) = &head.title {
        metadata.set("title", v.clone());
    }
    if let Some(v) = &head.owner_name {
        metadata.set("author", v.clone());
    }
    macro_rules! raw {
        ($opt:expr, $key:literal) => {
            if let Some(v) = &$opt {
                metadata.set($key, v.clone());
            }
        };
    }
    raw!(head.date_created, "opml:date_created");
    raw!(head.date_modified, "opml:date_modified");
    raw!(head.owner_email, "opml:owner_email");
    raw!(head.owner_id, "opml:owner_id");
    raw!(head.docs, "opml:docs");
    raw!(head.expansion_state, "opml:expansion_state");
    raw!(head.vert_scroll_state, "opml:vert_scroll_state");
    raw!(head.window_top, "opml:window_top");
    raw!(head.window_left, "opml:window_left");
    raw!(head.window_bottom, "opml:window_bottom");
    raw!(head.window_right, "opml:window_right");
    for (name, value) in &head.extra {
        metadata.set(format!("opml:head_extra:{name}"), value.clone());
    }
}

/// Build the node representing a top-level outline: a bare `paragraph` if
/// it has no children, or a `list` containing one `list_item` (so a
/// with-children outline is always structurally a list at every depth,
/// including the top level) if it does.
fn outline_to_top_node(o: &Outline, warnings: &mut [FidelityWarning]) -> Node {
    if o.children.is_empty() {
        build_paragraph(o, warnings)
    } else {
        let item = outline_to_list_item(o, warnings);
        Node::new(node::LIST).prop(prop::ORDERED, false).child(item)
    }
}

fn outline_to_list_item(o: &Outline, warnings: &mut [FidelityWarning]) -> Node {
    let para = build_paragraph(o, warnings);
    if o.children.is_empty() {
        Node::new(node::LIST_ITEM).child(para)
    } else {
        let items: Vec<Node> = o
            .children
            .iter()
            .map(|c| outline_to_list_item(c, warnings))
            .collect();
        let nested_list = Node::new(node::LIST)
            .prop(prop::ORDERED, false)
            .children(items);
        Node::new(node::LIST_ITEM).children(vec![para, nested_list])
    }
}

fn build_paragraph(o: &Outline, _warnings: &mut [FidelityWarning]) -> Node {
    let mut para = Node::new(node::PARAGRAPH);
    para = para.prop("opml:self_closing", o.self_closing);
    for (name, value) in &o.attrs {
        para = para.prop(format!("opml:attr:{name}"), value.clone());
    }

    let display_text = o.text().or_else(|| o.title()).unwrap_or("");
    let url = o.xml_url().or_else(|| o.html_url());

    let text_node = Node::new(node::TEXT).prop(prop::CONTENT, display_text.to_string());
    let content = match url {
        Some(url) => Node::new(node::LINK)
            .prop(prop::URL, url.to_string())
            .child(text_node),
        None => text_node,
    };

    para.child(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_opml() {
        let opml = r#"<?xml version="1.0"?>
<opml version="2.0">
  <head><title>Test</title></head>
  <body>
    <outline text="Item 1"/>
    <outline text="Item 2"/>
  </body>
</opml>"#;

        let result = parse(opml).unwrap();
        let doc = result.value;
        assert_eq!(doc.metadata.get_str("title"), Some("Test"));
        assert_eq!(doc.content.children.len(), 2);
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_nested_opml_preserves_depth() {
        let opml = r#"<?xml version="1.0"?>
<opml version="2.0">
  <head><title>Nested</title></head>
  <body>
    <outline text="Parent">
      <outline text="Child 1"/>
      <outline text="Child 2"/>
    </outline>
    <outline text="Sibling"/>
  </body>
</opml>"#;

        let result = parse(opml).unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 2);
        let parent_list = &doc.content.children[0];
        assert_eq!(parent_list.kind.as_str(), node::LIST);
        let parent_item = &parent_list.children[0];
        assert_eq!(parent_item.kind.as_str(), node::LIST_ITEM);
        // paragraph + nested list of 2 children
        assert_eq!(parent_item.children.len(), 2);
        let nested_list = &parent_item.children[1];
        assert_eq!(nested_list.kind.as_str(), node::LIST);
        assert_eq!(nested_list.children.len(), 2);
        assert_eq!(doc.content.children[1].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_opml_with_links() {
        let opml = r#"<?xml version="1.0"?>
<opml version="2.0">
  <body>
    <outline text="Example" xmlUrl="https://example.com/feed.xml"/>
  </body>
</opml>"#;

        let result = parse(opml).unwrap();
        let doc = result.value;
        let para = &doc.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(
            link.props.get_str(prop::URL),
            Some("https://example.com/feed.xml")
        );
    }

    #[test]
    fn test_unknown_attributes_are_raw_preserved() {
        let opml = r#"<opml version="2.0"><body>
  <outline text="X" isComment="true" appSpecific="v"/>
</body></opml>"#;
        let result = parse(opml).unwrap();
        let para = &result.value.content.children[0];
        assert_eq!(para.props.get_str("opml:attr:isComment"), Some("true"));
        assert_eq!(para.props.get_str("opml:attr:appSpecific"), Some("v"));
    }
}
