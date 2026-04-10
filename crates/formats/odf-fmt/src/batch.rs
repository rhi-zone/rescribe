//! Chunk-driven (batch) ODF parser and streaming writer.
//!
//! # Memory model
//!
//! ODF documents are ZIP archives. A ZIP's central directory lives at the end
//! of the file, so true incremental parsing is not possible. [`BatchParser`]
//! accumulates chunks until [`finish`](BatchParser::finish) is called, then
//! parses the complete buffer. Memory usage is O(full input).
//!
//! [`Writer`] accepts [`OdfEvent`] values and reconstructs an [`OdfDocument`]
//! AST internally; [`finish`](Writer::finish) emits the ZIP. Because ODF is
//! ZIP-based, bytes cannot be flushed incrementally — all output is produced
//! on `finish`. This limitation is documented and inherent to the format.
//!
//! # Example — batch parse
//! ```no_run
//! use odf_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"...chunk 1...");
//! p.feed(b"...chunk 2...");
//! let doc = p.finish().unwrap().value;
//! ```
//!
//! # Example — streaming writer
//! ```no_run
//! use odf_fmt::batch::Writer;
//! use odf_fmt::OdfEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OdfEvent::StartText);
//! w.write_event(OdfEvent::StartParagraph { style_name: None });
//! w.write_event(OdfEvent::Text(std::borrow::Cow::Borrowed("Hello")));
//! w.write_event(OdfEvent::EndParagraph);
//! w.write_event(OdfEvent::EndText);
//! let bytes = w.finish().unwrap();
//! ```

use crate::ast::*;
use crate::error::{Error, ParseResult};
use crate::events::OdfEvent;
use std::io::Write;

// ── BatchParser ───────────────────────────────────────────────────────────────

/// Chunk-driven ODF parser.
///
/// Feed input chunks with [`feed`](BatchParser::feed), then call
/// [`finish`](BatchParser::finish) to parse and return the full AST.
///
/// **Note:** ODF is ZIP-based. All chunks are buffered until `finish()`.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        BatchParser { buf: Vec::new() }
    }

    /// Accumulate a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Parse all accumulated input and return the document AST.
    pub fn finish(self) -> Result<ParseResult<OdfDocument>, Error> {
        crate::parser::parse(&self.buf)
    }
}

// ── Writer ────────────────────────────────────────────────────────────────────

/// Event-driven ODF writer.
///
/// Feed [`OdfEvent`] values with [`write_event`](Writer::write_event), then
/// call [`finish`](Writer::finish) to emit the ODF ZIP archive.
///
/// Internally this reconstructs an [`OdfDocument`] AST from events and then
/// calls [`emit`](crate::emit). Because ODF is ZIP-based, output bytes cannot
/// be flushed incrementally; all output is produced when `finish` is called.
pub struct Writer<W: Write> {
    sink: W,
    builder: DocBuilder,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink, builder: DocBuilder::new() }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OdfEvent<'_>) {
        self.builder.process(event);
    }

    /// Emit the complete ODF ZIP archive to the sink and return it.
    pub fn finish(mut self) -> Result<W, Error> {
        let doc = self.builder.finish();
        let bytes = crate::writer::emit(&doc)?;
        self.sink.write_all(&bytes).map_err(Error::Io)?;
        Ok(self.sink)
    }
}

// ── DocBuilder — event → AST ──────────────────────────────────────────────────

enum BuildFrame {
    Text { blocks: Vec<TextBlock> },
    Spreadsheet { sheets: Vec<Sheet>, named_ranges: Vec<NamedRange> },
    Presentation { pages: Vec<DrawPage> },
    Paragraph { style_name: Option<String>, content: Vec<Inline> },
    Heading { style_name: Option<String>, outline_level: Option<u32>, content: Vec<Inline> },
    List { style_name: Option<String>, items: Vec<ListItem> },
    ListItem { content: Vec<TextBlock> },
    TableText { name: Option<String>, style_name: Option<String>, rows: Vec<TableRow> },
    TableRow { style_name: Option<String>, cells: Vec<TableCell> },
    TableCell { style_name: Option<String>, content: Vec<TextBlock> },
    #[allow(dead_code)]
    Section { style_name: Option<String>, name: Option<String>, protected: bool, content: Vec<TextBlock> },
    InlineFrame { frame: crate::ast::Frame },
    // ODS
    Sheet { name: Option<String>, style_name: Option<String>, print: bool, columns: Vec<ColumnDef>, rows: Vec<SheetRow> },
    SheetRow { style_name: Option<String>, repeated: Option<u32>, cells: Vec<SheetCell> },
    SheetCell { cell: SheetCell },
    // ODP
    Slide { page: DrawPage },
    Shape { shape: DrawShape },
    TextBox { blocks: Vec<TextBlock> },
    Notes { notes: NotesPage },
    // Inline spans
    Span { style_name: Option<String>, content: Vec<Inline> },
    Hyperlink { href: Option<String>, title: Option<String>, style_name: Option<String>, content: Vec<Inline> },
    Note { note: Note },
    #[allow(dead_code)]
    NoteBody { content: Vec<TextBlock> },
}

