//! RTF (Rich Text Format) writer for rescribe.
//!
//! Thin adapter over [`rtf_fmt`]: maps the rescribe document model to
//! the `rtf_fmt` AST, then builds RTF output.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use rtf_fmt::{Block, Inline, RtfDoc, TableRow};

/// Emit a document as RTF.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as RTF with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let rtf = doc_to_rtf(doc);
    let output = rtf_fmt::build(&rtf);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn doc_to_rtf(doc: &Document) -> RtfDoc {
    RtfDoc {
        blocks: nodes_to_blocks(&doc.content.children),
    }
}

fn nodes_to_blocks(nodes: &[Node]) -> Vec<Block> {
    nodes.iter().flat_map(node_to_blocks).collect()
}

/// Convert a rescribe node to zero or more `Block`s.
fn node_to_blocks(node: &Node) -> Vec<Block> {
    match node.kind.as_str() {
        node::DOCUMENT => nodes_to_blocks(&node.children),

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            vec![Block::Heading {
                level,
                inlines: nodes_to_inlines(&node.children),
            }]
        }

        node::PARAGRAPH => vec![Block::Paragraph {
            inlines: nodes_to_inlines(&node.children),
        }],

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            vec![Block::CodeBlock { content }]
        }

        node::BLOCKQUOTE => vec![Block::Blockquote {
            children: nodes_to_blocks(&node.children),
        }],

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<Block>> = node
                .children
                .iter()
                .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                .map(|item| nodes_to_blocks(&item.children))
                .collect();
            vec![Block::List { ordered, items }]
        }

        node::LIST_ITEM => nodes_to_blocks(&node.children),

        node::TABLE => {
            let rows: Vec<TableRow> = node
                .children
                .iter()
                .filter(|r| {
                    r.kind.as_str() == node::TABLE_ROW || r.kind.as_str() == node::TABLE_HEADER
                })
                .map(|row| TableRow {
                    cells: row
                        .children
                        .iter()
                        .map(|cell| nodes_to_inlines(&cell.children))
                        .collect(),
                })
                .collect();
            vec![Block::Table { rows }]
        }

        node::HORIZONTAL_RULE => vec![Block::HorizontalRule],

        node::DIV | node::SPAN | node::FIGURE => nodes_to_blocks(&node.children),

        // Inline nodes at block level: wrap in a paragraph
        node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK => {
            vec![Block::Paragraph {
                inlines: nodes_to_inlines(std::slice::from_ref(node)),
            }]
        }

        _ => nodes_to_blocks(&node.children),
    }
}

fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
    nodes.iter().map(node_to_inline).collect()
}

fn node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text(s)
        }

        node::STRONG => Inline::Bold(nodes_to_inlines(&node.children)),

        node::EMPHASIS => Inline::Italic(nodes_to_inlines(&node.children)),

        node::UNDERLINE => Inline::Underline(nodes_to_inlines(&node.children)),

        node::STRIKEOUT => Inline::Strikethrough(nodes_to_inlines(&node.children)),

        node::CODE => {
            let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let mut children = nodes_to_inlines(&node.children);
            if !s.is_empty() {
                children.insert(0, Inline::Text(s));
            }
            Inline::Code(
                children
                    .iter()
                    .map(|i| match i {
                        Inline::Text(t) => t.clone(),
                        _ => String::new(),
                    })
                    .collect(),
            )
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            Inline::Link {
                url,
                children: nodes_to_inlines(&node.children),
            }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
            Inline::Image { url, alt }
        }

        node::LINE_BREAK => Inline::LineBreak,

        node::SOFT_BREAK => Inline::SoftBreak,

        node::SUPERSCRIPT => Inline::Superscript(nodes_to_inlines(&node.children)),

        node::SUBSCRIPT => Inline::Subscript(nodes_to_inlines(&node.children)),

        _ => {
            // Unknown inline: recurse into children, or emit empty text
            let children = nodes_to_inlines(&node.children);
            if children.is_empty() {
                Inline::Text(String::new())
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                // Wrap in a span-like bold to group — best effort
                Inline::Bold(children)
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
    fn test_emit_rtf_header() {
        let doc = doc(|d| d.para(|p| p.text("Hello")));
        let output = emit_str(&doc);
        assert!(output.starts_with("{\\rtf1"));
        assert!(output.ends_with('}'));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
        assert!(output.contains("\\par"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("{\\b bold}"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("{\\i italic}"));
    }

    #[test]
    fn test_emit_underline() {
        let doc = doc(|d| d.para(|p| p.underline(|u| u.text("underlined"))));
        let output = emit_str(&doc);
        assert!(output.contains("{\\ul underlined}"));
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        let output = emit_str(&doc);
        assert!(output.contains("\\b "));
        assert!(output.contains("Title"));
    }

    #[test]
    fn test_emit_escaped_chars() {
        let doc = doc(|d| d.para(|p| p.text("Open { and close }")));
        let output = emit_str(&doc);
        assert!(output.contains("\\{"));
        assert!(output.contains("\\}"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("HYPERLINK"));
        assert!(output.contains("http://example.com"));
        assert!(output.contains("click"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("\\bullet"));
        assert!(output.contains("one"));
        assert!(output.contains("two"));
    }
}
