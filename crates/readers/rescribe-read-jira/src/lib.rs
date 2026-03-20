//! Jira markup reader for rescribe.
//!
//! Parses Jira/Confluence wiki markup into rescribe documents.
//! Thin adapter over `jira-fmt` standalone library.

use jira_fmt::{Block, Inline, parse as jira_parse};
use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse Jira markup source into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Jira markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (jira_doc, _diags) = jira_parse(input);

    let mut children = Vec::new();
    for block in jira_doc.blocks {
        children.push(block_to_node(&block));
    }

    let root = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(root);
    Ok(ConversionResult::ok(doc))
}

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines, .. } => {
            Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))
        }

        Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(inlines_to_nodes(inlines)),

        Block::CodeBlock { content, language, .. } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }

        Block::Blockquote { children, .. } => {
            let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
            Node::new(node::BLOCKQUOTE).children(block_children)
        }

        Block::Panel { children, .. } => {
            let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
            Node::new(node::DIV)
                .prop("jira:type", "panel")
                .children(block_children)
        }

        Block::List { ordered, items, .. } => {
            let mut list_items = Vec::new();
            for item_blocks in items {
                let mut item_children = Vec::new();
                for item_block in item_blocks {
                    item_children.push(block_to_node(item_block));
                }
                list_items.push(Node::new(node::LIST_ITEM).children(item_children));
            }
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        Block::Table { rows, .. } => {
            let mut table_rows = Vec::new();
            let mut first_row = true;
            let mut has_header = false;

            for row in rows {
                let has_header_cells = row.cells.iter().any(|c| c.is_header);
                if first_row && has_header_cells {
                    has_header = true;
                }
                first_row = false;
            }

            first_row = true;
            for row in rows {
                let mut cells = Vec::new();
                for cell in &row.cells {
                    let cell_kind = if cell.is_header {
                        node::TABLE_HEADER
                    } else {
                        node::TABLE_CELL
                    };
                    cells.push(Node::new(cell_kind).children(inlines_to_nodes(&cell.inlines)));
                }

                let table_row = Node::new(node::TABLE_ROW).children(cells);
                if first_row && has_header {
                    table_rows.push(Node::new(node::TABLE_HEAD).child(table_row));
                } else {
                    table_rows.push(table_row);
                }
                first_row = false;
            }

            Node::new(node::TABLE).children(table_rows)
        }

        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),
    }
}

fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
    inlines.iter().map(inline_to_node).collect()
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children, _) => Node::new(node::STRONG).children(inlines_to_nodes(children)),

        Inline::Italic(children, _) => Node::new(node::EMPHASIS).children(inlines_to_nodes(children)),

        Inline::Underline(children, _) => {
            Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
        }

        Inline::Strikethrough(children, _) => {
            Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
        }

        Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, children, .. } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .children(inlines_to_nodes(children)),

        Inline::Image { url, alt, .. } => {
            let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if let Some(alt_text) = alt {
                img = img.prop(prop::ALT, alt_text.clone());
            }
            img
        }

        Inline::Superscript(children, _) => {
            Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
        }

        Inline::Subscript(children, _) => {
            Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
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
        let doc = parse_str("h1. Title");
        let heading = &doc.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world!");
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("This is *bold* text.");
        let para = &doc.content.children[0];
        assert_eq!(para.children[1].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("This is _italic_ text.");
        let para = &doc.content.children[0];
        assert_eq!(para.children[1].kind.as_str(), node::EMPHASIS);
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("Use {{code}} here.");
        let para = &doc.content.children[0];
        assert_eq!(para.children[1].kind.as_str(), node::CODE);
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("Click [here|https://example.com].");
        let para = &doc.content.children[0];
        let link = &para.children[1];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_str("* Item 1\n* Item 2");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_str("{code:java}\npublic class Test {}\n{code}");
        let code = &doc.content.children[0];
        assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
        assert_eq!(code.props.get_str(prop::LANGUAGE), Some("java"));
    }
}
