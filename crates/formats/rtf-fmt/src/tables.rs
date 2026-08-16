//! Shared "used fonts/colors" table computation.
//!
//! `emit()` (the tree-based builder, `emit.rs`) and `sem_events::events()`
//! (the semantic event iterator) both need to agree on the *exact same*
//! font/color index assignment — first-occurrence order walking the document
//! tree — because the header (`\fonttbl`/`\colortbl`) built from these
//! tables and the body's `\f<n>`/`\cf<n>` references built from the same
//! tables must use identical indices for the two independent emission paths
//! to produce identical bytes. This module is the single source of truth so
//! they cannot silently diverge.
//!
//! Note this table is *not* [`crate::ast::RtfDoc::color_table`] — that field
//! is what `parse()` captured from a source document's own `\colortbl` (it
//! may contain unused entries, or differ in order from what the writer would
//! choose). The tables computed here are the writer's own canonical choice,
//! built fresh from actual usage in the AST being emitted.

use crate::ast::{Block, Inline, RtfDoc};

/// Collect all unique RGB colors referenced in `doc`, in first-occurrence
/// document order. Does not include an entry for the auto/default color
/// (body references use index 0 for "not found", matching `emit_inline`'s
/// `unwrap_or(0)` lookup convention).
pub fn collect_used_colors(doc: &RtfDoc) -> Vec<(u8, u8, u8)> {
    let mut colors: Vec<(u8, u8, u8)> = Vec::new();
    for block in &doc.blocks {
        collect_colors_in_block(block, &mut colors);
    }
    colors
}

fn collect_colors_in_block(block: &Block, out: &mut Vec<(u8, u8, u8)>) {
    match block {
        Block::Paragraph { inlines, .. } | Block::Heading { inlines, .. } => {
            for inline in inlines {
                collect_colors_in_inline(inline, out);
            }
        }
        Block::Blockquote { children, .. } => {
            for child in children {
                collect_colors_in_block(child, out);
            }
        }
        Block::List { items, .. } => {
            for item in items {
                for block in item {
                    collect_colors_in_block(block, out);
                }
            }
        }
        Block::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    for inline in cell {
                        collect_colors_in_inline(inline, out);
                    }
                }
            }
        }
        Block::CodeBlock { .. } | Block::HorizontalRule { .. } => {}
    }
}

fn collect_colors_in_inline(inline: &Inline, out: &mut Vec<(u8, u8, u8)>) {
    match inline {
        Inline::Color {
            r, g, b, children, ..
        } => {
            let rgb = (*r, *g, *b);
            if !out.contains(&rgb) {
                out.push(rgb);
            }
            for child in children {
                collect_colors_in_inline(child, out);
            }
        }
        Inline::Bold { children, .. }
        | Inline::Italic { children, .. }
        | Inline::Underline { children, .. }
        | Inline::Strikethrough { children, .. }
        | Inline::Superscript { children, .. }
        | Inline::Subscript { children, .. }
        | Inline::FontSize { children, .. }
        | Inline::AllCaps { children, .. }
        | Inline::SmallCaps { children, .. }
        | Inline::Hidden { children, .. }
        | Inline::CharSpan { children, .. }
        | Inline::Link { children, .. }
        | Inline::Font { children, .. }
        | Inline::Lang { children, .. } => {
            for child in children {
                collect_colors_in_inline(child, out);
            }
        }
        Inline::BgColor {
            r, g, b, children, ..
        } => {
            let rgb = (*r, *g, *b);
            if !out.contains(&rgb) {
                out.push(rgb);
            }
            for child in children {
                collect_colors_in_inline(child, out);
            }
        }
        Inline::Footnote { content, .. } => {
            for block in content {
                collect_colors_in_block(block, out);
            }
        }
        Inline::Shape { text, .. } => {
            for block in text {
                collect_colors_in_block(block, out);
            }
        }
        Inline::Text { .. }
        | Inline::Code { .. }
        | Inline::Image { .. }
        | Inline::LineBreak { .. }
        | Inline::SoftBreak { .. } => {}
    }
}

/// Collect all unique font names referenced via `Inline::Font` in `doc`, in
/// first-occurrence document order. Does not include the implicit default
/// font (`"Times New Roman"`, index 0) — see [`build_font_map`] for the
/// indexed table with the default prepended.
pub fn collect_used_fonts(doc: &RtfDoc) -> Vec<String> {
    let mut fonts: Vec<String> = Vec::new();
    for block in &doc.blocks {
        collect_fonts_in_block(block, &mut fonts);
    }
    fonts
}

