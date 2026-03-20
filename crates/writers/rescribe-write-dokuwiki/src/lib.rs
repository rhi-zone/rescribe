//! DokuWiki writer for rescribe.
//!
//! Emits documents as DokuWiki markup.
//! Thin adapter over the standalone `dokuwiki` crate.

use dokuwiki::{Block as FmtBlock, DokuwikiDoc, Inline as FmtInline, build as fmt_build};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Emit a document as DokuWiki.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as DokuWiki with custom options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut warnings = Vec::new();
    let blocks = convert_nodes(&doc.content.children, &mut warnings);
    let fmt_doc = DokuwikiDoc { blocks };
    let output = fmt_build(&fmt_doc);

    Ok(ConversionResult::with_warnings(
        output.into_bytes(),
        warnings,
    ))
}

fn convert_nodes(nodes: &[Node], warnings: &mut Vec<FidelityWarning>) -> Vec<FmtBlock> {
    nodes
        .iter()
        .filter_map(|n| convert_node(n, warnings))
        .collect()
}

fn convert_node(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Option<FmtBlock> {
    match node.kind.as_str() {
        node::DOCUMENT => {
            let children = convert_nodes(&node.children, warnings);
            if children.is_empty() {
                None
            } else if children.len() == 1 {
                Some(children.into_iter().next().unwrap())
            } else {
                // Wrap multiple top-level blocks; shouldn't happen but just in case
                Some(FmtBlock::Paragraph {
                    inlines: vec![],
                    span: dokuwiki::Span::NONE,
                })
            }
        }

        node::PARAGRAPH => {
            let inlines = node
                .children
                .iter()
                .filter_map(|n| convert_inline(n, warnings))
                .collect();
            Some(FmtBlock::Paragraph { inlines, span: dokuwiki::Span::NONE })
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
            let inlines = node
                .children
                .iter()
                .filter_map(|n| convert_inline(n, warnings))
                .collect();
            Some(FmtBlock::Heading { level, inlines, span: dokuwiki::Span::NONE })
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let language = node.props.get_str(prop::LANGUAGE).map(|s| s.to_string());
            Some(FmtBlock::CodeBlock { language, content, span: dokuwiki::Span::NONE })
        }

        node::BLOCKQUOTE => {
            let children = convert_nodes(&node.children, warnings);
            Some(FmtBlock::Blockquote { children, span: dokuwiki::Span::NONE })
        }

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let mut items = Vec::new();
            for child in &node.children {
                if child.kind.as_str() == node::LIST_ITEM {
                    let item_blocks = convert_nodes(&child.children, warnings);
                    items.push(item_blocks);
                }
            }
            Some(FmtBlock::List { ordered, items, span: dokuwiki::Span::NONE })
        }

        node::HORIZONTAL_RULE => Some(FmtBlock::HorizontalRule(dokuwiki::Span::NONE)),

        node::LIST_ITEM
        | node::TABLE
        | node::TABLE_ROW
        | node::TABLE_CELL
        | node::TABLE_HEAD
        | node::TABLE_BODY
        | node::TABLE_FOOT
        | node::FIGURE
        | node::DIV
        | node::SPAN
        | node::RAW_BLOCK
        | node::RAW_INLINE
        | node::DEFINITION_LIST
        | node::DEFINITION_TERM
        | node::DEFINITION_DESC => {
            // Try to preserve content where possible
            if child_is_simple(&node.children) {
                let inlines: Vec<FmtInline> = node
                    .children
                    .iter()
                    .filter_map(|n| convert_inline(n, warnings))
                    .collect();
                if !inlines.is_empty() {
                    return Some(FmtBlock::Paragraph {
                        inlines,
                        span: dokuwiki::Span::NONE,
                    });
                }
            }
            None
        }

        _ => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                format!(
                    "Unsupported block type for DokuWiki: {}",
                    node.kind.as_str()
                ),
            ));
            None
        }
    }
}

