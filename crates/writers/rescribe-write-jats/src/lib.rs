//! JATS XML writer for rescribe.
//!
//! Translates rescribe's document IR into `jats_fmt::JatsDoc` (the
//! standalone JATS/XML AST from the `jats-fmt` crate) and serializes it via
//! `jats_fmt::emit`. All XML writing lives in `jats-fmt` — this crate is a
//! thin IR↔AST translator only (per CLAUDE.md's "adapter layer must never
//! contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use rescribe_write_jats::emit;
//! use rescribe_core::{Document, Node, Properties};
//!
//! let doc = Document {
//!     content: Node::new("document"),
//!     resources: Default::default(),
//!     metadata: Properties::new(),
//!     source: None,
//! };
//!
//! let result = emit(&doc).unwrap();
//! let xml = String::from_utf8(result.value).unwrap();
//! ```

use jats_fmt::{JatsDoc, Node as JNode, XmlDecl};
use rescribe_core::{ConversionResult, Document, EmitError, Node};
use rescribe_std::{node, prop};

/// Emit a document to JATS XML.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let warnings = Vec::new();

    let mut root_children = Vec::new();
    // Any metadata key ending in `_raw` is a whole-subtree verbatim capture
    // of an unmodeled `<article-meta>`/`<journal-meta>` front-matter
    // element (see `rescribe-read-jats`'s `convert_children`/
    // `extract_metadata` — `{tag}_raw`, e.g. `contrib-group_raw` or
    // `pub-date_raw`). Collected once here so both the "do we even need an
    // `<article-meta>`" check and the splice-back loop below share one
    // scan.
    let mut meta_raw_fields: Vec<(&str, &str)> = doc
        .metadata
        .iter()
        .filter_map(|(key, _)| {
            let tag = key.strip_suffix("_raw")?;
            Some((tag, doc.metadata.get_str(key)?))
        })
        .collect();
    meta_raw_fields.sort_unstable_by_key(|(tag, _)| *tag);
    let title = doc.metadata.get_str("title");
    if title.is_some() || !meta_raw_fields.is_empty() {
        let mut article_meta_children = Vec::new();
        if let Some(title) = title {
            article_meta_children.push(jats_element(
                "title-group",
                vec![],
                vec![jats_element(
                    "article-title",
                    vec![],
                    vec![jats_text(title)],
                )],
            ));
        }
        // Splice back every raw-captured `<article-meta>`/`<journal-meta>`
        // subtree byte-for-byte (see `rescribe-read-jats`'s
        // `convert_children`/`extract_metadata` — any `{tag}_raw` metadata
        // field, e.g. `contrib-group_raw` or `pub-date_raw`). This is
        // lossless where reconstructing the element from its flattened
        // text would not be; sorted by tag for deterministic output since
        // `Properties` iterates in unspecified order.
        for (_, raw) in &meta_raw_fields {
            article_meta_children.push(JNode::Raw {
                content: (*raw).to_string(),
                span: jats_fmt::Span::NONE,
            });
        }
        root_children.push(jats_element(
            "front",
            vec![],
            vec![jats_element("article-meta", vec![], article_meta_children)],
        ));
    }

    let mut body_children = Vec::new();
    for child in &doc.content.children {
        body_children.extend(write_node(child));
    }
    root_children.push(jats_element("body", vec![], body_children));

    let root = JNode::Element {
        name: "article".to_string(),
        attrs: vec![
            (
                "xmlns:xlink".to_string(),
                "http://www.w3.org/1999/xlink".to_string(),
            ),
            ("article-type".to_string(), "research-article".to_string()),
        ],
        children: root_children,
        span: jats_fmt::Span::NONE,
    };

    let doc_ast = JatsDoc {
        xml_decl: Some(XmlDecl {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
        }),
        nodes: vec![root],
    };

    let bytes = jats_fmt::emit(&doc_ast);
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

fn jats_element(name: &str, attrs: Vec<(String, String)>, children: Vec<JNode>) -> JNode {
    JNode::Element {
        name: name.to_string(),
        attrs,
        children,
        span: jats_fmt::Span::NONE,
    }
}

fn jats_text(content: impl Into<String>) -> JNode {
    JNode::Text {
        content: content.into(),
        span: jats_fmt::Span::NONE,
    }
}

/// Build the generic `id` attribute for a node, if the IR node carries the
/// corresponding raw-preserved property (see `rescribe-read-jats`'s
/// `attach_generic_attrs` for the reader side of this round trip).
fn generic_attrs(node: &Node) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    if let Some(id) = node.props.get_str(prop::ID) {
        attrs.push(("id".to_string(), id.to_string()));
    }
    attrs
}

