//! Typst reader for rescribe.
//!
//! Thin AST→IR translator over `typst-fmt`'s `TypstDoc`/`Block`/`Inline` —
//! all Typst parsing lives in that standalone crate now (see its module
//! docs), not here. This adapter's only job is mapping `typst-fmt`'s
//! domain-typed AST onto rescribe's `Node` tree; the construct mapping
//! (which node kind/property each `Block`/`Inline` variant becomes) is
//! unchanged from the pre-extraction version of this file.
//!
//! # Features
//! - `syntax` (default): fast parse-only path via `typst-fmt`
//! - `eval`: full compiler path via the `typst` crate; adds `parse_evaluated()`

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_format_api::Parse as _;
use rescribe_std::{node, prop};
use typst_fmt::{Block, Inline, TypstDoc};

/// Parse Typst source into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Typst source with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, _diags) = TypstDoc::parse(input.as_bytes());
    let children: Vec<Node> = doc.blocks.iter().map(convert_block).collect();
    let doc_node = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(doc_node);
    Ok(ConversionResult::ok(doc))
}

fn convert_blocks(blocks: &[Block]) -> Vec<Node> {
    blocks.iter().map(convert_block).collect()
}

fn convert_inlines(inlines: &[Inline]) -> Vec<Node> {
    inlines.iter().map(convert_inline).collect()
}

fn convert_block(block: &Block) -> Node {
    match block {
        Block::Paragraph(inlines) => Node::new(node::PARAGRAPH).children(convert_inlines(inlines)),
        Block::Heading { level, body } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(convert_inlines(body)),
        Block::CodeBlock { lang, content } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = lang
                && !lang.is_empty()
            {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }
        Block::List { ordered, items } => Node::new(node::LIST)
            .prop(prop::ORDERED, *ordered)
            .children(items.iter().map(|item| convert_list_item(item))),
        Block::DefinitionList(entries) => {
            let mut children = Vec::with_capacity(entries.len() * 2);
            for (term, desc) in entries {
                children.push(Node::new(node::DEFINITION_TERM).children(convert_inlines(term)));
                children.push(Node::new(node::DEFINITION_DESC).children(convert_inlines(desc)));
            }
            Node::new(node::DEFINITION_LIST).children(children)
        }
        Block::Quote(body) => Node::new(node::BLOCKQUOTE).children(convert_blocks(body)),
        Block::Table { columns, rows } => {
            let row_nodes: Vec<Node> =
                rows.iter()
                    .map(|row| {
                        Node::new(node::TABLE_ROW).children(row.iter().map(|cell| {
                            Node::new(node::TABLE_CELL).children(convert_inlines(cell))
                        }))
                    })
                    .collect();
            Node::new(node::TABLE)
                .prop("columns", *columns as i64)
                .children(row_nodes)
        }
        Block::Figure { body, caption } => {
            let mut children = Vec::new();
            if let Some(b) = body {
                children.push(convert_block(b));
            }
            if let Some(cap) = caption {
                children.push(Node::new(node::PARAGRAPH).children(convert_inlines(cap)));
            }
            Node::new(node::FIGURE).children(children)
        }
        Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
        Block::MathDisplay(source) => Node::new("math_display").prop("math:source", source.clone()),
        Block::Image { url } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),
        Block::Raw(text) => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, "typst")
            .prop(prop::CONTENT, text.clone()),
    }
}

fn convert_list_item(item: &[Block]) -> Node {
    Node::new(node::LIST_ITEM).children(convert_blocks(item))
}

fn convert_inline(inline: &Inline) -> Node {
    match inline {
        Inline::Text(t) => Node::new(node::TEXT).prop(prop::CONTENT, t.clone()),
        Inline::Strong(body) => Node::new(node::STRONG).children(convert_inlines(body)),
        Inline::Emph(body) => Node::new(node::EMPHASIS).children(convert_inlines(body)),
        Inline::Underline(body) => Node::new(node::UNDERLINE).children(convert_inlines(body)),
        Inline::Strike(body) => Node::new(node::STRIKEOUT).children(convert_inlines(body)),
        Inline::Subscript(body) => Node::new(node::SUBSCRIPT).children(convert_inlines(body)),
        Inline::Superscript(body) => Node::new(node::SUPERSCRIPT).children(convert_inlines(body)),
        Inline::Code(content) => Node::new(node::CODE).prop(prop::CONTENT, content.clone()),
        Inline::Link { url, body } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .children(convert_inlines(body)),
        Inline::Image { url } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),
        Inline::LineBreak => Node::new(node::LINE_BREAK),
        Inline::MathInline(source) => Node::new("math_inline").prop("math:source", source.clone()),
        Inline::MathDisplay(source) => Node::new("math_block").prop("math:source", source.clone()),
        Inline::Footnote(body) => Node::new(node::FOOTNOTE_DEF).children(convert_inlines(body)),
        Inline::SmallCaps(body) => Node::new(node::SMALL_CAPS).children(convert_inlines(body)),
        Inline::Quoted { double, body } => Node::new(node::QUOTED)
            .prop(prop::QUOTE_TYPE, if *double { "double" } else { "single" })
            .children(convert_inlines(body)),
        Inline::Raw(text) => Node::new(node::RAW_BLOCK)
            .prop(prop::FORMAT, "typst")
            .prop(prop::CONTENT, text.clone()),
    }
}

// ---------------------------------------------------------------------------
// Eval path (requires `eval` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "eval")]
mod eval_impl {
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
    pub struct MinimalWorld {
        library: LazyHash<Library>,
        font_book: LazyHash<FontBook>,
        fonts: Vec<typst_kit::fonts::FontSlot>,
        source: Source,
        main_id: FileId,
    }

