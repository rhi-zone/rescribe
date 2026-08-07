//! AST↔`rescribe::Document` translation, gated behind the `rescribe` feature.
//!
//! This module only ever calls into `crate::parse`/`crate::emit` — it never
//! tokenizes, parses, or emits txt2tags bytes itself. See CLAUDE.md's "The
//! `rescribe` feature module must never contain parsing or writing logic".

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions, Properties};
    use rescribe_std::{node, prop};

    /// Parse txt2tags markup.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse txt2tags markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (t2t_doc, _diagnostics) = crate::parse::parse(input);

        let mut metadata = Properties::new();
        if let Some(title) = &t2t_doc.title {
            metadata.set(prop::TITLE, title.clone());
        }
        if let Some(author) = &t2t_doc.author {
            metadata.set("author", author.clone());
        }
        if let Some(date) = &t2t_doc.date {
            metadata.set("date", date.clone());
        }

        let mut nodes = Vec::new();
        for block in &t2t_doc.blocks {
            nodes.push(block_to_node(block));
        }

        let root = Node::new(node::DOCUMENT).children(nodes);
        let doc = Document::new().with_content(root).with_metadata(metadata);

        Ok(ConversionResult::ok(doc))
    }

    fn block_to_node(block: &Block) -> Node {
        match block {
            Block::Paragraph { inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                Node::new(node::PARAGRAPH).children(children)
            }

            Block::Heading {
                level,
                numbered,
                inlines,
                ..
            } => {
                let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                let mut heading = Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(children);

                if *numbered {
                    heading = heading.prop("numbered", true);
                }

                heading
            }

            Block::CodeBlock { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }

            Block::RawBlock { content, .. } => {
                Node::new(node::RAW_BLOCK).prop(prop::CONTENT, content.clone())
            }

            Block::Blockquote { children, .. } => {
                let para_children: Vec<Node> = children.iter().map(block_to_node).collect();
                Node::new(node::BLOCKQUOTE).children(para_children)
            }

            Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_blocks| {
                        let item_children: Vec<Node> =
                            item_blocks.iter().map(block_to_node).collect();
                        Node::new(node::LIST_ITEM).children(item_children)
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
                            .map(|cell_inlines| {
                                let children: Vec<Node> =
                                    cell_inlines.iter().map(inline_to_node).collect();
                                if row.is_header {
                                    Node::new(node::TABLE_HEADER).children(children)
                                } else {
                                    Node::new(node::TABLE_CELL).children(children)
                                }
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();

                Node::new(node::TABLE).children(table_rows)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            Block::DefinitionList { items, .. } => {
                let children: Vec<Node> = items
                    .iter()
                    .flat_map(|(term, desc)| {
                        let term_children: Vec<Node> = term.iter().map(inline_to_node).collect();
                        let term_node = Node::new(node::DEFINITION_TERM).children(term_children);
                        let desc_children: Vec<Node> = desc.iter().map(block_to_node).collect();
                        let desc_node = Node::new(node::DEFINITION_DESC).children(desc_children);
                        vec![term_node, desc_node]
                    })
                    .collect();
                Node::new(node::DEFINITION_LIST).children(children)
            }
        }
    }

    fn inline_to_node(inline: &Inline) -> Node {
        match inline {
            Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Bold(children, _) => {
                let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::STRONG).children(nodes)
            }

            Inline::Italic(children, _) => {
                let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::EMPHASIS).children(nodes)
            }

            Inline::Underline(children, _) => {
                let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::UNDERLINE).children(nodes)
            }

            Inline::Strikethrough(children, _) => {
                let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::STRIKEOUT).children(nodes)
            }

            Inline::Code(content, _) => Node::new(node::CODE).prop(prop::CONTENT, content.clone()),

            Inline::Link { url, children, .. } => {
                let nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(nodes)
            }

            Inline::Image { url, .. } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),

            Inline::LineBreak(_) => Node::new(node::LINE_BREAK),

            Inline::SoftBreak(_) => Node::new(node::SOFT_BREAK),

            Inline::Verbatim(content, _) => {
                Node::new(node::RAW_INLINE).prop(prop::CONTENT, content.clone())
            }

            Inline::Tagged(content, _) => Node::new(node::RAW_INLINE)
                .prop(prop::CONTENT, content.clone())
                .prop("t2t:tagged", true),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_core::Document;

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
        fn test_parse_numbered_heading() {
            let doc = parse_str("+ Numbered +\n");
            assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
            assert_eq!(
                doc.content.children[0].props.get_bool("numbered"),
                Some(true)
            );
        }

        #[test]
        fn test_parse_paragraph() {
            let doc = parse_str("Hello world\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
        }

        #[test]
        fn test_parse_bold() {
            let doc = parse_str("**bold**\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::STRONG);
        }

        #[test]
        fn test_parse_italic() {
            let doc = parse_str("//italic//\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
        }

        #[test]
        fn test_parse_underline() {
            let doc = parse_str("__underline__\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::UNDERLINE);
        }

        #[test]
        fn test_parse_strikethrough() {
            let doc = parse_str("--strike--\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::STRIKEOUT);
        }

        #[test]
        fn test_parse_monospace() {
            let doc = parse_str("``code``\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::CODE);
        }

        #[test]
        fn test_parse_unordered_list() {
            let doc = parse_str("- item1\n- item2\n");
            assert_eq!(doc.content.children.len(), 1);
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_ordered_list() {
            let doc = parse_str("+ first\n+ second\n");
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
        }

        #[test]
        fn test_parse_verbatim_block() {
            let doc = parse_str("```\ncode here\n```\n");
            assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
            assert_eq!(
                doc.content.children[0].props.get_str(prop::CONTENT),
                Some("code here")
            );
        }

        #[test]
        fn test_parse_link() {
            let doc = parse_str("[click here http://example.com]\n");
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("http://example.com"));
        }

        #[test]
        fn test_parse_quote() {
            let doc = parse_str("\tquoted text\n");
            assert_eq!(doc.content.children[0].kind.as_str(), node::BLOCKQUOTE);
        }

        #[test]
        fn test_skip_comments() {
            let doc = parse_str("% comment\ntext\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, Inline, Span, TableRow};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_std::{node, prop};

    /// Emit a document as txt2tags markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as txt2tags markup with custom options.
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

        let t2t_doc = crate::T2tDoc {
            blocks,
            ..Default::default()
        };
        let output = crate::emit::emit(&t2t_doc);

        Ok(ConversionResult::ok(output.into_bytes()))
    }

    fn node_to_block(node: &Node) -> Option<Block> {
        match node.kind.as_str() {
            node::DOCUMENT => {
                // Document nodes should not be converted directly
                None
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(5) as u8;
                let numbered = node.props.get_bool("numbered").unwrap_or(false);
                let inlines = node.children.iter().map(node_to_inline).collect();

                Some(Block::Heading {
                    level,
                    numbered,
                    inlines,
                    span: Span::NONE,
                })
            }

            node::PARAGRAPH => {
                let inlines = node.children.iter().map(node_to_inline).collect();
                Some(Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                })
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Some(Block::CodeBlock {
                    content,
                    span: Span::NONE,
                })
            }

            node::RAW_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Some(Block::RawBlock {
                    content,
                    span: Span::NONE,
                })
            }

            node::BLOCKQUOTE => {
                let children: Vec<Block> = node.children.iter().filter_map(node_to_block).collect();
                Some(Block::Blockquote {
                    children,
                    span: Span::NONE,
                })
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<Block>> = node
                    .children
                    .iter()
                    .filter_map(|child| {
                        if child.kind.as_str() == node::LIST_ITEM {
                            let item_blocks: Vec<Block> =
                                child.children.iter().filter_map(node_to_block).collect();
                            if !item_blocks.is_empty() {
                                Some(item_blocks)
                            } else {
                                None
                            }
                        } else {
                            None
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
                let rows = node
                    .children
                    .iter()
                    .filter_map(|row_node| {
                        if row_node.kind.as_str() == node::TABLE_ROW {
                            let is_header = row_node
                                .children
                                .first()
                                .map(|c| c.kind.as_str() == node::TABLE_HEADER)
                                .unwrap_or(false);

                            let cells: Vec<Vec<Inline>> = row_node
                                .children
                                .iter()
                                .map(|cell| cell.children.iter().map(node_to_inline).collect())
                                .collect();

                            Some(TableRow {
                                cells,
                                is_header,
                                span: Span::NONE,
                            })
                        } else {
                            None
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
                // These container nodes should not emit themselves
                // but their children may be processed by the parent
                None
            }

            // Inline nodes at block level should be wrapped in paragraphs
            node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK | node::IMAGE => {
                let inlines = vec![node_to_inline(node)];
                Some(Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                })
            }

            _ => None,
        }
    }

    fn node_to_inline(node: &Node) -> Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
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

            node::UNDERLINE => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Underline(children, Span::NONE)
            }

            node::STRIKEOUT => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Strikethrough(children, Span::NONE)
            }

            node::CODE => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Code(content, Span::NONE)
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Link {
                    url,
                    children,
                    span: Span::NONE,
                }
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                Inline::Image {
                    url,
                    span: Span::NONE,
                }
            }

            node::LINE_BREAK => Inline::LineBreak(Span::NONE),

            node::SOFT_BREAK => Inline::SoftBreak(Span::NONE),

            // For unsupported nodes, emit children as text
            _ => {
                let children: Vec<Inline> = node.children.iter().map(node_to_inline).collect();
                match <[Inline; 1]>::try_from(children) {
                    Ok([only]) => only,
                    Err(children) if children.is_empty() => Inline::Text(String::new(), Span::NONE),
                    Err(_) => {
                        // Multiple children, wrap in a generic container
                        // Since t2t doesn't have generic containers, just concatenate
                        Inline::Text(String::new(), Span::NONE)
                    }
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
            assert!(output.contains("**bold**"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("//italic//"));
        }

        #[test]
        fn test_emit_underline() {
            let doc = doc(|d| d.para(|p| p.underline(|u| u.text("underlined"))));
            let output = emit_str(&doc);
            assert!(output.contains("__underlined__"));
        }

        #[test]
        fn test_emit_strikeout() {
            let doc = doc(|d| d.para(|p| p.strike(|s| s.text("strikeout"))));
            let output = emit_str(&doc);
            assert!(output.contains("--strikeout--"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("``code``"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("[click http://example.com]"));
        }

        #[test]
        fn test_emit_unordered_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("- one"));
            assert!(output.contains("- two"));
        }

        #[test]
        fn test_emit_ordered_list() {
            let doc =
                doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
            let output = emit_str(&doc);
            assert!(output.contains("+ first"));
            assert!(output.contains("+ second"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block("print hi"));
            let output = emit_str(&doc);
            assert!(output.contains("```"));
            assert!(output.contains("print hi"));
        }

        #[test]
        fn test_emit_horizontal_rule() {
            use rescribe_core::Node;
            let root = Node::new(node::DOCUMENT).children(vec![Node::new(node::HORIZONTAL_RULE)]);
            let document = Document::new().with_content(root);
            let output = emit_str(&document);
            assert!(output.contains("--------------------"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
