//! Streaming event iterator over a Textile document.
//!
//! # Example
//! ```no_run
//! use textile_fmt::events;
//!
//! for event in events("h1. Title\n\nHello *world*\n") {
//!     println!("{event:?}");
//! }
//! ```

use crate::ast::{Block, BlockAttrs, Inline, TextileDoc};

/// A streaming event from a Textile document.
#[derive(Debug, PartialEq, Clone)]
pub enum TextileEvent {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph {
        align: Option<String>,
        attrs: BlockAttrs,
    },
    EndParagraph,
    StartHeading {
        level: u8,
        attrs: BlockAttrs,
    },
    EndHeading,
    /// Leaf: a code block (`bc. ...`).
    CodeBlock {
        content: String,
        language: Option<String>,
    },
    StartBlockquote {
        attrs: BlockAttrs,
    },
    EndBlockquote,
    StartList {
        ordered: bool,
    },
    EndList,
    StartListItem,
    EndListItem,
    StartTable,
    EndTable,
    StartTableRow {
        attrs: BlockAttrs,
    },
    EndTableRow,
    StartTableCell {
        is_header: bool,
        align: Option<String>,
    },
    EndTableCell,
    /// Leaf: horizontal rule (`---`).
    HorizontalRule,
    /// Leaf: footnote definition (`fn1. content`).
    StartFootnoteDef {
        label: String,
    },
    EndFootnoteDef,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    /// Raw (notextile) block: verbatim content.
    RawBlock {
        content: String,
    },

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(String),
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartUnderline,
    EndUnderline,
    StartStrikethrough,
    EndStrikethrough,
    /// Leaf: inline code (`@code@`).
    InlineCode(String),
    StartLink {
        url: String,
        title: Option<String>,
    },
    EndLink,
    /// Leaf: inline image (`!url(alt)!`).
    InlineImage {
        url: String,
        alt: Option<String>,
    },
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    /// Leaf: inline footnote reference (`[1]`).
    FootnoteRef {
        label: String,
    },
    /// Hard line break.
    LineBreak,
    /// Raw inline (`==content==`).
    RawInline {
        content: String,
    },
    StartCitation,
    EndCitation,
    StartGenericSpan {
        attrs: BlockAttrs,
    },
    EndGenericSpan,
    /// Leaf: acronym (`ABC(meaning)`).
    Acronym {
        text: String,
        title: String,
    },
}

// ── Event iterator ────────────────────────────────────────────────────────────

/// Iterator over [`TextileEvent`]s produced from a parsed [`TextileDoc`].
pub struct EventIter {
    /// Flat list of events, emitted front-to-back.
    queue: Vec<TextileEvent>,
    pos: usize,
}

impl EventIter {
    fn new(doc: &TextileDoc) -> Self {
        let mut queue = Vec::new();
        for block in &doc.blocks {
            push_block_events(block, &mut queue);
        }
        EventIter { queue, pos: 0 }
    }
}

impl Iterator for EventIter {
    type Item = TextileEvent;

    fn next(&mut self) -> Option<TextileEvent> {
        if self.pos < self.queue.len() {
            let ev = self.queue[self.pos].clone();
            self.pos += 1;
            Some(ev)
        } else {
            None
        }
    }
}

// ── AST → event helpers ───────────────────────────────────────────────────────

fn push_block_events(block: &Block, out: &mut Vec<TextileEvent>) {
    match block {
        Block::Paragraph { inlines, align, attrs, .. } => {
            out.push(TextileEvent::StartParagraph {
                align: align.clone(),
                attrs: attrs.clone(),
            });
            for inline in inlines {
                push_inline_events(inline, out);
            }
            out.push(TextileEvent::EndParagraph);
        }

        Block::Heading { level, inlines, attrs, .. } => {
            out.push(TextileEvent::StartHeading { level: *level, attrs: attrs.clone() });
            for inline in inlines {
                push_inline_events(inline, out);
            }
            out.push(TextileEvent::EndHeading);
        }

        Block::CodeBlock { content, language, .. } => {
            out.push(TextileEvent::CodeBlock {
                content: content.clone(),
                language: language.clone(),
            });
        }

        Block::Blockquote { blocks, attrs, .. } => {
            out.push(TextileEvent::StartBlockquote { attrs: attrs.clone() });
            for b in blocks {
                push_block_events(b, out);
            }
            out.push(TextileEvent::EndBlockquote);
        }

        Block::List { ordered, items, .. } => {
            out.push(TextileEvent::StartList { ordered: *ordered });
            for item_blocks in items {
                out.push(TextileEvent::StartListItem);
                for b in item_blocks {
                    push_block_events(b, out);
                }
                out.push(TextileEvent::EndListItem);
            }
            out.push(TextileEvent::EndList);
        }

        Block::Table { rows, .. } => {
            out.push(TextileEvent::StartTable);
            for row in rows {
                out.push(TextileEvent::StartTableRow { attrs: row.attrs.clone() });
                for cell in &row.cells {
                    out.push(TextileEvent::StartTableCell {
                        is_header: cell.is_header,
                        align: cell.align.clone(),
                    });
                    for inline in &cell.inlines {
                        push_inline_events(inline, out);
                    }
                    out.push(TextileEvent::EndTableCell);
                }
                out.push(TextileEvent::EndTableRow);
            }
            out.push(TextileEvent::EndTable);
        }

        Block::HorizontalRule { .. } => {
            out.push(TextileEvent::HorizontalRule);
        }

        Block::FootnoteDef { label, inlines, .. } => {
            out.push(TextileEvent::StartFootnoteDef { label: label.clone() });
            for inline in inlines {
                push_inline_events(inline, out);
            }
            out.push(TextileEvent::EndFootnoteDef);
        }

        Block::DefinitionList { items, .. } => {
            out.push(TextileEvent::StartDefinitionList);
            for (term, def) in items {
                out.push(TextileEvent::StartDefinitionTerm);
                for inline in term {
                    push_inline_events(inline, out);
                }
                out.push(TextileEvent::EndDefinitionTerm);
                out.push(TextileEvent::StartDefinitionDesc);
                for inline in def {
                    push_inline_events(inline, out);
                }
                out.push(TextileEvent::EndDefinitionDesc);
            }
            out.push(TextileEvent::EndDefinitionList);
        }

        Block::Raw { content, .. } => {
            out.push(TextileEvent::RawBlock { content: content.clone() });
        }
    }
}

