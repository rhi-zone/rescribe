//! POD (Plain Old Documentation) parser, emitter, and AST.
//!
//! Standalone crate with no rescribe dependency.
//!
//! # API
//!
//! ```rust
//! use pod_fmt::{parse, build};
//!
//! let (doc, _diagnostics) = parse("=head1 Title\n\nBody text.\n");
//! let output = build(&doc);
//! ```

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::{Block, Diagnostic, Inline, PodDoc, Severity, Span};
pub use emit::{build, collect_inline_text};
pub use parse::parse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("=head1 NAME\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { .. }));
    }

    #[test]
    fn test_parse_heading_level2() {
        let (doc, _) = parse("=head2 DESCRIPTION\n");
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 2);
        } else {
            panic!("expected heading");
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, _) = parse("=pod\n\nThis is a paragraph.\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("=pod\n\nThis is B<bold> text.\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("=pod\n\nThis is I<italic> text.\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("=pod\n\nUse C<my $var> here.\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Code(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("=pod\n\nSee L<perlpod> for details.\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_verbatim() {
        let (doc, _) = parse("=pod\n\n    print \"Hello\";\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_list() {
        let (doc, _) = parse("=over\n\n=item * First\n\n=item * Second\n\n=back\n");
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_parse_escape() {
        let (doc, _) = parse("=pod\n\nE<lt>tag E<gt>\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            let text = inlines
                .iter()
                .filter_map(|i| {
                    if let Inline::Text(s, _) = i {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>();
            assert!(text.contains('<'));
            assert!(text.contains('>'));
        }
    }

    #[test]
    fn test_parse_double_brackets() {
        let (doc, _) = parse("=pod\n\nC<< $a <=> $b >>\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            let code = inlines.iter().find(|i| matches!(i, Inline::Code(..)));
            assert!(code.is_some());
            if let Some(Inline::Code(content, _)) = code {
                assert!(content.contains("<=>"));
            }
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = PodDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("NAME".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("=head1 NAME"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("B<bold>"));
    }

    #[test]
    fn test_build_italic() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("I<italic>"));
    }

    #[test]
    fn test_build_code() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("$var".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("C<$var>"));
    }

    #[test]
    fn test_build_code_with_angle_brackets() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("$a <=> $b".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("C<< $a <=> $b >>"));
    }

    #[test]
    fn test_build_link() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "perlpod".into(),
                    label: "perlpod".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("L<perlpod>"));
    }

    #[test]
    fn test_build_link_with_label() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "perlpod".into(),
                    label: "documentation".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("L<documentation|perlpod>"));
    }

    #[test]
    fn test_build_list() {
        let doc = PodDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("one".into(), Span::NONE)],
                        span: Span::NONE,
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("two".into(), Span::NONE)],
                        span: Span::NONE,
                    }],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("=over"));
        assert!(out.contains("=item * one"));
        assert!(out.contains("=item * two"));
        assert!(out.contains("=back"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = PodDoc {
            blocks: vec![Block::CodeBlock {
                content: "print 'Hello';".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("    print 'Hello';"));
    }

    #[test]
    fn test_build_pod_cut() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Content".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.starts_with("=pod"));
        assert!(out.ends_with("=cut\n"));
    }
}
