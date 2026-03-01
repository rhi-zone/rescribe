//! VimWiki writer for rescribe.
//!
//! Emits documents as VimWiki markup.
//! Thin adapter over `vimwiki-fmt` crate.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use vimwiki_fmt::*;

/// Emit a document as VimWiki markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as VimWiki markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let blocks = doc.content.children.iter().map(node_to_block).collect();
    let vimwiki_doc = VimwikiDoc { blocks };
    let output = vimwiki_fmt::build(&vimwiki_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn node_to_block(node: &Node) -> Block {
    match node.kind.as_str() {
        node::PARAGRAPH => {
            let inlines = node.children.iter().map(node_to_inline).collect();
            Block::Paragraph { inlines }
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(6) as usize;
            let inlines = node.children.iter().map(node_to_inline).collect();
            Block::Heading { level, inlines }
        }

        node::CODE_BLOCK => {
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            Block::CodeBlock { language, content }
        }

        node::BLOCKQUOTE => {
            let mut inlines = Vec::new();
            for child in &node.children {
                if child.kind.as_str() == node::PARAGRAPH {
                    inlines.extend(child.children.iter().map(node_to_inline));
                }
            }
            Block::Blockquote { inlines }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .filter(|n| n.kind.as_str() == node::LIST_ITEM)
                .map(|item_node| {
                    let checked = item_node.props.get_bool("checked");
                    let mut inlines = Vec::new();
                    for child in &item_node.children {
                        if child.kind.as_str() == node::PARAGRAPH {
                            inlines.extend(child.children.iter().map(node_to_inline));
                        }
                    }
                    ListItem { checked, inlines }
                })
                .collect();

            Block::List { ordered, items }
        }

        node::TABLE => {
            let rows = node
                .children
                .iter()
                .filter(|n| n.kind.as_str() == node::TABLE_ROW)
                .map(|row_node| {
                    let cells = row_node
                        .children
                        .iter()
                        .filter(|n| n.kind.as_str() == node::TABLE_CELL)
                        .map(|cell_node| cell_node.children.iter().map(node_to_inline).collect())
                        .collect();
                    TableRow { cells }
                })
                .collect();

            Block::Table { rows }
        }

        node::HORIZONTAL_RULE => Block::HorizontalRule,

        // Fallback for unhandled block types
        _ => {
            let inlines = node.children.iter().map(node_to_inline).collect();
            Block::Paragraph { inlines }
        }
    }
}

fn node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            Inline::Text(content)
        }

        node::STRONG => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Bold(children)
        }

        node::EMPHASIS => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Italic(children)
        }

        node::STRIKEOUT => {
            let children = node.children.iter().map(node_to_inline).collect();
            Inline::Strikethrough(children)
        }

        node::CODE => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            Inline::Code(content)
        }

        node::LINK => {
            let url = node
                .props
                .get_str(prop::URL)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let label = if node.children.is_empty() {
                url.clone()
            } else {
                node.children
                    .iter()
                    .filter_map(|n| {
                        if n.kind.as_str() == node::TEXT {
                            n.props.get_str(prop::CONTENT).map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<String>()
            };
            Inline::Link { url, label }
        }

        node::IMAGE => {
            let url = node
                .props
                .get_str(prop::URL)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
            Inline::Image { url, alt }
        }

        // Fallback for inline nodes: wrap in text or return text representation
        _ => {
            let children: Vec<Inline> = node.children.iter().map(node_to_inline).collect();
            if children.is_empty() {
                Inline::Text(String::new())
            } else {
                // Wrap unhandled inline types
                Inline::Text(format!("[{}]", node.kind))
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
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("_italic_"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("`code`"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("MyPage", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[[MyPage|click]]"));
    }

    #[test]
    fn test_emit_unordered_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("* one"));
        assert!(output.contains("* two"));
    }

    #[test]
    fn test_emit_ordered_list() {
        let doc =
            doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
        let output = emit_str(&doc);
        assert!(output.contains("1. first"));
        assert!(output.contains("2. second"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("print hi"));
        let output = emit_str(&doc);
        assert!(output.contains("{{{"));
        assert!(output.contains("print hi"));
        assert!(output.contains("}}}"));
    }
}
