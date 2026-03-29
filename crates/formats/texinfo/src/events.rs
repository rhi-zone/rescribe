//! Streaming event iterator over a parsed Texinfo document.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a Texinfo document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading {
        level: u8,
        kind: HeadingKind,
    },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList {
        ordered: bool,
    },
    EndList,
    StartListItem,
    EndListItem,
    CodeBlock {
        variant: CodeBlockVariant,
        content: Cow<'a, str>,
    },
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartTable,
    EndTable,
    StartTableRow {
        is_header: bool,
    },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartMenu,
    EndMenu,
    MenuEntry {
        node: String,
        description: Option<String>,
    },
    HorizontalRule,
    RawBlock {
        environment: String,
        content: String,
    },
    StartFloat {
        float_type: Option<String>,
        label: Option<String>,
    },
    EndFloat,
    NoIndent,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    SoftBreak,
    LineBreak,
    StartStrong,
    EndStrong,
    StartEmphasis,
    EndEmphasis,
    InlineCode(Cow<'a, str>),
    StartVar,
    EndVar,
    File(Cow<'a, str>),
    Command(Cow<'a, str>),
    Option(Cow<'a, str>),
    Env(Cow<'a, str>),
    Samp(Cow<'a, str>),
    Kbd(Cow<'a, str>),
    Key(Cow<'a, str>),
    StartDfn,
    EndDfn,
    Cite(Cow<'a, str>),
    Acronym {
        abbrev: String,
        expansion: Option<String>,
    },
    Abbr {
        abbrev: String,
        expansion: Option<String>,
    },
    Roman(Cow<'a, str>),
    SmallCaps(Cow<'a, str>),
    StartDirectItalic,
    EndDirectItalic,
    StartDirectBold,
    EndDirectBold,
    DirectTypewriter(Cow<'a, str>),
    StartLink {
        url: String,
    },
    EndLink,
    Image {
        file: String,
        width: Option<String>,
        height: Option<String>,
        alt: Option<String>,
        extension: Option<String>,
    },
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    StartFootnoteDef,
    EndFootnoteDef,
    CrossRef {
        kind: CrossRefKind,
        node: String,
        text: Option<String>,
    },
    Anchor {
        name: String,
    },
    NoBreak(Cow<'a, str>),
    Email {
        address: String,
        text: Option<String>,
    },
    Symbol(SymbolKind),
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { variant, content } => Event::CodeBlock {
                variant,
                content: Cow::Owned(content.into_owned()),
            },
            Event::File(cow) => Event::File(Cow::Owned(cow.into_owned())),
            Event::Command(cow) => Event::Command(Cow::Owned(cow.into_owned())),
            Event::Option(cow) => Event::Option(Cow::Owned(cow.into_owned())),
            Event::Env(cow) => Event::Env(Cow::Owned(cow.into_owned())),
            Event::Samp(cow) => Event::Samp(Cow::Owned(cow.into_owned())),
            Event::Kbd(cow) => Event::Kbd(Cow::Owned(cow.into_owned())),
            Event::Key(cow) => Event::Key(Cow::Owned(cow.into_owned())),
            Event::Cite(cow) => Event::Cite(Cow::Owned(cow.into_owned())),
            Event::Roman(cow) => Event::Roman(Cow::Owned(cow.into_owned())),
            Event::SmallCaps(cow) => Event::SmallCaps(Cow::Owned(cow.into_owned())),
            Event::DirectTypewriter(cow) => Event::DirectTypewriter(Cow::Owned(cow.into_owned())),
            Event::NoBreak(cow) => Event::NoBreak(Cow::Owned(cow.into_owned())),
            // All remaining variants contain only owned types or no data.
            // Safety: transmute is safe because the only lifetime-bearing fields
            // have been handled above.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// ── Event generation from AST ────────────────────────────────────────────────

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

/// Pull-based event iterator. Parses the full input eagerly (Texinfo requires
/// full input), then yields events lazily from the AST.
pub struct EventIter<'a> {
    /// Pre-computed event list.
    events: Vec<OwnedEvent>,
    /// Current position in the event list.
    pos: usize,
    /// Keep the input alive for the lifetime parameter.
    _input: &'a str,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a str) -> Self {
        let (doc, _diags) = crate::parse::parse(input);
        let mut events = Vec::new();
        emit_doc_events(&doc, &mut events);
        EventIter {
            events,
            pos: 0,
            _input: input,
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.events.len() {
            let idx = self.pos;
            self.pos += 1;
            // Clone is needed because we yield from a pre-built vec.
            // Events are small; this is acceptable for correctness.
            Some(clone_event(&self.events[idx]))
        } else {
            None
        }
    }
}

fn clone_event(e: &OwnedEvent) -> OwnedEvent {
    match e {
        Event::StartParagraph => Event::StartParagraph,
        Event::EndParagraph => Event::EndParagraph,
        Event::StartHeading { level, kind } => Event::StartHeading {
            level: *level,
            kind: kind.clone(),
        },
        Event::EndHeading => Event::EndHeading,
        Event::StartBlockquote => Event::StartBlockquote,
        Event::EndBlockquote => Event::EndBlockquote,
        Event::StartList { ordered } => Event::StartList { ordered: *ordered },
        Event::EndList => Event::EndList,
        Event::StartListItem => Event::StartListItem,
        Event::EndListItem => Event::EndListItem,
        Event::CodeBlock { variant, content } => Event::CodeBlock {
            variant: variant.clone(),
            content: Cow::Owned(content.clone().into_owned()),
        },
        Event::StartDefinitionList => Event::StartDefinitionList,
        Event::EndDefinitionList => Event::EndDefinitionList,
        Event::StartDefinitionTerm => Event::StartDefinitionTerm,
        Event::EndDefinitionTerm => Event::EndDefinitionTerm,
        Event::StartDefinitionDesc => Event::StartDefinitionDesc,
        Event::EndDefinitionDesc => Event::EndDefinitionDesc,
        Event::StartTable => Event::StartTable,
        Event::EndTable => Event::EndTable,
        Event::StartTableRow { is_header } => Event::StartTableRow {
            is_header: *is_header,
        },
        Event::EndTableRow => Event::EndTableRow,
        Event::StartTableCell => Event::StartTableCell,
        Event::EndTableCell => Event::EndTableCell,
        Event::StartMenu => Event::StartMenu,
        Event::EndMenu => Event::EndMenu,
        Event::MenuEntry { node, description } => Event::MenuEntry {
            node: node.clone(),
            description: description.clone(),
        },
        Event::HorizontalRule => Event::HorizontalRule,
        Event::RawBlock { environment, content } => Event::RawBlock {
            environment: environment.clone(),
            content: content.clone(),
        },
        Event::StartFloat { float_type, label } => Event::StartFloat {
            float_type: float_type.clone(),
            label: label.clone(),
        },
        Event::EndFloat => Event::EndFloat,
        Event::NoIndent => Event::NoIndent,
        Event::Text(s) => Event::Text(Cow::Owned(s.clone().into_owned())),
        Event::SoftBreak => Event::SoftBreak,
        Event::LineBreak => Event::LineBreak,
        Event::StartStrong => Event::StartStrong,
        Event::EndStrong => Event::EndStrong,
        Event::StartEmphasis => Event::StartEmphasis,
        Event::EndEmphasis => Event::EndEmphasis,
        Event::InlineCode(s) => Event::InlineCode(Cow::Owned(s.clone().into_owned())),
        Event::StartVar => Event::StartVar,
        Event::EndVar => Event::EndVar,
        Event::File(s) => Event::File(Cow::Owned(s.clone().into_owned())),
        Event::Command(s) => Event::Command(Cow::Owned(s.clone().into_owned())),
        Event::Option(s) => Event::Option(Cow::Owned(s.clone().into_owned())),
        Event::Env(s) => Event::Env(Cow::Owned(s.clone().into_owned())),
        Event::Samp(s) => Event::Samp(Cow::Owned(s.clone().into_owned())),
        Event::Kbd(s) => Event::Kbd(Cow::Owned(s.clone().into_owned())),
        Event::Key(s) => Event::Key(Cow::Owned(s.clone().into_owned())),
        Event::StartDfn => Event::StartDfn,
        Event::EndDfn => Event::EndDfn,
        Event::Cite(s) => Event::Cite(Cow::Owned(s.clone().into_owned())),
        Event::Acronym { abbrev, expansion } => Event::Acronym {
            abbrev: abbrev.clone(),
            expansion: expansion.clone(),
        },
        Event::Abbr { abbrev, expansion } => Event::Abbr {
            abbrev: abbrev.clone(),
            expansion: expansion.clone(),
        },
        Event::Roman(s) => Event::Roman(Cow::Owned(s.clone().into_owned())),
        Event::SmallCaps(s) => Event::SmallCaps(Cow::Owned(s.clone().into_owned())),
        Event::StartDirectItalic => Event::StartDirectItalic,
        Event::EndDirectItalic => Event::EndDirectItalic,
        Event::StartDirectBold => Event::StartDirectBold,
        Event::EndDirectBold => Event::EndDirectBold,
        Event::DirectTypewriter(s) => {
            Event::DirectTypewriter(Cow::Owned(s.clone().into_owned()))
        }
        Event::StartLink { url } => Event::StartLink { url: url.clone() },
        Event::EndLink => Event::EndLink,
        Event::Image { file, width, height, alt, extension } => Event::Image {
            file: file.clone(),
            width: width.clone(),
            height: height.clone(),
            alt: alt.clone(),
            extension: extension.clone(),
        },
        Event::StartSuperscript => Event::StartSuperscript,
        Event::EndSuperscript => Event::EndSuperscript,
        Event::StartSubscript => Event::StartSubscript,
        Event::EndSubscript => Event::EndSubscript,
        Event::StartFootnoteDef => Event::StartFootnoteDef,
        Event::EndFootnoteDef => Event::EndFootnoteDef,
        Event::CrossRef { kind, node, text } => Event::CrossRef {
            kind: kind.clone(),
            node: node.clone(),
            text: text.clone(),
        },
        Event::Anchor { name } => Event::Anchor { name: name.clone() },
        Event::NoBreak(s) => Event::NoBreak(Cow::Owned(s.clone().into_owned())),
        Event::Email { address, text } => Event::Email {
            address: address.clone(),
            text: text.clone(),
        },
        Event::Symbol(kind) => Event::Symbol(kind.clone()),
    }
}

// ── AST → Events ─────────────────────────────────────────────────────────────

fn emit_doc_events(doc: &TexinfoDoc, out: &mut Vec<OwnedEvent>) {
    for block in &doc.blocks {
        emit_block_events(block, out);
    }
}

fn emit_block_events(block: &Block, out: &mut Vec<OwnedEvent>) {
    match block {
        Block::Heading { level, kind, inlines, .. } => {
            out.push(Event::StartHeading {
                level: *level,
                kind: kind.clone(),
            });
            for inline in inlines {
                emit_inline_events(inline, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for inline in inlines {
                emit_inline_events(inline, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock { variant, content, .. } => {
            out.push(Event::CodeBlock {
                variant: variant.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                emit_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for inline in item {
                    emit_inline_events(inline, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for (term, desc_blocks) in items {
                out.push(Event::StartDefinitionTerm);
                for inline in term {
                    emit_inline_events(inline, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for desc_block in desc_blocks {
                    emit_block_events(desc_block, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow {
                    is_header: row.is_header,
                });
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    for inline in cell {
                        emit_inline_events(inline, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::Menu { entries, .. } => {
            out.push(Event::StartMenu);
            for entry in entries {
                out.push(Event::MenuEntry {
                    node: entry.node.clone(),
                    description: entry.description.clone(),
                });
            }
            out.push(Event::EndMenu);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
        Block::RawBlock { environment, content, .. } => {
            out.push(Event::RawBlock {
                environment: environment.clone(),
                content: content.clone(),
            });
        }
        Block::Float { float_type, label, children, .. } => {
            out.push(Event::StartFloat {
                float_type: float_type.clone(),
                label: label.clone(),
            });
            for child in children {
                emit_block_events(child, out);
            }
            out.push(Event::EndFloat);
        }
        Block::NoIndent { .. } => {
            out.push(Event::NoIndent);
        }
    }
}

fn emit_inline_events(inline: &Inline, out: &mut Vec<OwnedEvent>) {
    match inline {
        Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
        Inline::Strong(children, _) => {
            out.push(Event::StartStrong);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndStrong);
        }
        Inline::Emphasis(children, _) => {
            out.push(Event::StartEmphasis);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndEmphasis);
        }
        Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
        Inline::Var(children, _) => {
            out.push(Event::StartVar);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndVar);
        }
        Inline::File(s, _) => out.push(Event::File(Cow::Owned(s.clone()))),
        Inline::Command(s, _) => out.push(Event::Command(Cow::Owned(s.clone()))),
        Inline::Option(s, _) => out.push(Event::Option(Cow::Owned(s.clone()))),
        Inline::Env(s, _) => out.push(Event::Env(Cow::Owned(s.clone()))),
        Inline::Samp(s, _) => out.push(Event::Samp(Cow::Owned(s.clone()))),
        Inline::Kbd(s, _) => out.push(Event::Kbd(Cow::Owned(s.clone()))),
        Inline::Key(s, _) => out.push(Event::Key(Cow::Owned(s.clone()))),
        Inline::Dfn(children, _) => {
            out.push(Event::StartDfn);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndDfn);
        }
        Inline::Cite(s, _) => out.push(Event::Cite(Cow::Owned(s.clone()))),
        Inline::Acronym { abbrev, expansion, .. } => {
            out.push(Event::Acronym {
                abbrev: abbrev.clone(),
                expansion: expansion.clone(),
            });
        }
        Inline::Abbr { abbrev, expansion, .. } => {
            out.push(Event::Abbr {
                abbrev: abbrev.clone(),
                expansion: expansion.clone(),
            });
        }
        Inline::Roman(s, _) => out.push(Event::Roman(Cow::Owned(s.clone()))),
        Inline::SmallCaps(s, _) => out.push(Event::SmallCaps(Cow::Owned(s.clone()))),
        Inline::DirectItalic(children, _) => {
            out.push(Event::StartDirectItalic);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndDirectItalic);
        }
        Inline::DirectBold(children, _) => {
            out.push(Event::StartDirectBold);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndDirectBold);
        }
        Inline::DirectTypewriter(s, _) => {
            out.push(Event::DirectTypewriter(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image { file, width, height, alt, extension, .. } => {
            out.push(Event::Image {
                file: file.clone(),
                width: width.clone(),
                height: height.clone(),
                alt: alt.clone(),
                extension: extension.clone(),
            });
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndSubscript);
        }
        Inline::LineBreak { .. } => out.push(Event::LineBreak),
        Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
        Inline::FootnoteDef { content, .. } => {
            out.push(Event::StartFootnoteDef);
            for child in content {
                emit_inline_events(child, out);
            }
            out.push(Event::EndFootnoteDef);
        }
        Inline::CrossRef { kind, node, text, .. } => {
            out.push(Event::CrossRef {
                kind: kind.clone(),
                node: node.clone(),
                text: text.clone(),
            });
        }
        Inline::Anchor { name, .. } => {
            out.push(Event::Anchor { name: name.clone() });
        }
        Inline::NoBreak(s, _) => out.push(Event::NoBreak(Cow::Owned(s.clone()))),
        Inline::Email { address, text, .. } => {
            out.push(Event::Email {
                address: address.clone(),
                text: text.clone(),
            });
        }
        Inline::Symbol(kind, _) => out.push(Event::Symbol(kind.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("@chapter Hello").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events("@example\nfn main() {}\n@end example").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> =
            events("@itemize\n@item one\n@item two\n@end itemize").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartList { ordered: false })));
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, OwnedEvent::StartListItem))
                .count(),
            2
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }

    #[test]
    fn test_events_strong() {
        let evs: Vec<_> = events("@strong{bold}").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartStrong)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndStrong)));
    }
}
