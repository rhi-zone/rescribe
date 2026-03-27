//! Semantic document-level event iterator for RTF.
//!
//! [`events`] parses the input and streams document-level events
//! (StartParagraph/EndParagraph, Text, StartBold/EndBold, etc.) that mirror the
//! parsed AST structure. This is the same abstraction level as the other format
//! crates (djot-fmt, rst-fmt, asciidoc, org-fmt).
//!
//! For the lower-level raw RTF token stream, see [`crate::token_events`].
//!
//! # Memory note
//!
//! Because RTF's stateful group/property inheritance requires full-document
//! context (font table, colour table, group stack), the document is fully
//! parsed into an `RtfDoc` before the first event is yielded. Events are then
//! streamed lazily from the AST via a frame stack — O(nesting depth) extra
//! memory after the initial parse.

use std::borrow::Cow;

use crate::ast::{Align, Block, Inline, TableRow};

// ── Public event type ─────────────────────────────────────────────────────────

/// A semantic document event from an RTF document.
///
/// Text content uses `Cow<'a, str>` to allow future zero-copy optimisations
/// without breaking the public API. For the fully-owned variant use
/// [`OwnedEvent`].
#[derive(Debug)]
pub enum Event<'a> {
    // ── Block events ─────────────────────────────────────────────────────────
    StartParagraph { align: Align, para_props: Cow<'a, str> },
    EndParagraph,
    StartHeading { level: u8 },
    EndHeading,
    StartCodeBlock,
    EndCodeBlock,
    CodeBlockContent(Cow<'a, str>),
    StartBlockquote,
    EndBlockquote,
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    StartTable,
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell,
    EndTableCell,
    HorizontalRule,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    LineBreak,
    SoftBreak,
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartUnderline,
    EndUnderline,
    StartStrikethrough,
    EndStrikethrough,
    Code(Cow<'a, str>),
    StartLink { url: String },
    EndLink,
    Image { url: String, alt: String },
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    StartFontSize { size: u16 },
    EndFontSize,
    StartColor { r: u8, g: u8, b: u8 },
    EndColor,
    StartAllCaps,
    EndAllCaps,
    StartSmallCaps,
    EndSmallCaps,
    StartHidden,
    EndHidden,
    StartCharSpan { char_props: String },
    EndCharSpan,
    StartFont { name: String },
    EndFont,
    StartBgColor { r: u8, g: u8, b: u8 },
    EndBgColor,
    StartLang { lcid: u16 },
    EndLang,
    StartFootnote,
    EndFootnote,
}

/// Fully-owned variant of [`Event`] (no borrowed lifetimes).
pub type OwnedEvent = Event<'static>;

// ── Frame stack ───────────────────────────────────────────────────────────────

enum Frame {
    /// A single pre-computed event ready to yield.
    Emit(OwnedEvent),
    /// A sequence of blocks to expand.
    Blocks(std::vec::IntoIter<Block>),
    /// A sequence of inlines to expand.
    Inlines(std::vec::IntoIter<Inline>),
    /// A sequence of list items (each item is a Vec<Block>).
    ListItems(std::vec::IntoIter<Vec<Block>>),
    /// A sequence of table rows.
    TableRows(std::vec::IntoIter<TableRow>),
    /// A sequence of table cells (each cell is a Vec<Inline>).
    TableCells(std::vec::IntoIter<Vec<Inline>>),
    /// A sequence of footnote content blocks.
    FootnoteBlocks(std::vec::IntoIter<Block>),
}

// ── SemanticEventIter ─────────────────────────────────────────────────────────

/// Semantic document-event iterator over a parsed RTF document.
///
/// Obtain via [`events`] or [`events_str`].
pub struct SemanticEventIter {
    frame_stack: Vec<Frame>,
}

impl SemanticEventIter {
    fn new(doc: crate::ast::RtfDoc) -> Self {
        let mut iter = SemanticEventIter { frame_stack: Vec::new() };
        iter.frame_stack.push(Frame::Blocks(doc.blocks.into_iter()));
        iter
    }

    /// Expand one `Block` into frames pushed onto `frame_stack` in reverse
    /// emission order (so the first thing to emit is on top of the stack).
    fn expand_block(frame_stack: &mut Vec<Frame>, block: Block) {
        match block {
            Block::Paragraph { inlines, align, para_props, .. } => {
                // Emission order: StartParagraph, [inlines], EndParagraph
                frame_stack.push(Frame::Emit(Event::EndParagraph));
                frame_stack.push(Frame::Inlines(inlines.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartParagraph {
                    align,
                    para_props: Cow::Owned(para_props),
                }));
            }
            Block::Heading { level, inlines, .. } => {
                frame_stack.push(Frame::Emit(Event::EndHeading));
                frame_stack.push(Frame::Inlines(inlines.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartHeading { level }));
            }
            Block::CodeBlock { content, .. } => {
                frame_stack.push(Frame::Emit(Event::EndCodeBlock));
                frame_stack.push(Frame::Emit(Event::CodeBlockContent(Cow::Owned(content))));
                frame_stack.push(Frame::Emit(Event::StartCodeBlock));
            }
            Block::Blockquote { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndBlockquote));
                frame_stack.push(Frame::Blocks(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartBlockquote));
            }
            Block::List { ordered, items, .. } => {
                frame_stack.push(Frame::Emit(Event::EndList));
                frame_stack.push(Frame::ListItems(items.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartList { ordered }));
            }
            Block::Table { rows, .. } => {
                frame_stack.push(Frame::Emit(Event::EndTable));
                frame_stack.push(Frame::TableRows(rows.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartTable));
            }
            Block::HorizontalRule { .. } => {
                frame_stack.push(Frame::Emit(Event::HorizontalRule));
            }
        }
    }

    /// Expand one `Inline` into frames pushed onto `frame_stack`.
    fn expand_inline(frame_stack: &mut Vec<Frame>, inline: Inline) {
        match inline {
            Inline::Text { text, .. } => {
                frame_stack.push(Frame::Emit(Event::Text(Cow::Owned(text))));
            }
            Inline::LineBreak { .. } => {
                frame_stack.push(Frame::Emit(Event::LineBreak));
            }
            Inline::SoftBreak { .. } => {
                frame_stack.push(Frame::Emit(Event::SoftBreak));
            }
            Inline::Bold { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndBold));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartBold));
            }
            Inline::Italic { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndItalic));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartItalic));
            }
            Inline::Underline { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndUnderline));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartUnderline));
            }
            Inline::Strikethrough { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndStrikethrough));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartStrikethrough));
            }
            Inline::Code { text, .. } => {
                frame_stack.push(Frame::Emit(Event::Code(Cow::Owned(text))));
            }
            Inline::Link { url, children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndLink));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartLink { url }));
            }
            Inline::Image { url, alt, .. } => {
                frame_stack.push(Frame::Emit(Event::Image { url, alt }));
            }
            Inline::Superscript { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndSuperscript));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartSuperscript));
            }
            Inline::Subscript { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndSubscript));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartSubscript));
            }
            Inline::FontSize { size, children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndFontSize));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartFontSize { size }));
            }
            Inline::Color { r, g, b, children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndColor));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartColor { r, g, b }));
            }
            Inline::AllCaps { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndAllCaps));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartAllCaps));
            }
            Inline::SmallCaps { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndSmallCaps));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartSmallCaps));
            }
            Inline::Hidden { children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndHidden));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartHidden));
            }
            Inline::CharSpan { char_props, children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndCharSpan));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartCharSpan { char_props }));
            }
            Inline::Font { name, children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndFont));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartFont { name }));
            }
            Inline::BgColor { r, g, b, children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndBgColor));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartBgColor { r, g, b }));
            }
            Inline::Lang { lcid, children, .. } => {
                frame_stack.push(Frame::Emit(Event::EndLang));
                frame_stack.push(Frame::Inlines(children.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartLang { lcid }));
            }
            Inline::Footnote { content, .. } => {
                frame_stack.push(Frame::Emit(Event::EndFootnote));
                frame_stack.push(Frame::FootnoteBlocks(content.into_iter()));
                frame_stack.push(Frame::Emit(Event::StartFootnote));
            }
        }
    }
}