fn push_inline_events(inline: &Inline, out: &mut Vec<TextileEvent>) {
    match inline {
        Inline::Text(s, _) => out.push(TextileEvent::Text(s.clone())),

        Inline::Bold(children, _) => {
            out.push(TextileEvent::StartBold);
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndBold);
        }

        Inline::Italic(children, _) => {
            out.push(TextileEvent::StartItalic);
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndItalic);
        }

        Inline::Underline(children, _) => {
            out.push(TextileEvent::StartUnderline);
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndUnderline);
        }

        Inline::Strikethrough(children, _) => {
            out.push(TextileEvent::StartStrikethrough);
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndStrikethrough);
        }

        Inline::Code(s, _) => out.push(TextileEvent::InlineCode(s.clone())),

        Inline::Link { url, title, children, .. } => {
            out.push(TextileEvent::StartLink { url: url.clone(), title: title.clone() });
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndLink);
        }

        Inline::Image { url, alt, .. } => {
            out.push(TextileEvent::InlineImage { url: url.clone(), alt: alt.clone() });
        }

        Inline::Superscript(children, _) => {
            out.push(TextileEvent::StartSuperscript);
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndSuperscript);
        }

        Inline::Subscript(children, _) => {
            out.push(TextileEvent::StartSubscript);
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndSubscript);
        }

        Inline::FootnoteRef { label, .. } => {
            out.push(TextileEvent::FootnoteRef { label: label.clone() });
        }

        Inline::LineBreak(_) => out.push(TextileEvent::LineBreak),

        Inline::Raw(s, _) => out.push(TextileEvent::RawInline { content: s.clone() }),

        Inline::Citation(children, _) => {
            out.push(TextileEvent::StartCitation);
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndCitation);
        }

        Inline::GenericSpan { attrs, children, .. } => {
            out.push(TextileEvent::StartGenericSpan { attrs: attrs.clone() });
            for c in children {
                push_inline_events(c, out);
            }
            out.push(TextileEvent::EndGenericSpan);
        }

        Inline::Acronym { text, title, .. } => {
            out.push(TextileEvent::Acronym { text: text.clone(), title: title.clone() });
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse `input` and return a streaming iterator of [`TextileEvent`]s.
pub fn events(input: &str) -> EventIter {
    let (doc, _diags) = crate::parse::parse(input);
    EventIter::new(&doc)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("h1. Title\n").collect();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartHeading { level: 1, .. })));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world\n").collect();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartParagraph { .. })));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_bold_italic() {
        let evs: Vec<_> = events("*bold* _italic_\n").collect();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::EndBold)));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::EndItalic)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events("bc. code here\n").collect();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::CodeBlock { .. })));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("* item1\n* item2\n").collect();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartList { ordered: false })));
        assert_eq!(
            evs.iter().filter(|e| matches!(e, TextileEvent::StartListItem)).count(),
            2
        );
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::EndList)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("|cell1|cell2|\n").collect();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("---\n").collect();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::HorizontalRule)));
    }

    #[test]
    fn test_events_footnote_def() {
        let evs: Vec<_> = events("fn1. Footnote content\n").collect();
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::StartFootnoteDef { label } if label == "1")));
        assert!(evs.iter().any(|e| matches!(e, TextileEvent::EndFootnoteDef)));
    }
}
