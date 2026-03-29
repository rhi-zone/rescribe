//! Markua emitter — converts [`MarkuaDoc`] to Markua text.

use crate::ast::{Block, Inline, MarkuaDoc};

/// Emit a [`MarkuaDoc`] as a Markua-formatted string.
pub fn emit(doc: &MarkuaDoc) -> String {
    let mut out = String::new();
    for block in &doc.blocks {
        emit_block(block, &mut out);
    }
    out
}

/// Alias for [`emit`] — kept for backwards compatibility.
#[inline]
pub fn build(doc: &MarkuaDoc) -> String {
    emit(doc)
}

// ── Block emitter ─────────────────────────────────────────────────────────────

fn emit_block(block: &Block, out: &mut String) {
    match block {
        Block::Paragraph { inlines, .. } => {
            emit_inlines(inlines, out);
            out.push_str("\n\n");
        }

        Block::Heading { level, inlines, .. } => {
            for _ in 0..*level {
                out.push('#');
            }
            out.push(' ');
            emit_inlines(inlines, out);
            out.push_str("\n\n");
        }

        Block::CodeBlock {
            content, language, ..
        } => {
            out.push_str("```");
            if let Some(lang) = language {
                out.push_str(lang);
            }
            out.push('\n');
            out.push_str(content);
            if !content.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n\n");
        }

        Block::Blockquote { children, .. } => {
            for child in children {
                match child {
                    Block::Paragraph { inlines, .. } => {
                        out.push_str("> ");
                        emit_inlines(inlines, out);
                        out.push('\n');
                    }
                    other => {
                        // Prefix each line with "> "
                        let mut inner = String::new();
                        emit_block(other, &mut inner);
                        for line in inner.lines() {
                            out.push_str("> ");
                            out.push_str(line);
                            out.push('\n');
                        }
                    }
                }
            }
            out.push('\n');
        }

        Block::List { ordered, items, .. } => {
            let mut num = 1u32;
            for item_blocks in items {
                if *ordered {
                    out.push_str(&format!("{}. ", num));
                    num += 1;
                } else {
                    out.push_str("- ");
                }

                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => emit_inlines(inlines, out),
                        other => emit_block(other, out),
                    }
                }
                out.push('\n');
            }
            out.push('\n');
        }

        Block::Table { rows, .. } => {
            for (row_idx, row) in rows.iter().enumerate() {
                out.push('|');
                for cell in &row.cells {
                    out.push(' ');
                    emit_inlines(cell, out);
                    out.push_str(" |");
                }
                out.push('\n');

                if row_idx == 0 {
                    out.push('|');
                    for _ in &row.cells {
                        out.push_str(" --- |");
                    }
                    out.push('\n');
                }
            }
            out.push('\n');
        }

        Block::HorizontalRule { .. } => {
            out.push_str("* * *\n\n");
        }

        Block::SpecialBlock {
            block_type,
            children,
            ..
        } => {
            let prefix = match block_type.as_str() {
                "aside" => "A> ",
                "blurb" => "B> ",
                "warning" => "W> ",
                "tip" => "T> ",
                "error" => "E> ",
                "discussion" => "D> ",
                "question" => "Q> ",
                "information" => "I> ",
                "exercise" => "X> ",
                _ => "",
            };

            if !prefix.is_empty() {
                for child in children {
                    let mut inner = String::new();
                    emit_block(child, &mut inner);
                    for line in inner.trim_end().lines() {
                        out.push_str(prefix);
                        out.push_str(line);
                        out.push('\n');
                    }
                }
                out.push('\n');
            }
        }

        Block::DefinitionList { items, .. } => {
            for (term, def_blocks) in items {
                emit_inlines(term, out);
                out.push('\n');
                for def_block in def_blocks {
                    match def_block {
                        Block::Paragraph { inlines, .. } => {
                            out.push_str(": ");
                            emit_inlines(inlines, out);
                            out.push('\n');
                        }
                        other => {
                            let mut inner = String::new();
                            emit_block(other, &mut inner);
                            for line in inner.trim_end().lines() {
                                out.push_str(": ");
                                out.push_str(line);
                                out.push('\n');
                            }
                        }
                    }
                }
            }
            out.push('\n');
        }

        Block::PageBreak { .. } => {
            out.push_str("{pagebreak}\n\n");
        }

        Block::Figure {
            caption, body, ..
        } => {
            emit_block(body, out);
            if !caption.is_empty() {
                out.push_str("Figure: ");
                emit_inlines(caption, out);
                out.push_str("\n\n");
            }
        }
    }
}

