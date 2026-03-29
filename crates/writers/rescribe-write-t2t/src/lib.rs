//! txt2tags (t2t) writer for rescribe.
//!
//! Thin adapter that uses the `t2t` crate to emit rescribe documents
//! as txt2tags markup.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use t2t::{Block, Inline, Span};

/// Emit a document as txt2tags markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as txt2tags markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut blocks = Vec::new();

    for node in &doc.content.children {
        if let Some(block) = node_to_block(node) {
            blocks.push(block);
        }
    }

    let t2t_doc = t2t::T2tDoc {
        blocks,
        ..Default::default()
    };
    let output = t2t::emit(&t2t_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn node_to_block(node: &Node) -> Option<Block> {
    match node.kind.as_str() {
        node::DOCUMENT => {
            // Document nodes should not be converted directly
            None
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(5) as u8;
            let numbered = node.props.get_bool("numbered").unwrap_or(false);
            let inlines = node.children.iter().map(node_to_inline).collect();

            Some(Block::Heading {
                level,
                numbered,
                inlines,
                span: Span::NONE,
            })
        }

        node::PARAGRAPH => {
            let inlines = node.children.iter().map(node_to_inline).collect();
            Some(Block::Paragraph {
                inlines,
                span: Span::NONE,
            })
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(Block::CodeBlock {
                content,
                span: Span::NONE,
            })
        }

        node::RAW_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(Block::RawBlock {
                content,
                span: Span::NONE,
            })
        }

        node::BLOCKQUOTE => {
            let children: Vec<Block> = node.children.iter().filter_map(node_to_block).collect();
            Some(Block::Blockquote {
                children,
                span: Span::NONE,
            })
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<Block>> = node
                .children
                .iter()
                .filter_map(|child| {
                    if child.kind.as_str() == node::LIST_ITEM {
                        let item_blocks: Vec<Block> =
                            child.children.iter().filter_map(node_to_block).collect();
                        if !item_blocks.is_empty() {
                            Some(item_blocks)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            Some(Block::List {
                ordered,
                items,
                span: Span::NONE,
            })
        }

        node::TABLE => {
            let rows = node
                .children
                .iter()
                .filter_map(|row_node| {
                    if row_node.kind.as_str() == node::TABLE_ROW {
                        let is_header = row_node
                            .children
                            .first()
                            .map(|c| c.kind.as_str() == node::TABLE_HEADER)
                            .unwrap_or(false);

                        let cells: Vec<Vec<Inline>> = row_node
                            .children
                            .iter()
                            .map(|cell| cell.children.iter().map(node_to_inline).collect())
                            .collect();

                        Some(t2t::TableRow {
                            cells,
                            is_header,
                            span: Span::NONE,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            Some(Block::Table {
                rows,
                span: Span::NONE,
            })
        }

        node::HORIZONTAL_RULE => Some(Block::HorizontalRule { span: Span::NONE }),

        node::DIV | node::SPAN | node::FIGURE => {
            // These container nodes should not emit themselves
            // but their children may be processed by the parent
            None
        }

        // Inline nodes at block level should be wrapped in paragraphs
        node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK | node::IMAGE => {
            let inlines = vec![node_to_inline(node)];
            Some(Block::Paragraph {
                inlines,
                span: Span::NONE,
            })
        }

        _ => None,
    }
}

fn node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text(content, Span::NONE)
        }

        node::STRONG => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Bold(children, Span::NONE)
        }

        node::EMPHASIS => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Italic(children, Span::NONE)
        }

        node::UNDERLINE => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Underline(children, Span::NONE)
        }

        node::STRIKEOUT => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Strikethrough(children, Span::NONE)
        }

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Code(content, Span::NONE)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Link {
                url,
                children,
                span: Span::NONE,
            }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            Inline::Image {
                url,
                span: Span::NONE,
            }
        }

        node::LINE_BREAK => Inline::LineBreak(Span::NONE),

        node::SOFT_BREAK => Inline::SoftBreak(Span::NONE),

        // For unsupported nodes, emit children as text
        _ => {
            let children: Vec<Inline> = node.children.iter().map(node_to_inline).collect();
            if children.is_empty() {
                Inline::Text(String::new(), Span::NONE)
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                // Multiple children, wrap in a generic container
                // Since t2t doesn't have generic containers, just concatenate
                Inline::Text(String::new(), Span::NONE)
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
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("== Subtitle =="));
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
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("//italic//"));
    }

    #[test]
    fn test_emit_underline() {
        let doc = doc(|d| d.para(|p| p.underline(|u| u.text("underlined"))));
        let output = emit_str(&doc);
        assert!(output.contains("__underlined__"));
    }

    #[test]
    fn test_emit_strikeout() {
        let doc = doc(|d| d.para(|p| p.strike(|s| s.text("strikeout"))));
        let output = emit_str(&doc);
        assert!(output.contains("--strikeout--"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("``code``"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[click http://example.com]"));
    }

    #[test]
    fn test_emit_unordered_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("- one"));
        assert!(output.contains("- two"));
    }

    #[test]
    fn test_emit_ordered_list() {
        let doc =
            doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let output = emit_str(&doc);
        assert!(output.contains("+ first"));
        assert!(output.contains("+ second"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("print hi"));
        let output = emit_str(&doc);
        assert!(output.contains("```"));
        assert!(output.contains("print hi"));
    }

    #[test]
    fn test_emit_horizontal_rule() {
        use rescribe_core::Node;
        let root = Node::new(node::DOCUMENT).children(vec![Node::new(node::HORIZONTAL_RULE)]);
        let document = Document::new().with_content(root);
        let output = emit_str(&document);
        assert!(output.contains("--------------------"));
    }
}
