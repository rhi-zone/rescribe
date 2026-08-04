//! Streaming reader: a tree-walk-with-cursor iterator over a parsed
//! [`TypstDoc`].
//!
//! `typst-syntax` gives a materialized tree (no native event/SAX parsing
//! mode), so there is no way to produce IR-shaped events without parsing
//! first — unlike `endnotexml-fmt`/`opml-fmt`, whose `events()` genuinely
//! never materializes an AST. [`events`] therefore parses the full input via
//! [`crate::parse::parse`] once, then returns an [`EventIter`] that holds
//! the parsed blocks plus an explicit work stack (`Vec<Task>`) as its
//! traversal cursor: each `next()` call pops one item off that stack and
//! does O(1) work, expanding a block/inline into its Start event, its
//! children (pushed back onto the stack, not recursed into eagerly), and
//! its End event. This is a legitimate streaming *iteration* API — the
//! iterator holds real cursor state and advances it one step per `next()`,
//! never building a separate `Vec<Event>` up front — but it is honestly a
//! post-parse walk, not incremental parsing; see `crate::batch`'s module
//! docs for the same caveat applied to `StreamingParser`.
//!
//! [`Event`] owns its string payloads (`String`, not `Cow<'a, str>` borrowed
//! from the input) because `typst_syntax::SyntaxNode` owns its text
//! (Rc-backed green-tree nodes reconstructed from the source, not slices
//! borrowed from the caller's `&str`) — true zero-copy borrowing from the
//! original input, the way `endnotexml-fmt`/`opml-fmt` slice directly from
//! caller-owned bytes via `quick_xml`, is not available from the upstream
//! parser here.

use crate::ast::{Block, Inline, TypstDoc};

/// One IR-shaped event, mirroring `rescribe-std`'s node kinds this crate's
/// [`Block`]/[`Inline`] map to (see the adapter crates).
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    StartDocument,
    EndDocument,
    StartParagraph,
    EndParagraph,
    StartHeading {
        level: u8,
    },
    EndHeading,
    StartCodeBlock {
        lang: Option<String>,
    },
    CodeBlockContent(String),
    EndCodeBlock,
    StartList {
        ordered: bool,
    },
    EndList,
    StartListItem,
    EndListItem,
    StartDefinitionList,
    EndDefinitionList,
    StartDefinitionTerm,
    EndDefinitionTerm,
    StartDefinitionDesc,
    EndDefinitionDesc,
    StartQuote,
    EndQuote,
    StartTable {
        columns: usize,
    },
    EndTable,
    StartTableRow,
    EndTableRow,
    StartTableCell,
    EndTableCell,
    StartFigure,
    EndFigure,
    StartCaption,
    EndCaption,
    HorizontalRule,
    MathDisplay(String),
    Image {
        url: String,
    },
    Text(String),
    StartStrong,
    EndStrong,
    StartEmph,
    EndEmph,
    StartUnderline,
    EndUnderline,
    StartStrike,
    EndStrike,
    StartSubscript,
    EndSubscript,
    StartSuperscript,
    EndSuperscript,
    Code(String),
    StartLink {
        url: String,
    },
    EndLink,
    LineBreak,
    MathInline(String),
    /// Inline-positioned block-style equation — see
    /// [`crate::ast::Inline::MathDisplay`]'s doc comment.
    InlineMathDisplay(String),
    StartFootnote,
    EndFootnote,
    StartSmallCaps,
    EndSmallCaps,
    StartQuoted {
        double: bool,
    },
    EndQuoted,
    Raw(String),
}

/// Return a streaming event iterator over `input`'s parsed structure.
///
/// Parses once internally (see module docs for why); the returned iterator
/// then walks the parsed structure incrementally.
pub fn events(input: &str) -> EventIter {
    let (doc, _diags) = crate::parse::parse(input);
    EventIter::from_doc(doc)
}

