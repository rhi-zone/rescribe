//! AST↔`rescribe::Document` translation for Textile.
//!
//! This module only translates between [`TextileDoc`](crate::TextileDoc) and
//! rescribe's `Document` IR — no Textile tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, BlockAttrs, Inline, TextileDoc};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse Textile markup.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse Textile markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (doc, _diags) = TextileDoc::parse(input.as_bytes());

        let blocks = doc.blocks.iter().map(convert_block).collect::<Vec<_>>();

        let root = Node::new(node::DOCUMENT).children(blocks);
        let document = Document::new().with_content(root);

        Ok(ConversionResult::ok(document))
    }

    /// Apply block-level attributes (class, id, style, lang) to a node.
    fn apply_block_attrs(mut n: Node, attrs: &BlockAttrs) -> Node {
        if let Some(class) = &attrs.class {
            n = n.prop(prop::CLASSES, class.clone());
        }
        if let Some(id) = &attrs.id {
            n = n.prop(prop::ID, id.clone());
        }
        if let Some(style) = &attrs.style {
            n = n.prop("style", style.clone());
        }
        if let Some(lang) = &attrs.lang {
            n = n.prop("lang", lang.clone());
        }
        if attrs.indent_left > 0 {
            n = n.prop("textile:indent-left", attrs.indent_left as i64);
        }
        if attrs.indent_right > 0 {
            n = n.prop("textile:indent-right", attrs.indent_right as i64);
        }
        n
    }

    fn convert_block(block: &Block) -> Node {
        match block {
            Block::Paragraph {
                inlines,
                align,
                attrs,
                ..
            } => {
                let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
                let mut n = Node::new(node::PARAGRAPH).children(children);
                if let Some(a) = align {
                    n = n.prop(prop::STYLE_ALIGN, a.clone());
                }
                n = apply_block_attrs(n, attrs);
                n
            }

            Block::Heading {
                level,
                inlines,
                attrs,
                ..
            } => {
                let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
                let mut n = Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(children);
                n = apply_block_attrs(n, attrs);
                n
            }

            Block::CodeBlock {
                content, language, ..
            } => {
                let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                if let Some(lang) = language {
                    n = n.prop(prop::LANGUAGE, lang.clone());
                }
                n
            }

            Block::Blockquote { blocks, attrs, .. } => {
                let children: Vec<Node> = blocks.iter().map(convert_block).collect();
                let mut n = Node::new(node::BLOCKQUOTE).children(children);
                n = apply_block_attrs(n, attrs);
                n
            }

            Block::List { ordered, items, .. } => {
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

            Block::Table { rows, .. } => {
                let table_rows: Vec<Node> = rows
                    .iter()
                    .map(|row| {
                        let cells: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                let children: Vec<Node> =
                                    cell.inlines.iter().map(convert_inline).collect();
                                let kind = if cell.is_header {
                                    node::TABLE_HEADER
                                } else {
                                    node::TABLE_CELL
                                };
                                let mut n = Node::new(kind).children(children);
                                if let Some(align) = &cell.align {
                                    n = n.prop(prop::ALIGN, align.clone());
                                }
                                n
                            })
                            .collect();
                        let mut row_node = Node::new(node::TABLE_ROW).children(cells);
                        row_node = apply_block_attrs(row_node, &row.attrs);
                        row_node
                    })
                    .collect();
                Node::new(node::TABLE).children(table_rows)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            Block::FootnoteDef { label, inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
                Node::new(node::FOOTNOTE_DEF)
                    .prop("label", label.clone())
                    .children(children)
            }

            Block::DefinitionList { items, .. } => {
                let children: Vec<Node> = items
                    .iter()
                    .flat_map(|(term, def)| {
                        let term_children: Vec<Node> = term.iter().map(convert_inline).collect();
                        let def_children: Vec<Node> = def.iter().map(convert_inline).collect();
                        vec![
                            Node::new(node::DEFINITION_TERM).children(term_children),
                            Node::new(node::DEFINITION_DESC).children(def_children),
                        ]
                    })
                    .collect();
                Node::new(node::DEFINITION_LIST).children(children)
            }

            Block::Raw { content, .. } => Node::new(node::RAW_BLOCK)
                .prop(prop::CONTENT, content.clone())
                .prop(prop::FORMAT, "textile"),
        }
    }

    fn convert_inline(inline: &Inline) -> Node {
        match inline {
            Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Bold(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::STRONG).children(converted)
            }

            Inline::Italic(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::EMPHASIS).children(converted)
            }

            Inline::Underline(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::UNDERLINE).children(converted)
            }

            Inline::Strikethrough(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::STRIKEOUT).children(converted)
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link {
                url,
                title,
                children,
                ..
            } => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                let mut n = Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(converted);
                if let Some(t) = title {
                    n = n.prop(prop::TITLE, t.clone());
                }
                n
            }

            Inline::Image { url, alt, .. } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if let Some(alt_text) = alt {
                    n = n.prop(prop::ALT, alt_text.clone());
                }
                n
            }

            Inline::Superscript(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::SUPERSCRIPT).children(converted)
            }

            Inline::Subscript(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::SUBSCRIPT).children(converted)
            }

            Inline::FootnoteRef { label, .. } => {
                Node::new(node::FOOTNOTE_REF).prop("label", label.clone())
            }

            Inline::LineBreak(_) => Node::new(node::LINE_BREAK),

            Inline::Raw(content, _) => Node::new(node::RAW_INLINE)
                .prop(prop::CONTENT, content.clone())
                .prop(prop::FORMAT, "textile"),

            Inline::Citation(children, _) => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                Node::new(node::SPAN)
                    .prop("textile:cite", true)
                    .children(converted)
            }

            Inline::GenericSpan {
                attrs, children, ..
            } => {
                let converted: Vec<Node> = children.iter().map(convert_inline).collect();
                let mut n = Node::new(node::SPAN).children(converted);
                n = apply_block_attrs(n, attrs);
                n
            }

            Inline::Acronym { text, title, .. } => Node::new(node::SPAN)
                .prop("textile:abbr", text.clone())
                .prop(prop::TITLE, title.clone()),
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
            let doc = parse_str("h1. Title\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
            assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        }

        #[test]
        fn test_parse_heading_levels() {
            let doc = parse_str("h2. Level 2\nh3. Level 3\n");
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
            let doc = parse_str("*bold*\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children.len(), 1);
            assert_eq!(para.children[0].kind.as_str(), node::STRONG);
        }

        #[test]
        fn test_parse_italic() {
            let doc = parse_str("_italic_\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children.len(), 1);
            assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
        }

        #[test]
        fn test_parse_code() {
            let doc = parse_str("@code@\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children.len(), 1);
            assert_eq!(para.children[0].kind.as_str(), node::CODE);
        }

        #[test]
        fn test_parse_link() {
            let doc = parse_str("\"Example\":https://example.com\n");
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
        fn test_parse_code_block() {
            let doc = parse_str("bc. code here\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, BlockAttrs, Inline, Span, TableCell, TableRow, TextileDoc};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document as Textile markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as Textile markup with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let blocks = doc
            .content
            .children
            .iter()
            .map(convert_node_to_block)
            .collect::<Vec<_>>();

        let textile_doc = TextileDoc {
            blocks,
            span: Span::NONE,
        };
        let output = textile_doc.emit();

        Ok(ConversionResult::ok(output))
    }

    fn convert_node_to_block(node: &Node) -> Block {
        let dummy = Span::NONE;
        match node.kind.as_str() {
            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(6) as u8;
                let inlines = node.children.iter().map(convert_node_to_inline).collect();
                Block::Heading {
                    level,
                    inlines,
                    attrs: BlockAttrs::default(),
                    span: dummy,
                }
            }

            node::PARAGRAPH => {
                let inlines = node.children.iter().map(convert_node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    align: None,
                    attrs: BlockAttrs::default(),
                    span: dummy,
                }
            }

            node::CODE_BLOCK => {
                let content = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Block::CodeBlock {
                    content,
                    language: node.props.get_str(prop::LANGUAGE).map(|s| s.to_string()),
                    span: dummy,
                }
            }

            node::BLOCKQUOTE => {
                let blocks = node.children.iter().map(convert_node_to_block).collect();
                Block::Blockquote {
                    blocks,
                    attrs: BlockAttrs::default(),
                    span: dummy,
                }
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<Block>> = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                    .map(|item| item.children.iter().map(convert_node_to_block).collect())
                    .collect();
                Block::List {
                    ordered,
                    items,
                    span: dummy,
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
                                let inlines: Vec<Inline> =
                                    cell.children.iter().map(convert_node_to_inline).collect();
                                let align = cell.props.get_str(prop::ALIGN).map(|s| s.to_string());
                                TableCell {
                                    is_header,
                                    align,
                                    inlines,
                                    span: dummy,
                                }
                            })
                            .collect();
                        TableRow {
                            attrs: BlockAttrs::default(),
                            cells,
                            span: dummy,
                        }
                    })
                    .collect();
                Block::Table { rows, span: dummy }
            }

            node::DOCUMENT => {
                let inlines: Vec<Inline> =
                    node.children.iter().map(convert_node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    align: None,
                    attrs: BlockAttrs::default(),
                    span: dummy,
                }
            }

            node::DIV | node::SPAN => {
                let inlines: Vec<Inline> =
                    node.children.iter().map(convert_node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    align: None,
                    attrs: BlockAttrs::default(),
                    span: dummy,
                }
            }

            node::FIGURE => {
                let inlines: Vec<Inline> =
                    node.children.iter().map(convert_node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    align: None,
                    attrs: BlockAttrs::default(),
                    span: dummy,
                }
            }

            _ => {
                let inlines: Vec<Inline> =
                    node.children.iter().map(convert_node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    align: None,
                    attrs: BlockAttrs::default(),
                    span: dummy,
                }
            }
        }
    }

    fn convert_node_to_inline(node: &Node) -> Inline {
        let dummy = Span::NONE;
        match node.kind.as_str() {
            node::TEXT => {
                let s = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                Inline::Text(s, dummy)
            }

            node::STRONG => {
                let children = node.children.iter().map(convert_node_to_inline).collect();
                Inline::Bold(children, dummy)
            }

            node::EMPHASIS => {
                let children = node.children.iter().map(convert_node_to_inline).collect();
                Inline::Italic(children, dummy)
            }

            node::UNDERLINE => {
                let children = node.children.iter().map(convert_node_to_inline).collect();
                Inline::Underline(children, dummy)
            }

            node::STRIKEOUT => {
                let children = node.children.iter().map(convert_node_to_inline).collect();
                Inline::Strikethrough(children, dummy)
            }

            node::CODE => {
                let s = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                Inline::Code(s, dummy)
            }

            node::LINK => {
                let url = node
                    .props
                    .get_str(prop::URL)
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                let children = node.children.iter().map(convert_node_to_inline).collect();
                Inline::Link {
                    url,
                    title: node.props.get_str(prop::TITLE).map(|s| s.to_string()),
                    children,
                    span: dummy,
                }
            }

            node::IMAGE => {
                let url = node
                    .props
                    .get_str(prop::URL)
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                let alt = node.props.get_str(prop::ALT).map(|x| x.to_string());
                Inline::Image {
                    url,
                    alt,
                    span: dummy,
                }
            }

            node::SUPERSCRIPT => {
                let children = node.children.iter().map(convert_node_to_inline).collect();
                Inline::Superscript(children, dummy)
            }

            node::SUBSCRIPT => {
                let children = node.children.iter().map(convert_node_to_inline).collect();
                Inline::Subscript(children, dummy)
            }

            node::LINE_BREAK => Inline::Text("\n".to_string(), dummy),

            node::SOFT_BREAK => Inline::Text(" ".to_string(), dummy),

            _ => {
                let children: Vec<Inline> =
                    node.children.iter().map(convert_node_to_inline).collect();
                if children.is_empty() {
                    Inline::Text(String::new(), dummy)
                } else if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    // Wrap multiple children as text sequence
                    Inline::Text(String::new(), dummy)
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
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
            let output = emit_str(&doc);
            assert!(output.contains("h2. Subtitle"));
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
            assert!(output.contains("@code@"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("\"click\":https://example.com"));
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
            assert!(output.contains("bc. "));
            assert!(output.contains("print('hi')"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
