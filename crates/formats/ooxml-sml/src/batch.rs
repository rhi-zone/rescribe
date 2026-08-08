//! Chunk-driven readers for XLSX workbooks: [`BatchParser`] (AST-materializing,
//! buffers all input) and [`StreamingParser`] (genuinely incremental, push-based).
//!
//! # `BatchParser` — buffers, then materializes
//!
//! [`BatchParser`] accepts arbitrary-sized chunks via [`BatchParser::feed`]
//! and parses the complete workbook on [`BatchParser::finish`]. All chunks
//! are buffered until `finish()` is called; memory usage is O(full input).
//! Use this when the full document is going to be materialized as a
//! [`Workbook`] anyway — there is nothing to gain from incremental draining
//! in that case.
//!
//! # `StreamingParser<H>` — genuinely incremental, chunk-fed reader
//!
//! [`StreamingParser`] drives [`ooxml_opc::StreamingParser`] for OPC
//! container-level entry delivery (`feed()` accepts archive bytes directly,
//! no seeking, no full-archive buffering). For each `xl/worksheets/sheetN.xml`
//! part it feeds `PartData` chunks straight into [`crate::chunked_events::ChunkedSmlEvents`]
//! — a chunk-resumable tokenizer producing the same [`OwnedSmlEvent`]
//! vocabulary as [`crate::events::events`] (see that module's own doc
//! comment for the technique) — dispatching `handler.handle(Event::Sheet {
//! .. })` per event as soon as it is provably complete, with no need to see
//! the rest of the part first. `xl/sharedStrings.xml` is still buffered in
//! full (see "Shared strings" below for why that is a deliberate, scoped
//! exception, not an oversight). No `Workbook` or intermediate AST is
//! materialized for either.
//!
//! ## Memory model: O(nesting depth + largest token) for worksheet parts
//!
//! **Update (2026-08-08):** [`ooxml_opc::StreamingParser`] does not buffer a
//! generic part's decompressed bytes in full before delivering it — its
//! `Event::Part` is `PartStart`/`PartData`/`PartEnd`, with `PartData` chunks
//! forwarded as they decompress once `[Content_Types].xml` has streamed past
//! (see its own module docs for the exact contract and the two narrow
//! exceptions that remain there). That closes the OPC-container-layer part
//! of this crate's memory bound.
//!
//! **Update (2026-08-08, later same day):** this crate's own XML layer is
//! now also chunk-fed for worksheet parts. [`crate::chunked_events::ChunkedSmlEvents`]
//! tokenizes a worksheet's XML incrementally as `PartData` chunks arrive —
//! it never holds more than the current in-progress token plus the
//! container-nesting stack (bounded by document depth, which for a
//! worksheet is a handful of levels: `worksheet`, `sheetData`, `row`, `c`,
//! `is`, nested in that order). A worksheet with a million rows streams
//! through in O(nesting depth + largest token) memory, not O(worksheet XML size) —
//! see [`crate::chunked_events`]'s module docs for exactly how, and that
//! module's `pending_buffer_stays_small_for_a_large_worksheet` test for the
//! structural evidence (it asserts the tokenizer's own internal buffer stays
//! bounded while tokenizing a multi-megabyte synthetic worksheet fed in
//! small chunks). `streaming_large_workbook_matches_seekable_reader` (below,
//! in this module) is this bound's end-to-end correctness counterpart: it
//! drives the same large worksheet through the full `StreamingParser<H>`
//! pipeline (OPC container layer included) and checks the resulting events
//! against the seekable `Workbook` reader — `ooxml_opc::StreamingParser`
//! does not expose its inner handler for introspection, so the internal
//! buffer-size assertion itself is only made at the `chunked_events` layer,
//! where the bound was actually implemented.
//!
//! A part's classification (worksheet / shared-strings / generic) is
//! decidable from its `PartStart` (path and resolved content type) alone —
//! never from its content — so classification always happens immediately at
//! `PartStart`, before any of a part's `PartData` has arrived. Non-worksheet,
//! non-`sharedStrings.xml` parts (`xl/workbook.xml`, `xl/styles.xml`,
//! `docProps/*`, chart/drawing parts, ...) are not specially interpreted —
//! this crate's [`Event::Part`] passes each one's complete content through
//! to the caller, same as before, so those still need their bytes
//! accumulated (they are typically small compared to worksheet data, and
//! interpreting them is out of scope for this `StreamingParser` regardless
//! — see [`Event::Part`]'s own doc comment).
//!
//! ## Shared strings: still fully buffered — a deliberate, scoped exception
//!
//! `xl/sharedStrings.xml` is parsed via [`crate::workbook::bootstrap`] into a
//! typed `Vec<String>` table (a different code path from [`crate::events::events`]
//! entirely — see "Shared strings" further below for why the *table itself*
//! is never resolved into cell values here regardless of how it's parsed).
//! That path needs the part's complete bytes: `bootstrap` deserializes the
//! whole `<sst>` element via `serde`/`quick-xml`'s tree deserializer, not an
//! incremental tokenizer, and there's no proportionate value in giving it
//! one — `sharedStrings.xml` is bounded, small (repeated-string) metadata
//! for a workbook, not the multi-hundred-MB-per-part case that motivated
//! this task (a worksheet with millions of rows). This mirrors how
//! `ooxml-opc`'s own module docs draw the same line for `[Content_Types].xml`
//! and `.rels` parts — both are also always fully buffered there, for the
//! same "bounded small-metadata part" reason. `xl/sharedStrings.xml` staying
//! fully buffered here is that same call, made explicitly rather than left
//! implicit; it is not a remaining gap in the worksheet-streaming work this
//! module docs otherwise describe as closed.
//!
//! ## Shared strings: resolution is deliberately left to the caller
//!
//! A cell's `t="s"` value is an index into `xl/sharedStrings.xml`, a
//! *separate* OPC part from the sheet data it is referenced from, and (per
//! ZIP/OPC ordering — neither spec mandates entry order) that part may
//! stream past before or after any given worksheet part. A single-pass
//! streaming reader cannot always resolve a shared-string index at the
//! moment it sees the referencing cell.
//!
//! This crate does not buffer to force an ordering, and does not do a
//! two-pass read (which would mean buffering the whole input, defeating the
//! purpose of streaming). Instead it follows the same pattern already
//! established by [`crate::events::events`] (which never resolves
//! shared-string indices either — see that module's docs) and by
//! [`crate::streaming::SmlWriter::set_shared_strings`] on the write side
//! (which treats the string table as caller-supplied, resolved
//! independently of event order): `Event::Sheet`'s `SmlEvent::CellValue` for
//! a `CellType::SharedString` cell always carries the **raw numeric
//! index**, never a resolved string. `xl/sharedStrings.xml`, once it
//! streams past, is delivered as [`Event::SharedStrings`] — a complete,
//! already-parsed `Vec<String>` table — independent of when any given
//! worksheet's cells were delivered. Resolving indices against that table
//! (before or after the fact, depending on which arrived first) is the
//! caller's responsibility, exactly as it already is for [`events()`]'s
//! output via [`crate::ext::ResolveContext`].
//!
//! [`events()`]: crate::events::events
//!
//! # Example — batch parse
//! ```ignore
//! use ooxml_sml::BatchParser;
//!
//! let mut parser = BatchParser::new();
//! for chunk in file_stream {
//!     parser.feed(&chunk);
//! }
//! let workbook = parser.finish()?;
//! ```
//!
//! # Example — chunk-fed streaming reader
//! ```ignore
//! use ooxml_sml::batch::{Event, StreamingParser};
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: Event| events.push(ev));
//! for chunk in file_stream {
//!     p.feed(&chunk);
//! }
//! let diagnostics = p.finish();
//! ```

