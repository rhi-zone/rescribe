//! AST↔`rescribe::Document` translation for Muse markup.
//!
//! This module only translates between [`MuseDoc`](crate::MuseDoc) and
//! rescribe's `Document` IR — no Muse markup parsing/emitting happens here
//! (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse Muse markup.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse Muse markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        // Parse using the format-specific crate
        let (muse_doc, _diagnostics) = crate::MuseDoc::parse(input.as_bytes());

        // Convert muse_doc to rescribe Document
        let blocks = convert_blocks(&muse_doc.blocks);
        let root = Node::new(node::DOCUMENT).children(blocks);

        let mut metadata = rescribe_core::Properties::new();
        if let Some(t) = &muse_doc.title {
            metadata.set("title", t.clone());
        }
        if let Some(a) = &muse_doc.author {
            metadata.set("author", a.clone());
        }
        if let Some(d) = &muse_doc.date {
            metadata.set("date", d.clone());
        }
        if let Some(d) = &muse_doc.description {
            metadata.set("description", d.clone());
        }
        if let Some(k) = &muse_doc.keywords {
            metadata.set("keywords", k.clone());
        }

        let doc = Document::new().with_content(root).with_metadata(metadata);
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
                        let item_nodes = convert_blocks(item_blocks);
                        Node::new(node::LIST_ITEM).children(item_nodes)
                    })
                    .collect();
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            crate::Block::DefinitionList { items, .. } => {
                let mut children: Vec<Node> = Vec::new();
                for (term_inlines, desc_blocks) in items {
                    let term_node =
                        Node::new(node::DEFINITION_TERM).children(convert_inlines(term_inlines));
                    let desc_node =
                        Node::new(node::DEFINITION_DESC).children(convert_blocks(desc_blocks));
                    children.push(term_node);
                    children.push(desc_node);
                }
                Node::new(node::DEFINITION_LIST).children(children)
            }

            crate::Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            crate::Block::Verse { children, .. } => Node::new(node::BLOCKQUOTE)
                .prop("muse:block-type", "verse")
                .children(convert_blocks(children)),

            crate::Block::CenteredBlock { children, .. } => Node::new(node::DIV)
                .prop("style:align", "center")
                .children(convert_blocks(children)),

            crate::Block::RightBlock { children, .. } => Node::new(node::DIV)
                .prop("style:align", "right")
                .children(convert_blocks(children)),

            crate::Block::LiteralBlock { content, .. } => {
                Node::new(node::RAW_BLOCK).prop(prop::CONTENT, content.clone())
            }

            crate::Block::SrcBlock { lang, content, .. } => {
                let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                if let Some(lang) = lang {
                    n = n.prop(prop::LANGUAGE, lang.clone());
                }
                n
            }

            crate::Block::Comment { content, .. } => Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "muse")
                .prop(prop::CONTENT, content.clone()),

            crate::Block::Table { rows, .. } => {
                let row_nodes: Vec<Node> = rows
                    .iter()
                    .map(|row| {
                        let cell_kind = if row.header {
                            node::TABLE_HEADER
                        } else {
                            node::TABLE_CELL
                        };
                        let cells: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| Node::new(cell_kind).children(convert_inlines(cell)))
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(row_nodes)
            }

            crate::Block::FootnoteDef { label, content, .. } => Node::new(node::FOOTNOTE_DEF)
                .prop(prop::LABEL, label.clone())
                .children(convert_inlines(content)),
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

            crate::Inline::Underline(children, _) => {
                Node::new(node::UNDERLINE).children(convert_inlines(children))
            }

            crate::Inline::Strikethrough(children, _) => {
                Node::new(node::STRIKEOUT).children(convert_inlines(children))
            }

            crate::Inline::Superscript(children, _) => {
                Node::new(node::SUPERSCRIPT).children(convert_inlines(children))
            }

            crate::Inline::Subscript(children, _) => {
                Node::new(node::SUBSCRIPT).children(convert_inlines(children))
            }

            crate::Inline::FootnoteRef { label, .. } => {
                Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone())
            }

            crate::Inline::LineBreak(_) => Node::new(node::LINE_BREAK),

            crate::Inline::Anchor { name, .. } => {
                Node::new(node::SPAN).prop(prop::ID, name.clone())
            }

            crate::Inline::Image { src, alt, .. } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, src.clone());
                if let Some(alt_text) = alt {
                    n = n.prop(prop::ALT, alt_text.clone());
                }
                n
            }
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
            let doc = parse_str("* Title\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
            assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        }

        #[test]
        fn test_parse_heading_levels() {
            let doc = parse_str("** Level 2\n*** Level 3\n");
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
        fn test_parse_emphasis() {
            let doc = parse_str("text with *emphasis*\n");
            let para = &doc.content.children[0];
            assert!(
                para.children
                    .iter()
                    .any(|n| n.kind.as_str() == node::EMPHASIS)
            );
        }

        #[test]
        fn test_parse_code() {
            let doc = parse_str("=code=\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children.len(), 1);
            assert_eq!(para.children[0].kind.as_str(), node::CODE);
        }

        #[test]
        fn test_parse_link() {
            let doc = parse_str("[[https://example.com][Example]]\n");
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
        }

        #[test]
        fn test_parse_unordered_list() {
            let doc = parse_str(" - item1\n - item2\n");
            assert_eq!(doc.content.children.len(), 1);
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), node::LIST);
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_example_block() {
            let doc = parse_str("<example>\ncode here\n</example>\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, Inline, MuseDoc, Span};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document as Muse markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as Muse markup with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        // Convert rescribe nodes to muse blocks
        let blocks = convert_nodes_to_blocks(&doc.content.children);

        // Build using the format-specific crate
        let muse_doc = MuseDoc {
            blocks,
            span: Span::NONE,
            ..Default::default()
        };
        let output = muse_doc.emit();

        Ok(ConversionResult::ok(output))
    }

    fn convert_nodes_to_blocks(nodes: &[Node]) -> Vec<Block> {
        nodes.iter().map(convert_node_to_block).collect()
    }

    fn convert_node_to_block(node: &Node) -> Block {
        match node.kind.as_str() {
            rescribe_std::node::DOCUMENT => {
                // Flatten document, just process children
                // This shouldn't normally happen at top level
                let children: Vec<Block> =
                    node.children.iter().map(convert_node_to_block).collect();
                // Return first block or empty paragraph
                children
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| Block::Paragraph {
                        inlines: vec![],
                        span: Span::NONE,
                    })
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(5) as u8;
                let inlines = convert_nodes_to_inlines(&node.children);
                Block::Heading {
                    level,
                    inlines,
                    span: Span::NONE,
                }
            }

            node::PARAGRAPH => {
                let inlines = convert_nodes_to_inlines(&node.children);
                Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                }
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Block::CodeBlock {
                    content,
                    span: Span::NONE,
                }
            }

            node::BLOCKQUOTE => {
                let children = convert_nodes_to_blocks(&node.children);
                Block::Blockquote {
                    children,
                    span: Span::NONE,
                }
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<Block>> = node
                    .children
                    .iter()
                    .filter(|n| n.kind.as_str() == node::LIST_ITEM)
                    .map(|n| convert_nodes_to_blocks(&n.children))
                    .collect();
                Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                }
            }

            node::DEFINITION_LIST => {
                let mut items = Vec::new();
                let mut i = 0;
                while i < node.children.len() {
                    if node.children[i].kind.as_str() == node::DEFINITION_TERM {
                        let term_inlines = convert_nodes_to_inlines(&node.children[i].children);
                        let mut desc_blocks = Vec::new();
                        if i + 1 < node.children.len()
                            && node.children[i + 1].kind.as_str() == node::DEFINITION_DESC
                        {
                            desc_blocks = convert_nodes_to_blocks(&node.children[i + 1].children);
                            i += 1;
                        }
                        items.push((term_inlines, desc_blocks));
                    }
                    i += 1;
                }
                Block::DefinitionList {
                    items,
                    span: Span::NONE,
                }
            }

            node::HORIZONTAL_RULE => Block::HorizontalRule { span: Span::NONE },

            node::DIV | node::SPAN | node::FIGURE => {
                // Containers that pass through to their children
                let children = convert_nodes_to_blocks(&node.children);
                // Return first block or empty paragraph
                children
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| Block::Paragraph {
                        inlines: vec![],
                        span: Span::NONE,
                    })
            }

            // Inline nodes at block level (shouldn't happen, but handle them)
            node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK => {
                let inlines = vec![convert_node_to_inline(node)];
                Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                }
            }

            _ => {
                // Unknown block type, process children
                let children = convert_nodes_to_blocks(&node.children);
                children
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| Block::Paragraph {
                        inlines: vec![],
                        span: Span::NONE,
                    })
            }
        }
    }

    fn convert_nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
        nodes.iter().map(convert_node_to_inline).collect()
    }

    fn convert_node_to_inline(node: &Node) -> Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Text(content, Span::NONE)
            }

            node::STRONG => {
                let children = convert_nodes_to_inlines(&node.children);
                Inline::Bold(children, Span::NONE)
            }

            node::EMPHASIS => {
                let children = convert_nodes_to_inlines(&node.children);
                Inline::Italic(children, Span::NONE)
            }

            node::CODE => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Code(content, Span::NONE)
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let children = convert_nodes_to_inlines(&node.children);
                Inline::Link {
                    url,
                    children,
                    span: Span::NONE,
                }
            }

            node::STRIKEOUT => {
                let children = convert_nodes_to_inlines(&node.children);
                Inline::Strikethrough(children, Span::NONE)
            }

            node::UNDERLINE => {
                let children = convert_nodes_to_inlines(&node.children);
                Inline::Underline(children, Span::NONE)
            }

            node::SUBSCRIPT => {
                let children = convert_nodes_to_inlines(&node.children);
                Inline::Subscript(children, Span::NONE)
            }

            node::SUPERSCRIPT => {
                let children = convert_nodes_to_inlines(&node.children);
                Inline::Superscript(children, Span::NONE)
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
                Inline::Image {
                    src: url,
                    alt,
                    span: Span::NONE,
                }
            }

            node::LINE_BREAK => Inline::LineBreak(Span::NONE),

            node::SOFT_BREAK => Inline::Text(" ".to_string(), Span::NONE),

            _ => {
                // Unknown inline type, process children
                let children = convert_nodes_to_inlines(&node.children);
                if children.is_empty() {
                    Inline::Text(String::new(), Span::NONE)
                } else {
                    children.into_iter().next().unwrap()
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
            assert!(output.contains("* Title"));
        }

        #[test]
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
            let output = emit_str(&doc);
            assert!(output.contains("** Subtitle"));
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
        fn test_emit_emphasis() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("emphasis"))));
            let output = emit_str(&doc);
            assert!(output.contains("*emphasis*"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("=code="));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("[[https://example.com][click]]"));
        }

        #[test]
        fn test_emit_unordered_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains(" - one"));
            assert!(output.contains(" - two"));
        }

        #[test]
        fn test_emit_ordered_list() {
            let doc =
                doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
            let output = emit_str(&doc);
            assert!(output.contains(" 1. first"));
            assert!(output.contains(" 2. second"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block("print hi"));
            let output = emit_str(&doc);
            assert!(output.contains("<example>"));
            assert!(output.contains("print hi"));
            assert!(output.contains("</example>"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
