//! Streaming event iterator over a parsed `DjotDoc`.

use crate::ast::*;

/// A streaming event from a Djot document (borrowed variant — kept for reference).
#[allow(dead_code)]
#[derive(Debug)]
pub enum Event<'a> {
    // Block events
    StartParagraph { attr: &'a Attr },
    EndParagraph,
    StartHeading { level: u8, attr: &'a Attr },
    EndHeading,
    StartBlockquote { attr: &'a Attr },
    EndBlockquote,
    StartList { kind: &'a ListKind, tight: bool, attr: &'a Attr },
    EndList,
    StartListItem { checked: Option<bool> },
    EndListItem,
    StartCodeBlock { language: Option<&'a str>, attr: &'a Attr },
    EndCodeBlock,
    CodeBlockContent(&'a str),
    RawBlock { format: &'a str, content: &'a str },
    StartDiv { class: Option<&'a str>, attr: &'a Attr },
    EndDiv,
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell { alignment: Alignment },
    EndTableCell,
    ThematicBreak { attr: &'a Attr },
    StartDefinitionList { attr: &'a Attr },
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartFootnoteDef { label: &'a str },
    EndFootnoteDef,
    // Inline events
    Text(&'a str),
    SoftBreak,
    HardBreak,
    StartEmphasis { attr: &'a Attr },
    EndEmphasis,
    StartStrong { attr: &'a Attr },
    EndStrong,
    StartDelete { attr: &'a Attr },
    EndDelete,
    StartInsert { attr: &'a Attr },
    EndInsert,
    StartHighlight { attr: &'a Attr },
    EndHighlight,
    StartSubscript { attr: &'a Attr },
    EndSubscript,
    StartSuperscript { attr: &'a Attr },
    EndSuperscript,
    Verbatim { content: &'a str, attr: &'a Attr },
    MathInline(&'a str),
    MathDisplay(&'a str),
    RawInline { format: &'a str, content: &'a str },
    StartLink { url: &'a str, title: Option<&'a str>, attr: &'a Attr },
    EndLink,
    StartImage { url: &'a str, title: Option<&'a str>, attr: &'a Attr },
    EndImage,
    StartSpan { attr: &'a Attr },
    EndSpan,
    FootnoteRef(&'a str),
    Symbol(&'a str),
    Autolink { url: &'a str, is_email: bool },
}

/// An iterator that owns the `DjotDoc` and yields events from it.
struct DocEventIterator {
    doc: Box<DjotDoc>,
    // We need to yield events that borrow from `doc`. We pre-collect them
    // into an owned queue using String-based versions, then yield those.
    queue: std::collections::VecDeque<OwnedEvent>,
    pos: usize,
}

/// An owned version of Event (no lifetime) for the internal queue.
#[derive(Debug)]
pub enum OwnedEvent {
    StartParagraph { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndParagraph,
    StartHeading { level: u8, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndHeading,
    StartBlockquote { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndBlockquote,
    StartList { kind: ListKind, tight: bool, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndList,
    StartListItem { checked: Option<bool> },
    EndListItem,
    StartCodeBlock { language: Option<String>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndCodeBlock,
    CodeBlockContent(String),
    RawBlock { format: String, content: String },
    StartDiv { class: Option<String>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndDiv,
    StartTable,
    EndTable,
    StartTableRow { is_header: bool },
    EndTableRow,
    StartTableCell { alignment: Alignment },
    EndTableCell,
    ThematicBreak { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    StartDefinitionList { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartFootnoteDef { label: String },
    EndFootnoteDef,
    Text(String),
    SoftBreak,
    HardBreak,
    StartEmphasis { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndEmphasis,
    StartStrong { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndStrong,
    StartDelete { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndDelete,
    StartInsert { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndInsert,
    StartHighlight { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndHighlight,
    StartSubscript { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndSubscript,
    StartSuperscript { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndSuperscript,
    Verbatim { content: String, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    MathInline(String),
    MathDisplay(String),
    RawInline { format: String, content: String },
    StartLink { url: String, title: Option<String>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndLink,
    StartImage { url: String, title: Option<String>, id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndImage,
    StartSpan { id: Option<String>, classes: Vec<String>, kv: Vec<(String, String)> },
    EndSpan,
    FootnoteRef(String),
    Symbol(String),
    Autolink { url: String, is_email: bool },
}

impl DocEventIterator {
    fn new(input: &str) -> Self {
        let (doc, _diags) = crate::parse::parse(input);
        let doc = Box::new(doc);
        let mut queue = std::collections::VecDeque::new();
        collect_doc_events(&doc, &mut queue);
        DocEventIterator { doc, queue, pos: 0 }
    }
}

fn collect_doc_events(doc: &DjotDoc, queue: &mut std::collections::VecDeque<OwnedEvent>) {
    collect_blocks_events(&doc.blocks, queue);
    for fn_def in &doc.footnotes {
        queue.push_back(OwnedEvent::StartFootnoteDef { label: fn_def.label.clone() });
        collect_blocks_events(&fn_def.blocks, queue);
        queue.push_back(OwnedEvent::EndFootnoteDef);
    }
}

fn collect_blocks_events(blocks: &[Block], queue: &mut std::collections::VecDeque<OwnedEvent>) {
    for block in blocks {
        collect_block_events(block, queue);
    }
}

fn collect_block_events(block: &Block, queue: &mut std::collections::VecDeque<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartParagraph {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndParagraph);
        }
        Block::Heading { level, inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartHeading {
                level: *level,
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndHeading);
        }
        Block::Blockquote { blocks, attr, .. } => {
            queue.push_back(OwnedEvent::StartBlockquote {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_blocks_events(blocks, queue);
            queue.push_back(OwnedEvent::EndBlockquote);
        }
        Block::List { kind, items, tight, attr, .. } => {
            queue.push_back(OwnedEvent::StartList {
                kind: kind.clone(),
                tight: *tight,
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            for item in items {
                queue.push_back(OwnedEvent::StartListItem { checked: item.checked });
                collect_blocks_events(&item.blocks, queue);
                queue.push_back(OwnedEvent::EndListItem);
            }
            queue.push_back(OwnedEvent::EndList);
        }
        Block::CodeBlock { language, content, attr, .. } => {
            queue.push_back(OwnedEvent::StartCodeBlock {
                language: language.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            queue.push_back(OwnedEvent::CodeBlockContent(content.clone()));
            queue.push_back(OwnedEvent::EndCodeBlock);
        }
        Block::RawBlock { format, content, .. } => {
            queue.push_back(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Block::Div { class, blocks, attr, .. } => {
            queue.push_back(OwnedEvent::StartDiv {
                class: class.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_blocks_events(blocks, queue);
            queue.push_back(OwnedEvent::EndDiv);
        }
        Block::Table { caption: _, rows, .. } => {
            queue.push_back(OwnedEvent::StartTable);
            for row in rows {
                queue.push_back(OwnedEvent::StartTableRow { is_header: row.is_header });
                for cell in &row.cells {
                    queue.push_back(OwnedEvent::StartTableCell { alignment: cell.alignment.clone() });
                    collect_inlines_events(&cell.inlines, queue);
                    queue.push_back(OwnedEvent::EndTableCell);
                }
                queue.push_back(OwnedEvent::EndTableRow);
            }
            queue.push_back(OwnedEvent::EndTable);
        }
        Block::ThematicBreak { attr, .. } => {
            queue.push_back(OwnedEvent::ThematicBreak {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
        }
        Block::DefinitionList { items, attr, .. } => {
            queue.push_back(OwnedEvent::StartDefinitionList {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            for item in items {
                queue.push_back(OwnedEvent::StartDefinitionTerm);
                collect_inlines_events(&item.term, queue);
                queue.push_back(OwnedEvent::EndDefinitionTerm);
                queue.push_back(OwnedEvent::StartDefinitionDesc);
                collect_blocks_events(&item.definitions, queue);
                queue.push_back(OwnedEvent::EndDefinitionDesc);
            }
            queue.push_back(OwnedEvent::EndDefinitionList);
        }
    }
}

fn collect_inlines_events(inlines: &[Inline], queue: &mut std::collections::VecDeque<OwnedEvent>) {
    for inline in inlines {
        collect_inline_events(inline, queue);
    }
}

fn collect_inline_events(inline: &Inline, queue: &mut std::collections::VecDeque<OwnedEvent>) {
    match inline {
        Inline::Text { content, .. } => {
            queue.push_back(OwnedEvent::Text(content.clone()));
        }
        Inline::SoftBreak { .. } => {
            queue.push_back(OwnedEvent::SoftBreak);
        }
        Inline::HardBreak { .. } => {
            queue.push_back(OwnedEvent::HardBreak);
        }
        Inline::Emphasis { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartEmphasis {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndEmphasis);
        }
        Inline::Strong { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartStrong {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndStrong);
        }
        Inline::Delete { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartDelete {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndDelete);
        }
        Inline::Insert { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartInsert {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndInsert);
        }
        Inline::Highlight { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartHighlight {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndHighlight);
        }
        Inline::Subscript { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartSubscript {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndSubscript);
        }
        Inline::Superscript { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartSuperscript {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndSuperscript);
        }
        Inline::Verbatim { content, attr, .. } => {
            queue.push_back(OwnedEvent::Verbatim {
                content: content.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
        }
        Inline::MathInline { content, .. } => {
            queue.push_back(OwnedEvent::MathInline(content.clone()));
        }
        Inline::MathDisplay { content, .. } => {
            queue.push_back(OwnedEvent::MathDisplay(content.clone()));
        }
        Inline::RawInline { format, content, .. } => {
            queue.push_back(OwnedEvent::RawInline {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Inline::Link { inlines, url, title, attr, .. } => {
            queue.push_back(OwnedEvent::StartLink {
                url: url.clone(),
                title: title.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndLink);
        }
        Inline::Image { inlines, url, title, attr, .. } => {
            queue.push_back(OwnedEvent::StartImage {
                url: url.clone(),
                title: title.clone(),
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndImage);
        }
        Inline::Span { inlines, attr, .. } => {
            queue.push_back(OwnedEvent::StartSpan {
                id: attr.id.clone(),
                classes: attr.classes.clone(),
                kv: attr.kv.clone(),
            });
            collect_inlines_events(inlines, queue);
            queue.push_back(OwnedEvent::EndSpan);
        }
        Inline::FootnoteRef { label, .. } => {
            queue.push_back(OwnedEvent::FootnoteRef(label.clone()));
        }
        Inline::Symbol { name, .. } => {
            queue.push_back(OwnedEvent::Symbol(name.clone()));
        }
        Inline::Autolink { url, is_email, .. } => {
            queue.push_back(OwnedEvent::Autolink { url: url.clone(), is_email: *is_email });
        }
    }
}

impl Iterator for DocEventIterator {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let _ = &self.doc; // keep doc alive
        let _ = self.pos;
        self.queue.pop_front()
    }
}

/// A public event iterator that yields `OwnedEvent` items (not borrowed).
/// The `events()` function in lib.rs returns this.
pub struct EventIter {
    inner: DocEventIterator,
}

impl EventIter {
    pub(crate) fn new(input: &str) -> Self {
        EventIter { inner: DocEventIterator::new(input) }
    }
}

impl Iterator for EventIter {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

// Re-export OwnedEvent as the public event type since true borrowing
// across Iterator::next is not possible without unsafe or GATs.
pub use OwnedEvent as EventOwned;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = EventIter::new("# Hello").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = EventIter::new("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph { .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = EventIter::new("```rust\ncode\n```").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, OwnedEvent::StartCodeBlock { language: Some(l), .. } if l == "rust")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndCodeBlock)));
    }
}
