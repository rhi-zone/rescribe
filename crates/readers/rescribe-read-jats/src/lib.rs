//! JATS XML reader for rescribe.
//!
//! Translates `jats_fmt::JatsDoc` (the standalone JATS/XML AST from the
//! `jats-fmt` crate) into rescribe's document IR. Supports JATS 1.0/1.1/1.2/
//! 1.3 elements commonly used in scholarly publishing.
//!
//! All XML tokenizing/parsing lives in `jats-fmt` — this crate is a thin
//! AST↔IR translator only (per CLAUDE.md's "adapter layer must never
//! contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use rescribe_read_jats::parse;
//!
//! let jats = r#"<?xml version="1.0"?>
//! <article>
//!   <front>
//!     <article-meta>
//!       <title-group>
//!         <article-title>Test Article</article-title>
//!       </title-group>
//!     </article-meta>
//!   </front>
//!   <body>
//!     <p>Hello, world!</p>
//!   </body>
//! </article>"#;
//!
//! let result = parse(jats).unwrap();
//! let doc = result.value;
//! ```

use jats_fmt::Node as JNode;
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Parse JATS XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diagnostics) = jats_fmt::parse(input.as_bytes());

    let mut warnings: Vec<FidelityWarning> = diagnostics
        .into_iter()
        .map(|d| {
            FidelityWarning::new(
                Severity::Major,
                WarningKind::FeatureLost("xml-parse-error".to_string()),
                format!("JATS XML parse error: {}", d.message),
            )
        })
        .collect();

    let mut metadata = Properties::new();
    let mut children = Vec::new();
    for top in &doc.nodes {
        if let JNode::Element {
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
                    // The root element itself carries no rescribe-level
                    // semantics (shouldn't normally happen for `<article>`),
                    // pass its children through rather than dropping them.
                    children.extend(converted);
                }
            }
        }
        // Leading/trailing Comment/PI/Doctype/whitespace-Text at the very
        // top level (outside the root element) carry no IR meaning and have
        // no cross-format equivalent to model; JATS documents otherwise
        // consist of exactly one root `<article>` element.
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

