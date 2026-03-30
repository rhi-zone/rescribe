//! BBCode reader for rescribe.
//!
//! Parses BBCode forum markup into rescribe's document IR.
//! Uses `bbcode-fmt` for parsing and AST, adapts to rescribe types.

use bbcode_fmt::{Block, Inline};
use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse BBCode markup into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse BBCode markup with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, _diagnostics) = bbcode_fmt::parse(input);

    let mut blocks = Vec::new();
    for block in &doc.blocks {
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

fn block_to_node(block: &Block) -> Node {
    match block {
        Block::Paragraph { inlines, .. } => {
            let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
            Node::new(node::PARAGRAPH).children(children)
        }

        Block::CodeBlock { language, content, .. } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }

        Block::Blockquote { author, children, .. } => {
            let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
            let mut n = Node::new(node::BLOCKQUOTE);
            if let Some(a) = author {
                n = n.prop("bbcode:author", a.clone());
            }
            n.children(block_children)
        }

        Block::List { ordered, items, .. } => {
            let list_items: Vec<Node> = items
                .iter()
                .map(|inlines| {
                    let inline_nodes: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                    Node::new(node::LIST_ITEM).children(inline_nodes)
                })
                .collect();
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        Block::Table { rows, .. } => {
            let table_rows: Vec<Node> = rows
                .iter()
                .map(|row| {
                    let cells: Vec<Node> = row
                        .cells
                        .iter()
                        .map(|(is_header, inlines)| {
                            let kind = if *is_header {
                                node::TABLE_HEADER
                            } else {
                                node::TABLE_CELL
                            };
                            let children: Vec<Node> = inlines.iter().map(inline_to_node).collect();
                            Node::new(kind).children(children)
                        })
                        .collect();
                    Node::new(node::TABLE_ROW).children(cells)
                })
                .collect();
            Node::new(node::TABLE).children(table_rows)
        }

        Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),

        Block::Heading { level, children, .. } => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(child_nodes)
        }

        Block::Alignment { kind, children, .. } => {
            let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
            let align = match kind {
                bbcode_fmt::AlignKind::Center => "center",
                bbcode_fmt::AlignKind::Left => "left",
                bbcode_fmt::AlignKind::Right => "right",
            };
            Node::new(node::DIV)
                .prop("style:align", align)
                .children(block_children)
        }

        Block::Spoiler { children, .. } => {
            let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
            Node::new(node::DIV)
                .prop("bbcode:spoiler", true)
                .children(block_children)
        }

        Block::Preformatted { content, .. } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        Block::Indent { children, .. } => {
            let block_children: Vec<Node> = children.iter().map(block_to_node).collect();
            Node::new(node::BLOCKQUOTE).children(block_children)
        }
    }
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s, _) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Bold(children, _) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::STRONG).children(child_nodes)
        }

        Inline::Italic(children, _) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::EMPHASIS).children(child_nodes)
        }

        Inline::Underline(children, _) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::UNDERLINE).children(child_nodes)
        }

        Inline::Strikethrough(children, _) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::STRIKEOUT).children(child_nodes)
        }

        Inline::Code(s, _) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, children, .. } => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(child_nodes)
        }

        Inline::Image { url, width, height, .. } => {
            let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if let Some(w) = width {
                n = n.prop("bbcode:width", *w as i64);
            }
            if let Some(h) = height {
                n = n.prop("bbcode:height", *h as i64);
            }
            n
        }

        Inline::Subscript(children, _) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::SUBSCRIPT).children(child_nodes)
        }

        Inline::Superscript(children, _) => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::SUPERSCRIPT).children(child_nodes)
        }

        Inline::Color { value, children, .. } => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::SPAN)
                .prop("style:color", value.clone())
                .children(child_nodes)
        }

        Inline::Size { value, children, .. } => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::SPAN)
                .prop("style:size", value.clone())
                .children(child_nodes)
        }

        Inline::Font { name, children, .. } => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::SPAN)
                .prop("style:font", name.clone())
                .children(child_nodes)
        }

        Inline::Email { addr, children, .. } => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            Node::new(node::LINK)
                .prop(prop::URL, format!("mailto:{}", addr))
                .children(child_nodes)
        }

        Inline::Noparse(s, _) => Node::new(node::RAW_INLINE).prop(prop::CONTENT, s.clone()),

        Inline::Span {
            attr,
            value,
            children,
            ..
        } => {
            let child_nodes: Vec<Node> = children.iter().map(inline_to_node).collect();
            let prop_key = format!("style:{}", attr);
            Node::new(node::SPAN)
                .prop(prop_key, value.clone())
                .children(child_nodes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bold() {
        let result = parse("This is [b]bold[/b] text").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let result = parse("This is [i]italic[/i] text").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_link() {
        let result = parse("[url=http://example.com]Example[/url]").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let result = parse("[list]\n[*]Item 1\n[*]Item 2\n[/list]").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let result = parse("[code]print('hello')[/code]").unwrap();
        assert!(!result.value.content.children.is_empty());
    }
}
