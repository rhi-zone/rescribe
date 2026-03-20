//! txt2tags (t2t) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-t2t` and `rescribe-write-t2t` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

// Re-export the primary public API for convenience.
pub use ast::{Block, Diagnostic, Inline, Severity, Span, T2tDoc, TableRow};
pub use emit::emit;
pub use parse::{parse, parse_str};

// ── Legacy compatibility (removed) ────────────────────────────────────────────
//
// The old `T2tError` type and `build()` / `parse() -> Result<>` signatures have
// been replaced by infallible `parse() -> (T2tDoc, Vec<Diagnostic>)` and
// `emit(doc) -> String`. Update callers accordingly.

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::emit::emit;
    use crate::parse::parse_str;

    #[test]
    fn test_parse_heading() {
        let doc = parse_str("= Title =\n");
        assert_eq!(doc.blocks.len(), 1);
        match &doc.blocks[0] {
            Block::Heading {
                level,
                numbered,
                inlines: _,
                ..
            } => {
                assert_eq!(*level, 1);
                assert!(!*numbered);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse_str("== Subtitle ==\n");
        match &doc.blocks[0] {
            Block::Heading { level, .. } => {
                assert_eq!(*level, 2);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_numbered_heading() {
        let doc = parse_str("+ Numbered +\n");
        match &doc.blocks[0] {
            Block::Heading { numbered, .. } => {
                assert!(*numbered);
            }
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("**bold**\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Bold(..)));
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("//italic//\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Italic(..)));
    }

    #[test]
    fn test_parse_underline() {
        let doc = parse_str("__underline__\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Underline(..)));
    }

    #[test]
    fn test_parse_strikethrough() {
        let doc = parse_str("--strike--\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Strikethrough(..)));
    }

    #[test]
    fn test_parse_monospace() {
        let doc = parse_str("``code``\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(matches!(inlines[0], Inline::Code(..)));
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse_str("- item1\n- item2\n");
        assert_eq!(doc.blocks.len(), 1);
        let Block::List { ordered, items, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(!*ordered);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_str("+ first\n+ second\n");
        let Block::List { ordered, .. } = &doc.blocks[0] else {
            panic!("expected list");
        };
        assert!(*ordered);
    }

    #[test]
    fn test_parse_verbatim_block() {
        let doc = parse_str("```\ncode here\n```\n");
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
        let Block::CodeBlock { content, .. } = &doc.blocks[0] else {
            panic!("expected code block");
        };
        assert_eq!(content, "code here");
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("[click here http://example.com]\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
        if let Inline::Link { url, .. } = link {
            assert_eq!(url, "http://example.com");
        }
    }

    #[test]
    fn test_parse_quote() {
        let doc = parse_str("\tquoted text\n");
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_skip_comments() {
        let doc = parse_str("% comment\ntext\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_build_heading() {
        let doc = T2tDoc {
            blocks: vec![Block::Heading {
                level: 1,
                numbered: false,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = emit(&doc);
        assert!(output.contains("= Title ="));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = emit(&doc);
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = emit(&doc);
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_build_italic() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = emit(&doc);
        assert!(output.contains("//italic//"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = T2tDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let output = emit(&doc);
        assert!(output.contains("```"));
        assert!(output.contains("print hi"));
    }

    #[test]
    fn test_strip_spans() {
        let doc = T2tDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("hello".into(), Span::new(0, 5))],
                span: Span::new(0, 5),
            }],
            span: Span::new(0, 5),
        };
        let stripped = doc.strip_spans();
        assert_eq!(stripped.span, Span::NONE);
        if let Block::Paragraph { span, .. } = &stripped.blocks[0] {
            assert_eq!(*span, Span::NONE);
        }
    }
}
