//! AST↔`rescribe::Document` translation, gated behind the `rescribe` feature.
//!
//! This module only ever calls into `crate::parse`/`RstDoc::emit` — it never
//! tokenizes, parses, or emits RST bytes itself. See CLAUDE.md's "The
//! `rescribe` feature module must never contain parsing or writing logic".

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub mod read {
    use crate::{Block, DefinitionItem, Inline, RstDoc, TableRow};
    use rescribe_core::{
        ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Severity,
        WarningKind,
    };
    use rescribe_std::{Node, node, prop};

    /// Parse RST text into a rescribe Document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse RST with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let rst = crate::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;
        let (children, warnings) = doc_to_nodes(&rst);
        let root = Node::new(node::DOCUMENT).children(children);
        let doc = Document::new().with_content(root);
        Ok(ConversionResult::with_warnings(doc, warnings))
    }

    fn doc_to_nodes(rst: &RstDoc) -> (Vec<Node>, Vec<FidelityWarning>) {
        let mut warnings = Vec::new();
        let nodes = rst
            .blocks
            .iter()
            .map(|b| block_to_node(b, &mut warnings))
            .collect();
        (nodes, warnings)
    }

    fn block_to_node(block: &Block, warnings: &mut Vec<FidelityWarning>) -> Node {
        match block {
            Block::Paragraph { inlines } => {
                Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines, warnings))
            }

            Block::Heading { level, inlines } => Node::new(node::HEADING)
                .prop(prop::LEVEL, *level)
                .children(inlines_to_nodes(inlines, warnings)),

            Block::CodeBlock { language, content } => {
                let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.to_string());
                if let Some(lang) = language {
                    n = n.prop(prop::LANGUAGE, lang.to_string());
                }
                n
            }

            Block::Blockquote { children } => {
                let child_nodes: Vec<Node> = children
                    .iter()
                    .map(|b| block_to_node(b, warnings))
                    .collect();
                Node::new(node::BLOCKQUOTE).children(child_nodes)
            }

            Block::List { ordered, items } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_blocks| {
                        let child_nodes: Vec<Node> = item_blocks
                            .iter()
                            .map(|b| block_to_node(b, warnings))
                            .collect();
                        Node::new(node::LIST_ITEM).children(child_nodes)
                    })
                    .collect();
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            Block::DefinitionList { items } => {
                let children = def_items_to_nodes(items, warnings);
                Node::new(node::DEFINITION_LIST).children(children)
            }

            Block::Figure { url, alt, caption } => {
                let mut img = Node::new(node::IMAGE).prop(prop::URL, url.to_string());
                if let Some(alt_text) = alt {
                    img = img.prop(prop::ALT, alt_text.to_string());
                }
                let mut figure_children = vec![img];
                if let Some(cap_inlines) = caption {
                    let cap_nodes = inlines_to_nodes(cap_inlines, warnings);
                    figure_children.push(Node::new(node::CAPTION).children(cap_nodes));
                }
                Node::new(node::FIGURE).children(figure_children)
            }

            Block::Image { url, alt, title } => {
                let mut n = Node::new(node::IMAGE).prop(prop::URL, url.to_string());
                if let Some(alt_text) = alt {
                    n = n.prop(prop::ALT, alt_text.to_string());
                }
                if let Some(title_text) = title {
                    n = n.prop(prop::TITLE, title_text.to_string());
                }
                n
            }

            Block::RawBlock { format, content } => Node::new(node::RAW_BLOCK)
                .prop(prop::CONTENT, content.to_string())
                .prop("format", format.to_string()),

            Block::Div {
                class,
                directive,
                children,
            } => {
                if let Some(dir_name) = directive {
                    warnings.push(FidelityWarning::new(
                        Severity::Minor,
                        WarningKind::UnsupportedNode(format!("rst:{}", dir_name)),
                        format!("Unknown directive: {}", dir_name),
                    ));
                }
                let child_nodes: Vec<Node> = children
                    .iter()
                    .map(|b| block_to_node(b, warnings))
                    .collect();
                let mut n = Node::new(node::DIV).children(child_nodes);
                if let Some(cls) = class {
                    n = n.prop("class", cls.to_string());
                }
                if let Some(dir_name) = directive {
                    n = n.prop("rst:directive", dir_name.to_string());
                }
                n
            }

            Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),

            Block::Table { rows } => {
                let row_nodes: Vec<Node> = rows
                    .iter()
                    .map(|r| table_row_to_node(r, warnings))
                    .collect();
                Node::new(node::TABLE).children(row_nodes)
            }

            Block::FootnoteDef { label, inlines } => {
                let child_nodes = inlines_to_nodes(inlines, warnings);
                Node::new(node::FOOTNOTE_DEF)
                    .prop(prop::LABEL, label.to_string())
                    .children(child_nodes)
            }

            Block::MathDisplay { source } => {
                Node::new("math_display").prop("math:source", source.to_string())
            }

            Block::Admonition {
                admonition_type,
                children,
            } => {
                let child_nodes: Vec<Node> = children
                    .iter()
                    .map(|b| block_to_node(b, warnings))
                    .collect();
                Node::new("admonition")
                    .prop("admonition_type", admonition_type.to_string())
                    .children(child_nodes)
            }
        }
    }

    fn def_items_to_nodes(
        items: &[DefinitionItem],
        warnings: &mut Vec<FidelityWarning>,
    ) -> Vec<Node> {
        let mut nodes = Vec::new();
        for item in items {
            let term_nodes = inlines_to_nodes(&item.term, warnings);
            let desc_nodes = inlines_to_nodes(&item.desc, warnings);
            nodes.push(Node::new(node::DEFINITION_TERM).children(term_nodes));
            nodes.push(Node::new(node::DEFINITION_DESC).children(desc_nodes));
        }
        nodes
    }

    fn table_row_to_node(row: &TableRow, warnings: &mut Vec<FidelityWarning>) -> Node {
        let cells: Vec<Node> = row
            .cells
            .iter()
            .map(|cell| Node::new(node::TABLE_CELL).children(inlines_to_nodes(cell, warnings)))
            .collect();
        let row_kind = if row.is_header {
            node::TABLE_HEADER
        } else {
            node::TABLE_ROW
        };
        Node::new(row_kind).children(cells)
    }

    fn inlines_to_nodes(inlines: &[Inline], warnings: &mut Vec<FidelityWarning>) -> Vec<Node> {
        inlines
            .iter()
            .map(|i| inline_to_node(i, warnings))
            .collect()
    }

    fn inline_to_node(inline: &Inline, warnings: &mut Vec<FidelityWarning>) -> Node {
        match inline {
            Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.to_string()),

            Inline::Emphasis(children) => {
                Node::new(node::EMPHASIS).children(inlines_to_nodes(children, warnings))
            }

            Inline::Strong(children) => {
                Node::new(node::STRONG).children(inlines_to_nodes(children, warnings))
            }

            Inline::Strikeout(children) => {
                Node::new(node::STRIKEOUT).children(inlines_to_nodes(children, warnings))
            }

            Inline::Underline(children) => {
                Node::new(node::UNDERLINE).children(inlines_to_nodes(children, warnings))
            }

            Inline::Subscript(children) => {
                Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children, warnings))
            }

            Inline::Superscript(children) => {
                Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children, warnings))
            }

            Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.to_string()),

            Inline::Link { url, children } => Node::new(node::LINK)
                .prop(prop::URL, url.to_string())
                .children(inlines_to_nodes(children, warnings)),

            Inline::Image { url, alt } => Node::new(node::IMAGE)
                .prop(prop::URL, url.to_string())
                .prop(prop::ALT, alt.to_string()),

            Inline::LineBreak => Node::new(node::LINE_BREAK),

            Inline::SoftBreak => Node::new(node::SOFT_BREAK),

            Inline::FootnoteRef { label } => {
                Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.to_string())
            }

            Inline::FootnoteDef { label, children } => Node::new(node::FOOTNOTE_DEF)
                .prop(prop::LABEL, label.to_string())
                .children(inlines_to_nodes(children, warnings)),

            Inline::SmallCaps(children) => {
                Node::new(node::SMALL_CAPS).children(inlines_to_nodes(children, warnings))
            }

            Inline::Quoted {
                quote_type,
                children,
            } => Node::new(node::QUOTED)
                .prop(prop::QUOTE_TYPE, quote_type.to_string())
                .children(inlines_to_nodes(children, warnings)),

            Inline::MathInline { source } => {
                Node::new("math_inline").prop("math:source", source.to_string())
            }

            Inline::RstSpan { role, children } => {
                let child_nodes = inlines_to_nodes(children, warnings);
                match role.as_ref() {
                    "small-caps" | "sc" => Node::new(node::SMALL_CAPS).children(child_nodes),
                    "strike" | "del" | "s" => Node::new(node::STRIKEOUT).children(child_nodes),
                    "underline" | "u" => Node::new(node::UNDERLINE).children(child_nodes),
                    _ => Node::new(node::SPAN)
                        .prop("rst:role", role.to_string())
                        .children(child_nodes),
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_core::Document;

        fn root_children(doc: &Document) -> &[Node] {
            &doc.content.children
        }

        #[test]
        fn test_parse_heading() {
            let input = "Hello World\n===========\n\nSome text.";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            assert_eq!(children.len(), 2);
            assert_eq!(children[0].kind.as_str(), node::HEADING);
            assert_eq!(children[0].props.get_int(prop::LEVEL), Some(1));
        }

        #[test]
        fn test_parse_paragraph() {
            let input = "This is a paragraph.\n\nThis is another.";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            assert_eq!(children.len(), 2);
            assert_eq!(children[0].kind.as_str(), node::PARAGRAPH);
            assert_eq!(children[1].kind.as_str(), node::PARAGRAPH);
        }

        #[test]
        fn test_parse_emphasis() {
            let input = "This is *emphasized* text.";
            let result = parse(input).unwrap();
            let doc = result.value;
            let para = &root_children(&doc)[0];

            assert!(
                para.children
                    .iter()
                    .any(|n| n.kind.as_str() == node::EMPHASIS)
            );
        }

        #[test]
        fn test_parse_strong() {
            let input = "This is **strong** text.";
            let result = parse(input).unwrap();
            let doc = result.value;
            let para = &root_children(&doc)[0];

            assert!(
                para.children
                    .iter()
                    .any(|n| n.kind.as_str() == node::STRONG)
            );
        }

        #[test]
        fn test_parse_bullet_list() {
            let input = "* First item\n* Second item\n* Third item";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            assert_eq!(children.len(), 1);
            assert_eq!(children[0].kind.as_str(), node::LIST);
            assert_eq!(children[0].props.get_bool(prop::ORDERED), Some(false));
            assert_eq!(children[0].children.len(), 3);
        }

        #[test]
        fn test_parse_numbered_list() {
            let input = "1. First item\n2. Second item\n3. Third item";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            assert_eq!(children.len(), 1);
            assert_eq!(children[0].kind.as_str(), node::LIST);
            assert_eq!(children[0].props.get_bool(prop::ORDERED), Some(true));
            assert_eq!(children[0].children.len(), 3);
        }

        #[test]
        fn test_parse_code_block() {
            let input = "Example::\n\n    def hello():\n        print('Hello')";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            // Should have paragraph and code block
            assert!(children.iter().any(|n| n.kind.as_str() == node::CODE_BLOCK));
        }

        #[test]
        fn test_parse_inline_code() {
            let input = "Use ``code here`` in text.";
            let result = parse(input).unwrap();
            let doc = result.value;
            let para = &root_children(&doc)[0];

            assert!(para.children.iter().any(|n| n.kind.as_str() == node::CODE));
        }

        #[test]
        fn test_parse_link() {
            let input = "Click `here <https://example.com>`_ for more.";
            let result = parse(input).unwrap();
            let doc = result.value;
            let para = &root_children(&doc)[0];

            let link = para.children.iter().find(|n| n.kind.as_str() == node::LINK);
            assert!(link.is_some());
            assert_eq!(
                link.unwrap().props.get_str(prop::URL),
                Some("https://example.com")
            );
        }

        #[test]
        fn test_parse_directive() {
            let input = ".. code-block:: python\n\n   print('hello')";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            assert_eq!(children.len(), 1);
            assert_eq!(children[0].kind.as_str(), node::CODE_BLOCK);
            assert_eq!(children[0].props.get_str(prop::LANGUAGE), Some("python"));
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub mod write {
    use crate::{Block, DefinitionItem, Inline, RstDoc, TableRow};
    use rescribe_core::{
        ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
        WarningKind,
    };
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};
    use std::borrow::Cow;

    /// Emit a document as RST.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as RST with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut warnings = Vec::new();
        let rst = doc_to_rst(doc, &mut warnings);
        let output = rst.emit();
        Ok(ConversionResult::with_warnings(output, warnings))
    }

    fn doc_to_rst<'d>(doc: &'d Document, warnings: &mut Vec<FidelityWarning>) -> RstDoc<'d> {
        RstDoc {
            blocks: nodes_to_blocks(&doc.content.children, warnings),
        }
    }

    fn nodes_to_blocks<'d>(
        nodes: &'d [Node],
        warnings: &mut Vec<FidelityWarning>,
    ) -> Vec<Block<'d>> {
        nodes
            .iter()
            .flat_map(|n| node_to_blocks(n, warnings))
            .collect()
    }

    fn node_to_blocks<'d>(node: &'d Node, warnings: &mut Vec<FidelityWarning>) -> Vec<Block<'d>> {
        match node.kind.as_str() {
            node::DOCUMENT => nodes_to_blocks(&node.children, warnings),

            node::PARAGRAPH => vec![Block::Paragraph {
                inlines: nodes_to_inlines(&node.children, warnings),
            }],

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1);
                vec![Block::Heading {
                    level,
                    inlines: nodes_to_inlines(&node.children, warnings),
                }]
            }

            node::CODE_BLOCK => {
                let language = node.props.get_str(prop::LANGUAGE).map(Cow::Borrowed);
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").into();
                vec![Block::CodeBlock { language, content }]
            }

            node::BLOCKQUOTE => vec![Block::Blockquote {
                children: nodes_to_blocks(&node.children, warnings),
            }],

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<Block<'d>>> = node
                    .children
                    .iter()
                    .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                    .map(|item| nodes_to_blocks(&item.children, warnings))
                    .collect();
                vec![Block::List { ordered, items }]
            }

            node::LIST_ITEM => nodes_to_blocks(&node.children, warnings),

            node::DEFINITION_LIST => {
                let items = definition_list_to_items(&node.children, warnings);
                vec![Block::DefinitionList { items }]
            }

            node::FIGURE => {
                let img = node
                    .children
                    .iter()
                    .find(|c| c.kind.as_str() == node::IMAGE);
                if let Some(img_node) = img {
                    let url = img_node.props.get_str(prop::URL).unwrap_or("").into();
                    let alt = img_node.props.get_str(prop::ALT).map(Cow::Borrowed);
                    let caption_node = node
                        .children
                        .iter()
                        .find(|c| c.kind.as_str() == node::CAPTION);
                    let caption = caption_node.map(|cap| nodes_to_inlines(&cap.children, warnings));
                    vec![Block::Figure { url, alt, caption }]
                } else {
                    vec![]
                }
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").into();
                let alt = node.props.get_str(prop::ALT).map(Cow::Borrowed);
                let title = node.props.get_str(prop::TITLE).map(Cow::Borrowed);
                vec![Block::Image { url, alt, title }]
            }

            node::TABLE => {
                let rows = collect_table_rows(node, warnings);
                vec![Block::Table { rows }]
            }

            node::HORIZONTAL_RULE => vec![Block::HorizontalRule],

            node::DIV | node::SPAN => nodes_to_blocks(&node.children, warnings),

            node::RAW_BLOCK | node::RAW_INLINE => {
                let format = node.props.get_str(prop::FORMAT).unwrap_or("").into();
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").into();
                vec![Block::RawBlock { format, content }]
            }

            node::DEFINITION_TERM | node::DEFINITION_DESC => {
                // These are handled inside DEFINITION_LIST
                vec![]
            }

            node::FOOTNOTE_DEF => {
                let label = node.props.get_str(prop::LABEL).unwrap_or("").into();
                let inlines = nodes_to_inlines(&node.children, warnings);
                vec![Block::FootnoteDef { label, inlines }]
            }

            "math_display" => {
                let source = node.props.get_str("math:source").unwrap_or("").into();
                vec![Block::MathDisplay { source }]
            }

            "admonition" => {
                let admonition_type = node
                    .props
                    .get_str("admonition_type")
                    .unwrap_or("note")
                    .to_lowercase()
                    .into();
                let children = nodes_to_blocks(&node.children, warnings);
                vec![Block::Admonition {
                    admonition_type,
                    children,
                }]
            }

            // Inline nodes at block level: wrap in a paragraph
            node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK => {
                vec![Block::Paragraph {
                    inlines: nodes_to_inlines(std::slice::from_ref(node), warnings),
                }]
            }

            _ => {
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                    format!("Unknown node type for RST: {}", node.kind.as_str()),
                ));
                nodes_to_blocks(&node.children, warnings)
            }
        }
    }

    fn definition_list_to_items<'d>(
        nodes: &'d [Node],
        warnings: &mut Vec<FidelityWarning>,
    ) -> Vec<DefinitionItem<'d>> {
        let mut items = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            if nodes[i].kind.as_str() == node::DEFINITION_TERM {
                let term = nodes_to_inlines(&nodes[i].children, warnings);
                let desc =
                    if i + 1 < nodes.len() && nodes[i + 1].kind.as_str() == node::DEFINITION_DESC {
                        let d = nodes_to_inlines(&nodes[i + 1].children, warnings);
                        i += 1;
                        d
                    } else {
                        vec![]
                    };
                items.push(DefinitionItem { term, desc });
            }
            i += 1;
        }
        items
    }

    fn collect_table_rows<'d>(
        node: &'d Node,
        warnings: &mut Vec<FidelityWarning>,
    ) -> Vec<TableRow<'d>> {
        let mut rows = Vec::new();
        collect_table_rows_inner(&node.children, &mut rows, warnings);
        rows
    }

    fn collect_table_rows_inner<'d>(
        nodes: &'d [Node],
        rows: &mut Vec<TableRow<'d>>,
        warnings: &mut Vec<FidelityWarning>,
    ) {
        for n in nodes {
            match n.kind.as_str() {
                node::TABLE_HEAD | node::TABLE_BODY | node::TABLE_FOOT => {
                    collect_table_rows_inner(&n.children, rows, warnings);
                }
                node::TABLE_ROW => {
                    let cells: Vec<Vec<Inline<'d>>> = n
                        .children
                        .iter()
                        .map(|cell| nodes_to_inlines(&cell.children, warnings))
                        .collect();
                    rows.push(TableRow {
                        cells,
                        is_header: false,
                    });
                }
                node::TABLE_HEADER => {
                    let cells: Vec<Vec<Inline<'d>>> = n
                        .children
                        .iter()
                        .map(|cell| nodes_to_inlines(&cell.children, warnings))
                        .collect();
                    rows.push(TableRow {
                        cells,
                        is_header: true,
                    });
                }
                _ => {}
            }
        }
    }

    fn nodes_to_inlines<'d>(
        nodes: &'d [Node],
        warnings: &mut Vec<FidelityWarning>,
    ) -> Vec<Inline<'d>> {
        nodes.iter().map(|n| node_to_inline(n, warnings)).collect()
    }

    fn node_to_inline<'d>(node: &'d Node, warnings: &mut Vec<FidelityWarning>) -> Inline<'d> {
        match node.kind.as_str() {
            node::TEXT => {
                let s = node.props.get_str(prop::CONTENT).unwrap_or("").into();
                Inline::Text(s)
            }

            node::EMPHASIS => Inline::Emphasis(nodes_to_inlines(&node.children, warnings)),

            node::STRONG => Inline::Strong(nodes_to_inlines(&node.children, warnings)),

            node::STRIKEOUT => Inline::Strikeout(nodes_to_inlines(&node.children, warnings)),

            node::UNDERLINE => Inline::Underline(nodes_to_inlines(&node.children, warnings)),

            node::SUBSCRIPT => Inline::Subscript(nodes_to_inlines(&node.children, warnings)),

            node::SUPERSCRIPT => Inline::Superscript(nodes_to_inlines(&node.children, warnings)),

            node::CODE => {
                let s = node.props.get_str(prop::CONTENT).unwrap_or("").into();
                Inline::Code(s)
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").into();
                Inline::Link {
                    url,
                    children: nodes_to_inlines(&node.children, warnings),
                }
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").into();
                let alt = node.props.get_str(prop::ALT).unwrap_or("").into();
                Inline::Image { url, alt }
            }

            node::LINE_BREAK => Inline::LineBreak,

            node::SOFT_BREAK => Inline::SoftBreak,

            node::FOOTNOTE_REF => {
                let label = node.props.get_str(prop::LABEL).unwrap_or("").into();
                Inline::FootnoteRef { label }
            }

            node::FOOTNOTE_DEF => {
                let label = node.props.get_str(prop::LABEL).unwrap_or("").into();
                Inline::FootnoteDef {
                    label,
                    children: nodes_to_inlines(&node.children, warnings),
                }
            }

            node::SMALL_CAPS => Inline::SmallCaps(nodes_to_inlines(&node.children, warnings)),

            node::QUOTED => {
                let quote_type = node
                    .props
                    .get_str(prop::QUOTE_TYPE)
                    .unwrap_or("double")
                    .into();
                Inline::Quoted {
                    quote_type,
                    children: nodes_to_inlines(&node.children, warnings),
                }
            }

            node::SPAN => {
                let role = node.props.get_str("rst:role").unwrap_or("span").into();
                Inline::RstSpan {
                    role,
                    children: nodes_to_inlines(&node.children, warnings),
                }
            }

            node::RAW_INLINE => {
                let format = node.props.get_str(prop::FORMAT).unwrap_or("");
                let content = node.props.get_str(prop::CONTENT).unwrap_or("");
                if format == "rst" {
                    Inline::Text(content.into())
                } else {
                    Inline::Text(Cow::Borrowed(""))
                }
            }

            "math_inline" => {
                let source = node.props.get_str("math:source").unwrap_or("").into();
                Inline::MathInline { source }
            }

            _ => {
                // Unknown inline: recurse into children
                let children = nodes_to_inlines(&node.children, warnings);
                if children.is_empty() {
                    Inline::Text(Cow::Borrowed(""))
                } else if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    Inline::Strong(children)
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
        fn test_emit_paragraph() {
            let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
            let output = emit_str(&doc);
            assert!(output.contains("Hello, world!"));
        }

        #[test]
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Title")));
            let output = emit_str(&doc);
            assert!(output.contains("====="));
            assert!(output.contains("Title"));
        }

        #[test]
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
            let output = emit_str(&doc);
            assert!(output.contains("--------"));
            assert!(output.contains("Subtitle"));
        }

        #[test]
        fn test_emit_emphasis() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("*italic*"));
        }

        #[test]
        fn test_emit_strong() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            let output = emit_str(&doc);
            assert!(output.contains("**bold**"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("``code``"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("`click <https://example.com>`_"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
            let output = emit_str(&doc);
            assert!(output.contains(".. code-block:: python"));
            assert!(output.contains("   print('hi')"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("- one"));
            assert!(output.contains("- two"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