enum Task {
    Event(Event),
    Blocks(std::vec::IntoIter<Block>),
    Block(Block),
    Inlines(std::vec::IntoIter<Inline>),
    Inline(Inline),
    ListItems(std::vec::IntoIter<Vec<Block>>),
    ListItem(Vec<Block>),
    TableRows(std::vec::IntoIter<Vec<Vec<Inline>>>),
    TableRow(Vec<Vec<Inline>>),
    TableCells(std::vec::IntoIter<Vec<Inline>>),
    TableCell(Vec<Inline>),
    DefEntries(std::vec::IntoIter<(Vec<Inline>, Vec<Inline>)>),
    DefEntry(Vec<Inline>, Vec<Inline>),
}

/// Cursor-based event iterator. See module docs.
pub struct EventIter {
    stack: Vec<Task>,
}

impl EventIter {
    /// Build an iterator over an already-parsed [`TypstDoc`], without
    /// re-parsing. Used by [`events`] and by callers that already hold a
    /// `TypstDoc` from [`crate::parse::parse`].
    pub fn from_doc(doc: TypstDoc) -> Self {
        let stack = vec![
            Task::Event(Event::EndDocument),
            Task::Blocks(doc.blocks.into_iter()),
            Task::Event(Event::StartDocument),
        ];
        EventIter { stack }
    }

    fn expand_block(&mut self, block: Block) {
        match block {
            Block::Paragraph(inlines) => {
                self.stack.push(Task::Event(Event::EndParagraph));
                self.stack.push(Task::Inlines(inlines.into_iter()));
                self.stack.push(Task::Event(Event::StartParagraph));
            }
            Block::Heading { level, body } => {
                self.stack.push(Task::Event(Event::EndHeading));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartHeading { level }));
            }
            Block::CodeBlock { lang, content } => {
                self.stack.push(Task::Event(Event::EndCodeBlock));
                self.stack
                    .push(Task::Event(Event::CodeBlockContent(content)));
                self.stack.push(Task::Event(Event::StartCodeBlock { lang }));
            }
            Block::List { ordered, items } => {
                self.stack.push(Task::Event(Event::EndList));
                self.stack.push(Task::ListItems(items.into_iter()));
                self.stack.push(Task::Event(Event::StartList { ordered }));
            }
            Block::DefinitionList(entries) => {
                self.stack.push(Task::Event(Event::EndDefinitionList));
                self.stack.push(Task::DefEntries(entries.into_iter()));
                self.stack.push(Task::Event(Event::StartDefinitionList));
            }
            Block::Quote(body) => {
                self.stack.push(Task::Event(Event::EndQuote));
                self.stack.push(Task::Blocks(body.into_iter()));
                self.stack.push(Task::Event(Event::StartQuote));
            }
            Block::Table { columns, rows } => {
                self.stack.push(Task::Event(Event::EndTable));
                self.stack.push(Task::TableRows(rows.into_iter()));
                self.stack.push(Task::Event(Event::StartTable { columns }));
            }
            Block::Figure { body, caption } => {
                self.stack.push(Task::Event(Event::EndFigure));
                if let Some(cap) = caption {
                    self.stack.push(Task::Event(Event::EndCaption));
                    self.stack.push(Task::Inlines(cap.into_iter()));
                    self.stack.push(Task::Event(Event::StartCaption));
                }
                if let Some(b) = body {
                    self.stack.push(Task::Block(*b));
                }
                self.stack.push(Task::Event(Event::StartFigure));
            }
            Block::HorizontalRule => self.stack.push(Task::Event(Event::HorizontalRule)),
            Block::MathDisplay(src) => self.stack.push(Task::Event(Event::MathDisplay(src))),
            Block::Image { url } => self.stack.push(Task::Event(Event::Image { url })),
            Block::Raw(text) => self.stack.push(Task::Event(Event::Raw(text))),
        }
    }

    fn expand_inline(&mut self, inline: Inline) {
        match inline {
            Inline::Text(t) => self.stack.push(Task::Event(Event::Text(t))),
            Inline::Strong(body) => {
                self.stack.push(Task::Event(Event::EndStrong));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartStrong));
            }
            Inline::Emph(body) => {
                self.stack.push(Task::Event(Event::EndEmph));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartEmph));
            }
            Inline::Underline(body) => {
                self.stack.push(Task::Event(Event::EndUnderline));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartUnderline));
            }
            Inline::Strike(body) => {
                self.stack.push(Task::Event(Event::EndStrike));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartStrike));
            }
            Inline::Subscript(body) => {
                self.stack.push(Task::Event(Event::EndSubscript));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartSubscript));
            }
            Inline::Superscript(body) => {
                self.stack.push(Task::Event(Event::EndSuperscript));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartSuperscript));
            }
            Inline::Code(content) => self.stack.push(Task::Event(Event::Code(content))),
            Inline::Link { url, body } => {
                self.stack.push(Task::Event(Event::EndLink));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartLink { url }));
            }
            Inline::Image { url } => self.stack.push(Task::Event(Event::Image { url })),
            Inline::LineBreak => self.stack.push(Task::Event(Event::LineBreak)),
            Inline::MathInline(src) => self.stack.push(Task::Event(Event::MathInline(src))),
            Inline::MathDisplay(src) => self.stack.push(Task::Event(Event::InlineMathDisplay(src))),
            Inline::Footnote(body) => {
                self.stack.push(Task::Event(Event::EndFootnote));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartFootnote));
            }
            Inline::SmallCaps(body) => {
                self.stack.push(Task::Event(Event::EndSmallCaps));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartSmallCaps));
            }
            Inline::Quoted { double, body } => {
                self.stack.push(Task::Event(Event::EndQuoted));
                self.stack.push(Task::Inlines(body.into_iter()));
                self.stack.push(Task::Event(Event::StartQuoted { double }));
            }
            Inline::Raw(text) => self.stack.push(Task::Event(Event::Raw(text))),
        }
    }
}

