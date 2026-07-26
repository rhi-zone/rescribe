//! DocBook reader for rescribe.
//!
//! Translates `docbook_fmt::DocBookDoc` (the standalone DocBook/XML AST from
//! the `docbook-fmt` crate) into rescribe's document IR. Supports DocBook 5
//! and DocBook 4 elements.
//!
//! All XML tokenizing/parsing lives in `docbook-fmt` — this crate is a thin
//! AST↔IR translator only (per CLAUDE.md's "adapter layer must never
//! contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use rescribe_read_docbook::parse;
//!
//! let docbook = r#"<?xml version="1.0"?>
//! <article xmlns="http://docbook.org/ns/docbook">
//!   <title>Example Article</title>
//!   <para>Hello, world!</para>
//! </article>"#;
//!
//! let result = parse(docbook).unwrap();
//! let doc = result.value;
//! ```

use docbook_fmt::Node as DbNode;
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Parse DocBook XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diagnostics) = docbook_fmt::parse(input.as_bytes());

    let mut warnings: Vec<FidelityWarning> = diagnostics
        .into_iter()
        .map(|d| {
            FidelityWarning::new(
                Severity::Major,
                WarningKind::FeatureLost("xml-parse-error".to_string()),
                format!("DocBook XML parse error: {}", d.message),
            )
        })
        .collect();

    let mut metadata = Properties::new();
    let mut children = Vec::new();
    for top in &doc.nodes {
        if let DbNode::Element {
            name,
            attrs,
            children: kids,
            ..
        } = top
        {
            let converted = convert_children(kids, name, &mut metadata, &mut warnings);
            match convert_element(name, attrs, converted.clone(), None) {
                Some(node) => children.push(node),
                None => {
                    // Root element itself carries no rescribe-level
                    // semantics (shouldn't normally happen for
                    // article/book/etc, but pass its children through
                    // rather than dropping them).
                    children.extend(converted);
                }
            }
        }
        // Leading/trailing Comment/PI/Doctype/whitespace-Text at the very
        // top level (outside the root element) carry no IR meaning and
        // have no cross-format equivalent to model; DocBook documents
        // otherwise consist of exactly one root element.
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

/// Convert a slice of DocBook child nodes into rescribe IR nodes,
/// discarding nodes that only exist to be unwrapped (e.g. `<info>`, which
/// is consumed for metadata) and passing through "structural" wrapper
/// elements (like `<tgroup>`) as their own children.
fn convert_children(
    children: &[DbNode],
    parent_name: &str,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) -> Vec<Node> {
    let mut out = Vec::new();
    for child in children {
        match child {
            DbNode::Element {
                name,
                attrs,
                children: kids,
                ..
            } => {
                let converted_kids = convert_children(kids, name, metadata, warnings);
                match convert_element(name, attrs, converted_kids.clone(), Some(parent_name)) {
                    Some(node) => out.push(node),
                    None => {
                        if name == "info" || name == "articleinfo" || name == "bookinfo" {
                            extract_metadata(kids, metadata);
                        } else {
                            // Pass-through wrapper element (e.g. tgroup,
                            // imageobject, mediaobject): splice its already
                            // converted children directly into the parent.
                            out.extend(converted_kids);
                        }
                    }
                }
            }
            DbNode::Text { content, .. } => {
                if !content.trim().is_empty() {
                    out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
                }
            }
            DbNode::Cdata { content, .. } => {
                out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
            }
            DbNode::EntityRef { name, .. } => {
                // Named entity DocBook/the DTD defines but we cannot resolve
                // without the DTD — raw-preserve verbatim rather than drop.
                out.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::CONTENT, format!("&{name};"))
                        .prop("docbook:entity", name.clone()),
                );
            }
            DbNode::Comment { .. }
            | DbNode::ProcessingInstruction { .. }
            | DbNode::Doctype { .. } => {
                // No cross-format meaning and no natural IR raw-block slot
                // inside inline/block flow content; DocBook's own semantic
                // model has no equivalent for a bare PI/comment here.
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("comment-or-pi".to_string()),
                    format!("dropped non-content DocBook node inside <{parent_name}>"),
                ));
            }
        }
    }
    out
}

