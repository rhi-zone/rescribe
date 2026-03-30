//! ANSI terminal writer for rescribe.
//!
//! Converts rescribe's document IR directly to ANSI escape sequences.
//! Does not go through the ansi-fmt AST — sequences are emitted directly
//! as bytes for efficiency.

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
    let mut ctx = EmitContext::new();
    for child in &doc.content.children {
        emit_block(child, &mut ctx);
    }
    Ok(ConversionResult::with_warnings(ctx.output, ctx.warnings))
}

// ── Context ───────────────────────────────────────────────────────────────────

struct EmitContext {
    output: Vec<u8>,
    warnings: Vec<FidelityWarning>,
}

impl EmitContext {
    fn new() -> Self {
        Self { output: Vec::new(), warnings: Vec::new() }
    }

    fn push(&mut self, s: &str) {
        self.output.extend_from_slice(s.as_bytes());
    }

    fn warn(&mut self, kind: WarningKind, msg: impl Into<String>) {
        self.warnings.push(FidelityWarning::new(Severity::Minor, kind, msg.into()));
    }
}

// ── Block emission ────────────────────────────────────────────────────────────

fn emit_block(n: &Node, ctx: &mut EmitContext) {
    match n.kind.as_str() {
        node::DOCUMENT => {
            for child in &n.children {
                emit_block(child, ctx);
            }
        }

        node::PARAGRAPH => {
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push("\n\n");
        }

        node::HEADING => {
            let level = n.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
            let prefix = "#".repeat(level);
            ctx.push("\x1b[1m");
            ctx.push(&prefix);
            ctx.push(" ");
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push("\x1b[0m");
            ctx.push("\n\n");
        }

        node::CODE_BLOCK => {
            let lang = n.props.get_str(prop::LANGUAGE).unwrap_or("");
            let content = n.props.get_str(prop::CONTENT).unwrap_or("");
            if !lang.is_empty() {
                ctx.push("\x1b[2m");
                ctx.push(lang);
                ctx.push("\x1b[0m");
                ctx.push("\n");
            }
            ctx.push(content);
            ctx.push("\n\n");
        }

        node::BLOCKQUOTE => {
            // Emit children, but prefix each line with "│ ".
            // Simple approach: collect content, then prefix lines.
            let mut sub = EmitContext::new();
            for child in &n.children {
                emit_block(child, &mut sub);
            }
            ctx.warnings.extend(sub.warnings);
            let text = String::from_utf8_lossy(&sub.output);
            for line in text.lines() {
                ctx.push("│ ");
                ctx.push(line);
                ctx.push("\n");
            }
            ctx.push("\n");
        }

        node::LIST => {
            let ordered = n.props.get_bool(prop::ORDERED).unwrap_or(false);
            let mut index = 1usize;
            for child in &n.children {
                if child.kind.as_str() == node::LIST_ITEM {
                    emit_list_item(child, ordered, index, ctx);
                    index += 1;
                } else {
                    emit_block(child, ctx);
                }
            }
            ctx.push("\n");
        }

        node::LIST_ITEM => {
            // Standalone list item (not inside LIST): use bullet.
            emit_list_item(n, false, 1, ctx);
        }

        node::TABLE => {
            for child in &n.children {
                emit_block(child, ctx);
            }
            ctx.push("\n");
        }

        node::TABLE_ROW => {
            for child in &n.children {
                ctx.push("│ ");
                for inline in &child.children {
                    emit_inline(inline, ctx);
                }
                ctx.push(" ");
            }
            ctx.push("│\n");
        }

        node::TABLE_CELL | node::TABLE_HEADER => {
            ctx.push("│ ");
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push(" │\n");
        }

        node::TABLE_HEAD | node::TABLE_BODY | node::TABLE_FOOT => {
            for child in &n.children {
                emit_block(child, ctx);
            }
        }

        node::HORIZONTAL_RULE => {
            ctx.push("───────────────────────────────────────────────────────\n\n");
        }

        node::DIV | node::FIGURE => {
            for child in &n.children {
                emit_block(child, ctx);
            }
        }

        node::SPAN => {
            // Block-level span: emit inline content + newline.
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push("\n\n");
        }

        node::RAW_BLOCK => {
            let format = n.props.get_str(prop::FORMAT).unwrap_or("");
            let content = n.props.get_str(prop::CONTENT).unwrap_or("");
            if format == "ansi" || format.is_empty() {
                ctx.push(content);
            }
            // Other formats: silently drop (they are format-specific raw content).
        }

        node::DEFINITION_LIST => {
            for child in &n.children {
                emit_block(child, ctx);
            }
            ctx.push("\n");
        }

        node::DEFINITION_TERM => {
            ctx.push("\x1b[1m");
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push("\x1b[0m");
            ctx.push("\n");
        }

        node::DEFINITION_DESC => {
            ctx.push("  ");
            for child in &n.children {
                emit_block(child, ctx);
            }
        }

        _ => {
            // Unknown block: try to render children, warn.
            let has_children = !n.children.is_empty();
            if has_children {
                for child in &n.children {
                    emit_block(child, ctx);
                }
            } else {
                // Leaf unknown node: try inline rendering.
                emit_inline(n, ctx);
                ctx.push("\n");
            }
            ctx.warn(
                WarningKind::UnsupportedNode(n.kind.as_str().to_string()),
                format!("Unknown block node type for ANSI: {}", n.kind.as_str()),
            );
        }
    }
}

