//! Chunk-driven (batch) ODF parser and streaming writer, plus the
//! genuinely incremental push-based [`StreamingParser`].
//!
//! # Memory model
//!
//! [`BatchParser`] accumulates chunks until [`finish`](BatchParser::finish)
//! is called, then parses the complete buffer via `zip::ZipArchive` over an
//! in-memory `Cursor` — memory usage is O(full input). Use this when the
//! full document is going to be materialized as an [`OdfDocument`] AST
//! anyway (there is nothing to gain from incremental draining in that
//! case).
//!
//! [`StreamingParser`] is the true chunk-fed reader: it drives ZIP entry
//! delivery via `zip-fmt`'s hand-rolled push-based `StreamingParser` (which
//! parses local file headers directly, rather than requiring the
//! end-of-file central directory the way `zip::ZipArchive` does) and feeds
//! `content.xml`'s bytes — the dominant contributor to a real document's
//! size — into `crate::content_stream::ContentDriver` token-by-token as
//! they arrive, rather than buffering the whole entry first. See
//! `content_stream`'s module docs for the exact memory-bound contract
//! (O(largest token + nesting depth), plus one documented, bounded
//! exception for a handful of subtree-consuming constructs).
//!
//! **Event order versus `events()`/`parse()`:** `events::extract_events`
//! reads named ZIP entries in a fixed logical order (mimetype, then
//! meta.xml, then styles.xml, then content.xml, then embedded images),
//! regardless of their physical position in the archive, because
//! `zip::ZipArchive` supports random access by name.
//! [`StreamingParser`] cannot do this — a true push-based reader only ever
//! sees entries in the order the archive stream delivers them, and
//! reordering would require buffering the whole archive first, defeating
//! the point of streaming. For this crate's own [`Writer`]/[`emit`](crate::emit)
//! output the physical order matches the logical order, so the two APIs'
//! event sequences coincide; for an archive written by some other producer
//! that interleaves these parts differently, [`StreamingParser`]'s event
//! order will differ from `events()`'s. This is an inherent property of
//! genuine streaming over a reorderable container, not a bug — see
//! `docs/format-audit.md`'s odf-fmt entry.
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
//! # Example — chunk-fed streaming reader
//! ```no_run
//! use odf_fmt::batch::StreamingParser;
//! use odf_fmt::OdfEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OdfEvent<'static>| events.push(ev));
//! p.feed(b"...chunk 1...");
//! p.feed(b"...chunk 2...");
//! let diagnostics = p.finish();
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
use crate::content_stream::ContentDriver;
use crate::error::{Diagnostic, Error, ParseResult};
use crate::events::OdfEvent;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Write;
use std::rc::Rc;

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

// ── StreamingParser — genuinely incremental, chunk-fed reader ──────────────

/// Handler trait for streaming ODF events — the shared
/// [`rescribe_format_api::Handler`], not a locally declared trait; see that
/// crate's docs for why bounding `H` by one shared trait (instead of each
/// format crate declaring its own concrete `Handler`) is required for a
/// common `StreamingParse` trait to exist at all. Implemented automatically
/// for any `FnMut(OdfEvent<'static>)`.
pub use rescribe_format_api::Handler;

/// Which ZIP entry is currently open, and what (if anything) is
/// accumulated for it. `content.xml` is the one entry that does *not*
/// accumulate its whole content — its bytes are fed straight into
/// `ContentDriver` as they arrive.
enum CurrentEntry {
    None,
    Mimetype(Vec<u8>),
    Meta(Vec<u8>),
    Styles(Vec<u8>),
    Content,
    Image(String, Vec<u8>),
    /// Any other entry (`META-INF/manifest.xml`, `settings.xml`, ...) —
    /// `events::extract_events` doesn't emit anything for these either.
    Other,
}

/// Routes `zip-fmt`'s per-entry byte stream to `OdfEvent`s, dispatching
/// each to the wrapped user handler as soon as it's provably complete. See
/// the module docs for the full memory/ordering contract.
struct Router<H: Handler<OdfEvent<'static>>> {
    handler: H,
    current: CurrentEntry,
    content_driver: ContentDriver,
    /// Scratch queue: `ContentDriver` and the whole-entry parsers below both
    /// produce zero or more events per call, drained into `handler`
    /// immediately after each call rather than held here.
    scratch: VecDeque<OdfEvent<'static>>,
    diagnostics: Vec<Diagnostic>,
}

