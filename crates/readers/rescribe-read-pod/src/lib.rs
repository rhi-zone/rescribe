//! POD (Plain Old Documentation) reader for rescribe.
//!
//! Parses Perl POD markup into the rescribe document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse POD markup.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse POD markup with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let doc = pod_fmt::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;

    let blocks: Vec<Node> = doc.blocks.into_iter().map(convert_block).collect();
    let root = Node::new(node::DOCUMENT).children(blocks);
    let rescribe_doc = Document::new().with_content(root);

    Ok(ConversionResult::ok(rescribe_doc))
}

fn convert_block(block: pod_fmt::Block) -> Node {
    match block {
        pod_fmt::Block::Heading { level, inlines } => Node::new(node::HEADING)
            .prop(prop::LEVEL, level as i64)
            .children(convert_inlines(inlines)),

        pod_fmt::Block::Paragraph { inlines } => {
            Node::new(node::PARAGRAPH).children(convert_inlines(inlines))
        }

        pod_fmt::Block::CodeBlock { content } => {
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content)
        }

        pod_fmt::Block::List { ordered, items } => {
            let list_items: Vec<Node> = items
                .into_iter()
                .map(|item_blocks| {
                    let children: Vec<Node> = item_blocks.into_iter().map(convert_block).collect();
                    Node::new(node::LIST_ITEM).children(children)
                })
                .collect();

            Node::new(node::LIST)
                .prop(prop::ORDERED, ordered)
                .children(list_items)
        }
    }
}

fn convert_inlines(inlines: Vec<pod_fmt::Inline>) -> Vec<Node> {
    inlines.into_iter().map(convert_inline).collect()
}

fn convert_inline(inline: pod_fmt::Inline) -> Node {
    match inline {
        pod_fmt::Inline::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s),

        pod_fmt::Inline::Bold(children) => {
            Node::new(node::STRONG).children(convert_inlines(children))
        }

        pod_fmt::Inline::Italic(children) => {
            Node::new(node::EMPHASIS).children(convert_inlines(children))
        }

        pod_fmt::Inline::Underline(children) => {
            Node::new(node::UNDERLINE).children(convert_inlines(children))
        }

        pod_fmt::Inline::Code(content) => Node::new(node::CODE).prop(prop::CONTENT, content),

        pod_fmt::Inline::Link { url, label } => {
            let text_node = Node::new(node::TEXT).prop(prop::CONTENT, label);
            Node::new(node::LINK)
                .prop(prop::URL, url)
                .children(vec![text_node])
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
        let doc = parse_str("=head1 NAME\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::HEADING);
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse_str("=head2 DESCRIPTION\n");
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(2));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("=pod\n\nThis is a paragraph.\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("=pod\n\nThis is B<bold> text.\n");
        let para = &doc.content.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::STRONG)
        );
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("=pod\n\nThis is I<italic> text.\n");
        let para = &doc.content.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::EMPHASIS)
        );
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("=pod\n\nUse C<my $var> here.\n");
        let para = &doc.content.children[0];
        assert!(para.children.iter().any(|n| n.kind.as_str() == node::CODE));
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("=pod\n\nSee L<perlpod> for details.\n");
        let para = &doc.content.children[0];
        assert!(para.children.iter().any(|n| n.kind.as_str() == node::LINK));
    }

    #[test]
    fn test_parse_verbatim() {
        let doc = parse_str("=pod\n\n    print \"Hello\";\n");
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::CODE_BLOCK);
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_str("=over\n\n=item * First\n\n=item * Second\n\n=back\n");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_escape() {
        let doc = parse_str("=pod\n\nE<lt>tag E<gt>\n");
        let para = &doc.content.children[0];
        let text = para.children[0].props.get_str(prop::CONTENT).unwrap_or("");
        assert!(text.contains('<'));
        assert!(text.contains('>'));
    }

    #[test]
    fn test_parse_double_brackets() {
        let doc = parse_str("=pod\n\nC<< $a <=> $b >>\n");
        let para = &doc.content.children[0];
        let code = para.children.iter().find(|n| n.kind.as_str() == node::CODE);
        assert!(code.is_some());
        let content = code.unwrap().props.get_str(prop::CONTENT).unwrap_or("");
        assert!(content.contains("<=>"));
    }
}
