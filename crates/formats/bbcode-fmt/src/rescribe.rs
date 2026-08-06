//! AST↔`rescribe::Document` translation for BBCode.
//!
//! This module only translates between [`BbcodeDoc`](crate::BbcodeDoc) and
//! rescribe's `Document` IR — no BBCode tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.
//!
//! # Mapping
//!
//! Blocks and inlines map roughly 1:1 onto rescribe's standard node kinds
//! (`paragraph`, `code_block`, `blockquote`, `list`/`list_item`, `table`,
//! `heading`, `strong`, `emphasis`, `underline`, `strikeout`, `link`,
//! `image`, `subscript`, `superscript`). BBCode-specific constructs with no
//! direct IR equivalent are raw-preserved or approximated: alignment and
//! spoiler blocks become `div` nodes with `style:align`/`bbcode:spoiler`
//! properties, blockquote authors become `bbcode:author`, `[noparse]`
//! becomes `raw_inline`, and generic `[attr=value]...[/attr]` spans become
//! `span` nodes with a `style:{attr}` property.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_std::{node, prop};

    /// Parse BBCode markup into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse BBCode markup with options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (doc, _diagnostics) = crate::parse(input);

        let mut blocks = Vec::new();
        for block in &doc.blocks {
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

    fn block_to_node(block: &Block) -> Node {
        match block {
            Block::Paragraph { inlines, .. } => {
                let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                Node::new(node::PARAGRAPH).children(children)
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

            Block::Blockquote {
                author, children, ..
            } => {
                let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
                let mut n = Node::new(node::BLOCKQUOTE);
                if let Some(a) = author {
                    n = n.prop("bbcode:author", a.clone());
                }
                n.children(block_children)
            }

            Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|inlines| {
                        let inline_nodes: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                        Node::new(node::LIST_ITEM).children(inline_nodes)
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
                            .map(|(is_header, inlines)| {
                                let kind = if *is_header {
                                    node::TABLE_HEADER
                                } else {
                                    node::TABLE_CELL
                                };
                                let children: Vec<Node> =
                                    inlines.iter().map(inline_to_node).collect();
                                Node::new(kind).children(children)
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(table_rows)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            Block::Heading {
                level, children, ..
            } => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(child_nodes)
            }

            Block::Alignment { kind, children, .. } => {
                let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
                let align = match kind {
                    crate::AlignKind::Center => "center",
                    crate::AlignKind::Left => "left",
                    crate::AlignKind::Right => "right",
                };
                Node::new(node::DIV)
                    .prop("style:align", align)
                    .children(block_children)
            }

            Block::Spoiler { children, .. } => {
                let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
                Node::new(node::DIV)
                    .prop("bbcode:spoiler", true)
                    .children(block_children)
            }

            Block::Preformatted { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }

            Block::Indent { children, .. } => {
                let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
                Node::new(node::BLOCKQUOTE).children(block_children)
            }
        }
    }

    fn inline_to_node(inline: &Inline) -> Node {
        match inline {
            Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Bold(children, _) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::STRONG).children(child_nodes)
            }

            Inline::Italic(children, _) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::EMPHASIS).children(child_nodes)
            }

            Inline::Underline(children, _) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::UNDERLINE).children(child_nodes)
            }

            Inline::Strikethrough(children, _) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::STRIKEOUT).children(child_nodes)
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link { url, children, .. } => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(child_nodes)
            }

            Inline::Image {
                url, width, height, ..
            } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                if let Some(w) = width {
                    n = n.prop("bbcode:width", *w as i64);
                }
                if let Some(h) = height {
                    n = n.prop("bbcode:height", *h as i64);
                }
                n
            }

            Inline::Subscript(children, _) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::SUBSCRIPT).children(child_nodes)
            }

            Inline::Superscript(children, _) => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::SUPERSCRIPT).children(child_nodes)
            }

            Inline::Color {
                value, children, ..
            } => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::SPAN)
                    .prop("style:color", value.clone())
                    .children(child_nodes)
            }

            Inline::Size {
                value, children, ..
            } => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::SPAN)
                    .prop("style:size", value.clone())
                    .children(child_nodes)
            }

            Inline::Font { name, children, .. } => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::SPAN)
                    .prop("style:font", name.clone())
                    .children(child_nodes)
            }

            Inline::Email { addr, children, .. } => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                Node::new(node::LINK)
                    .prop(prop::URL, format!("mailto:{}", addr))
                    .children(child_nodes)
            }

            Inline::Noparse(s, _) => Node::new(node::RAW_INLINE).prop(prop::CONTENT, s.clone()),

            Inline::Span {
                attr,
                value,
                children,
                ..
            } => {
                let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
                let prop_key = format!("style:{}", attr);
                Node::new(node::SPAN)
                    .prop(prop_key, value.clone())
                    .children(child_nodes)
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_bold() {
            let result = parse("This is [b]bold[/b] text").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_italic() {
            let result = parse("This is [i]italic[/i] text").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_link() {
            let result = parse("[url=http://example.com]Example[/url]").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_list() {
            let result = parse("[list]\n[*]Item 1\n[*]Item 2\n[/list]").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_code() {
            let result = parse("[code]print('hello')[/code]").unwrap();
            assert!(!result.value.content.children.is_empty());
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, Inline, Span, TableRow};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_std::{node, prop};

    /// Emit a document to BBCode markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document to BBCode markup with options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut blocks = Vec::new();
        for child in &doc.content.children {
            if child.kind.as_str() != node::DOCUMENT {
                blocks.push(node_to_block(child));
            } else {
                for doc_child in &child.children {
                    blocks.push(node_to_block(doc_child));
                }
            }
        }

        let bbcode_doc = crate::BbcodeDoc {
            blocks,
            span: crate::Span::NONE,
        };
        let output = crate::emit(&bbcode_doc);
        Ok(ConversionResult::ok(output.into_bytes()))
    }

    fn node_to_block(node: &Node) -> Block {
        match node.kind.as_str() {
            node::PARAGRAPH => {
                let inlines = node.children.iter().map(node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                }
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                Block::CodeBlock {
                    language,
                    content,
                    span: Span::NONE,
                }
            }

            node::BLOCKQUOTE => {
                let children = node.children.iter().map(node_to_block).collect();
                Block::Blockquote {
                    author: None,
                    children,
                    span: Span::NONE,
                }
            }

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                    .map(|item| {
                        item.children
                            .iter()
                            .filter(|child| child.kind.as_str() == node::PARAGRAPH)
                            .flat_map(|para| {
                                para.children.iter().map(node_to_inline).collect::<Vec<_>>()
                            })
                            .collect()
                    })
                    .collect();
                Block::List {
                    ordered,
                    items,
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
                                let inlines = cell.children.iter().map(node_to_inline).collect();
                                (is_header, inlines)
                            })
                            .collect();
                        TableRow {
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

            node::HEADING => {
                // BBCode doesn't have native headings - use bold text in paragraph
                let inlines = node.children.iter().map(node_to_inline).collect();
                Block::Paragraph {
                    inlines: vec![Inline::Bold(inlines, Span::NONE)],
                    span: Span::NONE,
                }
            }

            node::DIV | node::SPAN | node::FIGURE => {
                // Transparent wrapper - emit children as paragraph
                let inlines = node.children.iter().map(node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                }
            }

            _ => {
                // Fallback: treat as paragraph
                let inlines = node.children.iter().map(node_to_inline).collect();
                Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                }
            }
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
                    width: None,
                    height: None,
                    span: Span::NONE,
                }
            }

            node::SUBSCRIPT => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Subscript(children, Span::NONE)
            }

            node::SUPERSCRIPT => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Superscript(children, Span::NONE)
            }

            node::LINE_BREAK => {
                // BBCode doesn't have explicit line break inline element
                // Use newline in text instead
                Inline::Text("\n".to_string(), Span::NONE)
            }

            node::SOFT_BREAK => Inline::Text(" ".to_string(), Span::NONE),

            node::SPAN => {
                let attr = node
                    .props
                    .get_str("style:color")
                    .map(|_| "color".to_string())
                    .unwrap_or_else(|| "color".to_string());
                let value = node
                    .props
                    .get_str("style:color")
                    .or_else(|| node.props.get_str("style:size"))
                    .unwrap_or("inherit")
                    .to_string();
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Span {
                    attr,
                    value,
                    children,
                    span: Span::NONE,
                }
            }

            _ => {
                // Fallback: collect children
                let children: Vec<Inline> = node.children.iter().map(node_to_inline).collect();
                if children.is_empty() {
                    Inline::Text("".to_string(), Span::NONE)
                } else if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    // Wrap in a container if multiple children
                    Inline::Bold(children, Span::NONE)
                }
            }
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
        fn test_emit_bold() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            assert!(emit_str(&doc).contains("[b]bold[/b]"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            assert!(emit_str(&doc).contains("[i]italic[/i]"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("Example"))));
            assert!(emit_str(&doc).contains("[url=http://example.com]Example[/url]"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("[list]"));
            assert!(output.contains("[*]one"));
            assert!(output.contains("[/list]"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block("print('hello')"));
            let output = emit_str(&doc);
            assert!(output.contains("[code]"));
            assert!(output.contains("print('hello')"));
            assert!(output.contains("[/code]"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
