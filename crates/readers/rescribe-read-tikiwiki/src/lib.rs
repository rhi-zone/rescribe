//! TikiWiki reader for rescribe.
//!
//! Thin adapter converting tikiwiki AST to rescribe's document IR.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use tikiwiki::Inline as TwInline;

/// Parse TikiWiki markup into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse TikiWiki markup with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (tw_doc, _diags) = tikiwiki::parse(input);

    let mut blocks = Vec::new();
    for block in &tw_doc.blocks {
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

fn block_to_node(block: &tikiwiki::Block) -> Node {
    use tikiwiki::Block;

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

        Block::Blockquote { inlines, .. } => Node::new(node::BLOCKQUOTE)
            .child(Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines))),

        Block::List { ordered, items, .. } => {
            let mut list_items = Vec::new();
            for item_inlines in items {
                list_items
                    .push(Node::new(node::LIST_ITEM).child(
                        Node::new(node::PARAGRAPH).children(inlines_to_nodes(item_inlines)),
                    ));
            }
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        Block::Table { rows, .. } => {
            let mut table_rows = Vec::new();
            for row in rows {
                let mut cells = Vec::new();
                for cell_inlines in &row.cells {
                    cells
                        .push(Node::new(node::TABLE_CELL).children(inlines_to_nodes(cell_inlines)));
                }
                table_rows.push(Node::new(node::TABLE_ROW).children(cells));
            }
            Node::new(node::TABLE).children(table_rows)
        }

        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),
    }
}

fn inlines_to_nodes(inlines: &[TwInline]) -> Vec<Node> {
    inlines.iter().map(inline_to_node).collect()
}

fn inline_to_node(inline: &TwInline) -> Node {
    use tikiwiki::Inline;

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
            let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if !alt.is_empty() {
                n = n.prop(prop::ALT, alt.clone());
            }
            n
        }

        Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let result = parse("!Heading 1\n!!Heading 2").unwrap();
        assert_eq!(result.value.content.children.len(), 2);
    }

    #[test]
    fn test_parse_bold() {
        let result = parse("This is __bold__ text").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let result = parse("This is ''italic'' text").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_link() {
        let result = parse("[http://example.com|Example]").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let result = parse("*Item 1\n*Item 2").unwrap();
        assert_eq!(result.value.content.children.len(), 1);
        assert_eq!(result.value.content.children[0].kind.as_str(), node::LIST);
    }

    #[test]
    fn test_parse_table() {
        let result = parse("||A|B||\n||C|D||").unwrap();
        assert_eq!(result.value.content.children.len(), 1);
        assert_eq!(result.value.content.children[0].kind.as_str(), node::TABLE);
    }
}
