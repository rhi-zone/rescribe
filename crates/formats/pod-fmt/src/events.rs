//! Streaming event iterator over a parsed `PodDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a POD document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading { level: u32 },
    EndHeading,
    /// Leaf: a verbatim / code block.
    CodeBlock { content: Cow<'a, str> },
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    /// Leaf: `=begin`/`=end` raw region.
    RawBlock { format: String, content: String },
    /// Leaf: `=for` format-specific paragraph.
    ForBlock { format: String, content: String },
    /// Leaf: `=encoding` declaration.
    Encoding { encoding: String },

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartUnderline,
    EndUnderline,
    StartFilename,
    EndFilename,
    StartNonBreaking,
    EndNonBreaking,
    /// Leaf: inline code span.
    InlineCode(Cow<'a, str>),
    StartLink { url: String, label: String },
    EndLink,
    /// Leaf: index entry (invisible).
    IndexEntry(String),
    /// Leaf: zero-width / null.
    Null,
    /// Leaf: resolved entity text.
    Entity(String),
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { content } => {
                Event::CodeBlock { content: Cow::Owned(content.into_owned()) }
            }
            Event::StartParagraph => Event::StartParagraph,
            Event::EndParagraph => Event::EndParagraph,
            Event::StartHeading { level } => Event::StartHeading { level },
            Event::EndHeading => Event::EndHeading,
            Event::StartList { ordered } => Event::StartList { ordered },
            Event::EndList => Event::EndList,
            Event::StartListItem => Event::StartListItem,
            Event::EndListItem => Event::EndListItem,
            Event::StartDefinitionList => Event::StartDefinitionList,
            Event::EndDefinitionList => Event::EndDefinitionList,
            Event::StartDefinitionTerm => Event::StartDefinitionTerm,
            Event::EndDefinitionTerm => Event::EndDefinitionTerm,
            Event::StartDefinitionDesc => Event::StartDefinitionDesc,
            Event::EndDefinitionDesc => Event::EndDefinitionDesc,
            Event::RawBlock { format, content } => Event::RawBlock { format, content },
            Event::ForBlock { format, content } => Event::ForBlock { format, content },
            Event::Encoding { encoding } => Event::Encoding { encoding },
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
            Event::StartLink { url, label } => Event::StartLink { url, label },
            Event::EndLink => Event::EndLink,
            Event::IndexEntry(s) => Event::IndexEntry(s),
            Event::Null => Event::Null,
            Event::Entity(s) => Event::Entity(s),
        }
    }
}

// ── True pull iterator ────────────────────────────────────────────────────────

/// Event iterator that walks a parsed `PodDoc` using a frame stack,
/// yielding events lazily without buffering.
pub struct EventIter<'a> {
    /// Stack of frames; we iterate depth-first.
    stack: Vec<Frame<'a>>,
}

