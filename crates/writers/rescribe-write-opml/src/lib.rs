//! OPML writer for rescribe.
//!
//! Thin IR→AST adapter over [`opml_fmt`] (the standalone OPML parser/AST/
//! emitter crate). All XML emission lives in `opml-fmt`; this crate only
//! translates rescribe's `Document` into `opml_fmt::OpmlDoc`, then calls
//! `opml_fmt::emit`. Per CLAUDE.md's adapter-layer rule, no `quick_xml`
//! appears in this crate's production code.
//!
//! # Mapping (inverse of `rescribe-read-opml`)
//!
//! - A `paragraph` (or `list_item` wrapping a `paragraph` plus a nested
//!   `list`) that carries `opml:attr:*` properties round-trips exactly:
//!   those properties *are* `Outline::attrs`, used verbatim rather than
//!   re-derived from the rendered text/link — this is what makes
//!   `rescribe-read-opml` → `rescribe-write-opml` lossless.
//! - A `paragraph`/`list`/heading/etc. with no `opml:attr:*` properties
//!   (content that did not originate from OPML — e.g. converting a Markdown
//!   document to OPML) is a foreign node: its `text`/`link` content is
//!   extracted and synthesized into `text`/`xmlUrl` attributes instead, so
//!   any document can still be written as OPML.

use opml_fmt::{Body, Head, OpmlDoc, Outline, Span};
use rescribe_core::{ConversionResult, Document, EmitError, Node};
use rescribe_format_api::Emit as _;
use rescribe_std::{node, prop};

/// Emit a document as OPML.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let head = build_head(doc);
    let version = doc
        .metadata
        .get_str("opml:version")
        .map(str::to_string)
        .unwrap_or_else(|| "2.0".to_string());

    let mut outlines = Vec::new();
    for child in &doc.content.children {
        push_top_outline(child, &mut outlines);
    }

    let opml_doc = OpmlDoc {
        xml_decl: Some(opml_fmt::XmlDecl {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
        }),
        version,
        head,
        body: Body {
            outlines,
            span: Span::NONE,
        },
        span: Span::NONE,
    };

    let bytes = opml_doc.emit();
    Ok(ConversionResult::ok(bytes))
}

fn build_head(doc: &Document) -> Head {
    let m = &doc.metadata;
    Head {
        title: m.get_str("title").map(str::to_string),
        date_created: m.get_str("opml:date_created").map(str::to_string),
        date_modified: m.get_str("opml:date_modified").map(str::to_string),
        owner_name: m.get_str("author").map(str::to_string),
        owner_email: m.get_str("opml:owner_email").map(str::to_string),
        owner_id: m.get_str("opml:owner_id").map(str::to_string),
        docs: m.get_str("opml:docs").map(str::to_string),
        expansion_state: m.get_str("opml:expansion_state").map(str::to_string),
        vert_scroll_state: m.get_str("opml:vert_scroll_state").map(str::to_string),
        window_top: m.get_str("opml:window_top").map(str::to_string),
        window_left: m.get_str("opml:window_left").map(str::to_string),
        window_bottom: m.get_str("opml:window_bottom").map(str::to_string),
        window_right: m.get_str("opml:window_right").map(str::to_string),
        extra: m
            .iter()
            .filter_map(|(k, v)| {
                k.strip_prefix("opml:head_extra:").map(|name| {
                    let value = match v {
                        rescribe_core::PropValue::String(s) => s.clone(),
                        other => format!("{other:?}"),
                    };
                    (name.to_string(), value)
                })
            })
            .collect(),
        span: Span::NONE,
    }
}

fn push_top_outline(n: &Node, out: &mut Vec<Outline>) {
    match n.kind.as_str() {
        node::PARAGRAPH => out.push(outline_from_paragraph(n)),
        node::LIST => {
            for item in &n.children {
                if item.kind.as_str() == node::LIST_ITEM {
                    out.push(outline_from_list_item(item));
                }
            }
        }
        _ => out.push(outline_from_text_fallback(n)),
    }
}

/// Build an `Outline` from a `list_item`: its first non-`list` child is
/// treated as the item's own content (a `paragraph`, ideally); any `list`
/// child holds nested outlines.
fn outline_from_list_item(item: &Node) -> Outline {
    let mut own: Option<&Node> = None;
    let mut children = Vec::new();
    for child in &item.children {
        if child.kind.as_str() == node::LIST {
            for li in &child.children {
                if li.kind.as_str() == node::LIST_ITEM {
                    children.push(outline_from_list_item(li));
                } else {
                    children.push(outline_from_text_fallback(li));
                }
            }
        } else if own.is_none() {
            own = Some(child);
        }
    }

    let mut o = match own {
        Some(p) if p.kind.as_str() == node::PARAGRAPH => outline_from_paragraph(p),
        Some(other) => outline_from_text_fallback(other),
        None => Outline {
            attrs: vec![("text".to_string(), String::new())],
            children: Vec::new(),
            self_closing: true,
            span: Span::NONE,
        },
    };
    let has_children = !children.is_empty();
    o.children = children;
    if has_children {
        o.self_closing = false;
    }
    o
}

