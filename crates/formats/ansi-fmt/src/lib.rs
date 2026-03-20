//! ANSI terminal text parser, emitter, and AST.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-ansi` and `rescribe-write-ansi` as thin adapter layers.
//!
//! # API
//!
//! ```rust
//! use ansi_fmt::{parse, emit};
//!
//! let (doc, _diagnostics) = parse("\x1b[1mHello\x1b[0m world");
//! let output = emit(&doc);
//! ```

pub mod ast;
pub mod emit;
pub mod parse;

// Re-export the most-used types at the crate root for convenience.
pub use ast::{
    AnsiDoc, Block, DefinitionItem, Diagnostic, Inline, Severity, Span, TableCell, TableRow,
};
pub use emit::{
    build, collect_inline_text, emit, BG_BLACK, BLUE, BOLD, CYAN, DIM, GREEN, ITALIC, MAGENTA,
    RESET, STRIKETHROUGH, UNDERLINE, YELLOW,
};
pub use parse::{parse, strip_ansi};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let (doc, _) = parse("Hello world");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("\x1b[1mBold text\x1b[0m");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("\x1b[3mItalic text\x1b[0m");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_underline() {
        let (doc, _) = parse("\x1b[4mUnderlined\x1b[0m");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("\x1b[1mBold\x1b[0m"), "Bold");
        assert_eq!(strip_ansi("\x1b[31mRed\x1b[0m"), "Red");
        assert_eq!(strip_ansi("Plain text"), "Plain text");
    }

    #[test]
    fn test_combined_styles() {
        let (doc, _) = parse("\x1b[1;3mBold and italic\x1b[0m");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_build_simple() {
        let doc = AnsiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello world".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn test_build_bold() {
        let doc = AnsiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("bold"));
        assert!(output.contains(BOLD));
    }

    #[test]
    fn test_build_italic() {
        let doc = AnsiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = build(&doc);
        assert!(output.contains("italic"));
        assert!(output.contains(ITALIC));
    }

    #[test]
    fn test_roundtrip_plain() {
        let input = "Hello world";
        let (doc, _) = parse(input);
        let emitted = emit(&doc);
        let (doc2, _) = parse(&emitted);
        let text1 = collect_text(&doc.blocks);
        let text2 = collect_text(&doc2.blocks);
        assert_eq!(text1, text2);
    }

    #[test]
    fn test_roundtrip_bold() {
        let input = "\x1b[1mHello\x1b[0m";
        let (doc, _) = parse(input);
        let emitted = emit(&doc);
        let (doc2, _) = parse(&emitted);
        let text1 = collect_text(&doc.blocks);
        let text2 = collect_text(&doc2.blocks);
        assert_eq!(text1, text2);
    }

    fn collect_text(blocks: &[Block]) -> String {
        let mut s = String::new();
        for b in blocks {
            if let Block::Paragraph { inlines, .. } = b {
                s.push_str(&collect_inline_text(inlines));
            }
        }
        s
    }
}