/// Convert a slice of JATS child nodes into rescribe IR nodes, discarding
/// nodes that only exist to be unwrapped (e.g. `<article-meta>`/
/// `<journal-meta>`, which are consumed for metadata) and passing through
/// "structural" wrapper elements (like `<title-group>`) as their own
/// children.
fn convert_children(
    children: &[JNode],
    parent_name: &str,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) -> Vec<Node> {
    let mut out = Vec::new();
    for child in children {
        match child {
            JNode::Element {
                name,
                attrs,
                children: kids,
                ..
            } => {
                let converted_kids = convert_children(kids, name, metadata, warnings);
                match convert_element(name, attrs, converted_kids.clone(), Some(parent_name)) {
                    Some(node) => out.push(node),
                    None => {
                        if name == "article-meta" || name == "journal-meta" {
                            extract_metadata(&converted_kids, metadata);
                        } else {
                            // Pass-through wrapper element (e.g.
                            // title-group, fn-group's own nested wrappers):
                            // splice its already converted children
                            // directly into the parent.
                            out.extend(converted_kids);
                        }
                    }
                }
            }
            JNode::Text { content, .. } => {
                if !content.trim().is_empty() {
                    out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
                }
            }
            JNode::Cdata { content, .. } => {
                out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
            }
            JNode::EntityRef { name, .. } => {
                // Named entity the DTD defines but we cannot resolve
                // without it — raw-preserve verbatim rather than drop.
                out.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::CONTENT, format!("&{name};"))
                        .prop("jats:entity", name.clone()),
                );
            }
            JNode::Comment { .. } | JNode::ProcessingInstruction { .. } | JNode::Doctype { .. } => {
                // No cross-format meaning and no natural IR raw-block slot
                // inside inline/block flow content; JATS's own semantic
                // model has no equivalent for a bare PI/comment here.
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("comment-or-pi".to_string()),
                    format!("dropped non-content JATS node inside <{parent_name}>"),
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

/// Convert one JATS element (with its already-converted children) into a
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
    let href = get_attr(attrs, "href").or_else(|| get_attr(attrs, "xlink:href"));
    let rid = get_attr(attrs, "rid");
    let ref_type = get_attr(attrs, "ref-type");
    let content_type = get_attr(attrs, "content-type");
    let list_type = get_attr(attrs, "list-type");

    match name {
        // Document structure
        "article" => Some(Node::new(node::DIV).children(children)),
        "front" | "body" | "back" => None, // Pass through
        // Handled by the caller (`convert_children`) via `extract_metadata`.
        "article-meta" | "journal-meta" => None,

        // Sections
        "sec" => Some(Node::new(node::DIV).children(children)),

        // Titles
        "title" | "article-title" => {
            let level = match parent {
                Some("article") | Some("front") | Some("article-meta") => 1,
                Some("sec") => 2,
                Some("fig") | Some("table-wrap") => 3,
                _ => 2,
            };
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level as i64)
                    .children(children),
            )
        }
        "subtitle" => Some(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 2i64)
                .children(children),
        ),

        // Paragraphs
        "p" => Some(Node::new(node::PARAGRAPH).children(children)),

        // Abstract
        "abstract" => Some(
            Node::new(node::DIV)
                .prop("html:class", "abstract")
                .children(children),
        ),

        // Lists
        "list" => {
            let ordered = list_type == Some("order");
            Some(
                Node::new(node::LIST)
                    .prop(prop::ORDERED, ordered)
                    .children(children),
            )
        }
        "list-item" => Some(Node::new(node::LIST_ITEM).children(children)),

        // Definition lists
        "def-list" => Some(Node::new(node::DEFINITION_LIST).children(children)),
        "def-item" => None, // Pass through
        "term" => Some(Node::new(node::DEFINITION_TERM).children(children)),
        "def" => Some(Node::new(node::DEFINITION_DESC).children(children)),

        // Code
        "code" | "preformat" => {
            let text = extract_text(&children);
            let mut node = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, text);
            if let Some(lang) = content_type {
                node = node.prop(prop::LANGUAGE, lang.to_string());
            }
            Some(node)
        }
        "monospace" => {
            let text = extract_text(&children);
            Some(Node::new(node::CODE).prop(prop::CONTENT, text))
        }

        // Block quote
        "disp-quote" | "boxed-text" => Some(Node::new(node::BLOCKQUOTE).children(children)),

        // Inline formatting
        "italic" => Some(Node::new(node::EMPHASIS).children(children)),
        "bold" => Some(Node::new(node::STRONG).children(children)),
        "underline" => Some(Node::new(node::UNDERLINE).children(children)),
        "strike" => Some(Node::new(node::STRIKEOUT).children(children)),
        "sub" => Some(Node::new(node::SUBSCRIPT).children(children)),
        "sup" => Some(Node::new(node::SUPERSCRIPT).children(children)),
        "sc" => Some(Node::new(node::SMALL_CAPS).children(children)),

        // Links
        "ext-link" => {
            let mut node = Node::new(node::LINK).children(children);
            if let Some(url) = href {
                node = node.prop(prop::URL, url.to_string());
            }
            Some(node)
        }
        "xref" => {
            let mut node = Node::new(node::LINK).children(children);
            if let Some(r) = rid {
                node = node.prop(prop::URL, format!("#{r}"));
            }
            // Preserved for both the empty (`<xref .../>`) and full
            // (`<xref ...>text</xref>`) element shapes alike — the old
            // hand-rolled reader only attached this for the self-closing
            // case, an inconsistency this rewrite closes (additive fidelity
            // fix, not a construct change).
            if let Some(rt) = ref_type {
                node = node.prop("jats:ref-type", rt.to_string());
            }
            Some(node)
        }
        "uri" => {
            let url = extract_text(&children);
            Some(
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, url)),
            )
        }

        // Figures
        "fig" | "fig-group" => Some(Node::new(node::FIGURE).children(children)),
        "caption" => Some(
            Node::new("figcaption")
                .prop("html:tag", "figcaption")
                .children(children),
        ),
        "graphic" | "inline-graphic" => {
            href.map(|url| Node::new(node::IMAGE).prop(prop::URL, url.to_string()))
        }

        // Tables
        "table-wrap" => Some(Node::new(node::FIGURE).children(children)),
        "table" => Some(Node::new(node::TABLE).children(children)),
        "thead" => Some(Node::new(node::TABLE_HEAD).children(children)),
        "tbody" => Some(Node::new(node::TABLE_BODY).children(children)),
        "tr" => Some(Node::new(node::TABLE_ROW).children(children)),
        "th" => Some(Node::new(node::TABLE_HEADER).children(children)),
        "td" => Some(Node::new(node::TABLE_CELL).children(children)),

        // Math
        "disp-formula" => {
            let text = extract_text(&children);
            Some(Node::new("math_display").prop("math:source", text))
        }
        "inline-formula" => {
            let text = extract_text(&children);
            Some(Node::new("math_inline").prop("math:source", text))
        }
        "tex-math" | "mml:math" => {
            // Already captured by the parent formula element.
            None
        }

        // Footnotes
        "fn" => Some(Node::new(node::FOOTNOTE_DEF).children(children)),
        "fn-group" => Some(Node::new(node::DIV).children(children)),

        // References
        "ref-list" => Some(
            Node::new(node::DIV)
                .prop("html:class", "references")
                .children(children),
        ),
        "ref" => Some(
            Node::new(node::PARAGRAPH)
                .prop("jats:ref", true)
                .children(children),
        ),
        "mixed-citation" | "element-citation" => Some(Node::new(node::SPAN).children(children)),

        // Metadata elements (usually skip, but may contain useful info)
        "contrib-group" | "contrib" | "name" | "surname" | "given-names" | "aff" | "pub-date"
        | "volume" | "issue" | "fpage" | "lpage" | "kwd-group" | "kwd" => None,

        // Line break
        "break" => Some(Node::new(node::LINE_BREAK)),

        // Default: pass through children
        _ => None,
    }
}

