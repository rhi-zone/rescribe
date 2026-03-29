//! TWiki emitter.

use crate::ast::*;

/// Build a TWiki string from a [`TwikiDoc`].
pub fn build(doc: &TwikiDoc) -> String {
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
            | Inline::BoldItalic(c, _)
            | Inline::BoldCode(c, _)
            | Inline::Strikethrough(c, _)
            | Inline::Superscript(c, _)
            | Inline::Subscript(c, _)
            | Inline::Underline(c, _) => {
                s.push_str(&collect_inline_text(c));
            }
            Inline::Code(t, _) => s.push_str(t),
            Inline::Link { label, .. } => s.push_str(label),
            Inline::LineBreak { .. } => s.push('\n'),
            Inline::Image { url, .. } => s.push_str(url),
            Inline::RawInline { content, .. } => s.push_str(content),
            Inline::WikiWord { word, .. } => s.push_str(word),
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
            output.push_str("---");
            for _ in 0..(*level as usize).min(6) {
                output.push('+');
            }
            output.push(' ');
            build_inlines(inlines, output);
            output.push('\n');
        }

        Block::CodeBlock { content, .. } => {
            output.push_str("<verbatim>\n");
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("</verbatim>\n\n");
        }

        Block::List { ordered, items, .. } => {
            build_list_items(items, *ordered, 1, output);
            output.push('\n');
        }

        Block::Table { rows, .. } => {
            for row in rows {
                output.push('|');
                for cell in &row.cells {
                    output.push(' ');
                    if cell.is_header {
                        output.push('*');
                        build_inlines(&cell.inlines, output);
                        output.push('*');
                    } else {
                        build_inlines(&cell.inlines, output);
                    }
                    output.push_str(" |");
                }
                output.push('\n');
            }
            output.push('\n');
        }

        Block::HorizontalRule { .. } => {
            output.push_str("---\n\n");
        }

        Block::RawBlock { content, .. } => {
            output.push_str(content);
            output.push_str("\n\n");
        }

        Block::DefinitionList { items, .. } => {
            for item in items {
                output.push_str("   $ ");
                build_inlines(&item.term, output);
                output.push_str(": ");
                build_inlines(&item.desc, output);
                output.push('\n');
            }
            output.push('\n');
        }

        Block::Blockquote { children, .. } => {
            output.push_str("<blockquote>\n");
            for child in children {
                build_block(child, output);
            }
            output.push_str("</blockquote>\n\n");
        }
    }
}

fn build_list_items(items: &[ListItem], ordered: bool, depth: usize, output: &mut String) {
    for item in items {
        for _ in 0..depth {
            output.push_str("   ");
        }
        if ordered {
            output.push_str("1. ");
        } else {
            output.push_str("* ");
        }
        build_inlines(&item.inlines, output);
        output.push('\n');
        for child in &item.children {
            if let Block::List { ordered: child_ordered, items: child_items, .. } = child {
                build_list_items(child_items, *child_ordered, depth + 1, output);
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
            output.push('*');
            build_inlines(children, output);
            output.push('*');
        }

        Inline::Italic(children, _) => {
            output.push('_');
            build_inlines(children, output);
            output.push('_');
        }

        Inline::BoldItalic(children, _) => {
            output.push_str("__");
            build_inlines(children, output);
            output.push_str("__");
        }

        Inline::Code(s, _) => {
            output.push('=');
            output.push_str(s);
            output.push('=');
        }

        Inline::BoldCode(children, _) => {
            output.push_str("==");
            build_inlines(children, output);
            output.push_str("==");
        }

        Inline::Link { url, label, .. } => {
            output.push_str("[[");
            output.push_str(url);
            output.push_str("][");
            output.push_str(label);
            output.push_str("]]");
        }

        Inline::LineBreak { .. } => output.push_str("%BR%"),

        Inline::Strikethrough(children, _) => {
            output.push_str("<del>");
            build_inlines(children, output);
            output.push_str("</del>");
        }

        Inline::Superscript(children, _) => {
            output.push_str("<sup>");
            build_inlines(children, output);
            output.push_str("</sup>");
        }

        Inline::Subscript(children, _) => {
            output.push_str("<sub>");
            build_inlines(children, output);
            output.push_str("</sub>");
        }

        Inline::Underline(children, _) => {
            output.push_str("<u>");
            build_inlines(children, output);
            output.push_str("</u>");
        }

        Inline::Image { url, alt, .. } => {
            if alt.is_empty() {
                output.push_str(&format!("<img src=\"{}\" />", url));
            } else {
                output.push_str(&format!("<img src=\"{}\" alt=\"{}\" />", url, alt));
            }
        }

        Inline::RawInline { content, .. } => {
            output.push_str(content);
        }

        Inline::WikiWord { word, .. } => {
            output.push_str(word);
        }
    }
}
