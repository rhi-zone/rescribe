//! AST↔`rescribe::Document` translation for Markua.
//!
//! This module only translates between [`MarkuaDoc`](crate::MarkuaDoc) and
//! rescribe's `Document` IR — no Markua tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature.
//!
//! Unlike some other format crates in this workspace, markua has no
//! per-mode reader/writer features to gate against — every API mode is
//! unconditionally compiled in this crate (see `Cargo.toml`'s `[features]`
//! comment) — so `parse`/`emit` below are gated on `feature = "rescribe"`
//! alone.

#[cfg(feature = "rescribe")]
mod read {
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse Markua markup.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse Markua markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (markua_doc, _diagnostics) = crate::MarkuaDoc::parse(input.as_bytes());
        let nodes = convert_blocks(&markua_doc.blocks);

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

            crate::Block::CodeBlock {
                content, language, ..
            } => {
                let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.as_str());
                if let Some(lang) = language {
                    n = n.prop(prop::LANGUAGE, lang.as_str());
                }
                n
            }

            crate::Block::Blockquote { children, .. } => {
                Node::new(node::BLOCKQUOTE).children(convert_blocks(children))
            }

            crate::Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_blocks| {
                        Node::new(node::LIST_ITEM).children(convert_blocks(item_blocks))
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
                            .map(|cell| Node::new(node::TABLE_CELL).children(convert_inlines(cell)))
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();

                Node::new(node::TABLE).children(table_rows)
            }

            crate::Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            crate::Block::SpecialBlock {
                block_type,
                children,
                ..
            } => Node::new(node::DIV)
                .prop("class", block_type.as_str())
                .children(convert_blocks(children)),

            crate::Block::DefinitionList { items, .. } => {
                let dl_nodes: Vec<Node> = items
                    .iter()
                    .flat_map(|(term, def_blocks)| {
                        let dt = Node::new(node::DEFINITION_TERM).children(convert_inlines(term));
                        let dd =
                            Node::new(node::DEFINITION_DESC).children(convert_blocks(def_blocks));
                        vec![dt, dd]
                    })
                    .collect();
                Node::new(node::DEFINITION_LIST).children(dl_nodes)
            }

            crate::Block::PageBreak { .. } => Node::new(node::HORIZONTAL_RULE),

            crate::Block::Figure { caption, body, .. } => {
                let mut children = vec![convert_block(body)];
                if !caption.is_empty() {
                    children.push(Node::new(node::PARAGRAPH).children(convert_inlines(caption)));
                }
                Node::new(node::FIGURE).children(children)
            }
        }
    }

    fn convert_inlines(inlines: &[crate::Inline]) -> Vec<Node> {
        inlines.iter().map(convert_inline).collect()
    }

    fn convert_inline(inline: &crate::Inline) -> Node {
        match inline {
            crate::Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.as_str()),

            crate::Inline::Strong(children, _) => {
                Node::new(node::STRONG).children(convert_inlines(children))
            }

            crate::Inline::Emphasis(children, _) => {
                Node::new(node::EMPHASIS).children(convert_inlines(children))
            }

            crate::Inline::Strikethrough(children, _) => {
                Node::new(node::STRIKEOUT).children(convert_inlines(children))
            }

            crate::Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.as_str()),

            crate::Inline::Link { url, children, .. } => Node::new(node::LINK)
                .prop(prop::URL, url.as_str())
                .children(convert_inlines(children)),

            crate::Inline::Image { url, alt, .. } => Node::new(node::IMAGE)
                .prop(prop::URL, url.as_str())
                .prop(prop::ALT, alt.as_str()),

            crate::Inline::Subscript(children, _) => {
                Node::new(node::SUBSCRIPT).children(convert_inlines(children))
            }

            crate::Inline::Superscript(children, _) => {
                Node::new(node::SUPERSCRIPT).children(convert_inlines(children))
            }

            crate::Inline::Underline(children, _) => {
                Node::new(node::UNDERLINE).children(convert_inlines(children))
            }

            crate::Inline::SmallCaps(children, _) => Node::new(node::SPAN)
                .prop("style:variant", "small-caps")
                .children(convert_inlines(children)),

            crate::Inline::FootnoteRef { content, .. } => {
                Node::new(node::FOOTNOTE_REF).children(convert_inlines(content))
            }

            crate::Inline::IndexTerm { term, .. } => Node::new(node::RAW_INLINE)
                .prop(prop::CONTENT, format!("i[{}]", term))
                .prop("markua:type", "index-term"),

            crate::Inline::MathInline { content, .. } => {
                Node::new("math_inline").prop(prop::CONTENT, content.as_str())
            }

            crate::Inline::LineBreak(_) => Node::new(node::LINE_BREAK),

            crate::Inline::SoftBreak(_) => Node::new(node::SOFT_BREAK),
        }
    }
}

#[cfg(feature = "rescribe")]
mod write {
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document as Markua markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as Markua markup with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let markua_blocks = convert_blocks(&doc.content.children);
        let markua_doc = crate::MarkuaDoc {
            blocks: markua_blocks,
            span: crate::Span::NONE,
            title: None,
            author: None,
            description: None,
        };
        let output = markua_doc.emit();