use crate::Result;
use crate::workbook::Workbook;
use std::io::Cursor;

/// Chunk-driven XLSX parser. Buffers all input; materializes a [`Workbook`]
/// on [`finish`](BatchParser::finish). See the [module docs](self) for when
/// to prefer [`StreamingParser`] instead.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    pub fn finish(self) -> Result<Workbook<Cursor<Vec<u8>>>> {
        Workbook::from_reader(Cursor::new(self.buf))
    }
}

// ── StreamingParser — genuinely incremental, chunk-fed reader ──────────────

use std::cell::RefCell;
use std::rc::Rc;

pub use rescribe_format_api::Handler;
use rescribe_format_api::{Diagnostic, Severity};

use crate::generated_events::OwnedSmlEvent;
use crate::writer::{CT_SHARED_STRINGS, CT_WORKSHEET};

/// One SML-level streaming event, delivered by [`StreamingParser`]. See the
/// [module docs](self) for the memory-bound and shared-strings contracts.
#[derive(Debug, Clone)]
pub enum Event {
    /// `[Content_Types].xml`, passed through unchanged from the OPC layer.
    ContentTypes(ooxml_opc::ContentTypes),
    /// A `_rels/*.rels` part, passed through unchanged from the OPC layer.
    Relationships {
        part_path: String,
        relationships: ooxml_opc::Relationships,
    },
    /// `xl/sharedStrings.xml`, fully parsed into its string table. May be
    /// delivered before or after any `Event::Sheet` events, depending on
    /// physical ZIP entry order — see the [module docs](self).
    SharedStrings(Vec<String>),
    /// One SAX event from a worksheet part's XML (`xl/worksheets/sheetN.xml`),
    /// tagged with the OPC part path it came from — produced by running
    /// [`crate::events::events`] over that part's bytes. Shared-string cell
    /// values are **not** resolved here; see the [module docs](self).
    Sheet { path: String, event: OwnedSmlEvent },
    /// Any other package part not specially interpreted above
    /// (`xl/workbook.xml`, `xl/styles.xml`, `docProps/*`, chart/drawing
    /// parts, ...) — passed through so callers aren't starved of package
    /// structure. Interpreting these (sheet order/names from
    /// `xl/workbook.xml`, number formats from `xl/styles.xml`, ...) is
    /// package-level `events()` work, explicitly out of scope for this
    /// crate's `StreamingParser` for now.
    Part {
        path: String,
        content_type: Option<String>,
        content: Vec<u8>,
    },
}

