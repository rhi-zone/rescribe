//! Jira markup writer for rescribe.
//!
//! Emits documents as Jira/Confluence wiki markup.
//! Thin adapter over `jira-fmt` standalone library.

use jira_fmt::{Block, Inline, JiraDoc, Span, build as jira_build};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};

/// Emit a document as Jira markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Jira markup with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut blocks = Vec::new();
    for child in &doc.content.children {
        blocks.push(node_to_block(child));
    }

    let jira_doc = JiraDoc { blocks, span: Span::NONE };
    let output = jira_build(&jira_doc);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn node_to_block(node: &Node) -> Block {
    match node.kind.as_str() {
        node::PARAGRAPH => Block::Paragraph {
            inlines: nodes_to_inlines(&node.children),
            span: Span::NONE,
        },

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1).min(6) as u8;
            Block::Heading {
                level,
                inlines: nodes_to_inlines(&node.children),
                span: Span::NONE,
            }
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            Block::CodeBlock { content, language, span: Span::NONE }
        }

        node::BLOCKQUOTE => {
            let children: Vec<Block> = node.children.iter().map(node_to_block).collect();
            Block::Blockquote { children, span: Span::NONE }
        }

        node::DIV => {
            let is_panel = node
                .props
                .get_str("jira:type")
                .map(|s| s == "panel")
                .unwrap_or(false);
            if is_panel {
                let title = node.props.get_str("jira:panel-title").map(|s| s.to_owned());
                let children: Vec<Block> = node.children.iter().map(node_to_block).collect();
                Block::Panel { title, children, span: Span::NONE }
            } else {
                let children: Vec<Block> = node.children.iter().map(node_to_block).collect();
                Block::Blockquote { children, span: Span::NONE }
            }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let mut items = Vec::new();
            for child in &node.children {
                if child.kind.as_str() == node::LIST_ITEM {
                    let mut content = Vec::new();
                    for block_node in &child.children {
                        if block_node.kind.as_str() == node::LIST {
                            content.push(jira_fmt::ast::ListItemContent::NestedList(
                                node_to_block(block_node),
                            ));
                        } else {
                            // Treat any other block as inline content
                            content.push(jira_fmt::ast::ListItemContent::Inline(
                                nodes_to_inlines(&block_node.children),
                            ));
                        }
                    }
                    items.push(jira_fmt::ast::ListItem { children: content });
                }
            }
            Block::List { ordered, items, span: Span::NONE }
        }

        node::TABLE => {
            let mut rows = Vec::new();
            for child in &node.children {
                if child.kind.as_str() == node::TABLE_HEAD {
                    for row_node in &child.children {
                        rows.push(node_to_table_row(row_node));
                    }
                } else if child.kind.as_str() == node::TABLE_ROW {
                    rows.push(node_to_table_row(child));
                }
            }
            Block::Table { rows, span: Span::NONE }
        }

        node::HORIZONTAL_RULE => Block::HorizontalRule { span: Span::NONE },

        _ => Block::Paragraph {
            inlines: nodes_to_inlines(&node.children),
            span: Span::NONE,
        },
    }
}

fn node_to_table_row(node: &Node) -> jira_fmt::TableRow {
    let mut cells = Vec::new();
    for child in &node.children {
        let is_header = child.kind.as_str() == node::TABLE_HEADER;
        cells.push(jira_fmt::TableCell {
            is_header,
            inlines: nodes_to_inlines(&child.children),
            span: Span::NONE,
        });
    }
    jira_fmt::TableRow { cells, span: Span::NONE }
}

fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
    nodes.iter().map(node_to_inline).collect()
}

fn node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text(text, Span::NONE)
        }

        node::STRONG => Inline::Bold(nodes_to_inlines(&node.children), Span::NONE),

        node::EMPHASIS => Inline::Italic(nodes_to_inlines(&node.children), Span::NONE),

        node::UNDERLINE => Inline::Underline(nodes_to_inlines(&node.children), Span::NONE),

        node::STRIKEOUT => Inline::Strikethrough(nodes_to_inlines(&node.children), Span::NONE),

        node::CODE => {
            let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Code(text, Span::NONE)
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let children = nodes_to_inlines(&node.children);
            Inline::Link { url, children, span: Span::NONE }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
            Inline::Image { url, alt, span: Span::NONE }
        }

        node::SUPERSCRIPT => Inline::Superscript(nodes_to_inlines(&node.children), Span::NONE),

        node::SUBSCRIPT => Inline::Subscript(nodes_to_inlines(&node.children), Span::NONE),

        _ => {
            let children = nodes_to_inlines(&node.children);
            if children.is_empty() {
                Inline::Text(String::new(), Span::NONE)
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
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
        assert!(output.contains("h1. Title"));
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
        assert!(output.contains("{{code}}"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[click|https://example.com]"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
        let output = emit_str(&doc);
        assert!(output.contains("{code:python}"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("{code}"));
    }

    #[test]
    fn test_emit_list() {
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
        assert!(output.contains("# first"));
        assert!(output.contains("# second"));
    }
}