impl<H: Handler<OdfEvent<'static>>> Router<H> {
    fn new(handler: H) -> Self {
        Router {
            handler,
            current: CurrentEntry::None,
            content_driver: ContentDriver::new(),
            scratch: VecDeque::new(),
            diagnostics: Vec::new(),
        }
    }

    fn drain_scratch(&mut self) {
        while let Some(ev) = self.scratch.pop_front() {
            self.handler.handle(ev);
        }
    }

    fn start_entry(&mut self, name: &str) {
        self.current = match name {
            "mimetype" => CurrentEntry::Mimetype(Vec::new()),
            "meta.xml" => CurrentEntry::Meta(Vec::new()),
            "styles.xml" => CurrentEntry::Styles(Vec::new()),
            "content.xml" => {
                self.content_driver = ContentDriver::new();
                CurrentEntry::Content
            }
            _ if name.starts_with("Pictures/") || name.starts_with("media/") => {
                CurrentEntry::Image(name.to_string(), Vec::new())
            }
            _ => CurrentEntry::Other,
        };
    }

    fn data(&mut self, chunk: &[u8]) {
        match &mut self.current {
            CurrentEntry::Content => {
                self.content_driver.feed(chunk, &mut self.scratch);
                self.drain_scratch();
            }
            CurrentEntry::Mimetype(buf) | CurrentEntry::Meta(buf) | CurrentEntry::Styles(buf) => {
                buf.extend_from_slice(chunk);
            }
            CurrentEntry::Image(_, buf) => buf.extend_from_slice(chunk),
            CurrentEntry::None | CurrentEntry::Other => {}
        }
    }

    fn end_entry(&mut self) {
        match std::mem::replace(&mut self.current, CurrentEntry::None) {
            CurrentEntry::Mimetype(buf) => {
                let text = String::from_utf8_lossy(&buf).trim().to_string();
                self.handler.handle(OdfEvent::Mimetype(text));
            }
            CurrentEntry::Meta(buf) => {
                let xml = String::from_utf8_lossy(&buf).into_owned();
                self.handler
                    .handle(OdfEvent::Meta(crate::parser::parse_meta_xml(&xml)));
            }
            CurrentEntry::Styles(buf) => {
                let xml = String::from_utf8_lossy(&buf).into_owned();
                let (named_styles, page_layouts) =
                    crate::parser::parse_styles_xml(&xml, &mut self.diagnostics);
                for style in named_styles {
                    self.handler.handle(OdfEvent::NamedStyle(style));
                }
                for layout in page_layouts {
                    self.handler.handle(OdfEvent::PageLayout(layout));
                }
            }
            CurrentEntry::Content => {
                self.content_driver.finish(&mut self.scratch);
                self.drain_scratch();
            }
            CurrentEntry::Image(name, data) => {
                if !data.is_empty() {
                    self.handler.handle(OdfEvent::EmbeddedImage { name, data });
                }
            }
            CurrentEntry::None | CurrentEntry::Other => {}
        }
    }

    /// Final cleanup once the ZIP layer has signalled end of input: flush
    /// `content.xml`'s driver if it never received an `EndEntry` (a
    /// truncated archive cut off mid-entry — `zip-fmt`'s own `finish()`
    /// already reports this as a diagnostic; this ensures whatever body
    /// content *was* fully received still reaches the handler instead of
    /// being silently dropped).
    fn finish(mut self) -> Vec<Diagnostic> {
        if matches!(self.current, CurrentEntry::Content) {
            self.content_driver.finish(&mut self.scratch);
            self.drain_scratch();
        }
        self.diagnostics
    }
}

impl<H: Handler<OdfEvent<'static>>> Handler<zip_fmt::batch::Event> for Router<H> {
    fn handle(&mut self, event: zip_fmt::batch::Event) {
        match event {
            zip_fmt::batch::Event::StartEntry { name, .. } => self.start_entry(&name),
            zip_fmt::batch::Event::Data(chunk) => self.data(&chunk),
            zip_fmt::batch::Event::EndEntry { .. } => self.end_entry(),
            zip_fmt::batch::Event::ArchiveComment(_) => {}
        }
    }
}

/// Shared-ownership wrapper so [`StreamingParser::finish`] can recover
/// `Router`'s state after `zip_fmt::StreamingParser::finish` consumes its
/// own handler — see that method's body for why.
struct SharedRouter<H: Handler<OdfEvent<'static>>>(Rc<RefCell<Router<H>>>);

