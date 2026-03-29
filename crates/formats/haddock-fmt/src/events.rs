//! Streaming event iterator over a parsed `HaddockDoc`.

use std::borrow::Cow;

use crate::ast::*;

/// A streaming event from a Haddock document.
#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading { level: u8 },
    EndHeading,
    CodeBlock { content: Cow<'a, str> },
    AtCodeBlock { content: Cow<'a, str> },
    StartUnorderedList,
    EndUnorderedList,
    StartOrderedList,
    EndOrderedList,
    StartListItem,
    EndListItem,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    DocTest { expression: Cow<'a, str>, result: Option<Cow<'a, str>> },
    StartBlockquote,
    EndBlockquote,
    Property { key: Cow<'a, str>, name: Option<Cow<'a, str>> },
    EndProperty,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    InlineCode(Cow<'a, str>),
    StartStrong,
    EndStrong,
    StartEmphasis,
    EndEmphasis,
    StartLink { url: String, text: String },
    EndLink,
    ModuleLink { module: String },
}

/// Backwards-compatible alias for batch mode (all text is owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::InlineCode(cow) => Event::InlineCode(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { content } => Event::CodeBlock {
                content: Cow::Owned(content.into_owned()),
            },
            Event::AtCodeBlock { content } => Event::AtCodeBlock {
                content: Cow::Owned(content.into_owned()),
            },
            Event::DocTest { expression, result } => Event::DocTest {
                expression: Cow::Owned(expression.into_owned()),
                result: result.map(|r| Cow::Owned(r.into_owned())),
            },
            Event::Property { key, name } => Event::Property {
                key: Cow::Owned(key.into_owned()),
                name: name.map(|n| Cow::Owned(n.into_owned())),
            },
            // All other variants contain only String/'static fields.
            other => unsafe { std::mem::transmute::<Event<'_>, OwnedEvent>(other) },
        }
    }
}

// ── True pull iterator ────────────────────────────────────────────────────────

/// An iterator that produces [`Event`]s from a Haddock document.
pub struct EventIter<'a> {
    doc: HaddockDoc,
    block_idx: usize,
    /// Stack of pending events for the current block.
    pending: Vec<Event<'a>>,
}

/// Create an event iterator from input text.
pub fn events(input: &str) -> EventIter<'_> {
    let (doc, _) = crate::parse::parse(input);
    EventIter {
        doc,
        block_idx: 0,
        pending: Vec::new(),
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        // Drain pending events first (reversed so we can pop)
        if let Some(ev) = self.pending.pop() {
            return Some(ev);
        }

        if self.block_idx >= self.doc.blocks.len() {
            return None;
        }

        let block = self.doc.blocks[self.block_idx].clone();
        self.block_idx += 1;
        self.expand_block(block);
        self.pending.pop()
    }
}

impl<'a> EventIter<'a> {
    fn expand_block(&mut self, block: Block) {
        // Push events in reverse order so pop() yields them in forward order.
        match block {
            Block::Heading { level, inlines, .. } => {
                self.pending.push(Event::EndHeading);
                self.expand_inlines_reversed(&inlines);
                self.pending.push(Event::StartHeading { level });
            }
            Block::Paragraph { inlines, .. } => {
                self.pending.push(Event::EndParagraph);
                self.expand_inlines_reversed(&inlines);
                self.pending.push(Event::StartParagraph);
            }
            Block::CodeBlock { content, .. } => {
                self.pending.push(Event::CodeBlock {
                    content: Cow::Owned(content),
                });
            }
            Block::AtCodeBlock { content, .. } => {
                self.pending.push(Event::AtCodeBlock {
                    content: Cow::Owned(content),
                });
            }
            Block::UnorderedList { items, .. } => {
                self.pending.push(Event::EndUnorderedList);
                for item in items.into_iter().rev() {
                    self.pending.push(Event::EndListItem);
                    self.expand_inlines_reversed(&item);
                    self.pending.push(Event::StartListItem);
                }
                self.pending.push(Event::StartUnorderedList);
            }
            Block::OrderedList { items, .. } => {
                self.pending.push(Event::EndOrderedList);
                for item in items.into_iter().rev() {
                    self.pending.push(Event::EndListItem);
                    self.expand_inlines_reversed(&item);
                    self.pending.push(Event::StartListItem);
                }
                self.pending.push(Event::StartOrderedList);
            }
            Block::DefinitionList { items, .. } => {
                self.pending.push(Event::EndDefinitionList);
                for (term, desc) in items.into_iter().rev() {
                    self.pending.push(Event::EndDefinitionDesc);
                    self.expand_inlines_reversed(&desc);
                    self.pending.push(Event::StartDefinitionDesc);
                    self.pending.push(Event::EndDefinitionTerm);
                    self.expand_inlines_reversed(&term);
                    self.pending.push(Event::StartDefinitionTerm);
                }
                self.pending.push(Event::StartDefinitionList);
            }
            Block::DocTest { expression, result, .. } => {
                self.pending.push(Event::DocTest {
                    expression: Cow::Owned(expression),
                    result: result.map(Cow::Owned),
                });
            }
            Block::Blockquote { inlines, .. } => {
                self.pending.push(Event::EndBlockquote);
                self.expand_inlines_reversed(&inlines);
                self.pending.push(Event::StartBlockquote);
            }
            Block::Property { key, name, description, .. } => {
                self.pending.push(Event::EndProperty);
                self.expand_inlines_reversed(&description);
                self.pending.push(Event::Property {
                    key: Cow::Owned(key),
                    name: name.map(Cow::Owned),
                });
            }
        }
    }

    fn expand_inlines_reversed(&mut self, inlines: &[Inline]) {
        for inline in inlines.iter().rev() {
            self.expand_inline(inline);
        }
    }

    fn expand_inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text(s, _) => {
                self.pending.push(Event::Text(Cow::Owned(s.clone())));
            }
            Inline::Code(s, _) => {
                self.pending.push(Event::InlineCode(Cow::Owned(s.clone())));
            }
            Inline::Strong(children, _) => {
                self.pending.push(Event::EndStrong);
                self.expand_inlines_reversed(children);
                self.pending.push(Event::StartStrong);
            }
            Inline::Emphasis(children, _) => {
                self.pending.push(Event::EndEmphasis);
                self.expand_inlines_reversed(children);
                self.pending.push(Event::StartEmphasis);
            }
            Inline::Link { url, text, .. } => {
                self.pending.push(Event::EndLink);
                self.pending.push(Event::Text(Cow::Owned(text.clone())));
                self.pending.push(Event::StartLink {
                    url: url.clone(),
                    text: text.clone(),
                });
            }
            Inline::ModuleLink { module, .. } => {
                self.pending.push(Event::ModuleLink { module: module.clone() });
            }
        }
    }
}
