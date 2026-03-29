//! Muse markup writer for rescribe.
//!
//! Emits documents as Emacs Muse markup.

use muse_fmt::{Block, Inline, MuseDoc, Span};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document as Muse markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Muse markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    // Convert rescribe nodes to muse blocks
    let blocks = convert_nodes_to_blocks(&doc.content.children);

    // Build using the format-specific crate
    let muse_doc = MuseDoc {
        blocks,
        span: Span::NONE,
        ..Default::default()
    };
    let output = muse_fmt::build(&muse_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn convert_nodes_to_blocks(nodes: &[Node]) -> Vec<Block> {
    nodes.iter().map(convert_node_to_block).collect()
}

fn convert_node_to_block(node: &Node) -> Block {
    match node.kind.as_str() {
        rescribe_std::node::DOCUMENT => {
            // Flatten document, just process children
            // This shouldn't normally happen at top level
            let children: Vec<Block> =
                node.children.iter().map(convert_node_to_block).collect();
            // Return first block or empty paragraph
            children
                .into_iter()
                .next()
                .unwrap_or_else(|| Block::Paragraph { inlines: vec![], span: Span::NONE })
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(5) as u8;
            let inlines = convert_nodes_to_inlines(&node.children);
            Block::Heading { level, inlines, span: Span::NONE }
        }

        node::PARAGRAPH => {
            let inlines = convert_nodes_to_inlines(&node.children);
            Block::Paragraph { inlines, span: Span::NONE }
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Block::CodeBlock { content, span: Span::NONE }
        }

        node::BLOCKQUOTE => {
            let children = convert_nodes_to_blocks(&node.children);
            Block::Blockquote { children, span: Span::NONE }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<Block>> = node
                .children
                .iter()
                .filter(|n| n.kind.as_str() == node::LIST_ITEM)
                .map(|n| convert_nodes_to_blocks(&n.children))
                .collect();
            Block::List { ordered, items, span: Span::NONE }
        }

        node::DEFINITION_LIST => {
            let mut items = Vec::new();
            let mut i = 0;
            while i < node.children.len() {
                if node.children[i].kind.as_str() == node::DEFINITION_TERM {
                    let term_inlines = convert_nodes_to_inlines(&node.children[i].children);
                    let mut desc_blocks = Vec::new();
                    if i + 1 < node.children.len()
                        && node.children[i + 1].kind.as_str() == node::DEFINITION_DESC
                    {
                        desc_blocks = convert_nodes_to_blocks(&node.children[i + 1].children);
                        i += 1;
                    }
                    items.push((term_inlines, desc_blocks));
                }
                i += 1;
            }
            Block::DefinitionList { items, span: Span::NONE }
        }

        node::HORIZONTAL_RULE => Block::HorizontalRule { span: Span::NONE },

        node::DIV | node::SPAN | node::FIGURE => {
            // Containers that pass through to their children
            let children = convert_nodes_to_blocks(&node.children);
            // Return first block or empty paragraph
            children
                .into_iter()
                .next()
                .unwrap_or_else(|| Block::Paragraph { inlines: vec![], span: Span::NONE })
        }

        // Inline nodes at block level (shouldn't happen, but handle them)
        node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK => {
            let inlines = vec![convert_node_to_inline(node)];
            Block::Paragraph { inlines, span: Span::NONE }
        }

        _ => {
            // Unknown block type, process children
            let children = convert_nodes_to_blocks(&node.children);
            children
                .into_iter()
                .next()
                .unwrap_or_else(|| Block::Paragraph { inlines: vec![], span: Span::NONE })
        }
    }
}

fn convert_nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
    nodes.iter().map(convert_node_to_inline).collect()
}

fn convert_node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text(content, Span::NONE)
        }

        node::STRONG => {
            let children = convert_nodes_to_inlines(&node.children);
            Inline::Bold(children, Span::NONE)
        }

        node::EMPHASIS => {
            let children = convert_nodes_to_inlines(&node.children);
            Inline::Italic(children, Span::NONE)
        }

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Code(content, Span::NONE)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let children = convert_nodes_to_inlines(&node.children);
            Inline::Link { url, children, span: Span::NONE }
        }

        node::STRIKEOUT => {
            let children = convert_nodes_to_inlines(&node.children);
            Inline::Strikethrough(children, Span::NONE)
        }

        node::UNDERLINE => {
            let children = convert_nodes_to_inlines(&node.children);
            Inline::Underline(children, Span::NONE)
        }

        node::SUBSCRIPT => {
            let children = convert_nodes_to_inlines(&node.children);
            Inline::Subscript(children, Span::NONE)
        }

        node::SUPERSCRIPT => {
            let children = convert_nodes_to_inlines(&node.children);
            Inline::Superscript(children, Span::NONE)
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
            Inline::Image {
                src: url,
                alt,
                span: Span::NONE,
            }
        }

        node::LINE_BREAK => Inline::LineBreak(Span::NONE),

        node::SOFT_BREAK => Inline::Text(" ".to_string(), Span::NONE),

        _ => {
            // Unknown inline type, process children
            let children = convert_nodes_to_inlines(&node.children);
            if children.is_empty() {
                Inline::Text(String::new(), Span::NONE)
            } else {
                children.into_iter().next().unwrap()
            }
        }
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
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        let output = emit_str(&doc);
        assert!(output.contains("* Title"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("** Subtitle"));
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
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("emphasis"))));
        let output = emit_str(&doc);
        assert!(output.contains("*emphasis*"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("=code="));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[[https://example.com][click]]"));
    }

    #[test]
    fn test_emit_unordered_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains(" - one"));
        assert!(output.contains(" - two"));
    }

    #[test]
    fn test_emit_ordered_list() {
        let doc =
            doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let output = emit_str(&doc);
        assert!(output.contains(" 1. first"));
        assert!(output.contains(" 2. second"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("print hi"));
        let output = emit_str(&doc);
        assert!(output.contains("<example>"));
        assert!(output.contains("print hi"));
        assert!(output.contains("</example>"));
    }
}
