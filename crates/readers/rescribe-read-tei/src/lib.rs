//! TEI XML reader for rescribe.
//!
//! Translates `tei_fmt::TeiDoc` (the standalone TEI/XML AST from the
//! `tei-fmt` crate) into rescribe's document IR. Supports common TEI P5
//! elements used in digital humanities.
//!
//! All XML tokenizing/parsing lives in `tei-fmt` — this crate is a thin
//! AST↔IR translator only (per CLAUDE.md's "adapter layer must never
//! contain parsing or writing logic" rule).

use tei_fmt::Node as TNode;

use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Parse TEI XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diagnostics) = tei_fmt::parse(input.as_bytes());

    let mut warnings: Vec<FidelityWarning> = diagnostics
        .into_iter()
        .map(|d| {
            FidelityWarning::new(
                Severity::Major,
                WarningKind::FeatureLost("xml-parse-error".to_string()),
                format!("TEI XML parse error: {}", d.message),
            )
        })
        .collect();

    let mut metadata = Properties::new();
    let mut children = Vec::new();
    for top in &doc.nodes {
        if let TNode::Element {
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
                    // The root `<TEI>` element itself carries no rescribe-
                    // level semantics; pass its children through rather than
                    // dropping them.
                    children.extend(converted);
                }
            }
        }
        // Leading/trailing Comment/PI/Doctype/whitespace-Text at the very
        // top level (outside the root element) carry no IR meaning and have
        // no cross-format equivalent to model; TEI documents otherwise
        // consist of exactly one root `<TEI>` element.
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