fn get_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Convert one DocBook element (with its already-converted children) into a
/// rescribe node. Returns `None` for elements that either have no IR
/// representation of their own (pass-through wrappers) or are consumed for
/// side effects (metadata extraction) — see [`convert_children`] for how
/// those two cases are told apart.
fn convert_element(
    name: &str,
    attrs: &[(String, String)],
    children: Vec<Node>,
    parent: Option<&str>,
) -> Option<Node> {
    let role = get_attr(attrs, "role");
    let url = get_attr(attrs, "url").or_else(|| get_attr(attrs, "xlink:href"));
    let language = get_attr(attrs, "language");

    match name {
        // Document level
        "article" | "book" | "chapter" | "part" | "appendix" => {
            Some(Node::new(node::DIV).children(children))
        }

        // Sections
        "section" | "sect1" | "sect2" | "sect3" | "sect4" | "sect5" | "simplesect" => {
            Some(Node::new(node::DIV).children(children))
        }

        // Titles - convert to heading
        "title" => {
            let level = match parent {
                Some("article") | Some("book") | Some("chapter") | Some("part") => 1,
                Some("sect1") | Some("section") => 2,
                Some("sect2") => 3,
                Some("sect3") => 4,
                Some("sect4") => 5,
                Some("sect5") => 6,
                _ => 2,
            };
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level as i64)
                    .children(children),
            )
        }

        // Paragraphs
        "para" | "simpara" => Some(Node::new(node::PARAGRAPH).children(children)),

        // Block quote
        "blockquote" => Some(Node::new(node::BLOCKQUOTE).children(children)),

        // Lists
        "itemizedlist" => Some(
            Node::new(node::LIST)
                .prop(prop::ORDERED, false)
                .children(children),
        ),
        "orderedlist" => Some(
            Node::new(node::LIST)
                .prop(prop::ORDERED, true)
                .children(children),
        ),
        "listitem" => Some(Node::new(node::LIST_ITEM).children(children)),

        // Definition lists
        "variablelist" => Some(Node::new(node::DEFINITION_LIST).children(children)),
        "varlistentry" => Some(Node::new("docbook:varlistentry").children(children)),
        "term" => Some(Node::new(node::DEFINITION_TERM).children(children)),

        // Code
        "programlisting" | "screen" | "literallayout" => {
            let text = extract_text(&children);
            let mut node = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, text);
            if let Some(lang) = language {
                node = node.prop(prop::LANGUAGE, lang.to_string());
            }
            Some(node)
        }
        "code" | "literal" | "command" | "filename" | "option" | "computeroutput" | "userinput" => {
            let text = extract_text(&children);
            Some(Node::new(node::CODE).prop(prop::CONTENT, text))
        }

        // Inline formatting
        "emphasis" => {
            if role == Some("strong") || role == Some("bold") {
                Some(Node::new(node::STRONG).children(children))
            } else {
                Some(Node::new(node::EMPHASIS).children(children))
            }
        }
        "subscript" => Some(Node::new(node::SUBSCRIPT).children(children)),
        "superscript" => Some(Node::new(node::SUPERSCRIPT).children(children)),

        // Links
        "link" | "ulink" | "xref" => {
            let mut node = Node::new(node::LINK).children(children);
            if let Some(url) = url {
                node = node.prop(prop::URL, url.to_string());
            } else if let Some(linkend) = get_attr(attrs, "linkend") {
                node = node
                    .prop(prop::URL, format!("#{linkend}"))
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, linkend.to_string()));
            }
            Some(node)
        }

        // Figures and media
        "figure" | "informalfigure" => Some(Node::new(node::FIGURE).children(children)),
        "mediaobject" | "inlinemediaobject" | "imageobject" | "textobject" => None, // Pass through
        "imagedata" | "graphic" => get_attr(attrs, "fileref")
            .map(|url| Node::new(node::IMAGE).prop(prop::URL, url.to_string())),
        "caption" => Some(
            Node::new("figcaption")
                .prop("html:tag", "figcaption")
                .children(children),
        ),

        // Tables
        "table" | "informaltable" => Some(Node::new(node::TABLE).children(children)),
        "tgroup" | "thead" | "tbody" | "tfoot" => None, // Pass through
        "row" | "tr" => Some(Node::new(node::TABLE_ROW).children(children)),
        "entry" | "td" => Some(Node::new(node::TABLE_CELL).children(children)),
        "th" => Some(Node::new(node::TABLE_HEADER).children(children)),

        // Footnotes
        "footnote" => Some(Node::new(node::FOOTNOTE_DEF).children(children)),

        // Admonitions
        "note" | "tip" | "warning" | "caution" | "important" => Some(
            Node::new(node::BLOCKQUOTE)
                .prop("docbook:type", name)
                .children(children),
        ),

        // Abstract and other metadata
        "abstract" => Some(
            Node::new(node::DIV)
                .prop("html:class", "abstract")
                .children(children),
        ),
        // Handled by the caller (`convert_children`) via `extract_metadata`.
        "info" | "articleinfo" | "bookinfo" => None,
        "author" | "authorgroup" | "date" | "copyright" | "legalnotice" | "pubdate"
        | "releaseinfo" | "revhistory" | "revision" => None, // Handled in extract_metadata
        "personname" | "firstname" | "surname" | "othername" => None, // Just text extraction

        // Line break
        "sbr" => Some(Node::new(node::LINE_BREAK)),

        // Horizontal rule equivalent
        "bridgehead" => Some(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 4i64)
                .children(children),
        ),

        // Anchors: cross-format equivalent to an empty link target
        "anchor" => {
            let mut node = Node::new(node::LINK);
            if let Some(id) = get_attr(attrs, "id").or_else(|| get_attr(attrs, "xml:id")) {
                node = node.prop("id", id.to_string());
            }
            Some(node)
        }

        // Default: pass through children (structural/unknown wrapper)
        _ => None,
    }
}

