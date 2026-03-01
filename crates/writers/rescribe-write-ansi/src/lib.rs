//! ANSI terminal writer for rescribe.
//!
//! Thin adapter converting rescribe's document IR to ansi-fmt's AST.

use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Emit a document as ANSI-formatted text.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as ANSI-formatted text with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut ctx = ConvertContext::new();

    let blocks = doc
        .content
        .children
        .iter()
        .map(|n| node_to_ansi_block(n, &mut ctx))
        .collect();
    let ansi_doc = ansi_fmt::AnsiDoc { blocks };

    let output = ansi_fmt::build(&ansi_doc);

    Ok(ConversionResult::with_warnings(
        output.into_bytes(),
        ctx.warnings,
    ))
}

struct ConvertContext {
    warnings: Vec<FidelityWarning>,
}

impl ConvertContext {
    fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    fn warn(&mut self, kind: WarningKind, msg: impl Into<String>) {
        self.warnings
            .push(FidelityWarning::new(Severity::Minor, kind, msg.into()));
    }
}

fn node_to_ansi_block(node: &Node, ctx: &mut ConvertContext) -> ansi_fmt::Block {
    match node.kind.as_str() {
        node::DOCUMENT => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_block(n, ctx))
                .collect::<Vec<_>>();
            if children.len() == 1 {
                children[0].clone()
            } else {
                ansi_fmt::Block::Div { children }
            }
        }

        node::PARAGRAPH => {
            let inlines = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Block::Paragraph { inlines }
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            let inlines = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Block::Heading { level, inlines }
        }

        node::CODE_BLOCK => {
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            ansi_fmt::Block::CodeBlock { language, content }
        }

        node::BLOCKQUOTE => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_block(n, ctx))
                .collect();
            ansi_fmt::Block::Blockquote { children }
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .filter(|n| n.kind.as_str() == node::LIST_ITEM)
                .map(|n| {
                    n.children
                        .iter()
                        .map(|c| node_to_ansi_block(c, ctx))
                        .collect()
                })
                .collect();
            ansi_fmt::Block::List { ordered, items }
        }

        node::LIST_ITEM => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_block(n, ctx))
                .collect();
            ansi_fmt::Block::ListItem { children }
        }

        node::TABLE => {
            let rows: Vec<ansi_fmt::TableRow> = node
                .children
                .iter()
                .filter_map(|n| {
                    if n.kind.as_str() == node::TABLE_ROW {
                        match node_to_table_row(n, ctx) {
                            ansi_fmt::Block::TableRow { cells } => {
                                Some(ansi_fmt::TableRow { cells })
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();
            ansi_fmt::Block::Table { rows }
        }

        node::TABLE_ROW => node_to_table_row(node, ctx),

        node::TABLE_CELL | node::TABLE_HEADER => {
            let inlines = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Block::TableCell { inlines }
        }

        node::TABLE_HEAD => {
            let cells = node
                .children
                .iter()
                .filter_map(|n| {
                    if n.kind.as_str() == node::TABLE_CELL {
                        Some(ansi_fmt::TableCell {
                            inlines: n
                                .children
                                .iter()
                                .map(|c| node_to_ansi_inline(c, ctx))
                                .collect(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            ansi_fmt::Block::TableHeader { cells }
        }

        node::TABLE_BODY => {
            let rows: Vec<ansi_fmt::TableRow> = node
                .children
                .iter()
                .filter_map(|n| {
                    if n.kind.as_str() == node::TABLE_ROW {
                        match node_to_table_row(n, ctx) {
                            ansi_fmt::Block::TableRow { cells } => {
                                Some(ansi_fmt::TableRow { cells })
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();
            ansi_fmt::Block::TableBody { rows }
        }

        node::TABLE_FOOT => {
            let rows: Vec<ansi_fmt::TableRow> = node
                .children
                .iter()
                .filter_map(|n| {
                    if n.kind.as_str() == node::TABLE_ROW {
                        match node_to_table_row(n, ctx) {
                            ansi_fmt::Block::TableRow { cells } => {
                                Some(ansi_fmt::TableRow { cells })
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();
            ansi_fmt::Block::TableFoot { rows }
        }

        node::HORIZONTAL_RULE => ansi_fmt::Block::HorizontalRule,

        node::DIV => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_block(n, ctx))
                .collect();
            ansi_fmt::Block::Div { children }
        }

        node::SPAN => {
            let inlines = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Block::Span { inlines }
        }

        node::RAW_BLOCK => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            ansi_fmt::Block::RawBlock { content }
        }

        node::RAW_INLINE => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            ansi_fmt::Block::RawInline { content }
        }

        node::DEFINITION_LIST => {
            let items = collect_definition_items(&node.children, ctx);
            ansi_fmt::Block::DefinitionList { items }
        }

        node::DEFINITION_TERM => {
            let inlines = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Block::DefinitionTerm { inlines }
        }

        node::DEFINITION_DESC => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_block(n, ctx))
                .collect();
            ansi_fmt::Block::DefinitionDesc { children }
        }

        node::FIGURE => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_block(n, ctx))
                .collect();
            ansi_fmt::Block::Figure { children }
        }

        _ => {
            ctx.warn(
                WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                format!("Unknown node type for ANSI: {}", node.kind.as_str()),
            );
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_block(n, ctx))
                .collect();
            ansi_fmt::Block::Div { children }
        }
    }
}

fn node_to_table_row(node: &Node, ctx: &mut ConvertContext) -> ansi_fmt::Block {
    let cells = node
        .children
        .iter()
        .filter(|n| {
            let k = n.kind.as_str();
            k == node::TABLE_CELL || k == node::TABLE_HEADER
        })
        .map(|n| ansi_fmt::TableCell {
            inlines: n
                .children
                .iter()
                .map(|c| node_to_ansi_inline(c, ctx))
                .collect(),
        })
        .collect();

    ansi_fmt::Block::TableRow { cells }
}

fn node_to_ansi_inline(node: &Node, ctx: &mut ConvertContext) -> ansi_fmt::Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            ansi_fmt::Inline::Text(content)
        }

        node::EMPHASIS => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Inline::Italic(children)
        }

        node::STRONG => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Inline::Bold(children)
        }

        node::STRIKEOUT => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Inline::Strikethrough(children)
        }

        node::UNDERLINE => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Inline::Underline(children)
        }

        node::CODE => {
            let content = node
                .props
                .get_str(prop::CONTENT)
                .map(|s| s.to_string())
                .unwrap_or_default();
            ansi_fmt::Inline::Text(content)
        }

        node::LINE_BREAK => ansi_fmt::Inline::Text("\n".to_string()),

        node::SOFT_BREAK => ansi_fmt::Inline::Text(" ".to_string()),

        node::LINK => {
            let url = node
                .props
                .get_str(prop::URL)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let mut children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect::<Vec<_>>();

            if !url.is_empty() {
                children.push(ansi_fmt::Inline::Text(format!(" ({})", url)));
            }

            if children.is_empty() {
                ansi_fmt::Inline::Text(url)
            } else if children.len() == 1 {
                children.pop().unwrap()
            } else {
                ansi_fmt::Inline::Text(format!("{} ({})", collect_inline_text(&children), url))
            }
        }

        node::IMAGE => {
            let alt = node
                .props
                .get_str(prop::ALT)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Image".to_string());
            ansi_fmt::Inline::Text(format!("[{}]", alt))
        }

        node::SUBSCRIPT | node::SUPERSCRIPT => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Inline::Italic(children)
        }

        node::SMALL_CAPS => {
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Inline::Bold(children)
        }

        node::QUOTED => {
            let quote_type = node.props.get_str(prop::QUOTE_TYPE).unwrap_or("double");
            let (left, right) = if quote_type == "single" {
                ("'", "'")
            } else {
                ("\u{201C}", "\u{201D}")
            };

            let inner = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect::<Vec<_>>();

            let mut result = vec![ansi_fmt::Inline::Text(left.to_string())];
            result.extend(inner);
            result.push(ansi_fmt::Inline::Text(right.to_string()));
            ansi_fmt::Inline::Text(collect_inline_text(&result))
        }

        node::FOOTNOTE_REF => {
            let label = node
                .props
                .get_str(prop::LABEL)
                .map(|s| s.to_string())
                .unwrap_or_default();
            ansi_fmt::Inline::Text(format!("[{}]", label))
        }

        node::FOOTNOTE_DEF => {
            let label = node
                .props
                .get_str(prop::LABEL)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect::<Vec<_>>();
            let text = collect_inline_text(&children);
            ansi_fmt::Inline::Text(format!("[{}] {}", label, text))
        }

        "math_inline" | "math_display" => {
            let source = node
                .props
                .get_str("math:source")
                .map(|s| s.to_string())
                .unwrap_or_default();
            ansi_fmt::Inline::Text(source)
        }

        _ => {
            ctx.warn(
                WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                format!("Unknown inline node type for ANSI: {}", node.kind.as_str()),
            );
            let children = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();
            ansi_fmt::Inline::Italic(children)
        }
    }
}

fn collect_definition_items(
    nodes: &[Node],
    ctx: &mut ConvertContext,
) -> Vec<ansi_fmt::DefinitionItem> {
    let mut items = Vec::new();
    let mut i = 0;

    while i < nodes.len() {
        let node = &nodes[i];
        if node.kind.as_str() == node::DEFINITION_TERM {
            let term = node
                .children
                .iter()
                .map(|n| node_to_ansi_inline(n, ctx))
                .collect();

            let mut desc = Vec::new();
            i += 1;

            while i < nodes.len() && nodes[i].kind.as_str() == node::DEFINITION_DESC {
                desc.extend(nodes[i].children.iter().map(|n| node_to_ansi_block(n, ctx)));
                i += 1;
            }

            items.push(ansi_fmt::DefinitionItem { term, desc });
        } else {
            i += 1;
        }
    }

    items
}

fn collect_inline_text(inlines: &[ansi_fmt::Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            ansi_fmt::Inline::Text(s) => text.push_str(s),
            ansi_fmt::Inline::Bold(c)
            | ansi_fmt::Inline::Italic(c)
            | ansi_fmt::Inline::Underline(c)
            | ansi_fmt::Inline::Strikethrough(c) => {
                text.push_str(&collect_inline_text(c));
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
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_heading() {
        let doc = doc(|d| d.heading(1, |h| h.text("Title")));
        let output = emit_str(&doc);
        assert!(output.contains("# Title"));
        assert!(output.contains("\x1b[1m"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("bold"));
        assert!(output.contains("\x1b[1m"));
    }

    #[test]
    fn test_emit_italic() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("italic"));
        assert!(output.contains("\x1b[3m"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("code"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block_lang("fn main() {}", "rust"));
        let output = emit_str(&doc);
        assert!(output.contains("rust"));
        assert!(output.contains("fn main() {}"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("click"));
        assert!(output.contains("https://example.com"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("one"));
        assert!(output.contains("two"));
        assert!(output.contains("•"));
    }

    #[test]
    fn test_emit_horizontal_rule() {
        let doc = doc(|d| d.hr());
        let output = emit_str(&doc);
        assert!(output.contains("───"));
    }
}
