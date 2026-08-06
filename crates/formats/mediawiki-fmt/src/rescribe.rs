//! AST↔`rescribe::Document` translation for MediaWiki.
//!
//! This module only translates between [`MediawikiDoc`](crate::MediawikiDoc)
//! and rescribe's `Document` IR — no markup tokenizing/parsing/emitting
//! happens here (that all lives in the rest of this crate; see `crate::parse`
//! and `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline, parse_str as parse_mediawiki};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, Properties};
    use rescribe_std::{node, prop};

    /// Parse MediaWiki text into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        let (fmt_doc, _diags) = parse_mediawiki(input);

        let children: Vec<Node> = fmt_doc.blocks.iter().map(block_to_node).collect();

        let document = Document {
            content: Node::new(node::DOCUMENT).children(children),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        Ok(ConversionResult::with_warnings(document, vec![]))
    }

    fn block_to_node(block: &Block) -> Node {
        match block {
            Block::Paragraph { inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                Node::new(node::PARAGRAPH).children(children)
            }

            Block::Heading { level, inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(children)
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

            Block::List { ordered, items, .. } => {
                let children: Vec<Node> = items
                    .iter()
                    .map(|item_blocks| {
                        let para_children: Vec<Node> = item_blocks
                            .iter()
                            .flat_map(|block| {
                                if let Block::Paragraph { inlines, .. } = block {
                                    inlines.iter().map(inline_to_node).collect::<Vec<_>>()
                                } else {
                                    vec![block_to_node(block)]
                                }
                            })
                            .collect();

                        Node::new(node::LIST_ITEM)
                            .child(Node::new(node::PARAGRAPH).children(para_children))
                    })
                    .collect();

                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(children)
            }

            Block::DefinitionList { items, .. } => {
                let children: Vec<Node> = items
                    .iter()
                    .flat_map(|item| {
                        let term_children: Vec<Node> =
                            item.term.iter().map(inline_to_node).collect();
                        let desc_children: Vec<Node> =
                            item.desc.iter().map(inline_to_node).collect();
                        vec![
                            Node::new(node::DEFINITION_TERM).children(term_children),
                            Node::new(node::DEFINITION_DESC).children(desc_children),
                        ]
                    })
                    .collect();
                Node::new(node::DEFINITION_LIST).children(children)
            }

            Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),

            Block::Blockquote { children, .. } => {
                let child_nodes: Vec<Node> = children.iter().map(block_to_node).collect();
                Node::new(node::BLOCKQUOTE).children(child_nodes)
            }

            Block::PreBlock { content, .. } => {
                Node::new("pre_block").prop(prop::CONTENT, content.clone())
            }

            Block::RawBlock { content, .. } => Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "mediawiki")
                .prop(prop::CONTENT, content.clone()),

            Block::Table { rows, .. } => {
                let children: Vec<Node> = rows
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
                                let children: Vec<Node> =
                                    cell.inlines.iter().map(inline_to_node).collect();
                                Node::new(kind).children(children)
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();

                Node::new(node::TABLE).children(children)
            }
        }
    }

    fn inline_to_node(inline: &Inline) -> Node {
        match inline {
            Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Bold(children) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::STRONG).children(child_nodes)
            }

            Inline::Italic(children) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::EMPHASIS).children(child_nodes)
            }

            Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link { url, text } => Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

            Inline::Image { url, alt } => Node::new(node::IMAGE)
                .prop(prop::URL, url.clone())
                .prop(prop::ALT, alt.clone()),

            Inline::LineBreak => Node::new(node::LINE_BREAK),

            Inline::Strikeout(children) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::STRIKEOUT).children(child_nodes)
            }

            Inline::Underline(children) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::UNDERLINE).children(child_nodes)
            }

            Inline::Subscript(children) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::SUBSCRIPT).children(child_nodes)
            }

            Inline::Superscript(children) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::SUPERSCRIPT).children(child_nodes)
            }

            Inline::FootnoteRef { label, content } => {
                let mut n = Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone());
                if let Some(c) = content {
                    n = n.prop(prop::CONTENT, c.clone());
                }
                n
            }

            Inline::MathInline { source } => {
                Node::new("math_inline").prop(prop::CONTENT, source.clone())
            }

            Inline::Template { content } => {
                Node::new("template").prop(prop::CONTENT, content.clone())
            }

            Inline::Nowiki { content } => Node::new("nowiki").prop(prop::CONTENT, content.clone()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_heading() {
            let result = parse("== Heading ==").unwrap();
            let doc = result.value;
            assert_eq!(doc.content.children.len(), 1);
            let heading = &doc.content.children[0];
            assert_eq!(heading.kind.as_str(), node::HEADING);
            assert_eq!(heading.props.get_int(prop::LEVEL), Some(2));
        }

        #[test]
        fn test_parse_bold() {
            let result = parse("'''bold'''").unwrap();
            let doc = result.value;
            let para = &doc.content.children[0];
            let strong = &para.children[0];
            assert_eq!(strong.kind.as_str(), node::STRONG);
        }

        #[test]
        fn test_parse_italic() {
            let result = parse("''italic''").unwrap();
            let doc = result.value;
            let para = &doc.content.children[0];
            let em = &para.children[0];
            assert_eq!(em.kind.as_str(), node::EMPHASIS);
        }

        #[test]
        fn test_parse_list() {
            let result = parse("* Item 1\n* Item 2").unwrap();
            let doc = result.value;
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_link() {
            let result = parse("[[Title|Link text]]").unwrap();
            let doc = result.value;
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("Title"));
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, Emit as _, Inline, MediawikiDoc, Span, TableCell, TableRow};
    use rescribe_core::{ConversionResult, Document, EmitError, Node};
    use rescribe_std::{node, prop};

    /// Emit a document as MediaWiki markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let blocks = doc
            .content
            .children
            .iter()
            .flat_map(node_to_block)
            .collect();

        let fmt_doc = MediawikiDoc {
            blocks,
            span: Span::NONE,
        };
        let output = fmt_doc.emit();

        Ok(ConversionResult::with_warnings(output, vec![]))
    }

    fn node_to_block(node: &Node) -> Vec<Block> {
        match node.kind.as_str() {
            node::PARAGRAPH => {
                let inlines = node.children.iter().flat_map(node_to_inline).collect();
                vec![Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                }]
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                let inlines = node.children.iter().flat_map(node_to_inline).collect();
                vec![Block::Heading {
                    level,
                    inlines,
                    span: Span::NONE,
                }]
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                vec![Block::CodeBlock {
                    language,
                    content,
                    span: Span::NONE,
                }]
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                    .map(|item_node| item_node.children.iter().flat_map(node_to_block).collect())
                    .collect();
                vec![Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                }]
            }

            node::BLOCKQUOTE => {
                // Flatten blockquote into its children for MediaWiki output
                node.children
                    .iter()
                    .flat_map(node_to_block)
                    .collect::<Vec<_>>()
            }

            node::HORIZONTAL_RULE => {
                vec![Block::HorizontalRule]
            }

            node::TABLE => {
                let rows = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                    .map(|row_node| {
                        let cells = row_node
                            .children
                            .iter()
                            .map(|cell_node| {
                                let is_header = cell_node.kind.as_str() == node::TABLE_HEADER;
                                let inlines =
                                    cell_node.children.iter().flat_map(node_to_inline).collect();
                                TableCell {
                                    is_header,
                                    inlines,
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
                vec![Block::Table {
                    rows,
                    caption: None,
                    span: Span::NONE,
                }]
            }

            _ => {
                // Skip unknown block types
                vec![]
            }
        }
    }

    fn node_to_inline(node: &Node) -> Vec<Inline> {
        match node.kind.as_str() {
            node::TEXT => {
                if let Some(content) = node.props.get_str(prop::CONTENT) {
                    vec![Inline::Text(content.to_string())]
                } else {
                    vec![]
                }
            }

            node::STRONG => {
                let children = node.children.iter().flat_map(node_to_inline).collect();
                vec![Inline::Bold(children)]
            }

            node::EMPHASIS => {
                let children = node.children.iter().flat_map(node_to_inline).collect();
                vec![Inline::Italic(children)]
            }

            node::CODE => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                vec![Inline::Code(content)]
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let text = extract_text(node);
                vec![Inline::Link { url, text }]
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
                vec![Inline::Image { url, alt }]
            }

            node::LINE_BREAK => {
                vec![Inline::LineBreak]
            }

            node::STRIKEOUT => {
                let children = node.children.iter().flat_map(node_to_inline).collect();
                vec![Inline::Strikeout(children)]
            }

            node::UNDERLINE => {
                let children = node.children.iter().flat_map(node_to_inline).collect();
                vec![Inline::Underline(children)]
            }

            node::SUBSCRIPT => {
                let children = node.children.iter().flat_map(node_to_inline).collect();
                vec![Inline::Subscript(children)]
            }

            node::SUPERSCRIPT => {
                let children = node.children.iter().flat_map(node_to_inline).collect();
                vec![Inline::Superscript(children)]
            }

            node::SOFT_BREAK => {
                vec![Inline::Text(" ".to_string())]
            }

            _ => {
                // Recursively emit children
                node.children.iter().flat_map(node_to_inline).collect()
            }
        }
    }

    fn extract_text(node: &Node) -> String {
        let mut result = String::new();
        extract_text_recursive(node, &mut result);
        result
    }

    fn extract_text_recursive(node: &Node, output: &mut String) {
        if node.kind.as_str() == node::TEXT
            && let Some(content) = node.props.get_str(prop::CONTENT)
        {
            output.push_str(content);
        }
        for child in &node.children {
            extract_text_recursive(child, output);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_std::builder::doc;

        #[test]
        fn test_emit_heading() {
            let document = doc(|d| d.heading(2, |i| i.text("Title")));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("== Title =="));
        }

        #[test]
        fn test_emit_bold() {
            let document = doc(|d| d.para(|i| i.strong(|i| i.text("bold"))));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("'''bold'''"));
        }

        #[test]
        fn test_emit_italic() {
            let document = doc(|d| d.para(|i| i.em(|i| i.text("italic"))));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("''italic''"));
        }

        #[test]
        fn test_emit_list() {
            let document =
                doc(|d| d.bullet_list(|l| l.item(|i| i.text("Item 1")).item(|i| i.text("Item 2"))));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("* Item 1"));
            assert!(output.contains("* Item 2"));
        }

        #[test]
        fn test_emit_link() {
            let document =
                doc(|d| d.para(|i| i.link("https://example.com", |i| i.text("Example"))));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("[https://example.com Example]"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::parse;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::emit;