/// Extract `<article-meta>`/`<journal-meta>` metadata (currently: title,
/// found by searching the already-converted children for a `HEADING` —
/// matches the pre-split reader's approach).
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
    fn test_parse_simple_article() {
        let jats = r#"<?xml version="1.0"?>
<article>
  <front>
    <article-meta>
      <title-group>
        <article-title>Test Article</article-title>
      </title-group>
    </article-meta>
  </front>
  <body>
    <p>Hello, world!</p>
  </body>
</article>"#;

        let result = parse(jats).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.metadata.get_str("title"), Some("Test Article"));
    }

    #[test]
    fn test_parse_sections() {
        let jats = r#"<?xml version="1.0"?>
<article>
  <body>
    <sec>
      <title>Introduction</title>
      <p>Content here.</p>
    </sec>
  </body>
</article>"#;

        let result = parse(jats).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_lists() {
        let jats = r#"<?xml version="1.0"?>
<article>
  <body>
    <list list-type="bullet">
      <list-item><p>Item 1</p></list-item>
      <list-item><p>Item 2</p></list-item>
    </list>
  </body>
</article>"#;

        let result = parse(jats).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_formatting() {
        let jats = r#"<?xml version="1.0"?>
<article>
  <body>
    <p><italic>italic</italic> and <bold>bold</bold> text</p>
  </body>
</article>"#;

        let result = parse(jats).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_table() {
        let jats = r#"<?xml version="1.0"?>
<article>
  <body>
    <table-wrap>
      <table>
        <thead>
          <tr><th>Header</th></tr>
        </thead>
        <tbody>
          <tr><td>Cell</td></tr>
        </tbody>
      </table>
    </table-wrap>
  </body>
</article>"#;

        let result = parse(jats).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_unresolvable_entity_preserved() {
        let jats = r#"<article><body><p>a &custom; b</p></body></article>"#;
        let result = parse(jats).unwrap();
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