fn child_is_simple(children: &[Node]) -> bool {
    !children.is_empty()
        && children.iter().all(|n| {
            matches!(
                n.kind.as_str(),
                node::TEXT
                    | node::EMPHASIS
                    | node::STRONG
                    | node::CODE
                    | node::LINK
                    | node::IMAGE
                    | node::LINE_BREAK
                    | node::SOFT_BREAK
                    | node::UNDERLINE
            )
        })
}

fn convert_inline(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Option<FmtInline> {
    match node.kind.as_str() {
        node::TEXT => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(FmtInline::Text(content, dokuwiki::Span::NONE))
        }

        node::EMPHASIS => {
            let children = node
                .children
                .iter()
                .filter_map(|n| convert_inline(n, warnings))
                .collect();
            Some(FmtInline::Italic(children, dokuwiki::Span::NONE))
        }

        node::STRONG => {
            let children = node
                .children
                .iter()
                .filter_map(|n| convert_inline(n, warnings))
                .collect();
            Some(FmtInline::Bold(children, dokuwiki::Span::NONE))
        }

        node::UNDERLINE => {
            let children = node
                .children
                .iter()
                .filter_map(|n| convert_inline(n, warnings))
                .collect();
            Some(FmtInline::Underline(children, dokuwiki::Span::NONE))
        }

        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Some(FmtInline::Code(content, dokuwiki::Span::NONE))
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let children = node
                .children
                .iter()
                .filter_map(|n| convert_inline(n, warnings))
                .collect();
            Some(FmtInline::Link { url, children, span: dokuwiki::Span::NONE })
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).map(|s| s.to_string());
            Some(FmtInline::Image { url, alt, span: dokuwiki::Span::NONE })
        }

        node::LINE_BREAK => Some(FmtInline::LineBreak(dokuwiki::Span::NONE)),
        node::SOFT_BREAK => Some(FmtInline::SoftBreak(dokuwiki::Span::NONE)),

        node::STRIKEOUT
        | node::SUBSCRIPT
        | node::SUPERSCRIPT
        | node::SMALL_CAPS
        | node::QUOTED
        | node::FOOTNOTE_REF
        | node::FOOTNOTE_DEF => {
            // Fall back to rendering children as text
            let children: Vec<FmtInline> = node
                .children
                .iter()
                .filter_map(|n| convert_inline(n, warnings))
                .collect();
            if children.is_empty() {
                Some(FmtInline::Text(
                    format!("[{}]", node.kind.as_str()),
                    dokuwiki::Span::NONE,
                ))
            } else {
                Some(FmtInline::Bold(children, dokuwiki::Span::NONE))
            }
        }

        _ => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                format!(
                    "Unsupported inline type for DokuWiki: {}",
                    node.kind.as_str()
                ),
            ));
            None
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
        assert!(output.contains("====== Title ======"));
    }

    #[test]
    fn test_emit_heading_level2() {
        let doc = doc(|d| d.heading(2, |h| h.text("Subtitle")));
        let output = emit_str(&doc);
        assert!(output.contains("===== Subtitle ====="));
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
        let output = emit_str(&doc);
        assert!(output.contains("//italic//"));
    }

    #[test]
    fn test_emit_strong() {
        let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
        let output = emit_str(&doc);
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_emit_code() {
        let doc = doc(|d| d.para(|p| p.code("code")));
        let output = emit_str(&doc);
        assert!(output.contains("''code''"));
    }

    #[test]
    fn test_emit_link() {
        let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
        let output = emit_str(&doc);
        assert!(output.contains("[[https://example.com|click]]"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = doc(|d| d.code_block_lang("print('hi')", "python"));
        let output = emit_str(&doc);
        assert!(output.contains("<code python>"));
        assert!(output.contains("print('hi')"));
        assert!(output.contains("</code>"));
    }

    #[test]
    fn test_emit_list() {
        let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
        let output = emit_str(&doc);
        assert!(output.contains("  * one"));
        assert!(output.contains("  * two"));
    }
}