/// Build an `Outline` from a `paragraph`. If it carries `opml:attr:*`
/// properties (round-tripping content originally read from OPML), those are
/// the authoritative attribute set — used verbatim, not re-derived. Content
/// without them (a foreign paragraph, e.g. from a Markdown document being
/// converted to OPML) has its `text`/`link` extracted and synthesized into
/// `text`/`xmlUrl` attributes instead.
fn outline_from_paragraph(para: &Node) -> Outline {
    let mut attrs: Vec<(String, String)> = para
        .props
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix("opml:attr:").map(|name| {
                let value = match v {
                    rescribe_core::PropValue::String(s) => s.clone(),
                    other => format!("{other:?}"),
                };
                (name.to_string(), value)
            })
        })
        .collect();

    let self_closing = para.props.get_bool("opml:self_closing").unwrap_or(true);

    if attrs.is_empty() {
        let (text, url) = extract_para_content(para);
        attrs.push(("text".to_string(), text));
        if let Some(url) = url {
            attrs.push(("xmlUrl".to_string(), url));
        }
    }

    Outline {
        attrs,
        children: Vec::new(),
        self_closing,
        span: Span::NONE,
    }
}

/// Fallback for any node kind that isn't a `paragraph`/`list_item` (e.g. a
/// `heading` in foreign input) — extracts its text content into a plain
/// `text` outline so nothing is silently dropped.
fn outline_from_text_fallback(n: &Node) -> Outline {
    let text = extract_text(n);
    Outline {
        attrs: vec![("text".to_string(), text)],
        children: Vec::new(),
        self_closing: true,
        span: Span::NONE,
    }
}

fn extract_para_content(node: &Node) -> (String, Option<String>) {
    let mut text = String::new();
    let mut url: Option<String> = None;

    for child in &node.children {
        match child.kind.as_str() {
            node::TEXT => {
                if let Some(content) = child.props.get_str(prop::CONTENT) {
                    text.push_str(content);
                }
            }
            node::LINK => {
                if url.is_none() {
                    url = child.props.get_str(prop::URL).map(|s| s.to_string());
                }
                text.push_str(&extract_text(child));
            }
            _ => text.push_str(&extract_text(child)),
        }
    }

    (text, url)
}

fn extract_text(node: &Node) -> String {
    let mut out = String::new();
    extract_text_recursive(node, &mut out);
    out
}

fn extract_text_recursive(node: &Node, out: &mut String) {
    if node.kind.as_str() == node::TEXT
        && let Some(content) = node.props.get_str(prop::CONTENT)
    {
        out.push_str(content);
    }
    for child in &node.children {
        extract_text_recursive(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_format_api::Parse as _;
    use rescribe_std::builder::doc;

    #[test]
    fn test_emit_simple_list() {
        let document =
            doc(|d| d.bullet_list(|l| l.item(|i| i.text("Item 1")).item(|i| i.text("Item 2"))));

        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("<opml"));
        assert!(output.contains("Item 1"));
        assert!(output.contains("Item 2"));
    }

    #[test]
    fn test_emit_with_metadata() {
        let mut document = doc(|d| d.para(|i| i.text("Test")));
        document.metadata.set("title", "My Outline");
        document.metadata.set("author", "John Doe");

        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("<title>My Outline</title>"));
        assert!(output.contains("<ownerName>John Doe</ownerName>"));
    }

    #[test]
    fn test_roundtrip_preserves_outline_attrs() {
        let opml = r#"<opml version="2.0"><body>
  <outline text="X" isComment="true" appSpecific="v"/>
</body></opml>"#;
        let parsed = rescribe_read_opml::parse(opml).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(xml.contains(r#"text="X""#));
        assert!(xml.contains(r#"isComment="true""#));
        assert!(xml.contains(r#"appSpecific="v""#));
    }

    #[test]
    fn test_roundtrip_preserves_nesting() {
        let opml = r#"<opml version="2.0"><body>
  <outline text="Parent"><outline text="Child"/></outline>
</body></opml>"#;
        let parsed = rescribe_read_opml::parse(opml).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        let (doc2, diags) = OpmlDoc::parse(xml.as_bytes());
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc2.body.outlines.len(), 1);
        assert_eq!(doc2.body.outlines[0].children.len(), 1);
        assert_eq!(doc2.body.outlines[0].children[0].text(), Some("Child"));
    }
}
