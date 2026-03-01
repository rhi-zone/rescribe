//! Texinfo reader for rescribe.
//!
//! Parses GNU Texinfo documentation format into rescribe's document IR.
//!
//! # Example
//!
//! ```
//! use rescribe_read_texinfo::parse;
//!
//! let texinfo = r#"@chapter Introduction
//! This is the introduction.
//!
//! @section Getting Started
//! Here is how to get started."#;
//!
//! let result = parse(texinfo).unwrap();
//! let doc = result.value;
//! ```

use rescribe_core::{ConversionResult, Document, FidelityWarning, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use texinfo::{self, Block, Inline};

/// Parse Texinfo into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Texinfo with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let texinfo_doc = texinfo::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;

    let mut warnings: Vec<FidelityWarning> = Vec::new();
    let mut result_nodes = Vec::new();

    for block in texinfo_doc.blocks {
        result_nodes.push(block_to_node(&block, &mut warnings));
    }

    let mut metadata = rescribe_core::Properties::new();
    if let Some(title) = texinfo_doc.title {
        metadata.set("title", title);
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(result_nodes),
        resources: Default::default(),
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

fn block_to_node(block: &Block, _warnings: &mut Vec<FidelityWarning>) -> Node {
    match block {
        Block::Heading { level, inlines } => {
            let inline_nodes = inlines_to_nodes(inlines);
            Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inline_nodes)
        }

        Block::Paragraph { inlines } => {
            let inline_nodes = inlines_to_nodes(inlines);
            Node::new(node::PARAGRAPH).children(inline_nodes)
        }

        Block::CodeBlock { content } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
        }

        Block::Blockquote { children } => {
            let block_nodes: Vec<_> = children
                .iter()
                .map(|b| block_to_node(b, _warnings))
                .collect();
            Node::new(node::BLOCKQUOTE).children(block_nodes)
        }

        Block::List { ordered, items } => {
            let list_items: Vec<_> = items
                .iter()
                .map(|item_inlines| {
                    let inline_nodes = inlines_to_nodes(item_inlines);
                    Node::new(node::LIST_ITEM).children(inline_nodes)
                })
                .collect();

            Node::new(node::LIST)
                .prop(prop::ORDERED, *ordered)
                .children(list_items)
        }

        Block::DefinitionList { items } => {
            let mut def_nodes = Vec::new();
            for (term_inlines, desc_blocks) in items {
                let term_inline_nodes = inlines_to_nodes(term_inlines);
                def_nodes.push(Node::new(node::DEFINITION_TERM).children(term_inline_nodes));

                let desc_block_nodes: Vec<_> = desc_blocks
                    .iter()
                    .map(|b| block_to_node(b, _warnings))
                    .collect();
                def_nodes.push(Node::new(node::DEFINITION_DESC).children(desc_block_nodes));
            }
            Node::new(node::DEFINITION_LIST).children(def_nodes)
        }

        Block::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
    }
}

fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
    inlines.iter().map(inline_to_node).collect()
}

fn inline_to_node(inline: &Inline) -> Node {
    match inline {
        Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),

        Inline::Strong(children) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::STRONG).children(inline_nodes)
        }

        Inline::Emphasis(children) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::EMPHASIS).children(inline_nodes)
        }

        Inline::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),

        Inline::Link { url, children } => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(inline_nodes)
        }

        Inline::Superscript(children) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::SUPERSCRIPT).children(inline_nodes)
        }

        Inline::Subscript(children) => {
            let inline_nodes = inlines_to_nodes(children);
            Node::new(node::SUBSCRIPT).children(inline_nodes)
        }

        Inline::LineBreak => Node::new(node::LINE_BREAK),

        Inline::SoftBreak => Node::new(node::SOFT_BREAK),

        Inline::FootnoteDef { content } => {
            let inline_nodes = inlines_to_nodes(content);
            Node::new(node::FOOTNOTE_DEF).children(inline_nodes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = r#"@chapter Introduction
This is the introduction paragraph.

@section Getting Started
Here is how to get started."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_headings() {
        let input = r#"@chapter Chapter One
@section Section One
@subsection Subsection One
@subsubsection Sub-subsection"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 4);
    }

    #[test]
    fn test_parse_emphasis() {
        let input = r#"This is @emph{emphasized} and @strong{bold} text."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let input = r#"Use @code{printf} to print."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let input = r#"@itemize
@item First item
@item Second item
@end itemize"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.content.children[0].kind.as_str(), node::LIST);
    }

    #[test]
    fn test_parse_enumerate() {
        let input = r#"@enumerate
@item First
@item Second
@end enumerate"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        let list = &doc.content.children[0];
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    fn test_parse_example() {
        let input = r#"@example
int main() {
    return 0;
}
@end example"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }

    #[test]
    fn test_parse_url() {
        let input = r#"Visit @uref{https://example.com, Example Site}."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_quotation() {
        let input = r#"@quotation
This is a quoted passage.
@end quotation"#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.content.children[0].kind.as_str(), node::BLOCKQUOTE);
    }

    #[test]
    fn test_skip_comments() {
        let input = r#"@c This is a comment
This is visible.
@comment Another comment
Still visible."#;

        let result = parse(input).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }
}