enum Frame<'a> {
    // Block frames — each carries a slice of children still to emit.
    DocBlocks { blocks: std::slice::Iter<'a, Block> },
    HeadingOpen { level: u32, inlines: std::slice::Iter<'a, Inline> },
    HeadingClose,
    ParagraphOpen { inlines: std::slice::Iter<'a, Inline> },
    ParagraphClose,
    ListOpen { ordered: bool, items: std::slice::Iter<'a, Vec<Block>> },
    ListClose,
    ListItemOpen { blocks: std::slice::Iter<'a, Block> },
    ListItemClose,
    DefListOpen { items: std::slice::Iter<'a, DefinitionItem> },
    DefListClose,
    DefTermOpen { inlines: std::slice::Iter<'a, Inline> },
    DefTermClose,
    DefDescOpen { blocks: std::slice::Iter<'a, Block> },
    DefDescClose,

    // Inline frames
    InlineBoldOpen { children: std::slice::Iter<'a, Inline> },
    InlineBoldClose,
    InlineItalicOpen { children: std::slice::Iter<'a, Inline> },
    InlineItalicClose,
    InlineUnderlineOpen { children: std::slice::Iter<'a, Inline> },
    InlineUnderlineClose,
    InlineFilenameOpen { children: std::slice::Iter<'a, Inline> },
    InlineFilenameClose,
    InlineNonBreakingOpen { children: std::slice::Iter<'a, Inline> },
    InlineNonBreakingClose,

    // Leaf inlines
    LeafText(&'a str),
    LeafCode(&'a str),
    LeafLinkOpen { url: &'a str, label: &'a str },
    LeafLinkClose,
    LeafIndexEntry(&'a str),
    LeafNull,
    LeafEntity(&'a str),

    // Leaf blocks
    LeafCodeBlock(&'a str),
    LeafRawBlock { format: &'a str, content: &'a str },
    LeafForBlock { format: &'a str, content: &'a str },
    LeafEncoding(&'a str),
}

impl<'a> EventIter<'a> {
    pub fn new(doc: &'a PodDoc) -> Self {
        EventIter {
            stack: vec![Frame::DocBlocks { blocks: doc.blocks.iter() }],
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        loop {
            let frame = self.stack.last_mut()?;
            match frame {
                Frame::DocBlocks { blocks } => {
                    if let Some(block) = blocks.next() {
                        push_block_frames(&mut self.stack, block);
                    } else {
                        self.stack.pop();
                    }
                }

                // Heading
                Frame::HeadingOpen { level, inlines } => {
                    let level = *level;
                    // Drain inlines first, then schedule close
                    let remaining: Vec<&'a Inline> = inlines.collect();
                    // Replace with close
                    *self.stack.last_mut().unwrap() = Frame::HeadingClose;
                    // Push inlines in reverse
                    for inline in remaining.into_iter().rev() {
                        push_inline_frames(&mut self.stack, inline);
                    }
                    return Some(Event::StartHeading { level });
                }
                Frame::HeadingClose => {
                    self.stack.pop();
                    return Some(Event::EndHeading);
                }

                // Paragraph
                Frame::ParagraphOpen { inlines } => {
                    let remaining: Vec<&'a Inline> = inlines.collect();
                    *self.stack.last_mut().unwrap() = Frame::ParagraphClose;
                    for inline in remaining.into_iter().rev() {
                        push_inline_frames(&mut self.stack, inline);
                    }
                    return Some(Event::StartParagraph);
                }
                Frame::ParagraphClose => {
                    self.stack.pop();
                    return Some(Event::EndParagraph);
                }

                // List
                Frame::ListOpen { ordered, items } => {
                    let ordered = *ordered;
                    let remaining: Vec<&'a Vec<Block>> = items.collect();
                    *self.stack.last_mut().unwrap() = Frame::ListClose;
                    for item in remaining.into_iter().rev() {
                        self.stack.push(Frame::ListItemOpen { blocks: item.iter() });
                    }
                    return Some(Event::StartList { ordered });
                }
                Frame::ListClose => {
                    self.stack.pop();
                    return Some(Event::EndList);
                }
                Frame::ListItemOpen { blocks } => {
                    let remaining: Vec<&'a Block> = blocks.collect();
                    *self.stack.last_mut().unwrap() = Frame::ListItemClose;
                    for block in remaining.into_iter().rev() {
                        push_block_frames(&mut self.stack, block);
                    }
                    return Some(Event::StartListItem);
                }
                Frame::ListItemClose => {
                    self.stack.pop();
                    return Some(Event::EndListItem);
                }

                // Definition list
                Frame::DefListOpen { items } => {
                    let remaining: Vec<&'a DefinitionItem> = items.collect();
                    *self.stack.last_mut().unwrap() = Frame::DefListClose;
                    for item in remaining.into_iter().rev() {
                        self.stack.push(Frame::DefDescOpen { blocks: item.desc.iter() });
                        self.stack.push(Frame::DefTermOpen { inlines: item.term.iter() });
                    }
                    return Some(Event::StartDefinitionList);
                }
                Frame::DefListClose => {
                    self.stack.pop();
                    return Some(Event::EndDefinitionList);
                }
                Frame::DefTermOpen { inlines } => {
                    let remaining: Vec<&'a Inline> = inlines.collect();
                    *self.stack.last_mut().unwrap() = Frame::DefTermClose;
                    for inline in remaining.into_iter().rev() {
                        push_inline_frames(&mut self.stack, inline);
                    }
                    return Some(Event::StartDefinitionTerm);
                }
                Frame::DefTermClose => {
                    self.stack.pop();
                    return Some(Event::EndDefinitionTerm);
                }
                Frame::DefDescOpen { blocks } => {
                    let remaining: Vec<&'a Block> = blocks.collect();
                    *self.stack.last_mut().unwrap() = Frame::DefDescClose;
                    for block in remaining.into_iter().rev() {
                        push_block_frames(&mut self.stack, block);
                    }
                    return Some(Event::StartDefinitionDesc);
                }
                Frame::DefDescClose => {
                    self.stack.pop();
                    return Some(Event::EndDefinitionDesc);
                }

                // Inline bold
                Frame::InlineBoldOpen { children } => {
                    let remaining: Vec<&'a Inline> = children.collect();
                    *self.stack.last_mut().unwrap() = Frame::InlineBoldClose;
                    for inline in remaining.into_iter().rev() {
                        push_inline_frames(&mut self.stack, inline);
                    }
                    return Some(Event::StartBold);
                }
                Frame::InlineBoldClose => {
                    self.stack.pop();
                    return Some(Event::EndBold);
                }

                // Inline italic
                Frame::InlineItalicOpen { children } => {
                    let remaining: Vec<&'a Inline> = children.collect();
                    *self.stack.last_mut().unwrap() = Frame::InlineItalicClose;
                    for inline in remaining.into_iter().rev() {
                        push_inline_frames(&mut self.stack, inline);
                    }
                    return Some(Event::StartItalic);
                }
                Frame::InlineItalicClose => {
                    self.stack.pop();
                    return Some(Event::EndItalic);
                }

                // Inline underline
                Frame::InlineUnderlineOpen { children } => {
                    let remaining: Vec<&'a Inline> = children.collect();
                    *self.stack.last_mut().unwrap() = Frame::InlineUnderlineClose;
                    for inline in remaining.into_iter().rev() {
                        push_inline_frames(&mut self.stack, inline);
                    }
                    return Some(Event::StartUnderline);
                }
                Frame::InlineUnderlineClose => {
                    self.stack.pop();
                    return Some(Event::EndUnderline);
                }

                // Inline filename
                Frame::InlineFilenameOpen { children } => {
                    let remaining: Vec<&'a Inline> = children.collect();
                    *self.stack.last_mut().unwrap() = Frame::InlineFilenameClose;
                    for inline in remaining.into_iter().rev() {
                        push_inline_frames(&mut self.stack, inline);
                    }
                    return Some(Event::StartFilename);
                }
                Frame::InlineFilenameClose => {
                    self.stack.pop();
                    return Some(Event::EndFilename);
                }

                // Inline non-breaking
                Frame::InlineNonBreakingOpen { children } => {
                    let remaining: Vec<&'a Inline> = children.collect();
                    *self.stack.last_mut().unwrap() = Frame::InlineNonBreakingClose;
                    for inline in remaining.into_iter().rev() {
                        push_inline_frames(&mut self.stack, inline);
                    }
                    return Some(Event::StartNonBreaking);
                }
                Frame::InlineNonBreakingClose => {
                    self.stack.pop();
                    return Some(Event::EndNonBreaking);
                }

                // Leaf inlines
                Frame::LeafText(s) => {
                    let s = *s;
                    self.stack.pop();
                    return Some(Event::Text(Cow::Borrowed(s)));
                }
                Frame::LeafCode(s) => {
                    let s = *s;
                    self.stack.pop();
                    return Some(Event::InlineCode(Cow::Borrowed(s)));
                }
                Frame::LeafLinkOpen { url, label } => {
                    let url = url.to_string();
                    let label = label.to_string();
                    *self.stack.last_mut().unwrap() = Frame::LeafLinkClose;
                    return Some(Event::StartLink { url, label });
                }
                Frame::LeafLinkClose => {
                    self.stack.pop();
                    return Some(Event::EndLink);
                }
                Frame::LeafIndexEntry(s) => {
                    let s = s.to_string();
                    self.stack.pop();
                    return Some(Event::IndexEntry(s));
                }
                Frame::LeafNull => {
                    self.stack.pop();
                    return Some(Event::Null);
                }
                Frame::LeafEntity(s) => {
                    let s = s.to_string();
                    self.stack.pop();
                    return Some(Event::Entity(s));
                }

                // Leaf blocks
                Frame::LeafCodeBlock(content) => {
                    let content = *content;
                    self.stack.pop();
                    return Some(Event::CodeBlock { content: Cow::Borrowed(content) });
                }
                Frame::LeafRawBlock { format, content } => {
                    let format = format.to_string();
                    let content = content.to_string();
                    self.stack.pop();
                    return Some(Event::RawBlock { format, content });
                }
                Frame::LeafForBlock { format, content } => {
                    let format = format.to_string();
                    let content = content.to_string();
                    self.stack.pop();
                    return Some(Event::ForBlock { format, content });
                }
                Frame::LeafEncoding(enc) => {
                    let enc = enc.to_string();
                    self.stack.pop();
                    return Some(Event::Encoding { encoding: enc });
                }
            }
        }
    }
}

fn push_block_frames<'a>(stack: &mut Vec<Frame<'a>>, block: &'a Block) {
    match block {
        Block::Heading { level, inlines, .. } => {
            stack.push(Frame::HeadingOpen { level: *level, inlines: inlines.iter() });
        }
        Block::Paragraph { inlines, .. } => {
            stack.push(Frame::ParagraphOpen { inlines: inlines.iter() });
        }
        Block::CodeBlock { content, .. } => {
            stack.push(Frame::LeafCodeBlock(content));
        }
        Block::List { ordered, items, .. } => {
            stack.push(Frame::ListOpen { ordered: *ordered, items: items.iter() });
        }
        Block::DefinitionList { items, .. } => {
            stack.push(Frame::DefListOpen { items: items.iter() });
        }
        Block::RawBlock { format, content, .. } => {
            stack.push(Frame::LeafRawBlock { format, content });
        }
        Block::ForBlock { format, content, .. } => {
            stack.push(Frame::LeafForBlock { format, content });
        }
        Block::Encoding { encoding, .. } => {
            stack.push(Frame::LeafEncoding(encoding));
        }
    }
}

fn push_inline_frames<'a>(stack: &mut Vec<Frame<'a>>, inline: &'a Inline) {
    match inline {
        Inline::Text(s, _) => {
            stack.push(Frame::LeafText(s));
        }
        Inline::Bold(children, _) => {
            stack.push(Frame::InlineBoldOpen { children: children.iter() });
        }
        Inline::Italic(children, _) => {
            stack.push(Frame::InlineItalicOpen { children: children.iter() });
        }
        Inline::Underline(children, _) => {
            stack.push(Frame::InlineUnderlineOpen { children: children.iter() });
        }
        Inline::Code(s, _) => {
            stack.push(Frame::LeafCode(s));
        }
        Inline::Link { url, label, .. } => {
            stack.push(Frame::LeafLinkOpen { url, label });
        }
        Inline::Filename(children, _) => {
            stack.push(Frame::InlineFilenameOpen { children: children.iter() });
        }
        Inline::NonBreaking(children, _) => {
            stack.push(Frame::InlineNonBreakingOpen { children: children.iter() });
        }
        Inline::IndexEntry(s, _) => {
            stack.push(Frame::LeafIndexEntry(s));
        }
        Inline::Null(_) => {
            stack.push(Frame::LeafNull);
        }
        Inline::Entity(s, _) => {
            stack.push(Frame::LeafEntity(s));
        }
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(_input: &str) -> EventIter<'_> {
    // We need the parsed doc to live as long as the iterator.
    // Since EventIter borrows from PodDoc, we need a different approach
    // for the public API — see lib.rs.
    unreachable!("use pod_fmt::events() instead");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let (doc, _) = crate::parse::parse("=head1 Hello");
        let evs: Vec<_> = EventIter::new(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, Event::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let (doc, _) = crate::parse::parse("=pod\n\nHello world\n");
        let evs: Vec<_> = EventIter::new(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_list() {
        let (doc, _) = crate::parse::parse("=over\n\n=item * One\n\n=item * Two\n\n=back\n");
        let evs: Vec<_> = EventIter::new(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartList { ordered: false })));
        assert_eq!(evs.iter().filter(|e| matches!(e, Event::StartListItem)).count(), 2);
        assert!(evs.iter().any(|e| matches!(e, Event::EndList)));
    }

    #[test]
    fn test_events_bold() {
        let (doc, _) = crate::parse::parse("=pod\n\nB<bold> text\n");
        let evs: Vec<_> = EventIter::new(&doc).collect();
        assert!(evs.iter().any(|e| matches!(e, Event::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndBold)));
    }
}
