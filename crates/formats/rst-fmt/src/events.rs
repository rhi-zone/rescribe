//! Streaming event iterator over a parsed [`RstDoc`].

use std::collections::VecDeque;

use crate::{Block, DefinitionItem, Inline, RstDoc, TableRow};

/// An owned event from an RST document.
#[derive(Debug, Clone)]
pub enum OwnedEvent {
    // Block events
    StartParagraph,
    EndParagraph,
    StartHeading { level: i64 },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList { ordered: bool },
    EndList,
    StartListItem,
    EndListItem,
    StartCodeBlock { language: Option<String> },
    EndCodeBlock,
    CodeBlockContent(String),
    RawBlock { format: String, content: String },
    StartDiv { class: Option<String>, directive: Option<String> },
    EndDiv,
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartFootnoteDef { label: String },
    EndFootnoteDef,
    MathDisplay { source: String },
    StartAdmonition { admonition_type: String },
    EndAdmonition,
    StartFigure { url: String, alt: Option<String> },
    EndFigure,
    /// Image block (standalone, no caption)
    ImageBlock { url: String, alt: Option<String>, title: Option<String> },
    StartLineBlock,
    EndLineBlock,
    StartLineBlockLine,
    EndLineBlockLine,
    // Inline events
    Text(String),
    SoftBreak,
    LineBreak,
    StartEmphasis,
    EndEmphasis,
    StartStrong,
    EndStrong,
    StartStrikeout,
    EndStrikeout,
    StartUnderline,
    EndUnderline,
    StartSubscript,
    EndSubscript,
    StartSuperscript,
    EndSuperscript,
    StartSmallCaps,
    EndSmallCaps,
    Code(String),
    StartLink { url: String },
    EndLink,
    InlineImage { url: String, alt: String },
    FootnoteRef { label: String },
    StartFootnoteDefInline { label: String },
    EndFootnoteDefInline,
    StartQuoted { quote_type: String },
    EndQuoted,
    MathInline { source: String },
    StartRstSpan { role: String },
    EndRstSpan,
}

/// A public event iterator that yields [`OwnedEvent`] items.
pub struct EventIter {
    queue: VecDeque<OwnedEvent>,
}

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front()
    }
}

/// Walk an [`RstDoc`] and collect events into the given queue.
pub(crate) fn events_from_doc(doc: &RstDoc, queue: &mut VecDeque<OwnedEvent>) {
    collect_blocks_events(&doc.blocks, queue);
}

fn collect_blocks_events(blocks: &[Block], queue: &mut VecDeque<OwnedEvent>) {
    for block in blocks {
        collect_block_events(block, queue);
    }
}

fn collect_block_events(block: &Block, queue: &mut VecDeque<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines } => {
            queue.push_back(OwnedEvent::StartParagraph);
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndParagraph);
        }
        Block::Heading { level, inlines } => {
            queue.push_back(OwnedEvent::StartHeading { level: *level });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndHeading);
        }
        Block::CodeBlock { language, content } => {
            queue.push_back(OwnedEvent::StartCodeBlock { language: language.clone() });
            queue.push_back(OwnedEvent::CodeBlockContent(content.clone()));
            queue.push_back(OwnedEvent::EndCodeBlock);
        }
        Block::Blockquote { children } => {
            queue.push_back(OwnedEvent::StartBlockquote);
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndBlockquote);
        }
        Block::List { ordered, items } => {
            queue.push_back(OwnedEvent::StartList { ordered: *ordered });
            for item in items {
                queue.push_back(OwnedEvent::StartListItem);
                collect_blocks_events(item, queue);
                queue.push_back(OwnedEvent::EndListItem);
            }
            queue.push_back(OwnedEvent::EndList);
        }
        Block::DefinitionList { items } => {
            queue.push_back(OwnedEvent::StartDefinitionList);
            for item in items {
                collect_definition_item_events(item, queue);
            }
            queue.push_back(OwnedEvent::EndDefinitionList);
        }
        Block::Figure { url, alt, caption } => {
            queue.push_back(OwnedEvent::StartFigure { url: url.clone(), alt: alt.clone() });
            if let Some(cap) = caption {
                collect_inlines_events(cap, queue);
            }
            queue.push_back(OwnedEvent::EndFigure);
        }
        Block::Image { url, alt, title } => {
            queue.push_back(OwnedEvent::ImageBlock {
                url: url.clone(),
                alt: alt.clone(),
                title: title.clone(),
            });
        }
        Block::RawBlock { format, content } => {
            queue.push_back(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Block::Div { class, directive, children } => {
            queue.push_back(OwnedEvent::StartDiv {
                class: class.clone(),
                directive: directive.clone(),
            });
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndDiv);
        }
        Block::HorizontalRule => {
            queue.push_back(OwnedEvent::HorizontalRule);
        }
        Block::Table { rows } => {
            queue.push_back(OwnedEvent::StartTable);
            for row in rows {
                collect_table_row_events(row, queue);
            }
            queue.push_back(OwnedEvent::EndTable);
        }
        Block::FootnoteDef { label, inlines } => {
            queue.push_back(OwnedEvent::StartFootnoteDef { label: label.clone() });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndFootnoteDef);
        }
        Block::MathDisplay { source } => {
            queue.push_back(OwnedEvent::MathDisplay { source: source.clone() });
        }
        Block::Admonition { admonition_type, children } => {
            queue.push_back(OwnedEvent::StartAdmonition { admonition_type: admonition_type.clone() });
            collect_blocks_events(children, queue);
            queue.push_back(OwnedEvent::EndAdmonition);
        }
        Block::LineBlock { lines } => {
            queue.push_back(OwnedEvent::StartLineBlock);
            for line in lines {
                queue.push_back(OwnedEvent::StartLineBlockLine);
                collect_inlines_events(line, queue);
                queue.push_back(OwnedEvent::EndLineBlockLine);
            }
            queue.push_back(OwnedEvent::EndLineBlock);
        }
    }
}

