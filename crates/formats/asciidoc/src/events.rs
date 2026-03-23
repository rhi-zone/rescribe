//! Streaming event iterator over a parsed `AsciiDoc`.

use crate::ast::*;
use std::collections::VecDeque;

/// An owned event from an AsciiDoc document.
#[derive(Debug)]
pub enum OwnedEvent {
    // ── Document ──────────────────────────────────────────────────────────────
    StartDocument,
    EndDocument,

    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph { id: Option<String>, role: Option<String>, checked: Option<bool> },
    EndParagraph,
    StartHeading { level: usize, id: Option<String>, role: Option<String> },
    EndHeading,
    StartCodeBlock { language: Option<String> },
    EndCodeBlock,
    CodeBlockContent(String),
    StartBlockquote { attribution: Option<String> },
    EndBlockquote,
    StartList { ordered: bool, style: Option<String> },
    EndList,
    StartListItem,
    EndListItem,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    HorizontalRule,
    PageBreak,
    Figure { url: String, alt: Option<String>, title: Option<String> },
    StartDiv { class: Option<String>, title: Option<String> },
    EndDiv,
    RawBlock { format: String, content: String },
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell,
    EndTableCell,

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(String),
    SoftBreak,
    LineBreak,
    StartStrong,
    EndStrong,
    StartEmphasis,
    EndEmphasis,
    Code(String),
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    StartHighlight,
    EndHighlight,
    StartStrikeout,
    EndStrikeout,
    StartUnderline,
    EndUnderline,
    StartSmallCaps,
    EndSmallCaps,
    StartQuoted { quote_type: String },
    EndQuoted,
    StartLink { url: String, target: Option<String> },
    EndLink,
    InlineImage { url: String, alt: Option<String>, title: Option<String> },
    FootnoteRef { label: String },
    StartFootnoteDef { label: String },
    EndFootnoteDef,
    MathInline { source: String },
    MathDisplay { source: String },
    RawInline { format: String, content: String },
    Anchor { id: String },
}

/// Public iterator that yields `OwnedEvent` items.
pub struct EventIter {
    queue: VecDeque<OwnedEvent>,
}

impl EventIter {
    pub(crate) fn new(input: &str) -> Self {
        let (doc, _diags) = crate::parse::parse(input);
        let mut queue = VecDeque::new();
        collect_doc_events(&doc, &mut queue);
        EventIter { queue }
    }
}

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front()
    }
}

fn collect_doc_events(doc: &AsciiDoc, queue: &mut VecDeque<OwnedEvent>) {
    queue.push_back(OwnedEvent::StartDocument);
    collect_blocks_events(&doc.blocks, queue);
    queue.push_back(OwnedEvent::EndDocument);
}

fn collect_blocks_events(blocks: &[Block], queue: &mut VecDeque<OwnedEvent>) {
    for block in blocks {
        collect_block_events(block, queue);
    }
}

fn collect_block_events(block: &Block, queue: &mut VecDeque<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, id, role, checked, .. } => {
            queue.push_back(OwnedEvent::StartParagraph {
                id: id.clone(),
                role: role.clone(),
                checked: *checked,
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndParagraph);
        }
        Block::Heading { level, inlines, id, role, .. } => {
            queue.push_back(OwnedEvent::StartHeading {
                level: *level,
                id: id.clone(),
                role: role.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndHeading);
        }
        Block::CodeBlock { content, language, .. } => {
            queue.push_back(OwnedEvent::StartCodeBlock { language: language.clone() });
            queue.push_back(OwnedEvent::CodeBlockContent(content.clone()));
            queue.push_back(OwnedEvent::EndCodeBlock);
        }
        Block::Blockquote { children, attribution, .. } => {
            queue.push_back(OwnedEvent::StartBlockquote { attribution: attribution.clone() });
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndBlockquote);
        }
        Block::List { ordered, items, style, .. } => {
            queue.push_back(OwnedEvent::StartList {
                ordered: *ordered,
                style: style.clone(),
            });
            for item in items {
                queue.push_back(OwnedEvent::StartListItem);
                collect_blocks_events(item, queue);
                queue.push_back(OwnedEvent::EndListItem);
            }
            queue.push_back(OwnedEvent::EndList);
        }
        Block::DefinitionList { items, .. } => {
            queue.push_back(OwnedEvent::StartDefinitionList);
            for item in items {
                queue.push_back(OwnedEvent::StartDefinitionTerm);
                collect_inlines_events(&item.term, queue);
                queue.push_back(OwnedEvent::EndDefinitionTerm);
                queue.push_back(OwnedEvent::StartDefinitionDesc);
                collect_inlines_events(&item.desc, queue);
                queue.push_back(OwnedEvent::EndDefinitionDesc);
            }
            queue.push_back(OwnedEvent::EndDefinitionList);
        }
        Block::HorizontalRule { .. } => {
            queue.push_back(OwnedEvent::HorizontalRule);
        }
        Block::PageBreak { .. } => {
            queue.push_back(OwnedEvent::PageBreak);
        }
        Block::Figure { image, .. } => {
            queue.push_back(OwnedEvent::Figure {
                url: image.url.clone(),
                alt: image.alt.clone(),
                title: image.height.clone().or_else(|| image.width.clone()),
            });
        }
        Block::Div { class, title, children, .. } => {
            queue.push_back(OwnedEvent::StartDiv {
                class: class.clone(),
                title: title.clone(),
            });
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndDiv);
        }
        Block::RawBlock { format, content, .. } => {
            queue.push_back(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Block::Table { rows, .. } => {
            queue.push_back(OwnedEvent::StartTable);
            for row in rows {
                queue.push_back(OwnedEvent::StartTableRow { is_header: row.is_header });
                for cell in &row.cells {
                    queue.push_back(OwnedEvent::StartTableCell);
                    collect_inlines_events(cell, queue);
                    queue.push_back(OwnedEvent::EndTableCell);
                }
                queue.push_back(OwnedEvent::EndTableRow);
            }
            queue.push_back(OwnedEvent::EndTable);
        }
    }
}

