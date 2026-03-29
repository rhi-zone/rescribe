//! Man page (roff/troff) reader for rescribe.
//!
//! Thin adapter layer around the `man-fmt` crate.
//! Parses Unix manual pages into rescribe's document IR.

use man_fmt::{Block, Inline, ManDoc};
use rescribe_core::{ConversionResult, Document, Node, ParseError, Properties};
use rescribe_std::{node, prop};

/// Parse man page source into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (man_doc, _diags) = man_fmt::parse(input);

    let rescribe_doc = convert_to_document(man_doc);

    Ok(ConversionResult::ok(rescribe_doc))
}

fn convert_to_document(man: ManDoc) -> Document {
    let mut metadata = Properties::new();

    if let Some(title) = man.title {
        metadata.set("title", title);
    }
    if let Some(section) = man.section {
        metadata.set("man:section", section);
    }
    if let Some(date) = man.date {
        metadata.set("man:date", date);
    }
    if let Some(source) = man.source {
        metadata.set("man:source", source);
    }
    if let Some(manual) = man.manual {
        metadata.set("man:manual", manual);
    }

    let mut children = Vec::new();
    for block in man.blocks {
        if let Some(node) = convert_block(&block) {
            children.push(node);
        }
    }

    let content = Node::new(node::DOCUMENT).children(children);

    Document {
        content,
        resources: Default::default(),
        metadata,
        source: None,
    }
}

fn convert_block(block: &Block) -> Option<Node> {
    match block {
        Block::Heading { level, inlines, .. } => {
            let level_int = *level as i64;
            let children = convert_inlines(inlines);
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level_int)
                    .children(children),
            )
        }

        Block::Paragraph { inlines, .. } => {
            let children = convert_inlines(inlines);
            Some(Node::new(node::PARAGRAPH).children(children))
        }

        Block::CodeBlock { content, .. } => {
            Some(Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone()))
        }

        Block::List { ordered, items, .. } => {
            let mut children = Vec::new();
            for item_blocks in items {
                let mut item_children = Vec::new();
                for block in item_blocks {
                    if let Some(n) = convert_block(block) {
                        item_children.push(n);
                    }
                }
                let item_node = Node::new(node::LIST_ITEM).children(item_children);
                children.push(item_node);
            }
            Some(
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(children),
            )
        }

        Block::DefinitionList { items, .. } => {
            let mut children = Vec::new();
            for (term_inlines, content_blocks) in items {
                let term_children = convert_inlines(term_inlines);
                let term_node = Node::new(node::DEFINITION_TERM).children(term_children);
                children.push(term_node);

                for content_block in content_blocks {
                    let content_children = match content_block {
                        Block::Paragraph { inlines, .. } => {
                            vec![Node::new(node::PARAGRAPH).children(convert_inlines(inlines))]
                        }
                        other => {
                            if let Some(n) = convert_block(other) {
                                vec![n]
                            } else {
                                vec![]
                            }
                        }
                    };

                    let desc_node = Node::new(node::DEFINITION_DESC).children(content_children);
                    children.push(desc_node);
                }
            }
            Some(Node::new(node::DEFINITION_LIST).children(children))
        }

        Block::IndentedParagraph { inlines, .. } => {
            let children = convert_inlines(inlines);
            Some(Node::new(node::PARAGRAPH).children(children))
        }

        Block::ExampleBlock { content, .. } => {
            Some(Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone()))
        }

        Block::Comment { .. } => {
            // Comments are not represented in the IR
            None
        }

        Block::HorizontalRule { .. } => Some(Node::new(node::HORIZONTAL_RULE)),
    }
}

fn convert_inlines(inlines: &[Inline]) -> Vec<Node> {
    let mut nodes = Vec::new();
    for inline in inlines {
        if let Some(node) = convert_inline(inline) {
            nodes.push(node);
        }
    }
    nodes
}

fn convert_inline(inline: &Inline) -> Option<Node> {
    match inline {
        Inline::Text(text, _) => Some(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        Inline::Bold(children, _) => {
            let child_nodes = convert_inlines(children);
            Some(Node::new(node::STRONG).children(child_nodes))
        }

        Inline::Italic(children, _) => {
            let child_nodes = convert_inlines(children);
            Some(Node::new(node::EMPHASIS).children(child_nodes))
        }

        Inline::Code(text, _) => {
            Some(Node::new(node::CODE).prop(prop::CONTENT, text.clone()))
        }

        Inline::Superscript(children, _) => {
            let child_nodes = convert_inlines(children);
            Some(Node::new(node::SUPERSCRIPT).children(child_nodes))
        }

        Inline::Subscript(children, _) => {
            let child_nodes = convert_inlines(children);
            Some(Node::new(node::SUBSCRIPT).children(child_nodes))
        }

        Inline::Link { url, children, .. } => {
            let child_nodes = convert_inlines(children);
            Some(
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(child_nodes),
            )
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
    fn test_parse_title() {
        let doc = parse_str(".TH TEST 1 \"2024-01-01\" \"Version 1.0\"");
        assert_eq!(doc.metadata.get_str("title"), Some("TEST"));
        assert_eq!(doc.metadata.get_str("man:section"), Some("1"));
    }

    #[test]
    fn test_parse_sections() {
        let doc = parse_str(".SH NAME\ntest \\- a test program\n.SH SYNOPSIS\ntest [options]");
        assert_eq!(doc.content.children.len(), 4); // 2 headings + 2 paragraphs
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str(".B bold text");
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
        assert_eq!(para.children[0].kind.as_str(), node::STRONG);
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str(".I italic text");
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
        assert_eq!(para.children[0].kind.as_str(), node::EMPHASIS);
    }

    #[test]
    fn test_parse_preformatted() {
        let doc = parse_str(".nf\ncode line 1\ncode line 2\n.fi");
        let code = &doc.content.children[0];
        assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
    }

    #[test]
    fn test_parse_inline_font() {
        let doc = parse_str("This is \\fBbold\\fR text");
        let para = &doc.content.children[0];
        // Should have multiple children
        assert!(para.children.len() >= 2);
    }
}