// ── Inline emitter ────────────────────────────────────────────────────────────

fn emit_inlines(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        emit_inline(inline, out);
    }
}

fn emit_inline(inline: &Inline, out: &mut String) {
    match inline {
        Inline::Text(s, _) => out.push_str(s),

        Inline::Strong(children, _) => {
            out.push_str("**");
            emit_inlines(children, out);
            out.push_str("**");
        }

        Inline::Emphasis(children, _) => {
            out.push('*');
            emit_inlines(children, out);
            out.push('*');
        }

        Inline::Strikethrough(children, _) => {
            out.push_str("~~");
            emit_inlines(children, out);
            out.push_str("~~");
        }

        Inline::Subscript(children, _) => {
            out.push('~');
            emit_inlines(children, out);
            out.push('~');
        }

        Inline::Superscript(children, _) => {
            out.push('^');
            emit_inlines(children, out);
            out.push('^');
        }

        Inline::Underline(children, _) => {
            out.push_str("[underline]#");
            emit_inlines(children, out);
            out.push('#');
        }

        Inline::SmallCaps(children, _) => {
            out.push_str("[smallcaps]#");
            emit_inlines(children, out);
            out.push('#');
        }

        Inline::Code(s, _) => {
            if s.contains('`') {
                out.push_str("`` ");
                out.push_str(s);
                out.push_str(" ``");
            } else {
                out.push('`');
                out.push_str(s);
                out.push('`');
            }
        }

        Inline::Link { url, children, .. } => {
            out.push('[');
            if children.is_empty() {
                out.push_str(url);
            } else {
                emit_inlines(children, out);
            }
            out.push_str("](");
            out.push_str(url);
            out.push(')');
        }

        Inline::Image { url, alt, .. } => {
            out.push_str("![");
            out.push_str(alt);
            out.push_str("](");
            out.push_str(url);
            out.push(')');
        }

        Inline::LineBreak(_) => out.push_str("\\\n"),

        Inline::SoftBreak(_) => out.push(' '),

        Inline::FootnoteRef { content, .. } => {
            out.push_str("^[");
            emit_inlines(content, out);
            out.push(']');
        }

        Inline::IndexTerm { term, .. } => {
            out.push_str("i[");
            out.push_str(term);
            out.push(']');
        }

        Inline::MathInline { content, .. } => {
            out.push('$');
            out.push_str(content);
            out.push('$');
        }
    }
}

/// Collect plain text from inline nodes (used by table width calculation etc.).
pub fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s, _) | Inline::Code(s, _) => text.push_str(s),
            Inline::Strong(ch, _)
            | Inline::Emphasis(ch, _)
            | Inline::Strikethrough(ch, _)
            | Inline::Subscript(ch, _)
            | Inline::Superscript(ch, _)
            | Inline::Underline(ch, _)
            | Inline::SmallCaps(ch, _) => {
                text.push_str(&collect_inline_text(ch));
            }
            Inline::Link { children, .. } => text.push_str(&collect_inline_text(children)),
            Inline::FootnoteRef { content, .. } => text.push_str(&collect_inline_text(content)),
            Inline::Image { alt, .. } => text.push_str(alt),
            Inline::IndexTerm { term, .. } => text.push_str(term),
            Inline::MathInline { content, .. } => text.push_str(content),
            Inline::LineBreak(_) => text.push('\n'),
            Inline::SoftBreak(_) => text.push(' '),
        }
    }
    text
}