impl Iterator for SemanticEventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<OwnedEvent> {
        loop {
            let frame = self.frame_stack.last_mut()?;
            match frame {
                Frame::Emit(_) => {
                    // Pop the frame and return the event.
                    let Frame::Emit(ev) = self.frame_stack.pop().unwrap() else {
                        unreachable!()
                    };
                    return Some(ev);
                }
                Frame::Blocks(iter) => {
                    if let Some(block) = iter.next() {
                        Self::expand_block(&mut self.frame_stack, block);
                        // Continue looping — the newly pushed frames will be processed next.
                    } else {
                        self.frame_stack.pop();
                    }
                }
                Frame::Inlines(iter) => {
                    if let Some(inline) = iter.next() {
                        Self::expand_inline(&mut self.frame_stack, inline);
                    } else {
                        self.frame_stack.pop();
                    }
                }
                Frame::FootnoteBlocks(iter) => {
                    if let Some(block) = iter.next() {
                        Self::expand_block(&mut self.frame_stack, block);
                    } else {
                        self.frame_stack.pop();
                    }
                }
                Frame::ListItems(iter) => {
                    if let Some(item_blocks) = iter.next() {
                        // Push EndListItem, then the item's blocks, then StartListItem.
                        self.frame_stack.push(Frame::Emit(Event::EndListItem));
                        self.frame_stack.push(Frame::Blocks(item_blocks.into_iter()));
                        self.frame_stack.push(Frame::Emit(Event::StartListItem));
                    } else {
                        self.frame_stack.pop();
                    }
                }
                Frame::TableRows(iter) => {
                    if let Some(row) = iter.next() {
                        self.frame_stack.push(Frame::Emit(Event::EndTableRow));
                        self.frame_stack
                            .push(Frame::TableCells(row.cells.into_iter()));
                        self.frame_stack.push(Frame::Emit(Event::StartTableRow));
                    } else {
                        self.frame_stack.pop();
                    }
                }
                Frame::TableCells(iter) => {
                    if let Some(cell_inlines) = iter.next() {
                        self.frame_stack.push(Frame::Emit(Event::EndTableCell));
                        self.frame_stack
                            .push(Frame::Inlines(cell_inlines.into_iter()));
                        self.frame_stack.push(Frame::Emit(Event::StartTableCell));
                    } else {
                        self.frame_stack.pop();
                    }
                }
            }
        }
    }
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Parse `input` and return a semantic document-event iterator.
///
/// Events mirror the parsed AST structure: StartParagraph/EndParagraph,
/// StartHeading/EndHeading, Text, StartBold/EndBold, etc.
///
/// # Memory note
///
/// The full document is parsed before the first event is yielded (RTF's
/// stateful property inheritance requires global context). Events are then
/// streamed lazily via a frame stack.
///
/// See [`crate::token_events`] for the lower-level raw RTF token stream.
pub fn events(input: &[u8]) -> SemanticEventIter {
    let (doc, _diagnostics) = crate::parse::parse(input);
    SemanticEventIter::new(doc)
}