fn collect_fonts_in_block(block: &Block, out: &mut Vec<String>) {
    match block {
        Block::Paragraph { inlines, .. } | Block::Heading { inlines, .. } => {
            for inline in inlines {
                collect_fonts_in_inline(inline, out);
            }
        }
        Block::Blockquote { children, .. } => {
            for child in children {
                collect_fonts_in_block(child, out);
            }
        }
        Block::List { items, .. } => {
            for item in items {
                for block in item {
                    collect_fonts_in_block(block, out);
                }
            }
        }
        Block::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    for inline in cell {
                        collect_fonts_in_inline(inline, out);
                    }
                }
            }
        }
        Block::CodeBlock { .. } | Block::HorizontalRule { .. } => {}
    }
}

fn collect_fonts_in_inline(inline: &Inline, out: &mut Vec<String>) {
    match inline {
        Inline::Font { name, children, .. } => {
            if !out.contains(name) {
                out.push(name.clone());
            }
            for child in children {
                collect_fonts_in_inline(child, out);
            }
        }
        Inline::Bold { children, .. }
        | Inline::Italic { children, .. }
        | Inline::Underline { children, .. }
        | Inline::Strikethrough { children, .. }
        | Inline::Superscript { children, .. }
        | Inline::Subscript { children, .. }
        | Inline::FontSize { children, .. }
        | Inline::AllCaps { children, .. }
        | Inline::SmallCaps { children, .. }
        | Inline::Hidden { children, .. }
        | Inline::CharSpan { children, .. }
        | Inline::Color { children, .. }
        | Inline::BgColor { children, .. }
        | Inline::Link { children, .. }
        | Inline::Lang { children, .. } => {
            for child in children {
                collect_fonts_in_inline(child, out);
            }
        }
        Inline::Footnote { content, .. } => {
            for block in content {
                collect_fonts_in_block(block, out);
            }
        }
        Inline::Shape { text, .. } => {
            for block in text {
                collect_fonts_in_block(block, out);
            }
        }
        Inline::Text { .. }
        | Inline::Code { .. }
        | Inline::Image { .. }
        | Inline::LineBreak { .. }
        | Inline::SoftBreak { .. } => {}
    }
}

/// Incremental variant of [`collect_used_fonts`]: appends first-occurrence
/// fonts found in `blocks` into an existing `out` accumulator (already
/// containing whatever was found in earlier blocks), so a caller that only
/// has the document available piece-by-piece (e.g.
/// [`crate::batch::StreamingParser`], which parses one paragraph/table/list
/// increment at a time) can reproduce the exact same first-occurrence order
/// [`collect_used_fonts`] computes from a fully-parsed [`RtfDoc`] — by
/// calling this once per increment, in document order, with the same `out`
/// vector threaded through. The result after the last increment is
/// byte-identical to `collect_used_fonts(&whole_doc)`.
pub(crate) fn collect_used_fonts_incremental(blocks: &[Block], out: &mut Vec<String>) {
    for block in blocks {
        collect_fonts_in_block(block, out);
    }
}

/// Incremental variant of [`collect_used_colors`] — see
/// [`collect_used_fonts_incremental`]'s doc comment for the exact contract.
pub(crate) fn collect_used_colors_incremental(blocks: &[Block], out: &mut Vec<(u8, u8, u8)>) {
    for block in blocks {
        collect_colors_in_block(block, out);
    }
}

/// The full, indexed font table an emitter should write to `\fonttbl` and
/// look `Inline::Font { name }` up against: index 0 is always the implicit
/// default (`"Times New Roman"`), followed by [`collect_used_fonts`] in
/// first-occurrence order.
pub fn build_font_map(doc: &RtfDoc) -> Vec<String> {
    let mut v = vec!["Times New Roman".to_string()];
    v.extend(collect_used_fonts(doc));
    v
}

/// The full color table an emitter should write to `\colortbl` and look
/// `Inline::Color`/`Inline::BgColor` up against (1-indexed at emission time;
/// index 0 is the implicit auto/default color and has no entry here).
pub fn build_color_map(doc: &RtfDoc) -> Vec<(u8, u8, u8)> {
    collect_used_colors(doc)
}
