//! Streaming event iterator over a parsed `BbcodeDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a BBCode document.
///
/// Raw text content fields use `Cow<'a, str>` so that future optimisations can
/// yield borrowed slices of the input without changing the public API.
/// For the common case of fully-owned events (e.g. batch mode) use the
/// [`OwnedEvent`] type alias.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartBlockquote {
        author: Option<String>,
    },
    EndBlockquote,
    StartList {
        ordered: bool,
    },
    EndList,
    StartListItem,
    EndListItem,
    /// Leaf: a code block.
    CodeBlock {
        language: Option<String>,
        content: Cow<'a, str>,
    },
    StartTable,
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell {
        is_header: bool,
    },
    EndTableCell,
    /// Leaf: a horizontal rule `[hr]`.
    HorizontalRule,
    StartHeading {
        level: u8,
    },
    EndHeading,
    StartAlignment {
        kind: AlignKind,
    },
    EndAlignment,
    StartSpoiler,
    EndSpoiler,
    /// Leaf: a preformatted block.
    Preformatted {
        content: Cow<'a, str>,
    },
    StartIndent,
    EndIndent,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartUnderline,
    EndUnderline,
    StartStrikethrough,
    EndStrikethrough,
    StartSubscript,
    EndSubscript,
    StartSuperscript,
    EndSuperscript,
    /// Leaf: inline code span.
    InlineCode(Cow<'a, str>),
    StartLink {
        url: String,
    },
    EndLink,
    /// Leaf: inline image.
    InlineImage {
        url: String,
        width: Option<u32>,
        height: Option<u32>,
    },
    StartColor {
        value: String,
    },
    EndColor,
    StartSize {
        value: String,
    },
    EndSize,
    StartFont {
        name: String,
    },
    EndFont,
    StartEmail {
        addr: String,
    },
    EndEmail,
    /// Leaf: noparse/verbatim span.
    Noparse(Cow<'a, str>),
    StartSpan {
        attr: String,
        value: String,
    },
    EndSpan,
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event (all `Cow::Borrowed` text fields become `Cow::Owned`).
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { language, content } => Event::CodeBlock {
                language,
                content: Cow::Owned(content.into_owned()),
            },
            Event::Preformatted { content } => Event::Preformatted {
                content: Cow::Owned(content.into_owned()),
            },
            Event::Noparse(cow) => Event::Noparse(Cow::Owned(cow.into_owned())),
            // All other variants contain only String/'static fields.
            // Safety: the only non-'static field is Cow<'a, str> and we've
            // handled every variant that contains one above.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// ── Pull iterator ─────────────────────────────────────────────────────────────

/// An iterator that yields [`Event`]s from a BBCode document.
///
/// Constructed by [`events()`].
pub struct EventIter<'a> {
    /// Pre-computed list of events.  We parse once and then iterate.
    events: Vec<Event<'a>>,
    pos: usize,
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.events.len() {
            // We need to take ownership; swap with a dummy.
            let idx = self.pos;
            self.pos += 1;
            // Replace with a dummy event so we can return owned.
            let dummy = Event::Text(Cow::Borrowed(""));
            Some(std::mem::replace(&mut self.events[idx], dummy))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.events.len() - self.pos;
        (remaining, Some(remaining))
    }
}

/// Parse BBCode input and return a streaming iterator of events.
pub fn events(input: &str) -> EventIter<'_> {
    let (doc, _) = crate::parse::parse(input);
    let mut evts = Vec::new();
    for block in &doc.blocks {
        emit_block_events(block, &mut evts);
    }
    EventIter {
        events: evts,
        pos: 0,
    }
}

fn emit_block_events<'a>(block: &Block, out: &mut Vec<Event<'a>>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for inline in inlines {
                emit_inline_events(inline, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock {
            language, content, ..
        } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote {
            author, children, ..
        } => {
            out.push(Event::StartBlockquote {
                author: author.clone(),
            });
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
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow);
                for (is_header, inlines) in &row.cells {
                    out.push(Event::StartTableCell {
                        is_header: *is_header,
                    });
                    for inline in inlines {
                        emit_inline_events(inline, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
        Block::Heading {
            level, children, ..
        } => {
            out.push(Event::StartHeading { level: *level });
            for inline in children {
                emit_inline_events(inline, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Alignment {
            kind, children, ..
        } => {
            out.push(Event::StartAlignment { kind: *kind });
            for child in children {
                emit_block_events(child, out);
            }
            out.push(Event::EndAlignment);
        }
        Block::Spoiler { children, .. } => {
            out.push(Event::StartSpoiler);
            for child in children {
                emit_block_events(child, out);
            }
            out.push(Event::EndSpoiler);
        }
        Block::Preformatted { content, .. } => {
            out.push(Event::Preformatted {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Indent { children, .. } => {
            out.push(Event::StartIndent);
            for child in children {
                emit_block_events(child, out);
            }
            out.push(Event::EndIndent);
        }
    }
}

fn emit_inline_events<'a>(inline: &Inline, out: &mut Vec<Event<'a>>) {
    match inline {
        Inline::Text(s, _) => {
            out.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Code(s, _) => {
            out.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink {
                url: url.clone(),
            });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image {
            url,
            width,
            height,
            ..
        } => {
            out.push(Event::InlineImage {
                url: url.clone(),
                width: *width,
                height: *height,
            });
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndSubscript);
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndSuperscript);
        }
        Inline::Color {
            value, children, ..
        } => {
            out.push(Event::StartColor {
                value: value.clone(),
            });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndColor);
        }
        Inline::Size {
            value, children, ..
        } => {
            out.push(Event::StartSize {
                value: value.clone(),
            });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndSize);
        }
        Inline::Font {
            name, children, ..
        } => {
            out.push(Event::StartFont {
                name: name.clone(),
            });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndFont);
        }
        Inline::Email {
            addr, children, ..
        } => {
            out.push(Event::StartEmail {
                addr: addr.clone(),
            });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndEmail);
        }
        Inline::Noparse(s, _) => {
            out.push(Event::Noparse(Cow::Owned(s.clone())));
        }
        Inline::Span {
            attr,
            value,
            children,
            ..
        } => {
            out.push(Event::StartSpan {
                attr: attr.clone(),
                value: value.clone(),
            });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndSpan);
        }
    }
}
