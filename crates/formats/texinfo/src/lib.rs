//! Texinfo parser, AST, and emitter.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-texinfo` and `rescribe-write-texinfo` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::*;
pub use emit::emit;
pub use parse::parse;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = r#"@chapter Introduction
This is the introduction paragraph.

@section Getting Started
Here is how to get started."#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_headings() {
        let input = r#"@chapter Chapter One
@section Section One
@subsection Subsection One
@subsubsection Sub-subsection"#;

        let (doc, _diags) = parse(input);
        assert_eq!(doc.blocks.len(), 4);
    }

    #[test]
    fn test_parse_emphasis() {
        let input = r#"This is @emph{emphasized} and @strong{bold} text."#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let input = r#"Use @code{printf} to print."#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let input = r#"@itemize
@item First item
@item Second item
@end itemize"#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::List { .. }));
    }

    #[test]
    fn test_parse_enumerate() {
        let input = r#"@enumerate
@item First
@item Second
@end enumerate"#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::List { ordered: true, .. }));
    }

    #[test]
    fn test_parse_example() {
        let input = r#"@example
int main() {
    return 0;
}
@end example"#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_url() {
        let input = r#"Visit @uref{https://example.com, Example Site}."#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_quotation() {
        let input = r#"@quotation
This is a quoted passage.
@end quotation"#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_skip_comments() {
        let input = r#"@c This is a comment
This is visible.
@comment Another comment
Still visible."#;

        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_emit_header() {
        let doc = TexinfoDoc {
            title: Some("Test".to_string()),
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.starts_with("\\input texinfo"));
        assert!(out.ends_with("@bye\n"));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = TexinfoDoc {
            title: None,
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
    fn test_emit_strong() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("@strong{bold}"));
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Emphasis(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("@emph{italic}"));
    }

    #[test]
    fn test_emit_code() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("printf".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("@code{printf}"));
    }

    #[test]
    fn test_emit_link() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    children: vec![Inline::Text("Example".into(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("@uref{https://example.com, Example}"));
    }

    #[test]
    fn test_emit_list() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Inline::Text("one".into(), Span::NONE)],
                    vec![Inline::Text("two".into(), Span::NONE)],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("@itemize @bullet"));
        assert!(out.contains("@item one"));
        assert!(out.contains("@item two"));
        assert!(out.contains("@end itemize"));
    }

    #[test]
    fn test_emit_enumerate() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::List {
                ordered: true,
                items: vec![
                    vec![Inline::Text("first".into(), Span::NONE)],
                    vec![Inline::Text("second".into(), Span::NONE)],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("@enumerate"));
        assert!(out.contains("@item first"));
        assert!(out.contains("@end enumerate"));
    }

    #[test]
    fn test_emit_code_block() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::CodeBlock {
                content: "int main() {}".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("@example"));
        assert!(out.contains("int main() {}"));
        assert!(out.contains("@end example"));
    }

    #[test]
    fn test_emit_blockquote() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Blockquote {
                children: vec![Block::Paragraph {
                    inlines: vec![Inline::Text("Quoted text".into(), Span::NONE)],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("@quotation"));
        assert!(out.contains("Quoted text"));
        assert!(out.contains("@end quotation"));
    }

    #[test]
    fn test_escape_special_chars() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Use @{braces}".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        // @ -> @@, { -> @{, } -> @}
        assert!(out.contains("Use @@@{braces@}"));
    }

    #[test]
    fn test_parse_settitle() {
        let input = "@settitle My Book\n\n@chapter Introduction\nContent here.";
        let (doc, _diags) = parse(input);
        assert_eq!(doc.title, Some("My Book".to_string()));
    }

    #[test]
    fn test_strip_spans() {
        let doc = TexinfoDoc {
            title: Some("Test".to_string()),
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("hello".into(), Span { start: 5, end: 10 })],
                span: Span { start: 1, end: 20 },
            }],
            span: Span { start: 0, end: 100 },
        };
        let stripped = doc.strip_spans();
        assert_eq!(stripped.span, Span::NONE);
        assert_eq!(
            stripped.blocks[0],
            Block::Paragraph {
                inlines: vec![Inline::Text("hello".into(), Span::NONE)],
                span: Span::NONE,
            }
        );
    }
}
