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
            // All other variants contain only String/'static fields, so they
            // convert without touching any borrowed data. Listed explicitly
            // (rather than a catch-all `unsafe { transmute }`) so that adding
            // a new `Cow<'a, str>`-bearing variant without updating this
            // function is a compile error, not a silent soundness hazard.
            Event::StartParagraph => Event::StartParagraph,
            Event::EndParagraph => Event::EndParagraph,
            Event::StartBlockquote { author } => Event::StartBlockquote { author },
            Event::EndBlockquote => Event::EndBlockquote,
            Event::StartList { ordered } => Event::StartList { ordered },
            Event::EndList => Event::EndList,
            Event::StartListItem => Event::StartListItem,
            Event::EndListItem => Event::EndListItem,
            Event::StartTable => Event::StartTable,
            Event::EndTable => Event::EndTable,
            Event::StartTableRow => Event::StartTableRow,
            Event::EndTableRow => Event::EndTableRow,
            Event::StartTableCell { is_header } => Event::StartTableCell { is_header },
            Event::EndTableCell => Event::EndTableCell,
            Event::HorizontalRule => Event::HorizontalRule,
            Event::StartHeading { level } => Event::StartHeading { level },
            Event::EndHeading => Event::EndHeading,
            Event::StartAlignment { kind } => Event::StartAlignment { kind },
            Event::EndAlignment => Event::EndAlignment,
            Event::StartSpoiler => Event::StartSpoiler,
            Event::EndSpoiler => Event::EndSpoiler,
            Event::StartIndent => Event::StartIndent,
            Event::EndIndent => Event::EndIndent,
            Event::StartBold => Event::StartBold,
            Event::EndBold => Event::EndBold,
            Event::StartItalic => Event::StartItalic,
            Event::EndItalic => Event::EndItalic,
            Event::StartUnderline => Event::StartUnderline,
            Event::EndUnderline => Event::EndUnderline,
            Event::StartStrikethrough => Event::StartStrikethrough,
            Event::EndStrikethrough => Event::EndStrikethrough,
            Event::StartSubscript => Event::StartSubscript,
            Event::EndSubscript => Event::EndSubscript,
            Event::StartSuperscript => Event::StartSuperscript,
            Event::EndSuperscript => Event::EndSuperscript,
            Event::StartLink { url } => Event::StartLink { url },
            Event::EndLink => Event::EndLink,
            Event::InlineImage { url, width, height } => Event::InlineImage { url, width, height },
            Event::StartColor { value } => Event::StartColor { value },
            Event::EndColor => Event::EndColor,
            Event::StartSize { value } => Event::StartSize { value },
            Event::EndSize => Event::EndSize,
            Event::StartFont { name } => Event::StartFont { name },
            Event::EndFont => Event::EndFont,
            Event::StartEmail { addr } => Event::StartEmail { addr },
            Event::EndEmail => Event::EndEmail,
            Event::StartSpan { attr, value } => Event::StartSpan { attr, value },
            Event::EndSpan => Event::EndSpan,
        }
    }
}

// ── Pull iterator ─────────────────────────────────────────────────────────────

/// An iterator that yields [`Event`]s from a BBCode document.
///
/// Constructed by [`events_str()`], or indirectly via
/// [`crate::Events::events`] (`rescribe_format_api`'s trait).
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
pub fn events_str(input: &str) -> EventIter<'_> {
    let (doc, _) = crate::parse::parse_str(input);
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
        Block::Alignment { kind, children, .. } => {
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
            out.push(Event::StartLink { url: url.clone() });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image {
            url, width, height, ..
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
        Inline::Font { name, children, .. } => {
            out.push(Event::StartFont { name: name.clone() });
            for child in children {
                emit_inline_events(child, out);
            }
            out.push(Event::EndFont);
        }
        Inline::Email { addr, children, .. } => {
            out.push(Event::StartEmail { addr: addr.clone() });
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the removed catch-all `unsafe { transmute }` in
    /// `Event::into_owned`: the old code matched every `Cow`-bearing variant
    /// explicitly and then transmuted everything else, which was only sound
    /// because the catch-all happened to cover no borrowed data — a future
    /// variant with a `Cow<'a, str>` field added without updating that catch
    /// arm would have silently produced a dangling/incorrect owned event.
    /// Now every variant is listed explicitly, so this is enforced by
    /// exhaustiveness checking at compile time; this test just confirms
    /// `into_owned()` still round-trips correctly across a document that
    /// exercises every event family (borrowed-text leaves, block containers,
    /// inline containers, and plain unit/struct variants with no lifetime).
    #[test]
    fn test_into_owned_round_trip_all_families() {
        let input = "[b]bold[/b] [i]italic[/i]\n\
                     [code]let x = 1;[/code]\n\
                     [quote=Alice]\nquoted\n[/quote]\n\
                     [list]\n[*]one\n[*]two\n[/list]\n\
                     [hr]\n\
                     [h1]Heading[/h1]\n\
                     [center]\ncentered\n[/center]\n\
                     [spoiler]\nhidden\n[/spoiler]\n\
                     [color=red]red[/color] [size=12]sized[/size] \
                     [font=Arial]fonted[/font] [email=a@b.com]mail[/email] \
                     [noparse][b]raw[/b][/noparse] \
                     [url=https://example.com]link[/url] \
                     [img]https://example.com/x.png[/img]";

        let borrowed: Vec<Event<'_>> = {
            let (doc, _) = crate::parse::parse_str(input);
            let mut evts = Vec::new();
            for block in &doc.blocks {
                emit_block_events(block, &mut evts);
            }
            evts
        };
        assert!(!borrowed.is_empty());

        // Every variant must survive into_owned() without panicking (no
        // `unreachable!`/transmute-induced corruption) and text content must
        // be preserved byte-for-byte.
        let owned: Vec<OwnedEvent> = borrowed.into_iter().map(Event::into_owned).collect();

        assert!(
            owned
                .iter()
                .any(|e| matches!(e, Event::Text(t) if t == "bold"))
        );
        assert!(owned.iter().any(
            |e| matches!(e, Event::CodeBlock { content, .. } if content.contains("let x = 1;"))
        ));
        assert!(
            owned
                .iter()
                .any(|e| matches!(e, Event::Noparse(t) if t.contains("[b]raw[/b]")))
        );
        assert!(owned.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(
            owned
                .iter()
                .any(|e| matches!(e, Event::StartHeading { level: 1 }))
        );
        assert!(
            owned
                .iter()
                .any(|e| matches!(e, Event::StartBlockquote { author: Some(a) } if a == "Alice"))
        );
        assert!(owned.iter().any(|e| matches!(e, Event::HorizontalRule)));

        // `owned: Vec<OwnedEvent>` is `Vec<Event<'static>>` — this line only
        // compiles if into_owned() truly produced 'static data, which is the
        // property the removed transmute was (fragile-ly) relying on.
        let _: Vec<Event<'static>> = owned;
    }

    #[test]
    fn test_events_basic() {
        let evs: Vec<_> = events_str("[b]hi[/b]").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::Text(t) if t == "hi")));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBold)));
    }
}
