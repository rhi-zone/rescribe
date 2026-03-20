//! BBCode writer for rescribe.
//!
//! Serializes rescribe's document IR to BBCode forum markup.
//! Uses `bbcode-fmt` for building, adapts from rescribe types.

use bbcode_fmt::{Block, Inline, Span, TableRow};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document to BBCode markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to BBCode markup with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut blocks = Vec::new();
    for child in &doc.content.children {
        if child.kind.as_str() != node::DOCUMENT {
            blocks.push(node_to_block(child));
        } else {
            for doc_child in &child.children {
                blocks.push(node_to_block(doc_child));
            }
        }
    }

    let bbcode_doc = bbcode_fmt::BbcodeDoc {
        blocks,
        span: bbcode_fmt::Span::NONE,
    };
    let output = bbcode_fmt::emit(&bbcode_doc);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn node_to_block(node: &Node) -> Block {
    match node.kind.as_str() {
        node::PARAGRAPH => {
            let inlines = node.children.iter().map(node_to_inline).collect();
            Block::Paragraph {
                inlines,
                span: Span::NONE,
            }
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Block::CodeBlock {
                content,
                span: Span::NONE,
            }
        }

        node::BLOCKQUOTE => {
            let children = node.children.iter().map(node_to_block).collect();
            Block::Blockquote {
                children,
                span: Span::NONE,
            }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                .map(|item| {
                    item.children
                        .iter()
                        .filter(|child| child.kind.as_str() == node::PARAGRAPH)
                        .flat_map(|para| {
                            para.children.iter().map(node_to_inline).collect::<Vec<_>>()
                        })
                        .collect()
                })
                .collect();
            Block::List {
                ordered,
                items,
                span: Span::NONE,
            }
        }

        node::TABLE => {
            let rows = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::TABLE_ROW)
                .map(|row| {
                    let cells = row
                        .children
                        .iter()
                        .map(|cell| {
                            let is_header = cell.kind.as_str() == node::TABLE_HEADER;
                            let inlines = cell.children.iter().map(node_to_inline).collect();
                            (is_header, inlines)
                        })
                        .collect();
                    TableRow {
                        cells,
                        span: Span::NONE,
                    }
                })
                .collect();
            Block::Table {
                rows,
                span: Span::NONE,
            }
        }

        node::HEADING => {
            // BBCode doesn't have native headings - use bold text in paragraph
            let inlines = node.children.iter().map(node_to_inline).collect();
            Block::Paragraph {
                inlines: vec![Inline::Bold(inlines, Span::NONE)],
                span: Span::NONE,
            }
        }

        node::DIV | node::SPAN | node::FIGURE => {
            // Transparent wrapper - emit children as paragraph
            let inlines = node.children.iter().map(node_to_inline).collect();
            Block::Paragraph {
                inlines,
                span: Span::NONE,
            }
        }

        _ => {
            // Fallback: treat as paragraph
            let inlines = node.children.iter().map(node_to_inline).collect();
            Block::Paragraph {
                inlines,
                span: Span::NONE,
            }
        }
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

        node::SUBSCRIPT => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Subscript(children, Span::NONE)
        }

        node::SUPERSCRIPT => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Superscript(children, Span::NONE)
        }

        node::LINE_BREAK => {
            // BBCode doesn't have explicit line break inline element
            // Use newline in text instead
            Inline::Text("\n".to_string(), Span::NONE)
        }

        node::SOFT_BREAK => Inline::Text(" ".to_string(), Span::NONE),

        node::SPAN => {
            let attr = node
                .props
                .get_str("style:color")
                .map(|_| "color".to_string())
                .unwrap_or_else(|| "color".to_string());
            let value = node
                .props
                .get_str("style:color")
                .or_else(|| node.props.get_str("style:size"))
                .unwrap_or("inherit")
                .to_string();
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Span {
                attr,
                value,
                children,
                span: Span::NONE,
            }
        }

        _ => {
            // Fallback: collect children
            let children: Vec<Inline> = node.children.iter().map(node_to_inline).collect();
            if children.is_empty() {
                Inline::Text("".to_string(), Span::NONE)
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                // Wrap in a container if multiple children
                Inline::Bold(children, Span::NONE)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        assert!(emit_str(&doc).contains("[b]bold[/b]"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        assert!(emit_str(&doc).contains("[i]italic[/i]"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("Example"))));
        assert!(emit_str(&doc).contains("[url=http://example.com]Example[/url]"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("[list]"));
        assert!(output.contains("[*]one"));
        assert!(output.contains("[/list]"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("print('hello')"));
        let output = emit_str(&doc);
        assert!(output.contains("[code]"));
        assert!(output.contains("print('hello')"));
        assert!(output.contains("[/code]"));
    }
}