/// Convert a slice of TEI child nodes into rescribe IR nodes, discarding
/// nodes that only exist to be unwrapped (e.g. `<teiHeader>`/`<fileDesc>`,
/// which are consumed for metadata) and passing through "structural"
/// wrapper elements (like `<text>`/`<body>`) as their own children.
fn convert_children(
    children: &[TNode],
    parent_name: &str,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) -> Vec<Node> {
    let mut out = Vec::new();
    for child in children {
        match child {
            TNode::Element {
                name,
                attrs,
                children: kids,
                ..
            } => {
                let converted_kids = convert_children(kids, name, metadata, warnings);
                match convert_element(name, attrs, converted_kids.clone(), Some(parent_name)) {
                    Some(node) => out.push(node),
                    None => {
                        if name == "teiHeader" {
                            extract_metadata(&converted_kids, metadata);
                        } else {
                            // Pass-through wrapper element (e.g. `text`,
                            // `body`, `front`, `back`, `fileDesc`,
                            // `titleStmt`): splice its already converted
                            // children directly into the parent.
                            out.extend(converted_kids);
                        }
                    }
                }
            }
            TNode::Text { content, .. } => {
                if !content.trim().is_empty() {
                    out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
                }
            }
            TNode::Cdata { content, .. } => {
                out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
            }
            TNode::EntityRef { name, .. } => {
                // Named entity the DTD defines but we cannot resolve
                // without it — raw-preserve verbatim rather than drop.
                out.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::CONTENT, format!("&{name};"))
                        .prop("tei:entity", name.clone()),
                );
            }
            TNode::Comment { .. } | TNode::ProcessingInstruction { .. } | TNode::Doctype { .. } => {
                // No cross-format meaning and no natural IR raw-block slot
                // inside inline/block flow content; TEI's own semantic
                // model has no equivalent for a bare PI/comment here.
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("comment-or-pi".to_string()),
                    format!("dropped non-content TEI node inside <{parent_name}>"),
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

/// Attach the two generic TEI attributes that apply to (almost) any
/// element — `xml:id` and `n` — as raw-preserved properties, if present.
///
/// The pre-split reader captured both into a `FrameAttrs` struct but never
/// actually read them back out when building IR nodes, so `xml:id` and `n`
/// were parsed and then silently discarded on every element that carried
/// them. This closes that gap: `xml:id` becomes the standard `id` property
/// (it is rescribe's own identity-attribute prop, and TEI's `xml:id` is
/// exactly that construct), `n` becomes `tei:n` (TEI-specific numbering
/// with no standard cross-format equivalent).
fn attach_generic_attrs(mut node: Node, attrs: &[(String, String)]) -> Node {
    if let Some(id) = get_attr(attrs, "xml:id") {
        node = node.prop(prop::ID, id.to_string());
    }
    if let Some(n) = get_attr(attrs, "n") {
        node = node.prop("tei:n", n.to_string());
    }
    node
}

/// Convert one TEI element (with its already-converted children) into a
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
    let rend = get_attr(attrs, "rend");
    let target = get_attr(attrs, "target");
    let url = get_attr(attrs, "url");

    let node = match name {
        // Document structure
        "TEI" | "text" | "body" | "front" | "back" => None, // Pass through
        // Handled by the caller (`convert_children`) via `extract_metadata`.
        "teiHeader" => None,
        "fileDesc" | "titleStmt" | "publicationStmt" | "sourceDesc" => None, // Pass through into teiHeader extraction

        // Divisions
        "div" | "div1" | "div2" | "div3" | "div4" => Some(Node::new(node::DIV).children(children)),

        // Headings
        "head" => {
            let level = match parent {
                Some("div1") | Some("div") => 1,
                Some("div2") => 2,
                Some("div3") => 3,
                Some("div4") => 4,
                _ => 2,
            };
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level as i64)
                    .children(children),
            )
        }

        // Paragraphs
        "p" => Some(Node::new(node::PARAGRAPH).children(children)),

        // Lists
        "list" => {
            let ordered = rend == Some("numbered");
            Some(
                Node::new(node::LIST)
                    .prop(prop::ORDERED, ordered)
                    .children(children),
            )
        }
        "item" => Some(Node::new(node::LIST_ITEM).children(children)),

        // Glossary/definition lists
        "gloss" => Some(Node::new(node::DEFINITION_LIST).children(children)),
        "term" => Some(Node::new(node::DEFINITION_TERM).children(children)),
        "def" | "desc" => Some(Node::new(node::DEFINITION_DESC).children(children)),

        // Block quote
        "quote" | "cit" => Some(Node::new(node::BLOCKQUOTE).children(children)),

        // Poetry/verse
        "lg" => Some(
            Node::new(node::DIV)
                .prop("tei:type", "verse")
                .children(children),
        ),
        "l" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "line")
                .children(children),
        ),

        // Code
        "code" | "eg" => {
            let text = extract_text(&children);
            Some(Node::new(node::CODE_BLOCK).prop(prop::CONTENT, text))
        }

        // Highlighting (inline formatting)
        "hi" => match rend {
            Some("bold") | Some("b") => Some(Node::new(node::STRONG).children(children)),
            Some("italic") | Some("i") | Some("it") => {
                Some(Node::new(node::EMPHASIS).children(children))
            }
            Some("underline") | Some("u") => Some(Node::new(node::UNDERLINE).children(children)),
            Some("strike") | Some("strikethrough") => {
                Some(Node::new(node::STRIKEOUT).children(children))
            }
            Some("sup") | Some("superscript") => {
                Some(Node::new(node::SUPERSCRIPT).children(children))
            }
            Some("sub") | Some("subscript") => Some(Node::new(node::SUBSCRIPT).children(children)),
            Some("sc") | Some("smallcaps") => Some(Node::new(node::SMALL_CAPS).children(children)),
            other => {
                let mut n = Node::new(node::EMPHASIS).children(children);
                // Preserve an unrecognized `rend` value raw rather than
                // silently coercing it to plain emphasis.
                if let Some(r) = other {
                    n = n.prop("tei:rend", r.to_string());
                }
                Some(n)
            }
        },

        // Semantic highlighting
        "emph" => Some(Node::new(node::EMPHASIS).children(children)),
        "foreign" => Some(Node::new(node::EMPHASIS).children(children)),
        "title" => {
            // Could be in metadata or inline, depending on ancestry.
            if matches!(parent, Some("titleStmt") | Some("teiHeader")) {
                let title = extract_text(&children);
                if title.is_empty() {
                    None
                } else {
                    // Extraction happens via `extract_metadata` walking the
                    // converted `teiHeader` subtree, so just surface the
                    // heading-shaped node here; `extract_metadata` looks
                    // for `HEADING` nodes.
                    Some(
                        Node::new(node::HEADING)
                            .prop(prop::LEVEL, 1i64)
                            .children(children),
                    )
                }
            } else {
                Some(Node::new(node::EMPHASIS).children(children))
            }
        }

        // Links
        "ref" | "ptr" => {
            let mut n = Node::new(node::LINK).children(children);
            if let Some(t) = target {
                n = n.prop(prop::URL, t.to_string());
            }
            Some(n)
        }

        // Figures
        "figure" => Some(Node::new(node::FIGURE).children(children)),
        "figDesc" => Some(
            Node::new("figcaption")
                .prop("html:tag", "figcaption")
                .children(children),
        ),
        "graphic" => url.map(|u| Node::new(node::IMAGE).prop(prop::URL, u.to_string())),

        // Tables
        "table" => Some(Node::new(node::TABLE).children(children)),
        "row" => Some(Node::new(node::TABLE_ROW).children(children)),
        "cell" => {
            let role = rend;
            if role == Some("header") || role == Some("label") {
                Some(Node::new(node::TABLE_HEADER).children(children))
            } else {
                Some(Node::new(node::TABLE_CELL).children(children))
            }
        }

        // Notes/footnotes
        "note" => Some(Node::new(node::FOOTNOTE_DEF).children(children)),

        // Formula
        "formula" => {
            let text = extract_text(&children);
            Some(Node::new("math_display").prop("math:source", text))
        }

        // Line/page breaks
        "lb" => Some(Node::new(node::LINE_BREAK)),
        "pb" => Some(Node::new(node::HORIZONTAL_RULE)),

        // Default: pass through children
        _ => None,
    };

    node.map(|n| attach_generic_attrs(n, attrs))
}

