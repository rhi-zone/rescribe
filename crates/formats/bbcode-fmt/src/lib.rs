//! BBCode parser, AST, and emitter.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-bbcode` and `rescribe-write-bbcode` as thin adapter layers.
//!
//! # Public API
//!
//! - [`parse`] — parse a BBCode string, infallible, returns `(BbcodeDoc, Vec<Diagnostic>)`
//! - [`emit`] — emit a [`BbcodeDoc`] to a BBCode string
//! - [`BbcodeDoc`], [`Block`], [`Inline`], [`TableRow`] — AST types
//! - [`Span`], [`Diagnostic`], [`Severity`] — metadata types

pub mod ast;
pub mod emit;
pub mod parse;

// Re-export the most-used public types at crate root for convenience.
pub use ast::{BbcodeDoc, Block, Diagnostic, Inline, Severity, Span, TableRow};

/// Parse a BBCode string into a [`BbcodeDoc`].
///
/// Always succeeds.  Malformed markup is tolerated; any detected problems are
/// returned in the `Vec<Diagnostic>`.
pub fn parse(input: &str) -> (BbcodeDoc, Vec<Diagnostic>) {
    parse::parse(input)
}

/// Emit a [`BbcodeDoc`] to a BBCode string.
pub fn emit(doc: &BbcodeDoc) -> String {
    emit::emit(doc)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("This is [b]bold[/b] text");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("This is [i]italic[/i] text");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("[url=http://example.com]Example[/url]");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse("[list]\n[*]Item 1\n[*]Item 2\n[/list]");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("[code]print('hello')[/code]");
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = BbcodeDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = BbcodeDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("[b]bold[/b]"));
    }

    #[test]
    fn test_emit_link() {
        let doc = BbcodeDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    children: vec![Inline::Text("Example".into(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("[url=http://example.com]"));
        assert!(out.contains("Example"));
        assert!(out.contains("[/url]"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = BbcodeDoc {
            blocks: vec![Block::CodeBlock {
                content: "print('hello')".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("[code]"));
        assert!(out.contains("print('hello')"));
        assert!(out.contains("[/code]"));
    }

    #[test]
    fn test_emit_list() {
        let doc = BbcodeDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Inline::Text("Item 1".into(), Span::NONE)],
                    vec![Inline::Text("Item 2".into(), Span::NONE)],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("[list]"));
        assert!(out.contains("[*]"));
        assert!(out.contains("[/list]"));
    }

    #[test]
    fn test_roundtrip_bold() {
        let input = "Text with [b]bold[/b] word";
        let (doc, _) = parse(input);
        let output = emit(&doc);
        assert!(output.contains("[b]"));
        assert!(output.contains("bold"));
        assert!(output.contains("[/b]"));
    }
}
