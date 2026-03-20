//! ANSI emitter — converts [`AnsiDoc`] to ANSI-formatted text.

use crate::ast::{AnsiDoc, Block, Inline, TableRow};

// ── ANSI escape codes ─────────────────────────────────────────────────────────

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";
pub const UNDERLINE: &str = "\x1b[4m";
pub const STRIKETHROUGH: &str = "\x1b[9m";

pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";

pub const BG_BLACK: &str = "\x1b[40m";

// ── Public API ────────────────────────────────────────────────────────────────

/// Emit an [`AnsiDoc`] as an ANSI-formatted string.
pub fn emit(doc: &AnsiDoc) -> String {
    let mut out = String::new();
    for block in &doc.blocks {
        emit_block(block, &mut out);
    }
    out
}

/// Alias for [`emit`] — kept for backwards compatibility.
#[inline]
pub fn build(doc: &AnsiDoc) -> String {
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
            out.push_str(BOLD);
            let color = match level {
                1 => BLUE,
                2 => GREEN,
                3 => YELLOW,
                4 => CYAN,
                _ => MAGENTA,
            };
            out.push_str(color);
            for _ in 0..*level {
                out.push('#');
            }
            out.push(' ');
            emit_inlines(inlines, out);
            out.push_str(RESET);
            out.push_str("\n\n");
        }

        Block::CodeBlock { language, content, .. } => {
            out.push_str(DIM);
            out.push_str("┌─");
            if let Some(lang) = language {
                out.push(' ');
                out.push_str(lang);
                out.push(' ');
            }
            out.push_str("─\n");
            out.push_str(RESET);

            out.push_str(BG_BLACK);
            out.push_str(CYAN);

            for line in content.lines() {
                out.push_str(DIM);
                out.push_str("│ ");
                out.push_str(RESET);
                out.push_str(BG_BLACK);
                out.push_str(CYAN);
                out.push_str(line);
                out.push_str(RESET);
                out.push('\n');
            }

            out.push_str(RESET);
            out.push_str(DIM);
            out.push_str("└─\n");
            out.push_str(RESET);
            out.push('\n');
        }

        Block::Blockquote { children, .. } => {
            for line in collect_block_text(children).lines() {
                out.push_str(DIM);
                out.push_str("│ ");
                out.push_str(RESET);
                out.push_str(ITALIC);
                out.push_str(line);
                out.push_str(RESET);
                out.push('\n');
            }
            out.push('\n');
        }

        Block::List { ordered, items, .. } => {
            for (idx, item_blocks) in items.iter().enumerate() {
                if *ordered {
                    out.push_str(YELLOW);
                    out.push_str(&format!("{}. ", idx + 1));
                    out.push_str(RESET);
                } else {
                    out.push_str(GREEN);
                    out.push_str("• ");
                    out.push_str(RESET);
                }
                for block in item_blocks {
                    emit_block(block, out);
                }
            }
            out.push('\n');
        }

        Block::ListItem { children, .. } => {
            for child in children {
                emit_block(child, out);
            }
        }

        Block::Table { rows, .. } => {
            let col_widths = calculate_column_widths(rows);
            emit_table_border(&col_widths, out, '┌', '┬', '┐');

            let mut is_header = true;
            for row in rows {
                out.push('│');
                for (i, cell) in row.cells.iter().enumerate() {
                    let width = col_widths.get(i).copied().unwrap_or(1);
                    if is_header {
                        out.push_str(BOLD);
                    }
                    out.push(' ');
                    let text = collect_inline_text(&cell.inlines);
                    out.push_str(&text);
                    for _ in text.len()..width {
                        out.push(' ');
                    }
                    if is_header {
                        out.push_str(RESET);
                    }
                    out.push_str(" │");
                }
                out.push('\n');

                if is_header && rows.len() > 1 {
                    emit_table_border(&col_widths, out, '├', '┼', '┤');
                    is_header = false;
                }
            }

            emit_table_border(&col_widths, out, '└', '┴', '┘');
            out.push('\n');
        }

        Block::TableRow { cells, .. } => {
            out.push('│');
            for cell in cells {
                out.push(' ');
                emit_inlines(&cell.inlines, out);
                out.push_str(" │");
            }
            out.push('\n');
        }

        Block::TableCell { inlines, .. } => {
            emit_inlines(inlines, out);
        }

        Block::TableHeader { cells, .. } => {
            for cell in cells {
                emit_inlines(&cell.inlines, out);
            }
        }

        Block::TableBody { rows, .. } => {
            for row in rows {
                emit_block(
                    &Block::TableRow {
                        cells: row.cells.clone(),
                        span: crate::ast::Span::NONE,
                    },
                    out,
                );
            }
        }

        Block::TableFoot { rows, .. } => {
            for row in rows {
                emit_block(
                    &Block::TableRow {
                        cells: row.cells.clone(),
                        span: crate::ast::Span::NONE,
                    },
                    out,
                );
            }
        }

        Block::HorizontalRule { .. } => {
            out.push_str(DIM);
            out.push_str("────────────────────────────────────────");
            out.push_str(RESET);
            out.push_str("\n\n");
        }

        Block::Div { children, .. } => {
            for child in children {
                emit_block(child, out);
            }
        }

        Block::SpanBlock { inlines, .. } => {
            emit_inlines(inlines, out);
        }

        Block::RawBlock { content, .. } => {
            out.push_str(content);
            out.push('\n');
        }

        Block::RawInline { content, .. } => {
            out.push_str(content);
        }

        Block::DefinitionList { items, .. } => {
            for item in items {
                emit_inlines(&item.term, out);
                out.push('\n');
                for desc_block in &item.desc {
                    out.push_str("  ");
                    emit_block(desc_block, out);
                }
            }
            out.push('\n');
        }

        Block::DefinitionTerm { inlines, .. } => {
            out.push_str(BOLD);
            emit_inlines(inlines, out);
            out.push_str(RESET);
            out.push('\n');
        }

        Block::DefinitionDesc { children, .. } => {
            out.push_str("  ");
            for child in children {
                emit_block(child, out);
            }
        }

        Block::Figure { children, .. } => {
            for child in children {
                emit_block(child, out);
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

        Inline::Bold(children, _) => {
            out.push_str(BOLD);
            emit_inlines(children, out);
            out.push_str(RESET);
        }

        Inline::Italic(children, _) => {
            out.push_str(ITALIC);
            emit_inlines(children, out);
            out.push_str(RESET);
        }

        Inline::Underline(children, _) => {
            out.push_str(UNDERLINE);
            emit_inlines(children, out);
            out.push_str(RESET);
        }

        Inline::Strikethrough(children, _) => {
            out.push_str(STRIKETHROUGH);
            emit_inlines(children, out);
            out.push_str(RESET);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn emit_table_border(widths: &[usize], out: &mut String, left: char, mid: char, right: char) {
    out.push_str(DIM);
    out.push(left);
    for (i, w) in widths.iter().enumerate() {
        for _ in 0..(*w + 2) {
            out.push('─');
        }
        if i < widths.len() - 1 {
            out.push(mid);
        }
    }
    out.push(right);
    out.push_str(RESET);
    out.push('\n');
}

fn calculate_column_widths(rows: &[TableRow]) -> Vec<usize> {
    let num_cols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    let mut widths = vec![1usize; num_cols];

    for row in rows {
        for (i, cell) in row.cells.iter().enumerate() {
            let text = collect_inline_text(&cell.inlines);
            if text.len() > widths[i] {
                widths[i] = text.len();
            }
        }
    }
    widths
}

pub fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => text.push_str(s),
            Inline::Bold(c, _)
            | Inline::Italic(c, _)
            | Inline::Underline(c, _)
            | Inline::Strikethrough(c, _) => {
                text.push_str(&collect_inline_text(c));
            }
        }
    }
    text
}

fn collect_block_text(blocks: &[Block]) -> String {
    let mut text = String::new();
    for block in blocks {
        if let Block::Paragraph { inlines, .. } = block {
            text.push_str(&collect_inline_text(inlines));
            text.push('\n');
        }
    }
    text
}

