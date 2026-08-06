//! Typst writer for rescribe.
//!
//! Thin IR→AST translator over `typst-fmt`'s `TypstDoc`/`Block`/`Inline` —
//! all Typst markup-generation logic lives in the rest of this crate (see
//! its module docs), not here. This module's only job is mapping rescribe
//! `Node`s onto `typst-fmt`'s domain-typed AST and collecting fidelity
//! warnings for anything it can't represent; the actual byte output for a
//! given `TypstDoc` is produced by `typst_fmt`'s `rescribe_format_api::Emit`
//! impl.
//!
//! The construct mapping (which rescribe node kind becomes which
//! `Block`/`Inline` variant, and the exact fidelity-warning behavior for
//! unrecognized kinds) is unchanged from the pre-collapse
//! `rescribe-write-typst` crate.

use crate::{Block, Inline, Span, TypstDoc};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
    WarningKind,
};
use rescribe_format_api::Emit as _;
use rescribe_std::{node, prop};

/// Emit a document as Typst.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as Typst with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut warnings = Vec::new();
    let blocks = convert_nodes_to_blocks(&doc.content.children, &mut warnings);
    let ast = TypstDoc {
        blocks,
        span: Span::NONE,
    };
    let bytes = ast.emit();
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

fn unsupported(node: &Node, warnings: &mut Vec<FidelityWarning>) {
    warnings.push(FidelityWarning::new(
        Severity::Minor,
        WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
        format!("Unknown node type for Typst: {}", node.kind.as_str()),
    ));
}