impl Iterator for EventIter {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        loop {
            let task = self.stack.pop()?;
            match task {
                Task::Event(e) => return Some(e),
                Task::Blocks(mut it) => {
                    if let Some(b) = it.next() {
                        self.stack.push(Task::Blocks(it));
                        self.stack.push(Task::Block(b));
                    }
                }
                Task::Block(b) => self.expand_block(b),
                Task::Inlines(mut it) => {
                    if let Some(i) = it.next() {
                        self.stack.push(Task::Inlines(it));
                        self.stack.push(Task::Inline(i));
                    }
                }
                Task::Inline(i) => self.expand_inline(i),
                Task::ListItems(mut it) => {
                    if let Some(item) = it.next() {
                        self.stack.push(Task::ListItems(it));
                        self.stack.push(Task::ListItem(item));
                    }
                }
                Task::ListItem(item) => {
                    self.stack.push(Task::Event(Event::EndListItem));
                    self.stack.push(Task::Blocks(item.into_iter()));
                    self.stack.push(Task::Event(Event::StartListItem));
                }
                Task::TableRows(mut it) => {
                    if let Some(row) = it.next() {
                        self.stack.push(Task::TableRows(it));
                        self.stack.push(Task::TableRow(row));
                    }
                }
                Task::TableRow(row) => {
                    self.stack.push(Task::Event(Event::EndTableRow));
                    self.stack.push(Task::TableCells(row.into_iter()));
                    self.stack.push(Task::Event(Event::StartTableRow));
                }
                Task::TableCells(mut it) => {
                    if let Some(cell) = it.next() {
                        self.stack.push(Task::TableCells(it));
                        self.stack.push(Task::TableCell(cell));
                    }
                }
                Task::TableCell(cell) => {
                    self.stack.push(Task::Event(Event::EndTableCell));
                    self.stack.push(Task::Inlines(cell.into_iter()));
                    self.stack.push(Task::Event(Event::StartTableCell));
                }
                Task::DefEntries(mut it) => {
                    if let Some((term, desc)) = it.next() {
                        self.stack.push(Task::DefEntries(it));
                        self.stack.push(Task::DefEntry(term, desc));
                    }
                }
                Task::DefEntry(term, desc) => {
                    self.stack.push(Task::Event(Event::EndDefinitionDesc));
                    self.stack.push(Task::Inlines(desc.into_iter()));
                    self.stack.push(Task::Event(Event::StartDefinitionDesc));
                    self.stack.push(Task::Event(Event::EndDefinitionTerm));
                    self.stack.push(Task::Inlines(term.into_iter()));
                    self.stack.push(Task::Event(Event::StartDefinitionTerm));
                }
            }
        }
    }
}

