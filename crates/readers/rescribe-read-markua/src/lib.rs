//! Markua (Leanpub) reader for rescribe.
//!
//! Thin adapter from standalone markua format crate to rescribe document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
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
    let (markua_doc, _diagnostics) = markua::parse(input);
    let nodes = convert_blocks(&markua_doc.blocks);

    let root = Node::new(node::DOCUMENT).children(nodes);
    let doc = Document::new().with_content(root);

    Ok(ConversionResult::ok(doc))
}

fn convert_blocks(blocks: &[markua::Block]) -> Vec<Node> {
    blocks.iter().map(convert_block).collect()
}

fn convert_block(block: &markua::Block) -> Node {
    match block {
        markua::Block::Paragraph { inlines, .. } => {
            Node::new(node::PARAGRAPH).children(convert_inlines(inlines))
        }

        markua::Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
            .prop(prop::LEVEL, *level as i64)
            .children(convert_inlines(inlines)),

        markua::Block::CodeBlock {
            content, language, ..
        } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.as_str());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.as_str());
            }
            n
        }

        markua::Block::Blockquote { children, .. } => {
            Node::new(node::BLOCKQUOTE).children(convert_blocks(children))
        }

        markua::Block::List { ordered, items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|item_blocks| Node::new(node::LIST_ITEM).children(convert_blocks(item_blocks)))
                .collect();

            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        markua::Block::Table { rows, .. } => {
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

        markua::Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

        markua::Block::SpecialBlock {
            block_type,
            inlines,
            ..
        } => {
            let para = Node::new(node::PARAGRAPH).children(convert_inlines(inlines));
            Node::new(node::DIV)
                .prop("class", block_type.as_str())
                .children(vec![para])
        }
    }
}

fn convert_inlines(inlines: &[markua::Inline]) -> Vec<Node> {
    inlines.iter().map(convert_inline).collect()
}

fn convert_inline(inline: &markua::Inline) -> Node {
    match inline {
        markua::Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.as_str()),

        markua::Inline::Strong(children, _) => {
            Node::new(node::STRONG).children(convert_inlines(children))
        }

        markua::Inline::Emphasis(children, _) => {
            Node::new(node::EMPHASIS).children(convert_inlines(children))
        }

        markua::Inline::Strikethrough(children, _) => {
            Node::new(node::STRIKEOUT).children(convert_inlines(children))
        }

        markua::Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.as_str()),

        markua::Inline::Link { url, children, .. } => Node::new(node::LINK)
            .prop(prop::URL, url.as_str())
            .children(convert_inlines(children)),

        markua::Inline::Image { url, alt, .. } => Node::new(node::IMAGE)
            .prop(prop::URL, url.as_str())
            .prop(prop::ALT, alt.as_str()),

        markua::Inline::LineBreak(_) => Node::new(node::LINE_BREAK),

        markua::Inline::SoftBreak(_) => Node::new(node::SOFT_BREAK),
    }
}
