//! Full Typst-compiler evaluation path for rescribe.
//!
//! Unlike [`super::read`], which only walks `typst-fmt`'s syntax-only
//! `TypstDoc` AST, this module drives the real `typst` compiler (via
//! `typst`/`typst-kit`/`typst-html`) to resolve `#let`/`#for`/`#if`/show
//! rules etc. before producing rescribe IR, then walks the resulting
//! `typst-html` `HtmlDocument` tree. That HTML tree is itself an
//! already-parsed AST (produced upstream by the `typst` crate) — this
//! module's job is translating *that* structure to `Document`, not parsing
//! Typst source text, so it stays in step with `crate::rescribe`'s
//! AST→IR-translation-only contract. Preserved unchanged from the
//! pre-collapse `rescribe-read-typst` crate's `eval` feature.

use std::path::PathBuf;

use typst::Feature;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontSearcher;
use typst_syntax::{FileId, Source, VirtualPath};

use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Severity, WarningKind,
};
use rescribe_std::{node, prop};

use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

/// A minimal `World` implementation for single-file in-memory compilation.
struct MinimalWorld {
    library: LazyHash<Library>,
    font_book: LazyHash<FontBook>,
    fonts: Vec<typst_kit::fonts::FontSlot>,
    source: Source,
    main_id: FileId,
}

impl MinimalWorld {
    fn new(text: &str) -> Self {
        let library = LazyHash::new(
            Library::builder()
                .with_features(std::iter::once(Feature::Html).collect())
                .build(),
        );
        let main_id = FileId::new_fake(VirtualPath::new("input.typ"));
        let source = Source::new(main_id, text.to_string());
        let searched = FontSearcher::new().include_system_fonts(false).search();
        Self {
            library,
            font_book: LazyHash::new(searched.book),
            fonts: searched.fonts,
            source,
            main_id,
        }
    }
}

impl World for MinimalWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.font_book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(PathBuf::from(
                id.vpath().as_rootless_path(),
            )))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(PathBuf::from(
            id.vpath().as_rootless_path(),
        )))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)?.get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

/// Walk the root `HtmlDocument` and produce a flat list of IR block nodes.
fn convert_html_doc_to_nodes(html_doc: &HtmlDocument) -> Vec<Node> {
    // The root is always `<html>` containing `<head>` and `<body>`.
    // We skip head/html wrapper and walk body children directly.
    let mut blocks = Vec::new();
    collect_html_blocks(&html_doc.root, &mut blocks);
    blocks
}

/// Recursively collect block-level nodes from an `HtmlElement`.
fn collect_html_blocks(elem: &HtmlElement, out: &mut Vec<Node>) {
    let tag = elem.tag.resolve();
    let tag_str = tag.as_str();
    match tag_str {
        "html" | "body" => {
            // Transparent containers — descend into children.
            for child in &elem.children {
                collect_html_node_blocks(child, out);
            }
        }
        "head" => { /* skip head entirely */ }
        _ => {
            if let Some(n) = convert_html_element(elem) {
                out.push(n);
            }
        }
    }
}

fn collect_html_node_blocks(node: &HtmlNode, out: &mut Vec<Node>) {
    match node {
        HtmlNode::Element(elem) => collect_html_blocks(elem, out),
        HtmlNode::Text(text, _) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.push(
                    Node::new(node::PARAGRAPH)
                        .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, trimmed)]),
                );
            }
        }
        HtmlNode::Tag(_) | HtmlNode::Frame(_) => {}
    }
}

/// Convert a single `HtmlElement` to a rescribe `Node`.
fn convert_html_element(elem: &HtmlElement) -> Option<Node> {
    let tag = elem.tag.resolve();
    let tag_str = tag.as_str();
    match tag_str {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level = (tag_str.as_bytes()[1] - b'0') as i64;
            let children = collect_inline_children(elem);
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level)
                    .children(children),
            )
        }
        "p" => {
            let children = collect_inline_children(elem);
            Some(Node::new(node::PARAGRAPH).children(children))
        }
        "ul" => {
            let items = collect_list_items(elem);
            Some(
                Node::new(node::LIST)
                    .prop(prop::ORDERED, false)
                    .children(items),
            )
        }
        "ol" => {
            let items = collect_list_items(elem);
            Some(
                Node::new(node::LIST)
                    .prop(prop::ORDERED, true)
                    .children(items),
            )
        }
        "li" => {
            let children = collect_inline_children(elem);
            Some(
                Node::new(node::LIST_ITEM)
                    .children(vec![Node::new(node::PARAGRAPH).children(children)]),
            )
        }
        "pre" => {
            let text = extract_text_content(elem);
            Some(Node::new(node::CODE_BLOCK).prop(prop::CONTENT, text))
        }
        "blockquote" => {
            let mut inner = Vec::new();
            for child in &elem.children {
                collect_html_node_blocks(child, &mut inner);
            }
            if inner.is_empty() {
                let text = extract_text_content(elem);
                inner.push(
                    Node::new(node::PARAGRAPH)
                        .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, text)]),
                );
            }
            Some(Node::new(node::BLOCKQUOTE).children(inner))
        }
        "table" => {
            let rows = collect_table_rows(elem);
            Some(Node::new(node::TABLE).children(rows))
        }
        "figure" => {
            let mut children = Vec::new();
            for child in &elem.children {
                if let HtmlNode::Element(child_elem) = child {
                    let child_tag = child_elem.tag.resolve();
                    match child_tag.as_str() {
                        "figcaption" => {
                            let cap_children = collect_inline_children(child_elem);
                            children.push(Node::new(node::PARAGRAPH).children(cap_children));
                        }
                        _ => {
                            if let Some(n) = convert_html_element(child_elem) {
                                children.push(n);
                            }
                        }
                    }
                }
            }
            Some(Node::new(node::FIGURE).children(children))
        }
        "hr" => Some(Node::new(node::HORIZONTAL_RULE)),
        "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside" => {
            // Generic containers: collect block children.
            let mut inner = Vec::new();
            for child in &elem.children {
                collect_html_node_blocks(child, &mut inner);
            }
            Some(Node::new(node::DIV).children(inner))
        }
        _ => {
            // Try as an inline element wrapped in a paragraph.
            convert_html_inline(elem)
                .map(|inline| Node::new(node::PARAGRAPH).children(vec![inline]))
        }
    }
}

