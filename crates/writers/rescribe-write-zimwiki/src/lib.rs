//! ZimWiki writer for rescribe.
//!
//! Thin adapter layer converting rescribe Document model to zimwiki AST.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use zimwiki::{Block, Inline, ListItem, TableRow, ZimwikiDoc, build as build_zimwiki};

/// Emit a document as ZimWiki markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as ZimWiki markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut blocks = Vec::new();

    for node in &doc.content.children {
        if let Some(block) = convert_node(node) {
            blocks.push(block);
        }
    }

    let zimwiki_doc = ZimwikiDoc { blocks };
    let output = build_zimwiki(&zimwiki_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn convert_node(node: &Node) -> Option<Block> {
    match node.kind.as_str() {
        node::DOCUMENT => {
            for child in &node.children {
                if let Some(block) = convert_node(child) {
                    return Some(block);
                }
            }
            None
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            let inlines: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Block::Heading { level, inlines })
        }

        node::PARAGRAPH => {
            let inlines: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Block::Paragraph { inlines })
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(Block::CodeBlock { content })
        }

        node::BLOCKQUOTE => {
            let children: Vec<Block> = node.children.iter().filter_map(convert_node).collect();
            Some(Block::Blockquote { children })
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .filter(|n| n.kind.as_str() == node::LIST_ITEM)
                .map(|n| {
                    let children: Vec<Block> = n.children.iter().filter_map(convert_node).collect();
                    let checked = n.props.get_bool("checked");
                    ListItem { checked, children }
                })
                .collect();
            Some(Block::List { ordered, items })
        }

        node::TABLE => {
            let rows = node
                .children
                .iter()
                .filter(|n| n.kind.as_str() == node::TABLE_ROW)
                .map(|row| {
                    let cells: Vec<Vec<Inline>> = row
                        .children
                        .iter()
                        .map(|cell| cell.children.iter().filter_map(convert_inline).collect())
                        .collect();
                    TableRow { cells }
                })
                .collect();
            Some(Block::Table { rows })
        }

        node::HORIZONTAL_RULE => Some(Block::HorizontalRule),

        node::DIV | node::SPAN | node::FIGURE => {
            for child in &node.children {
                if let Some(block) = convert_node(child) {
                    return Some(block);
                }
            }
            None
        }

        _ => None,
    }
}

fn convert_inline(node: &Node) -> Option<Inline> {
    match node.kind.as_str() {
        node::TEXT => {
            let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(Inline::Text(s))
        }

        node::STRONG => {
            let children: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Inline::Bold(children))
        }

        node::EMPHASIS => {
            let children: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Inline::Italic(children))
        }

        node::UNDERLINE => {
            let children: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Inline::Underline(children))
        }

        node::STRIKEOUT => {
            let children: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Inline::Strikethrough(children))
        }

        node::SUBSCRIPT => {
            let children: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Inline::Subscript(children))
        }

        node::SUPERSCRIPT => {
            let children: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Inline::Superscript(children))
        }

        node::CODE => {
            let s = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(Inline::Code(s))
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let children: Vec<Inline> = node.children.iter().filter_map(convert_inline).collect();
            Some(Inline::Link { url, children })
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            Some(Inline::Image { url })
        }

        node::LINE_BREAK => Some(Inline::LineBreak),

        node::SOFT_BREAK => Some(Inline::SoftBreak),

        _ => None,
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
    fn test_emit_heading_level1() {
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        let output = emit_str(&doc);
        // Level 1 = 6 equals signs in ZimWiki
        assert!(output.contains("====== Title ======"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        // Level 2 = 5 equals signs in ZimWiki
        assert!(output.contains("===== Subtitle ====="));
    }

    #[test]
    fn test_emit_heading_level3() {
        let doc = doc(|d| d.heading(3, |h| h.text("Section")));
        let output = emit_str(&doc);
        // Level 3 = 4 equals signs in ZimWiki
        assert!(output.contains("==== Section ===="));
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
    fn test_emit_strikethrough() {
        let doc = doc(|d| d.para(|p| p.strike(|s| s.text("deleted"))));
        let output = emit_str(&doc);
        assert!(output.contains("~~deleted~~"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("''code''"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("MyPage", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[[MyPage|click]]"));
    }

    #[test]
    fn test_emit_link_no_label() {
        let doc = doc(|d| d.para(|p| p.link("MyPage", |l| l)));
        let output = emit_str(&doc);
        assert!(output.contains("[[MyPage]]"));
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
        assert!(output.contains("'''"));
        assert!(output.contains("print hi"));
    }

    #[test]
    fn test_emit_horizontal_rule() {
        let mut root = Node::new(node::DOCUMENT);
        root.children.push(
            Node::new(node::PARAGRAPH)
                .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, "before")]),
        );
        root.children.push(Node::new(node::HORIZONTAL_RULE));
        root.children.push(
            Node::new(node::PARAGRAPH)
                .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, "after")]),
        );
        let doc = Document::new().with_content(root);
        let output = emit_str(&doc);
        assert!(output.contains("----"));
    }

    #[test]
    fn test_emit_image() {
        let mut root = Node::new(node::DOCUMENT);
        root.children.push(
            Node::new(node::PARAGRAPH)
                .children(vec![Node::new(node::IMAGE).prop(prop::URL, "image.png")]),
        );
        let doc = Document::new().with_content(root);
        let output = emit_str(&doc);
        assert!(output.contains("{{image.png}}"));
    }
}
