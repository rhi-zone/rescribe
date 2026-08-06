//! AST↔`rescribe::Document` translation for Creole.
//!
//! This module only translates between [`CreoleDoc`](crate::CreoleDoc) and
//! rescribe's `Document` IR — no Creole tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.
//!
//! # Mapping
//!
//! Blocks and inlines map roughly 1:1 onto rescribe's standard node kinds
//! (`paragraph`, `heading`, `code_block`, `blockquote`, `list`/`list_item`,
//! `table`, `definition_list`/`definition_term`/`definition_desc`,
//! `horizontal_rule`, `strong`, `emphasis`, `code`, `link`, `image`,
//! `line_break`). Creole has no strikethrough/underline/superscript/
//! subscript, so the writer unwraps those inline kinds to their children
//! (or, for multiple children, falls back to a debug-formatted text node)
//! when emitting back to Creole markup.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_std::{node, prop};

    /// Parse Creole markup.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse Creole markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (creole_doc, _diagnostics) = crate::parse(input);
        let nodes = convert_blocks(&creole_doc.blocks);
        let root = Node::new(node::DOCUMENT).children(nodes);
        let doc = Document::new().with_content(root);
        Ok(ConversionResult::ok(doc))
    }

    fn convert_blocks(blocks: &[crate::Block]) -> Vec<Node> {
        blocks.iter().map(convert_block).collect()
    }

    fn convert_block(block: &crate::Block) -> Node {
        match block {
            crate::Block::Paragraph { inlines, .. } => {
                Node::new(node::PARAGRAPH).children(convert_inlines(inlines))
            }

            crate::Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(convert_inlines(inlines)),

            crate::Block::CodeBlock { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }

            crate::Block::Blockquote { children, .. } => {
                Node::new(node::BLOCKQUOTE).children(convert_blocks(children))
            }

            crate::Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_blocks| {
                        let children: Vec<Node> = item_blocks.iter().map(convert_block).collect();
                        Node::new(node::LIST_ITEM).children(children)
                    })
                    .collect();
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            crate::Block::Table { rows, .. } => {
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
                                Node::new(kind).children(convert_inlines(&cell.inlines))
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(table_rows)
            }

            crate::Block::DefinitionList { items, .. } => {
                let children: Vec<Node> = items
                    .iter()
                    .flat_map(|item| {
                        let term_children: Vec<Node> =
                            item.term.iter().map(convert_inline).collect();
                        let desc_children: Vec<Node> =
                            item.desc.iter().map(convert_inline).collect();
                        vec![
                            Node::new(node::DEFINITION_TERM).children(term_children),
                            Node::new(node::DEFINITION_DESC).children(desc_children),
                        ]
                    })
                    .collect();
                Node::new(node::DEFINITION_LIST).children(children)
            }

            crate::Block::HorizontalRule(_) => Node::new(node::HORIZONTAL_RULE),
        }
    }

    fn convert_inlines(inlines: &[crate::Inline]) -> Vec<Node> {
        inlines.iter().map(convert_inline).collect()
    }

    fn convert_inline(inline: &crate::Inline) -> Node {
        match inline {
            crate::Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            crate::Inline::Bold(children, _) => {
                Node::new(node::STRONG).children(convert_inlines(children))
            }

            crate::Inline::Italic(children, _) => {
                Node::new(node::EMPHASIS).children(convert_inlines(children))
            }

            crate::Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            crate::Inline::Link { url, children, .. } => Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(convert_inlines(children)),

            crate::Inline::Image { url, alt, .. } => {
                let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if let Some(alt_text) = alt {
                    img = img.prop(prop::ALT, alt_text.clone());
                }
                img
            }

            crate::Inline::LineBreak(_) => Node::new(node::LINE_BREAK),
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
            let doc = parse_str("= Title\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
            assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        }

        #[test]
        fn test_parse_heading_levels() {
            let doc = parse_str("== Level 2\n=== Level 3\n");
            assert_eq!(doc.content.children.len(), 2);
            assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(2));
            assert_eq!(doc.content.children[1].props.get_int(prop::LEVEL), Some(3));
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
            assert_eq!(para.children.len(), 1);
            assert_eq!(para.children[0].kind.as_str(), node::STRONG);
        }

        #[test]
        fn test_parse_italic() {
            let doc = parse_str("//italic//\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children.len(), 1);
            assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
        }

        #[test]
        fn test_parse_link() {
            let doc = parse_str("[[https://example.com|Example]]\n");
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
        }

        #[test]
        fn test_parse_list() {
            let doc = parse_str("* item1\n* item2\n");
            assert_eq!(doc.content.children.len(), 1);
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_nowiki() {
            let doc = parse_str("{{{code}}}\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document as Creole markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as Creole markup with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let blocks = convert_nodes(&doc.content.children);
        let creole_doc = crate::CreoleDoc { blocks };
        let output = creole_doc.emit();
        Ok(ConversionResult::ok(output))
    }

    fn convert_nodes(nodes: &[Node]) -> Vec<crate::Block> {
        nodes.iter().map(convert_node).collect()
    }

    fn convert_node(node: &Node) -> crate::Block {
        match node.kind.as_str() {
            node::DOCUMENT => {
                // Shouldn't happen in well-formed rescribe tree, but handle it
                let children = convert_nodes(&node.children);
                // Return the first block or an empty paragraph
                children
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| crate::Block::Paragraph {
                        inlines: vec![],
                        span: crate::Span::NONE,
                    })
            }

            node::PARAGRAPH => crate::Block::Paragraph {
                inlines: convert_inlines(&node.children),
                span: crate::Span::NONE,
            },

            node::HEADING => {
                let level = (node.props.get_int(prop::LEVEL).unwrap_or(1).clamp(1, 6)) as u8;
                crate::Block::Heading {
                    level,
                    inlines: convert_inlines(&node.children),
                    span: crate::Span::NONE,
                }
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                crate::Block::CodeBlock {
                    content,
                    span: crate::Span::NONE,
                }
            }

            node::BLOCKQUOTE => crate::Block::Blockquote {
                children: convert_nodes(&node.children),
                span: crate::Span::NONE,
            },

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<crate::Block>> = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                    .map(|item| convert_nodes(&item.children))
                    .collect();
                crate::Block::List {
                    ordered,
                    items,
                    span: crate::Span::NONE,
                }
            }

            node::TABLE => {
                let rows: Vec<crate::TableRow> = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                    .map(|row| crate::TableRow {
                        cells: row
                            .children
                            .iter()
                            .map(|cell| crate::TableCell {
                                is_header: cell.kind.as_str() == node::TABLE_HEADER,
                                inlines: convert_inlines(&cell.children),
                                span: crate::Span::NONE,
                            })
                            .collect(),
                        span: crate::Span::NONE,
                    })
                    .collect();
                crate::Block::Table {
                    rows,
                    span: crate::Span::NONE,
                }
            }

            node::HORIZONTAL_RULE => crate::Block::HorizontalRule(crate::Span::NONE),

            // Handle other nodes by recursing on children
            _ => crate::Block::Paragraph {
                inlines: convert_inlines(&node.children),
                span: crate::Span::NONE,
            },
        }
    }

    fn convert_inlines(nodes: &[Node]) -> Vec<crate::Inline> {
        nodes.iter().map(convert_inline).collect()
    }

    fn convert_inline(node: &Node) -> crate::Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                crate::Inline::Text(content, crate::Span::NONE)
            }

            node::STRONG => crate::Inline::Bold(convert_inlines(&node.children), crate::Span::NONE),

            node::EMPHASIS => {
                crate::Inline::Italic(convert_inlines(&node.children), crate::Span::NONE)
            }

            node::CODE => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                crate::Inline::Code(content, crate::Span::NONE)
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let children = convert_inlines(&node.children);
                crate::Inline::Link {
                    url,
                    children,
                    span: crate::Span::NONE,
                }
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
                crate::Inline::Image {
                    url,
                    alt,
                    span: crate::Span::NONE,
                }
            }

            node::LINE_BREAK => crate::Inline::LineBreak(crate::Span::NONE),

            // Creole doesn't have strikethrough, underline, superscript, subscript
            // Just emit the children instead
            node::STRIKEOUT | node::UNDERLINE | node::SUPERSCRIPT | node::SUBSCRIPT => {
                // Wrap multiple inlines in a text node if they're all text
                let children = convert_inlines(&node.children);
                if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    crate::Inline::Text(format!("{:?}", children), crate::Span::NONE)
                }
            }

            // Fallback: recurse
            _ => {
                let children = convert_inlines(&node.children);
                if children.is_empty() {
                    crate::Inline::Text(String::new(), crate::Span::NONE)
                } else if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    crate::Inline::Text(format!("{:?}", children), crate::Span::NONE)
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
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("{{{code}}}"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("[[https://example.com|click]]"));
        }

        #[test]
        fn test_emit_list() {
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
            assert!(output.contains("# first"));
            assert!(output.contains("# second"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block("print('hi')"));
            let output = emit_str(&doc);
            assert!(output.contains("{{{\n"));
            assert!(output.contains("print('hi')"));
            assert!(output.contains("}}}\n"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