fn collect_inlines_events(inlines: &[Inline], queue: &mut VecDeque<OwnedEvent>) {
    for inline in inlines {
        collect_inline_events(inline, queue);
    }
}

fn collect_inline_events(inline: &Inline, queue: &mut VecDeque<OwnedEvent>) {
    match inline {
        Inline::Text { text, .. } => {
            queue.push_back(OwnedEvent::Text(text.clone()));
        }
        Inline::SoftBreak { .. } => {
            queue.push_back(OwnedEvent::SoftBreak);
        }
        Inline::LineBreak { .. } => {
            queue.push_back(OwnedEvent::LineBreak);
        }
        Inline::Strong(children, _) => {
            queue.push_back(OwnedEvent::StartStrong);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndStrong);
        }
        Inline::Emphasis(children, _) => {
            queue.push_back(OwnedEvent::StartEmphasis);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndEmphasis);
        }
        Inline::Code(content, _) => {
            queue.push_back(OwnedEvent::Code(content.clone()));
        }
        Inline::Superscript(children, _) => {
            queue.push_back(OwnedEvent::StartSuperscript);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            queue.push_back(OwnedEvent::StartSubscript);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSubscript);
        }
        Inline::Highlight(children, _) => {
            queue.push_back(OwnedEvent::StartHighlight);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndHighlight);
        }
        Inline::Strikeout(children, _) => {
            queue.push_back(OwnedEvent::StartStrikeout);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndStrikeout);
        }
        Inline::Underline(children, _) => {
            queue.push_back(OwnedEvent::StartUnderline);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndUnderline);
        }
        Inline::SmallCaps(children, _) => {
            queue.push_back(OwnedEvent::StartSmallCaps);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSmallCaps);
        }
        Inline::Quoted { quote_type, children, .. } => {
            let qt = match quote_type {
                QuoteType::Single => "single".to_string(),
                QuoteType::Double => "double".to_string(),
            };
            queue.push_back(OwnedEvent::StartQuoted { quote_type: qt });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndQuoted);
        }
        Inline::Link { url, children, target, .. } => {
            queue.push_back(OwnedEvent::StartLink {
                url: url.clone(),
                target: target.clone(),
            });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndLink);
        }
        Inline::Image(img, _) => {
            queue.push_back(OwnedEvent::InlineImage {
                url: img.url.clone(),
                alt: img.alt.clone(),
                title: img.height.clone().or_else(|| img.width.clone()),
            });
        }
        Inline::FootnoteRef { label, .. } => {
            queue.push_back(OwnedEvent::FootnoteRef { label: label.clone() });
        }
        Inline::FootnoteDef { label, children, .. } => {
            queue.push_back(OwnedEvent::StartFootnoteDef { label: label.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndFootnoteDef);
        }
        Inline::MathInline { source, .. } => {
            queue.push_back(OwnedEvent::MathInline { source: source.clone() });
        }
        Inline::MathDisplay { source, .. } => {
            queue.push_back(OwnedEvent::MathDisplay { source: source.clone() });
        }
        Inline::RawInline { format, content, .. } => {
            queue.push_back(OwnedEvent::RawInline {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Inline::Anchor { id, .. } => {
            queue.push_back(OwnedEvent::Anchor { id: id.clone() });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = EventIter::new("== Hello World").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 2, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello World")));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = EventIter::new("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = EventIter::new("[source,python]\n----\nprint('hello')\n----").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartCodeBlock { language: Some(l) } if l == "python")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndCodeBlock)));
    }

    #[test]
    fn test_events_strong() {
        let evs: Vec<_> = EventIter::new("This is *bold* text.").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartStrong)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndStrong)));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = EventIter::new("* item one\n* item two").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartList { ordered: false, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartListItem)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }

    #[test]
    fn test_events_document_wrap() {
        let evs: Vec<_> = EventIter::new("Hello").collect();
        assert!(matches!(evs.first(), Some(OwnedEvent::StartDocument)));
        assert!(matches!(evs.last(), Some(OwnedEvent::EndDocument)));
    }
}
