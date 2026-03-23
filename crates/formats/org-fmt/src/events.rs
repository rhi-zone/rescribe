//! Streaming event iterator over a parsed `OrgDoc`.

use std::collections::VecDeque;

use crate::ast::*;

/// An owned event from an Org-mode document (no borrowed data).
#[derive(Debug)]
pub enum OwnedEvent {
    // ── Block events ──────────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading {
        level: usize,
        todo: Option<String>,
        priority: Option<String>,
        tags: Vec<String>,
    },
    EndHeading,
    StartBlockquote,
    EndBlockquote,
    StartList {
        ordered: bool,
        start: Option<u64>,
    },
    EndList,
    StartListItem {
        checkbox: Option<CheckboxState>,
    },
    EndListItem,
    /// Leaf: a fenced code block.
    CodeBlock {
        language: Option<String>,
        header_args: Option<String>,
        name: Option<String>,
        content: String,
    },
    /// Leaf: a raw export block.
    RawBlock {
        format: String,
        content: String,
    },
    /// Leaf: a horizontal rule (`-----`).
    HorizontalRule,
    StartTable,
    EndTable,
    StartTableRow {
        is_header: bool,
    },
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartDiv,
    EndDiv,
    StartFigure,
    EndFigure,
    StartCaption,
    EndCaption,
    /// Unknown block kind (preserved for diagnostics).
    UnknownBlock {
        kind: String,
    },

    // ── Inline events ─────────────────────────────────────────────────────────
    Text(String),
    SoftBreak,
    LineBreak,
    StartBold,
    EndBold,
    StartItalic,
    EndItalic,
    StartUnderline,
    EndUnderline,
    StartStrikethrough,
    EndStrikethrough,
    StartSuperscript,
    EndSuperscript,
    StartSubscript,
    EndSubscript,
    /// Leaf: inline verbatim/code span.
    InlineCode(String),
    StartLink {
        url: String,
    },
    EndLink,
    /// Leaf: standalone image link (no children).
    InlineImage {
        url: String,
    },
    /// Leaf: footnote reference.
    FootnoteRef {
        label: String,
    },
    StartFootnoteDefinition {
        label: String,
    },
    EndFootnoteDefinition,
    /// Leaf: inline math `$...$`.
    MathInline {
        source: String,
    },
    /// Leaf: Org timestamp `<...>` or `[...]`.
    Timestamp {
        active: bool,
        value: String,
    },
    /// Leaf: export snippet `@@backend:value@@`.
    ExportSnippet {
        backend: String,
        value: String,
    },
}

// ── Internal iterator ─────────────────────────────────────────────────────────

struct DocEventIterator {
    // The doc is kept alive so that any future borrowing extension is safe.
    _doc: Box<OrgDoc>,
    queue: VecDeque<OwnedEvent>,
}

impl DocEventIterator {
    fn new(input: &str) -> Self {
        let (doc, _diags) = crate::parse::parse(input);
        let doc = Box::new(doc);
        let mut queue = VecDeque::new();
        collect_blocks_events(&doc.blocks, &mut queue);
        DocEventIterator { _doc: doc, queue }
    }
}

impl Iterator for DocEventIterator {
    type Item = OwnedEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front()
    }
}

// ── Collection helpers ────────────────────────────────────────────────────────

fn collect_blocks_events(blocks: &[Block], q: &mut VecDeque<OwnedEvent>) {
    for block in blocks {
        collect_block_events(block, q);
    }
}

