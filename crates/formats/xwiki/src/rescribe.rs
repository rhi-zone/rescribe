//! AST<->`rescribe::Document` translation for XWiki.
//!
//! This module only translates between [`XwikiDoc`](crate::XwikiDoc) and
//! rescribe's `Document` IR — no XWiki tokenizing/parsing/emitting happens
//! here. Enabled by the `rescribe` feature; each direction is additionally
//! gated on the reader/writer mode feature it depends on.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline, XwikiDoc};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse XWiki markup into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse XWiki markup with options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (doc, _diags) = XwikiDoc::parse(input.as_bytes());

        let mut result = Vec::new();
        for block in &doc.blocks {
            result.push(block_to_node(block));
        }

        let document = Document {
            content: Node::new(node::DOCUMENT).children(result),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };

        Ok(ConversionResult::ok(document))
    }

    fn block_to_node(block: &Block) -> Node {
        match block {
            Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inlines_to_nodes(inlines)),

            Block::Paragraph { inlines, .. } => {
                Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
            }

            Block::CodeBlock {
                content, language, ..
            } => {
                let mut node = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                if let Some(lang) = language {
                    node = node.prop(prop::LANGUAGE, lang.clone());
                }
                node
            }

            Block::Table { rows, .. } => {
                let table_rows: Vec<Node> = rows
                    .iter()
                    .map(|row| {
                        let cells: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                let kind = if cell.is_header {
                                    node::TABLE_HEADER
                                } else {
                                    node::TABLE_CELL
                                };
                                Node::new(kind).children(inlines_to_nodes(&cell.inlines))
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(table_rows)
            }

            Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_blocks| {
                        let children: Vec<Node> = item_blocks.iter().map(block_to_node).collect();
                        Node::new(node::LIST_ITEM).children(children)
                    })
                    .collect();
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            Block::Blockquote { children, .. } => {
                let child_nodes: Vec<Node> = children.iter().map(block_to_node).collect();
                Node::new(node::BLOCKQUOTE).children(child_nodes)
            }

            Block::MacroBlock {
                name,
                params,
                content,
                ..
            } => Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "xwiki")
                .prop("xwiki:macro-name", name.clone())
                .prop("xwiki:macro-params", params.clone())
                .prop(prop::CONTENT, content.clone()),

            Block::MacroInline { name, params, .. } => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "xwiki")
                .prop("xwiki:macro-name", name.clone())
                .prop("xwiki:macro-params", params.clone()),
        }
    }

    fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
        inlines.iter().map(inline_to_node).collect()
    }

    fn inline_to_node(inline: &Inline) -> Node {
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

            Inline::Strikeout(children, _) => {
                Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link { url, label, .. } => Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .child(Node::new(node::TEXT).prop(prop::CONTENT, label.clone())),

            Inline::Image { url, alt, .. } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if let Some(alt_text) = alt {
                    n = n.prop(prop::ALT, alt_text.clone());
                }
                n
            }

            Inline::Superscript(children, _) => {
                Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::Subscript(children, _) => {
                Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

            Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_heading() {
            let result = parse("= Heading 1 =\n== Heading 2 ==").unwrap();
            assert_eq!(result.value.content.children.len(), 2);
        }

        #[test]
        fn test_parse_bold() {
            let result = parse("This is **bold** text").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_italic() {
            let result = parse("This is //italic// text").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_link() {
            let result = parse("[[Example>>http://example.com]]").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_list() {
            let result = parse("* Item 1\n* Item 2").unwrap();
            assert_eq!(result.value.content.children.len(), 1);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, Inline, Span, XwikiDoc};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document to XWiki markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document to XWiki markup with options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let blocks: Vec<Block> = doc.content.children.iter().map(node_to_block).collect();
        let xwiki_doc = XwikiDoc {
            blocks,
            span: Span::NONE,
        };
        let output = xwiki_doc.emit();
        Ok(ConversionResult::ok(output))
    }

    fn node_to_block(node: &Node) -> Block {
        match node.kind.as_str() {
            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                Block::Heading {
                    level: level.min(6),
                    inlines: nodes_to_inlines(&node.children),
                    span: Span::NONE,
                }
            }

            node::PARAGRAPH => Block::Paragraph {
                inlines: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                Block::CodeBlock {
                    content,
                    language,
                    span: Span::NONE,
                }
            }

            node::TABLE => {
                let rows = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                    .map(|row| {
                        let cells = row
                            .children
                            .iter()
                            .map(|cell| {
                                let is_header = cell.kind.as_str() == node::TABLE_HEADER;
                                crate::TableCell {
                                    is_header,
                                    inlines: block_content_to_inlines(&cell.children),
                                    span: Span::NONE,
                                }
                            })
                            .collect();
                        crate::TableRow {
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

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                    .map(|item| item.children.iter().map(node_to_block).collect())
                    .collect();
                Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                }
            }

            node::HORIZONTAL_RULE => Block::HorizontalRule { span: Span::NONE },

            node::BLOCKQUOTE => {
                let children: Vec<Block> = node.children.iter().map(node_to_block).collect();
                Block::Blockquote {
                    children,
                    span: Span::NONE,
                }
            }

            _ => {
                // For unhandled node kinds, recursively process children
                let blocks: Vec<Block> = node.children.iter().map(node_to_block).collect();
                match <[Block; 1]>::try_from(blocks) {
                    Ok([only]) => only,
                    Err(blocks) => match blocks.into_iter().next() {
                        Some(first) => first,
                        None => Block::Paragraph {
                            inlines: vec![],
                            span: Span::NONE,
                        },
                    },
                }
            }
        }
    }

    fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
        nodes.iter().map(node_to_inline).collect()
    }

    /// Extract inlines from a node's children where those children may be
    /// either inline nodes directly (this crate's own reader shape) or block
    /// nodes such as `paragraph` wrapping the inlines (e.g.
    /// rescribe-fmt-pandoc-json wraps table-cell content in a block, since
    /// table cells are block content in Pandoc's AST). Handle both shapes.
    fn block_content_to_inlines(children: &[Node]) -> Vec<Inline> {
        let mut inlines = Vec::new();
        for child in children {
            if child.kind.as_str() == node::PARAGRAPH {
                inlines.extend(nodes_to_inlines(&child.children));
            } else {
                inlines.push(node_to_inline(child));
            }
        }
        inlines
    }

    fn node_to_inline(node: &Node) -> Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Text(content, Span::NONE)
            }

            node::STRONG => Inline::Bold(nodes_to_inlines(&node.children), Span::NONE),

            node::EMPHASIS => Inline::Italic(nodes_to_inlines(&node.children), Span::NONE),

            node::UNDERLINE => Inline::Underline(nodes_to_inlines(&node.children), Span::NONE),

            node::STRIKEOUT => Inline::Strikeout(nodes_to_inlines(&node.children), Span::NONE),

            node::CODE => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Code(content, Span::NONE)
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let label = if node.children.is_empty() {
                    url.clone()
                } else {
                    nodes_to_inlines(&node.children)
                        .iter()
                        .map(|i| match i {
                            Inline::Text(s, _) => s.clone(),
                            _ => String::new(),
                        })
                        .collect::<Vec<_>>()
                        .join("")
                };
                Inline::Link {
                    url,
                    label,
                    span: Span::NONE,
                }
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
                Inline::Image {
                    url,
                    alt,
                    params: vec![],
                    span: Span::NONE,
                }
            }

            node::SUPERSCRIPT => Inline::Superscript(nodes_to_inlines(&node.children), Span::NONE),

            node::SUBSCRIPT => Inline::Subscript(nodes_to_inlines(&node.children), Span::NONE),

            node::LINE_BREAK => Inline::LineBreak { span: Span::NONE },

            node::SOFT_BREAK => Inline::SoftBreak { span: Span::NONE },

            _ => Inline::Text(String::new(), Span::NONE),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_std::builder::*;

        fn emit_str(doc: &Document) -> String {
            String::from_utf8(emit(doc).unwrap().value).unwrap()
        }

        #[test]
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Title")));
            assert!(emit_str(&doc).contains("= Title ="));
        }

        #[test]
        fn test_emit_bold() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            assert!(emit_str(&doc).contains("**bold**"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            assert!(emit_str(&doc).contains("//italic//"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("Example"))));
            assert!(emit_str(&doc).contains("[[Example>>http://example.com]]"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("* one"));
            assert!(output.contains("* two"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
