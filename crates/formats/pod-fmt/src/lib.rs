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

#[cfg(feature = "reader-streaming")]
pub mod events;

#[cfg(feature = "reader-batch")]
pub mod batch;

#[cfg(feature = "writer-streaming")]
pub mod writer;

pub use ast::{Block, DefinitionItem, Diagnostic, Inline, PodDoc, Severity, Span};
pub use emit::{build, collect_inline_text};
pub use parse::parse;

#[cfg(feature = "reader-streaming")]
pub use events::{Event, EventIter, OwnedEvent};

#[cfg(feature = "reader-batch")]
pub use batch::{BatchParser, BatchSink, Handler, StreamingParser};

#[cfg(feature = "writer-streaming")]
pub use writer::Writer;

/// Parse `input` and return a streaming iterator of [`Event`] items.
#[cfg(feature = "reader-streaming")]
pub fn events(input: &str) -> OwnedEventIter {
    let (doc, _) = parse::parse(input);
    OwnedEventIter { doc, pos: 0, events: None }
}

/// An owned event iterator that holds the parsed doc and yields events.
#[cfg(feature = "reader-streaming")]
pub struct OwnedEventIter {
    doc: PodDoc,
    pos: usize,
    events: Option<Vec<OwnedEvent>>,
}

#[cfg(feature = "reader-streaming")]
impl Iterator for OwnedEventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        if self.events.is_none() {
            // Collect all events eagerly (we own the doc, can't borrow from it
            // while also being an iterator over ourselves).
            let evts: Vec<OwnedEvent> =
                events::EventIter::new(&self.doc).map(|e| e.into_owned()).collect();
            self.events = Some(evts);
        }
        let evts = self.events.as_ref().unwrap();
        if self.pos < evts.len() {
            let evt = evts[self.pos].clone();
            self.pos += 1;
            Some(evt)
        } else {
            None
        }
    }
}

// We need Clone on Event for OwnedEventIter
// Let's add it via a manual impl since Event has Cow fields
#[cfg(feature = "reader-streaming")]
impl Clone for OwnedEvent {
    fn clone(&self) -> Self {
        match self {
            Event::StartParagraph => Event::StartParagraph,
            Event::EndParagraph => Event::EndParagraph,
            Event::StartHeading { level } => Event::StartHeading { level: *level },
            Event::EndHeading => Event::EndHeading,
            Event::CodeBlock { content } => {
                Event::CodeBlock { content: content.clone() }
            }
            Event::StartList { ordered } => Event::StartList { ordered: *ordered },
            Event::EndList => Event::EndList,
            Event::StartListItem => Event::StartListItem,
            Event::EndListItem => Event::EndListItem,
            Event::StartDefinitionList => Event::StartDefinitionList,
            Event::EndDefinitionList => Event::EndDefinitionList,
            Event::StartDefinitionTerm => Event::StartDefinitionTerm,
            Event::EndDefinitionTerm => Event::EndDefinitionTerm,
            Event::StartDefinitionDesc => Event::StartDefinitionDesc,
            Event::EndDefinitionDesc => Event::EndDefinitionDesc,
            Event::RawBlock { format, content } => {
                Event::RawBlock { format: format.clone(), content: content.clone() }
            }
            Event::ForBlock { format, content } => {
                Event::ForBlock { format: format.clone(), content: content.clone() }
            }
            Event::Encoding { encoding } => Event::Encoding { encoding: encoding.clone() },
            Event::Text(cow) => Event::Text(cow.clone()),
            Event::StartBold => Event::StartBold,
            Event::EndBold => Event::EndBold,
            Event::StartItalic => Event::StartItalic,
            Event::EndItalic => Event::EndItalic,
            Event::StartUnderline => Event::StartUnderline,
            Event::EndUnderline => Event::EndUnderline,
            Event::StartFilename => Event::StartFilename,
            Event::EndFilename => Event::EndFilename,
            Event::StartNonBreaking => Event::StartNonBreaking,
            Event::EndNonBreaking => Event::EndNonBreaking,
            Event::InlineCode(cow) => Event::InlineCode(cow.clone()),
            Event::StartLink { url, label } => {
                Event::StartLink { url: url.clone(), label: label.clone() }
            }
            Event::EndLink => Event::EndLink,
            Event::IndexEntry(s) => Event::IndexEntry(s.clone()),
            Event::Null => Event::Null,
            Event::Entity(s) => Event::Entity(s.clone()),
        }
    }
}

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
    fn test_parse_filename() {
        let (doc, _) = parse("=pod\n\nSee F<config.txt> for details.\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Filename(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_non_breaking() {
        let (doc, _) = parse("=pod\n\nUse S<no break here> please.\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::NonBreaking(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_index_entry() {
        let (doc, _) = parse("=pod\n\nSee X<term> here.\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::IndexEntry(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_null() {
        let (doc, _) = parse("=pod\n\nSee Z<> here.\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Null(..))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_begin_end() {
        let (doc, _) = parse("=begin html\n\n<p>Raw HTML</p>\n\n=end html\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::RawBlock { .. }));
    }

    #[test]
    fn test_parse_for() {
        let (doc, _) = parse("=for html <br/>\n");
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::ForBlock { .. }));
    }

    #[test]
    fn test_parse_encoding() {
        let (doc, _) = parse("=encoding UTF-8\n");
        assert_eq!(doc.blocks.len(), 1);
        if let Block::Encoding { encoding, .. } = &doc.blocks[0] {
            assert_eq!(encoding, "UTF-8");
        } else {
            panic!("expected encoding");
        }
    }

    #[test]
    fn test_parse_definition_list() {
        let (doc, _) = parse("=over 4\n\n=item Term\n\nDescription.\n\n=back\n");
        assert!(matches!(doc.blocks[0], Block::DefinitionList { .. }));
        if let Block::DefinitionList { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 1);
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let (doc, _) = parse(
            "=over\n\n=item * Outer\n\n=over\n\n=item * Inner\n\n=back\n\n=back\n",
        );
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 1);
            assert!(items[0].iter().any(|b| matches!(b, Block::List { .. })));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_parse_nested_formatting() {
        let (doc, _) = parse("=pod\n\nB<I<bold italic>>\n");
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            if let Inline::Bold(ch, _) = &inlines[0] {
                assert!(ch.iter().any(|i| matches!(i, Inline::Italic(..))));
            } else {
                panic!("expected bold");
            }
        } else {
            panic!("expected paragraph");
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
        assert!(out.contains("=item *"));
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