    impl MinimalWorld {
        pub fn new(text: &str) -> Self {
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
    pub fn convert_html_doc_to_nodes(html_doc: &HtmlDocument) -> Vec<Node> {
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
    /// Falls back to the syntax-only `parse()` result (with a warning attached) if
    /// compilation fails.
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
                let mut cr = super::parse(input)?;
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
}

/// Parse Typst source through the full typst compiler (resolving `#let`, `#for`, `#if`,
/// show rules, etc.) before converting to the rescribe IR.
///
/// On compilation failure, falls back to the syntax-only result with errors attached as
/// fidelity warnings.
///
/// Only available with the `eval` feature.
#[cfg(feature = "eval")]
pub fn parse_evaluated(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    eval_impl::parse_evaluated(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> Document {
        parse(input).unwrap().value
    }

    #[test]
    fn test_parse_heading() {
        let doc = parse_str("= Title");
        let heading = &doc.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse_str("= Level 1\n\n== Level 2\n\n=== Level 3");
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        assert_eq!(doc.content.children[1].props.get_int(prop::LEVEL), Some(2));
        assert_eq!(doc.content.children[2].props.get_int(prop::LEVEL), Some(3));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world!");
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("This is *bold* text.");
        let para = &doc.content.children[0];
        let strong = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::STRONG);
        assert!(strong.is_some(), "Expected a strong node in paragraph");
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("This is _italic_ text.");
        let para = &doc.content.children[0];
        let emph = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::EMPHASIS);
        assert!(emph.is_some(), "Expected an emphasis node in paragraph");
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("Use `code` here.");
        let para = &doc.content.children[0];
        let code = para.children.iter().find(|c| c.kind.as_str() == node::CODE);
        assert!(code.is_some(), "Expected a code node in paragraph");
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_str("```rust\nfn main() {}\n```");
        let code = &doc.content.children[0];
        assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
        assert_eq!(code.props.get_str(prop::LANGUAGE), Some("rust"));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_str("- Item 1\n- Item 2");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_str("+ First\n+ Second");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    fn test_parse_image() {
        let doc = parse_str("#image(\"photo.png\")");
        let img = &doc.content.children[0];
        assert_eq!(img.kind.as_str(), node::IMAGE);
        assert_eq!(img.props.get_str(prop::URL), Some("photo.png"));
    }

    #[test]
    fn test_parse_math_inline() {
        let doc = parse_str("Here is $x^2$ math.");
        let para = &doc.content.children[0];
        let math = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == "math_inline");
        assert!(math.is_some(), "Expected a math_inline node");
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("Visit https://typst.app for info.");
        let para = &doc.content.children[0];
        let link = para.children.iter().find(|c| c.kind.as_str() == node::LINK);
        assert!(link.is_some(), "Expected a link node");
    }

    #[test]
    fn test_parse_term_list() {
        let doc = parse_str("/ Rust: A systems language\n/ Python: A scripting language");
        assert_eq!(
            doc.content.children.len(),
            1,
            "Adjacent term items should merge"
        );
        let dl = &doc.content.children[0];
        assert_eq!(dl.kind.as_str(), node::DEFINITION_LIST);
        // Two terms merged: 4 children total (term+desc, term+desc).
        assert_eq!(dl.children.len(), 4);
        assert_eq!(dl.children[0].kind.as_str(), node::DEFINITION_TERM);
        assert_eq!(dl.children[1].kind.as_str(), node::DEFINITION_DESC);
    }

    #[test]
    fn test_parse_footnote() {
        let doc = parse_str("#footnote[A note here]");
        // Footnotes are inline; they end up inside a paragraph.
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
        let footnote = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::FOOTNOTE_DEF);
        assert!(
            footnote.is_some(),
            "Expected a footnote_def node in paragraph"
        );
        assert!(!footnote.unwrap().children.is_empty());
    }

    #[test]
    fn test_parse_sub_super() {
        let doc = parse_str("#sub[2] and #super[3]");
        let para = &doc.content.children[0];
        let sub = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::SUBSCRIPT);
        assert!(sub.is_some(), "Expected subscript node");
        let sup = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::SUPERSCRIPT);
        assert!(sup.is_some(), "Expected superscript node");
    }

    #[test]
    fn test_parse_underline_strike() {
        let doc = parse_str("#underline[hello] and #strike[world]");
        let para = &doc.content.children[0];
        let u = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::UNDERLINE);
        assert!(u.is_some(), "Expected underline node");
        let s = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::STRIKEOUT);
        assert!(s.is_some(), "Expected strikeout node");
    }

    #[test]
    fn test_parse_table() {
        let doc = parse_str("#table(columns: 2, [A], [B], [C], [D])");
        let table = &doc.content.children[0];
        assert_eq!(table.kind.as_str(), node::TABLE);
        // 4 cells / 2 columns = 2 rows
        assert_eq!(table.children.len(), 2, "Expected 2 rows");
        assert_eq!(table.children[0].kind.as_str(), node::TABLE_ROW);
        assert_eq!(
            table.children[0].children.len(),
            2,
            "Expected 2 cells per row"
        );
    }

    #[test]
    fn test_parse_figure() {
        let doc = parse_str("#figure(image(\"cat.png\"), caption: [A cat])");
        let figure = &doc.content.children[0];
        assert_eq!(figure.kind.as_str(), node::FIGURE);
        // First child should be an image.
        assert_eq!(figure.children[0].kind.as_str(), node::IMAGE);
        // Second child should be a paragraph (caption).
        assert_eq!(figure.children[1].kind.as_str(), node::PARAGRAPH);
    }

    #[cfg(feature = "eval")]
    mod eval_tests {
        use super::super::*;

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
}
