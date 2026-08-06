//! AST↔`rescribe::Document` translation for TikiWiki.
//!
//! This module only translates between `tikiwiki`'s AST and rescribe's
//! `Document` IR — no TikiWiki tokenizing/parsing/emitting happens here
//! (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`/`TikiwikiDoc::emit`). Enabled by the `rescribe` feature;
//! each direction is additionally gated on the reader/writer mode feature it
//! depends on, so enabling `rescribe` alone (with no mode feature) compiles
//! nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::Inline as TwInline;
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_std::{node, prop};

    /// Parse TikiWiki markup into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse TikiWiki markup with options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (tw_doc, _diags) = crate::parse(input);

        let mut blocks = Vec::new();
        for block in &tw_doc.blocks {
            blocks.push(block_to_node(block));
        }

        let document = Document {
            content: Node::new(node::DOCUMENT).children(blocks),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };

        Ok(ConversionResult::ok(document))
    }

    fn block_to_node(block: &crate::Block) -> Node {
        use crate::Block;

        match block {
            Block::Paragraph { inlines, .. } => {
                Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
            }

            Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inlines_to_nodes(inlines)),

            Block::CodeBlock {
                content, language, ..
            } => {
                let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                if let Some(lang) = language {
                    n = n.prop(prop::LANGUAGE, lang.clone());
                }
                n
            }

            Block::Blockquote { blocks, .. } => {
                let block_nodes: Vec<_> = blocks.iter().map(block_to_node).collect();
                Node::new(node::BLOCKQUOTE).children(block_nodes)
            }

            Block::List { ordered, items, .. } => {
                let mut list_items = Vec::new();
                for item in items {
                    let mut item_children = inlines_to_nodes(&item.inlines);
                    for child_block in &item.children {
                        item_children.push(block_to_node(child_block));
                    }
                    list_items.push(Node::new(node::LIST_ITEM).children(item_children));
                }
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            Block::Table { rows, .. } => {
                let mut table_rows = Vec::new();
                for row in rows {
                    let mut cells = Vec::new();
                    for cell in &row.cells {
                        cells.push(
                            Node::new(node::TABLE_CELL).children(inlines_to_nodes(&cell.inlines)),
                        );
                    }
                    table_rows.push(Node::new(node::TABLE_ROW).children(cells));
                }
                Node::new(node::TABLE).children(table_rows)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),
        }
    }

    fn inlines_to_nodes(inlines: &[TwInline]) -> Vec<Node> {
        inlines.iter().map(inline_to_node).collect()
    }

    fn inline_to_node(inline: &TwInline) -> Node {
        use crate::Inline;

        match inline {
            Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Bold(children, _) => {
                Node::new(node::STRONG).children(inlines_to_nodes(children))
            }

            Inline::Italic(children, _) => {
                Node::new(node::EMPHASIS).children(inlines_to_nodes(children))
            }

            Inline::Underline(children, _) => {
                Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
            }

            Inline::Strikethrough(children, _) => {
                Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link { url, children, .. } => Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(inlines_to_nodes(children)),

            Inline::Image { url, alt, .. } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if !alt.is_empty() {
                    n = n.prop(prop::ALT, alt.clone());
                }
                n
            }

            Inline::Superscript(children, _) => {
                Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::Subscript(children, _) => {
                Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::WikiLink { page, children, .. } => {
                let label_nodes = if children.is_empty() {
                    vec![Node::new(node::TEXT).prop(prop::CONTENT, page.clone())]
                } else {
                    inlines_to_nodes(children)
                };
                Node::new("wikilink")
                    .prop("page", page.clone())
                    .children(label_nodes)
            }

            Inline::Nowiki(s, _) => Node::new("nowiki").prop(prop::CONTENT, s.clone()),

            Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_heading() {
            let result = parse("!Heading 1\n!!Heading 2").unwrap();
            assert_eq!(result.value.content.children.len(), 2);
        }

        #[test]
        fn test_parse_bold() {
            let result = parse("This is __bold__ text").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_italic() {
            let result = parse("This is ''italic'' text").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_link() {
            let result = parse("[http://example.com|Example]").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_list() {
            let result = parse("*Item 1\n*Item 2").unwrap();
            assert_eq!(result.value.content.children.len(), 1);
            assert_eq!(result.value.content.children[0].kind.as_str(), node::LIST);
        }

        #[test]
        fn test_parse_table() {
            let result = parse("||A|B||\n||C|D||").unwrap();
            assert_eq!(result.value.content.children.len(), 1);
            assert_eq!(result.value.content.children[0].kind.as_str(), node::TABLE);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{
        Block as TwBlock, Inline as TwInline, ListItem as TwListItem, Span,
        TableCell as TwTableCell,
    };
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document to TikiWiki markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document to TikiWiki markup with options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut blocks = Vec::new();
        for node in &doc.content.children {
            if let Some(block) = node_to_block(node) {
                blocks.push(block);
            }
        }

        let tw_doc = crate::TikiwikiDoc {
            blocks,
            span: Span::NONE,
        };
        let output = tw_doc.emit();
        Ok(ConversionResult::ok(output))
    }

    fn node_to_block(node: &Node) -> Option<TwBlock> {
        match node.kind.as_str() {
            node::DOCUMENT => None,

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                let inlines = nodes_to_inlines(&node.children);
                Some(TwBlock::Heading {
                    level,
                    inlines,
                    span: Span::NONE,
                })
            }

            node::PARAGRAPH => {
                let inlines = nodes_to_inlines(&node.children);
                Some(TwBlock::Paragraph {
                    inlines,
                    span: Span::NONE,
                })
            }

            node::CODE_BLOCK => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .unwrap_or_default()
                    .to_string();
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                Some(TwBlock::CodeBlock {
                    content,
                    language,
                    span: Span::NONE,
                })
            }

            node::BLOCKQUOTE => {
                let blocks = node.children.iter().filter_map(node_to_block).collect();
                Some(TwBlock::Blockquote {
                    blocks,
                    span: Span::NONE,
                })
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let mut items = Vec::new();
                for child in &node.children {
                    if child.kind.as_str() == node::LIST_ITEM {
                        let inlines = nodes_to_inlines(&child.children);
                        items.push(TwListItem {
                            inlines,
                            children: Vec::new(),
                            span: Span::NONE,
                        });
                    }
                }
                Some(TwBlock::List {
                    ordered,
                    items,
                    span: Span::NONE,
                })
            }

            node::TABLE => {
                let mut rows = Vec::new();
                for row_node in &node.children {
                    if row_node.kind.as_str() == node::TABLE_ROW {
                        let mut cells = Vec::new();
                        for cell_node in &row_node.children {
                            if cell_node.kind.as_str() == node::TABLE_CELL
                                || cell_node.kind.as_str() == node::TABLE_HEADER
                            {
                                let inlines = nodes_to_inlines(&cell_node.children);
                                cells.push(TwTableCell {
                                    inlines,
                                    span: Span::NONE,
                                });
                            }
                        }
                        rows.push(crate::TableRow {
                            cells,
                            is_header: false,
                            span: Span::NONE,
                        });
                    }
                }
                Some(TwBlock::Table {
                    rows,
                    span: Span::NONE,
                })
            }

            node::HORIZONTAL_RULE => Some(TwBlock::HorizontalRule { span: Span::NONE }),

            node::DIV | node::SPAN | node::FIGURE => {
                // For containers, extract first block child if any
                for child in &node.children {
                    if let Some(block) = node_to_block(child) {
                        return Some(block);
                    }
                }
                None
            }

            _ => None,
        }
    }

    fn nodes_to_inlines(nodes: &[Node]) -> Vec<TwInline> {
        nodes.iter().filter_map(node_to_inline).collect()
    }

    fn node_to_inline(node: &Node) -> Option<TwInline> {
        match node.kind.as_str() {
            node::TEXT => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .unwrap_or_default()
                    .to_string();
                if !content.is_empty() {
                    Some(TwInline::Text(content, Span::NONE))
                } else {
                    None
                }
            }

            node::STRONG => {
                let children = nodes_to_inlines(&node.children);
                if !children.is_empty() {
                    Some(TwInline::Bold(children, Span::NONE))
                } else {
                    None
                }
            }

            node::EMPHASIS => {
                let children = nodes_to_inlines(&node.children);
                if !children.is_empty() {
                    Some(TwInline::Italic(children, Span::NONE))
                } else {
                    None
                }
            }

            node::UNDERLINE => {
                let children = nodes_to_inlines(&node.children);
                if !children.is_empty() {
                    Some(TwInline::Underline(children, Span::NONE))
                } else {
                    None
                }
            }

            node::STRIKEOUT => {
                let children = nodes_to_inlines(&node.children);
                if !children.is_empty() {
                    Some(TwInline::Strikethrough(children, Span::NONE))
                } else {
                    None
                }
            }

            node::CODE => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .unwrap_or_default()
                    .to_string();
                if !content.is_empty() {
                    Some(TwInline::Code(content, Span::NONE))
                } else {
                    None
                }
            }

            node::LINK => {
                let url = node
                    .props
                    .get_str(prop::URL)
                    .unwrap_or_default()
                    .to_string();
                let children = nodes_to_inlines(&node.children);
                Some(TwInline::Link {
                    url,
                    children,
                    span: Span::NONE,
                })
            }

            node::IMAGE => {
                let url = node
                    .props
                    .get_str(prop::URL)
                    .unwrap_or_default()
                    .to_string();
                let alt = node
                    .props
                    .get_str(prop::ALT)
                    .unwrap_or_default()
                    .to_string();
                Some(TwInline::Image {
                    url,
                    alt,
                    span: Span::NONE,
                })
            }

            node::LINE_BREAK => Some(TwInline::LineBreak { span: Span::NONE }),

            node::SOFT_BREAK => Some(TwInline::Text(" ".to_string(), Span::NONE)),

            _ => {
                let children = nodes_to_inlines(&node.children);
                if !children.is_empty() {
                    // Return first inline from children
                    children.into_iter().next()
                } else {
                    None
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_core::Properties;
        use rescribe_std::builder::*;

        fn emit_str(doc: &Document) -> String {
            String::from_utf8(emit(doc).unwrap().value).unwrap()
        }

        #[test]
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Title")));
            assert!(emit_str(&doc).contains("! Title"));
        }

        #[test]
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Section")));
            assert!(emit_str(&doc).contains("!! Section"));
        }

        #[test]
        fn test_emit_bold() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            assert!(emit_str(&doc).contains("__bold__"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            assert!(emit_str(&doc).contains("''italic''"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("Example"))));
            assert!(emit_str(&doc).contains("[http://example.com|Example]"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("* one"));
            assert!(output.contains("* two"));
        }

        #[test]
        fn test_emit_table() {
            let doc = Document {
                content: Node::new(node::DOCUMENT).child(
                    Node::new(node::TABLE).child(
                        Node::new(node::TABLE_ROW)
                            .child(
                                Node::new(node::TABLE_CELL)
                                    .child(Node::new(node::TEXT).prop(prop::CONTENT, "A")),
                            )
                            .child(
                                Node::new(node::TABLE_CELL)
                                    .child(Node::new(node::TEXT).prop(prop::CONTENT, "B")),
                            ),
                    ),
                ),
                resources: Default::default(),
                metadata: Properties::new(),
                source: None,
            };
            assert!(emit_str(&doc).contains("||A|B||"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
