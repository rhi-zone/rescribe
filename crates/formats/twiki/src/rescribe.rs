//! AST↔`rescribe::Document` translation for TWiki.
//!
//! This module only translates between `twiki`'s AST and rescribe's
//! `Document` IR — no TWiki tokenizing/parsing/emitting happens here (that
//! all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`/`TwikiDoc::emit`). Enabled by the `rescribe` feature; each
//! direction is additionally gated on the reader/writer mode feature it
//! depends on, so enabling `rescribe` alone (with no mode feature) compiles
//! nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline, TwikiDoc};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse TWiki markup into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse TWiki markup with options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (twiki_doc, _diags) = TwikiDoc::parse(input.as_bytes());
        let mut result = Vec::new();

        for block in twiki_doc.blocks {
            result.push(block_to_node(&block));
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
            Block::Paragraph { inlines, .. } => {
                Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
            }
            Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inlines_to_nodes(inlines)),
            Block::CodeBlock { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }
            Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item| {
                        let mut item_node = Node::new(node::LIST_ITEM).child(
                            Node::new(node::PARAGRAPH).children(inlines_to_nodes(&item.inlines)),
                        );
                        for child_block in &item.children {
                            item_node = item_node.child(block_to_node(child_block));
                        }
                        item_node
                    })
                    .collect();
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }
            Block::RawBlock { content, .. } => Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "twiki")
                .prop(prop::CONTENT, content.clone()),
            Block::DefinitionList { items, .. } => {
                let def_nodes: Vec<Node> = items
                    .iter()
                    .map(|item| {
                        Node::new("definition_item")
                            .child(
                                Node::new(node::DEFINITION_TERM)
                                    .children(inlines_to_nodes(&item.term)),
                            )
                            .child(
                                Node::new(node::DEFINITION_DESC)
                                    .children(inlines_to_nodes(&item.desc)),
                            )
                    })
                    .collect();
                Node::new(node::DEFINITION_LIST).children(def_nodes)
            }

            Block::Blockquote { children, .. } => {
                let block_nodes: Vec<_> = children.iter().map(block_to_node).collect();
                Node::new(node::BLOCKQUOTE).children(block_nodes)
            }
            Block::Table { rows, .. } => {
                let row_nodes: Vec<Node> = rows
                    .iter()
                    .map(|row| {
                        let cells: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                if cell.is_header {
                                    Node::new(node::TABLE_HEADER)
                                        .children(inlines_to_nodes(&cell.inlines))
                                } else {
                                    Node::new(node::TABLE_CELL)
                                        .children(inlines_to_nodes(&cell.inlines))
                                }
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(row_nodes)
            }
            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),
        }
    }

    fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
        let mut nodes = Vec::new();
        for inline in inlines {
            nodes.push(inline_to_node(inline));
        }
        nodes
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
            Inline::BoldItalic(children, _) => Node::new(node::STRONG)
                .child(Node::new(node::EMPHASIS).children(inlines_to_nodes(children))),
            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),
            Inline::BoldCode(children, _) => Node::new(node::STRONG)
                .child(Node::new(node::CODE).prop(prop::CONTENT, children_to_text(children))),
            Inline::Link { url, label, .. } => Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .child(Node::new(node::TEXT).prop(prop::CONTENT, label.clone())),
            Inline::Strikethrough(children, _) => {
                Node::new("strikethrough").children(inlines_to_nodes(children))
            }
            Inline::Superscript(children, _) => {
                Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
            }
            Inline::Subscript(children, _) => {
                Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
            }
            Inline::Underline(children, _) => {
                Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
            }
            Inline::Image { url, alt, .. } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if !alt.is_empty() {
                    n = n.prop(prop::ALT, alt.clone());
                }
                n
            }
            Inline::RawInline { content, .. } => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "twiki")
                .prop(prop::CONTENT, content.clone()),
            Inline::WikiWord { word, .. } => Node::new("wikiword").prop("word", word.clone()),
            Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),
        }
    }

    fn children_to_text(children: &[Inline]) -> String {
        let mut s = String::new();
        for child in children {
            match child {
                Inline::Text(t, _) => s.push_str(t),
                Inline::Bold(ch, _)
                | Inline::Italic(ch, _)
                | Inline::BoldItalic(ch, _)
                | Inline::Strikethrough(ch, _)
                | Inline::Superscript(ch, _)
                | Inline::Subscript(ch, _)
                | Inline::Underline(ch, _) => {
                    s.push_str(&children_to_text(ch));
                }
                Inline::Code(c, _) => s.push_str(c),
                Inline::BoldCode(ch, _) => s.push_str(&children_to_text(ch)),
                Inline::Link { label, .. } => s.push_str(label),
                Inline::LineBreak { .. } => s.push('\n'),
                Inline::Image { alt, .. } => s.push_str(alt),
                Inline::RawInline { content, .. } => s.push_str(content),
                Inline::WikiWord { word, .. } => s.push_str(word),
            }
        }
        s
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_heading() {
            let result = parse("---+ Heading 1\n---++ Heading 2").unwrap();
            assert_eq!(result.value.content.children.len(), 2);
        }

        #[test]
        fn test_parse_bold() {
            let result = parse("This is *bold* text").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_table() {
            let result = parse("| A | B |\n| C | D |").unwrap();
            assert_eq!(result.value.content.children.len(), 1);
            assert_eq!(result.value.content.children[0].kind.as_str(), node::TABLE);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, Inline, ListItem, Span, TableCell, TableRow, TwikiDoc};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document to TWiki markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document to TWiki markup with options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let blocks: Vec<Block> = doc
            .content
            .children
            .iter()
            .filter_map(node_to_block)
            .collect();

        let twiki_doc = TwikiDoc {
            blocks,
            span: Span::NONE,
        };
        let output = twiki_doc.emit();
        Ok(ConversionResult::ok(output))
    }

    fn node_to_block(node: &Node) -> Option<Block> {
        match node.kind.as_str() {
            node::DOCUMENT => {
                // Document nodes should have been flattened; skip
                None
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                Some(Block::Heading {
                    level,
                    inlines: node_children_to_inlines(&node.children),
                    span: Span::NONE,
                })
            }

            node::PARAGRAPH => Some(Block::Paragraph {
                inlines: node_children_to_inlines(&node.children),
                span: Span::NONE,
            }),

            node::CODE_BLOCK => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Some(Block::CodeBlock {
                    content,
                    span: Span::NONE,
                })
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<ListItem> = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                    .map(|item| {
                        let inlines = item
                            .children
                            .iter()
                            .find(|c| c.kind.as_str() == node::PARAGRAPH)
                            .map(|para| node_children_to_inlines(&para.children))
                            .unwrap_or_default();
                        ListItem {
                            inlines,
                            children: Vec::new(),
                            span: Span::NONE,
                        }
                    })
                    .collect();
                Some(Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                })
            }

            node::TABLE => {
                let rows: Vec<TableRow> = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                    .map(|row| {
                        let cells: Vec<TableCell> = row
                            .children
                            .iter()
                            .map(|cell| {
                                let is_header = cell.kind.as_str() == node::TABLE_HEADER;
                                let inlines = node_children_to_inlines(&cell.children);
                                TableCell {
                                    inlines,
                                    is_header,
                                    span: Span::NONE,
                                }
                            })
                            .collect();
                        TableRow {
                            cells,
                            span: Span::NONE,
                        }
                    })
                    .collect();
                Some(Block::Table {
                    rows,
                    span: Span::NONE,
                })
            }

            node::HORIZONTAL_RULE => Some(Block::HorizontalRule { span: Span::NONE }),

            node::DIV | node::SPAN | node::FIGURE => {
                // These are container nodes; flatten their children
                None
            }

            _ => None,
        }
    }

    fn node_children_to_inlines(nodes: &[Node]) -> Vec<Inline> {
        let mut inlines = Vec::new();
        for node in nodes {
            if let Some(inline) = node_to_inline(node) {
                inlines.push(inline);
            }
        }
        inlines
    }

    fn node_to_inline(node: &Node) -> Option<Inline> {
        match node.kind.as_str() {
            node::TEXT => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Some(Inline::Text(content, Span::NONE))
            }

            node::STRONG => {
                // Check if child is emphasis (bold italic)
                if node.children.len() == 1 && node.children[0].kind.as_str() == node::EMPHASIS {
                    let children = node_children_to_inlines(&node.children[0].children);
                    Some(Inline::BoldItalic(children, Span::NONE))
                } else if node.children.len() == 1 && node.children[0].kind.as_str() == node::CODE {
                    // Bold code
                    let children = node_children_to_inlines(&node.children);
                    Some(Inline::BoldCode(children, Span::NONE))
                } else {
                    let children = node_children_to_inlines(&node.children);
                    Some(Inline::Bold(children, Span::NONE))
                }
            }

            node::EMPHASIS => {
                let children = node_children_to_inlines(&node.children);
                Some(Inline::Italic(children, Span::NONE))
            }

            node::CODE => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Some(Inline::Code(content, Span::NONE))
            }

            node::LINK => {
                let url = node
                    .props
                    .get_str(prop::URL)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let label = node
                    .children
                    .iter()
                    .find(|c| c.kind.as_str() == node::TEXT)
                    .and_then(|c| c.props.get_str(prop::CONTENT))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| url.clone());
                Some(Inline::Link {
                    url,
                    label,
                    span: Span::NONE,
                })
            }

            node::LINE_BREAK => Some(Inline::LineBreak { span: Span::NONE }),
            node::SOFT_BREAK => Some(Inline::Text(" ".to_string(), Span::NONE)),

            _ => None,
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
            assert!(emit_str(&doc).contains("---+ Title"));
        }

        #[test]
        fn test_emit_bold() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            assert!(emit_str(&doc).contains("*bold*"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            assert!(emit_str(&doc).contains("_italic_"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("Example"))));
            assert!(emit_str(&doc).contains("[[http://example.com][Example]]"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("   * one"));
            assert!(output.contains("   * two"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