/// Convert one rescribe IR (block-level) node into zero or more JATS AST
/// nodes.
fn write_node(node: &Node) -> Vec<JNode> {
    match node.kind.as_str() {
        node::DOCUMENT => node.children.iter().flat_map(write_node).collect(),

        // A `div` tagged `jats:tag` is a `generic_div` (an unrecognized
        // block-level element raw-preserved by the reader's catch-all, see
        // `rescribe-read-jats::generic_div`) — re-emit its original tag
        // name. Any other `div` (article/sec/abstract/etc, which the
        // reader unwraps regardless of the writer re-wrapping them) just
        // flattens into its children, same as `DOCUMENT`.
        node::DIV => match node.props.get_str("jats:tag") {
            Some(tag) => vec![jats_element(
                tag,
                generic_attrs(node),
                node.children.iter().flat_map(write_node).collect(),
            )],
            None => node.children.iter().flat_map(write_node).collect(),
        },

        node::HEADING => vec![jats_element(
            "sec",
            vec![],
            vec![jats_element(
                "title",
                vec![],
                node.children.iter().flat_map(write_inline).collect(),
            )],
        )],

        node::PARAGRAPH => vec![jats_element(
            "p",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::BLOCKQUOTE => vec![jats_element(
            "disp-quote",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let list_type = if ordered { "order" } else { "bullet" };
            vec![jats_element(
                "list",
                vec![("list-type".to_string(), list_type.to_string())],
                node.children.iter().flat_map(write_node).collect(),
            )]
        }

        node::LIST_ITEM => vec![jats_element(
            "list-item",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::DEFINITION_LIST => {
            let mut entries = Vec::new();
            let mut i = 0;
            while i < node.children.len() {
                let mut entry_children = write_node(&node.children[i]);
                if i + 1 < node.children.len() {
                    entry_children.extend(write_node(&node.children[i + 1]));
                }
                entries.push(jats_element("def-item", vec![], entry_children));
                i += 2;
            }
            vec![jats_element("def-list", vec![], entries)]
        }

        node::DEFINITION_TERM => vec![jats_element(
            "term",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::DEFINITION_DESC => vec![jats_element(
            "def",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::CODE_BLOCK => {
            let mut attrs = Vec::new();
            if let Some(lang) = node.props.get_str(prop::LANGUAGE) {
                attrs.push(("content-type".to_string(), lang.to_string()));
            }
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            vec![jats_element("code", attrs, vec![jats_text(content)])]
        }

        node::TABLE => {
            let has_structure = node.children.iter().any(|c| {
                c.kind.as_str() == node::TABLE_HEAD || c.kind.as_str() == node::TABLE_BODY
            });
            let table_children: Vec<JNode> = if has_structure {
                node.children.iter().flat_map(write_node).collect()
            } else {
                vec![jats_element(
                    "tbody",
                    vec![],
                    node.children.iter().flat_map(write_node).collect(),
                )]
            };
            vec![jats_element(
                "table-wrap",
                vec![],
                vec![jats_element("table", vec![], table_children)],
            )]
        }

        node::TABLE_HEAD => vec![jats_element(
            "thead",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::TABLE_BODY => vec![jats_element(
            "tbody",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::TABLE_ROW => vec![jats_element(
            "tr",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::TABLE_CELL => vec![jats_element(
            "td",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::TABLE_HEADER => vec![jats_element(
            "th",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::FIGURE => vec![jats_element(
            "fig",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::IMAGE => vec![jats_element(
            "graphic",
            node.props
                .get_str(prop::URL)
                .map(|url| vec![("xlink:href".to_string(), url.to_string())])
                .unwrap_or_default(),
            vec![],
        )],

        node::HORIZONTAL_RULE => Vec::new(), // JATS doesn't have HR

        node::FOOTNOTE_DEF => vec![jats_element(
            "fn",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        "math_display" => {
            let mut formula_children = Vec::new();
            if let Some(source) = node.props.get_str("math:source") {
                formula_children.push(jats_element("tex-math", vec![], vec![jats_text(source)]));
            }
            vec![jats_element("disp-formula", vec![], formula_children)]
        }

        // Inline nodes that appear at block level: wrap in a <p>.
        node::TEXT | node::EMPHASIS | node::STRONG | node::CODE | node::LINK => {
            vec![jats_element("p", vec![], write_inline(node))]
        }

        _ => {
            // Unknown block - recurse into children
            node.children.iter().flat_map(write_node).collect()
        }
    }
}

/// Convert one rescribe IR (inline-level) node into zero or more JATS AST
/// nodes.
fn write_inline(node: &Node) -> Vec<JNode> {
    match node.kind.as_str() {
        node::TEXT => match node.props.get_str(prop::CONTENT) {
            Some(content) => vec![jats_text(content)],
            None => Vec::new(),
        },

        node::EMPHASIS => vec![jats_element(
            "italic",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::STRONG => vec![jats_element(
            "bold",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::UNDERLINE => vec![jats_element(
            "underline",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::STRIKEOUT => vec![jats_element(
            "strike",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::CODE => {
            let mut children: Vec<JNode> = node
                .props
                .get_str(prop::CONTENT)
                .map(jats_text)
                .into_iter()
                .collect();
            children.extend(node.children.iter().flat_map(write_inline));
            vec![jats_element("monospace", vec![], children)]
        }

        node::LINK => {
            let mut attrs = Vec::new();
            if let Some(url) = node.props.get_str(prop::URL) {
                attrs.push(("xlink:href".to_string(), url.to_string()));
                attrs.push(("ext-link-type".to_string(), "uri".to_string()));
            }
            vec![jats_element(
                "ext-link",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )]
        }

        node::SUBSCRIPT => vec![jats_element(
            "sub",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::SUPERSCRIPT => vec![jats_element(
            "sup",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::SMALL_CAPS => vec![jats_element(
            "sc",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::LINE_BREAK => vec![jats_element("break", vec![], vec![])],

        node::SOFT_BREAK => vec![jats_text(" ")],

        node::IMAGE => vec![jats_element(
            "inline-graphic",
            node.props
                .get_str(prop::URL)
                .map(|url| vec![("xlink:href".to_string(), url.to_string())])
                .unwrap_or_default(),
            vec![],
        )],

        "math_inline" => {
            let mut formula_children = Vec::new();
            if let Some(source) = node.props.get_str("math:source") {
                formula_children.push(jats_element("tex-math", vec![], vec![jats_text(source)]));
            }
            vec![jats_element("inline-formula", vec![], formula_children)]
        }

        // A raw entity reference preserved by the reader: re-emit verbatim.
        node::RAW_INLINE => match node.props.get_str("jats:entity") {
            Some(name) => vec![JNode::EntityRef {
                name: name.to_string(),
                span: jats_fmt::Span::NONE,
            }],
            None => node.children.iter().flat_map(write_inline).collect(),
        },

        // A `span` tagged `jats:tag` is a `generic_span` (an unrecognized
        // inline-level element raw-preserved by the reader's catch-all, see
        // `rescribe-read-jats::generic_span`) — re-emit its original tag
        // name.
        node::SPAN => match node.props.get_str("jats:tag") {
            Some(tag) => vec![jats_element(
                tag,
                generic_attrs(node),
                node.children.iter().flat_map(write_inline).collect(),
            )],
            None => node.children.iter().flat_map(write_inline).collect(),
        },

        _ => {
            // Unknown inline - recurse
            node.children.iter().flat_map(write_inline).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::Properties;

    #[test]
    fn test_emit_empty() {
        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<article"));
        assert!(xml.contains("</article>"));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = Document {
            content: Node::new(node::DOCUMENT).child(
                Node::new(node::PARAGRAPH)
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, "Hello, world!")),
            ),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<p>Hello, world!</p>"));
    }

    #[test]
    fn test_emit_with_title() {
        let mut metadata = Properties::new();
        metadata.set("title", "Test Article".to_string());

        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata,
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<article-title>Test Article</article-title>"));
    }

    #[test]
    fn test_emit_formatting() {
        let doc = Document {
            content: Node::new(node::DOCUMENT).child(
                Node::new(node::PARAGRAPH)
                    .child(
                        Node::new(node::EMPHASIS)
                            .child(Node::new(node::TEXT).prop(prop::CONTENT, "italic")),
                    )
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, " and "))
                    .child(
                        Node::new(node::STRONG)
                            .child(Node::new(node::TEXT).prop(prop::CONTENT, "bold")),
                    ),
            ),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<italic>italic</italic>"));
        assert!(xml.contains("<bold>bold</bold>"));
    }

    #[test]
    fn test_roundtrip_through_reader() {
        let jats = r#"<?xml version="1.0"?>
<article><body><p>Hello <italic>world</italic></p></body></article>"#;
        let parsed = rescribe_read_jats::parse(jats).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(xml.contains("<p>Hello <italic>world</italic></p>"));
    }
}
