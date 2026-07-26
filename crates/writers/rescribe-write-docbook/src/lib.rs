//! DocBook writer for rescribe.
//!
//! Translates rescribe's document IR into `docbook_fmt::DocBookDoc` (the
//! standalone DocBook/XML AST from the `docbook-fmt` crate) and serializes
//! it via `docbook_fmt::emit`. All XML writing lives in `docbook-fmt` —
//! this crate is a thin IR↔AST translator only (per CLAUDE.md's "adapter
//! layer must never contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use rescribe_write_docbook::emit;
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

use docbook_fmt::{DocBookDoc, Node as DbNode, XmlDecl};
use rescribe_core::{ConversionResult, Document, EmitError, Node};
use rescribe_std::{node, prop};

/// Emit a document to DocBook XML.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let warnings = Vec::new();

    let mut root_children = Vec::new();
    if let Some(title) = doc.metadata.get_str("title") {
        root_children.push(db_element("title", vec![], vec![db_text(title)]));
    }
    for child in &doc.content.children {
        root_children.extend(write_node(child));
    }

    let root = DbNode::Element {
        name: "article".to_string(),
        attrs: vec![
            (
                "xmlns".to_string(),
                "http://docbook.org/ns/docbook".to_string(),
            ),
            ("version".to_string(), "5.0".to_string()),
        ],
        children: root_children,
        span: docbook_fmt::Span::NONE,
    };

    let doc_ast = DocBookDoc {
        xml_decl: Some(XmlDecl {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
        }),
        nodes: vec![root],
    };

    let bytes = docbook_fmt::emit(&doc_ast);
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

fn db_element(name: &str, attrs: Vec<(String, String)>, children: Vec<DbNode>) -> DbNode {
    DbNode::Element {
        name: name.to_string(),
        attrs,
        children,
        span: docbook_fmt::Span::NONE,
    }
}

fn db_text(content: impl Into<String>) -> DbNode {
    DbNode::Text {
        content: content.into(),
        span: docbook_fmt::Span::NONE,
    }
}

