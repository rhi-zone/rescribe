//! XWiki emitter.

use crate::ast::*;

/// Build an XWiki string from an [`XwikiDoc`].
pub fn build(doc: &XwikiDoc) -> String {
    let mut output = String::new();
    for block in &doc.blocks {
        build_block(block, &mut output);
    }
    output
}

/// Collect plain text from a slice of inlines.
pub fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t, _) => s.push_str(t),
            Inline::Bold(c, _)
            | Inline::Italic(c, _)
            | Inline::Underline(c, _)
            | Inline::Strikeout(c, _)
            | Inline::Superscript(c, _)
            | Inline::Subscript(c, _) => s.push_str(&collect_inline_text(c)),
            Inline::Code(t, _) => s.push_str(t),
            Inline::Link { label, .. } => s.push_str(label),
            Inline::Image { url, .. } => s.push_str(url),
            Inline::LineBreak { .. } => s.push('\n'),
            Inline::SoftBreak { .. } => s.push(' '),
        }
    }
    s
}

fn build_block(block: &Block, output: &mut String) {
    match block {
        Block::Heading { level, inlines, .. } => {
            for _ in 0..*level {
                output.push('=');
            }
            output.push(' ');
            build_inlines(inlines, output);
            output.push(' ');
            for _ in 0..*level {
                output.push('=');
            }
            output.push('\n');
        }

        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, output);
            output.push_str("\n\n");
        }

        Block::CodeBlock { content, language, .. } => {
            if let Some(lang) = language {
                output.push_str(&format!("{{{{code language=\"{}\"}}}}\n", lang));
            } else {
                output.push_str("{{code}}\n");
            }
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("{{/code}}\n\n");
        }

        Block::Table { rows, .. } => {
            for row in rows {
                output.push('|');
                for cell in &row.cells {
                    if cell.is_header {
                        output.push('=');
                    }
                    build_inlines(&cell.inlines, output);
                    output.push('|');
                }
                output.push('\n');
            }
            output.push('\n');
        }

        Block::List { ordered, items, .. } => {
            for item_blocks in items {
                if *ordered {
                    output.push_str("1. ");
                } else {
                    output.push_str("* ");
                }
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => build_inlines(inlines, output),
                        other => build_block(other, output),
                    }
                }
                output.push('\n');
            }
            output.push('\n');
        }

        Block::HorizontalRule { .. } => {
            output.push_str("----\n\n");
        }

        Block::Blockquote { children, .. } => {
            output.push_str("{{quote}}\n");
            for child in children {
                build_block(child, output);
            }
            output.push_str("{{/quote}}\n\n");
        }

        Block::MacroBlock { name, params, content, .. } => {
            output.push_str("{{");
            output.push_str(name);
            if !params.is_empty() {
                output.push(' ');
                output.push_str(params);
            }
            output.push_str("}}\n");
            output.push_str(content);
            if !content.is_empty() && !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&format!("{{{{/{}}}}}\n\n", name));
        }

        Block::MacroInline { name, params, .. } => {
            output.push_str("{{");
            output.push_str(name);
            if !params.is_empty() {
                output.push(' ');
                output.push_str(params);
            }
            output.push_str("/}}\n\n");
        }
    }
}

fn build_inlines(inlines: &[Inline], output: &mut String) {
    for inline in inlines {
        build_inline(inline, output);
    }
}

fn build_inline(inline: &Inline, output: &mut String) {
    match inline {
        Inline::Text(s, _) => output.push_str(s),

        Inline::Bold(children, _) => {
            output.push_str("**");
            build_inlines(children, output);
            output.push_str("**");
        }

        Inline::Italic(children, _) => {
            output.push_str("//");
            build_inlines(children, output);
            output.push_str("//");
        }

        Inline::Underline(children, _) => {
            output.push_str("__");
            build_inlines(children, output);
            output.push_str("__");
        }

        Inline::Strikeout(children, _) => {
            output.push_str("--");
            build_inlines(children, output);
            output.push_str("--");
        }

        Inline::Superscript(children, _) => {
            output.push_str("^^");
            build_inlines(children, output);
            output.push_str("^^");
        }

        Inline::Subscript(children, _) => {
            output.push_str("~~");
            build_inlines(children, output);
            output.push_str("~~");
        }

        Inline::Code(s, _) => {
            output.push_str("##");
            output.push_str(s);
            output.push_str("##");
        }

        Inline::Link { url, label, .. } => {
            output.push_str("[[");
            output.push_str(label);
            output.push_str(">>");
            output.push_str(url);
            output.push_str("]]");
        }

        Inline::Image { url, alt, params, .. } => {
            output.push_str("[[image:");
            output.push_str(url);
            let has_alt = alt.is_some();
            let has_params = !params.is_empty();
            if has_alt || has_params {
                output.push_str("||");
                if let Some(alt_text) = alt {
                    output.push_str(&format!("alt=\"{}\"", alt_text));
                    if has_params {
                        output.push(' ');
                    }
                }
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                output.push_str(&param_strs.join(" "));
            }
            output.push_str("]]");
        }

        Inline::LineBreak { .. } => output.push_str("\\\\ "),
        Inline::SoftBreak { .. } => output.push(' '),
    }
}
