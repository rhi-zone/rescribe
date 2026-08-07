//! AST<->`rescribe::Document` translation for ZimWiki.
//!
//! This module only translates between [`ZimwikiDoc`](crate::ZimwikiDoc)
//! and rescribe's `Document` IR — no ZimWiki tokenizing/parsing/emitting
//! happens here. Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline, ListItem, ZimwikiDoc};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse ZimWiki markup.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse ZimWiki markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (zimwiki_doc, _diags) = ZimwikiDoc::parse(input.as_bytes());

        let mut nodes = Vec::new();
        for block in &zimwiki_doc.blocks {
            nodes.push(convert_block(block));
        }

        let root = Node::new(node::DOCUMENT).children(nodes);
        let doc = Document::new().with_content(root);

        Ok(ConversionResult::ok(doc))
    }

    fn convert_block(block: &Block) -> Node {
        match block {
            Block::Paragraph { inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
                Node::new(node::PARAGRAPH).children(children)
            }

            Block::Heading { level, inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(children)
            }

            Block::CodeBlock { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }

            Block::Blockquote { children, .. } => {
                let child_nodes: Vec<Node> = children.iter().map(convert_block).collect();
                Node::new(node::BLOCKQUOTE).children(child_nodes)
            }

            Block::List { ordered, items, .. } => {
                let child_nodes: Vec<Node> = items.iter().map(convert_list_item).collect();
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(child_nodes)
            }

            Block::Table { rows, .. } => {
                let child_nodes: Vec<Node> = rows
                    .iter()
                    .map(|row| {
                        let cells: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                let inlines: Vec<Node> = cell.iter().map(convert_inline).collect();
                                Node::new(node::TABLE_CELL).children(inlines)
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(child_nodes)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),
        }
    }

    fn convert_list_item(item: &ListItem) -> Node {
        let children: Vec<Node> = item.children.iter().map(convert_block).collect();
        let mut list_item = Node::new(node::LIST_ITEM).children(children);
        if let Some(checked) = item.checked {
            list_item = list_item.prop("checked", checked);
        }
        list_item
    }

    fn convert_inline(inline: &Inline) -> Node {
        match inline {
            Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Bold(children, _) => {
                let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::STRONG).children(inlines)
            }

            Inline::Italic(children, _) => {
                let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::EMPHASIS).children(inlines)
            }

            Inline::Underline(children, _) => {
                let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::UNDERLINE).children(inlines)
            }

            Inline::Strikethrough(children, _) => {
                let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::STRIKEOUT).children(inlines)
            }

            Inline::Subscript(children, _) => {
                let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::SUBSCRIPT).children(inlines)
            }

            Inline::Superscript(children, _) => {
                let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::SUPERSCRIPT).children(inlines)
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link { url, children, .. } => {
                let inlines: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(inlines)
            }

            Inline::Image { url, .. } => Node::new(node::IMAGE).prop(prop::URL, url.clone()),

            Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

            Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn parse_str(input: &str) -> Document {
            parse(input).unwrap().value
        }

        #[test]
        fn test_parse_heading_level1() {
            let doc = parse_str("====== Title ======\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
            assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        }

        #[test]
        fn test_parse_heading_level2() {
            let doc = parse_str("===== Subtitle =====\n");
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
        fn test_parse_strikethrough() {
            let doc = parse_str("~~strike~~\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::STRIKEOUT);
        }

        #[test]
        fn test_parse_code() {
            let doc = parse_str("''code''\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children[0].kind.as_str(), node::CODE);
        }

        #[test]
        fn test_parse_link() {
            let doc = parse_str("[[MyPage]]\n");
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("MyPage"));
        }

        #[test]
        fn test_parse_link_with_label() {
            let doc = parse_str("[[MyPage|click here]]\n");
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.props.get_str(prop::URL), Some("MyPage"));
        }

        #[test]
        fn test_parse_unordered_list() {
            let doc = parse_str("* item1\n* item2\n");
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_checkbox_list() {
            let doc = parse_str("[ ] unchecked\n[*] checked\n");
            let list = &doc.content.children[0];
            assert_eq!(list.children[0].props.get_bool("checked"), Some(false));
            assert_eq!(list.children[1].props.get_bool("checked"), Some(true));
        }

        #[test]
        fn test_parse_verbatim() {
            let doc = parse_str("'''\ncode here\n'''\n");
            assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, Inline, ListItem, Span, TableRow, ZimwikiDoc};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document as ZimWiki markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as ZimWiki markup with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut blocks = Vec::new();

        for node in &doc.content.children {
            if let Some(block) = convert_node(node) {
                blocks.push(block);
            }
        }

        let zimwiki_doc = ZimwikiDoc {
            blocks,
            span: crate::Span::NONE,
        };
        let output = zimwiki_doc.emit();

        Ok(ConversionResult::ok(output))
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
                inlines.extend(child.children.iter().filter_map(convert_inline));
            } else if let Some(inline) = convert_inline(child) {
                inlines.push(inline);
            }
        }
        inlines
    }

    fn convert_node(node: &Node) -> Option<Block> {
        match node.kind.as_str() {
            node::DOCUMENT => {
                for child in &node.children {
                    if let Some(block) = convert_node(child) {
                        return Some(block);
                    }
                }
                None
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                let inlines: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
                Some(Block::Heading {
                    level,
                    inlines,
                    span: Span::NONE,
                })
            }

            node::PARAGRAPH => {
                let inlines: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
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

            node::BLOCKQUOTE => {
                let children: Vec<Block> = node.children.iter().filter_map(convert_node).collect();
                Some(Block::Blockquote {
                    children,
                    span: Span::NONE,
                })
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items = node
                    .children
                    .iter()
                    .filter(|n| n.kind.as_str() == node::LIST_ITEM)
                    .map(|n| {
                        let children: Vec<Block> =
                            n.children.iter().filter_map(convert_node).collect();
                        let checked = n.props.get_bool("checked");
                        ListItem {
                            checked,
                            children,
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
                let rows = node
                    .children
                    .iter()
                    .filter(|n| n.kind.as_str() == node::TABLE_ROW)
                    .map(|row| {
                        let cells: Vec<Vec<Inline>> = row
                            .children
                            .iter()
                            .map(|cell| block_content_to_inlines(&cell.children))
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
                for child in &node.children {
                    if let Some(block) = convert_node(child) {
                        return Some(block);
                    }
                }
                None
            }

            _ => None,
        }
    }

    fn convert_inline(node: &Node) -> Option<Inline> {
        match node.kind.as_str() {
            node::TEXT => {
                let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Some(Inline::Text(s, Span::NONE))
            }

            node::STRONG => {
                let children: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
                Some(Inline::Bold(children, Span::NONE))
            }

            node::EMPHASIS => {
                let children: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
                Some(Inline::Italic(children, Span::NONE))
            }

            node::UNDERLINE => {
                let children: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
                Some(Inline::Underline(children, Span::NONE))
            }

            node::STRIKEOUT => {
                let children: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
                Some(Inline::Strikethrough(children, Span::NONE))
            }

            node::SUBSCRIPT => {
                let children: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
                Some(Inline::Subscript(children, Span::NONE))
            }

            node::SUPERSCRIPT => {
                let children: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
                Some(Inline::Superscript(children, Span::NONE))
            }

            node::CODE => {
                let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Some(Inline::Code(s, Span::NONE))
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let children: Vec<Inline> =
                    node.children.iter().filter_map(convert_inline).collect();
                Some(Inline::Link {
                    url,
                    children,
                    span: Span::NONE,
                })
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                Some(Inline::Image {
                    url,
                    span: Span::NONE,
                })
            }

            node::LINE_BREAK => Some(Inline::LineBreak { span: Span::NONE }),

            node::SOFT_BREAK => Some(Inline::SoftBreak { span: Span::NONE }),

            _ => None,
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
        fn test_emit_heading_level1() {
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
        fn test_emit_heading_level3() {
            let doc = doc(|d| d.heading(3, |h| h.text("Section")));
            let output = emit_str(&doc);
            assert!(output.contains("==== Section ===="));
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
        fn test_emit_strikethrough() {
            let doc = doc(|d| d.para(|p| p.strike(|s| s.text("deleted"))));
            let output = emit_str(&doc);
            assert!(output.contains("~~deleted~~"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("''code''"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("MyPage", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("[[MyPage|click]]"));
        }

        #[test]
        fn test_emit_link_no_label() {
            let doc = doc(|d| d.para(|p| p.link("MyPage", |l| l)));
            let output = emit_str(&doc);
            assert!(output.contains("[[MyPage]]"));
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
            assert!(output.contains("'''"));
            assert!(output.contains("print hi"));
        }

        #[test]
        fn test_emit_horizontal_rule() {
            let mut root = Node::new(node::DOCUMENT);
            root.children.push(
                Node::new(node::PARAGRAPH)
                    .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, "before")]),
            );
            root.children.push(Node::new(node::HORIZONTAL_RULE));
            root.children.push(
                Node::new(node::PARAGRAPH)
                    .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, "after")]),
            );
            let doc = Document::new().with_content(root);
            let output = emit_str(&doc);
            assert!(output.contains("----"));
        }

        #[test]
        fn test_emit_image() {
            let mut root = Node::new(node::DOCUMENT);
            root.children.push(
                Node::new(node::PARAGRAPH)
                    .children(vec![Node::new(node::IMAGE).prop(prop::URL, "image.png")]),
            );
            let doc = Document::new().with_content(root);
            let output = emit_str(&doc);
            assert!(output.contains("{{image.png}}"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