fn emit_list_item(n: &Node, ordered: bool, index: usize, ctx: &mut EmitContext) {
    let bullet = if ordered { format!("{}. ", index) } else { "• ".to_string() };
    ctx.push(&bullet);
    for child in &n.children {
        // If child is a paragraph, emit its inlines without the trailing newlines.
        if child.kind.as_str() == node::PARAGRAPH {
            for inline in &child.children {
                emit_inline(inline, ctx);
            }
        } else {
            emit_inline(child, ctx);
        }
    }
    ctx.push("\n");
}

// ── Inline emission ───────────────────────────────────────────────────────────

fn emit_inline(n: &Node, ctx: &mut EmitContext) {
    match n.kind.as_str() {
        node::TEXT => {
            let content = n.props.get_str(prop::CONTENT).unwrap_or("");
            ctx.push(content);
        }

        node::STRONG => {
            ctx.push("\x1b[1m");
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push("\x1b[0m");
        }

        node::EMPHASIS => {
            ctx.push("\x1b[3m");
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push("\x1b[0m");
        }

        node::UNDERLINE => {
            ctx.push("\x1b[4m");
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push("\x1b[0m");
        }

        node::STRIKEOUT => {
            ctx.push("\x1b[9m");
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push("\x1b[0m");
        }

        node::CODE => {
            // Dim for inline code.
            ctx.push("\x1b[2m");
            let content = n.props.get_str(prop::CONTENT).unwrap_or("");
            ctx.push(content);
            ctx.push("\x1b[0m");
        }

        node::LINK => {
            let url = n.props.get_str(prop::URL).unwrap_or("");
            // Text content from children or CONTENT prop.
            let has_children = !n.children.is_empty();
            if has_children {
                for child in &n.children {
                    emit_inline(child, ctx);
                }
            } else if let Some(content) = n.props.get_str(prop::CONTENT) {
                ctx.push(content);
            } else {
                ctx.push(url);
            }
            if !url.is_empty() {
                ctx.push(" (");
                ctx.push(url);
                ctx.push(")");
            }
        }

        node::IMAGE => {
            let alt = n.props.get_str(prop::ALT).unwrap_or("Image");
            ctx.push("[");
            ctx.push(alt);
            ctx.push("]");
        }

        node::LINE_BREAK => {
            ctx.push("\n");
        }

        node::SOFT_BREAK => {
            ctx.push(" ");
        }

        node::SPAN => {
            // Apply style from properties.
            let bold = n.props.get_bool("style:bold").unwrap_or(false);
            let italic = n.props.get_bool("style:italic").unwrap_or(false);
            let underline = n.props.get_bool("style:underline").unwrap_or(false);
            let strikethrough = n.props.get_bool("style:strikethrough").unwrap_or(false);
            let dim = n.props.get_bool("style:dim").unwrap_or(false);
            let fg_color = n.props.get_str("style:color");

            let any_style = bold || italic || underline || strikethrough || dim || fg_color.is_some();

            if any_style {
                let mut codes: Vec<&str> = Vec::new();
                if bold { codes.push("1"); }
                if dim { codes.push("2"); }
                if italic { codes.push("3"); }
                if underline { codes.push("4"); }
                if strikethrough { codes.push("9"); }
                let sgr = format!("\x1b[{}m", codes.join(";"));
                ctx.push(&sgr);
            }

            // Content can be in prop or children.
            if let Some(content) = n.props.get_str(prop::CONTENT) {
                ctx.push(content);
            }
            for child in &n.children {
                emit_inline(child, ctx);
            }

            if any_style {
                ctx.push("\x1b[0m");
            }
        }

        node::RAW_INLINE => {
            let format = n.props.get_str(prop::FORMAT).unwrap_or("");
            let content = n.props.get_str(prop::CONTENT).unwrap_or("");
            if format == "ansi" || format.is_empty() {
                ctx.push(content);
            }
            // Other formats: silently drop.
        }

        node::SUBSCRIPT | node::SUPERSCRIPT => {
            // No terminal representation; emit content as-is.
            for child in &n.children {
                emit_inline(child, ctx);
            }
        }

        node::FOOTNOTE_REF => {
            let label = n.props.get_str(prop::LABEL).unwrap_or("");
            ctx.push("[");
            ctx.push(label);
            ctx.push("]");
        }

        node::FOOTNOTE_DEF => {
            let label = n.props.get_str(prop::LABEL).unwrap_or("");
            ctx.push("[");
            ctx.push(label);
            ctx.push("] ");
            for child in &n.children {
                emit_inline(child, ctx);
            }
        }

        node::SMALL_CAPS | node::ALL_CAPS => {
            for child in &n.children {
                emit_inline(child, ctx);
            }
        }

        node::QUOTED => {
            let quote_type = n.props.get_str(prop::QUOTE_TYPE).unwrap_or("double");
            let (left, right) = if quote_type == "single" {
                ("\u{2018}", "\u{2019}")
            } else {
                ("\u{201C}", "\u{201D}")
            };
            ctx.push(left);
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.push(right);
        }

        "math_inline" | "math_display" => {
            let source = n.props.get_str("math:source").unwrap_or("");
            ctx.push(source);
        }

        _ => {
            // Unknown inline: emit children.
            for child in &n.children {
                emit_inline(child, ctx);
            }
            ctx.warn(
                WarningKind::UnsupportedNode(n.kind.as_str().to_string()),
                format!("Unknown inline node type for ANSI: {}", n.kind.as_str()),
            );
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
