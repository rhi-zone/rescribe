//! Markua (Leanpub) writer for rescribe.
//!
//! Thin adapter from rescribe document model to standalone markua format crate.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document as Markua markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Markua markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let markua_blocks = convert_blocks(&doc.content.children);
    let markua_doc = markua::MarkuaDoc {
        blocks: markua_blocks,
    };
    let output = markua::build(&markua_doc);

    Ok(ConversionResult::ok(output.into_bytes()))
}

fn convert_blocks(nodes: &[Node]) -> Vec<markua::Block> {
    nodes.iter().map(convert_block).collect()
}

fn convert_block(node: &Node) -> markua::Block {
    match node.kind.as_str() {
        node::DOCUMENT => {
            if node.children.len() == 1 {
                convert_block(&node.children[0])
            } else {
                convert_blocks(&node.children)
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| markua::Block::Paragraph {
                        inlines: Vec::new(),
                    })
            }
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            markua::Block::Heading {
                level,
                inlines: convert_inlines(&node.children),
            }
        }

        node::PARAGRAPH => markua::Block::Paragraph {
            inlines: convert_inlines(&node.children),
        },

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            markua::Block::CodeBlock { content, language }
        }

        node::BLOCKQUOTE => markua::Block::Blockquote {
            children: convert_blocks(&node.children),
        },

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<markua::Block>> = node
                .children
                .iter()
                .map(|item_node| {
                    if item_node.kind.as_str() == node::LIST_ITEM {
                        convert_blocks(&item_node.children)
                    } else {
                        vec![convert_block(item_node)]
                    }
                })
                .collect();
            markua::Block::List { ordered, items }
        }

        node::TABLE => {
            let rows: Vec<markua::TableRow> = node
                .children
                .iter()
                .filter(|n| n.kind.as_str() == node::TABLE_ROW)
                .map(|row_node| {
                    let cells: Vec<Vec<markua::Inline>> = row_node
                        .children
                        .iter()
                        .map(|cell_node| convert_inlines(&cell_node.children))
                        .collect();
                    markua::TableRow { cells }
                })
                .collect();
            markua::Block::Table { rows }
        }

        node::HORIZONTAL_RULE => markua::Block::HorizontalRule,

        node::DIV => {
            if let Some(class) = node.props.get_str("class") {
                let block_type = class.to_string();
                let inlines: Vec<markua::Inline> = node
                    .children
                    .iter()
                    .flat_map(|child| {
                        if child.kind.as_str() == node::PARAGRAPH {
                            convert_inlines(&child.children)
                        } else {
                            Vec::new()
                        }
                    })
                    .collect();
                markua::Block::SpecialBlock {
                    block_type,
                    inlines,
                }
            } else {
                markua::Block::Paragraph {
                    inlines: convert_inlines(&node.children),
                }
            }
        }

        _ => markua::Block::Paragraph {
            inlines: convert_inlines(&node.children),
        },
    }
}

fn convert_inlines(nodes: &[Node]) -> Vec<markua::Inline> {
    nodes.iter().map(convert_inline).collect()
}

fn convert_inline(node: &Node) -> markua::Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            markua::Inline::Text(content)
        }

        node::STRONG => markua::Inline::Strong(convert_inlines(&node.children)),

        node::EMPHASIS => markua::Inline::Emphasis(convert_inlines(&node.children)),

        node::STRIKEOUT => markua::Inline::Strikethrough(convert_inlines(&node.children)),

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            markua::Inline::Code(content)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let children = convert_inlines(&node.children);
            markua::Inline::Link { url, children }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
            markua::Inline::Image { url, alt }
        }

        node::LINE_BREAK => markua::Inline::LineBreak,

        node::SOFT_BREAK => markua::Inline::SoftBreak,

        _ => markua::Inline::Text(String::new()),
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
        assert!(output.contains("# Title"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("## Subtitle"));
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
        assert!(output.contains("*italic*"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("`code`"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[click](https://example.com)"));
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
        assert!(output.contains("1. first"));
        assert!(output.contains("2. second"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block("print hi"));
        let output = emit_str(&doc);
        assert!(output.contains("```"));
        assert!(output.contains("print hi"));
    }

    #[test]
    fn test_emit_code_block_with_language() {
        let doc = doc(|d| d.code_block_lang("puts 'hello'", "ruby"));
        let output = emit_str(&doc);
        assert!(output.contains("```ruby"));
    }

    #[test]
    fn test_emit_blockquote() {
        let doc = doc(|d| d.blockquote(|b| b.para(|p| p.text("quoted"))));
        let output = emit_str(&doc);
        assert!(output.contains("> quoted"));
    }

    #[test]
    fn test_emit_aside() {
        let div = Node::new(node::DIV).prop("class", "aside").children(vec![
            Node::new(node::PARAGRAPH).children(vec![
                Node::new(node::TEXT).prop(prop::CONTENT, "This is an aside."),
            ]),
        ]);
        let root = Node::new(node::DOCUMENT).children(vec![div]);
        let doc = Document::new().with_content(root);
        let output = emit_str(&doc);
        assert!(output.contains("A> This is an aside."));
    }

    #[test]
    fn test_emit_warning() {
        let div = Node::new(node::DIV).prop("class", "warning").children(vec![
            Node::new(node::PARAGRAPH).children(vec![
                Node::new(node::TEXT).prop(prop::CONTENT, "Be careful!"),
            ]),
        ]);
        let root = Node::new(node::DOCUMENT).children(vec![div]);
        let doc = Document::new().with_content(root);
        let output = emit_str(&doc);
        assert!(output.contains("W> Be careful!"));
    }

    #[test]
    fn test_emit_scene_break() {
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
        assert!(output.contains("* * *"));
    }

    #[test]
    fn test_emit_image() {
        let mut root = Node::new(node::DOCUMENT);
        root.children.push(Node::new(node::PARAGRAPH).children(vec![
            Node::new(node::IMAGE)
                .prop(prop::URL, "image.png")
                .prop(prop::ALT, "Alt text"),
        ]));
        let doc = Document::new().with_content(root);
        let output = emit_str(&doc);
        assert!(output.contains("![Alt text](image.png)"));
    }
}
