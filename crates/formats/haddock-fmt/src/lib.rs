//! Haddock documentation format parser, emitter, and AST.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-haddock` and `rescribe-write-haddock` as thin adapter layers.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

pub use ast::{Block, Diagnostic, HaddockDoc, Inline, Severity, Span};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::{build, collect_inline_text};
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> events::EventIter<'_> {
    events::events(input)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let (doc, _) = parse("= Title\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_heading_levels() {
        let (doc, _) = parse("== Level 2\n=== Level 3\n");
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 2, .. }));
        assert!(matches!(doc.blocks[1], Block::Heading { level: 3, .. }));
    }

    #[test]
    fn test_parse_heading_h3() {
        let (doc, _) = parse("=== Sub-subsection\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 3, .. }));
    }

    #[test]
    fn test_parse_heading_h4() {
        let (doc, _) = parse("==== Deep heading\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { level: 4, .. }));
    }

    #[test]
    fn test_parse_paragraph() {
        let (doc, _) = parse("Hello world\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let (doc, _) = parse("__bold__\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Strong(_, _)));
    }

    #[test]
    fn test_parse_italic() {
        let (doc, _) = parse("/italic/\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Emphasis(_, _)));
    }

    #[test]
    fn test_parse_code() {
        let (doc, _) = parse("@code@\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(inlines[0], Inline::Code(_, _)));
    }

    #[test]
    fn test_parse_link() {
        let (doc, _) = parse("\"Example\"<https://example.com>\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        let link = &inlines[0];
        assert!(matches!(link, Inline::Link { .. }));
        if let Inline::Link { url, text, .. } = link {
            assert_eq!(url, "https://example.com");
            assert_eq!(text, "Example");
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let (doc, _) = parse("* item1\n* item2\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::UnorderedList { .. }));
        if let Block::UnorderedList { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        }
    }

    #[test]
    fn test_parse_code_block() {
        let (doc, _) = parse("> code here\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_at_code_block() {
        let (doc, _) = parse("@\nfoo bar\nbaz\n@\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::AtCodeBlock { .. }));
        if let Block::AtCodeBlock { content, .. } = &doc.blocks[0] {
            assert_eq!(content, "foo bar\nbaz");
        }
    }

    #[test]
    fn test_parse_doctest() {
        let (doc, _) = parse(">>> 1 + 1\n2\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::DocTest { .. }));
        if let Block::DocTest { expression, result, .. } = &doc.blocks[0] {
            assert_eq!(expression, "1 + 1");
            assert_eq!(result.as_deref(), Some("2"));
        }
    }

    #[test]
    fn test_parse_property_since() {
        let (doc, _) = parse("@since 4.2.0\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Property { key, description, .. } = &doc.blocks[0] {
            assert_eq!(key, "since");
            assert_eq!(description.len(), 1);
        } else {
            panic!("expected Property");
        }
    }

    #[test]
    fn test_parse_property_deprecated() {
        let (doc, _) = parse("@deprecated Use newFunc instead\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Property { key, .. } = &doc.blocks[0] {
            assert_eq!(key, "deprecated");
        } else {
            panic!("expected Property");
        }
    }

    #[test]
    fn test_parse_property_param() {
        let (doc, _) = parse("@param x The input value\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Property { key, name, .. } = &doc.blocks[0] {
            assert_eq!(key, "param");
            assert_eq!(name.as_deref(), Some("x"));
        } else {
            panic!("expected Property");
        }
    }

    #[test]
    fn test_parse_property_returns() {
        let (doc, _) = parse("@returns The computed result\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Property { key, .. } = &doc.blocks[0] {
            assert_eq!(key, "returns");
        } else {
            panic!("expected Property");
        }
    }

    #[test]
    fn test_parse_module_link() {
        let (doc, _) = parse("See \"Data.Map\" for details.\n");
        let Block::Paragraph { inlines, .. } = &doc.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::ModuleLink { .. })));
    }

    #[test]
    fn test_build_heading() {
        let doc = HaddockDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Title".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("= Title"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = HaddockDoc {
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
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(
                    vec![Inline::Text("bold".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("__bold__"));
    }

    #[test]
    fn test_build_italic() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Emphasis(
                    vec![Inline::Text("italic".into(), Span::NONE)],
                    Span::NONE,
                )],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("/italic/"));
    }

    #[test]
    fn test_build_code() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("code".into(), Span::NONE)],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("@code@"));
    }

    #[test]
    fn test_build_link() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".into(),
                    text: "click".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("\"click\"<https://example.com>"));
    }

    #[test]
    fn test_build_unordered_list() {
        let doc = HaddockDoc {
            blocks: vec![Block::UnorderedList {
                items: vec![
                    vec![Inline::Text("one".into(), Span::NONE)],
                    vec![Inline::Text("two".into(), Span::NONE)],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("* one"));
        assert!(out.contains("* two"));
    }

    #[test]
    fn test_build_ordered_list() {
        let doc = HaddockDoc {
            blocks: vec![Block::OrderedList {
                items: vec![
                    vec![Inline::Text("first".into(), Span::NONE)],
                    vec![Inline::Text("second".into(), Span::NONE)],
                ],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("(1) first"));
        assert!(out.contains("(2) second"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = HaddockDoc {
            blocks: vec![Block::CodeBlock {
                content: "print hi".into(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("> print hi"));
    }

    #[test]
    fn test_build_module_link() {
        let doc = HaddockDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::ModuleLink {
                    module: "Data.Map".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = build(&doc);
        assert!(out.contains("\"Data.Map\""));
    }

    #[test]
    fn test_events_roundtrip() {
        let input = "= Hello\n\nWorld.\n";
        let evs: Vec<OwnedEvent> = events(input).map(|e| e.into_owned()).collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_writer_roundtrip() {
        let input = "= Hello\n\nWorld.\n";
        let evs: Vec<OwnedEvent> = events(input).map(|e| e.into_owned()).collect();

        let mut w = Writer::new(Vec::<u8>::new());
        for ev in evs {
            w.write_event(ev);
        }
        let bytes = w.finish();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains("= Hello"));
        assert!(output.contains("World."));
    }

    #[test]
    fn test_parse_sample_no_panic() {
        let long = "a".repeat(100_000);
        let samples: &[&str] = &[
            "",
            "Hello World",
            "= Heading",
            "== Level 2",
            "=== Level 3",
            "__bold__ /italic/ @code@",
            "* item 1\n* item 2",
            "(1) first\n(2) second",
            "> code line\n> another",
            "[term] description",
            ">>> 1 + 1\n2",
            "@since 1.0",
            "@deprecated",
            "@param x desc",
            "@returns desc",
            "\"Data.Map\"",
            "<https://example.com>",
            "\"link\"<https://example.com>",
            "@\ncode block\n@",
            "This is __bold__ /italic/ @code@ text.\n\n= Heading\n\n* item",
            // Adversarial
            "____",
            "//",
            "@@",
            "''",
            ">>>",
            "<<<",
            "=",
            "======",
            "[",
            "[]",
            "[[[]]]",
            "\n\n\n\n",
            &long,
        ];
        for sample in samples {
            let _ = parse(sample);
        }
    }
}