/// What the part currently open (since the last `PartStart`) is being
/// treated as — decided from `PartStart`'s path/content-type alone, never
/// from content. `Worksheet` carries its own chunk-resumable tokenizer,
/// fed directly from `PartData`, with no accumulation buffer of its own;
/// `SharedStrings` and `Generic` still accumulate their `PartData` chunks
/// into `InnerHandler::buf` (see the module docs' "Memory model" and
/// "Shared strings" sections for why that remains correct/deliberate for
/// each). Classification itself never waits for content either way.
enum PartKind {
    Worksheet {
        path: String,
        tokenizer: crate::chunked_events::ChunkedSmlEvents,
    },
    SharedStrings,
    Generic {
        path: String,
        content_type: Option<String>,
    },
}

/// Routes `ooxml_opc`'s per-part byte stream to [`Event`]s.
struct InnerHandler<H: Handler<Event>> {
    handler: H,
    diagnostics: Rc<RefCell<Vec<Diagnostic>>>,
    kind: PartKind,
    /// Accumulator for the part currently open, used by `SharedStrings` and
    /// `Generic` parts only — `Worksheet` parts are tokenized incrementally
    /// via `PartKind::Worksheet`'s own `tokenizer` field instead (see the
    /// module docs' "Memory model" section).
    buf: Vec<u8>,
}

