//! TikiWiki emitter.

use crate::ast::*;

/// Build a TikiWiki string from a [`TikiwikiDoc`].
pub fn build(doc: &TikiwikiDoc) -> String {
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
            | Inline::Strikethrough(c, _)
            | Inline::Superscript(c, _)
            | Inline::Subscript(c, _) => s.push_str(&collect_inline_text(c)),
            Inline::Code(t, _) => s.push_str(t),
            Inline::Link { children, .. } | Inline::WikiLink { children, .. } => {
                s.push_str(&collect_inline_text(children));
            }
            Inline::Image { alt, .. } => s.push_str(alt),
            Inline::Nowiki(t, _) => s.push_str(t),
            Inline::LineBreak { .. } => s.push('\n'),
        }
    }
    s
}

fn build_block(block: &Block, output: &mut String) {
    match block {
        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, output);
            output.push_str("\n\n");
        }

        Block::Heading { level, inlines, .. } => {
            for _ in 0..(*level as usize).min(6) {
                output.push('!');
            }
            output.push(' ');
            build_inlines(inlines, output);
            output.push('\n');
        }

        Block::CodeBlock { content, language, .. } => {
            if let Some(lang) = language {
                output.push_str(&format!("{{CODE(lang={})}}\n", lang));
            } else {
                output.push_str("{CODE()}\n");
            }
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("{CODE}\n\n");
        }

        Block::Blockquote { blocks, .. } => {
            output.push_str("{QUOTE()}\n");
            for block in blocks {
                build_block(block, output);
            }
            output.push_str("{QUOTE}\n\n");
        }

        Block::List { ordered, items, .. } => {
            let marker = if *ordered { '#' } else { '*' };
            build_list_items(items, output, marker, 1);
            output.push('\n');
        }

        Block::Table { rows, .. } => {
            for row in rows {
                output.push_str("||");
                for (i, cell) in row.cells.iter().enumerate() {
                    if i > 0 {
                        output.push('|');
                    }
                    if row.is_header {
                        output.push('^');
                        build_inlines(&cell.inlines, output);
                        output.push('^');
                    } else {
                        build_inlines(&cell.inlines, output);
                    }
                }
                output.push_str("||\n");
            }
            output.push('\n');
        }

        Block::HorizontalRule { .. } => {
            output.push_str("---\n\n");
        }
    }
}

fn build_list_items(items: &[ListItem], output: &mut String, marker: char, depth: usize) {
    for item in items {
        for _ in 0..depth {
            output.push(marker);
        }
        output.push(' ');
        build_inlines(&item.inlines, output);
        output.push('\n');
        for child in &item.children {
            if let Block::List { items: nested_items, .. } = child {
                build_list_items(nested_items, output, marker, depth + 1);
            }
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
            output.push_str("__");
            build_inlines(children, output);
            output.push_str("__");
        }

        Inline::Italic(children, _) => {
            output.push_str("''");
            build_inlines(children, output);
            output.push_str("''");
        }

        Inline::Underline(children, _) => {
            output.push_str("===");
            build_inlines(children, output);
            output.push_str("===");
        }

        Inline::Strikethrough(children, _) => {
            output.push_str("--");
            build_inlines(children, output);
            output.push_str("--");
        }

        Inline::Superscript(children, _) => {
            output.push('^');
            build_inlines(children, output);
            output.push('^');
        }

        Inline::Subscript(children, _) => {
            output.push_str(",,");
            build_inlines(children, output);
            output.push_str(",,");
        }

        Inline::Code(s, _) => {
            output.push_str("-+");
            output.push_str(s);
            output.push_str("+-");
        }

        Inline::Link { url, children, .. } => {
            output.push('[');
            output.push_str(url);
            if !children.is_empty() {
                output.push('|');
                build_inlines(children, output);
            }
            output.push(']');
        }

        Inline::WikiLink { page, children, .. } => {
            output.push_str("((");
            output.push_str(page);
            // Only emit |label if the label differs from the page name
            let label_text = collect_inline_text(children);
            if label_text != *page {
                output.push('|');
                build_inlines(children, output);
            }
            output.push_str("))");
        }

        Inline::Image { url, alt, .. } => {
            output.push_str("{img src=\"");
            output.push_str(url);
            if !alt.is_empty() {
                output.push_str("\" alt=\"");
                output.push_str(alt);
            }
            output.push_str("\"}");
        }

        Inline::Nowiki(s, _) => {
            output.push_str("~np~");
            output.push_str(s);
            output.push_str("~/np~");
        }

        Inline::LineBreak { .. } => output.push_str("%%%"),
    }
}
