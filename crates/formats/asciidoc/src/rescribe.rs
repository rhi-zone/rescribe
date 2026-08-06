//! AST↔`rescribe::Document` translation for AsciiDoc.
//!
//! This module only translates between [`AsciiDoc`](crate::AsciiDoc) and
//! rescribe's `Document` IR — no AsciiDoc tokenizing/parsing/emitting
//! happens here (that all lives in the rest of this crate; see
//! `crate::parse_str` and the `Emit` trait impl on `AsciiDoc`). Enabled by
//! the `rescribe` feature; each direction is additionally gated on the
//! reader/writer mode feature it depends on, so enabling `rescribe` alone
//! (with no mode feature) compiles nothing.
//!
//! # Mapping
//!
//! Blocks and inlines map onto rescribe's standard node kinds
//! (`paragraph`, `heading`, `code_block`, `blockquote`, `list`/`list_item`,
//! `definition_list`/`definition_term`/`definition_desc`, `table`,
//! `horizontal_rule`, `figure`/`image`, `div`, `raw_block`/`raw_inline`,
//! `strong`, `emphasis`, `code`, `superscript`, `subscript`, `strikeout`,
//! `underline`, `small_caps`, `quoted`, `link`, `line_break`, `soft_break`,
//! `footnote_ref`, `footnote_def`). AsciiDoc-specific metadata with no
//! direct IR equivalent is raw-preserved as plain string properties (`id`,
//! `role`, `attribution`, `class`, `title`) rather than namespaced, matching
//! the original adapter's behavior. Math blocks/inlines use the shared
//! `math_block`/`math_inline` node kinds with a `math:source`/`math:flavor`
//! property pair.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{AsciiDoc, Block, DefinitionItem, ImageData, Inline, QuoteType, TableRow};
    use rescribe_core::{ConversionResult, Document, ParseOptions};
    use rescribe_std::{Node, node, prop};

    /// Parse AsciiDoc text into a rescribe Document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, rescribe_core::ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse AsciiDoc with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, rescribe_core::ParseError> {
        let (ast, _diagnostics) = crate::parse_str(input);
        let children = doc_to_nodes(&ast);
        let root = Node::new(node::DOCUMENT).children(children);
        let doc = Document::new().with_content(root);
        Ok(ConversionResult::ok(doc))
    }

    fn doc_to_nodes(ast: &AsciiDoc) -> Vec<Node> {
        ast.blocks.iter().map(block_to_node).collect()
    }

    fn block_to_node(block: &Block) -> Node {
        match block {
            Block::Paragraph {
                inlines, id, role, ..
            } => {
                let mut n = Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines));
                if let Some(id) = id {
                    n = n.prop("id", id.clone());
                }
                if let Some(role) = role {
                    n = n.prop("role", role.clone());
                }
                n
            }

            Block::Heading {
                level,
                inlines,
                id,
                role,
                ..
            } => {
                let mut n = Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(inlines_to_nodes(inlines));
                if let Some(id) = id {
                    n = n.prop("id", id.clone());
                }
                if let Some(role) = role {
                    n = n.prop("role", role.clone());
                }
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

            Block::Blockquote {
                children,
                attribution,
                ..
            } => {
                let mut n =
                    Node::new(node::BLOCKQUOTE).children(children.iter().map(block_to_node));
                if let Some(attr) = attribution {
                    n = n.prop("attribution", attr.clone());
                }
                n
            }

            Block::List {
                ordered,
                items,
                style,
                ..
            } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_blocks| {
                        let mut li = Node::new(node::LIST_ITEM)
                            .children(item_blocks.iter().map(block_to_node));
                        // Propagate checklist state from first paragraph to the list_item
                        if let Some(Block::Paragraph {
                            checked: Some(c), ..
                        }) = item_blocks.first()
                        {
                            li = li.prop("asciidoc:checked", *c);
                        }
                        li
                    })
                    .collect();
                let mut n = Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items);
                if let Some(s) = style {
                    n = n.prop("list:style", s.clone());
                }
                n
            }

            Block::DefinitionList { items, .. } => {
                let children: Vec<Node> = items.iter().flat_map(definition_item_to_nodes).collect();
                Node::new(node::DEFINITION_LIST).children(children)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

            Block::PageBreak { .. } => Node::new(node::DIV).prop("class", "page-break".to_string()),

            Block::Figure { image, .. } => {
                let img = image_data_to_node(image);
                Node::new(node::FIGURE).children(vec![img])
            }

            Block::Div {
                class,
                title,
                children,
                ..
            } => {
                let mut n = Node::new(node::DIV).children(children.iter().map(block_to_node));
                if let Some(cls) = class {
                    n = n.prop("class", cls.clone());
                }
                if let Some(t) = title {
                    n = n.prop("title", t.clone());
                }
                n
            }

            Block::RawBlock {
                format, content, ..
            } => Node::new(node::RAW_BLOCK)
                .prop(prop::CONTENT, content.clone())
                .prop("format", format.clone()),

            Block::MathBlock {
                content, flavor, ..
            } => {
                let mut n = Node::new("math_block").prop("math:source", content.clone());
                if let Some(f) = flavor {
                    n = n.prop("math:flavor", f.clone());
                }
                n
            }

            Block::Table { rows, .. } => {
                let row_nodes: Vec<Node> = rows.iter().map(table_row_to_node).collect();
                Node::new(node::TABLE).children(row_nodes)
            }
        }
    }

    fn definition_item_to_nodes(item: &DefinitionItem) -> Vec<Node> {
        vec![
            Node::new(node::DEFINITION_TERM).children(inlines_to_nodes(&item.term)),
            Node::new(node::DEFINITION_DESC).children(inlines_to_nodes(&item.desc)),
        ]
    }

    fn table_row_to_node(row: &TableRow) -> Node {
        let cells: Vec<Node> = row
            .cells
            .iter()
            .map(|cell| Node::new(node::TABLE_CELL).children(inlines_to_nodes(cell)))
            .collect();
        if row.is_header {
            Node::new(node::TABLE_HEADER).children(cells)
        } else {
            Node::new(node::TABLE_ROW).children(cells)
        }
    }

    fn image_data_to_node(img: &ImageData) -> Node {
        let mut n = Node::new(node::IMAGE).prop(prop::URL, img.url.clone());
        if let Some(alt) = &img.alt {
            n = n.prop(prop::ALT, alt.clone());
        }
        if let Some(w) = &img.width {
            n = n.prop("width", w.clone());
        }
        if let Some(h) = &img.height {
            n = n.prop("height", h.clone());
        }
        n
    }

    fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
        inlines.iter().map(inline_to_node).collect()
    }

    fn inline_to_node(inline: &Inline) -> Node {
        match inline {
            Inline::Text { text: s, .. } => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

            Inline::Strong(children, _) => {
                Node::new(node::STRONG).children(inlines_to_nodes(children))
            }

            Inline::Emphasis(children, _) => {
                Node::new(node::EMPHASIS).children(inlines_to_nodes(children))
            }

            Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

            Inline::Superscript(children, _) => {
                Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::Subscript(children, _) => {
                Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::Highlight(children, _) => Node::new(node::SPAN)
                .prop("class", "highlight".to_string())
                .children(inlines_to_nodes(children)),

            Inline::Strikeout(children, _) => {
                Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
            }

            Inline::Underline(children, _) => {
                Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
            }

            Inline::SmallCaps(children, _) => {
                Node::new(node::SMALL_CAPS).children(inlines_to_nodes(children))
            }

            Inline::Quoted {
                quote_type,
                children,
                ..
            } => {
                let qt = match quote_type {
                    QuoteType::Single => "single",
                    QuoteType::Double => "double",
                };
                Node::new(node::QUOTED)
                    .prop(prop::QUOTE_TYPE, qt.to_string())
                    .children(inlines_to_nodes(children))
            }

            Inline::Link {
                url,
                children,
                target,
                ..
            } => {
                let mut n = Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(inlines_to_nodes(children));
                if let Some(t) = target {
                    n = n.prop("target", t.clone());
                }
                n
            }

            Inline::Image(img, _) => image_data_to_node(img),

            Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

            Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),

            Inline::FootnoteRef { label, .. } => {
                Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone())
            }

            Inline::FootnoteDef {
                label, children, ..
            } => Node::new(node::FOOTNOTE_DEF)
                .prop(prop::LABEL, label.clone())
                .children(inlines_to_nodes(children)),

            Inline::MathInline {
                content, flavor, ..
            } => {
                let mut n = Node::new("math_inline").prop("math:source", content.clone());
                if let Some(f) = flavor {
                    n = n.prop("math:flavor", f.clone());
                }
                n
            }

            Inline::RawInline {
                format, content, ..
            } => Node::new(node::RAW_INLINE)
                .prop("format", format.clone())
                .prop(prop::CONTENT, content.clone()),

            Inline::Anchor { id, .. } => Node::new(node::SPAN).prop("id", id.clone()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn root_children(doc: &Document) -> &[Node] {
            &doc.content.children
        }

        #[test]
        fn test_parse_heading() {
            let input = "== Hello World\n\nSome text.";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            assert_eq!(children.len(), 2);
            assert_eq!(children[0].kind.as_str(), node::HEADING);
            assert_eq!(children[0].props.get_int(prop::LEVEL), Some(2));
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
        fn test_parse_strong() {
            let input = "This is *strong* text.";
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
        fn test_parse_emphasis() {
            let input = "This is _emphasized_ text.";
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
            let input = ". First item\n. Second item\n. Third item";
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
            let input = "[source,python]\n----\nprint('hello')\n----";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            assert_eq!(children.len(), 1);
            assert_eq!(children[0].kind.as_str(), node::CODE_BLOCK);
            assert_eq!(children[0].props.get_str(prop::LANGUAGE), Some("python"));
        }

        #[test]
        fn test_parse_inline_code() {
            let input = "Use `code here` in text.";
            let result = parse(input).unwrap();
            let doc = result.value;
            let para = &root_children(&doc)[0];

            assert!(para.children.iter().any(|n| n.kind.as_str() == node::CODE));
        }

        #[test]
        fn test_parse_link() {
            let input = "Visit https://example.com[Example Site] for more.";
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
        fn test_parse_block_image() {
            let input = "image::path/to/image.png[Alt text]";
            let result = parse(input).unwrap();
            let doc = result.value;
            let children = root_children(&doc);

            assert_eq!(children.len(), 1);
            assert_eq!(children[0].kind.as_str(), node::FIGURE);

            let img = &children[0].children[0];
            assert_eq!(img.kind.as_str(), node::IMAGE);
            assert_eq!(img.props.get_str(prop::URL), Some("path/to/image.png"));
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{AsciiDoc, Block, DefinitionItem, ImageData, Inline, QuoteType, TableRow};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document as AsciiDoc.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as AsciiDoc with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let ast = doc_to_ast(doc);
        let output = ast.emit();
        Ok(ConversionResult::ok(output))
    }

    fn doc_to_ast(doc: &Document) -> AsciiDoc {
        AsciiDoc {
            blocks: nodes_to_blocks(&doc.content.children),
            attributes: Default::default(),
            span: crate::Span::NONE,
        }
    }

    fn nodes_to_blocks(nodes: &[Node]) -> Vec<Block> {
        nodes.iter().flat_map(node_to_blocks).collect()
    }

    fn node_to_blocks(node: &Node) -> Vec<Block> {
        match node.kind.as_str() {
            node::DOCUMENT => nodes_to_blocks(&node.children),

            node::PARAGRAPH => vec![Block::Paragraph {
                inlines: nodes_to_inlines(&node.children),
                id: None,
                role: None,
                checked: None,
                span: crate::Span::NONE,
            }],

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
                vec![Block::Heading {
                    level,
                    inlines: nodes_to_inlines(&node.children),
                    id: None,
                    role: None,
                    span: crate::Span::NONE,
                }]
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
                vec![Block::CodeBlock {
                    content,
                    language,
                    span: crate::Span::NONE,
                }]
            }

            node::BLOCKQUOTE => vec![Block::Blockquote {
                children: nodes_to_blocks(&node.children),
                attribution: None,
                span: crate::Span::NONE,
            }],

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<Block>> = node
                    .children
                    .iter()
                    .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                    .map(|item| nodes_to_blocks(&item.children))
                    .collect();
                vec![Block::List {
                    ordered,
                    items,
                    style: None,
                    span: crate::Span::NONE,
                }]
            }

            node::LIST_ITEM => nodes_to_blocks(&node.children),

            node::DEFINITION_LIST => {
                // Pair up DEFINITION_TERM and DEFINITION_DESC children
                let mut items = Vec::new();
                let mut i = 0;
                while i < node.children.len() {
                    let child = &node.children[i];
                    if child.kind.as_str() == node::DEFINITION_TERM {
                        let term = nodes_to_inlines(&child.children);
                        let desc = if i + 1 < node.children.len()
                            && node.children[i + 1].kind.as_str() == node::DEFINITION_DESC
                        {
                            i += 1;
                            nodes_to_inlines(&node.children[i].children)
                        } else {
                            Vec::new()
                        };
                        items.push(DefinitionItem { term, desc });
                    }
                    i += 1;
                }
                vec![Block::DefinitionList {
                    items,
                    span: crate::Span::NONE,
                }]
            }

            node::DEFINITION_TERM | node::DEFINITION_DESC => {
                // These are handled inside DEFINITION_LIST; skip if encountered alone
                vec![]
            }

            node::TABLE => {
                let rows: Vec<TableRow> =
                    node.children.iter().flat_map(collect_table_rows).collect();
                vec![Block::Table {
                    rows,
                    span: crate::Span::NONE,
                }]
            }

            node::FIGURE => {
                // Look for an IMAGE child
                for child in &node.children {
                    if child.kind.as_str() == node::IMAGE {
                        return vec![Block::Figure {
                            image: node_to_image_data(child),
                            span: crate::Span::NONE,
                        }];
                    }
                }
                nodes_to_blocks(&node.children)
            }

            node::HORIZONTAL_RULE => vec![Block::HorizontalRule {
                span: crate::Span::NONE,
            }],

            node::DIV | node::SPAN => nodes_to_blocks(&node.children),

            node::RAW_BLOCK | node::RAW_INLINE => {
                let format = node
                    .props
                    .get_str(prop::FORMAT)
                    .unwrap_or("asciidoc")
                    .to_string();
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                if format == "asciidoc" {
                    vec![Block::RawBlock {
                        format,
                        content,
                        span: crate::Span::NONE,
                    }]
                } else {
                    vec![]
                }
            }

            "math_block" | "math_display" => {
                if let Some(source) = node.props.get_str("math:source") {
                    let flavor = node.props.get_str("math:flavor").map(|s| s.to_string());
                    vec![Block::MathBlock {
                        content: source.to_string(),
                        flavor,
                        span: crate::Span::NONE,
                    }]
                } else {
                    vec![]
                }
            }

            "admonition" => {
                let adm_type = node
                    .props
                    .get_str("admonition_type")
                    .unwrap_or("NOTE")
                    .to_uppercase();
                vec![Block::Div {
                    class: Some(format!("admonition {}", adm_type.to_lowercase())),
                    title: None,
                    children: nodes_to_blocks(&node.children),
                    span: crate::Span::NONE,
                }]
            }

            // Inline nodes at block level: wrap in a paragraph
            node::TEXT
            | node::STRONG
            | node::EMPHASIS
            | node::CODE
            | node::LINK
            | node::STRIKEOUT
            | node::UNDERLINE
            | node::SUPERSCRIPT
            | node::SUBSCRIPT => {
                vec![Block::Paragraph {
                    inlines: nodes_to_inlines(std::slice::from_ref(node)),
                    id: None,
                    role: None,
                    checked: None,
                    span: crate::Span::NONE,
                }]
            }

            _ => nodes_to_blocks(&node.children),
        }
    }

    fn collect_table_rows(node: &Node) -> Vec<TableRow> {
        match node.kind.as_str() {
            node::TABLE_HEAD | node::TABLE_BODY | node::TABLE_FOOT => {
                node.children.iter().flat_map(collect_table_rows).collect()
            }
            node::TABLE_ROW => {
                let cells: Vec<Vec<Inline>> = node
                    .children
                    .iter()
                    .map(|cell| nodes_to_inlines(&cell.children))
                    .collect();
                vec![TableRow {
                    cells,
                    is_header: false,
                }]
            }
            node::TABLE_HEADER => {
                let cells: Vec<Vec<Inline>> = node
                    .children
                    .iter()
                    .map(|cell| nodes_to_inlines(&cell.children))
                    .collect();
                vec![TableRow {
                    cells,
                    is_header: true,
                }]
            }
            _ => vec![],
        }
    }

    fn node_to_image_data(node: &Node) -> ImageData {
        ImageData {
            url: node.props.get_str(prop::URL).unwrap_or("").to_string(),
            alt: node.props.get_str(prop::ALT).map(|s| s.to_string()),
            width: node.props.get_str("width").map(|s| s.to_string()),
            height: node.props.get_str("height").map(|s| s.to_string()),
        }
    }

    fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
        nodes.iter().map(node_to_inline).collect()
    }

    fn node_to_inline(node: &Node) -> Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Text {
                    text: s,
                    span: crate::Span::NONE,
                }
            }

            node::EMPHASIS => Inline::Emphasis(nodes_to_inlines(&node.children), crate::Span::NONE),

            node::STRONG => Inline::Strong(nodes_to_inlines(&node.children), crate::Span::NONE),

            node::STRIKEOUT => {
                Inline::Strikeout(nodes_to_inlines(&node.children), crate::Span::NONE)
            }

            node::UNDERLINE => {
                Inline::Underline(nodes_to_inlines(&node.children), crate::Span::NONE)
            }

            node::SUBSCRIPT => {
                Inline::Subscript(nodes_to_inlines(&node.children), crate::Span::NONE)
            }

            node::SUPERSCRIPT => {
                Inline::Superscript(nodes_to_inlines(&node.children), crate::Span::NONE)
            }

            node::CODE => {
                let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Code(s, crate::Span::NONE)
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                Inline::Link {
                    url,
                    children: nodes_to_inlines(&node.children),
                    target: None,
                    span: crate::Span::NONE,
                }
            }

            node::IMAGE => Inline::Image(node_to_image_data(node), crate::Span::NONE),

            node::LINE_BREAK => Inline::LineBreak {
                span: crate::Span::NONE,
            },

            node::SOFT_BREAK => Inline::SoftBreak {
                span: crate::Span::NONE,
            },

            node::FOOTNOTE_REF => {
                let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
                Inline::FootnoteRef {
                    label,
                    span: crate::Span::NONE,
                }
            }

            node::FOOTNOTE_DEF => {
                let label = node.props.get_str(prop::LABEL).unwrap_or("").to_string();
                Inline::FootnoteDef {
                    label,
                    children: nodes_to_inlines(&node.children),
                    span: crate::Span::NONE,
                }
            }

            node::SMALL_CAPS => {
                Inline::SmallCaps(nodes_to_inlines(&node.children), crate::Span::NONE)
            }

            node::QUOTED => {
                let qt = match node.props.get_str(prop::QUOTE_TYPE).unwrap_or("double") {
                    "single" => QuoteType::Single,
                    _ => QuoteType::Double,
                };
                Inline::Quoted {
                    quote_type: qt,
                    children: nodes_to_inlines(&node.children),
                    span: crate::Span::NONE,
                }
            }

            node::SPAN => {
                // Passthrough span: just recurse
                let children = nodes_to_inlines(&node.children);
                if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    // Wrap in a highlight to group
                    Inline::Highlight(children, crate::Span::NONE)
                }
            }

            node::RAW_INLINE => {
                let format = node
                    .props
                    .get_str(prop::FORMAT)
                    .unwrap_or("asciidoc")
                    .to_string();
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::RawInline {
                    format,
                    content,
                    span: crate::Span::NONE,
                }
            }

            "math_inline" => {
                let content = node.props.get_str("math:source").unwrap_or("").to_string();
                let flavor = node.props.get_str("math:flavor").map(|s| s.to_string());
                Inline::MathInline {
                    content,
                    flavor,
                    span: crate::Span::NONE,
                }
            }

            _ => {
                // Unknown inline: recurse into children, or emit empty text
                let children = nodes_to_inlines(&node.children);
                if children.is_empty() {
                    Inline::Text {
                        text: String::new(),
                        span: crate::Span::NONE,
                    }
                } else if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    Inline::Highlight(children, crate::Span::NONE)
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
            assert!(output.contains("== Title"));
        }

        #[test]
        fn test_emit_heading_level2() {
            let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
            let output = emit_str(&doc);
            assert!(output.contains("=== Subtitle"));
        }

        #[test]
        fn test_emit_emphasis() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("_italic_"));
        }

        #[test]
        fn test_emit_strong() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            let output = emit_str(&doc);
            assert!(output.contains("*bold*"));
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
            assert!(output.contains("https://example.com[click]"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
            let output = emit_str(&doc);
            assert!(output.contains("[source,python]"));
            assert!(output.contains("----"));
            assert!(output.contains("print('hi')"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("* one"));
            assert!(output.contains("* two"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