impl<H: Handler<Event>> InnerHandler<H> {
    fn classify(&self, path: &str, content_type: Option<String>) -> PartKind {
        let is_worksheet = content_type.as_deref() == Some(CT_WORKSHEET)
            || (path.starts_with("xl/worksheets/") && path.ends_with(".xml"));
        if is_worksheet {
            return PartKind::Worksheet {
                path: path.to_string(),
                tokenizer: crate::chunked_events::ChunkedSmlEvents::new(),
            };
        }
        let is_shared_strings =
            content_type.as_deref() == Some(CT_SHARED_STRINGS) || path == "xl/sharedStrings.xml";
        if is_shared_strings {
            return PartKind::SharedStrings;
        }
        PartKind::Generic {
            path: path.to_string(),
            content_type,
        }
    }

    fn finish_part(&mut self) {
        match std::mem::replace(
            &mut self.kind,
            PartKind::Generic {
                path: String::new(),
                content_type: None,
            },
        ) {
            PartKind::Worksheet { path, tokenizer } => {
                let handler = &mut self.handler;
                tokenizer.finish(&mut |event| {
                    handler.handle(Event::Sheet {
                        path: path.clone(),
                        event,
                    });
                });
            }
            PartKind::SharedStrings => {
                let content = std::mem::take(&mut self.buf);
                match crate::workbook::bootstrap::<crate::types::SharedStrings>(&content) {
                    Ok(sst) => {
                        let strings = crate::workbook::extract_shared_strings(&sst);
                        self.handler.handle(Event::SharedStrings(strings));
                    }
                    Err(e) => {
                        self.diagnostics.borrow_mut().push(Diagnostic::new(
                            Severity::Warning,
                            format!("failed to parse xl/sharedStrings.xml: {e}"),
                        ));
                    }
                }
            }
            PartKind::Generic { path, content_type } => {
                let content = std::mem::take(&mut self.buf);
                self.handler.handle(Event::Part {
                    path,
                    content_type,
                    content,
                });
            }
        }
    }
}

impl<H: Handler<Event>> Handler<ooxml_opc::StreamingEvent> for InnerHandler<H> {
    fn handle(&mut self, event: ooxml_opc::StreamingEvent) {
        match event {
            ooxml_opc::StreamingEvent::ContentTypes(ct) => {
                self.handler.handle(Event::ContentTypes(ct));
            }
            ooxml_opc::StreamingEvent::Relationships {
                part_path,
                relationships,
            } => {
                self.handler.handle(Event::Relationships {
                    part_path,
                    relationships,
                });
            }
            ooxml_opc::StreamingEvent::PartStart { path, content_type } => {
                self.buf.clear();
                self.kind = self.classify(&path, content_type);
            }
            ooxml_opc::StreamingEvent::PartData(chunk) => match &mut self.kind {
                PartKind::Worksheet { path, tokenizer } => {
                    let path = path.clone();
                    let handler = &mut self.handler;
                    tokenizer.feed(&chunk, &mut |event| {
                        handler.handle(Event::Sheet {
                            path: path.clone(),
                            event,
                        });
                    });
                }
                PartKind::SharedStrings | PartKind::Generic { .. } => {
                    self.buf.extend_from_slice(&chunk);
                }
            },
            ooxml_opc::StreamingEvent::PartEnd => self.finish_part(),
        }
    }
}

/// Genuinely incremental, chunk-fed XLSX reader. See the [module docs](self)
/// for the memory-bound and shared-strings contracts.
pub struct StreamingParser<H: Handler<Event>> {
    inner: ooxml_opc::StreamingParser<InnerHandler<H>>,
    /// Shared with `InnerHandler` so sml-level diagnostics (e.g. a
    /// malformed `sharedStrings.xml`) survive `inner.finish()`, which drops
    /// its handler. Mirrors `ooxml_opc::StreamingParser`'s own
    /// `Rc<RefCell<Shared<H>>>` pattern for the same reason.
    diagnostics: Rc<RefCell<Vec<Diagnostic>>>,
}