fn collect_definition_item_events(item: &DefinitionItem, queue: &mut VecDeque<OwnedEvent>) {
    queue.push_back(OwnedEvent::StartDefinitionTerm);
    collect_inlines_events(&item.term, queue);
    queue.push_back(OwnedEvent::EndDefinitionTerm);
    queue.push_back(OwnedEvent::StartDefinitionDesc);
    collect_inlines_events(&item.desc, queue);
    queue.push_back(OwnedEvent::EndDefinitionDesc);
}

fn collect_table_row_events(row: &TableRow, queue: &mut VecDeque<OwnedEvent>) {
    queue.push_back(OwnedEvent::StartTableRow { is_header: row.is_header });
    for cell in &row.cells {
        queue.push_back(OwnedEvent::StartTableCell);
        collect_inlines_events(cell, queue);
        queue.push_back(OwnedEvent::EndTableCell);
    }
    queue.push_back(OwnedEvent::EndTableRow);
}

fn collect_inlines_events(inlines: &[Inline], queue: &mut VecDeque<OwnedEvent>) {
    for inline in inlines {
        collect_inline_events(inline, queue);
    }
}

fn collect_inline_events(inline: &Inline, queue: &mut VecDeque<OwnedEvent>) {
    match inline {
        Inline::Text(s) => {
            queue.push_back(OwnedEvent::Text(s.clone()));
        }
        Inline::SoftBreak => {
            queue.push_back(OwnedEvent::SoftBreak);
        }
        Inline::LineBreak => {
            queue.push_back(OwnedEvent::LineBreak);
        }
        Inline::Emphasis(children) => {
            queue.push_back(OwnedEvent::StartEmphasis);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndEmphasis);
        }
        Inline::Strong(children) => {
            queue.push_back(OwnedEvent::StartStrong);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndStrong);
        }
        Inline::Strikeout(children) => {
            queue.push_back(OwnedEvent::StartStrikeout);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndStrikeout);
        }
        Inline::Underline(children) => {
            queue.push_back(OwnedEvent::StartUnderline);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndUnderline);
        }
        Inline::Subscript(children) => {
            queue.push_back(OwnedEvent::StartSubscript);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSubscript);
        }
        Inline::Superscript(children) => {
            queue.push_back(OwnedEvent::StartSuperscript);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSuperscript);
        }
        Inline::SmallCaps(children) => {
            queue.push_back(OwnedEvent::StartSmallCaps);
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndSmallCaps);
        }
        Inline::Code(s) => {
            queue.push_back(OwnedEvent::Code(s.clone()));
        }
        Inline::Link { url, children } => {
            queue.push_back(OwnedEvent::StartLink { url: url.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndLink);
        }
        Inline::Image { url, alt } => {
            queue.push_back(OwnedEvent::InlineImage { url: url.clone(), alt: alt.clone() });
        }
        Inline::FootnoteRef { label } => {
            queue.push_back(OwnedEvent::FootnoteRef { label: label.clone() });
        }
        Inline::FootnoteDef { label, children } => {
            queue.push_back(OwnedEvent::StartFootnoteDefInline { label: label.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndFootnoteDefInline);
        }
        Inline::Quoted { quote_type, children } => {
            queue.push_back(OwnedEvent::StartQuoted { quote_type: quote_type.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndQuoted);
        }
        Inline::MathInline { source } => {
            queue.push_back(OwnedEvent::MathInline { source: source.clone() });
        }
        Inline::RstSpan { role, children } => {
            queue.push_back(OwnedEvent::StartRstSpan { role: role.clone() });
            collect_inlines_events(children, queue);
            queue.push_back(OwnedEvent::EndRstSpan);
        }
    }
}

/// Parse `input` as RST and return a streaming event iterator.
///
/// On parse error, returns an iterator over an empty document.
pub fn events(input: &str) -> EventIter {
    let doc = crate::parse(input).unwrap_or_default();
    let mut queue = VecDeque::new();
    events_from_doc(&doc, &mut queue);
    EventIter { queue }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("Section\n=======\n").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events(".. code-block:: rust\n\n   let x = 1;\n").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartCodeBlock { language: Some(l) } if l == "rust")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndCodeBlock)));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("- item one\n- item two\n").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartList { ordered: false })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartListItem)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }
}
