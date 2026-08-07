//! AST↔`rescribe::Document` translation for DokuWiki.
//!
//! This module only translates between [`DokuwikiDoc`](crate::DokuwikiDoc)
//! and rescribe's `Document` IR — no DokuWiki markup parsing/emitting
//! happens here (that lives in the rest of this crate; see `crate::parse`
//! and `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block as FmtBlock, Inline as FmtInline};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_std::{node, prop};

    /// Parse DokuWiki source into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse DokuWiki source with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (doc, _diagnostics) = crate::parse::parse(input);

        let mut children = Vec::new();
        for block in &doc.blocks {
            children.push(convert_block(block));
        }

        let root = Node::new(node::DOCUMENT).children(children);
        let doc = Document::new().with_content(root);
        Ok(ConversionResult::ok(doc))
    }

    fn convert_block(block: &FmtBlock) -> Node {
        match block {
            FmtBlock::Paragraph { inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
                Node::new(node::PARAGRAPH).children(children)
            }

            FmtBlock::Heading { level, inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(children)
            }

            FmtBlock::CodeBlock {
                language, content, ..
            } => {
                let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                if let Some(lang) = language {
                    n = n.prop(prop::LANGUAGE, lang.clone());
                }
                n
            }

            FmtBlock::Blockquote { children, .. } => {
                let converted: Vec<Node> = children.iter().map(convert_block).collect();
                Node::new(node::BLOCKQUOTE).children(converted)
            }

            FmtBlock::List { ordered, items, .. } => {
                let mut list_items = Vec::new();
                for item in items {
                    let mut item_children: Vec<Node> =
                        item.inlines.iter().map(convert_inline).collect();
                    for block in &item.children {
                        item_children.push(convert_block(block));
                    }
                    list_items.push(Node::new(node::LIST_ITEM).children(item_children));
                }
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            FmtBlock::HorizontalRule(_) => Node::new(node::HORIZONTAL_RULE),

            FmtBlock::FileBlock {
                language,
                filename,
                content,
                ..
            } => {
                let mut n = Node::new("file_block").prop(prop::CONTENT, content.clone());
                if let Some(lang) = language {
                    n = n.prop(prop::LANGUAGE, lang.clone());
                }
                if let Some(fname) = filename {
                    n = n.prop("dokuwiki:filename", fname.clone());
                }
                n
            }

            FmtBlock::Table { rows, .. } => {
                let row_nodes: Vec<Node> = rows
                    .iter()
                    .map(|row| {
                        let cells: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                let kind = if row.is_header {
                                    node::TABLE_HEADER
                                } else {
                                    node::TABLE_CELL
                                };
                                Node::new(kind).children(cell.inlines.iter().map(convert_inline))
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(row_nodes)
            }

            FmtBlock::DefinitionList { items, .. } => {
                let mut def_nodes = Vec::new();
                for item in items {
                    def_nodes.push(
                        Node::new(node::DEFINITION_TERM)
                            .children(item.term.iter().map(convert_inline)),
                    );
                    def_nodes.push(
                        Node::new(node::DEFINITION_DESC)
                            .children(item.desc.iter().map(convert_inline)),
                    );
                }
                Node::new(node::DEFINITION_LIST).children(def_nodes)
            }

            FmtBlock::RawBlock {
                format, content, ..
            } => Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, format.clone())
                .prop(prop::CONTENT, content.clone()),

            FmtBlock::Macro { name, .. } => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "dokuwiki")
                .prop(prop::CONTENT, format!("{{{{{}}}}}", name)),
        }
    }

    fn convert_inline(inline: &FmtInline) -> Node {
        match inline {
            FmtInline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            FmtInline::Bold(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::STRONG).children(converted)
            }

            FmtInline::Italic(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::EMPHASIS).children(converted)
            }

            FmtInline::Underline(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::UNDERLINE).children(converted)
            }

            FmtInline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            FmtInline::Link { url, children, .. } => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(converted)
            }

            FmtInline::Image { url, alt, .. } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if let Some(alt_text) = alt {
                    n = n.prop(prop::ALT, alt_text.clone());
                }
                n
            }

            FmtInline::Strikethrough(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::STRIKEOUT).children(converted)
            }

            FmtInline::Superscript(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::SUPERSCRIPT).children(converted)
            }

            FmtInline::Subscript(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::SUBSCRIPT).children(converted)
            }

            FmtInline::Nowiki(s, _) => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "dokuwiki")
                .prop(prop::CONTENT, s.clone()),

            FmtInline::FootnoteRef { content, .. } => {
                Node::new(node::FOOTNOTE_REF).prop(prop::CONTENT, content.clone())
            }

            FmtInline::LineBreak(_) => Node::new(node::LINE_BREAK),
            FmtInline::SoftBreak(_) => Node::new(node::SOFT_BREAK),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn parse_str(input: &str) -> Document {
            parse(input).unwrap().value
        }

        #[test]
        fn test_parse_heading() {
            let doc = parse_str("====== Title ======");
            let heading = &doc.content.children[0];
            assert_eq!(heading.kind.as_str(), node::HEADING);
            assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
        }

        #[test]
        fn test_parse_heading_levels() {
            let doc = parse_str("====== H1 ======\n===== H2 =====\n==== H3 ====");
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
            let doc = parse_str("This is **bold** text.");
            let para = &doc.content.children[0];
            assert_eq!(para.children[1].kind.as_str(), node::STRONG);
        }

        #[test]
        fn test_parse_italic() {
            let doc = parse_str("This is //italic// text.");
            let para = &doc.content.children[0];
            assert_eq!(para.children[1].kind.as_str(), node::EMPHASIS);
        }

        #[test]
        fn test_parse_code() {
            let doc = parse_str("Use ''code'' here.");
            let para = &doc.content.children[0];
            assert_eq!(para.children[1].kind.as_str(), node::CODE);
        }

        #[test]
        fn test_parse_link() {
            let doc = parse_str("Click [[https://example.com|here]].");
            let para = &doc.content.children[0];
            let link = &para.children[1];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
        }

        #[test]
        fn test_parse_list() {
            let doc = parse_str("  * Item 1\n  * Item 2");
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_code_block() {
            let doc = parse_str("<code rust>\nfn main() {}\n</code>");
            let code = &doc.content.children[0];
            assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
            assert_eq!(code.props.get_str(prop::LANGUAGE), Some("rust"));
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block as FmtBlock, DokuwikiDoc, Inline as FmtInline};
    use rescribe_core::{
        ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
        WarningKind,
    };
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document as DokuWiki.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as DokuWiki with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut warnings = Vec::new();
        let blocks = convert_nodes(&doc.content.children, &mut warnings);
        let fmt_doc = DokuwikiDoc { blocks };
        let output = fmt_doc.emit();

        Ok(ConversionResult::with_warnings(output, warnings))
    }

    fn convert_nodes(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<FmtBlock> {
        nodes
            .iter()
            .filter_map(|n| convert_node(n, warnings))
            .collect()
    }

    fn convert_node(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Option<FmtBlock> {
        match node.kind.as_str() {
            node::DOCUMENT => {
                let children = convert_nodes(&node.children, warnings);
                if children.is_empty() {
                    None
                } else if children.len() == 1 {
                    Some(children.into_iter().next().unwrap())
                } else {
                    // Wrap multiple top-level blocks; shouldn't happen but just in case
                    Some(FmtBlock::Paragraph {
                        inlines: vec![],
                        span: crate::Span::NONE,
                    })
                }
            }

            node::PARAGRAPH => {
                let inlines = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtBlock::Paragraph {
                    inlines,
                    span: crate::Span::NONE,
                })
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                let inlines = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtBlock::Heading {
                    level,
                    inlines,
                    span: crate::Span::NONE,
                })
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                Some(FmtBlock::CodeBlock {
                    language,
                    content,
                    span: crate::Span::NONE,
                })
            }

            node::BLOCKQUOTE => {
                let children = convert_nodes(&node.children, warnings);
                Some(FmtBlock::Blockquote {
                    children,
                    span: crate::Span::NONE,
                })
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let mut items = Vec::new();
                for child in &node.children {
                    if child.kind.as_str() == node::LIST_ITEM {
                        // Collect inlines: direct inline children OR paragraph children's inlines
                        let mut inlines: Vec<crate::Inline> = Vec::new();
                        let mut nested_children: Vec<crate::Block> = Vec::new();
                        for item_child in &child.children {
                            match item_child.kind.as_str() {
                                node::PARAGRAPH => {
                                    inlines.extend(
                                        item_child
                                            .children
                                            .iter()
                                            .filter_map(|n| convert_inline(n, warnings)),
                                    );
                                }
                                node::LIST => {
                                    if let Some(b) = convert_node(item_child, warnings) {
                                        nested_children.push(b);
                                    }
                                }
                                _ => {
                                    if let Some(il) = convert_inline(item_child, warnings) {
                                        inlines.push(il);
                                    }
                                }
                            }
                        }
                        items.push(crate::ListItem {
                            inlines,
                            children: nested_children,
                        });
                    }
                }
                Some(FmtBlock::List {
                    ordered,
                    items,
                    span: crate::Span::NONE,
                })
            }

            node::HORIZONTAL_RULE => Some(FmtBlock::HorizontalRule(crate::Span::NONE)),

            node::LIST_ITEM
            | node::TABLE
            | node::TABLE_ROW
            | node::TABLE_CELL
            | node::TABLE_HEAD
            | node::TABLE_BODY
            | node::TABLE_FOOT
            | node::FIGURE
            | node::DIV
            | node::SPAN
            | node::RAW_BLOCK
            | node::RAW_INLINE
            | node::DEFINITION_LIST
            | node::DEFINITION_TERM
            | node::DEFINITION_DESC => {
                // Try to preserve content where possible
                if child_is_simple(&node.children) {
                    let inlines: Vec<FmtInline> = node
                        .children
                        .iter()
                        .filter_map(|n| convert_inline(n, warnings))
                        .collect();
                    if !inlines.is_empty() {
                        return Some(FmtBlock::Paragraph {
                            inlines,
                            span: crate::Span::NONE,
                        });
                    }
                }
                None
            }

            _ => {
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                    format!(
                        "Unsupported block type for DokuWiki: {}",
                        node.kind.as_str()
                    ),
                ));
                None
            }
        }
    }

    fn child_is_simple(children: &[Node]) -> bool {
        !children.is_empty()
            && children.iter().all(|n| {
                matches!(
                    n.kind.as_str(),
                    node::TEXT
                        | node::EMPHASIS
                        | node::STRONG
                        | node::CODE
                        | node::LINK
                        | node::IMAGE
                        | node::LINE_BREAK
                        | node::SOFT_BREAK
                        | node::UNDERLINE
                )
            })
    }

    fn convert_inline(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Option<FmtInline> {
        match node.kind.as_str() {
            node::TEXT => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Some(FmtInline::Text(content, crate::Span::NONE))
            }

            node::EMPHASIS => {
                let children = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtInline::Italic(children, crate::Span::NONE))
            }

            node::STRONG => {
                let children = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtInline::Bold(children, crate::Span::NONE))
            }

            node::UNDERLINE => {
                let children = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtInline::Underline(children, crate::Span::NONE))
            }

            node::CODE => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Some(FmtInline::Code(content, crate::Span::NONE))
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let children = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtInline::Link {
                    url,
                    children,
                    span: crate::Span::NONE,
                })
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
                Some(FmtInline::Image {
                    url,
                    alt,
                    span: crate::Span::NONE,
                })
            }

            node::LINE_BREAK => Some(FmtInline::LineBreak(crate::Span::NONE)),
            node::SOFT_BREAK => Some(FmtInline::SoftBreak(crate::Span::NONE)),

            node::STRIKEOUT => {
                let children = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtInline::Strikethrough(children, crate::Span::NONE))
            }

            node::SUPERSCRIPT => {
                let children = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtInline::Superscript(children, crate::Span::NONE))
            }

            node::SUBSCRIPT => {
                let children = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                Some(FmtInline::Subscript(children, crate::Span::NONE))
            }

            node::SMALL_CAPS | node::QUOTED | node::FOOTNOTE_REF | node::FOOTNOTE_DEF => {
                // Fall back to rendering children as text
                let children: Vec<FmtInline> = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                if children.is_empty() {
                    Some(FmtInline::Text(
                        format!("[{}]", node.kind.as_str()),
                        crate::Span::NONE,
                    ))
                } else {
                    Some(FmtInline::Bold(children, crate::Span::NONE))
                }
            }

            _ => {
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                    format!(
                        "Unsupported inline type for DokuWiki: {}",
                        node.kind.as_str()
                    ),
                ));
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_std::builder::*;

        fn emit_str(doc: &Document) -> String {
            let result = emit(doc).unwrap();
            String::from_utf8(result.value).unwrap()
        }

        #[test]
        fn test_emit_paragraph() {
            let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
            let output = emit_str(&doc);
            assert!(output.contains("Hello, world!"));
        }

        #[test]
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Title")));
            let output = emit_str(&doc);
            assert!(output.contains("====== Title ======"));
        }

        #[test]
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
            let output = emit_str(&doc);
            assert!(output.contains("===== Subtitle ====="));
        }

        #[test]
        fn test_emit_emphasis() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("//italic//"));
        }

        #[test]
        fn test_emit_strong() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            let output = emit_str(&doc);
            assert!(output.contains("**bold**"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("''code''"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("[[https://example.com|click]]"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
            let output = emit_str(&doc);
            assert!(output.contains("<code python>"));
            assert!(output.contains("print('hi')"));
            assert!(output.contains("</code>"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("  * one"));
            assert!(output.contains("  * two"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
