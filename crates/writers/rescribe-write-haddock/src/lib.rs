//! Haddock markup writer for rescribe.
//!
//! Emits documents as Haddock documentation markup.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document as Haddock markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Haddock markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let blocks = convert_nodes(&doc.content.children);
    let haddock_doc = haddock_fmt::HaddockDoc { blocks };
    let output = haddock_fmt::build(&haddock_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn convert_nodes(nodes: &[Node]) -> Vec<haddock_fmt::Block> {
    nodes.iter().map(convert_node).collect()
}

fn convert_node(node: &Node) -> haddock_fmt::Block {
    match node.kind.as_str() {
        node::DOCUMENT => {
            let blocks: Vec<_> = node.children.iter().map(convert_node).collect();
            if blocks.is_empty() {
                haddock_fmt::Block::Paragraph { inlines: vec![] }
            } else {
                blocks.into_iter().next().unwrap()
            }
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(6) as u8;
            let inlines = convert_inlines(&node.children);
            haddock_fmt::Block::Heading { level, inlines }
        }

        node::PARAGRAPH => {
            let inlines = convert_inlines(&node.children);
            haddock_fmt::Block::Paragraph { inlines }
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            haddock_fmt::Block::CodeBlock { content }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<haddock_fmt::Inline>> = node
                .children
                .iter()
                .filter(|child| child.kind.as_str() == node::LIST_ITEM)
                .map(|item| {
                    let mut inlines = Vec::new();
                    for item_child in &item.children {
                        inlines.extend(convert_inlines(&item_child.children));
                    }
                    inlines
                })
                .collect();

            if ordered {
                haddock_fmt::Block::OrderedList { items }
            } else {
                haddock_fmt::Block::UnorderedList { items }
            }
        }

        node::DEFINITION_LIST => {
            let mut items = Vec::new();
            let mut i = 0;
            while i < node.children.len() {
                if node.children[i].kind.as_str() == node::DEFINITION_TERM {
                    let term_inlines = convert_inlines(&node.children[i].children);
                    let desc_inlines = if i + 1 < node.children.len()
                        && node.children[i + 1].kind.as_str() == node::DEFINITION_DESC
                    {
                        let desc_node = &node.children[i + 1];
                        let mut inlines = Vec::new();
                        for desc_child in &desc_node.children {
                            inlines.extend(convert_inlines(&desc_child.children));
                        }
                        inlines
                    } else {
                        Vec::new()
                    };
                    items.push((term_inlines, desc_inlines));
                    if i + 1 < node.children.len()
                        && node.children[i + 1].kind.as_str() == node::DEFINITION_DESC
                    {
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            haddock_fmt::Block::DefinitionList { items }
        }

        node::DIV | node::SPAN => {
            let blocks = convert_nodes(&node.children);
            if let Some(first) = blocks.first() {
                first.clone()
            } else {
                haddock_fmt::Block::Paragraph { inlines: vec![] }
            }
        }

        node::FIGURE => {
            let blocks = convert_nodes(&node.children);
            if let Some(first) = blocks.first() {
                first.clone()
            } else {
                haddock_fmt::Block::Paragraph { inlines: vec![] }
            }
        }

        _ => {
            let blocks = convert_nodes(&node.children);
            if let Some(first) = blocks.first() {
                first.clone()
            } else {
                haddock_fmt::Block::Paragraph { inlines: vec![] }
            }
        }
    }
}

fn convert_inlines(nodes: &[Node]) -> Vec<haddock_fmt::Inline> {
    nodes.iter().map(convert_inline).collect()
}

fn convert_inline(node: &Node) -> haddock_fmt::Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            haddock_fmt::Inline::Text(text)
        }

        node::CODE => {
            let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            haddock_fmt::Inline::Code(text)
        }

        node::STRONG => {
            let children = convert_inlines(&node.children);
            haddock_fmt::Inline::Strong(children)
        }

        node::EMPHASIS => {
            let children = convert_inlines(&node.children);
            haddock_fmt::Inline::Emphasis(children)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let text = extract_text(&node.children);
            haddock_fmt::Inline::Link { url, text }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            haddock_fmt::Inline::Link {
                url: url.clone(),
                text: url,
            }
        }

        _ => {
            let children = convert_inlines(&node.children);
            if !children.is_empty() {
                children.into_iter().next().unwrap()
            } else {
                haddock_fmt::Inline::Text(String::new())
            }
        }
    }
}

fn extract_text(nodes: &[Node]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node.kind.as_str() {
            node::TEXT => {
                if let Some(content) = node.props.get_str(prop::CONTENT) {
                    text.push_str(content);
                }
            }
            _ => {
                text.push_str(&extract_text(&node.children));
            }
        }
    }
    text
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
        assert!(output.contains("= Title"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("== Subtitle"));
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
        assert!(output.contains("__bold__"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("/italic/"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("@code@"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("\"click\"<https://example.com>"));
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
        assert!(output.contains("(1) first"));
        assert!(output.contains("(2) second"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("print hi"));
        let output = emit_str(&doc);
        assert!(output.contains("> print hi"));
    }
}
