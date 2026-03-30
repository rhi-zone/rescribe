//! ANSI escape sequence reader for rescribe.
//!
//! Thin adapter converting ansi-fmt's AST to rescribe's document IR.
//!
//! ANSI is a terminal escape sequence format, not a document format.
//! The IR mapping separates:
//! - Control sequences (cursor move, erase, etc.) → top-level `raw_block` with `ansi:type` props
//! - Text content (text, hyperlinks, raw escapes) → grouped into `paragraph` nodes

use ansi_fmt::{AnsiDoc, AnsiNode, Color};
use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};

/// Parse ANSI-formatted text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse ANSI-formatted text with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (ansi_doc, _diagnostics) = ansi_fmt::parse(input.as_bytes());

    let blocks = build_document_nodes(&ansi_doc);

    let document = Document {
        content: Node::new(node::DOCUMENT).children(blocks),
        resources: Default::default(),
        metadata: Default::default(),
        source: None,
    };

    Ok(ConversionResult::ok(document))
}

/// Flush accumulated inline nodes as a paragraph, clearing the buffer.
fn flush_para(inline_buf: &mut Vec<&AnsiNode>, result: &mut Vec<Node>) {
    // Remove trailing newlines
    while matches!(inline_buf.last(), Some(AnsiNode::Newline { .. })) {
        inline_buf.pop();
    }
    if !inline_buf.is_empty() {
        let inlines: Vec<Node> = inline_buf.iter().map(|n| ansi_node_to_inline(n)).collect();
        result.push(Node::new(node::PARAGRAPH).children(inlines));
        inline_buf.clear();
    }
}

/// Build the top-level block list from AnsiDoc nodes.
///
/// - Control sequences (cursor, erase, etc.) → `raw_block` with `ansi:type` props
/// - Text, hyperlinks, raw escapes → accumulated into paragraphs (split on 2+ newlines)
fn build_document_nodes(doc: &AnsiDoc) -> Vec<Node> {
    let mut result: Vec<Node> = Vec::new();
    let mut inline_buf: Vec<&AnsiNode> = Vec::new();
    let mut consecutive_newlines: usize = 0;

    for node in &doc.nodes {
        match node {
            AnsiNode::Newline { .. } => {
                consecutive_newlines += 1;
                if consecutive_newlines == 1 {
                    // Single newline: keep in current paragraph as LINE_BREAK
                    inline_buf.push(node);
                } else if consecutive_newlines == 2 {
                    // Double newline: paragraph boundary — flush
                    flush_para(&mut inline_buf, &mut result);
                }
                // 3+ consecutive newlines: already flushed, ignore extras
            }

            AnsiNode::Text { .. } | AnsiNode::Hyperlink { .. } | AnsiNode::RawEscape { .. } => {
                consecutive_newlines = 0;
                inline_buf.push(node);
            }

            // Control sequences: flush pending paragraph, then emit raw_block
            _ => {
                consecutive_newlines = 0;
                flush_para(&mut inline_buf, &mut result);
                result.push(ansi_node_to_raw_block(node));
            }
        }
    }

    // Flush any remaining inline content
    flush_para(&mut inline_buf, &mut result);

    result
}

/// Convert a control-sequence AnsiNode to a top-level raw_block with ansi: props.
fn ansi_node_to_raw_block(n: &AnsiNode) -> Node {
    match n {
        AnsiNode::CursorMove { direction, count, .. } => {
            use ansi_fmt::CursorDirection;
            let type_str = match direction {
                CursorDirection::Up => "cursor-up",
                CursorDirection::Down => "cursor-down",
                CursorDirection::Forward => "cursor-forward",
                CursorDirection::Back => "cursor-back",
            };
            Node::new(node::RAW_BLOCK)
                .prop("ansi:type", type_str)
                .prop("ansi:count", *count as i64)
        }

        AnsiNode::CursorPosition { row, col, .. } => Node::new(node::RAW_BLOCK)
            .prop("ansi:type", "cursor-position")
            .prop("ansi:row", *row as i64)
            .prop("ansi:col", *col as i64),

        AnsiNode::EraseDisplay { mode, .. } => {
            use ansi_fmt::EraseMode;
            let mode_str = match mode {
                EraseMode::ToEnd => "to-end",
                EraseMode::ToBeginning => "to-beginning",
                EraseMode::All => "all",
            };
            Node::new(node::RAW_BLOCK)
                .prop("ansi:type", "erase-display")
                .prop("ansi:mode", mode_str)
        }

        AnsiNode::EraseLine { mode, .. } => {
            use ansi_fmt::EraseMode;
            let mode_str = match mode {
                EraseMode::ToEnd => "to-end",
                EraseMode::ToBeginning => "to-beginning",
                EraseMode::All => "all",
            };
            Node::new(node::RAW_BLOCK)
                .prop("ansi:type", "erase-line")
                .prop("ansi:mode", mode_str)
        }

        AnsiNode::CursorVisibility { visible, .. } => {
            let type_str = if *visible { "cursor-show" } else { "cursor-hide" };
            Node::new(node::RAW_BLOCK).prop("ansi:type", type_str)
        }

        AnsiNode::SaveCursor { .. } => {
            Node::new(node::RAW_BLOCK).prop("ansi:type", "save-cursor")
        }

        AnsiNode::RestoreCursor { .. } => {
            Node::new(node::RAW_BLOCK).prop("ansi:type", "restore-cursor")
        }

        AnsiNode::ScrollRegion { top, bottom, .. } => Node::new(node::RAW_BLOCK)
            .prop("ansi:type", "scroll-region")
            .prop("ansi:top", *top as i64)
            .prop("ansi:bottom", *bottom as i64),

        // Fallback: shouldn't happen since we route Text/Hyperlink/RawEscape to inline path
        _ => Node::new(node::RAW_BLOCK).prop(prop::FORMAT, "ansi"),
    }
}