impl<H: Handler<OdfEvent<'static>>> Clone for SharedRouter<H> {
    fn clone(&self) -> Self {
        SharedRouter(Rc::clone(&self.0))
    }
}

impl<H: Handler<OdfEvent<'static>>> Handler<zip_fmt::batch::Event> for SharedRouter<H> {
    fn handle(&mut self, event: zip_fmt::batch::Event) {
        self.0.borrow_mut().handle(event);
    }
}

/// Genuinely incremental, chunk-fed ODF reader. See the module docs for
/// the memory-bound and event-ordering contract.
pub struct StreamingParser<H: Handler<OdfEvent<'static>>> {
    inner: zip_fmt::StreamingParser<SharedRouter<H>>,
    router: SharedRouter<H>,
}

impl<H: Handler<OdfEvent<'static>>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        let router = SharedRouter(Rc::new(RefCell::new(Router::new(handler))));
        StreamingParser {
            inner: zip_fmt::StreamingParser::new(router.clone()),
            router,
        }
    }

    /// Feed the next chunk of ODF ZIP archive bytes. May be called with
    /// chunks of any size, including 1 byte, and any number of times.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.inner.feed(chunk);
    }

    /// Signal end of input. Returns accumulated diagnostics (ZIP-layer
    /// diagnostics from `zip-fmt` plus any from `styles.xml` parsing).
    ///
    /// `zip_fmt::Diagnostic` and this crate's own [`Diagnostic`] are both
    /// `rescribe_format_api::Diagnostic` re-exports (the same type), so
    /// the two diagnostic lists concatenate directly with no conversion.
    pub fn finish(self) -> Vec<Diagnostic> {
        let mut diags: Vec<Diagnostic> = self.inner.finish();
        // `zip_fmt::StreamingParser::finish` just consumed its own clone of
        // `self.router` — `self.router` (this one) is now the only
        // remaining `Rc`, so this always succeeds.
        let router = Rc::try_unwrap(self.router.0)
            .unwrap_or_else(|_| unreachable!("zip_fmt::StreamingParser::finish drops its handler"))
            .into_inner();
        diags.extend(router.finish());
        diags
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
        Writer {
            sink,
            builder: DocBuilder::new(),
        }
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
    Text {
        blocks: Vec<TextBlock>,
    },
    Spreadsheet {
        sheets: Vec<Sheet>,
        named_ranges: Vec<NamedRange>,
    },
    Presentation {
        pages: Vec<DrawPage>,
    },
    Paragraph {
        style_name: Option<String>,
        content: Vec<Inline>,
    },
    Heading {
        style_name: Option<String>,
        outline_level: Option<u32>,
        content: Vec<Inline>,
    },
    List {
        style_name: Option<String>,
        items: Vec<ListItem>,
    },
    ListItem {
        content: Vec<TextBlock>,
    },
    TableText {
        name: Option<String>,
        style_name: Option<String>,
        rows: Vec<TableRow>,
    },
    TableRow {
        style_name: Option<String>,
        cells: Vec<TableCell>,
    },
    TableCell {
        cell: TableCell,
    },
    #[allow(dead_code)]
    Section {
        style_name: Option<String>,
        name: Option<String>,
        protected: bool,
        content: Vec<TextBlock>,
    },
    InlineFrame {
        frame: crate::ast::Frame,
    },
    // ODS
    Sheet {
        name: Option<String>,
        style_name: Option<String>,
        print: bool,
        columns: Vec<ColumnDef>,
        rows: Vec<SheetRow>,
    },
    SheetRow {
        style_name: Option<String>,
        repeated: Option<u32>,
        cells: Vec<SheetCell>,
    },
    SheetCell {
        cell: SheetCell,
    },
    // ODP
    Slide {
        page: DrawPage,
    },
    Shape {
        shape: DrawShape,
    },
    TextBox {
        blocks: Vec<TextBlock>,
    },
    Notes {
        notes: NotesPage,
    },
    // Inline spans
    Span {
        style_name: Option<String>,
        content: Vec<Inline>,
    },
    Hyperlink {
        href: Option<String>,
        title: Option<String>,
        style_name: Option<String>,
        content: Vec<Inline>,
    },
    Note {
        note: Note,
    },
    NoteBody {
        content: Vec<TextBlock>,
    },
}

struct DocBuilder {
    stack: Vec<BuildFrame>,
    doc: OdfDocument,
}