fn collect_block_events(block: &Block, q: &mut VecDeque<OwnedEvent>) {
    match block {
        Block::Paragraph { inlines, .. } => {
            q.push_back(OwnedEvent::StartParagraph);
            collect_inlines_events(inlines, q);
            q.push_back(OwnedEvent::EndParagraph);
        }
        Block::Heading { level, todo, priority, tags, inlines, .. } => {
            q.push_back(OwnedEvent::StartHeading {
                level: *level,
                todo: todo.clone(),
                priority: priority.clone(),
                tags: tags.clone(),
            });
            collect_inlines_events(inlines, q);
            q.push_back(OwnedEvent::EndHeading);
        }
        Block::CodeBlock { language, header_args, name, content, .. } => {
            q.push_back(OwnedEvent::CodeBlock {
                language: language.clone(),
                header_args: header_args.clone(),
                name: name.clone(),
                content: content.clone(),
            });
        }
        Block::Blockquote { children, .. } => {
            q.push_back(OwnedEvent::StartBlockquote);
            collect_blocks_events(children, q);
            q.push_back(OwnedEvent::EndBlockquote);
        }
        Block::List { ordered, start, items, .. } => {
            q.push_back(OwnedEvent::StartList { ordered: *ordered, start: *start });
            for item in items {
                q.push_back(OwnedEvent::StartListItem { checkbox: item.checkbox });
                for content in &item.children {
                    match content {
                        ListItemContent::Inline(inlines) => {
                            collect_inlines_events(inlines, q);
                        }
                        ListItemContent::Block(block) => {
                            collect_block_events(block, q);
                        }
                    }
                }
                q.push_back(OwnedEvent::EndListItem);
            }
            q.push_back(OwnedEvent::EndList);
        }
        Block::Table { rows, .. } => {
            q.push_back(OwnedEvent::StartTable);
            for row in rows {
                q.push_back(OwnedEvent::StartTableRow { is_header: row.is_header });
                for cell in &row.cells {
                    q.push_back(OwnedEvent::StartTableCell);
                    collect_inlines_events(cell, q);
                    q.push_back(OwnedEvent::EndTableCell);
                }
                q.push_back(OwnedEvent::EndTableRow);
            }
            q.push_back(OwnedEvent::EndTable);
        }
        Block::HorizontalRule { .. } => {
            q.push_back(OwnedEvent::HorizontalRule);
        }
        Block::DefinitionList { items, .. } => {
            q.push_back(OwnedEvent::StartDefinitionList);
            for item in items {
                q.push_back(OwnedEvent::StartDefinitionTerm);
                collect_inlines_events(&item.term, q);
                q.push_back(OwnedEvent::EndDefinitionTerm);
                q.push_back(OwnedEvent::StartDefinitionDesc);
                collect_inlines_events(&item.desc, q);
                q.push_back(OwnedEvent::EndDefinitionDesc);
            }
            q.push_back(OwnedEvent::EndDefinitionList);
        }
        Block::Div { inlines, .. } => {
            q.push_back(OwnedEvent::StartDiv);
            collect_inlines_events(inlines, q);
            q.push_back(OwnedEvent::EndDiv);
        }
        Block::RawBlock { format, content, .. } => {
            q.push_back(OwnedEvent::RawBlock { format: format.clone(), content: content.clone() });
        }
        Block::Figure { children, .. } => {
            q.push_back(OwnedEvent::StartFigure);
            collect_blocks_events(children, q);
            q.push_back(OwnedEvent::EndFigure);
        }
        Block::Caption { inlines, .. } => {
            q.push_back(OwnedEvent::StartCaption);
            collect_inlines_events(inlines, q);
            q.push_back(OwnedEvent::EndCaption);
        }
        Block::Unknown { kind, .. } => {
            q.push_back(OwnedEvent::UnknownBlock { kind: kind.clone() });
        }
    }
}

fn collect_inlines_events(inlines: &[Inline], q: &mut VecDeque<OwnedEvent>) {
    for inline in inlines {
        collect_inline_events(inline, q);
    }
}