fn convert_nodes_to_blocks(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<Block> {
    nodes
        .iter()
        .flat_map(|n| convert_node_to_blocks(n, warnings))
        .collect()
}

fn convert_nodes_to_inlines(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<Inline> {
    nodes
        .iter()
        .flat_map(|n| convert_node_to_inlines(n, warnings))
        .collect()
}

/// Convert one `Node` to zero or more block-level constructs. Returns
/// multiple blocks for `DIV`/`DOCUMENT` (transparent containers, flattened
/// rather than wrapped — Typst markup has no generic block-container
/// construct) and zero for a non-`typst` `RAW_BLOCK`/`RAW_INLINE` (matches
/// the pre-existing writer's silent-skip behavior for that case).
fn convert_node_to_blocks(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Vec<Block> {
    match node.kind.as_str() {
        node::DOCUMENT | node::DIV | node::SPAN => {
            convert_nodes_to_blocks(&node.children, warnings)
        }
        node::PARAGRAPH => vec![Block::Paragraph(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1).clamp(1, 255) as u8;
            vec![Block::Heading {
                level,
                body: convert_nodes_to_inlines(&node.children, warnings),
            }]
        }
        node::CODE_BLOCK => {
            let lang = node.props.get_str(prop::LANGUAGE).map(str::to_owned);
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
            vec![Block::CodeBlock { lang, content }]
        }
        node::BLOCKQUOTE => vec![Block::Quote(convert_nodes_to_blocks(
            &node.children,
            warnings,
        ))],
        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items = node
                .children
                .iter()
                .map(|item| convert_list_item(item, warnings))
                .collect();
            vec![Block::List { ordered, items }]
        }
        node::TABLE => {
            let rows = table_rows(node, warnings);
            let columns = rows.first().map(|r| r.len()).unwrap_or(1);
            vec![Block::Table { columns, rows }]
        }
        node::FIGURE => {
            let mut body = None;
            let mut caption = None;
            for child in &node.children {
                if child.kind.as_str() == node::CAPTION {
                    caption = Some(convert_nodes_to_inlines(&child.children, warnings));
                } else if body.is_none() {
                    let blocks = convert_node_to_blocks(child, warnings);
                    body = blocks.into_iter().next().map(Box::new);
                }
            }
            vec![Block::Figure { body, caption }]
        }
        node::HORIZONTAL_RULE => vec![Block::HorizontalRule],
        node::DEFINITION_LIST => {
            let mut entries = Vec::new();
            let mut pending_term: Option<Vec<Inline>> = None;
            for child in &node.children {
                match child.kind.as_str() {
                    node::DEFINITION_TERM => {
                        if let Some(term) = pending_term.take() {
                            entries.push((term, Vec::new()));
                        }
                        pending_term = Some(convert_nodes_to_inlines(&child.children, warnings));
                    }
                    node::DEFINITION_DESC => {
                        let desc = convert_nodes_to_inlines(&child.children, warnings);
                        if let Some(term) = pending_term.take() {
                            entries.push((term, desc));
                        } else {
                            entries.push((Vec::new(), desc));
                        }
                    }
                    _ => {}
                }
            }
            if let Some(term) = pending_term.take() {
                entries.push((term, Vec::new()));
            }
            vec![Block::DefinitionList(entries)]
        }
        "math_display" => {
            let source = node.props.get_str("math:source").unwrap_or("").to_owned();
            vec![Block::MathDisplay(source)]
        }
        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_owned();
            vec![Block::Image { url }]
        }
        node::RAW_BLOCK | node::RAW_INLINE => {
            let format = node.props.get_str(prop::FORMAT).unwrap_or("");
            if format == "typst"
                && let Some(content) = node.props.get_str(prop::CONTENT)
            {
                vec![Block::Raw(content.to_owned())]
            } else {
                vec![]
            }
        }
        _ => {
            unsupported(node, warnings);
            convert_nodes_to_blocks(&node.children, warnings)
        }
    }
}

fn convert_list_item(item: &Node, warnings: &mut Vec<FidelityWarning>) -> Vec<Block> {
    convert_nodes_to_blocks(&item.children, warnings)
}

fn table_rows(table: &Node, warnings: &mut Vec<FidelityWarning>) -> Vec<Vec<Vec<Inline>>> {
    fn collect_rows(
        nodes: &[Node],
        warnings: &mut Vec<FidelityWarning>,
        out: &mut Vec<Vec<Vec<Inline>>>,
    ) {
        for n in nodes {
            match n.kind.as_str() {
                node::TABLE_HEAD | node::TABLE_BODY | node::TABLE_FOOT => {
                    collect_rows(&n.children, warnings, out);
                }
                node::TABLE_ROW => {
                    let cells = n
                        .children
                        .iter()
                        .map(|cell| convert_nodes_to_inlines(&cell.children, warnings))
                        .collect();
                    out.push(cells);
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    collect_rows(&table.children, warnings, &mut out);
    out
}

/// Convert one `Node` to zero or more inline-level constructs. Returns
/// multiple for `SPAN` (flattened, no generic inline-container construct in
/// Typst markup) and zero for a non-`typst` `RAW_INLINE`.
fn convert_node_to_inlines(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Vec<Inline> {
    match node.kind.as_str() {
        node::DIV | node::SPAN => convert_nodes_to_inlines(&node.children, warnings),
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
            vec![Inline::Text(content)]
        }
        node::EMPHASIS => vec![Inline::Emph(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::STRONG => vec![Inline::Strong(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::STRIKEOUT => vec![Inline::Strike(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::UNDERLINE => vec![Inline::Underline(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::SUBSCRIPT => vec![Inline::Subscript(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::SUPERSCRIPT => vec![Inline::Superscript(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
            vec![Inline::Code(content)]
        }
        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_owned();
            vec![Inline::Link {
                url,
                body: convert_nodes_to_inlines(&node.children, warnings),
            }]
        }
        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_owned();
            vec![Inline::Image { url }]
        }
        node::LINE_BREAK => vec![Inline::LineBreak],
        node::SOFT_BREAK => vec![Inline::Text("\n".to_owned())],
        node::FOOTNOTE_REF => {
            let label = node.props.get_str(prop::LABEL).unwrap_or("").to_owned();
            vec![Inline::Footnote(vec![Inline::Text(label)])]
        }
        node::FOOTNOTE_DEF => vec![Inline::Footnote(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::SMALL_CAPS => vec![Inline::SmallCaps(convert_nodes_to_inlines(
            &node.children,
            warnings,
        ))],
        node::QUOTED => {
            let double = node.props.get_str(prop::QUOTE_TYPE).unwrap_or("double") != "single";
            vec![Inline::Quoted {
                double,
                body: convert_nodes_to_inlines(&node.children, warnings),
            }]
        }
        "math_inline" => {
            let source = node.props.get_str("math:source").unwrap_or("").to_owned();
            vec![Inline::MathInline(source)]
        }
        "math_display" => {
            let source = node.props.get_str("math:source").unwrap_or("").to_owned();
            vec![Inline::MathDisplay(source)]
        }
        node::RAW_BLOCK | node::RAW_INLINE => {
            let format = node.props.get_str(prop::FORMAT).unwrap_or("");
            if format == "typst"
                && let Some(content) = node.props.get_str(prop::CONTENT)
            {
                vec![Inline::Raw(content.to_owned())]
            } else {
                vec![]
            }
        }
        _ => {
            unsupported(node, warnings);
            convert_nodes_to_inlines(&node.children, warnings)
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
    fn test_emit_paragraph() {
        let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
        let output = emit_str(&doc);
        assert!(output.contains("Hello, world!"));
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
    fn test_emit_emphasis() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("_italic_"));
    }

    #[test]
    fn test_emit_strong() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("*bold*"));
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
        assert!(output.contains("#link(\"https://example.com\")[click]"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
        let output = emit_str(&doc);
        assert!(output.contains("```python"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("```"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("- one"));
        assert!(output.contains("- two"));
    }

    #[test]
    fn test_emit_image() {
        let doc = doc(|d| d.para(|p| p.image("photo.png", "A photo")));
        let output = emit_str(&doc);
        assert!(output.contains("#image(\"photo.png\")"));
    }
}
