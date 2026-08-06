//! AST↔`rescribe::Document` translation for Jira wiki markup.
//!
//! This module only translates between [`JiraDoc`](crate::JiraDoc) and
//! rescribe's `Document` IR — no Jira markup tokenizing/parsing/emitting
//! happens here (that all lives in the rest of this crate; see
//! `crate::parse` and `crate::emit`). Enabled by the `rescribe` feature;
//! each direction is additionally gated on the reader/writer mode feature
//! it depends on, so enabling `rescribe` alone (with no mode feature)
//! compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline, ListItem, ListItemContent, parse_str as jira_parse};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_std::{node, prop};

    /// Parse Jira markup source into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse Jira markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (jira_doc, _diags) = jira_parse(input);

        let mut children = Vec::new();
        for block in jira_doc.blocks {
            children.push(block_to_node(&block));
        }

        let root = Node::new(node::DOCUMENT).children(children);
        let doc = Document::new().with_content(root);
        Ok(ConversionResult::ok(doc))
    }

    fn block_to_node(block: &Block) -> Node {
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

            Block::Blockquote { children, .. } => {
                let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
                Node::new(node::BLOCKQUOTE).children(block_children)
            }

            Block::Panel {
                title, children, ..
            } => {
                let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
                let mut n = Node::new(node::DIV).prop("jira:type", "panel");
                if let Some(t) = title {
                    n = n.prop("title", t.clone());
                }
                n.children(block_children)
            }

            Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items.iter().map(list_item_to_node).collect();
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            Block::Noformat { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }

            Block::Table { rows, .. } => {
                let mut result_rows = Vec::new();
                let has_header = rows
                    .first()
                    .is_some_and(|r| r.cells.iter().all(|c| c.is_header));

                let mut row_iter = rows.iter().peekable();
                if has_header && let Some(header_row) = row_iter.next() {
                    let cells: Vec<Node> = header_row
                        .cells
                        .iter()
                        .map(|cell| {
                            Node::new(node::TABLE_HEADER).children(inlines_to_nodes(&cell.inlines))
                        })
                        .collect();
                    result_rows.push(
                        Node::new(node::TABLE_HEAD)
                            .child(Node::new(node::TABLE_ROW).children(cells)),
                    );
                }

                for row in row_iter {
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
                    result_rows.push(Node::new(node::TABLE_ROW).children(cells));
                }

                Node::new(node::TABLE).children(result_rows)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),
        }
    }

    fn list_item_to_node(item: &ListItem) -> Node {
        let mut item_children = Vec::new();
        for content in &item.children {
            match content {
                ListItemContent::Inline(inlines) => {
                    let para = Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines));
                    item_children.push(para);
                }
                ListItemContent::NestedList(block) => {
                    item_children.push(block_to_node(block));
                }
            }
        }
        Node::new(node::LIST_ITEM).children(item_children)
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

            Inline::Strikethrough(children, _) => {
                Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link { url, children, .. } => Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(inlines_to_nodes(children)),

            Inline::Image { url, alt, .. } => {
                let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if let Some(alt_text) = alt {
                    img = img.prop(prop::ALT, alt_text.clone());
                }
                img
            }

            Inline::Superscript(children, _) => {
                Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::Subscript(children, _) => {
                Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::ColorSpan {
                color, children, ..
            } => Node::new(node::SPAN)
                .prop("style:color", color.clone())
                .children(inlines_to_nodes(children)),

            Inline::Mention(name, _) => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "jira")
                .prop(prop::CONTENT, format!("@{}", name)),
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
            let doc = parse_str("h1. Title");
            let heading = &doc.content.children[0];
            assert_eq!(heading.kind.as_str(), node::HEADING);
            assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
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
            assert_eq!(para.children[1].kind.as_str(), node::STRONG);
        }

        #[test]
        fn test_parse_italic() {
            let doc = parse_str("This is _italic_ text.");
            let para = &doc.content.children[0];
            assert_eq!(para.children[1].kind.as_str(), node::EMPHASIS);
        }

        #[test]
        fn test_parse_code() {
            let doc = parse_str("Use {{code}} here.");
            let para = &doc.content.children[0];
            assert_eq!(para.children[1].kind.as_str(), node::CODE);
        }

        #[test]
        fn test_parse_link() {
            let doc = parse_str("Click [here|https://example.com].");
            let para = &doc.content.children[0];
            let link = &para.children[1];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
        }

        #[test]
        fn test_parse_list() {
            let doc = parse_str("* Item 1\n* Item 2");
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_code_block() {
            let doc = parse_str("{code:java}\npublic class Test {}\n{code}");
            let code = &doc.content.children[0];
            assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
            assert_eq!(code.props.get_str(prop::LANGUAGE), Some("java"));
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, Inline, JiraDoc, Span};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit;
    use rescribe_std::{node, prop};

    /// Emit a document as Jira markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as Jira markup with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut blocks = Vec::new();
        for child in &doc.content.children {
            blocks.push(node_to_block(child));
        }

        let jira_doc = JiraDoc {
            blocks,
            span: Span::NONE,
        };
        let output = jira_doc.emit();
        Ok(ConversionResult::ok(output))
    }

    fn node_to_block(node: &Node) -> Block {
        match node.kind.as_str() {
            node::PARAGRAPH => Block::Paragraph {
                inlines: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(6) as u8;
                Block::Heading {
                    level,
                    inlines: nodes_to_inlines(&node.children),
                    span: Span::NONE,
                }
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                Block::CodeBlock {
                    content,
                    language,
                    span: Span::NONE,
                }
            }

            node::BLOCKQUOTE => {
                let children: Vec<Block> = node.children.iter().map(node_to_block).collect();
                Block::Blockquote {
                    children,
                    span: Span::NONE,
                }
            }

            node::DIV => {
                let is_panel = node
                    .props
                    .get_str("jira:type")
                    .map(|s| s == "panel")
                    .unwrap_or(false);
                if is_panel {
                    let title = node.props.get_str("jira:panel-title").map(|s| s.to_owned());
                    let children: Vec<Block> = node.children.iter().map(node_to_block).collect();
                    Block::Panel {
                        title,
                        children,
                        span: Span::NONE,
                    }
                } else {
                    let children: Vec<Block> = node.children.iter().map(node_to_block).collect();
                    Block::Blockquote {
                        children,
                        span: Span::NONE,
                    }
                }
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let mut items = Vec::new();
                for child in &node.children {
                    if child.kind.as_str() == node::LIST_ITEM {
                        let mut content = Vec::new();
                        for block_node in &child.children {
                            if block_node.kind.as_str() == node::LIST {
                                content.push(crate::ast::ListItemContent::NestedList(
                                    node_to_block(block_node),
                                ));
                            } else {
                                // Treat any other block as inline content
                                content.push(crate::ast::ListItemContent::Inline(
                                    nodes_to_inlines(&block_node.children),
                                ));
                            }
                        }
                        items.push(crate::ast::ListItem { children: content });
                    }
                }
                Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                }
            }

            node::TABLE => {
                let mut rows = Vec::new();
                for child in &node.children {
                    if child.kind.as_str() == node::TABLE_HEAD {
                        for row_node in &child.children {
                            rows.push(node_to_table_row(row_node));
                        }
                    } else if child.kind.as_str() == node::TABLE_ROW {
                        rows.push(node_to_table_row(child));
                    }
                }
                Block::Table {
                    rows,
                    span: Span::NONE,
                }
            }

            node::HORIZONTAL_RULE => Block::HorizontalRule { span: Span::NONE },

            _ => Block::Paragraph {
                inlines: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },
        }
    }

    fn node_to_table_row(node: &Node) -> crate::TableRow {
        let mut cells = Vec::new();
        for child in &node.children {
            let is_header = child.kind.as_str() == node::TABLE_HEADER;
            cells.push(crate::TableCell {
                is_header,
                inlines: nodes_to_inlines(&child.children),
                span: Span::NONE,
            });
        }
        crate::TableRow {
            cells,
            span: Span::NONE,
        }
    }

    fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
        nodes.iter().map(node_to_inline).collect()
    }

    fn node_to_inline(node: &Node) -> Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Text(text, Span::NONE)
            }

            node::STRONG => Inline::Bold(nodes_to_inlines(&node.children), Span::NONE),

            node::EMPHASIS => Inline::Italic(nodes_to_inlines(&node.children), Span::NONE),

            node::UNDERLINE => Inline::Underline(nodes_to_inlines(&node.children), Span::NONE),

            node::STRIKEOUT => Inline::Strikethrough(nodes_to_inlines(&node.children), Span::NONE),

            node::CODE => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Code(text, Span::NONE)
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let children = nodes_to_inlines(&node.children);
                Inline::Link {
                    url,
                    children,
                    span: Span::NONE,
                }
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
                Inline::Image {
                    url,
                    alt,
                    span: Span::NONE,
                }
            }

            node::SUPERSCRIPT => Inline::Superscript(nodes_to_inlines(&node.children), Span::NONE),

            node::SUBSCRIPT => Inline::Subscript(nodes_to_inlines(&node.children), Span::NONE),

            _ => {
                let children = nodes_to_inlines(&node.children);
                if children.is_empty() {
                    Inline::Text(String::new(), Span::NONE)
                } else if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    Inline::Text(String::new(), Span::NONE)
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
            assert!(output.contains("h1. Title"));
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
            assert!(output.contains("{{code}}"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("[click|https://example.com]"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
            let output = emit_str(&doc);
            assert!(output.contains("{code:python}"));
            assert!(output.contains("print('hi')"));
            assert!(output.contains("{code}"));
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
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
