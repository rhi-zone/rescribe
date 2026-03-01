//! DokuWiki reader for rescribe.
//!
//! Parses DokuWiki markup into rescribe documents.
//! Thin adapter over the standalone `dokuwiki` crate.

use dokuwiki::{Block as FmtBlock, Inline as FmtInline, parse as fmt_parse};
use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse DokuWiki source into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse DokuWiki source with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let doc = fmt_parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;

    let mut children = Vec::new();
    for block in &doc.blocks {
        children.push(convert_block(block));
    }

    let root = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(root);
    Ok(ConversionResult::ok(doc))
}

fn convert_block(block: &FmtBlock) -> Node {
    match block {
        FmtBlock::Paragraph { inlines } => {
            let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
            Node::new(node::PARAGRAPH).children(children)
        }

        FmtBlock::Heading { level, inlines } => {
            let children: Vec<Node> = inlines.iter().map(convert_inline).collect();
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(children)
        }

        FmtBlock::CodeBlock { language, content } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            n
        }

        FmtBlock::Blockquote { children } => {
            let converted: Vec<Node> = children.iter().map(convert_block).collect();
            Node::new(node::BLOCKQUOTE).children(converted)
        }

        FmtBlock::List { ordered, items } => {
            let mut list_items = Vec::new();
            for item_blocks in items {
                let mut item_children = Vec::new();
                for block in item_blocks {
                    item_children.push(convert_block(block));
                }
                let item = Node::new(node::LIST_ITEM).children(item_children);
                list_items.push(item);
            }
            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        FmtBlock::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
    }
}

fn convert_inline(inline: &FmtInline) -> Node {
    match inline {
        FmtInline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        FmtInline::Bold(children) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::STRONG).children(converted)
        }

        FmtInline::Italic(children) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::EMPHASIS).children(converted)
        }

        FmtInline::Underline(children) => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::UNDERLINE).children(converted)
        }

        FmtInline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        FmtInline::Link { url, children } => {
            let converted: Vec<Node> = children.iter().map(convert_inline).collect();
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(converted)
        }

        FmtInline::Image { url, alt } => {
            let mut n = Node::new(node::IMAGE).prop(prop::URL, url.clone());
            if let Some(alt_text) = alt {
                n = n.prop(prop::ALT, alt_text.clone());
            }
            n
        }

        FmtInline::LineBreak => Node::new(node::LINE_BREAK),
        FmtInline::SoftBreak => Node::new(node::SOFT_BREAK),
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
        let doc = parse_str("====== Title ======");
        let heading = &doc.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse_str("====== H1 ======\n===== H2 =====\n==== H3 ====");
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        assert_eq!(doc.content.children[1].props.get_int(prop::LEVEL), Some(2));
        assert_eq!(doc.content.children[2].props.get_int(prop::LEVEL), Some(3));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world!");
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("This is **bold** text.");
        let para = &doc.content.children[0];
        assert_eq!(para.children[1].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("This is //italic// text.");
        let para = &doc.content.children[0];
        assert_eq!(para.children[1].kind.as_str(), node::EMPHASIS);
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("Use ''code'' here.");
        let para = &doc.content.children[0];
        assert_eq!(para.children[1].kind.as_str(), node::CODE);
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("Click [[https://example.com|here]].");
        let para = &doc.content.children[0];
        let link = &para.children[1];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_str("  * Item 1\n  * Item 2");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_str("<code rust>\nfn main() {}\n</code>");
        let code = &doc.content.children[0];
        assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
        assert_eq!(code.props.get_str(prop::LANGUAGE), Some("rust"));
    }
}