/// Convert one rescribe IR (block-level) node into zero or more DocBook AST
/// nodes.
fn write_node(node: &Node) -> Vec<DbNode> {
    match node.kind.as_str() {
        node::DOCUMENT | node::DIV => node.children.iter().flat_map(write_node).collect(),

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1);
            let section_tag = match level {
                1 => "section",
                2 => "sect1",
                3 => "sect2",
                4 => "sect3",
                5 => "sect4",
                _ => "sect5",
            };
            vec![db_element(
                section_tag,
                vec![],
                vec![db_element(
                    "title",
                    vec![],
                    node.children.iter().flat_map(write_inline).collect(),
                )],
            )]
        }

        node::PARAGRAPH => vec![db_element(
            "para",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::BLOCKQUOTE => {
            let tag = node
                .props
                .get_str("docbook:type")
                .filter(|t| matches!(*t, "note" | "tip" | "warning" | "caution" | "important"))
                .unwrap_or("blockquote");
            vec![db_element(
                tag,
                vec![],
                node.children.iter().flat_map(write_node).collect(),
            )]
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let tag = if ordered {
                "orderedlist"
            } else {
                "itemizedlist"
            };
            vec![db_element(
                tag,
                vec![],
                node.children.iter().flat_map(write_node).collect(),
            )]
        }

        node::LIST_ITEM => vec![db_element(
            "listitem",
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
                entries.push(db_element("varlistentry", vec![], entry_children));
                i += 2;
            }
            vec![db_element("variablelist", vec![], entries)]
        }

        node::DEFINITION_TERM => vec![db_element(
            "term",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::DEFINITION_DESC => vec![db_element(
            "listitem",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::CODE_BLOCK => {
            let mut attrs = Vec::new();
            if let Some(lang) = node.props.get_str(prop::LANGUAGE) {
                attrs.push(("language".to_string(), lang.to_string()));
            }
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            vec![db_element("programlisting", attrs, vec![db_text(content)])]
        }

        node::TABLE => vec![db_element(
            "informaltable",
            vec![],
            vec![db_element(
                "tgroup",
                vec![],
                vec![db_element(
                    "tbody",
                    vec![],
                    node.children.iter().flat_map(write_node).collect(),
                )],
            )],
        )],

        node::TABLE_ROW => vec![db_element(
            "row",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::TABLE_CELL | node::TABLE_HEADER => vec![db_element(
            "entry",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::FIGURE => vec![db_element(
            "figure",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::IMAGE => vec![db_element(
            "mediaobject",
            vec![],
            vec![db_element(
                "imageobject",
                vec![],
                vec![db_element(
                    "imagedata",
                    node.props
                        .get_str(prop::URL)
                        .map(|url| vec![("fileref".to_string(), url.to_string())])
                        .unwrap_or_default(),
                    vec![],
                )],
            )],
        )],

        node::HORIZONTAL_RULE => Vec::new(), // DocBook doesn't have HR

        node::FOOTNOTE_DEF => vec![db_element(
            "footnote",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        // Inline nodes that appear at block level: wrap in a <para>.
        node::TEXT | node::EMPHASIS | node::STRONG | node::CODE | node::LINK => {
            vec![db_element("para", vec![], write_inline(node))]
        }

        _ => {
            // Unknown block - recurse into children
            node.children.iter().flat_map(write_node).collect()
        }
    }
}

/// Convert one rescribe IR (inline-level) node into zero or more DocBook AST
/// nodes.
fn write_inline(node: &Node) -> Vec<DbNode> {
    match node.kind.as_str() {
        node::TEXT => match node.props.get_str(prop::CONTENT) {
            Some(content) => vec![db_text(content)],
            None => Vec::new(),
        },

        node::EMPHASIS => vec![db_element(
            "emphasis",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::STRONG => vec![db_element(
            "emphasis",
            vec![("role".to_string(), "strong".to_string())],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::CODE => {
            let mut children: Vec<DbNode> = node
                .props
                .get_str(prop::CONTENT)
                .map(db_text)
                .into_iter()
                .collect();
            children.extend(node.children.iter().flat_map(write_inline));
            vec![db_element("code", vec![], children)]
        }

        node::LINK => {
            let mut attrs = Vec::new();
            if let Some(url) = node.props.get_str(prop::URL) {
                attrs.push(("xlink:href".to_string(), url.to_string()));
            }
            vec![db_element(
                "link",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )]
        }

        node::SUBSCRIPT => vec![db_element(
            "subscript",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::SUPERSCRIPT => vec![db_element(
            "superscript",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::LINE_BREAK => vec![db_element("sbr", vec![], vec![])],

        node::SOFT_BREAK => vec![db_text(" ")],

        node::IMAGE => vec![db_element(
            "inlinemediaobject",
            vec![],
            vec![db_element(
                "imageobject",
                vec![],
                vec![db_element(
                    "imagedata",
                    node.props
                        .get_str(prop::URL)
                        .map(|url| vec![("fileref".to_string(), url.to_string())])
                        .unwrap_or_default(),
                    vec![],
                )],
            )],
        )],

        // A raw entity reference preserved by the reader: re-emit verbatim.
        node::RAW_INLINE => match node.props.get_str("docbook:entity") {
            Some(name) => vec![DbNode::EntityRef {
                name: name.to_string(),
                span: docbook_fmt::Span::NONE,
            }],
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
        // An empty root element is emitted self-closing (`<article .../>`) —
        // valid XML equivalent to `<article ...></article>`.
        assert!(xml.contains("/>"));
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
        assert!(xml.contains("<para>Hello, world!</para>"));
    }

    #[test]
    fn test_emit_with_title() {
        let mut metadata = Properties::new();
        metadata.set("title", "Test Document".to_string());

        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata,
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<title>Test Document</title>"));
    }

    #[test]
    fn test_roundtrip_through_reader() {
        let docbook = r#"<?xml version="1.0"?>
<article><title>T</title><para>Hello <emphasis>world</emphasis></para></article>"#;
        let parsed = rescribe_read_docbook::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(xml.contains("<para>Hello <emphasis>world</emphasis></para>"));
    }
}