/// Extract `<info>`/`<articleinfo>`/`<bookinfo>` metadata (currently: title).
fn extract_metadata(nodes: &[DbNode], metadata: &mut Properties) {
    for node in nodes {
        if let DbNode::Element { name, children, .. } = node
            && name == "title"
        {
            let title = extract_docbook_text(children);
            if !title.is_empty() {
                metadata.set("title", title);
            }
        }
        if let DbNode::Element { children, .. } = node {
            extract_metadata(children, metadata);
        }
    }
}

fn extract_docbook_text(nodes: &[DbNode]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            DbNode::Text { content, .. } | DbNode::Cdata { content, .. } => text.push_str(content),
            DbNode::Element { children, .. } => text.push_str(&extract_docbook_text(children)),
            _ => {}
        }
    }
    text
}

fn extract_text(nodes: &[Node]) -> String {
    let mut text = String::new();
    for node in nodes {
        if node.kind.as_str() == node::TEXT
            && let Some(content) = node.props.get_str(prop::CONTENT)
        {
            text.push_str(content);
        }
        text.push_str(&extract_text(&node.children));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_article() {
        let docbook = r#"<?xml version="1.0"?>
<article xmlns="http://docbook.org/ns/docbook">
  <title>Test Article</title>
  <para>Hello, world!</para>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_sections() {
        let docbook = r#"<?xml version="1.0"?>
<article>
  <section>
    <title>Section 1</title>
    <para>Content</para>
  </section>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_lists() {
        let docbook = r#"<?xml version="1.0"?>
<article>
  <itemizedlist>
    <listitem><para>Item 1</para></listitem>
    <listitem><para>Item 2</para></listitem>
  </itemizedlist>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let docbook = r#"<?xml version="1.0"?>
<article>
  <programlisting language="rust">fn main() {}</programlisting>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_emphasis() {
        let docbook = r#"<?xml version="1.0"?>
<article>
  <para><emphasis>italic</emphasis> and <emphasis role="strong">bold</emphasis></para>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_unresolvable_entity_preserved() {
        let docbook = r#"<article><para>a &custom; b</para></article>"#;
        let result = parse(docbook).unwrap();
        let doc = result.value;
        let article = &doc.content.children[0];
        let para = &article.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::RAW_INLINE)
        );
    }
}
