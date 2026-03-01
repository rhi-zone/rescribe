//! ANSI escape sequence reader for rescribe.
//!
//! Thin adapter converting ansi-fmt's AST to rescribe's document IR.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse ANSI-formatted text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse ANSI-formatted text with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let ansi_doc = ansi_fmt::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;

    let mut blocks = Vec::new();
    for block in &ansi_doc.blocks {
        blocks.push(ansi_block_to_node(block));
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(blocks),
        resources: Default::default(),
        metadata: Default::default(),
        source: None,
    };

    Ok(ConversionResult::ok(document))
}

fn ansi_block_to_node(block: &ansi_fmt::Block) -> Node {
    match block {
        ansi_fmt::Block::Paragraph { inlines } => {
            let children: Vec<Node> = inlines.iter().map(ansi_inline_to_node).collect();
            Node::new(node::PARAGRAPH).children(children)
        }
        ansi_fmt::Block::Heading { level, inlines } => {
            let children: Vec<Node> = inlines.iter().map(ansi_inline_to_node).collect();
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(children)
        }
        ansi_fmt::Block::CodeBlock { language, content } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }
        ansi_fmt::Block::Blockquote { children } => {
            let nodes: Vec<Node> = children.iter().map(ansi_block_to_node).collect();
            Node::new(node::BLOCKQUOTE).children(nodes)
        }
        ansi_fmt::Block::List { ordered, items } => {
            let mut n = Node::new(node::LIST);
            if *ordered {
                n = n.prop(prop::ORDERED, true);
            }
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_blocks| {
                    let blocks: Vec<Node> = item_blocks.iter().map(ansi_block_to_node).collect();
                    Node::new(node::LIST_ITEM).children(blocks)
                })
                .collect();
            n.children(list_items)
        }
        ansi_fmt::Block::ListItem { children } => {
            let nodes: Vec<Node> = children.iter().map(ansi_block_to_node).collect();
            Node::new(node::LIST_ITEM).children(nodes)
        }
        ansi_fmt::Block::Table { rows } => {
            let table_rows: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let inlines: Vec<Node> =
                                cell.inlines.iter().map(ansi_inline_to_node).collect();
                            Node::new(node::TABLE_CELL).children(inlines)
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE).children(table_rows)
        }
        ansi_fmt::Block::TableRow { cells } => {
            let table_cells: Vec<Node> = cells
                .iter()
                .map(|cell| {
                    let inlines: Vec<Node> = cell.inlines.iter().map(ansi_inline_to_node).collect();
                    Node::new(node::TABLE_CELL).children(inlines)
                })
                .collect();
            Node::new(node::TABLE_ROW).children(table_cells)
        }
        ansi_fmt::Block::TableCell { inlines } => {
            let children: Vec<Node> = inlines.iter().map(ansi_inline_to_node).collect();
            Node::new(node::TABLE_CELL).children(children)
        }
        ansi_fmt::Block::TableHeader { cells } => {
            let table_cells: Vec<Node> = cells
                .iter()
                .map(|cell| {
                    let inlines: Vec<Node> = cell.inlines.iter().map(ansi_inline_to_node).collect();
                    Node::new(node::TABLE_CELL).children(inlines)
                })
                .collect();
            Node::new(node::TABLE_HEAD).children(table_cells)
        }
        ansi_fmt::Block::TableBody { rows } => {
            let table_rows: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let inlines: Vec<Node> =
                                cell.inlines.iter().map(ansi_inline_to_node).collect();
                            Node::new(node::TABLE_CELL).children(inlines)
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE_BODY).children(table_rows)
        }
        ansi_fmt::Block::TableFoot { rows } => {
            let table_rows: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let inlines: Vec<Node> =
                                cell.inlines.iter().map(ansi_inline_to_node).collect();
                            Node::new(node::TABLE_CELL).children(inlines)
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE_FOOT).children(table_rows)
        }
        ansi_fmt::Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
        ansi_fmt::Block::Div { children } => {
            let nodes: Vec<Node> = children.iter().map(ansi_block_to_node).collect();
            Node::new(node::DIV).children(nodes)
        }
        ansi_fmt::Block::Span { inlines } => {
            let children: Vec<Node> = inlines.iter().map(ansi_inline_to_node).collect();
            Node::new(node::SPAN).children(children)
        }
        ansi_fmt::Block::RawBlock { content } => {
            Node::new(node::RAW_BLOCK).prop(prop::CONTENT, content.clone())
        }
        ansi_fmt::Block::RawInline { content } => {
            Node::new(node::RAW_INLINE).prop(prop::CONTENT, content.clone())
        }
        ansi_fmt::Block::DefinitionList { items } => {
            let list_items: Vec<Node> = items
                .iter()
                .flat_map(|item| {
                    let term = Node::new(node::DEFINITION_TERM).children(
                        item.term
                            .iter()
                            .map(ansi_inline_to_node)
                            .collect::<Vec<_>>(),
                    );
                    let descs: Vec<Node> = item.desc.iter().map(ansi_block_to_node).collect();
                    let desc = Node::new(node::DEFINITION_DESC).children(descs);
                    vec![term, desc]
                })
                .collect();
            Node::new(node::DEFINITION_LIST).children(list_items)
        }
        ansi_fmt::Block::DefinitionTerm { inlines } => {
            let children: Vec<Node> = inlines.iter().map(ansi_inline_to_node).collect();
            Node::new(node::DEFINITION_TERM).children(children)
        }
        ansi_fmt::Block::DefinitionDesc { children } => {
            let nodes: Vec<Node> = children.iter().map(ansi_block_to_node).collect();
            Node::new(node::DEFINITION_DESC).children(nodes)
        }
        ansi_fmt::Block::Figure { children } => {
            let nodes: Vec<Node> = children.iter().map(ansi_block_to_node).collect();
            Node::new(node::FIGURE).children(nodes)
        }
    }
}

fn ansi_inline_to_node(inline: &ansi_fmt::Inline) -> Node {
    match inline {
        ansi_fmt::Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),
        ansi_fmt::Inline::Bold(children) => {
            let inner: Vec<Node> = children.iter().map(ansi_inline_to_node).collect();
            Node::new(node::STRONG).children(inner)
        }
        ansi_fmt::Inline::Italic(children) => {
            let inner: Vec<Node> = children.iter().map(ansi_inline_to_node).collect();
            Node::new(node::EMPHASIS).children(inner)
        }
        ansi_fmt::Inline::Underline(children) => {
            let inner: Vec<Node> = children.iter().map(ansi_inline_to_node).collect();
            Node::new(node::UNDERLINE).children(inner)
        }
        ansi_fmt::Inline::Strikethrough(children) => {
            let inner: Vec<Node> = children.iter().map(ansi_inline_to_node).collect();
            Node::new(node::STRIKEOUT).children(inner)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let result = parse("Hello world").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_bold() {
        let result = parse("\x1b[1mBold text\x1b[0m").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let result = parse("\x1b[3mItalic text\x1b[0m").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_underline() {
        let result = parse("\x1b[4mUnderlined\x1b[0m").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_combined_styles() {
        let result = parse("\x1b[1;3mBold and italic\x1b[0m").unwrap();
        assert!(!result.value.content.children.is_empty());
    }
}