/// Convert an `HtmlElement` to an inline rescribe `Node`.
fn convert_html_inline(elem: &HtmlElement) -> Option<Node> {
    let tag = elem.tag.resolve();
    let tag_str = tag.as_str();
    match tag_str {
        "strong" | "b" => {
            let children = collect_inline_children(elem);
            Some(Node::new(node::STRONG).children(children))
        }
        "em" | "i" => {
            let children = collect_inline_children(elem);
            Some(Node::new(node::EMPHASIS).children(children))
        }
        "code" => {
            let text = extract_text_content(elem);
            Some(Node::new(node::CODE).prop(prop::CONTENT, text))
        }
        "u" => {
            let children = collect_inline_children(elem);
            Some(Node::new(node::UNDERLINE).children(children))
        }
        "s" | "del" => {
            let children = collect_inline_children(elem);
            Some(Node::new(node::STRIKEOUT).children(children))
        }
        "sub" => {
            let children = collect_inline_children(elem);
            Some(Node::new(node::SUBSCRIPT).children(children))
        }
        "sup" => {
            let children = collect_inline_children(elem);
            Some(Node::new(node::SUPERSCRIPT).children(children))
        }
        "a" => {
            let href = elem
                .attrs
                .0
                .iter()
                .find(|(k, _)| k.resolve().as_str() == "href")
                .map(|(_, v)| v.as_str().to_owned())
                .unwrap_or_default();
            let children = collect_inline_children(elem);
            let display = if children.is_empty() {
                vec![Node::new(node::TEXT).prop(prop::CONTENT, href.clone())]
            } else {
                children
            };
            Some(
                Node::new(node::LINK)
                    .prop(prop::URL, href)
                    .children(display),
            )
        }
        "img" => {
            let src = elem
                .attrs
                .0
                .iter()
                .find(|(k, _)| k.resolve().as_str() == "src")
                .map(|(_, v)| v.as_str().to_owned())
                .unwrap_or_default();
            let alt = elem
                .attrs
                .0
                .iter()
                .find(|(k, _)| k.resolve().as_str() == "alt")
                .map(|(_, v)| v.as_str().to_owned());
            let mut n = Node::new(node::IMAGE).prop(prop::URL, src);
            if let Some(alt_text) = alt {
                n = n.prop(prop::ALT, alt_text);
            }
            Some(n)
        }
        "br" => Some(Node::new(node::LINE_BREAK)),
        "span" => {
            let children = collect_inline_children(elem);
            if children.is_empty() {
                None
            } else {
                Some(Node::new(node::SPAN).children(children))
            }
        }
        _ => None,
    }
}

/// Collect inline children of an element (text + inline elements).
fn collect_inline_children(elem: &HtmlElement) -> Vec<Node> {
    let mut nodes = Vec::new();
    for child in &elem.children {
        match child {
            HtmlNode::Text(text, _) => {
                if !text.is_empty() {
                    nodes.push(Node::new(node::TEXT).prop(prop::CONTENT, text.as_str()));
                }
            }
            HtmlNode::Element(child_elem) => {
                if let Some(inline) = convert_html_inline(child_elem) {
                    nodes.push(inline);
                } else if let Some(block) = convert_html_element(child_elem) {
                    // Block nested in inline context — unwrap if paragraph, else raw.
                    nodes.push(block);
                }
            }
            HtmlNode::Tag(_) | HtmlNode::Frame(_) => {}
        }
    }
    nodes
}

/// Extract all text content from an element recursively.
fn extract_text_content(elem: &HtmlElement) -> String {
    let mut buf = String::new();
    extract_text_recursive(&elem.children, &mut buf);
    buf
}