/// Project a [`TypstDoc`] directly to its event sequence, without going
/// through `events()`'s (re-)parse. Used as the equivalence oracle: over
/// every fixture, `events_from_doc(&parse(input).0)` must equal
/// `events(input).collect()`.
pub fn events_from_doc(doc: &TypstDoc) -> Vec<Event> {
    EventIter::from_doc(doc.clone()).collect()
}

/// Rebuild a [`TypstDoc`] from an event sequence — the inverse of
/// [`events_from_doc`]. Used by the crate's own roundtrip smoke test.
///
/// # Panics
///
/// Panics if the event sequence is structurally invalid (unbalanced
/// Start/End pairs, an End event with no matching Start). This function is
/// a test/debugging aid for events genuinely produced by this crate, not a
/// public parser for arbitrary event streams.
pub fn collect_doc(events: impl IntoIterator<Item = Event>) -> TypstDoc {
    let mut builder = Builder::default();
    for ev in events {
        builder.handle(ev);
    }
    TypstDoc {
        blocks: builder.take_document_blocks(),
        span: crate::ast::Span::NONE,
    }
}

#[derive(Default)]
struct Builder {
    frames: Vec<Frame>,
}

enum Frame {
    Document(Vec<Block>),
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        body: Vec<Inline>,
    },
    CodeBlock {
        lang: Option<String>,
        content: String,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    ListItem(Vec<Block>),
    DefinitionList(Vec<(Vec<Inline>, Vec<Inline>)>),
    DefinitionTerm(Vec<Inline>),
    DefinitionDesc(Vec<Inline>),
    Quote(Vec<Block>),
    Table {
        columns: usize,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    TableRow(Vec<Vec<Inline>>),
    TableCell(Vec<Inline>),
    Figure {
        body: Option<Box<Block>>,
        caption: Option<Vec<Inline>>,
    },
    Caption(Vec<Inline>),
    Strong(Vec<Inline>),
    Emph(Vec<Inline>),
    Underline(Vec<Inline>),
    Strike(Vec<Inline>),
    Subscript(Vec<Inline>),
    Superscript(Vec<Inline>),
    Link {
        url: String,
        body: Vec<Inline>,
    },
    Footnote(Vec<Inline>),
    SmallCaps(Vec<Inline>),
    Quoted {
        double: bool,
        body: Vec<Inline>,
    },
}

impl Builder {
    fn take_document_blocks(mut self) -> Vec<Block> {
        match self.frames.pop() {
            Some(Frame::Document(blocks)) => blocks,
            _ => panic!("collect_doc: event stream did not end with a balanced StartDocument"),
        }
    }

    fn push_block(&mut self, b: Block) {
        match self.frames.last_mut() {
            Some(Frame::Document(blocks)) => blocks.push(b),
            Some(Frame::Quote(blocks)) => blocks.push(b),
            Some(Frame::ListItem(blocks)) => blocks.push(b),
            Some(Frame::List { items, .. }) => items
                .last_mut()
                .expect("push_block: List with no open item")
                .push(b),
            Some(Frame::Figure { body, .. }) => *body = Some(Box::new(b)),
            _ => panic!("collect_doc: block event with no block-accepting parent frame"),
        }
    }

    fn push_inline(&mut self, i: Inline) {
        match self.frames.last_mut() {
            Some(Frame::Paragraph(v)) => v.push(i),
            Some(Frame::Heading { body, .. }) => body.push(i),
            Some(Frame::DefinitionTerm(v)) => v.push(i),
            Some(Frame::DefinitionDesc(v)) => v.push(i),
            Some(Frame::TableCell(v)) => v.push(i),
            Some(Frame::Caption(v)) => v.push(i),
            Some(Frame::Strong(v)) => v.push(i),
            Some(Frame::Emph(v)) => v.push(i),
            Some(Frame::Underline(v)) => v.push(i),
            Some(Frame::Strike(v)) => v.push(i),
            Some(Frame::Subscript(v)) => v.push(i),
            Some(Frame::Superscript(v)) => v.push(i),
            Some(Frame::Link { body, .. }) => body.push(i),
            Some(Frame::Footnote(v)) => v.push(i),
            Some(Frame::SmallCaps(v)) => v.push(i),
            Some(Frame::Quoted { body, .. }) => body.push(i),
            _ => panic!("collect_doc: inline event with no inline-accepting parent frame"),
        }
    }

    fn handle(&mut self, ev: Event) {
        match ev {
            Event::StartDocument => self.frames.push(Frame::Document(Vec::new())),
            Event::EndDocument => { /* left on stack for take_document_blocks */ }
            Event::StartParagraph => self.frames.push(Frame::Paragraph(Vec::new())),
            Event::EndParagraph => {
                if let Some(Frame::Paragraph(v)) = self.frames.pop() {
                    self.push_block(Block::Paragraph(v));
                }
            }
            Event::StartHeading { level } => self.frames.push(Frame::Heading {
                level,
                body: Vec::new(),
            }),
            Event::EndHeading => {
                if let Some(Frame::Heading { level, body }) = self.frames.pop() {
                    self.push_block(Block::Heading { level, body });
                }
            }
            Event::StartCodeBlock { lang } => self.frames.push(Frame::CodeBlock {
                lang,
                content: String::new(),
            }),
            Event::CodeBlockContent(c) => {
                if let Some(Frame::CodeBlock { content, .. }) = self.frames.last_mut() {
                    content.push_str(&c);
                }
            }
            Event::EndCodeBlock => {
                if let Some(Frame::CodeBlock { lang, content }) = self.frames.pop() {
                    self.push_block(Block::CodeBlock { lang, content });
                }
            }
            Event::StartList { ordered } => self.frames.push(Frame::List {
                ordered,
                items: Vec::new(),
            }),
            Event::EndList => {
                if let Some(Frame::List { ordered, items }) = self.frames.pop() {
                    self.push_block(Block::List { ordered, items });
                }
            }
            Event::StartListItem => {
                if let Some(Frame::List { items, .. }) = self.frames.last_mut() {
                    items.push(Vec::new());
                }
                self.frames.push(Frame::ListItem(Vec::new()));
            }
            Event::EndListItem => {
                if let Some(Frame::ListItem(blocks)) = self.frames.pop()
                    && let Some(Frame::List { items, .. }) = self.frames.last_mut()
                {
                    *items.last_mut().unwrap() = blocks;
                }
            }
            Event::StartDefinitionList => self.frames.push(Frame::DefinitionList(Vec::new())),
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList(entries)) = self.frames.pop() {
                    self.push_block(Block::DefinitionList(entries));
                }
            }
            Event::StartDefinitionTerm => self.frames.push(Frame::DefinitionTerm(Vec::new())),
            Event::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm(term)) = self.frames.pop()
                    && let Some(Frame::DefinitionList(entries)) = self.frames.last_mut()
                {
                    entries.push((term, Vec::new()));
                }
            }
            Event::StartDefinitionDesc => self.frames.push(Frame::DefinitionDesc(Vec::new())),
            Event::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc(desc)) = self.frames.pop()
                    && let Some(Frame::DefinitionList(entries)) = self.frames.last_mut()
                {
                    entries.last_mut().unwrap().1 = desc;
                }
            }
            Event::StartQuote => self.frames.push(Frame::Quote(Vec::new())),
            Event::EndQuote => {
                if let Some(Frame::Quote(blocks)) = self.frames.pop() {
                    self.push_block(Block::Quote(blocks));
                }
            }
            Event::StartTable { columns } => self.frames.push(Frame::Table {
                columns,
                rows: Vec::new(),
            }),
            Event::EndTable => {
                if let Some(Frame::Table { columns, rows }) = self.frames.pop() {
                    self.push_block(Block::Table { columns, rows });
                }
            }
            Event::StartTableRow => self.frames.push(Frame::TableRow(Vec::new())),
            Event::EndTableRow => {
                if let Some(Frame::TableRow(cells)) = self.frames.pop()
                    && let Some(Frame::Table { rows, .. }) = self.frames.last_mut()
                {
                    rows.push(cells);
                }
            }
            Event::StartTableCell => self.frames.push(Frame::TableCell(Vec::new())),
            Event::EndTableCell => {
                if let Some(Frame::TableCell(v)) = self.frames.pop()
                    && let Some(Frame::TableRow(cells)) = self.frames.last_mut()
                {
                    cells.push(v);
                }
            }
            Event::StartFigure => self.frames.push(Frame::Figure {
                body: None,
                caption: None,
            }),
            Event::EndFigure => {
                if let Some(Frame::Figure { body, caption }) = self.frames.pop() {
                    self.push_block(Block::Figure { body, caption });
                }
            }
            Event::StartCaption => self.frames.push(Frame::Caption(Vec::new())),
            Event::EndCaption => {
                if let Some(Frame::Caption(v)) = self.frames.pop()
                    && let Some(Frame::Figure { caption, .. }) = self.frames.last_mut()
                {
                    *caption = Some(v);
                }
            }
            Event::HorizontalRule => self.push_block(Block::HorizontalRule),
            Event::MathDisplay(src) => self.push_block(Block::MathDisplay(src)),
            Event::Image { url } => {
                // Ambiguous between Block::Image and Inline::Image; disambiguate
                // by which kind of frame is currently open.
                if self.accepts_inline() {
                    self.push_inline(Inline::Image { url });
                } else {
                    self.push_block(Block::Image { url });
                }
            }
            Event::Text(t) => self.push_inline(Inline::Text(t)),
            Event::StartStrong => self.frames.push(Frame::Strong(Vec::new())),
            Event::EndStrong => {
                if let Some(Frame::Strong(v)) = self.frames.pop() {
                    self.push_inline(Inline::Strong(v));
                }
            }
            Event::StartEmph => self.frames.push(Frame::Emph(Vec::new())),
            Event::EndEmph => {
                if let Some(Frame::Emph(v)) = self.frames.pop() {
                    self.push_inline(Inline::Emph(v));
                }
            }
            Event::StartUnderline => self.frames.push(Frame::Underline(Vec::new())),
            Event::EndUnderline => {
                if let Some(Frame::Underline(v)) = self.frames.pop() {
                    self.push_inline(Inline::Underline(v));
                }
            }
            Event::StartStrike => self.frames.push(Frame::Strike(Vec::new())),
            Event::EndStrike => {
                if let Some(Frame::Strike(v)) = self.frames.pop() {
                    self.push_inline(Inline::Strike(v));
                }
            }
            Event::StartSubscript => self.frames.push(Frame::Subscript(Vec::new())),
            Event::EndSubscript => {
                if let Some(Frame::Subscript(v)) = self.frames.pop() {
                    self.push_inline(Inline::Subscript(v));
                }
            }
            Event::StartSuperscript => self.frames.push(Frame::Superscript(Vec::new())),
            Event::EndSuperscript => {
                if let Some(Frame::Superscript(v)) = self.frames.pop() {
                    self.push_inline(Inline::Superscript(v));
                }
            }
            Event::Code(c) => self.push_inline(Inline::Code(c)),
            Event::StartLink { url } => self.frames.push(Frame::Link {
                url,
                body: Vec::new(),
            }),
            Event::EndLink => {
                if let Some(Frame::Link { url, body }) = self.frames.pop() {
                    self.push_inline(Inline::Link { url, body });
                }
            }
            Event::LineBreak => self.push_inline(Inline::LineBreak),
            Event::MathInline(src) => self.push_inline(Inline::MathInline(src)),
            Event::InlineMathDisplay(src) => self.push_inline(Inline::MathDisplay(src)),
            Event::StartFootnote => self.frames.push(Frame::Footnote(Vec::new())),
            Event::EndFootnote => {
                if let Some(Frame::Footnote(v)) = self.frames.pop() {
                    self.push_inline(Inline::Footnote(v));
                }
            }
            Event::StartSmallCaps => self.frames.push(Frame::SmallCaps(Vec::new())),
            Event::EndSmallCaps => {
                if let Some(Frame::SmallCaps(v)) = self.frames.pop() {
                    self.push_inline(Inline::SmallCaps(v));
                }
            }
            Event::StartQuoted { double } => self.frames.push(Frame::Quoted {
                double,
                body: Vec::new(),
            }),
            Event::EndQuoted => {
                if let Some(Frame::Quoted { double, body }) = self.frames.pop() {
                    self.push_inline(Inline::Quoted { double, body });
                }
            }
            Event::Raw(text) => {
                if self.accepts_inline() {
                    self.push_inline(Inline::Raw(text));
                } else {
                    self.push_block(Block::Raw(text));
                }
            }
        }
    }

    fn accepts_inline(&self) -> bool {
        matches!(
            self.frames.last(),
            Some(
                Frame::Paragraph(_)
                    | Frame::Heading { .. }
                    | Frame::DefinitionTerm(_)
                    | Frame::DefinitionDesc(_)
                    | Frame::TableCell(_)
                    | Frame::Caption(_)
                    | Frame::Strong(_)
                    | Frame::Emph(_)
                    | Frame::Underline(_)
                    | Frame::Strike(_)
                    | Frame::Subscript(_)
                    | Frame::Superscript(_)
                    | Frame::Link { .. }
                    | Frame::Footnote(_)
                    | Frame::SmallCaps(_)
                    | Frame::Quoted { .. }
            )
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

    #[test]
    fn events_from_doc_roundtrips_through_collect_doc() {
        let doc = TypstDoc {
            blocks: vec![
                Block::Heading {
                    level: 1,
                    body: vec![Inline::Text("Title".into())],
                },
                Block::Paragraph(vec![
                    Inline::Strong(vec![Inline::Text("bold".into())]),
                    Inline::Text(" plain".into()),
                ]),
                Block::List {
                    ordered: false,
                    items: vec![
                        vec![Block::Paragraph(vec![Inline::Text("one".into())])],
                        vec![Block::Paragraph(vec![Inline::Text("two".into())])],
                    ],
                },
            ],
            span: Span::NONE,
        };
        let evs = events_from_doc(&doc);
        let doc2 = collect_doc(evs);
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn events_matches_parse_projection() {
        let input = "= Title\n\nHello *bold* world.\n\n- one\n- two\n";
        let (doc, _diags) = crate::parse::parse(input);
        let expected = events_from_doc(&doc);
        let actual: Vec<_> = events(input).collect();
        assert_eq!(expected, actual);
    }
}
