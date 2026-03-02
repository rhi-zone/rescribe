//! RTF (Rich Text Format) writer for rescribe.
//!
//! Thin adapter over [`rtf_fmt`]: maps the rescribe document model to
//! the `rtf_fmt` AST, then emits RTF output.

use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
use rescribe_std::{node, prop};
use rtf_fmt::{Align, Block, Inline, RtfDoc, Span, TableRow};

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
    let output = rtf_fmt::emit(&rtf);
    Ok(ConversionResult::ok(output.into_bytes()))
}

fn doc_to_rtf(doc: &Document) -> RtfDoc {
    RtfDoc {
        blocks: nodes_to_blocks(&doc.content.children),
        color_table: vec![],
        span: Span::NONE,
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
                span: Span::NONE,
            }]
        }

        node::PARAGRAPH => {
            let align = parse_align(node.props.get_str(prop::STYLE_ALIGN));
            let para_props = node
                .props
                .get_str("rtf:para-props")
                .unwrap_or("")
                .to_string();
            vec![Block::Paragraph {
                inlines: nodes_to_inlines(&node.children),
                align,
                para_props,
                span: Span::NONE,
            }]
        }

        node::CODE_BLOCK => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            vec![Block::CodeBlock {
                content,
                span: Span::NONE,
            }]
        }

        node::BLOCKQUOTE => vec![Block::Blockquote {
            children: nodes_to_blocks(&node.children),
            span: Span::NONE,
        }],

        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let items: Vec<Vec<Block>> = node
                .children
                .iter()
                .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                .map(|item| nodes_to_blocks(&item.children))
                .collect();
            vec![Block::List {
                ordered,
                items,
                span: Span::NONE,
            }]
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
                    span: Span::NONE,
                })
                .collect();
            vec![Block::Table {
                rows,
                span: Span::NONE,
            }]
        }

        node::HORIZONTAL_RULE => vec![Block::HorizontalRule { span: Span::NONE }],

        node::DIV | node::FIGURE => nodes_to_blocks(&node.children),

        node::SPAN => {
            // A SPAN at block level: treat as a paragraph wrapping inline content
            vec![Block::Paragraph {
                inlines: nodes_to_inlines(std::slice::from_ref(node)),
                align: Align::Default,
                para_props: String::new(),
                span: Span::NONE,
            }]
        }

        // Inline nodes at block level: wrap in a paragraph
        node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK => {
            vec![Block::Paragraph {
                inlines: nodes_to_inlines(std::slice::from_ref(node)),
                align: Align::Default,
                para_props: String::new(),
                span: Span::NONE,
            }]
        }

        _ => nodes_to_blocks(&node.children),
    }
}

/// Parse a CSS-style alignment string to an `Align` value.
fn parse_align(s: Option<&str>) -> Align {
    match s {
        Some("left") => Align::Left,
        Some("right") => Align::Right,
        Some("center") => Align::Center,
        Some("justify") => Align::Justify,
        _ => Align::Default,
    }
}

fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
    nodes.iter().map(node_to_inline).collect()
}

fn node_to_inline(node: &Node) -> Inline {
    match node.kind.as_str() {
        node::TEXT => {
            let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Text {
                text,
                span: Span::NONE,
            }
        }

        node::STRONG => Inline::Bold {
            children: nodes_to_inlines(&node.children),
            span: Span::NONE,
        },

        node::EMPHASIS => Inline::Italic {
            children: nodes_to_inlines(&node.children),
            span: Span::NONE,
        },

        node::UNDERLINE => Inline::Underline {
            children: nodes_to_inlines(&node.children),
            span: Span::NONE,
        },

        node::STRIKEOUT => Inline::Strikethrough {
            children: nodes_to_inlines(&node.children),
            span: Span::NONE,
        },

        node::CODE => {
            let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            Inline::Code {
                text,
                span: Span::NONE,
            }
        }

        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            Inline::Link {
                url,
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            }
        }

        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
            let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
            Inline::Image {
                url,
                alt,
                span: Span::NONE,
            }
        }

        node::LINE_BREAK => Inline::LineBreak { span: Span::NONE },

        node::SOFT_BREAK => Inline::SoftBreak { span: Span::NONE },

        node::SUPERSCRIPT => Inline::Superscript {
            children: nodes_to_inlines(&node.children),
            span: Span::NONE,
        },

        node::SUBSCRIPT => Inline::Subscript {
            children: nodes_to_inlines(&node.children),
            span: Span::NONE,
        },

        node::SPAN => {
            // Check for style:size → FontSize
            if let Some(size_str) = node.props.get_str(prop::STYLE_SIZE)
                && let Some(size) = parse_half_points(size_str)
            {
                return Inline::FontSize {
                    size,
                    children: nodes_to_inlines(&node.children),
                    span: Span::NONE,
                };
            }
            // Check for style:color → Color
            if let Some(color_str) = node.props.get_str(prop::STYLE_COLOR)
                && let Some((r, g, b)) = parse_hex_color(color_str)
            {
                return Inline::Color {
                    r,
                    g,
                    b,
                    children: nodes_to_inlines(&node.children),
                    span: Span::NONE,
                };
            }
            // Plain span: pass children through
            let children = nodes_to_inlines(&node.children);
            if children.is_empty() {
                Inline::Text {
                    text: String::new(),
                    span: Span::NONE,
                }
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                // Wrap in bold as a neutral container (best we can do without a generic span)
                Inline::Bold {
                    children,
                    span: Span::NONE,
                }
            }
        }

        _ => {
            let children = nodes_to_inlines(&node.children);
            if children.is_empty() {
                Inline::Text {
                    text: String::new(),
                    span: Span::NONE,
                }
            } else if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                Inline::Bold {
                    children,
                    span: Span::NONE,
                }
            }
        }
    }
}

/// Parse a font size string like `"12pt"` or `"12.5pt"` to half-points.
/// Returns `None` if parsing fails.
fn parse_half_points(s: &str) -> Option<u16> {
    let s = s.trim();
    if let Some(pt_str) = s.strip_suffix("pt") {
        let pts: f64 = pt_str.trim().parse().ok()?;
        Some((pts * 2.0).round() as u16)
    } else {
        None
    }
}

/// Parse a `#rrggbb` hex color string to `(r, g, b)`.
/// Returns `None` if the format is wrong.
fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim().strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
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

    #[test]
    fn test_parse_half_points() {
        assert_eq!(parse_half_points("12pt"), Some(24));
        assert_eq!(parse_half_points("12.5pt"), Some(25));
        assert_eq!(parse_half_points("24pt"), Some(48));
        assert_eq!(parse_half_points("bad"), None);
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#ff0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex_color("#00ff00"), Some((0, 255, 0)));
        assert_eq!(parse_hex_color("#0080ff"), Some((0, 128, 255)));
        assert_eq!(parse_hex_color("bad"), None);
    }

    #[test]
    fn test_parse_align() {
        assert_eq!(parse_align(Some("left")), Align::Left);
        assert_eq!(parse_align(Some("right")), Align::Right);
        assert_eq!(parse_align(Some("center")), Align::Center);
        assert_eq!(parse_align(Some("justify")), Align::Justify);
        assert_eq!(parse_align(None), Align::Default);
    }
}
