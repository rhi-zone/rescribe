//! Texinfo parser, AST, and emitter.
//!
//! Standalone crate with no rescribe dependency by default. The optional
//! `rescribe` feature (default off) adds `crate::rescribe::{parse, emit}`,
//! a thin adapter that translates `texinfo::TexinfoDoc` to and from
//! rescribe's `Document` IR.

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

// Re-export everything callers need.
pub use ast::{
    Block, CodeBlockVariant, CrossRefKind, Diagnostic, HeadingKind, Inline, MenuEntry, Severity,
    Span, SymbolKind, TableRow, TexinfoDoc,
};
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};
pub use emit::emit;
pub use events::{Event, EventIter, OwnedEvent};
pub use parse::parse;
pub use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── rescribe-format-api trait implementations ───────────────────────────────
//
// `TexinfoDoc` is not lifetime-generic (unlike `RstDoc<'a>`), so it doesn't
// hit rst-fmt's structural wall — but `crate::parse::parse`/`crate::events::events`
// take `&str`, not the `&[u8]` the shared `Parse`/`Events` traits require.
// Texinfo source is plain text, and `BatchParser::finish`/`StreamingParser::feed`
// already bridge `&[u8]` -> text via `String::from_utf8_lossy` internally (see
// `batch.rs`) — this is an established, always-valid conversion for this
// format, not a structural mismatch, so `Parse`/`Events` are implemented by
// applying the same bridge, matching precedent.
//
// `crate::events::events` eagerly parses into a pre-computed `Vec<OwnedEvent>`
// regardless of input lifetime (see `events.rs`'s `EventIter`), so `Events`'s
// associated `Event<'a>` is `OwnedEvent` unconditionally here — the same
// "owned regardless of `'a`" case `rescribe_format_api::Events`'s docs
// anticipate for crates like `zip-fmt`.

impl Parse for TexinfoDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(input);
        parse::parse(&s)
    }
}

impl Emit for TexinfoDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self).into_bytes()
    }
}

/// Owned-event iterator backing [`TexinfoDoc`]'s [`Events`] impl. A thin
/// wrapper over `Vec<OwnedEvent>::into_iter` — see the module-level note on
/// why `Event<'a>` is `OwnedEvent` here regardless of `'a`.
pub struct EventIterOwned(std::vec::IntoIter<OwnedEvent>);

impl Iterator for EventIterOwned {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl Events for TexinfoDoc {
    type Event<'a> = OwnedEvent;
    type EventIter<'a> = EventIterOwned;

    fn events(input: &[u8]) -> Self::EventIter<'_> {
        let s = String::from_utf8_lossy(input);
        let owned: Vec<OwnedEvent> = events::events(&s).map(|e| e.into_owned()).collect();
        EventIterOwned(owned.into_iter())
    }
}

impl StreamingParse for TexinfoDoc {
    type Event = OwnedEvent;
    type Parser<H: Handler<OwnedEvent>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<OwnedEvent>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for TexinfoDoc {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

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
                variant: CodeBlockVariant::Example,
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

    #[test]
    fn test_parse_semantic_inlines() {
        let input = "@file{test.c} and @command{gcc} and @option{-O2} and @env{PATH}";
        let (doc, _diags) = parse(input);
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::File(..))));
            assert!(inlines.iter().any(|i| matches!(i, Inline::Command(..))));
            assert!(inlines.iter().any(|i| matches!(i, Inline::Option(..))));
            assert!(inlines.iter().any(|i| matches!(i, Inline::Env(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_symbols() {
        let input = "Use @dots{} and @copyright{} and @TeX{}.";
        let (doc, _diags) = parse(input);
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(
                inlines
                    .iter()
                    .any(|i| matches!(i, Inline::Symbol(SymbolKind::Dots, _)))
            );
            assert!(
                inlines
                    .iter()
                    .any(|i| matches!(i, Inline::Symbol(SymbolKind::Copyright, _)))
            );
            assert!(
                inlines
                    .iter()
                    .any(|i| matches!(i, Inline::Symbol(SymbolKind::TeX, _)))
            );
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_crossref() {
        let input = "@xref{Introduction} and @ref{Overview, display text}.";
        let (doc, _diags) = parse(input);
        if let Block::Paragraph { ref inlines, .. } = doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(
                i,
                Inline::CrossRef {
                    kind: CrossRefKind::Xref,
                    node,
                    ..
                } if node == "Introduction"
            )));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_multitable() {
        let input = "@multitable @columnfractions .5 .5\n@headitem Name @tab Age\n@item Alice @tab 30\n@end multitable";
        let (doc, _diags) = parse(input);
        assert!(matches!(doc.blocks[0], Block::Table { .. }));
    }

    #[test]
    fn test_parse_menu() {
        let input = "@menu\n* Introduction:: The intro\n* Advanced:: Advanced topics\n@end menu";
        let (doc, _diags) = parse(input);
        assert!(matches!(doc.blocks[0], Block::Menu { .. }));
    }
}
