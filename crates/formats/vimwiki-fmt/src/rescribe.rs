//! AST<->`rescribe::Document` translation for VimWiki.
//!
//! This module only translates between [`VimwikiDoc`](crate::VimwikiDoc) and
//! rescribe's `Document` IR — no VimWiki tokenizing/parsing/emitting happens
//! here. Enabled by the `rescribe` feature; each direction is additionally
//! gated on the reader/writer mode feature it depends on.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse VimWiki markup.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse VimWiki markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (doc, _diags) = crate::VimwikiDoc::parse(input.as_bytes());

        let nodes: Vec<Node> = doc.blocks.iter().map(block_to_node).collect();
        let root = Node::new(node::DOCUMENT).children(nodes);
        let rescribe_doc = Document::new().with_content(root);

        Ok(ConversionResult::ok(rescribe_doc))
    }

    fn block_to_node(block: &Block) -> Node {
        match block {
            Block::Paragraph { inlines, .. } => {
                let inline_nodes: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                Node::new(node::PARAGRAPH).children(inline_nodes)
            }

            Block::Heading { level, inlines, .. } => {
                let inline_nodes: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(inline_nodes)
            }

            Block::CodeBlock {
                language, content, ..
            } => {
                let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                if let Some(lang) = language {
                    n = n.prop(prop::LANGUAGE, lang.clone());
                }
                n
            }

            Block::Blockquote { inlines, .. } => {
                let inline_nodes: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                let para = Node::new(node::PARAGRAPH).children(inline_nodes);
                Node::new(node::BLOCKQUOTE).children(vec![para])
            }

            Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item| {
                        let inline_nodes: Vec<Node> =
                            item.inlines.iter().map(inline_to_node).collect();
                        let para = Node::new(node::PARAGRAPH).children(inline_nodes);
                        let mut list_item = Node::new(node::LIST_ITEM).children(vec![para]);
                        if let Some(checked) = item.checked {
                            list_item = list_item.prop("checked", checked);
                        }
                        list_item
                    })
                    .collect();

                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            Block::Table { rows, .. } => {
                let table_rows: Vec<Node> = rows
                    .iter()
                    .map(|row| {
                        let cells: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                let inline_nodes: Vec<Node> =
                                    cell.iter().map(inline_to_node).collect();
                                Node::new(node::TABLE_CELL).children(inline_nodes)
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();

                Node::new(node::TABLE).children(table_rows)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            Block::DefinitionList { items, .. } => {
                let mut children = Vec::new();
                for item in items {
                    let term = Node::new(node::DEFINITION_TERM)
                        .children(item.term.iter().map(inline_to_node).collect::<Vec<_>>());
                    let desc = Node::new(node::DEFINITION_DESC)
                        .children(item.desc.iter().map(inline_to_node).collect::<Vec<_>>());
                    children.push(term);
                    children.push(desc);
                }
                Node::new(node::DEFINITION_LIST).children(children)
            }
        }
    }

    fn inline_to_node(inline: &Inline) -> Node {
        match inline {
            Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Bold(children, _) => {
                let inner: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::STRONG).children(inner)
            }

            Inline::Italic(children, _) => {
                let inner: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::EMPHASIS).children(inner)
            }

            Inline::Strikethrough(children, _) => {
                let inner: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::STRIKEOUT).children(inner)
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link { url, label, .. } => {
                let text_node = Node::new(node::TEXT).prop(prop::CONTENT, label.clone());
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(vec![text_node])
            }

            Inline::Image {
                url, alt, style, ..
            } => {
                let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if let Some(a) = alt {
                    img = img.prop(prop::ALT, a.clone());
                }
                if let Some(s) = style {
                    img = img.prop("vimwiki:style", s.clone());
                }
                img
            }

            Inline::Superscript(children, _) => Node::new(node::SUPERSCRIPT)
                .children(children.iter().map(inline_to_node).collect::<Vec<_>>()),

            Inline::Subscript(children, _) => Node::new(node::SUBSCRIPT)
                .children(children.iter().map(inline_to_node).collect::<Vec<_>>()),
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
            let doc = parse_str("= Title =\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
            assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        }

        #[test]
        fn test_parse_heading_level2() {
            let doc = parse_str("== Subtitle ==\n");
            assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(2));
        }

        #[test]
        fn test_parse_paragraph() {
            let doc = parse_str("Hello world\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
        }

        #[test]
        fn test_parse_bold() {
            let doc = parse_str("*bold*\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::STRONG);
        }

        #[test]
        fn test_parse_italic() {
            let doc = parse_str("_italic_\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
        }

        #[test]
        fn test_parse_code() {
            let doc = parse_str("`code`\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::CODE);
        }

        #[test]
        fn test_parse_wiki_link() {
            let doc = parse_str("[[MyPage]]\n");
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("MyPage"));
        }

        #[test]
        fn test_parse_wiki_link_with_description() {
            let doc = parse_str("[[MyPage|click here]]\n");
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("MyPage"));
        }

        #[test]
        fn test_parse_unordered_list() {
            let doc = parse_str("* item1\n* item2\n");
            assert_eq!(doc.content.children.len(), 1);
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_ordered_list() {
            let doc = parse_str("1. first\n2. second\n");
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
        }

        #[test]
        fn test_parse_preformatted() {
            let doc = parse_str("{{{\ncode here\n}}}\n");
            assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
        }

        #[test]
        fn test_parse_checkbox() {
            let doc = parse_str("* [ ] unchecked\n* [X] checked\n");
            let list = &doc.content.children[0];
            assert_eq!(list.children[0].props.get_bool("checked"), Some(false));
            assert_eq!(list.children[1].props.get_bool("checked"), Some(true));
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, DefinitionItem, Inline, ListItem, Span, TableRow, VimwikiDoc};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document as VimWiki markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as VimWiki markup with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let blocks = doc.content.children.iter().map(node_to_block).collect();
        let vimwiki_doc = VimwikiDoc {
            blocks,
            span: Span::NONE,
        };
        let output = vimwiki_doc.emit();

        Ok(ConversionResult::ok(output))
    }

    /// Extract inlines from a node's children where those children may be
    /// either inline nodes directly, or block nodes such as `paragraph`
    /// wrapping the inlines (e.g. rescribe-fmt-pandoc-json wraps table-cell
    /// and definition-description content in a block since those are block
    /// content in Pandoc's AST). Handle both shapes.
    fn block_content_to_inlines(children: &[Node]) -> Vec<Inline> {
        let mut inlines = Vec::new();
        for child in children {
            if child.kind.as_str() == node::PARAGRAPH {
                inlines.extend(child.children.iter().map(node_to_inline));
            } else {
                inlines.push(node_to_inline(child));
            }
        }
        inlines
    }

    fn node_to_block(node: &Node) -> Block {
        match node.kind.as_str() {
            node::PARAGRAPH => {
                let inlines = node.children.iter().map(node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                }
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(6) as usize;
                let inlines = node.children.iter().map(node_to_inline).collect();
                Block::Heading {
                    level,
                    inlines,
                    span: Span::NONE,
                }
            }

            node::CODE_BLOCK => {
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Block::CodeBlock {
                    language,
                    content,
                    span: Span::NONE,
                }
            }

            node::BLOCKQUOTE => {
                let mut inlines = Vec::new();
                for child in &node.children {
                    if child.kind.as_str() == node::PARAGRAPH {
                        inlines.extend(child.children.iter().map(node_to_inline));
                    }
                }
                Block::Blockquote {
                    inlines,
                    span: Span::NONE,
                }
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items = node
                    .children
                    .iter()
                    .filter(|n| n.kind.as_str() == node::LIST_ITEM)
                    .map(|item_node| {
                        let checked = item_node.props.get_bool("checked");
                        let mut inlines = Vec::new();
                        for child in &item_node.children {
                            if child.kind.as_str() == node::PARAGRAPH {
                                inlines.extend(child.children.iter().map(node_to_inline));
                            }
                        }
                        ListItem {
                            checked,
                            inlines,
                            span: Span::NONE,
                        }
                    })
                    .collect();

                Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                }
            }

            node::TABLE => {
                let rows = node
                    .children
                    .iter()
                    .filter(|n| n.kind.as_str() == node::TABLE_ROW)
                    .map(|row_node| {
                        let cells = row_node
                            .children
                            .iter()
                            .filter(|n| {
                                matches!(n.kind.as_str(), node::TABLE_CELL | node::TABLE_HEADER)
                            })
                            .map(|cell_node| block_content_to_inlines(&cell_node.children))
                            .collect();
                        TableRow {
                            cells,
                            span: Span::NONE,
                        }
                    })
                    .collect();

                Block::Table {
                    rows,
                    span: Span::NONE,
                }
            }

            node::HORIZONTAL_RULE => Block::HorizontalRule { span: Span::NONE },

            node::DEFINITION_LIST => {
                // Children are a flat sequence of definition_term /
                // definition_desc siblings (this crate's own reader shape,
                // also matched by rescribe-fmt-pandoc-json's DefinitionList
                // translation), a term optionally followed by multiple descs.
                let mut items = Vec::new();
                let mut pending_term: Option<Vec<Inline>> = None;
                for child in &node.children {
                    match child.kind.as_str() {
                        node::DEFINITION_TERM => {
                            pending_term = Some(block_content_to_inlines(&child.children));
                        }
                        node::DEFINITION_DESC => {
                            let term = pending_term.clone().unwrap_or_default();
                            items.push(DefinitionItem {
                                term,
                                desc: block_content_to_inlines(&child.children),
                                span: Span::NONE,
                            });
                        }
                        _ => {}
                    }
                }
                Block::DefinitionList {
                    items,
                    span: Span::NONE,
                }
            }

            // Fallback for unhandled block types
            _ => {
                let inlines = node.children.iter().map(node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                }
            }
        }
    }

    fn node_to_inline(node: &Node) -> Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Inline::Text(content, Span::NONE)
            }

            node::STRONG => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Bold(children, Span::NONE)
            }

            node::EMPHASIS => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Italic(children, Span::NONE)
            }

            node::STRIKEOUT => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Strikethrough(children, Span::NONE)
            }

            node::CODE => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Inline::Code(content, Span::NONE)
            }

            node::LINK => {
                let url = node
                    .props
                    .get_str(prop::URL)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let label = if node.children.is_empty() {
                    url.clone()
                } else {
                    node.children
                        .iter()
                        .filter_map(|n| {
                            if n.kind.as_str() == node::TEXT {
                                n.props.get_str(prop::CONTENT).map(|s| s.to_string())
                            } else {
                                None
                            }
                        })
                        .collect::<String>()
                };
                Inline::Link {
                    url,
                    label,
                    span: Span::NONE,
                }
            }

            node::IMAGE => {
                let url = node
                    .props
                    .get_str(prop::URL)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
                Inline::Image {
                    url,
                    alt,
                    style: None,
                    span: Span::NONE,
                }
            }

            node::SUPERSCRIPT => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Superscript(children, Span::NONE)
            }

            node::SUBSCRIPT => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Subscript(children, Span::NONE)
            }

            // Fallback for inline nodes: wrap in text or return text representation
            _ => {
                let children: Vec<Inline> = node.children.iter().map(node_to_inline).collect();
                if children.is_empty() {
                    Inline::Text(String::new(), Span::NONE)
                } else {
                    // Wrap unhandled inline types
                    Inline::Text(format!("[{}]", node.kind), Span::NONE)
                }
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
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Title")));
            let output = emit_str(&doc);
            assert!(output.contains("= Title ="));
        }

        #[test]
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
            let output = emit_str(&doc);
            assert!(output.contains("== Subtitle =="));
        }

        #[test]
        fn test_emit_paragraph() {
            let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
            let output = emit_str(&doc);
            assert!(output.contains("Hello, world!"));
        }

        #[test]
        fn test_emit_bold() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            let output = emit_str(&doc);
            assert!(output.contains("*bold*"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("_italic_"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("`code`"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("MyPage", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("[[MyPage|click]]"));
        }

        #[test]
        fn test_emit_unordered_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("* one"));
            assert!(output.contains("* two"));
        }

        #[test]
        fn test_emit_ordered_list() {
            let doc =
                doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
            let output = emit_str(&doc);
            assert!(output.contains("1. first"));
            assert!(output.contains("2. second"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block("print hi"));
            let output = emit_str(&doc);
            assert!(output.contains("{{{"));
            assert!(output.contains("print hi"));
            assert!(output.contains("}}}"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
