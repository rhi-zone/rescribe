//! BBCode parser, AST, and emitter.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-bbcode` and `rescribe-write-bbcode` as thin adapter layers.
//!
//! # Public API
//!
//! - [`parse`] — parse a BBCode string, infallible, returns `(BbcodeDoc, Vec<Diagnostic>)`
//! - [`emit`] — emit a [`BbcodeDoc`] to a BBCode string
//! - [`events`] — parse and return a streaming iterator of [`Event`]s
//! - [`BbcodeDoc`], [`Block`], [`Inline`], [`TableRow`] — AST types
//! - [`Span`], [`Diagnostic`], [`Severity`] — metadata types
//! - [`BatchParser`], [`StreamingParser`], [`BatchSink`] — batch/streaming parsers
//! - [`Writer`] — streaming event-driven writer

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
pub mod writer;

// Re-export the most-used public types at crate root for convenience.
pub use ast::{AlignKind, BbcodeDoc, Block, Diagnostic, Inline, Severity, Span, TableRow};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use events::{Event, EventIter, OwnedEvent};
pub use writer::Writer;

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

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> events::EventIter<'_> {
    events::events(input)
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
    fn test_parse_heading() {
        let (doc, _) = parse("[h1]Title[/h1]");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(
            doc.blocks[0],
            Block::Heading { level: 1, .. }
        ));
    }

    #[test]
    fn test_parse_hr() {
        let (doc, _) = parse("[hr]");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::HorizontalRule { .. }));
    }

    #[test]
    fn test_parse_center() {
        let (doc, _) = parse("[center]\ncentered text\n[/center]");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(
            doc.blocks[0],
            Block::Alignment {
                kind: AlignKind::Center,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_spoiler() {
        let (doc, _) = parse("[spoiler]\nhidden text\n[/spoiler]");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Spoiler { .. }));
    }

    #[test]
    fn test_parse_font() {
        let (doc, _) = parse("[font=Arial]text[/font]");
        assert!(!doc.blocks.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(matches!(inlines[0], Inline::Font { .. }));
        }
    }

    #[test]
    fn test_parse_size() {
        let (doc, _) = parse("[size=14]text[/size]");
        assert!(!doc.blocks.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(matches!(inlines[0], Inline::Size { .. }));
        }
    }

    #[test]
    fn test_parse_email() {
        let (doc, _) = parse("[email]test@example.com[/email]");
        assert!(!doc.blocks.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(matches!(inlines[0], Inline::Email { .. }));
        }
    }

    #[test]
    fn test_parse_noparse() {
        let (doc, _) = parse("[noparse][b]not bold[/b][/noparse]");
        assert!(!doc.blocks.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(matches!(inlines[0], Inline::Noparse(..)));
        }
    }

    #[test]
    fn test_parse_img_dimensions() {
        let (doc, _) = parse("[img=100x50]https://example.com/img.png[/img]");
        assert!(!doc.blocks.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            if let Inline::Image {
                width, height, ..
            } = &inlines[0]
            {
                assert_eq!(*width, Some(100));
                assert_eq!(*height, Some(50));
            } else {
                panic!("Expected Image");
            }
        }
    }

    #[test]
    fn test_parse_named_quote() {
        let (doc, _) = parse("[quote=Author]\nquoted text\n[/quote]");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Blockquote { author, .. } = &doc.blocks[0] {
            assert_eq!(author.as_deref(), Some("Author"));
        } else {
            panic!("Expected Blockquote");
        }
    }

    #[test]
    fn test_parse_code_with_language() {
        let (doc, _) = parse("[code=rust]\nfn main() {}\n[/code]");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::CodeBlock { language, .. } = &doc.blocks[0] {
            assert_eq!(language.as_deref(), Some("rust"));
        } else {
            panic!("Expected CodeBlock");
        }
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
                language: None,
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

    #[test]
    fn test_events_basic() {
        let evts: Vec<_> = events("[b]hello[/b]").collect();
        assert!(evts.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(evts.iter().any(|e| matches!(e, Event::EndBold)));
    }
}

#[test]
fn test_timeout_case() {
    let input = "[codee:";
    let start = std::time::Instant::now();
    let (doc, _) = parse(input);
    let elapsed = start.elapsed();
    println!("elapsed: {:?}, blocks: {}", elapsed, doc.blocks.len());
    assert!(
        elapsed.as_millis() < 100,
        "parse took too long: {:?}",
        elapsed
    );
}

#[test]
fn parse_sample_no_panic() {
    // Smoke test: parse a representative BBCode sample and verify no panic.
    let sample = r#"
[h1]Welcome[/h1]

[b]Bold[/b] and [i]italic[/i] and [u]underline[/u].

[quote=Author]
This is a quote.
[/quote]

[list]
[*]Item 1
[*][b]Bold item[/b]
[/list]

[list=1]
[*]First
[*]Second
[/list]

[code=rust]
fn main() {}
[/code]

[table]
[tr][th]Header[/th][td]Cell[/td][/tr]
[/table]

[hr]

[center]
Centered text
[/center]

[color=red]Red[/color] [size=14]Big[/size] [font=Arial]Arial[/font]

[img]https://example.com/img.png[/img]
[img=100x50]https://example.com/img.png[/img]

[url=https://example.com]Link[/url]
[email]test@example.com[/email]

[spoiler]
Hidden content
[/spoiler]

[noparse][b]not bold[/b][/noparse]

[sub]sub[/sub] [sup]sup[/sup]

[pre]preformatted text[/pre]

[s]strikethrough[/s]

[youtube]dQw4w9WgXcQ[/youtube]
"#;
    let (doc, _) = parse(sample);
    assert!(doc.blocks.len() > 10, "should parse many blocks");

    // Also verify emit doesn't panic
    let _output = emit(&doc);

    // And events don't panic
    let evts: Vec<_> = events(sample).collect();
    assert!(!evts.is_empty());
}