struct DocBuilder {
    stack: Vec<BuildFrame>,
    doc: OdfDocument,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder { stack: vec![], doc: OdfDocument::default() }
    }

    fn finish(mut self) -> OdfDocument {
        // Drain any open frame off the top of the stack
        if let Some(frame) = self.stack.pop() {
            self.close_frame(frame);
        }
        self.doc
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OdfEvent<'_>) {
        match event {
            // ── Body open/close ───────────────────────────────────────────────
            OdfEvent::StartText => {
                self.stack.push(BuildFrame::Text { blocks: vec![] });
            }
            OdfEvent::EndText => {
                if let Some(BuildFrame::Text { blocks }) = self.stack.pop() {
                    self.doc.body = OdfBody::Text(blocks);
                }
            }
            OdfEvent::StartSpreadsheet => {
                self.stack.push(BuildFrame::Spreadsheet { sheets: vec![], named_ranges: vec![] });
            }
            OdfEvent::EndSpreadsheet => {
                if let Some(BuildFrame::Spreadsheet { sheets, named_ranges }) = self.stack.pop() {
                    self.doc.body = OdfBody::Spreadsheet(SpreadsheetBody { sheets, named_ranges });
                }
            }
            OdfEvent::StartPresentation => {
                self.stack.push(BuildFrame::Presentation { pages: vec![] });
            }
            OdfEvent::EndPresentation => {
                if let Some(BuildFrame::Presentation { pages }) = self.stack.pop() {
                    self.doc.body = OdfBody::Presentation(PresentationBody { pages });
                }
            }

            // ── Block elements ────────────────────────────────────────────────
            OdfEvent::StartParagraph { style_name } => {
                self.stack.push(BuildFrame::Paragraph { style_name: style_name.map(|s| s.into_owned()), content: vec![] });
            }
            OdfEvent::EndParagraph => {
                if let Some(BuildFrame::Paragraph { style_name, content }) = self.stack.pop() {
                    self.push_block(TextBlock::Paragraph(Paragraph { style_name, content, ..Default::default() }));
                }
            }
            OdfEvent::StartHeading { style_name, outline_level } => {
                self.stack.push(BuildFrame::Heading { style_name: style_name.map(|s| s.into_owned()), outline_level, content: vec![] });
            }
            OdfEvent::EndHeading => {
                if let Some(BuildFrame::Heading { style_name, outline_level, content }) = self.stack.pop() {
                    self.push_block(TextBlock::Heading(Heading { style_name, outline_level, content, ..Default::default() }));
                }
            }
            OdfEvent::StartList { style_name } => {
                self.stack.push(BuildFrame::List { style_name: style_name.map(|s| s.into_owned()), items: vec![] });
            }
            OdfEvent::EndList => {
                if let Some(BuildFrame::List { style_name, items }) = self.stack.pop() {
                    self.push_block(TextBlock::List(List { style_name, items, ..Default::default() }));
                }
            }
            OdfEvent::StartListItem => {
                self.stack.push(BuildFrame::ListItem { content: vec![] });
            }
            OdfEvent::EndListItem => {
                if let Some(BuildFrame::ListItem { content }) = self.stack.pop()
                    && let Some(BuildFrame::List { items, .. }) = self.stack.last_mut() {
                    items.push(ListItem { content, ..Default::default() });
                }
            }
            OdfEvent::StartTable { name, style_name } => {
                self.stack.push(BuildFrame::TableText {
                    name: name.map(|s| s.into_owned()),
                    style_name: style_name.map(|s| s.into_owned()),
                    rows: vec![],
                });
            }
            OdfEvent::EndTable => {
                if let Some(BuildFrame::TableText { name, style_name, rows }) = self.stack.pop() {
                    self.push_block(TextBlock::Table(Table { name, style_name, rows }));
                }
            }
            OdfEvent::StartRow { style_name } => {
                self.stack.push(BuildFrame::TableRow { style_name: style_name.map(|s| s.into_owned()), cells: vec![] });
            }
            OdfEvent::EndRow => {
                if let Some(BuildFrame::TableRow { style_name, cells }) = self.stack.pop()
                    && let Some(BuildFrame::TableText { rows, .. }) = self.stack.last_mut() {
                    rows.push(TableRow { style_name, cells });
                }
            }
            OdfEvent::StartCell { style_name, value_type, covered } => {
                self.stack.push(BuildFrame::TableCell {
                    style_name: style_name.map(|s| s.into_owned()),
                    content: vec![],
                });
                let _ = (value_type, covered); // available but not used in text-table cells
            }
            OdfEvent::EndCell => {
                if let Some(BuildFrame::TableCell { style_name, content }) = self.stack.pop()
                    && let Some(BuildFrame::TableRow { cells, .. }) = self.stack.last_mut() {
                    cells.push(TableCell { style_name, content, ..Default::default() });
                }
            }

            // ── ODS events ────────────────────────────────────────────────────
            OdfEvent::StartSheet { name, style_name } => {
                self.stack.push(BuildFrame::Sheet {
                    name: name.map(|s| s.into_owned()),
                    style_name: style_name.map(|s| s.into_owned()),
                    print: false,
                    columns: vec![],
                    rows: vec![],
                });
            }
            OdfEvent::EndSheet => {
                if let Some(BuildFrame::Sheet { name, style_name, print, columns, rows }) = self.stack.pop()
                    && let Some(BuildFrame::Spreadsheet { sheets, .. }) = self.stack.last_mut() {
                    sheets.push(Sheet { name, style_name, print, columns, rows });
                }
            }
            OdfEvent::StartSheetRow { style_name, repeated } => {
                self.stack.push(BuildFrame::SheetRow { style_name: style_name.map(|s| s.into_owned()), repeated, cells: vec![] });
            }
            OdfEvent::EndSheetRow => {
                if let Some(BuildFrame::SheetRow { style_name, repeated, cells }) = self.stack.pop()
                    && let Some(BuildFrame::Sheet { rows, .. }) = self.stack.last_mut() {
                    rows.push(SheetRow { style_name, repeated, cells, ..Default::default() });
                }
            }
            OdfEvent::StartSheetCell { style_name, value_type, value, formula, covered } => {
                let cell = SheetCell {
                    style_name: style_name.map(|s| s.into_owned()),
                    value_type: value_type.map(|s| s.into_owned()),
                    value: value.map(|s| s.into_owned()),
                    formula: formula.map(|s| s.into_owned()),
                    covered,
                    ..Default::default()
                };
                self.stack.push(BuildFrame::SheetCell { cell });
            }
            OdfEvent::EndSheetCell => {
                if let Some(BuildFrame::SheetCell { cell }) = self.stack.pop()
                    && let Some(BuildFrame::SheetRow { cells, .. }) = self.stack.last_mut() {
                    cells.push(cell);
                }
            }

            // ── ODP events ────────────────────────────────────────────────────
            OdfEvent::StartSlide { name, master_page_name, layout_name } => {
                let page = DrawPage {
                    name: name.map(|s| s.into_owned()),
                    master_page_name: master_page_name.map(|s| s.into_owned()),
                    layout_name: layout_name.map(|s| s.into_owned()),
                    ..Default::default()
                };
                self.stack.push(BuildFrame::Slide { page });
            }
            OdfEvent::EndSlide => {
                if let Some(BuildFrame::Slide { page }) = self.stack.pop()
                    && let Some(BuildFrame::Presentation { pages }) = self.stack.last_mut() {
                    pages.push(page);
                }
            }
            OdfEvent::StartShape { name, presentation_class, x, y, width, height } => {
                let shape = DrawShape {
                    name: name.map(|s| s.into_owned()),
                    presentation_class: presentation_class.map(|s| s.into_owned()),
                    x: x.map(|s| s.into_owned()),
                    y: y.map(|s| s.into_owned()),
                    width: width.map(|s| s.into_owned()),
                    height: height.map(|s| s.into_owned()),
                    ..Default::default()
                };
                self.stack.push(BuildFrame::Shape { shape });
            }
            OdfEvent::EndShape => {
                if let Some(BuildFrame::Shape { shape }) = self.stack.pop() {
                    self.attach_shape(shape);
                }
            }
            OdfEvent::StartTextBox => {
                self.stack.push(BuildFrame::TextBox { blocks: vec![] });
            }
            OdfEvent::EndTextBox => {
                if let Some(BuildFrame::TextBox { blocks }) = self.stack.pop()
                    && let Some(BuildFrame::Shape { shape }) = self.stack.last_mut() {
                    shape.content = DrawShapeContent::TextBox(blocks);
                }
            }
            OdfEvent::StartNotes { style_name } => {
                self.stack.push(BuildFrame::Notes { notes: NotesPage { style_name: style_name.map(|s| s.into_owned()), shapes: vec![] } });
            }
            OdfEvent::EndNotes => {
                if let Some(BuildFrame::Notes { notes }) = self.stack.pop()
                    && let Some(BuildFrame::Slide { page }) = self.stack.last_mut() {
                    page.notes = Some(Box::new(notes));
                }
            }

            // ── Inline events ─────────────────────────────────────────────────
            OdfEvent::StartSpan { style_name } => {
                self.stack.push(BuildFrame::Span { style_name: style_name.map(|s| s.into_owned()), content: vec![] });
            }
            OdfEvent::EndSpan => {
                if let Some(BuildFrame::Span { style_name, content }) = self.stack.pop() {
                    self.push_inline(Inline::Span(Span { style_name, content }));
                }
            }
            OdfEvent::StartHyperlink { href, title } => {
                self.stack.push(BuildFrame::Hyperlink {
                    href: href.map(|s| s.into_owned()),
                    title: title.map(|s| s.into_owned()),
                    style_name: None,
                    content: vec![],
                });
            }
            OdfEvent::EndHyperlink => {
                if let Some(BuildFrame::Hyperlink { href, title, style_name, content }) = self.stack.pop() {
                    self.push_inline(Inline::Hyperlink(Hyperlink { href, title, style_name, content }));
                }
            }
            OdfEvent::StartNote { note_class, id } => {
                let class = if note_class == "endnote" { NoteClass::Endnote } else { NoteClass::Footnote };
                self.stack.push(BuildFrame::Note {
                    note: Note { note_class: class, id: id.map(|s| s.into_owned()), citation: None, body: vec![] },
                });
            }
            OdfEvent::EndNote => {
                if let Some(BuildFrame::Note { note }) = self.stack.pop() {
                    self.push_inline(Inline::Note(note));
                }
            }
            OdfEvent::StartFrame { name, anchor_type } => {
                let frame = crate::ast::Frame {
                    name: name.map(|s| s.into_owned()),
                    anchor_type: anchor_type.map(|s| s.into_owned()),
                    ..Default::default()
                };
                self.stack.push(BuildFrame::InlineFrame { frame });
            }
            OdfEvent::EndFrame => {
                if let Some(BuildFrame::InlineFrame { frame }) = self.stack.pop() {
                    self.push_inline(Inline::Frame(frame));
                }
            }
            OdfEvent::Image { href } => {
                let href_s = href.into_owned();
                if let Some(BuildFrame::InlineFrame { frame }) = self.stack.last_mut() {
                    frame.content = FrameContent::Image { href: href_s, mime_type: None };
                }
            }

            // ── Text events ───────────────────────────────────────────────────
            OdfEvent::Text(cow) => {
                let s = cow.into_owned();
                self.push_inline(Inline::Text(s));
            }
            OdfEvent::LineBreak => {
                self.push_inline(Inline::LineBreak);
            }
            OdfEvent::Tab => {
                self.push_inline(Inline::Tab);
            }
            OdfEvent::Space { count } => {
                self.push_inline(Inline::Space { count });
            }

            OdfEvent::Unknown { .. } => {}
        }
    }

    fn attach_shape(&mut self, shape: DrawShape) {
        match self.stack.last_mut() {
            Some(BuildFrame::Slide { page }) => page.shapes.push(shape),
            Some(BuildFrame::Notes { notes }) => notes.shapes.push(shape),
            _ => {}
        }
    }

    fn push_block(&mut self, block: TextBlock) {
        match self.stack.last_mut() {
            Some(BuildFrame::Text { blocks }) => blocks.push(block),
            Some(BuildFrame::ListItem { content }) => content.push(block),
            Some(BuildFrame::TableCell { content, .. }) => content.push(block),
            Some(BuildFrame::Section { content, .. }) => content.push(block),
            Some(BuildFrame::TextBox { blocks }) => blocks.push(block),
            Some(BuildFrame::NoteBody { content }) => content.push(block),
            Some(BuildFrame::SheetCell { cell }) => cell.content.push(block),
            _ => {}
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(BuildFrame::Paragraph { content, .. }) => content.push(inline),
            Some(BuildFrame::Heading { content, .. }) => content.push(inline),
            Some(BuildFrame::Span { content, .. }) => content.push(inline),
            Some(BuildFrame::Hyperlink { content, .. }) => content.push(inline),
            _ => {}
        }
    }

    fn close_frame(&mut self, frame: BuildFrame) {
        if let BuildFrame::Text { blocks } = frame {
            self.doc.body = OdfBody::Text(blocks);
        }
    }
}