/// Convert a single inline AnsiNode to a rescribe Node.
fn ansi_node_to_inline(n: &AnsiNode) -> Node {
    match n {
        AnsiNode::Text { text, style, .. } => {
            if style.is_empty() {
                return Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
            }

            // Build from innermost (text) outward.
            // Order (outermost → innermost): strong > emphasis > strikeout > underline > span > text
            let mut inner = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());

            // Non-semantic span properties (color, dim, blink, etc.)
            let needs_span = style.fg.is_some()
                || style.bg.is_some()
                || style.underline_color.is_some()
                || style.dim
                || style.blink
                || style.rapid_blink
                || style.reverse
                || style.hidden
                || style.overline
                || style.double_underline;

            if needs_span {
                let mut span = Node::new(node::SPAN);
                if let Some(ref fg) = style.fg {
                    span = span.prop("style:color", color_to_string(fg));
                }
                if let Some(ref bg) = style.bg {
                    span = span.prop("style:background-color", color_to_string(bg));
                }
                if let Some(ref uc) = style.underline_color {
                    span = span.prop("style:underline-color", color_to_string(uc));
                }
                if style.dim {
                    span = span.prop("style:dim", true);
                }
                if style.blink || style.rapid_blink {
                    span = span.prop("style:blink", true);
                }
                if style.reverse {
                    span = span.prop("style:reverse", true);
                }
                if style.hidden {
                    span = span.prop("style:hidden", true);
                }
                if style.overline {
                    span = span.prop("style:overline", true);
                }
                if style.double_underline {
                    span = span.prop("style:double-underline", true);
                }
                inner = span.child(inner);
            }

            // Semantic wrappers: applied innermost to outermost
            if style.underline {
                inner = Node::new(node::UNDERLINE).child(inner);
            }
            if style.strikethrough {
                inner = Node::new(node::STRIKEOUT).child(inner);
            }
            if style.italic {
                inner = Node::new(node::EMPHASIS).child(inner);
            }
            if style.bold {
                inner = Node::new(node::STRONG).child(inner);
            }

            inner
        }

        AnsiNode::Newline { .. } => Node::new(node::LINE_BREAK),

        AnsiNode::Hyperlink { url, text, .. } => Node::new(node::LINK)
            .prop(prop::URL, url.clone())
            .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

        // Raw escapes: preserve as raw_inline with format = "ansi"
        AnsiNode::RawEscape { content, .. } => Node::new(node::RAW_INLINE)
            .prop(prop::FORMAT, "ansi")
            .prop(prop::CONTENT, content.clone()),

        // Control sequences shouldn't reach here (handled in build_document_nodes),
        // but provide a safe fallback.
        _ => Node::new(node::RAW_INLINE).prop(prop::FORMAT, "ansi"),
    }
}

fn color_to_string(color: &Color) -> String {
    match color {
        Color::Standard(n) => {
            let names = [
                "ansi-black",
                "ansi-red",
                "ansi-green",
                "ansi-yellow",
                "ansi-blue",
                "ansi-magenta",
                "ansi-cyan",
                "ansi-white",
            ];
            names.get(*n as usize).copied().unwrap_or("ansi-unknown").to_string()
        }
        Color::Bright(n) => {
            let names = [
                "ansi-bright-black",
                "ansi-bright-red",
                "ansi-bright-green",
                "ansi-bright-yellow",
                "ansi-bright-blue",
                "ansi-bright-magenta",
                "ansi-bright-cyan",
                "ansi-bright-white",
            ];
            names.get(*n as usize).copied().unwrap_or("ansi-bright-unknown").to_string()
        }
        Color::Palette(n) => format!("ansi-palette-{}", n),
        Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        Color::Default => "ansi-default".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let result = parse("Hello world").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_bold() {
        let result = parse("\x1b[1mBold text\x1b[0m").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let result = parse("\x1b[3mItalic text\x1b[0m").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_underline() {
        let result = parse("\x1b[4mUnderlined\x1b[0m").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_combined_styles() {
        let result = parse("\x1b[1;3mBold and italic\x1b[0m").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_hyperlink() {
        let result = parse("\x1b]8;;https://example.com\x07click here\x1b]8;;\x07").unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_multiline() {
        let result = parse("Line one\n\nLine two").unwrap();
        // Should produce two paragraphs.
        assert_eq!(result.value.content.children.len(), 2);
    }

    #[test]
    fn test_cursor_move_becomes_raw_block() {
        let result = parse("\x1b[4D").unwrap();
        let doc = result.value;
        assert_eq!(doc.content.children.len(), 1);
        assert_eq!(doc.content.children[0].kind.as_str(), node::RAW_BLOCK);
        assert_eq!(
            doc.content.children[0].props.get_str("ansi:type"),
            Some("cursor-back")
        );
        assert_eq!(doc.content.children[0].props.get_int("ansi:count"), Some(4));
    }
}