fn extract_text_recursive(children: &[HtmlNode], buf: &mut String) {
    for child in children {
        match child {
            HtmlNode::Text(text, _) => buf.push_str(text.as_str()),
            HtmlNode::Element(elem) => extract_text_recursive(&elem.children, buf),
            HtmlNode::Tag(_) | HtmlNode::Frame(_) => {}
        }
    }
}

/// Collect `<li>` items from a list element.
fn collect_list_items(elem: &HtmlElement) -> Vec<Node> {
    let mut items = Vec::new();
    for child in &elem.children {
        if let HtmlNode::Element(child_elem) = child {
            let tag = child_elem.tag.resolve();
            if tag.as_str() == "li" {
                let children = collect_inline_children(child_elem);
                items.push(
                    Node::new(node::LIST_ITEM)
                        .children(vec![Node::new(node::PARAGRAPH).children(children)]),
                );
            }
        }
    }
    items
}

/// Collect table rows from a `<table>` element.
fn collect_table_rows(elem: &HtmlElement) -> Vec<Node> {
    let mut rows = Vec::new();
    for child in &elem.children {
        if let HtmlNode::Element(child_elem) = child {
            let tag = child_elem.tag.resolve();
            match tag.as_str() {
                "tr" => {
                    rows.push(convert_table_row(child_elem));
                }
                "thead" | "tbody" | "tfoot" => {
                    // Recurse into section containers.
                    for inner in &child_elem.children {
                        if let HtmlNode::Element(row_elem) = inner {
                            let row_tag = row_elem.tag.resolve();
                            if row_tag.as_str() == "tr" {
                                rows.push(convert_table_row(row_elem));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    rows
}

fn convert_table_row(elem: &HtmlElement) -> Node {
    let mut cells = Vec::new();
    for child in &elem.children {
        if let HtmlNode::Element(child_elem) = child {
            let tag = child_elem.tag.resolve();
            let kind = match tag.as_str() {
                "th" => node::TABLE_HEADER,
                "td" => node::TABLE_CELL,
                _ => continue,
            };
            let children = collect_inline_children(child_elem);
            cells.push(Node::new(kind).children(children));
        }
    }
    Node::new(node::TABLE_ROW).children(cells)
}

/// Parse Typst source through the full compiler, resolving `#let`, `#for`, `#if`,
/// show rules, etc., before converting to rescribe IR.
///
/// Falls back to the syntax-only [`super::read::parse`] result (with a
/// warning attached) if compilation fails.
pub fn parse_evaluated(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let world = MinimalWorld::new(input);
    let result = typst::compile::<HtmlDocument>(&world);

    let warning_msgs: Vec<String> = result
        .warnings
        .iter()
        .map(|w| w.message.to_string())
        .collect();

    match result.output {
        Ok(html_doc) => {
            let blocks = convert_html_doc_to_nodes(&html_doc);
            let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(blocks));
            let mut cr = ConversionResult::ok(doc);
            for w in warning_msgs {
                cr = cr.warn(FidelityWarning::new(
                    Severity::Info,
                    WarningKind::FeatureLost("typst-compile-warning".to_owned()),
                    w,
                ));
            }
            Ok(cr)
        }
        Err(errors) => {
            // Compilation failed — fall back to syntax-only parse with warnings.
            let mut cr = super::read::parse(input)?;
            for e in errors.iter() {
                cr = cr.warn(FidelityWarning::new(
                    Severity::Major,
                    WarningKind::FeatureLost("typst-compile-error".to_owned()),
                    format!("typst compile error: {}", e.message),
                ));
            }
            for w in warning_msgs {
                cr = cr.warn(FidelityWarning::new(
                    Severity::Info,
                    WarningKind::FeatureLost("typst-compile-warning".to_owned()),
                    w,
                ));
            }
            Ok(cr)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_evaluated_basic() {
        let result = parse_evaluated("= Hello\n\nWorld paragraph.").unwrap();
        let doc = &result.value;
        // Should have at least a heading and a paragraph.
        assert!(
            doc.content.children.len() >= 2,
            "Expected at least heading + paragraph, got: {:?}",
            doc.content
                .children
                .iter()
                .map(|n| n.kind.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_evaluated_let_binding() {
        // #let resolves at eval time; the text "Alice" should appear.
        let result = parse_evaluated("#let name = \"Alice\"\nHello, #name!").unwrap();
        let doc = &result.value;
        // Walk all text nodes to find "Alice".
        fn has_text(node: &rescribe_core::Node, needle: &str) -> bool {
            if node.kind.as_str() == rescribe_std::node::TEXT
                && let Some(content) = node.props.get_str(rescribe_std::prop::CONTENT)
                && content.contains(needle)
            {
                return true;
            }
            node.children.iter().any(|c| has_text(c, needle))
        }
        assert!(
            has_text(&doc.content, "Alice"),
            "Expected evaluated text 'Alice' in document"
        );
    }

    #[test]
    fn test_parse_evaluated_fallback_on_error() {
        // Intentionally broken typst (missing closing brace) should not panic;
        // it should return a ConversionResult (possibly falling back to syntax parse).
        let result = parse_evaluated("= Heading\n\n#let x = {");
        assert!(result.is_ok(), "Should not error even on broken typst");
    }
}
