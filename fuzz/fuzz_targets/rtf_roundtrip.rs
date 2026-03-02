#![no_main]

//! Correct-direction RTF roundtrip fuzz target.
//!
//! Generates arbitrary RTF ASTs in canonical form, emits them to RTF bytes,
//! parses them back, and asserts structural equality.
//!
//! Direction: arbitrary AST → emit → parse → assert equal
//!
//! Why this direction: starting from bytes and round-tripping through parse
//! only tests consistency of the lossy parser with itself.  Starting from an
//! AST tests whether the emitter faithfully encodes every construct and whether
//! the parser faithfully recovers it.
//!
//! "Canonical form" means inlines are structured exactly as the parser
//! produces them: a flat list of single-leaf formatting wrappers in the fixed
//! order strikethrough → underline → italic → bold → superscript|subscript →
//! font_size → color, with Color outermost (matching `make_inline`'s wrapping
//! order).  Non-canonical nesting (e.g. Italic outer, Bold inner) would
//! trivially fail because the parser always re-wraps in canonical order.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rtf_fmt::{Align, Block, Inline, RtfDoc, Span, emit, parse};

// ── Input types ───────────────────────────────────────────────────────────────

/// Vertical-position state: superscript, subscript, or neither.
/// Mutually exclusive — RTF `\super` clears `\sub` and vice versa.
#[derive(Arbitrary, Debug)]
enum VertPos {
    Neither,
    Super,
    Sub,
}

/// Fuzz-friendly paragraph alignment.
#[derive(Arbitrary, Debug)]
enum FuzzAlign {
    Default,
    Left,
    Center,
    Right,
    Justify,
}

impl FuzzAlign {
    fn to_align(&self) -> Align {
        match self {
            FuzzAlign::Default => Align::Default,
            FuzzAlign::Left => Align::Left,
            FuzzAlign::Center => Align::Center,
            FuzzAlign::Right => Align::Right,
            FuzzAlign::Justify => Align::Justify,
        }
    }
}

/// One inline element in canonical parser-output form.
#[derive(Arbitrary, Debug)]
enum FuzzInline {
    LineBreak,
    Leaf {
        text: String,
        bold: bool,
        italic: bool,
        underline: bool,
        strikethrough: bool,
        vert: VertPos,
        font_size: u16,
        color: Option<(u8, u8, u8)>,
    },
}

/// A fuzz paragraph with explicit alignment.
#[derive(Arbitrary, Debug)]
struct FuzzPara {
    align: FuzzAlign,
    inlines: Vec<FuzzInline>,
}

// ── Conversion to rtf-fmt AST ─────────────────────────────────────────────────

/// Convert a fuzz inline to an `Inline`, returning `None` for empty text
/// (the parser drops empty text runs so they would cause spurious failures).
fn to_inline(fi: &FuzzInline, color_table: &[(u8, u8, u8)]) -> Option<Inline> {
    match fi {
        FuzzInline::LineBreak => Some(Inline::LineBreak { span: Span::NONE }),
        FuzzInline::Leaf { text, bold, italic, underline, strikethrough, vert, font_size, color } => {
            if text.is_empty() {
                return None;
            }
            // Build in the same order as `make_inline` so the roundtrip is
            // stable: innermost first, then each wrapper applied in sequence,
            // Color ending up outermost.
            let mut inline = Inline::Text { text: text.clone(), span: Span::NONE };
            if *strikethrough {
                inline = Inline::Strikethrough { children: vec![inline], span: Span::NONE };
            }
            if *underline {
                inline = Inline::Underline { children: vec![inline], span: Span::NONE };
            }
            if *italic {
                inline = Inline::Italic { children: vec![inline], span: Span::NONE };
            }
            if *bold {
                inline = Inline::Bold { children: vec![inline], span: Span::NONE };
            }
            match vert {
                VertPos::Super => {
                    inline = Inline::Superscript { children: vec![inline], span: Span::NONE };
                }
                VertPos::Sub => {
                    inline = Inline::Subscript { children: vec![inline], span: Span::NONE };
                }
                VertPos::Neither => {}
            }
            // font_size = 0 means "not set"; clamp to non-zero values only
            if *font_size != 0 {
                inline = Inline::FontSize {
                    size: (*font_size).max(1),
                    children: vec![inline],
                    span: Span::NONE,
                };
            }
            if let Some((r, g, b)) = color {
                // Only emit color if it's in the color_table (at index 1+)
                if color_table.contains(&(*r, *g, *b)) {
                    inline = Inline::Color {
                        r: *r,
                        g: *g,
                        b: *b,
                        children: vec![inline],
                        span: Span::NONE,
                    };
                }
            }
            Some(inline)
        }
    }
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|paras: Vec<FuzzPara>| {
    // Collect unique colors only from leaves whose text is non-empty — those
    // are the only ones that will actually produce Color inlines (empty-text
    // leaves are dropped by to_inline).  This keeps doc.color_table equal to
    // what the emitter's collect_colors() will derive from the inline tree.
    let mut color_table: Vec<(u8, u8, u8)> = Vec::new();
    for para in &paras {
        for fi in &para.inlines {
            if let FuzzInline::Leaf { color: Some(rgb), text, .. } = fi {
                if !text.is_empty() && !color_table.contains(rgb) {
                    color_table.push(*rgb);
                }
            }
        }
    }

    // Build a document from the fuzz input.
    let blocks: Vec<Block> = paras
        .iter()
        .filter_map(|fuzz_para| {
            let inlines: Vec<Inline> = fuzz_para.inlines
                .iter()
                .filter_map(|fi| to_inline(fi, &color_table))
                .collect();
            if inlines.is_empty() {
                None // parser skips empty paragraphs; skip here too
            } else {
                Some(Block::Paragraph {
                    inlines,
                    align: fuzz_para.align.to_align(),
                    para_props: String::new(),
                    span: Span::NONE,
                })
            }
        })
        .collect();

    if blocks.is_empty() {
        return;
    }

    // normalize() merges adjacent Text siblings, putting the doc into the
    // canonical form the parser always produces.  Without this, two adjacent
    // Text nodes would emit as one continuous run and re-parse as one node.
    let doc = RtfDoc { blocks, color_table, span: Span::NONE }.normalize();
    let emitted = emit(&doc);
    let (reparsed, _) = parse(&emitted);

    assert_eq!(
        doc.strip_spans(),
        reparsed.strip_spans(),
        "RTF roundtrip changed AST\n  emitted: {emitted:?}"
    );
});
