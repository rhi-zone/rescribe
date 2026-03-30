//! reStructuredText (RST) reader for rescribe.
//!
//! Thin adapter over [`rst_fmt`]: parses RST into the `rst_fmt` AST,
//! then maps it to the rescribe document model.

use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Severity, WarningKind,
};
use rescribe_std::{Node, node, prop};
use rst_fmt::{Block, DefinitionItem, Inline, RstDoc, TableRow};

/// Parse RST text into a rescribe Document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse RST with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let rst = rst_fmt::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;
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
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
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
            let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if let Some(alt_text) = alt {
                img = img.prop(prop::ALT, alt_text.clone());
            }
            let mut figure_children = vec![img];
            if let Some(cap_inlines) = caption {
                let cap_nodes = inlines_to_nodes(cap_inlines, warnings);
                figure_children.push(Node::new(node::CAPTION).children(cap_nodes));
            }
            Node::new(node::FIGURE).children(figure_children)
        }

        Block::Image { url, alt, title } => {
            let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if let Some(alt_text) = alt {
                n = n.prop(prop::ALT, alt_text.clone());
            }
            if let Some(title_text) = title {
                n = n.prop(prop::TITLE, title_text.clone());
            }
            n
        }

        Block::RawBlock { format, content } => Node::new(node::RAW_BLOCK)
            .prop(prop::CONTENT, content.clone())
            .prop("format", format.clone()),

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
                n = n.prop("class", cls.clone());
            }
            if let Some(dir_name) = directive {
                n = n.prop("rst:directive", dir_name.clone());
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
                .prop(prop::LABEL, label.clone())
                .children(child_nodes)
        }

        Block::MathDisplay { source } => {
            Node::new("math_display").prop("math:source", source.clone())
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
                .prop("admonition_type", admonition_type.clone())
                .children(child_nodes)
        }

    }
}

fn def_items_to_nodes(items: &[DefinitionItem], warnings: &mut Vec<FidelityWarning>) -> Vec<Node> {
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
    let row_kind = if row.is_header { node::TABLE_HEADER } else { node::TABLE_ROW };
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
        Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

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

        Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, children } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .children(inlines_to_nodes(children, warnings)),

        Inline::Image { url, alt } => Node::new(node::IMAGE)
            .prop(prop::URL, url.clone())
            .prop(prop::ALT, alt.clone()),

        Inline::LineBreak => Node::new(node::LINE_BREAK),

        Inline::SoftBreak => Node::new(node::SOFT_BREAK),

        Inline::FootnoteRef { label } => {
            Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone())
        }

        Inline::FootnoteDef { label, children } => Node::new(node::FOOTNOTE_DEF)
            .prop(prop::LABEL, label.clone())
            .children(inlines_to_nodes(children, warnings)),

        Inline::SmallCaps(children) => {
            Node::new(node::SMALL_CAPS).children(inlines_to_nodes(children, warnings))
        }

        Inline::Quoted {
            quote_type,
            children,
        } => Node::new(node::QUOTED)
            .prop(prop::QUOTE_TYPE, quote_type.clone())
            .children(inlines_to_nodes(children, warnings)),

        Inline::MathInline { source } => {
            Node::new("math_inline").prop("math:source", source.clone())
        }

        Inline::RstSpan { role, children } => {
            let child_nodes = inlines_to_nodes(children, warnings);
            match role.as_str() {
                "small-caps" | "sc" => Node::new(node::SMALL_CAPS).children(child_nodes),
                "strike" | "del" | "s" => Node::new(node::STRIKEOUT).children(child_nodes),
                "underline" | "u" => Node::new(node::UNDERLINE).children(child_nodes),
                _ => Node::new(node::SPAN)
                    .prop("rst:role", role.clone())
                    .children(child_nodes),
            }
        }
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