/// Extract `<teiHeader>` metadata (currently: title, found by searching the
/// already-converted children for a `HEADING` — matches the pre-split
/// reader's approach).
fn extract_metadata(nodes: &[Node], metadata: &mut Properties) {
    for node in nodes {
        if node.kind.as_str() == node::HEADING {
            let title = extract_text(&node.children);
            if !title.is_empty() {
                metadata.set("title", title);
            }
        }
        extract_metadata(&node.children, metadata);
    }
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
    fn test_parse_simple_document() {
        let tei = r#"<?xml version="1.0"?>
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt>
        <title>Test Document</title>
      </titleStmt>
    </fileDesc>
  </teiHeader>
  <text>
    <body>
      <p>Hello, world!</p>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.metadata.get_str("title"), Some("Test Document"));
    }

    #[test]
    fn test_parse_divisions() {
        let tei = r#"<?xml version="1.0"?>
<TEI>
  <text>
    <body>
      <div>
        <head>Introduction</head>
        <p>Content here.</p>
      </div>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_lists() {
        let tei = r#"<?xml version="1.0"?>
<TEI>
  <text>
    <body>
      <list>
        <item>Item 1</item>
        <item>Item 2</item>
      </list>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_formatting() {
        let tei = r#"<?xml version="1.0"?>
<TEI>
  <text>
    <body>
      <p><hi rend="italic">italic</hi> and <hi rend="bold">bold</hi> text</p>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_table() {
        let tei = r#"<?xml version="1.0"?>
<TEI>
  <text>
    <body>
      <table>
        <row>
          <cell rend="header">Header</cell>
        </row>
        <row>
          <cell>Cell</cell>
        </row>
      </table>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_xml_id_and_n_preserved() {
        // Regression test for the fidelity bug found while extracting
        // tei-fmt: the old hand-rolled reader captured `xml:id`/`n` into a
        // FrameAttrs struct but never read them back out, so they were
        // parsed and then silently discarded on every element.
        let tei =
            r#"<TEI><text><body><div xml:id="d1" n="1"><p>Text</p></div></body></text></TEI>"#;
        let result = parse(tei).unwrap();
        let doc = result.value;
        let div = doc
            .content
            .children
            .iter()
            .find(|n| n.kind.as_str() == node::DIV)
            .expect("div node");
        assert_eq!(div.props.get_str(prop::ID), Some("d1"));
        assert_eq!(div.props.get_str("tei:n"), Some("1"));
    }

    #[test]
    fn test_unresolvable_entity_preserved() {
        let tei = r#"<TEI><text><body><p>a &custom; b</p></body></text></TEI>"#;
        let result = parse(tei).unwrap();
        let doc = result.value;
        let para = doc
            .content
            .children
            .iter()
            .find(|n| n.kind.as_str() == node::PARAGRAPH)
            .expect("paragraph node");
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::RAW_INLINE)
        );
    }
}