impl<H: Handler<Event>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        let diagnostics = Rc::new(RefCell::new(Vec::new()));
        let inner_handler = InnerHandler {
            handler,
            diagnostics: diagnostics.clone(),
            kind: PartKind::Generic {
                path: String::new(),
                content_type: None,
            },
            buf: Vec::new(),
        };
        StreamingParser {
            inner: ooxml_opc::StreamingParser::new(inner_handler),
            diagnostics,
        }
    }

    /// Feed the next chunk of `.xlsx` archive bytes. May be called with
    /// chunks of any size, including 1 byte, and any number of times.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.inner.feed(chunk);
    }

    /// Signal end of input. Returns accumulated diagnostics (OPC-layer
    /// diagnostics from `ooxml-opc` plus any sml-level ones, e.g. a
    /// malformed `sharedStrings.xml`).
    pub fn finish(self) -> Vec<Diagnostic> {
        let mut diags = self.inner.finish();
        let sml_diags = Rc::try_unwrap(self.diagnostics)
            .unwrap_or_else(|_| {
                panic!("ooxml-sml StreamingParser: internal state still referenced after finish")
            })
            .into_inner();
        diags.extend(sml_diags);
        diags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::WorkbookBuilder;
    use std::io::Cursor as IoCursor;

    /// Build a small, non-trivial in-memory XLSX: two sheets, string,
    /// number, boolean and formula cells, exercising the shared-string
    /// path.
    fn build_test_workbook() -> Vec<u8> {
        let mut wb = WorkbookBuilder::new();
        {
            let sheet = wb.add_sheet("Sheet1");
            sheet.set_cell("A1", "Hello");
            sheet.set_cell("B1", "World");
            sheet.set_cell("A2", 42.0);
            sheet.set_cell("B2", true);
            sheet.set_cell("A3", "Hello"); // repeated string -> same shared index
        }
        {
            let sheet = wb.add_sheet("Sheet2");
            sheet.set_cell("A1", "Second sheet");
            sheet.set_cell("B1", 3.5);
        }
        let mut buf = IoCursor::new(Vec::new());
        wb.write(&mut buf).unwrap();
        buf.into_inner()
    }

    /// Feeding a real workbook byte-at-a-time (and in a few other uneven
    /// chunk sizes) must produce the same sheet cell events, and the same
    /// shared-string table, as reading it through the seekable `Workbook`
    /// path — the streaming reader's per-part SAX pass must agree with
    /// `parse()`/`events()`'s own semantics; it is a different code path to
    /// the same event vocabulary, not derived from it.
    fn collect(bytes: &[u8], chunk_size: usize) -> (Vec<Event>, Vec<Diagnostic>) {
        let mut events = Vec::new();
        let mut p = StreamingParser::new(|ev| events.push(ev));
        if chunk_size == 0 {
            p.feed(bytes);
        } else {
            for chunk in bytes.chunks(chunk_size) {
                p.feed(chunk);
            }
        }
        let diags = p.finish();
        (events, diags)
    }

    fn sheet_cell_values(events: &[Event], path: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut current_ref: Option<String> = None;
        for ev in events {
            let Event::Sheet { path: p, event } = ev else {
                continue;
            };
            if p != path {
                continue;
            }
            match event {
                OwnedSmlEvent::StartCell { props } => {
                    current_ref = props.reference.clone();
                }
                OwnedSmlEvent::CellValue(v) => {
                    if let Some(r) = current_ref.take() {
                        out.push((r, v.to_string()));
                    }
                }
                _ => {}
            }
        }
        out
    }

    #[test]
    fn streaming_matches_seekable_workbook_shared_strings_and_cells() {
        let bytes = build_test_workbook();

        // Seekable reference.
        let mut wb = crate::workbook::Workbook::from_reader(IoCursor::new(bytes.clone())).unwrap();
        let resolved = wb.resolved_sheets().unwrap();
        let expected_sheet_names: Vec<String> =
            resolved.iter().map(|s| s.name().to_string()).collect();

        for chunk_size in [0, 1, 7, 4096] {
            let (events, diags) = collect(&bytes, chunk_size);
            assert!(diags.is_empty(), "chunk_size={chunk_size}: {diags:?}");

            // ContentTypes and root Relationships must both surface.
            assert!(
                events.iter().any(|e| matches!(e, Event::ContentTypes(_))),
                "chunk_size={chunk_size}: missing ContentTypes"
            );
            assert!(
                events.iter().any(
                    |e| matches!(e, Event::Relationships { part_path, .. } if part_path.is_empty())
                ),
                "chunk_size={chunk_size}: missing root Relationships"
            );

            // Shared strings table: exactly the distinct strings written,
            // deduplicated. `WorkbookBuilder` keys its cells by a `HashMap`
            // (see `writer::SheetBuilder`), so emission order — and hence
            // first-seen order in the SST — is not guaranteed across runs;
            // only the deduplicated *set* is asserted here.
            let sst = events
                .iter()
                .find_map(|e| match e {
                    Event::SharedStrings(v) => Some(v.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("chunk_size={chunk_size}: missing SharedStrings"));
            let sst_set: std::collections::BTreeSet<&str> =
                sst.iter().map(|s| s.as_str()).collect();
            assert_eq!(
                sst_set,
                ["Hello", "World", "Second sheet"].into_iter().collect(),
                "chunk_size={chunk_size}: sst={sst:?}"
            );
            let idx_of = |s: &str| -> String {
                sst.iter()
                    .position(|v| v == s)
                    .unwrap_or_else(|| panic!("chunk_size={chunk_size}: {s:?} not in sst"))
                    .to_string()
            };

            // Every worksheet part must appear, one per sheet.
            let sheet_paths: std::collections::BTreeSet<String> = events
                .iter()
                .filter_map(|e| match e {
                    Event::Sheet { path, .. } => Some(path.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(
                sheet_paths.len(),
                expected_sheet_names.len(),
                "chunk_size={chunk_size}: expected {} worksheet parts, got {:?}",
                expected_sheet_names.len(),
                sheet_paths
            );

            // Cell values for sheet1: A1/B1 are shared-string indices, A2 is
            // a plain number, A3 repeats A1's ("Hello") index.
            let sheet1_cells = sheet_cell_values(&events, "xl/worksheets/sheet1.xml");
            let get = |cell_ref: &str| {
                sheet1_cells
                    .iter()
                    .find(|(r, _)| r == cell_ref)
                    .map(|(_, v)| v.clone())
            };
            assert_eq!(get("A1"), Some(idx_of("Hello")), "chunk_size={chunk_size}");
            assert_eq!(get("B1"), Some(idx_of("World")), "chunk_size={chunk_size}");
            assert_eq!(get("A2"), Some("42".to_string()), "chunk_size={chunk_size}");
            assert_eq!(get("A3"), get("A1"), "chunk_size={chunk_size}");
        }
    }

    #[test]
    fn streaming_no_panic_on_empty_and_truncated_input() {
        let mut events: Vec<Event> = Vec::new();
        {
            let p = StreamingParser::new(|ev| events.push(ev));
            let _diags = p.finish();
        }
        assert!(events.is_empty());

        let bytes = build_test_workbook();
        for cut in [0, 1, bytes.len() / 3, bytes.len() / 2, bytes.len() - 1] {
            let mut events = Vec::new();
            let mut p = StreamingParser::new(|ev| events.push(ev));
            p.feed(&bytes[..cut]);
            let _diags = p.finish();
            // Must not panic regardless of where the input was cut off.
        }
    }

    #[test]
    fn streaming_no_panic_on_arbitrary_bytes() {
        // Not a real archive at all — must not panic.
        for seed in [0u8, 1, 42, 255] {
            let data: Vec<u8> = (0..2000).map(|i| (i as u8).wrapping_add(seed)).collect();
            let mut events: Vec<Event> = Vec::new();
            let mut p = StreamingParser::new(|ev| events.push(ev));
            for chunk in data.chunks(13) {
                p.feed(chunk);
            }
            let _diags = p.finish();
        }
    }

    /// A large single-sheet workbook (tens of thousands of rows, string,
    /// number, boolean, and formula cells, all shared-string-indexed
    /// strings) run through the full `StreamingParser<H>` pipeline —
    /// OPC/ZIP container layer plus the new chunk-resumable worksheet
    /// tokenizer — at a small, adversarial `feed()` chunk size, and checked
    /// against the seekable `Workbook` reader. This is the end-to-end
    /// correctness counterpart to `chunked_events`'s
    /// `pending_buffer_stays_small_for_a_large_worksheet` test, which
    /// asserts the *memory bound* directly on the tokenizer itself (see
    /// this module's doc comment for why the bound can't also be asserted
    /// at this full-pipeline level: `ooxml_opc::StreamingParser` doesn't
    /// expose its inner handler for introspection).
    #[test]
    fn streaming_large_workbook_matches_seekable_reader() {
        let mut wb = WorkbookBuilder::new();
        {
            let sheet = wb.add_sheet("Big");
            for r in 1..=20_000u32 {
                sheet.set_cell(format!("A{r}").as_str(), format!("row-{r}"));
                sheet.set_cell(format!("B{r}").as_str(), r as f64);
                sheet.set_cell(format!("C{r}").as_str(), r % 2 == 0);
                sheet.set_formula(format!("D{r}").as_str(), format!("SUM(B1:B{r})"));
            }
        }
        let mut buf = IoCursor::new(Vec::new());
        wb.write(&mut buf).unwrap();
        let bytes = buf.into_inner();
        assert!(
            bytes.len() > 500_000,
            "expected a sizeable archive, got {} bytes",
            bytes.len()
        );

        // Seekable reference.
        let mut seekable =
            crate::workbook::Workbook::from_reader(IoCursor::new(bytes.clone())).unwrap();
        let resolved = seekable.resolved_sheets().unwrap();
        let expected_row_count = resolved[0].rows().count();

        let (events, diags) = collect(&bytes, 97);
        assert!(diags.is_empty(), "{diags:?}");

        let sst = events
            .iter()
            .find_map(|e| match e {
                Event::SharedStrings(v) => Some(v.clone()),
                _ => None,
            })
            .expect("missing SharedStrings");

        let sheet_path = events
            .iter()
            .find_map(|e| match e {
                Event::Sheet { path, .. } => Some(path.clone()),
                _ => None,
            })
            .expect("missing worksheet part");

        let cells = sheet_cell_values(&events, &sheet_path);
        // `WorkbookBuilder` writes no cached `<v>` for formula cells (see
        // `build_cell`'s `WriteCellValue::Formula` arm) — only A/B/C
        // (string/number/boolean) carry a `<v>`, so 3 per row, not 4;
        // formula presence is checked separately below via `Formula` events.
        assert_eq!(cells.len(), 20_000 * 3);

        let row_count = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::Sheet {
                        event: OwnedSmlEvent::StartRow { .. },
                        ..
                    }
                )
            })
            .count();
        assert_eq!(row_count, expected_row_count);

        let get = |cell_ref: &str| {
            cells
                .iter()
                .find(|(r, _)| r == cell_ref)
                .map(|(_, v)| v.clone())
        };
        // Spot-check a handful of rows across the range, resolving shared
        // strings by hand (the streaming reader never resolves them itself
        // — see the module docs' "Shared strings" section).
        for r in [1u32, 2, 10_000, 20_000] {
            let a = get(&format!("A{r}")).unwrap();
            let idx: usize = a.parse().unwrap();
            assert_eq!(sst[idx], format!("row-{r}"), "row {r}");
            assert_eq!(get(&format!("B{r}")), Some(r.to_string()), "row {r}");
            assert_eq!(
                get(&format!("C{r}")),
                Some(if r % 2 == 0 { "1" } else { "0" }.to_string()),
                "row {r}"
            );
        }

        // Formula text itself is preserved (not just the cached <v> value).
        let formula_events: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                Event::Sheet {
                    event: OwnedSmlEvent::Formula(f),
                    ..
                } => Some(f.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(formula_events.len(), 20_000);
        assert_eq!(formula_events[0], "SUM(B1:B1)");
        assert_eq!(formula_events[19_999], "SUM(B1:B20000)");
    }
}