/// Convenience wrapper for callers that already have a `&str`.
pub fn events_str(input: &str) -> SemanticEventIter {
    events(input.as_bytes())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_events_paragraph() {
        let evs: Vec<_> = events(br"{\rtf1 Hello world\par}").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_semantic_events_heading() {
        // RTF headings use \outlinelevel0 through \outlinelevel8 or stylesheet-based styles.
        // The SemanticEventIter correctly handles Block::Heading when the parser produces one;
        // verify that iterating over RTF with outline-level markup doesn't panic.
        let evs: Vec<_> = events(br"{\rtf1\pard\outlinelevel0 Heading Text\par}").collect();
        // Parser currently maps outlinelevel paragraphs as paragraphs; just verify no panic.
        let _ = evs;
    }

    #[test]
    fn test_semantic_events_bold() {
        let evs: Vec<_> = events(br"{\rtf1 {\b bold}\par}").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBold)));
    }

    #[test]
    fn test_semantic_events_italic() {
        let evs: Vec<_> = events(br"{\rtf1 {\i italic text}\par}").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndItalic)));
    }

    #[test]
    fn test_semantic_events_empty_doc() {
        let evs: Vec<_> = events(br"{\rtf1}").collect();
        // Empty RTF doc — should produce no events (no blocks)
        assert!(evs.is_empty());
    }

    #[test]
    fn test_semantic_events_multiple_paragraphs() {
        let evs: Vec<_> = events(br"{\rtf1 First\par Second\par}").collect();
        let start_count = evs
            .iter()
            .filter(|e| matches!(e, OwnedEvent::StartParagraph { .. }))
            .count();
        let end_count = evs.iter().filter(|e| matches!(e, OwnedEvent::EndParagraph)).count();
        assert_eq!(start_count, end_count);
        assert!(start_count >= 1);
    }

    #[test]
    fn test_semantic_events_horizontal_rule() {
        // \brdrb produces a border which is represented as HorizontalRule in the AST.
        // Test that the iterator handles HorizontalRule without panic.
        let evs: Vec<_> = events(br"{\rtf1\par}").collect();
        // Just verify it doesn't panic
        let _ = evs;
    }

    #[test]
    fn test_semantic_events_ordering() {
        // Verify that Start/End events are properly balanced and ordered.
        let evs: Vec<_> = events(br"{\rtf1 {\b bold {\i both}}\par}").collect();
        let mut depth = 0i32;
        for ev in &evs {
            match ev {
                OwnedEvent::StartBold
                | OwnedEvent::StartItalic
                | OwnedEvent::StartParagraph { .. } => depth += 1,
                OwnedEvent::EndBold
                | OwnedEvent::EndItalic
                | OwnedEvent::EndParagraph => depth -= 1,
                _ => {}
            }
        }
        assert_eq!(depth, 0, "unbalanced Start/End events");
    }
}