impl DocBuilder {
    fn new() -> Self {
        DocBuilder {
            stack: vec![],
            doc: OdfDocument::default(),
        }
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
                self.stack.push(BuildFrame::Spreadsheet {
                    sheets: vec![],
                    named_ranges: vec![],
                });
            }
            OdfEvent::EndSpreadsheet => {
                if let Some(BuildFrame::Spreadsheet {
                    sheets,
                    named_ranges,
                }) = self.stack.pop()
                {
                    self.doc.body = OdfBody::Spreadsheet(SpreadsheetBody {
                        sheets,
                        named_ranges,
                    });
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
                self.stack.push(BuildFrame::Paragraph {
                    style_name: style_name.map(|s| s.into_owned()),
                    content: vec![],
                });
            }
            OdfEvent::EndParagraph => {
                if let Some(BuildFrame::Paragraph {
                    style_name,
                    content,
                }) = self.stack.pop()
                {
                    self.push_block(TextBlock::Paragraph(Paragraph {
                        style_name,
                        content,
                        ..Default::default()
                    }));
                }
            }
            OdfEvent::StartHeading {
                style_name,
                outline_level,
            } => {
                self.stack.push(BuildFrame::Heading {
                    style_name: style_name.map(|s| s.into_owned()),
                    outline_level,
                    content: vec![],
                });
            }
            OdfEvent::EndHeading => {
                if let Some(BuildFrame::Heading {
                    style_name,
                    outline_level,
                    content,
                }) = self.stack.pop()
                {
                    self.push_block(TextBlock::Heading(Heading {
                        style_name,
                        outline_level,
                        content,
                        ..Default::default()
                    }));
                }
            }
            OdfEvent::StartList { style_name } => {
                self.stack.push(BuildFrame::List {
                    style_name: style_name.map(|s| s.into_owned()),
                    items: vec![],
                });
            }
            OdfEvent::EndList => {
                if let Some(BuildFrame::List { style_name, items }) = self.stack.pop() {
                    self.push_block(TextBlock::List(List {
                        style_name,
                        items,
                        ..Default::default()
                    }));
                }
            }
            OdfEvent::StartListItem => {
                self.stack.push(BuildFrame::ListItem { content: vec![] });
            }
            OdfEvent::EndListItem => {
                if let Some(BuildFrame::ListItem { content }) = self.stack.pop()
                    && let Some(BuildFrame::List { items, .. }) = self.stack.last_mut()
                {
                    items.push(ListItem {
                        content,
                        ..Default::default()
                    });
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
                if let Some(BuildFrame::TableText {
                    name,
                    style_name,
                    rows,
                }) = self.stack.pop()
                {
                    self.push_block(TextBlock::Table(Table {
                        name,
                        style_name,
                        rows,
                    }));
                }
            }
            OdfEvent::StartRow { style_name } => {
                self.stack.push(BuildFrame::TableRow {
                    style_name: style_name.map(|s| s.into_owned()),
                    cells: vec![],
                });
            }
            OdfEvent::EndRow => {
                if let Some(BuildFrame::TableRow { style_name, cells }) = self.stack.pop()
                    && let Some(BuildFrame::TableText { rows, .. }) = self.stack.last_mut()
                {
                    rows.push(TableRow { style_name, cells });
                }
            }
            OdfEvent::StartCell {
                style_name,
                value_type,
                value,
                col_span,
                row_span,
                covered,
            } => {
                self.stack.push(BuildFrame::TableCell {
                    cell: TableCell {
                        style_name: style_name.map(|s| s.into_owned()),
                        value_type: value_type.map(|s| s.into_owned()),
                        raw_value: value.map(|s| s.into_owned()),
                        col_span,
                        row_span,
                        covered,
                        content: vec![],
                    },
                });
            }
            OdfEvent::EndCell => {
                if let Some(BuildFrame::TableCell { cell }) = self.stack.pop()
                    && let Some(BuildFrame::TableRow { cells, .. }) = self.stack.last_mut()
                {
                    cells.push(cell);
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
                if let Some(BuildFrame::Sheet {
                    name,
                    style_name,
                    print,
                    columns,
                    rows,
                }) = self.stack.pop()
                    && let Some(BuildFrame::Spreadsheet { sheets, .. }) = self.stack.last_mut()
                {
                    sheets.push(Sheet {
                        name,
                        style_name,
                        print,
                        columns,
                        rows,
                    });
                }
            }
            OdfEvent::StartSheetRow {
                style_name,
                repeated,
            } => {
                self.stack.push(BuildFrame::SheetRow {
                    style_name: style_name.map(|s| s.into_owned()),
                    repeated,
                    cells: vec![],
                });
            }
            OdfEvent::EndSheetRow => {
                if let Some(BuildFrame::SheetRow {
                    style_name,
                    repeated,
                    cells,
                }) = self.stack.pop()
                    && let Some(BuildFrame::Sheet { rows, .. }) = self.stack.last_mut()
                {
                    rows.push(SheetRow {
                        style_name,
                        repeated,
                        cells,
                        ..Default::default()
                    });
                }
            }
            OdfEvent::StartSheetCell {
                style_name,
                value_type,
                value,
                formula,
                covered,
            } => {
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
                    && let Some(BuildFrame::SheetRow { cells, .. }) = self.stack.last_mut()
                {
                    cells.push(cell);
                }
            }

            // ── ODP events ────────────────────────────────────────────────────
            OdfEvent::StartSlide {
                name,
                master_page_name,
                layout_name,
            } => {
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
                    && let Some(BuildFrame::Presentation { pages }) = self.stack.last_mut()
                {
                    pages.push(page);
                }
            }
            OdfEvent::StartShape {
                name,
                presentation_class,
                x,
                y,
                width,
                height,
            } => {
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
                if let Some(BuildFrame::TextBox { blocks }) = self.stack.pop() {
                    match self.stack.last_mut() {
                        Some(BuildFrame::Shape { shape }) => {
                            shape.content = DrawShapeContent::TextBox(blocks);
                        }
                        // A `<draw:text-box>` in a text-document `<draw:frame>`
                        // only wins if the frame has no image: `parse()` gives
                        // `<draw:image>` priority over any sibling content.
                        Some(BuildFrame::InlineFrame { frame }) => {
                            if matches!(frame.content, FrameContent::Empty | FrameContent::Other(_))
                            {
                                frame.content = FrameContent::TextBox(blocks);
                            }
                        }
                        _ => {}
                    }
                }
            }
            OdfEvent::StartNotes { style_name } => {
                self.stack.push(BuildFrame::Notes {
                    notes: NotesPage {
                        style_name: style_name.map(|s| s.into_owned()),
                        shapes: vec![],
                    },
                });
            }
            OdfEvent::EndNotes => {
                if let Some(BuildFrame::Notes { notes }) = self.stack.pop()
                    && let Some(BuildFrame::Slide { page }) = self.stack.last_mut()
                {
                    page.notes = Some(Box::new(notes));
                }
            }

            // ── Inline events ─────────────────────────────────────────────────
            OdfEvent::StartSpan { style_name } => {
                self.stack.push(BuildFrame::Span {
                    style_name: style_name.map(|s| s.into_owned()),
                    content: vec![],
                });
            }
            OdfEvent::EndSpan => {
                if let Some(BuildFrame::Span {
                    style_name,
                    content,
                }) = self.stack.pop()
                {
                    self.push_inline(Inline::Span(Span {
                        style_name,
                        content,
                    }));
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
                if let Some(BuildFrame::Hyperlink {
                    href,
                    title,
                    style_name,
                    content,
                }) = self.stack.pop()
                {
                    self.push_inline(Inline::Hyperlink(Hyperlink {
                        href,
                        title,
                        style_name,
                        content,
                    }));
                }
            }
            OdfEvent::StartNote { note_class, id } => {
                let class = if note_class == "endnote" {
                    NoteClass::Endnote
                } else {
                    NoteClass::Footnote
                };
                self.stack.push(BuildFrame::Note {
                    note: Note {
                        note_class: class,
                        id: id.map(|s| s.into_owned()),
                        citation: None,
                        body: vec![],
                    },
                });
            }
            OdfEvent::EndNote => {
                if let Some(BuildFrame::Note { note }) = self.stack.pop() {
                    self.push_inline(Inline::Note(note));
                }
            }
            OdfEvent::NoteCitation(citation) => {
                if let Some(BuildFrame::Note { note }) = self.stack.last_mut() {
                    note.citation = Some(citation.into_owned());
                }
            }
            OdfEvent::StartNoteBody => {
                self.stack.push(BuildFrame::NoteBody { content: vec![] });
            }
            OdfEvent::EndNoteBody => {
                if let Some(BuildFrame::NoteBody { content }) = self.stack.pop()
                    && let Some(BuildFrame::Note { note }) = self.stack.last_mut()
                {
                    note.body = content;
                }
            }
            OdfEvent::StartFrame {
                name,
                style_name,
                anchor_type,
                width,
                height,
            } => {
                let frame = crate::ast::Frame {
                    name: name.map(|s| s.into_owned()),
                    style_name: style_name.map(|s| s.into_owned()),
                    anchor_type: anchor_type.map(|s| s.into_owned()),
                    width: width.map(|s| s.into_owned()),
                    height: height.map(|s| s.into_owned()),
                    ..Default::default()
                };
                self.stack.push(BuildFrame::InlineFrame { frame });
            }
            OdfEvent::EndFrame => {
                if let Some(BuildFrame::InlineFrame { frame }) = self.stack.pop() {
                    // A `<draw:frame>` is inline only when it sits inside a
                    // paragraph, heading, span or link; at block level it is a
                    // `TextBlock::Frame`, which is what `parse()` produces for
                    // a frame that is a direct child of `<office:text>` or of
                    // a cell / list item / text box.
                    if self.in_inline_context() {
                        self.push_inline(Inline::Frame(frame));
                    } else {
                        self.push_block(TextBlock::Frame(frame));
                    }
                }
            }
            OdfEvent::Image { href, mime_type } => {
                let href = href.into_owned();
                let mime_type = mime_type.map(|s| s.into_owned());
                if let Some(BuildFrame::InlineFrame { frame }) = self.stack.last_mut() {
                    // Unconditional: `parse()` lets an image override content
                    // already collected from sibling elements.
                    frame.content = FrameContent::Image { href, mime_type };
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
            OdfEvent::SoftHyphen => {
                self.push_inline(Inline::SoftHyphen);
            }
            OdfEvent::SoftPageBreak => {
                self.push_inline(Inline::SoftPageBreak);
            }
            OdfEvent::Bookmark { name } => {
                self.push_inline(Inline::Bookmark {
                    name: name.into_owned(),
                });
            }
            OdfEvent::Annotation { content } => {
                self.push_inline(Inline::Annotation {
                    content: content.into_owned(),
                });
            }
            OdfEvent::Field { name, value } => {
                self.push_inline(Inline::Field {
                    name: name.into_owned(),
                    value: value.into_owned(),
                });
            }

            // ── Package-level events ──────────────────────────────────────────
            OdfEvent::Mimetype(mimetype) => {
                self.doc.mimetype = mimetype;
            }
            OdfEvent::Meta(meta) => {
                self.doc.meta = meta;
            }
            OdfEvent::AutomaticStyle(style) => {
                self.doc.automatic_styles.push(style);
            }
            OdfEvent::NamedStyle(style) => {
                self.doc.named_styles.push(style);
            }
            OdfEvent::ListStyle(name, is_ordered) => {
                self.doc.list_styles.push((name, is_ordered));
            }
            OdfEvent::PageLayout(layout) => {
                self.doc.page_layouts.push(layout);
            }
            OdfEvent::EmbeddedImage { name, data } => {
                self.doc.images.insert(name, data);
            }

            // Raw-preserved element. Where it lands depends on what is open,
            // mirroring `parse()`: inline run, block sequence, frame body, or
            // — inside a table, row or note wrapper, where `parse()` skips
            // unrecognized children outright — nowhere.
            OdfEvent::Unknown { name, raw } => {
                let name = name.into_owned();
                let raw = raw.into_owned();
                match self.stack.last_mut() {
                    Some(
                        BuildFrame::Paragraph { .. }
                        | BuildFrame::Heading { .. }
                        | BuildFrame::Span { .. }
                        | BuildFrame::Hyperlink { .. },
                    ) => self.push_inline(Inline::Unknown { name, raw }),
                    Some(BuildFrame::InlineFrame { frame }) => {
                        if matches!(frame.content, FrameContent::Empty) {
                            frame.content = FrameContent::Other(raw);
                        }
                    }
                    Some(
                        BuildFrame::Text { .. }
                        | BuildFrame::ListItem { .. }
                        | BuildFrame::TableCell { .. }
                        | BuildFrame::Section { .. }
                        | BuildFrame::TextBox { .. }
                        | BuildFrame::NoteBody { .. },
                    ) => self.push_block(TextBlock::Unknown { name, raw }),
                    _ => {}
                }
            }
        }
    }

    /// Whether the innermost open frame accepts inline content.
    fn in_inline_context(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                BuildFrame::Paragraph { .. }
                    | BuildFrame::Heading { .. }
                    | BuildFrame::Span { .. }
                    | BuildFrame::Hyperlink { .. }
            )
        )
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
            Some(BuildFrame::TableCell { cell }) => cell.content.push(block),
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