fn collect_inline_events(inline: &Inline, q: &mut VecDeque<OwnedEvent>) {
    match inline {
        Inline::Text { text, .. } => {
            q.push_back(OwnedEvent::Text(text.clone()));
        }
        Inline::SoftBreak { .. } => {
            q.push_back(OwnedEvent::SoftBreak);
        }
        Inline::LineBreak { .. } => {
            q.push_back(OwnedEvent::LineBreak);
        }
        Inline::Bold(children, _) => {
            q.push_back(OwnedEvent::StartBold);
            collect_inlines_events(children, q);
            q.push_back(OwnedEvent::EndBold);
        }
        Inline::Italic(children, _) => {
            q.push_back(OwnedEvent::StartItalic);
            collect_inlines_events(children, q);
            q.push_back(OwnedEvent::EndItalic);
        }
        Inline::Underline(children, _) => {
            q.push_back(OwnedEvent::StartUnderline);
            collect_inlines_events(children, q);
            q.push_back(OwnedEvent::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            q.push_back(OwnedEvent::StartStrikethrough);
            collect_inlines_events(children, q);
            q.push_back(OwnedEvent::EndStrikethrough);
        }
        Inline::Superscript(children, _) => {
            q.push_back(OwnedEvent::StartSuperscript);
            collect_inlines_events(children, q);
            q.push_back(OwnedEvent::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            q.push_back(OwnedEvent::StartSubscript);
            collect_inlines_events(children, q);
            q.push_back(OwnedEvent::EndSubscript);
        }
        Inline::Code(content, _) => {
            q.push_back(OwnedEvent::InlineCode(content.clone()));
        }
        Inline::Link { url, children, .. } => {
            q.push_back(OwnedEvent::StartLink { url: url.clone() });
            collect_inlines_events(children, q);
            q.push_back(OwnedEvent::EndLink);
        }
        Inline::Image { url, .. } => {
            q.push_back(OwnedEvent::InlineImage { url: url.clone() });
        }
        Inline::FootnoteRef { label, .. } => {
            q.push_back(OwnedEvent::FootnoteRef { label: label.clone() });
        }
        Inline::FootnoteDefinition { label, children, .. } => {
            q.push_back(OwnedEvent::StartFootnoteDefinition { label: label.clone() });
            collect_inlines_events(children, q);
            q.push_back(OwnedEvent::EndFootnoteDefinition);
        }
        Inline::MathInline { source, .. } => {
            q.push_back(OwnedEvent::MathInline { source: source.clone() });
        }
        Inline::Timestamp { active, value, .. } => {
            q.push_back(OwnedEvent::Timestamp { active: *active, value: value.clone() });
        }
        Inline::ExportSnippet { backend, value, .. } => {
            q.push_back(OwnedEvent::ExportSnippet {
                backend: backend.clone(),
                value: value.clone(),
            });
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// A public streaming event iterator over an Org-mode document.
///
/// Constructed via [`events`].  Yields [`OwnedEvent`] items.
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

/// Parse `input` and return a streaming iterator of [`OwnedEvent`] items.
pub fn events(input: &str) -> EventIter {
    EventIter::new(input)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_heading() {
        let evs: Vec<_> = events("* Hello").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndHeading)));
    }

    #[test]
    fn test_events_paragraph() {
        let evs: Vec<_> = events("Hello world").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Text(t) if t == "Hello world")));
    }

    #[test]
    fn test_events_code_block() {
        let evs: Vec<_> = events("#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::CodeBlock { language: Some(l), .. } if l == "rust")));
    }

    #[test]
    fn test_events_list() {
        let evs: Vec<_> = events("- item 1\n- item 2").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartList { ordered: false, .. })));
        assert_eq!(evs.iter().filter(|e| matches!(e, OwnedEvent::StartListItem { .. })).count(), 2);
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndList)));
    }

    #[test]
    fn test_events_table() {
        let evs: Vec<_> = events("| Name | Age |\n|------+-----|\n| Alice | 30 |").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTable)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartTableRow { is_header: true })));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndTable)));
    }

    #[test]
    fn test_events_horizontal_rule() {
        let evs: Vec<_> = events("-----").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::HorizontalRule)));
    }

    #[test]
    fn test_events_bold_italic() {
        let evs: Vec<_> = events("*bold* /italic/").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBold)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartItalic)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndItalic)));
    }

    #[test]
    fn test_events_link() {
        let evs: Vec<_> = events("[[https://example.com][click here]]").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartLink { url } if url == "https://example.com")));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndLink)));
    }

    #[test]
    fn test_events_inline_code() {
        let evs: Vec<_> = events("Some =verbatim= text").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::InlineCode(s) if s == "verbatim")));
    }

    #[test]
    fn test_events_footnote_ref() {
        let evs: Vec<_> = events("See [fn:1].").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::FootnoteRef { label } if label == "1")));
    }

    #[test]
    fn test_events_math_inline() {
        let evs: Vec<_> = events("Solve $x^2 + y^2 = r^2$.").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::MathInline { .. })));
    }

    #[test]
    fn test_events_blockquote() {
        let evs: Vec<_> = events("#+BEGIN_QUOTE\nquoted\n#+END_QUOTE").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartBlockquote)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndBlockquote)));
    }

    #[test]
    fn test_events_definition_list() {
        let evs: Vec<_> = events("- Term :: Description").collect();
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionList)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionTerm)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartDefinitionDesc)));
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::EndDefinitionList)));
    }
}
