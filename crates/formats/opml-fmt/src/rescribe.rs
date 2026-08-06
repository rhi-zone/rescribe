//! AST↔`rescribe::Document` translation for OPML.
//!
//! This module only translates between [`OpmlDoc`](crate::OpmlDoc) and
//! rescribe's `Document` IR — no XML tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.
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
//!   `list` of the same shape — proper nesting, not flattened.
//! - The paragraph's content is `text` (or, wrapped in a `link`, when
//!   `xmlUrl`/`htmlUrl` is present). Separately, **every** OPML attribute
//!   on the outline is raw-preserved verbatim as an `opml:attr:{name}`
//!   property on that same node — the writer reconstructs `Outline::attrs`
//!   from these, not by re-deriving them from the rendered text/link, so
//!   round-tripping through rescribe's IR is exact.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Head, OpmlDoc, Outline};
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

    /// Build the node representing a top-level outline: a bare `paragraph`
    /// if it has no children, or a `list` containing one `list_item` if it
    /// does.
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
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Body, Head, OpmlDoc as OpmlAst, Outline, Span};
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

        let opml_doc = OpmlAst {
            xml_decl: Some(crate::XmlDecl {
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
            let parsed = crate::rescribe::parse(opml).unwrap();
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
            let parsed = crate::rescribe::parse(opml).unwrap();
            let emitted = emit(&parsed.value).unwrap();
            let xml = String::from_utf8(emitted.value).unwrap();
            let (doc2, diags) = OpmlAst::parse(xml.as_bytes());
            assert!(diags.is_empty(), "diagnostics: {diags:?}");
            assert_eq!(doc2.body.outlines.len(), 1);
            assert_eq!(doc2.body.outlines[0].children.len(), 1);
            assert_eq!(doc2.body.outlines[0].children[0].text(), Some("Child"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::parse;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::emit;
