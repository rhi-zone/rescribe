//! POD (Plain Old Documentation) writer for rescribe.
//!
//! Emits documents as Perl POD markup.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document as POD markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as POD markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let blocks = doc
        .content
        .children
        .iter()
        .map(convert_node_to_block)
        .collect();
    let pod_doc = pod_fmt::PodDoc { blocks, span: pod_fmt::Span::NONE };
    let output = pod_fmt::build(&pod_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn convert_node_to_block(node: &Node) -> pod_fmt::Block {
    match node.kind.as_str() {
        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1).clamp(1, 6) as u32;
            let inlines = node.children.iter().map(convert_node_to_inline).collect();
            pod_fmt::Block::Heading { level, inlines, span: pod_fmt::Span::NONE }
        }

        node::PARAGRAPH => {
            let inlines = node.children.iter().map(convert_node_to_inline).collect();
            pod_fmt::Block::Paragraph { inlines, span: pod_fmt::Span::NONE }
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            pod_fmt::Block::CodeBlock { content, span: pod_fmt::Span::NONE }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .filter(|n| n.kind.as_str() == node::LIST_ITEM)
                .map(|item_node| {
                    item_node
                        .children
                        .iter()
                        .map(convert_node_to_block)
                        .collect()
                })
                .collect();
            pod_fmt::Block::List { ordered, items, span: pod_fmt::Span::NONE }
        }

        node::BLOCKQUOTE => {
            // POD doesn't have native blockquote; output as paragraph with children
            let inlines = node.children.iter().map(convert_node_to_inline).collect();
            pod_fmt::Block::Paragraph { inlines, span: pod_fmt::Span::NONE }
        }

        node::TABLE => {
            // POD doesn't have native tables; render as paragraph placeholder
            pod_fmt::Block::Paragraph {
                inlines: vec![pod_fmt::Inline::Text(
                    "[Table not supported in POD]".to_string(),
                    pod_fmt::Span::NONE,
                )],
                span: pod_fmt::Span::NONE,
            }
        }

        // Inline nodes at block level
        node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK => {
            let inlines = vec![convert_node_to_inline(node)];
            pod_fmt::Block::Paragraph { inlines, span: pod_fmt::Span::NONE }
        }

        node::DOCUMENT => {
            // Document nodes get skipped; children become blocks
            pod_fmt::Block::Paragraph { inlines: vec![], span: pod_fmt::Span::NONE }
        }

        _ => {
            // Default: collect text from children
            let inlines = node.children.iter().map(convert_node_to_inline).collect();
            pod_fmt::Block::Paragraph { inlines, span: pod_fmt::Span::NONE }
        }
    }
}

fn convert_node_to_inline(node: &Node) -> pod_fmt::Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            pod_fmt::Inline::Text(content, pod_fmt::Span::NONE)
        }

        node::STRONG => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            pod_fmt::Inline::Bold(children, pod_fmt::Span::NONE)
        }

        node::EMPHASIS => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            pod_fmt::Inline::Italic(children, pod_fmt::Span::NONE)
        }

        node::UNDERLINE => {
            let children = node.children.iter().map(convert_node_to_inline).collect();
            pod_fmt::Inline::Underline(children, pod_fmt::Span::NONE)
        }

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            pod_fmt::Inline::Code(content, pod_fmt::Span::NONE)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let mut label = String::new();
            collect_text(&node.children, &mut label);
            pod_fmt::Inline::Link { url, label, span: pod_fmt::Span::NONE }
        }

        node::STRIKEOUT => {
            // POD doesn't support strikethrough; render as text
            let children: Vec<pod_fmt::Inline> =
                node.children.iter().map(convert_node_to_inline).collect();
            if children.is_empty() {
                pod_fmt::Inline::Text(String::new(), pod_fmt::Span::NONE)
            } else {
                children.into_iter().next().unwrap()
            }
        }

        node::SUBSCRIPT | node::SUPERSCRIPT => {
            // POD doesn't support sub/superscript; collect text
            let mut text = String::new();
            collect_text(&node.children, &mut text);
            pod_fmt::Inline::Text(text, pod_fmt::Span::NONE)
        }

        node::LINE_BREAK => pod_fmt::Inline::Text("\n".to_string(), pod_fmt::Span::NONE),

        node::SOFT_BREAK => pod_fmt::Inline::Text(" ".to_string(), pod_fmt::Span::NONE),

        node::IMAGE => {
            let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
            pod_fmt::Inline::Text(format!("[Image: {}]", alt), pod_fmt::Span::NONE)
        }

        _ => {
            // Default: collect text from children
            let mut text = String::new();
            collect_text(&node.children, &mut text);
            pod_fmt::Inline::Text(text, pod_fmt::Span::NONE)
        }
    }
}

fn collect_text(nodes: &[Node], output: &mut String) {
    for node in nodes {
        if node.kind.as_str() == node::TEXT
            && let Some(content) = node.props.get_str(prop::CONTENT)
        {
            output.push_str(content);
        }
        collect_text(&node.children, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        let result = emit(doc).unwrap();
        String::from_utf8(result.value).unwrap()
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("NAME")));
        let output = emit_str(&doc);
        assert!(output.contains("=head1 NAME"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("DESCRIPTION")));
        let output = emit_str(&doc);
        assert!(output.contains("=head2 DESCRIPTION"));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("B<bold>"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("I<italic>"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("$var")));
        let output = emit_str(&doc);
        assert!(output.contains("C<$var>"));
    }

    #[test]
    fn test_emit_code_with_angle_brackets() {
        let doc = doc(|d| d.para(|p| p.code("$a <=> $b")));
        let output = emit_str(&doc);
        assert!(output.contains("C<< $a <=> $b >>"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("perlpod", |l| l.text("perlpod"))));
        let output = emit_str(&doc);
        assert!(output.contains("L<perlpod>"));
    }

    #[test]
    fn test_emit_link_with_label() {
        let doc = doc(|d| d.para(|p| p.link("perlpod", |l| l.text("documentation"))));
        let output = emit_str(&doc);
        assert!(output.contains("L<documentation|perlpod>"));
    }

    #[test]
    fn test_emit_unordered_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("=over"));
        assert!(output.contains("=item * one"));
        assert!(output.contains("=item * two"));
        assert!(output.contains("=back"));
    }

    #[test]
    fn test_emit_ordered_list() {
        let doc =
            doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let output = emit_str(&doc);
        assert!(output.contains("=item 1."));
        assert!(output.contains("=item 2."));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("print 'Hello';"));
        let output = emit_str(&doc);
        assert!(output.contains("    print 'Hello';"));
    }

    #[test]
    fn test_emit_pod_cut() {
        let doc = doc(|d| d.para(|p| p.text("Content")));
        let output = emit_str(&doc);
        assert!(output.starts_with("=pod"));
        assert!(output.ends_with("=cut\n"));
    }
}
