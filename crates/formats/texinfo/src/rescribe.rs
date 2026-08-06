//! AST↔`rescribe::Document` translation for Texinfo.
//!
//! This module only translates between [`TexinfoDoc`](crate::TexinfoDoc)
//! and rescribe's `Document` IR — no Texinfo parsing/emitting happens here
//! (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Block, Inline};
    use rescribe_core::{
        ConversionResult, Document, FidelityWarning, Node, ParseError, ParseOptions,
    };
    use rescribe_std::{node, prop};

    /// Parse Texinfo into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse Texinfo with options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (texinfo_doc, _parse_diags) = crate::parse::parse(input);

        let mut warnings: Vec<FidelityWarning> = Vec::new();
        let mut result_nodes = Vec::new();

        for block in texinfo_doc.blocks {
            result_nodes.push(block_to_node(&block, &mut warnings));
        }

        let mut metadata = rescribe_core::Properties::new();
        if let Some(title) = texinfo_doc.title {
            metadata.set("title", title);
        }

        let document = Document {
            content: Node::new(node::DOCUMENT).children(result_nodes),
            resources: Default::default(),
            metadata,
            source: None,
        };

        Ok(ConversionResult::with_warnings(document, warnings))
    }

    fn block_to_node(block: &Block, _warnings: &mut Vec<FidelityWarning>) -> Node {
        match block {
            Block::Heading { level, inlines, .. } => {
                let inline_nodes = inlines_to_nodes(inlines);
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(inline_nodes)
            }

            Block::Paragraph { inlines, .. } => {
                let inline_nodes = inlines_to_nodes(inlines);
                Node::new(node::PARAGRAPH).children(inline_nodes)
            }

            Block::CodeBlock { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }

            Block::Blockquote { children, .. } => {
                let block_nodes: Vec<_> = children
                    .iter()
                    .map(|b| block_to_node(b, _warnings))
                    .collect();
                Node::new(node::BLOCKQUOTE).children(block_nodes)
            }

            Block::List { ordered, items, .. } => {
                let list_items: Vec<_> = items
                    .iter()
                    .map(|item_inlines| {
                        let inline_nodes = inlines_to_nodes(item_inlines);
                        Node::new(node::LIST_ITEM).children(inline_nodes)
                    })
                    .collect();

                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            Block::DefinitionList { items, .. } => {
                let mut def_nodes = Vec::new();
                for (term_inlines, desc_blocks) in items {
                    let term_inline_nodes = inlines_to_nodes(term_inlines);
                    def_nodes.push(Node::new(node::DEFINITION_TERM).children(term_inline_nodes));

                    let desc_block_nodes: Vec<_> = desc_blocks
                        .iter()
                        .map(|b| block_to_node(b, _warnings))
                        .collect();
                    def_nodes.push(Node::new(node::DEFINITION_DESC).children(desc_block_nodes));
                }
                Node::new(node::DEFINITION_LIST).children(def_nodes)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            Block::Table { rows, .. } => {
                let row_nodes: Vec<_> = rows
                    .iter()
                    .map(|row| {
                        let cells: Vec<_> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                let cell_kind = if row.is_header {
                                    node::TABLE_HEADER
                                } else {
                                    node::TABLE_CELL
                                };
                                Node::new(cell_kind).children(inlines_to_nodes(cell))
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(row_nodes)
            }

            Block::Menu { entries, .. } => {
                let mut def_nodes = Vec::new();
                for entry in entries {
                    def_nodes.push(
                        Node::new(node::DEFINITION_TERM)
                            .child(Node::new(node::TEXT).prop(prop::CONTENT, entry.node.clone())),
                    );
                    let desc_text = entry.description.clone().unwrap_or_default();
                    def_nodes.push(
                        Node::new(node::DEFINITION_DESC).child(
                            Node::new(node::PARAGRAPH)
                                .child(Node::new(node::TEXT).prop(prop::CONTENT, desc_text)),
                        ),
                    );
                }
                Node::new("menu").children(def_nodes)
            }

            Block::RawBlock {
                environment,
                content,
                ..
            } => Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, environment.clone())
                .prop(prop::CONTENT, content.clone()),

            Block::Float {
                float_type,
                label,
                children,
                ..
            } => {
                let mut n = Node::new("float");
                if let Some(ft) = float_type {
                    n = n.prop("texinfo:float-type", ft.clone());
                }
                if let Some(lbl) = label {
                    n = n.prop(prop::LABEL, lbl.clone());
                }
                let block_nodes: Vec<_> = children
                    .iter()
                    .map(|b| block_to_node(b, _warnings))
                    .collect();
                n.children(block_nodes)
            }

            Block::NoIndent { .. } => Node::new("noindent"),
        }
    }

    fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
        inlines.iter().map(inline_to_node).collect()
    }

    fn inline_to_node(inline: &Inline) -> Node {
        match inline {
            Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Strong(children, _) => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new(node::STRONG).children(inline_nodes)
            }

            Inline::Emphasis(children, _) => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new(node::EMPHASIS).children(inline_nodes)
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Link { url, children, .. } => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(inline_nodes)
            }

            Inline::Superscript(children, _) => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new(node::SUPERSCRIPT).children(inline_nodes)
            }

            Inline::Subscript(children, _) => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new(node::SUBSCRIPT).children(inline_nodes)
            }

            Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

            Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),

            Inline::FootnoteDef { content, .. } => {
                let inline_nodes = inlines_to_nodes(content);
                Node::new(node::FOOTNOTE_DEF).children(inline_nodes)
            }

            // Texinfo semantic inlines: format-specific node kinds
            Inline::Var(children, _) => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new("var").children(inline_nodes)
            }

            Inline::File(s, _) => Node::new("file").prop(prop::CONTENT, s.clone()),

            Inline::Command(s, _) => Node::new("command").prop(prop::CONTENT, s.clone()),

            Inline::Option(s, _) => Node::new("option").prop(prop::CONTENT, s.clone()),

            Inline::Env(s, _) => Node::new("env").prop(prop::CONTENT, s.clone()),

            Inline::Samp(s, _) => Node::new("samp").prop(prop::CONTENT, s.clone()),

            Inline::Kbd(s, _) => Node::new("kbd").prop(prop::CONTENT, s.clone()),

            Inline::Key(s, _) => Node::new("key").prop(prop::CONTENT, s.clone()),

            Inline::Dfn(children, _) => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new("dfn").children(inline_nodes)
            }

            Inline::Cite(s, _) => Node::new("cite").prop(prop::CONTENT, s.clone()),

            Inline::Acronym {
                abbrev, expansion, ..
            } => {
                let mut n = Node::new("acronym").prop("texinfo:abbrev", abbrev.clone());
                if let Some(exp) = expansion {
                    n = n.prop("texinfo:expansion", exp.clone());
                }
                n.prop(prop::CONTENT, abbrev.clone())
            }

            Inline::Abbr {
                abbrev, expansion, ..
            } => {
                let mut n = Node::new("abbr").prop("texinfo:abbrev", abbrev.clone());
                if let Some(exp) = expansion {
                    n = n.prop("texinfo:expansion", exp.clone());
                }
                n.prop(prop::CONTENT, abbrev.clone())
            }

            Inline::Roman(s, _) => {
                Node::new("roman").child(Node::new(node::TEXT).prop(prop::CONTENT, s.clone()))
            }

            Inline::SmallCaps(s, _) => {
                Node::new("small_caps").child(Node::new(node::TEXT).prop(prop::CONTENT, s.clone()))
            }

            Inline::DirectItalic(children, _) => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new("direct_italic").children(inline_nodes)
            }

            Inline::DirectBold(children, _) => {
                let inline_nodes = inlines_to_nodes(children);
                Node::new("direct_bold").children(inline_nodes)
            }

            Inline::DirectTypewriter(s, _) => {
                Node::new("direct_typewriter").prop(prop::CONTENT, s.clone())
            }

            Inline::Image { file, alt, .. } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, file.clone());
                if let Some(a) = alt {
                    n = n.prop(prop::ALT, a.clone());
                }
                n
            }

            Inline::CrossRef {
                node: ref_node,
                text,
                ..
            } => {
                let mut n = Node::new("cross_ref").prop("texinfo:node", ref_node.clone());
                if let Some(t) = text {
                    n = n.prop(prop::CONTENT, t.clone());
                }
                n
            }

            Inline::Anchor { name, .. } => Node::new("anchor").prop(prop::ID, name.clone()),

            Inline::NoBreak(s, _) => Node::new("no_break").prop(prop::CONTENT, s.clone()),

            Inline::Email { address, text, .. } => {
                let mut n = Node::new("email").prop("texinfo:address", address.clone());
                if let Some(t) = text {
                    n = n.prop(prop::CONTENT, t.clone());
                }
                n
            }

            Inline::Symbol(kind, _) => {
                use crate::SymbolKind;
                let sym = match kind {
                    SymbolKind::Dots => "dots",
                    SymbolKind::EndDots => "enddots",
                    SymbolKind::Minus => "minus",
                    SymbolKind::Copyright => "copyright",
                    SymbolKind::Registered => "registered",
                    SymbolKind::LaTeX => "latex",
                    SymbolKind::TeX => "tex",
                    SymbolKind::Tie => "tie",
                };
                Node::new("symbol").prop("symbol", sym.to_string())
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_simple() {
            let input = r#"@chapter Introduction
This is the introduction paragraph.

@section Getting Started
Here is how to get started."#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_headings() {
            let input = r#"@chapter Chapter One
@section Section One
@subsection Subsection One
@subsubsection Sub-subsection"#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert_eq!(doc.content.children.len(), 4);
        }

        #[test]
        fn test_parse_emphasis() {
            let input = r#"This is @emph{emphasized} and @strong{bold} text."#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_code() {
            let input = r#"Use @code{printf} to print."#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_list() {
            let input = r#"@itemize
@item First item
@item Second item
@end itemize"#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
            assert_eq!(doc.content.children[0].kind.as_str(), node::LIST);
        }

        #[test]
        fn test_parse_enumerate() {
            let input = r#"@enumerate
@item First
@item Second
@end enumerate"#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
            let list = &doc.content.children[0];
            assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
        }

        #[test]
        fn test_parse_example() {
            let input = r#"@example
int main() {
    return 0;
}
@end example"#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
            assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
        }

        #[test]
        fn test_parse_url() {
            let input = r#"Visit @uref{https://example.com, Example Site}."#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_quotation() {
            let input = r#"@quotation
This is a quoted passage.
@end quotation"#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
            assert_eq!(doc.content.children[0].kind.as_str(), node::BLOCKQUOTE);
        }

        #[test]
        fn test_skip_comments() {
            let input = r#"@c This is a comment
This is visible.
@comment Another comment
Still visible."#;

            let result = parse(input).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{Block, CodeBlockVariant, HeadingKind, Inline, Span, TexinfoDoc};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_std::{node, prop};

    /// Emit a document to Texinfo format.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document to Texinfo format with options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut blocks = Vec::new();

        for child in &doc.content.children {
            if let Some(block) = node_to_block(child) {
                blocks.push(block);
            }
        }

        let title = doc.metadata.get_str("title").map(|s| s.to_string());

        let texinfo_doc = TexinfoDoc {
            title,
            blocks,
            span: Span::NONE,
        };
        let output = crate::emit::emit(&texinfo_doc);

        Ok(ConversionResult::ok(output.into_bytes()))
    }

    fn node_to_block(node: &Node) -> Option<Block> {
        match node.kind.as_str() {
            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                let inlines = node.children.iter().map(node_to_inline).collect();
                Some(Block::Heading {
                    level,
                    kind: HeadingKind::Numbered,
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
                    variant: CodeBlockVariant::Example,
                    content,
                    span: Span::NONE,
                })
            }

            node::BLOCKQUOTE => {
                let children = node.children.iter().filter_map(node_to_block).collect();
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
                    .filter_map(|child| {
                        if child.kind.as_str() == node::LIST_ITEM {
                            // Extract inlines from the list item
                            // If the item contains paragraphs, extract inlines from those
                            let inlines = if child.children.len() == 1
                                && child.children[0].kind.as_str() == node::PARAGRAPH
                            {
                                child.children[0]
                                    .children
                                    .iter()
                                    .map(node_to_inline)
                                    .collect()
                            } else {
                                child.children.iter().map(node_to_inline).collect()
                            };
                            Some(inlines)
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

            node::DEFINITION_LIST => {
                let mut items = Vec::new();
                let mut i = 0;
                while i < node.children.len() {
                    let child = &node.children[i];
                    if child.kind.as_str() == node::DEFINITION_TERM {
                        let term = child.children.iter().map(node_to_inline).collect();
                        let mut desc_blocks = Vec::new();

                        if i + 1 < node.children.len() {
                            let next = &node.children[i + 1];
                            if next.kind.as_str() == node::DEFINITION_DESC {
                                desc_blocks =
                                    next.children.iter().filter_map(node_to_block).collect();
                                i += 1;
                            }
                        }

                        items.push((term, desc_blocks));
                    }
                    i += 1;
                }
                Some(Block::DefinitionList {
                    items,
                    span: Span::NONE,
                })
            }

            node::HORIZONTAL_RULE => Some(Block::HorizontalRule { span: Span::NONE }),

            node::DOCUMENT => {
                let children: Vec<_> = node.children.iter().filter_map(node_to_block).collect();
                if children.len() == 1 {
                    children.into_iter().next()
                } else {
                    None
                }
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
                Inline::Strong(children, Span::NONE)
            }

            node::EMPHASIS => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Emphasis(children, Span::NONE)
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

            node::SUPERSCRIPT => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Superscript(children, Span::NONE)
            }

            node::SUBSCRIPT => {
                let children = node.children.iter().map(node_to_inline).collect();
                Inline::Subscript(children, Span::NONE)
            }

            node::LINE_BREAK => Inline::LineBreak { span: Span::NONE },

            node::SOFT_BREAK => Inline::SoftBreak { span: Span::NONE },

            node::FOOTNOTE_DEF => {
                let content = node.children.iter().map(node_to_inline).collect();
                Inline::FootnoteDef {
                    content,
                    span: Span::NONE,
                }
            }

            _ => Inline::Text(String::new(), Span::NONE),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_core::Properties;
        use rescribe_std::builder::*;

        fn emit_str(doc: &Document) -> String {
            let result = emit(doc).unwrap();
            String::from_utf8(result.value).unwrap()
        }

        #[test]
        fn test_emit_empty() {
            let doc = Document {
                content: Node::new(node::DOCUMENT),
                resources: Default::default(),
                metadata: Properties::new(),
                source: None,
            };

            let output = emit_str(&doc);
            assert!(output.contains("\\input texinfo"));
            assert!(output.contains("@bye"));
        }

        #[test]
        fn test_emit_with_title() {
            let mut metadata = Properties::new();
            metadata.set("title", "Test Document".to_string());

            let doc = Document {
                content: Node::new(node::DOCUMENT),
                resources: Default::default(),
                metadata,
                source: None,
            };

            let output = emit_str(&doc);
            assert!(output.contains("@settitle Test Document"));
        }

        #[test]
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Chapter Title")));
            let output = emit_str(&doc);
            assert!(output.contains("@chapter Chapter Title"));
        }

        #[test]
        fn test_emit_section() {
            let doc = doc(|d| d.heading(2, |h| h.text("Section Title")));
            let output = emit_str(&doc);
            assert!(output.contains("@section Section Title"));
        }

        #[test]
        fn test_emit_paragraph() {
            let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
            let output = emit_str(&doc);
            assert!(output.contains("Hello, world!"));
        }

        #[test]
        fn test_emit_emphasis() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("@emph{italic}"));
        }

        #[test]
        fn test_emit_strong() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            let output = emit_str(&doc);
            assert!(output.contains("@strong{bold}"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("printf")));
            let output = emit_str(&doc);
            assert!(output.contains("@code{printf}"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("Example"))));
            let output = emit_str(&doc);
            assert!(output.contains("@uref{https://example.com, Example}"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("@itemize @bullet"));
            assert!(output.contains("@item one"));
            assert!(output.contains("@item two"));
            assert!(output.contains("@end itemize"));
        }

        #[test]
        fn test_emit_enumerate() {
            let doc =
                doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
            let output = emit_str(&doc);
            assert!(output.contains("@enumerate"));
            assert!(output.contains("@item first"));
            assert!(output.contains("@end enumerate"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block("int main() {}"));
            let output = emit_str(&doc);
            assert!(output.contains("@example"));
            assert!(output.contains("int main() {}"));
            assert!(output.contains("@end example"));
        }

        #[test]
        fn test_emit_blockquote() {
            let doc = doc(|d| d.blockquote(|b| b.para(|p| p.text("Quoted text"))));
            let output = emit_str(&doc);
            assert!(output.contains("@quotation"));
            assert!(output.contains("Quoted text"));
            assert!(output.contains("@end quotation"));
        }

        #[test]
        fn test_escape_special_chars() {
            let doc = doc(|d| d.para(|p| p.text("Use @{braces}")));
            let output = emit_str(&doc);
            // @ -> @@, { -> @{, } -> @}
            assert!(output.contains("Use @@@{braces@}"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
