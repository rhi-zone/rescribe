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
//! order strikethrough → underline → italic → bold → superscript|subscript,
//! with Bold outermost (matching `make_inline`'s wrapping order).  Non-
//! canonical nesting (e.g. Italic outer, Bold inner) would trivially fail
//! because the parser always re-wraps in canonical order.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rtf_fmt::{Block, Inline, RtfDoc, Span, emit, parse};

// ── Input types ───────────────────────────────────────────────────────────────

/// Vertical-position state: superscript, subscript, or neither.
/// Mutually exclusive — RTF `\super` clears `\sub` and vice versa.
#[derive(Arbitrary, Debug)]
enum VertPos {
    Neither,
    Super,
    Sub,
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
    },
}

// ── Conversion to rtf-fmt AST ─────────────────────────────────────────────────

/// Convert a fuzz inline to an `Inline`, returning `None` for empty text
/// (the parser drops empty text runs so they would cause spurious failures).
fn to_inline(fi: &FuzzInline) -> Option<Inline> {
    match fi {
        FuzzInline::LineBreak => Some(Inline::LineBreak { span: Span::NONE }),
        FuzzInline::Leaf { text, bold, italic, underline, strikethrough, vert } => {
            if text.is_empty() {
                return None;
            }
            // Build in the same order as `make_inline` so the roundtrip is
            // stable: innermost first, then each wrapper applied in sequence,
            // Bold ending up outermost among the styling flags.
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
            Some(inline)
        }
    }
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|paragraphs: Vec<Vec<FuzzInline>>| {
    // Build a document from the fuzz input.
    let blocks: Vec<Block> = paragraphs
        .iter()
        .filter_map(|fuzz_inlines| {
            let inlines: Vec<Inline> = fuzz_inlines.iter().filter_map(to_inline).collect();
            if inlines.is_empty() {
                None // parser skips empty paragraphs; skip here too
            } else {
                Some(Block::Paragraph { inlines, span: Span::NONE })
            }
        })
        .collect();

    if blocks.is_empty() {
        return;
    }

    // normalize() merges adjacent Text siblings, putting the doc into the
    // canonical form the parser always produces.  Without this, two adjacent
    // Text nodes would emit as one continuous run and re-parse as one node.
    let doc = RtfDoc { blocks, span: Span::NONE }.normalize();
    let emitted = emit(&doc);
    let (reparsed, _) = parse(&emitted);

    assert_eq!(
        doc.strip_spans(),
        reparsed.strip_spans(),
        "RTF roundtrip changed AST\n  emitted: {emitted:?}"
    );
});
