//! AST↔`rescribe::Document` translation for Haddock.
//!
//! This module only translates between this crate's `Block`/`Inline`/
//! `HaddockDoc` types and rescribe's `Document` IR — no Haddock parsing or
//! emitting happens here (that all lives in the rest of this crate; see
//! `crate::parse` and `crate::build`). Enabled by the `rescribe` feature;
//! each direction is additionally gated on the reader/writer mode feature
//! it depends on, so enabling `rescribe` alone (with no mode feature)
//! compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_std::{node, prop};

    /// Parse Haddock markup.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse Haddock markup with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (haddock_doc, _diagnostics) = crate::parse(input);
        let nodes = convert_blocks(&haddock_doc.blocks);

        let root = Node::new(node::DOCUMENT).children(nodes);
        let doc = Document::new().with_content(root);

        Ok(ConversionResult::ok(doc))
    }

    fn convert_blocks(blocks: &[Block]) -> Vec<Node> {
        blocks.iter().map(convert_block).collect()
    }

    fn convert_block(block: &Block) -> Node {
        match block {
            Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(convert_inlines(inlines)),

            Block::Paragraph { inlines, .. } => {
                Node::new(node::PARAGRAPH).children(convert_inlines(inlines))
            }

            Block::CodeBlock { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }

            Block::UnorderedList { items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_inlines| {
                        Node::new(node::LIST_ITEM).children(convert_inlines(item_inlines))
                    })
                    .collect();

                Node::new("unordered_list").children(list_items)
            }

            Block::OrderedList { items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_inlines| {
                        let para =
                            Node::new(node::PARAGRAPH).children(convert_inlines(item_inlines));
                        Node::new(node::LIST_ITEM).children(vec![para])
                    })
                    .collect();

                Node::new(node::LIST)
                    .prop(prop::ORDERED, true)
                    .children(list_items)
            }

            Block::DefinitionList { items, .. } => {
                let mut def_items = Vec::new();
                for (term_inlines, desc_inlines) in items {
                    def_items.push(
                        Node::new(node::DEFINITION_TERM).children(convert_inlines(term_inlines)),
                    );
                    def_items.push(Node::new(node::DEFINITION_DESC).children(vec![
                        Node::new(node::PARAGRAPH).children(convert_inlines(desc_inlines)),
                    ]));
                }
                Node::new(node::DEFINITION_LIST).children(def_items)
            }

            Block::AtCodeBlock { content, .. } => {
                Node::new("at_code_block").prop(prop::CONTENT, content.clone())
            }

            Block::DocTest {
                expression, result, ..
            } => {
                let mut n = Node::new("doc_test").prop("expression", expression.clone());
                if let Some(res) = result {
                    n = n.prop("result", res.clone());
                }
                n
            }

            Block::Blockquote { inlines, .. } => Node::new(node::BLOCKQUOTE).children(vec![
                Node::new(node::PARAGRAPH).children(convert_inlines(inlines)),
            ]),

            Block::Property {
                key,
                name,
                description,
                ..
            } => {
                let mut n = Node::new("property").prop("key", key.clone());
                if let Some(n_name) = name {
                    n = n.prop("name", n_name.clone());
                }
                n.children(convert_inlines(description))
            }
        }
    }

    fn convert_inlines(inlines: &[Inline]) -> Vec<Node> {
        inlines.iter().map(convert_inline).collect()
    }

    fn convert_inline(inline: &Inline) -> Node {
        match inline {
            Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Strong(children, _) => {
                Node::new(node::STRONG).children(convert_inlines(children))
            }

            Inline::Emphasis(children, _) => {
                Node::new(node::EMPHASIS).children(convert_inlines(children))
            }

            Inline::Link { url, text, .. } => {
                let text_node = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(vec![text_node])
            }

            Inline::ModuleLink { module, .. } => {
                Node::new("module_link").prop("module", module.clone())
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
            let doc = parse_str("__bold__\n");
            let para = &doc.content.children[0];
            assert_eq!(para.children.len(), 1);
            assert_eq!(para.children[0].kind.as_str(), node::STRONG);
        }

        #[test]
        fn test_parse_italic() {
            let doc = parse_str("/italic/\n");
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
            let doc = parse_str("\"Example\"<https://example.com>\n");
            let para = &doc.content.children[0];
            let link = &para.children[0];
            assert_eq!(link.kind.as_str(), node::LINK);
            assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
        }

        #[test]
        fn test_parse_unordered_list() {
            let doc = parse_str("* item1\n* item2\n");
            assert_eq!(doc.content.children.len(), 1);
            let list = &doc.content.children[0];
            assert_eq!(list.kind.as_str(), "unordered_list");
            assert_eq!(list.children.len(), 2);
        }

        #[test]
        fn test_parse_code_block() {
            let doc = parse_str("> code here\n");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, HaddockDoc, Inline, Span};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_std::{node, prop};

    /// Emit a document as Haddock markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as Haddock markup with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let blocks = convert_nodes(&doc.content.children);
        let haddock_doc = HaddockDoc {
            blocks,
            span: Span::NONE,
        };
        let output = crate::build(&haddock_doc);

        Ok(ConversionResult::ok(output.into_bytes()))
    }

    fn convert_nodes(nodes: &[Node]) -> Vec<Block> {
        nodes.iter().map(convert_node).collect()
    }

    fn convert_node(node: &Node) -> Block {
        match node.kind.as_str() {
            node::DOCUMENT => {
                let blocks: Vec<_> = node.children.iter().map(convert_node).collect();
                if blocks.is_empty() {
                    Block::Paragraph {
                        inlines: vec![],
                        span: Span::NONE,
                    }
                } else {
                    blocks.into_iter().next().unwrap()
                }
            }

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(6) as u8;
                let inlines = convert_inlines(&node.children);
                Block::Heading {
                    level,
                    inlines,
                    span: Span::NONE,
                }
            }

            node::PARAGRAPH => {
                let inlines = convert_inlines(&node.children);
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

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<Inline>> = node
                    .children
                    .iter()
                    .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                    .map(|item| {
                        let mut inlines = Vec::new();
                        for item_child in &item.children {
                            inlines.extend(convert_inlines(&item_child.children));
                        }
                        inlines
                    })
                    .collect();

                if ordered {
                    Block::OrderedList {
                        items,
                        span: Span::NONE,
                    }
                } else {
                    Block::UnorderedList {
                        items,
                        span: Span::NONE,
                    }
                }
            }

            node::DEFINITION_LIST => {
                let mut items = Vec::new();
                let mut i = 0;
                while i < node.children.len() {
                    if node.children[i].kind.as_str() == node::DEFINITION_TERM {
                        let term_inlines = convert_inlines(&node.children[i].children);
                        let desc_inlines = if i + 1 < node.children.len()
                            && node.children[i + 1].kind.as_str() == node::DEFINITION_DESC
                        {
                            let desc_node = &node.children[i + 1];
                            let mut inlines = Vec::new();
                            for desc_child in &desc_node.children {
                                inlines.extend(convert_inlines(&desc_child.children));
                            }
                            inlines
                        } else {
                            Vec::new()
                        };
                        items.push((term_inlines, desc_inlines));
                        if i + 1 < node.children.len()
                            && node.children[i + 1].kind.as_str() == node::DEFINITION_DESC
                        {
                            i += 2;
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                Block::DefinitionList {
                    items,
                    span: Span::NONE,
                }
            }

            node::DIV | node::SPAN => {
                let blocks = convert_nodes(&node.children);
                if let Some(first) = blocks.first() {
                    first.clone()
                } else {
                    Block::Paragraph {
                        inlines: vec![],
                        span: Span::NONE,
                    }
                }
            }

            node::FIGURE => {
                let blocks = convert_nodes(&node.children);
                if let Some(first) = blocks.first() {
                    first.clone()
                } else {
                    Block::Paragraph {
                        inlines: vec![],
                        span: Span::NONE,
                    }
                }
            }

            _ => {
                let blocks = convert_nodes(&node.children);
                if let Some(first) = blocks.first() {
                    first.clone()
                } else {
                    Block::Paragraph {
                        inlines: vec![],
                        span: Span::NONE,
                    }
                }
            }
        }
    }

    fn convert_inlines(nodes: &[Node]) -> Vec<Inline> {
        nodes.iter().map(convert_inline).collect()
    }

    fn convert_inline(node: &Node) -> Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Text(text, Span::NONE)
            }

            node::CODE => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Code(text, Span::NONE)
            }

            node::STRONG => {
                let children = convert_inlines(&node.children);
                Inline::Strong(children, Span::NONE)
            }

            node::EMPHASIS => {
                let children = convert_inlines(&node.children);
                Inline::Emphasis(children, Span::NONE)
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let text = extract_text(&node.children);
                Inline::Link {
                    url,
                    text,
                    span: Span::NONE,
                }
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                Inline::Link {
                    url: url.clone(),
                    text: url,
                    span: Span::NONE,
                }
            }

            _ => {
                let children = convert_inlines(&node.children);
                if !children.is_empty() {
                    children.into_iter().next().unwrap()
                } else {
                    Inline::Text(String::new(), Span::NONE)
                }
            }
        }
    }

    fn extract_text(nodes: &[Node]) -> String {
        let mut text = String::new();
        for node in nodes {
            match node.kind.as_str() {
                node::TEXT => {
                    if let Some(content) = node.props.get_str(prop::CONTENT) {
                        text.push_str(content);
                    }
                }
                _ => {
                    text.push_str(&extract_text(&node.children));
                }
            }
        }
        text
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
            assert!(output.contains("= Title"));
        }

        #[test]
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
            let output = emit_str(&doc);
            assert!(output.contains("== Subtitle"));
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
            assert!(output.contains("__bold__"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("/italic/"));
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
            assert!(output.contains("\"click\"<https://example.com>"));
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
            assert!(output.contains("(1) first"));
            assert!(output.contains("(2) second"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block("print hi"));
            let output = emit_str(&doc);
            assert!(output.contains("> print hi"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