        Ok(ConversionResult::ok(output))
    }

    fn convert_blocks(nodes: &[Node]) -> Vec<crate::Block> {
        nodes.iter().map(convert_block).collect()
    }

    fn convert_block(node: &Node) -> crate::Block {
        match node.kind.as_str() {
            node::DOCUMENT => {
                if node.children.len() == 1 {
                    convert_block(&node.children[0])
                } else {
                    convert_blocks(&node.children)
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| crate::Block::Paragraph {
                            inlines: Vec::new(),
                            span: crate::Span::NONE,
                        })
                }
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                crate::Block::Heading {
                    level,
                    inlines: convert_inlines(&node.children),
                    span: crate::Span::NONE,
                }
            }

            node::PARAGRAPH => crate::Block::Paragraph {
                inlines: convert_inlines(&node.children),
                span: crate::Span::NONE,
            },

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                crate::Block::CodeBlock {
                    content,
                    language,
                    span: crate::Span::NONE,
                }
            }

            node::BLOCKQUOTE => crate::Block::Blockquote {
                children: convert_blocks(&node.children),
                span: crate::Span::NONE,
            },

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<crate::Block>> = node
                    .children
                    .iter()
                    .map(|item_node| {
                        if item_node.kind.as_str() == node::LIST_ITEM {
                            convert_blocks(&item_node.children)
                        } else {
                            vec![convert_block(item_node)]
                        }
                    })
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
                    .filter(|n| n.kind.as_str() == node::TABLE_ROW)
                    .map(|row_node| {
                        let cells: Vec<Vec<crate::Inline>> = row_node
                            .children
                            .iter()
                            .map(|cell_node| convert_inlines(&cell_node.children))
                            .collect();
                        crate::TableRow {
                            cells,
                            span: crate::Span::NONE,
                        }
                    })
                    .collect();
                crate::Block::Table {
                    rows,
                    span: crate::Span::NONE,
                }
            }

            node::HORIZONTAL_RULE => crate::Block::HorizontalRule {
                span: crate::Span::NONE,
            },

            node::DIV => {
                if let Some(class) = node.props.get_str("class") {
                    let block_type = class.to_string();
                    crate::Block::SpecialBlock {
                        block_type,
                        children: convert_blocks(&node.children),
                        span: crate::Span::NONE,
                    }
                } else {
                    crate::Block::Paragraph {
                        inlines: convert_inlines(&node.children),
                        span: crate::Span::NONE,
                    }
                }
            }

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

            node::STRONG => {
                crate::Inline::Strong(convert_inlines(&node.children), crate::Span::NONE)
            }

            node::EMPHASIS => {
                crate::Inline::Emphasis(convert_inlines(&node.children), crate::Span::NONE)
            }

            node::STRIKEOUT => {
                crate::Inline::Strikethrough(convert_inlines(&node.children), crate::Span::NONE)
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
                let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
                crate::Inline::Image {
                    url,
                    alt,
                    span: crate::Span::NONE,
                }
            }

            node::LINE_BREAK => crate::Inline::LineBreak(crate::Span::NONE),

            node::SOFT_BREAK => crate::Inline::SoftBreak(crate::Span::NONE),

            _ => crate::Inline::Text(String::new(), crate::Span::NONE),
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
            assert!(output.contains("# Title"));
        }

        #[test]
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
            let output = emit_str(&doc);
            assert!(output.contains("## Subtitle"));
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
            assert!(output.contains("*italic*"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("`code`"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("[click](https://example.com)"));
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
            assert!(output.contains("1. first"));
            assert!(output.contains("2. second"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block("print hi"));
            let output = emit_str(&doc);
            assert!(output.contains("```"));
            assert!(output.contains("print hi"));
        }

        #[test]
        fn test_emit_code_block_with_language() {
            let doc = doc(|d| d.code_block_lang("puts 'hello'", "ruby"));
            let output = emit_str(&doc);
            assert!(output.contains("```ruby"));
        }

        #[test]
        fn test_emit_blockquote() {
            let doc = doc(|d| d.blockquote(|b| b.para(|p| p.text("quoted"))));
            let output = emit_str(&doc);
            assert!(output.contains("> quoted"));
        }

        #[test]
        fn test_emit_aside() {
            let div = Node::new(node::DIV).prop("class", "aside").children(vec![
                Node::new(node::PARAGRAPH).children(vec![
                    Node::new(node::TEXT).prop(prop::CONTENT, "This is an aside."),
                ]),
            ]);
            let root = Node::new(node::DOCUMENT).children(vec![div]);
            let doc = Document::new().with_content(root);
            let output = emit_str(&doc);
            assert!(output.contains("A> This is an aside."));
        }

        #[test]
        fn test_emit_warning() {
            let div = Node::new(node::DIV).prop("class", "warning").children(vec![
                Node::new(node::PARAGRAPH).children(vec![
                    Node::new(node::TEXT).prop(prop::CONTENT, "Be careful!"),
                ]),
            ]);
            let root = Node::new(node::DOCUMENT).children(vec![div]);
            let doc = Document::new().with_content(root);
            let output = emit_str(&doc);
            assert!(output.contains("W> Be careful!"));
        }

        #[test]
        fn test_emit_scene_break() {
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
            assert!(output.contains("* * *"));
        }

        #[test]
        fn test_emit_image() {
            let mut root = Node::new(node::DOCUMENT);
            root.children.push(Node::new(node::PARAGRAPH).children(vec![
                Node::new(node::IMAGE)
                    .prop(prop::URL, "image.png")
                    .prop(prop::ALT, "Alt text"),
            ]));
            let doc = Document::new().with_content(root);
            let output = emit_str(&doc);
            assert!(output.contains("![Alt text](image.png)"));
        }
    }
}

#[cfg(feature = "rescribe")]
pub use read::{parse, parse_with_options};
#[cfg(feature = "rescribe")]
pub use write::{emit, emit_with_options};
