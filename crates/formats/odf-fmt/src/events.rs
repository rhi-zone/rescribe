//! SAX-style event iterator for ODF documents.
//!
//! `events(input)` returns an [`EventIter`] that yields one [`OdfEvent`] per
//! `next()` call. The parser holds state between calls — no full AST is built.
//!
//! Supported event types cover all three ODF document types:
//! - Text documents (ODT): paragraphs, headings, lists, tables, spans, hyperlinks
//! - Spreadsheets (ODS): sheets, rows, cells with values and formulas
//! - Presentations (ODP): slides, shapes, text boxes, speaker notes
//!
//! Unsupported constructs emit [`OdfEvent::Unknown`] carrying the raw element name.

use crate::ast::{OdfMeta, PageLayout, StyleEntry};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{Cursor, Read};
use zip::ZipArchive;

// ── Event types ───────────────────────────────────────────────────────────────

/// A single parse event from an ODF document.
#[derive(Debug, Clone)]
pub enum OdfEvent<'a> {
    /// `<office:text>` opened.
    StartText,
    /// `</office:text>` closed.
    EndText,

    /// `<text:p>` opened.
    StartParagraph {
        style_name: Option<Cow<'a, str>>,
    },
    /// `</text:p>` closed.
    EndParagraph,

    /// `<text:h>` opened.
    StartHeading {
        style_name: Option<Cow<'a, str>>,
        outline_level: Option<u32>,
    },
    /// `</text:h>` closed.
    EndHeading,

    /// `<text:span>` opened.
    StartSpan {
        style_name: Option<Cow<'a, str>>,
    },
    /// `</text:span>` closed.
    EndSpan,

    /// `<text:a>` opened.
    StartHyperlink {
        href: Option<Cow<'a, str>>,
        title: Option<Cow<'a, str>>,
    },
    /// `</text:a>` closed.
    EndHyperlink,

    /// `<text:list>` opened.
    StartList {
        style_name: Option<Cow<'a, str>>,
    },
    /// `</text:list>` closed.
    EndList,

    /// `<text:list-item>` or `<text:list-header>` opened.
    StartListItem,
    /// `</text:list-item>` or `</text:list-header>` closed.
    EndListItem,

    /// `<table:table>` opened.
    StartTable {
        name: Option<Cow<'a, str>>,
        style_name: Option<Cow<'a, str>>,
    },
    /// `</table:table>` closed.
    EndTable,

    /// `<table:table-row>` opened.
    StartRow {
        style_name: Option<Cow<'a, str>>,
    },
    /// `</table:table-row>` closed.
    EndRow,

    /// `<table:table-cell>` or `<table:covered-table-cell>` opened.
    StartCell {
        style_name: Option<Cow<'a, str>>,
        value_type: Option<Cow<'a, str>>,
        /// The typed value attribute (`office:value`, `office:date-value`, …).
        value: Option<Cow<'a, str>>,
        /// `table:number-columns-spanned`.
        col_span: Option<u32>,
        /// `table:number-rows-spanned`.
        row_span: Option<u32>,
        covered: bool,
    },
    /// `</table:table-cell>` closed.
    EndCell,

    /// `<text:note>` (footnote / endnote).
    StartNote {
        note_class: Cow<'a, str>,
        id: Option<Cow<'a, str>>,
    },
    EndNote,

    /// `<text:note-citation>` — the in-text note marker, as flattened text.
    ///
    /// Emitted as a single event rather than a Start/Text/End triple: the
    /// citation is a text-only leaf in every ODF profile, and `parse()`'s
    /// `Note::citation` is likewise a single `Option<String>`.
    NoteCitation(Cow<'a, str>),
    /// `<text:note-body>` opened — its children are ordinary block events.
    StartNoteBody,
    /// `</text:note-body>` closed.
    EndNoteBody,

    /// `<draw:frame>` opened.
    StartFrame {
        name: Option<Cow<'a, str>>,
        style_name: Option<Cow<'a, str>>,
        anchor_type: Option<Cow<'a, str>>,
        width: Option<Cow<'a, str>>,
        height: Option<Cow<'a, str>>,
    },
    EndFrame,

    /// `<draw:image>` inside a frame.
    Image {
        href: Cow<'a, str>,
        mime_type: Option<Cow<'a, str>>,
    },

    /// A run of text.
    Text(Cow<'a, str>),

    /// `<text:line-break/>`.
    LineBreak,
    /// `<text:tab/>`.
    Tab,
    /// `<text:s/>` — one or more spaces.
    Space {
        count: u32,
    },
    /// `<text:soft-hyphen/>`.
    SoftHyphen,
    /// `<text:soft-page-break/>`.
    SoftPageBreak,
    /// `<text:bookmark/>` or `<text:bookmark-start/>` — a named anchor point.
    ///
    /// `<text:bookmark-end/>` produces no event, matching `parse()`, which
    /// models a bookmark as a single point rather than a range.
    Bookmark {
        name: Cow<'a, str>,
    },
    /// `<office:annotation>` — an inline comment, as flattened text.
    ///
    /// The annotation's `<text:p>` children are joined with a single space
    /// and all other children (`<dc:creator>`, `<dc:date>`, …) are dropped,
    /// matching `parse()`'s `Inline::Annotation { content }`.
    Annotation {
        content: Cow<'a, str>,
    },
    /// An inline field element (`<text:date>`, `<text:page-number>`, …),
    /// carrying the element name and its flattened text value.
    Field {
        name: Cow<'a, str>,
        value: Cow<'a, str>,
    },

    // ── ODS spreadsheet events ─────────────────────────────────────────────
    /// `<office:spreadsheet>` opened.
    StartSpreadsheet,
    /// `</office:spreadsheet>` closed.
    EndSpreadsheet,

    /// `<table:table>` opened (spreadsheet sheet).
    StartSheet {
        name: Option<Cow<'a, str>>,
        style_name: Option<Cow<'a, str>>,
    },
    /// `</table:table>` closed (spreadsheet sheet).
    EndSheet,

    /// `<table:table-row>` opened (spreadsheet row).
    StartSheetRow {
        style_name: Option<Cow<'a, str>>,
        repeated: Option<u32>,
    },
    /// `</table:table-row>` closed.
    EndSheetRow,

    /// `<table:table-cell>` or `<table:covered-table-cell>` opened (spreadsheet cell).
    StartSheetCell {
        style_name: Option<Cow<'a, str>>,
        value_type: Option<Cow<'a, str>>,
        value: Option<Cow<'a, str>>,
        formula: Option<Cow<'a, str>>,
        covered: bool,
    },
    /// `</table:table-cell>` or `</table:covered-table-cell>` closed.
    EndSheetCell,

    // ── ODP presentation events ────────────────────────────────────────────
    /// `<office:presentation>` opened.
    StartPresentation,
    /// `</office:presentation>` closed.
    EndPresentation,

    /// `<draw:page>` opened.
    StartSlide {
        name: Option<Cow<'a, str>>,
        master_page_name: Option<Cow<'a, str>>,
        layout_name: Option<Cow<'a, str>>,
    },
    /// `</draw:page>` closed.
    EndSlide,

    /// `<draw:frame>` or `<draw:custom-shape>` opened (presentation shape).
    StartShape {
        name: Option<Cow<'a, str>>,
        presentation_class: Option<Cow<'a, str>>,
        x: Option<Cow<'a, str>>,
        y: Option<Cow<'a, str>>,
        width: Option<Cow<'a, str>>,
        height: Option<Cow<'a, str>>,
    },
    /// `</draw:frame>` or `</draw:custom-shape>` closed.
    EndShape,

    /// `<draw:text-box>` opened.
    StartTextBox,
    /// `</draw:text-box>` closed.
    EndTextBox,

    /// `<presentation:notes>` opened.
    StartNotes {
        style_name: Option<Cow<'a, str>>,
    },
    /// `</presentation:notes>` closed.
    EndNotes,

    // ── Package-level events ───────────────────────────────────────────────
    // These carry the parts of an ODF document that live outside
    // `content.xml`'s body: the `mimetype` package entry, `meta.xml`,
    // `styles.xml`'s named styles / page layouts, `content.xml`'s
    // automatic styles and list styles, and embedded resource bytes
    // (`Pictures/`, `media/`). Without them, `batch::Writer`'s
    // reconstructed `OdfDocument` always had an empty `mimetype`, default
    // `meta`, and no styles or images — see `KNOWN_FAILURES["odt"]
    // ["streaming_writer"]` in `rescribe-fixtures`.
    /// The ZIP package's `mimetype` entry.
    Mimetype(String),

    /// Document metadata from `meta.xml` `<office:meta>`.
    Meta(OdfMeta),

    /// One entry from `content.xml`'s `<office:automatic-styles>`.
    AutomaticStyle(StyleEntry),

    /// One entry from `styles.xml`'s `<office:styles>` (named styles).
    NamedStyle(StyleEntry),

    /// One `<text:list-style>` from `content.xml`'s automatic-styles:
    /// `(style-name, is_ordered)`.
    ListStyle(String, bool),

    /// One `<style:page-layout>` from `styles.xml`.
    PageLayout(PageLayout),

    /// One embedded resource from `Pictures/` or `media/`, keyed by its
    /// path within the ZIP archive.
    ///
    /// Named `EmbeddedImage` rather than `Image` to avoid colliding with
    /// the existing inline `Image { href }` event above, which represents
    /// a `<draw:image>` element's `xlink:href` reference inside body
    /// content, not the referenced resource's bytes.
    EmbeddedImage {
        name: String,
        data: Vec<u8>,
    },

    /// Another raw-preserved package part with no cross-format IR
    /// equivalent — `settings.xml` (application view state) or an ODF
    /// 1.2+ RDF metadata part (`META-INF/manifest.rdf` and any other
    /// `*.rdf` part) — carried verbatim. Mirrors
    /// `ast::OdfDocument::extra_parts`; see that field's doc comment for
    /// why these aren't parsed further.
    ExtraPart {
        name: String,
        data: Vec<u8>,
    },

    /// An element not otherwise handled, with its full XML captured verbatim
    /// (opening tag, children, closing tag) so a writer can re-emit it.
    ///
    /// The whole subtree is consumed when this event is produced — no events
    /// are emitted for its descendants. Where the enclosing construct lands
    /// it (inline run, block sequence, frame body, or nowhere) is decided by
    /// the consumer, mirroring `parse()`, which produces `Inline::Unknown`,
    /// `TextBlock::Unknown`, `FrameContent::Other`, or nothing depending on
    /// the same context.
    Unknown {
        name: Cow<'a, str>,
        raw: Cow<'a, str>,
    },
}

/// Text-body elements that open an *inline* content model — inside them,
/// only the constructs `parser::parse_inlines` recognizes are structural;
/// everything else is an `Inline::Unknown` raw capture.
pub(crate) const INLINE_CONTAINERS: &[&str] = &["text:p", "text:h", "text:span", "text:a"];

/// Text-body elements that open a *block* content model.
pub(crate) const BLOCK_CONTAINERS: &[&str] = &[
    "text:list",
    "text:list-item",
    "text:list-header",
    "table:table",
    "table:table-row",
    "table:table-cell",
    "table:covered-table-cell",
    "text:note",
    "text:note-body",
    "draw:frame",
    "draw:text-box",
];

/// Elements `parser::parse_inlines` recognizes structurally; every other
/// element encountered in an inline content model is raw-captured.
pub(crate) const INLINE_RECOGNIZED: &[&str] = &[
    "text:span",
    "text:a",
    "text:note",
    "draw:frame",
    "office:annotation",
];

/// Inline field elements whose text content `parse()` captures as
/// `Inline::Field`. Kept in sync with `parser::parse_inlines`.
pub(crate) const FIELD_ELEMENTS: &[&str] = &[
    "text:page-number",
    "text:date",
    "text:time",
    "text:author-name",
    "text:author-initials",
    "text:chapter",
    "text:file-name",
    "text:sequence",
    "text:reference-ref",
    "text:bookmark-ref",
];

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse an ODF ZIP archive and return a SAX-style event iterator.
///
/// The iterator yields owned `OdfEvent<'static>` events — no borrowing from
/// the input slice after the initial ZIP extraction.
pub fn events(input: &[u8]) -> EventIter {
    EventIter::new(input)
}

// ── EventIter ─────────────────────────────────────────────────────────────────

/// An iterator over [`OdfEvent`] values from an ODF document.
///
/// Events are pre-buffered from the content.xml of the ZIP archive.
/// For large files consider using [`crate::parser::parse`] and walking
/// the AST, or a future `StreamingParser` that processes chunks without
/// loading the full content into memory.
pub struct EventIter {
    queue: VecDeque<OdfEvent<'static>>,
}

impl EventIter {
    fn new(input: &[u8]) -> Self {
        let queue = extract_events(input);
        Self { queue }
    }
}

impl Iterator for EventIter {
    type Item = OdfEvent<'static>;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front()
    }
}

// ── Event extraction ──────────────────────────────────────────────────────────

fn extract_events(input: &[u8]) -> VecDeque<OdfEvent<'static>> {
    let cursor = Cursor::new(input);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => return VecDeque::new(),
    };

    let mut events = VecDeque::new();

    // Package-level parts, emitted before body content so a consumer sees
    // document identity/metadata/styles before the content that depends on
    // them (mirroring the order `parser::parse` reads them in).
    if let Some(mimetype) = crate::parser::read_zip_text(&mut archive, "mimetype") {
        events.push_back(OdfEvent::Mimetype(mimetype.trim().to_string()));
    }

    if let Some(xml) = crate::parser::read_zip_text(&mut archive, "meta.xml") {
        events.push_back(OdfEvent::Meta(crate::parser::parse_meta_xml(&xml)));
    }

    if let Some(xml) = crate::parser::read_zip_text(&mut archive, "styles.xml") {
        let mut diags = Vec::new();
        let (named_styles, page_layouts) = crate::parser::parse_styles_xml(&xml, &mut diags);
        for style in named_styles {
            events.push_back(OdfEvent::NamedStyle(style));
        }
        for layout in page_layouts {
            events.push_back(OdfEvent::PageLayout(layout));
        }
    }

    // content.xml: automatic styles/list styles, then body content.
    let content_xml = {
        let mut f = match archive.by_name("content.xml") {
            Ok(f) => f,
            Err(_) => return events,
        };
        let mut s = String::new();
        if f.read_to_string(&mut s).is_err() {
            return events;
        }
        s
    };
    parse_content_events(&content_xml, &mut events);

    // Embedded resources — order among themselves is not significant, since
    // the AST stores them in an unordered `HashMap`.
    let file_names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    for name in &file_names {
        if (name.starts_with("Pictures/") || name.starts_with("media/"))
            && let Ok(mut f) = archive.by_name(name)
        {
            let mut data = Vec::new();
            if f.read_to_end(&mut data).is_ok() && !data.is_empty() {
                events.push_back(OdfEvent::EmbeddedImage {
                    name: name.clone(),
                    data,
                });
            }
        }
    }

    // Other raw-preserved package parts — see `ast::OdfDocument::extra_parts`
    // and `parser::parse_archive`'s matching scan.
    for name in &file_names {
        if (name == "settings.xml" || name.ends_with(".rdf"))
            && let Ok(mut f) = archive.by_name(name)
        {
            let mut data = Vec::new();
            if f.read_to_end(&mut data).is_ok() && !data.is_empty() {
                events.push_back(OdfEvent::ExtraPart {
                    name: name.clone(),
                    data,
                });
            }
        }
    }

    events
}

/// Which body section we are currently inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BodyKind {
    None,
    Text,
    Spreadsheet,
    Presentation,
}

fn parse_content_events(xml: &str, events: &mut VecDeque<OdfEvent<'static>>) {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_body = false;
    let mut body_kind = BodyKind::None;
    // Content model of each open text-body container: `true` = inline.
    // `open_names` is the parallel element-name stack used to match end tags.
    let mut text_ctx: Vec<bool> = Vec::new();
    let mut open_names: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "office:automatic-styles" => {
                        let (styles, list_styles) =
                            crate::parser::parse_auto_styles_block(&mut reader);
                        for style in styles {
                            events.push_back(OdfEvent::AutomaticStyle(style));
                        }
                        for (style_name, is_ordered) in list_styles {
                            events.push_back(OdfEvent::ListStyle(style_name, is_ordered));
                        }
                    }
                    "office:body" => {
                        in_body = true;
                    }
                    "office:text" if in_body => {
                        body_kind = BodyKind::Text;
                        events.push_back(OdfEvent::StartText);
                    }
                    "office:spreadsheet" if in_body => {
                        body_kind = BodyKind::Spreadsheet;
                        events.push_back(OdfEvent::StartSpreadsheet);
                    }
                    "office:presentation" if in_body => {
                        body_kind = BodyKind::Presentation;
                        events.push_back(OdfEvent::StartPresentation);
                    }
                    // Text-body elements are dispatched through a handler that
                    // gets the reader, because several of them (annotations,
                    // note citations, fields, and unknown elements captured
                    // verbatim) consume their own subtree exactly the way
                    // `parser.rs` does rather than letting the scan descend
                    // into it.
                    _ if body_kind == BodyKind::Text => {
                        let attrs = crate::parser::collect_attrs(e);
                        buf.clear();
                        let in_inline = text_ctx.last().copied().unwrap_or(false);
                        let consumed =
                            push_text_start_event(events, &name, &attrs, &mut reader, in_inline);
                        if !consumed {
                            if INLINE_CONTAINERS.contains(&name.as_str()) {
                                text_ctx.push(true);
                                open_names.push(name);
                            } else if BLOCK_CONTAINERS.contains(&name.as_str()) {
                                text_ctx.push(false);
                                open_names.push(name);
                            }
                        }
                        continue;
                    }
                    _ if body_kind == BodyKind::Spreadsheet => {
                        push_spreadsheet_start_event(events, &name, e);
                    }
                    _ if body_kind == BodyKind::Presentation => {
                        push_presentation_start_event(events, &name, e);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "office:text" && in_body && body_kind == BodyKind::None {
                    // Self-closing `<office:text/>` — mirrors parser.rs's
                    // `parse_content_xml` handling of the same case.
                    events.push_back(OdfEvent::StartText);
                    events.push_back(OdfEvent::EndText);
                } else if body_kind != BodyKind::None {
                    match name.as_str() {
                        "text:line-break" => events.push_back(OdfEvent::LineBreak),
                        "text:tab" => events.push_back(OdfEvent::Tab),
                        "text:s" => {
                            let count = e
                                .attributes()
                                .flatten()
                                .find(|a| a.key.as_ref() == b"text:c")
                                .and_then(|a| String::from_utf8_lossy(&a.value).parse::<u32>().ok())
                                .unwrap_or(1);
                            events.push_back(OdfEvent::Space { count });
                        }
                        "text:soft-hyphen" => events.push_back(OdfEvent::SoftHyphen),
                        "text:soft-page-break" => events.push_back(OdfEvent::SoftPageBreak),
                        "text:bookmark" | "text:bookmark-start" => {
                            let bm = get_attr(e, b"text:name").unwrap_or_default();
                            events.push_back(OdfEvent::Bookmark {
                                name: Cow::Owned(bm),
                            });
                        }
                        // `<text:bookmark-end/>` closes a bookmark range; the
                        // AST models a bookmark as a point, so it is dropped —
                        // same as `parser::parse_inlines`.
                        "text:bookmark-end" => {}
                        "text:p" => {
                            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
                            events.push_back(OdfEvent::StartParagraph { style_name });
                            events.push_back(OdfEvent::EndParagraph);
                        }
                        "draw:image" => {
                            let href = get_attr(e, b"xlink:href")
                                .map(Cow::Owned)
                                .unwrap_or(Cow::Borrowed(""));
                            let mime_type = get_attr(e, b"draw:mime-type").map(Cow::Owned);
                            events.push_back(OdfEvent::Image { href, mime_type });
                        }
                        // Self-closing spreadsheet cells (no content)
                        "table:table-cell" | "table:covered-table-cell"
                            if body_kind == BodyKind::Spreadsheet =>
                        {
                            push_spreadsheet_start_event(events, &name, e);
                            push_spreadsheet_end_event(events, &name);
                        }
                        // Self-closing text-table cells: `<table:covered-table-cell/>`
                        // is how a cell covered by a col/row span is written.
                        "table:table-cell" | "table:covered-table-cell"
                            if body_kind == BodyKind::Text =>
                        {
                            let attrs = crate::parser::collect_attrs(e);
                            push_text_start_event(events, &name, &attrs, &mut reader, false);
                            events.push_back(OdfEvent::EndCell);
                        }
                        "table:table-column" => {}
                        _ if FIELD_ELEMENTS.contains(&name.as_str()) => {
                            events.push_back(OdfEvent::Field {
                                name: Cow::Owned(name.clone()),
                                value: Cow::Borrowed(""),
                            });
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "office:text" if body_kind == BodyKind::Text => {
                        body_kind = BodyKind::None;
                        events.push_back(OdfEvent::EndText);
                    }
                    "office:spreadsheet" if body_kind == BodyKind::Spreadsheet => {
                        body_kind = BodyKind::None;
                        events.push_back(OdfEvent::EndSpreadsheet);
                    }
                    "office:presentation" if body_kind == BodyKind::Presentation => {
                        body_kind = BodyKind::None;
                        events.push_back(OdfEvent::EndPresentation);
                    }
                    "office:body" => {
                        in_body = false;
                    }
                    _ if body_kind != BodyKind::None => {
                        if open_names.last().is_some_and(|n| *n == name) {
                            open_names.pop();
                            text_ctx.pop();
                        }
                        push_end_event(events, &name, body_kind);
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if body_kind != BodyKind::None => {
                let text = e.decode().unwrap_or_default().into_owned();
                if !text.is_empty() {
                    events.push_back(OdfEvent::Text(Cow::Owned(text)));
                }
            }
            // Character/entity references (`&#160;`, `&nbsp;`, …) arrive as
            // their own event, not as part of the surrounding text run.
            // `parse()` decodes them into `Inline::Text`; without this arm
            // they were silently dropped (fixture non-breaking-space).
            Ok(Event::GeneralRef(ref e)) if body_kind != BodyKind::None => {
                let text = crate::parser::decode_general_ref(e);
                if !text.is_empty() {
                    events.push_back(OdfEvent::Text(Cow::Owned(text)));
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
}

/// Handle a `<office:text>`-body start tag.
///
/// Takes pre-collected attributes plus the reader, because several ODF text
/// constructs are leaves whose content `parse()` flattens rather than
/// descends into (`<office:annotation>`, `<text:note-citation>`, field
/// elements) and unknown elements are captured verbatim; all of those must
/// consume their own subtree here so the scan does not also emit events for
/// their descendants.
///
/// `in_inline` says whether the innermost open container has an inline
/// content model. It matters because the same element name means different
/// things in the two models: a `<table:table>` that is a child of
/// `<office:text>` is a real table, while one nested inside a `<text:p>` is
/// not something `parser::parse_inlines` models at all — it becomes an
/// `Inline::Unknown` raw capture (fixture path-deeply-nested-table).
///
/// Returns `true` if the element's subtree was consumed here, in which case
/// the caller must not track it as an open container.
#[allow(clippy::too_many_lines)]
pub(crate) fn push_text_start_event(
    events: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    attrs: &[(String, String)],
    reader: &mut quick_xml::Reader<&[u8]>,
    in_inline: bool,
) -> bool {
    let attr = |key: &str| crate::parser::attr_from_list(attrs, key).map(Cow::Owned);
    if in_inline && !INLINE_RECOGNIZED.contains(&name) && !FIELD_ELEMENTS.contains(&name) {
        let raw = crate::parser::capture_raw_from_name_attrs(name, attrs, reader);
        events.push_back(OdfEvent::Unknown {
            name: Cow::Owned(name.to_owned()),
            raw: Cow::Owned(raw),
        });
        return true;
    }
    match name {
        "text:p" => {
            events.push_back(OdfEvent::StartParagraph {
                style_name: attr("text:style-name"),
            });
        }
        "text:h" => {
            let outline_level = crate::parser::attr_from_list(attrs, "text:outline-level")
                .and_then(|s| s.parse::<u32>().ok());
            events.push_back(OdfEvent::StartHeading {
                style_name: attr("text:style-name"),
                outline_level,
            });
        }
        "text:span" => {
            events.push_back(OdfEvent::StartSpan {
                style_name: attr("text:style-name"),
            });
        }
        "text:a" => {
            events.push_back(OdfEvent::StartHyperlink {
                href: attr("xlink:href"),
                title: attr("xlink:title"),
            });
        }
        "text:list" => {
            events.push_back(OdfEvent::StartList {
                style_name: attr("text:style-name"),
            });
        }
        "text:list-item" | "text:list-header" => {
            events.push_back(OdfEvent::StartListItem);
        }
        "table:table" => {
            events.push_back(OdfEvent::StartTable {
                name: attr("table:name"),
                style_name: attr("table:style-name"),
            });
        }
        "table:table-row" => {
            events.push_back(OdfEvent::StartRow {
                style_name: attr("table:style-name"),
            });
        }
        // `<table:table-header-rows>` is transparent: `parse()` flattens its
        // rows into the enclosing table's row list, so emit nothing and let
        // the scan continue into the rows.
        "table:table-header-rows" => {}
        "table:table-cell" | "table:covered-table-cell" => {
            let value = crate::parser::cell_raw_value_attrs(attrs).map(Cow::Owned);
            events.push_back(OdfEvent::StartCell {
                style_name: attr("table:style-name"),
                value_type: attr("office:value-type"),
                value,
                col_span: crate::parser::attr_from_list(attrs, "table:number-columns-spanned")
                    .and_then(|s| s.parse().ok()),
                row_span: crate::parser::attr_from_list(attrs, "table:number-rows-spanned")
                    .and_then(|s| s.parse().ok()),
                covered: name == "table:covered-table-cell",
            });
        }
        "text:note" => {
            let note_class = attr("text:note-class").unwrap_or(Cow::Borrowed("footnote"));
            events.push_back(OdfEvent::StartNote {
                note_class,
                id: attr("text:id"),
            });
        }
        "text:note-citation" => {
            let text = crate::parser::read_text_until(reader, "text:note-citation");
            events.push_back(OdfEvent::NoteCitation(Cow::Owned(text)));
            return true;
        }
        "text:note-body" => events.push_back(OdfEvent::StartNoteBody),
        "draw:frame" => {
            events.push_back(OdfEvent::StartFrame {
                name: attr("draw:name"),
                style_name: attr("draw:style-name"),
                anchor_type: attr("text:anchor-type"),
                width: attr("svg:width"),
                height: attr("svg:height"),
            });
        }
        "draw:text-box" => events.push_back(OdfEvent::StartTextBox),
        "draw:image" => {
            events.push_back(OdfEvent::Image {
                href: attr("xlink:href").unwrap_or(Cow::Borrowed("")),
                mime_type: attr("draw:mime-type"),
            });
        }
        "office:annotation" => {
            let content = crate::parser::read_annotation_text(reader);
            events.push_back(OdfEvent::Annotation {
                content: Cow::Owned(content),
            });
            return true;
        }
        // `parse()`'s block-level reader drops a `<text:soft-page-break>` that
        // has an explicit end tag rather than modelling it, so consume it and
        // emit nothing. The self-closing form (the one writers actually
        // produce) is handled in the `Event::Empty` arm above.
        "text:soft-page-break" => {
            crate::parser::skip_element(reader);
            return true;
        }
        _ if FIELD_ELEMENTS.contains(&name) => {
            let value = crate::parser::read_text_until(reader, name);
            events.push_back(OdfEvent::Field {
                name: Cow::Owned(name.to_owned()),
                value: Cow::Owned(value),
            });
            return true;
        }
        _ => {
            let raw = crate::parser::capture_raw_from_name_attrs(name, attrs, reader);
            events.push_back(OdfEvent::Unknown {
                name: Cow::Owned(name.to_owned()),
                raw: Cow::Owned(raw),
            });
            return true;
        }
    }
    false
}

pub(crate) fn push_spreadsheet_start_event(
    events: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    e: &quick_xml::events::BytesStart<'_>,
) {
    match name {
        "table:table" => {
            let name_attr = get_attr(e, b"table:name").map(Cow::Owned);
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartSheet {
                name: name_attr,
                style_name,
            });
        }
        "table:table-row" => {
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            let repeated =
                get_attr(e, b"table:number-rows-repeated").and_then(|s| s.parse::<u32>().ok());
            events.push_back(OdfEvent::StartSheetRow {
                style_name,
                repeated,
            });
        }
        "table:table-cell" | "table:covered-table-cell" => {
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            let value_type = get_attr(e, b"office:value-type").map(Cow::Owned);
            let value = get_spreadsheet_value(e, value_type.as_deref()).map(Cow::Owned);
            let formula = get_attr(e, b"table:formula").map(Cow::Owned);
            let covered = name == "table:covered-table-cell";
            events.push_back(OdfEvent::StartSheetCell {
                style_name,
                value_type,
                value,
                formula,
                covered,
            });
        }
        "text:p" => {
            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartParagraph { style_name });
        }
        "text:span" => {
            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartSpan { style_name });
        }
        _ => {}
    }
}

pub(crate) fn push_presentation_start_event(
    events: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    e: &quick_xml::events::BytesStart<'_>,
) {
    match name {
        "draw:page" => {
            let name_attr = get_attr(e, b"draw:name").map(Cow::Owned);
            let master_page_name = get_attr(e, b"draw:master-page-name").map(Cow::Owned);
            let layout_name =
                get_attr(e, b"presentation:presentation-page-layout-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartSlide {
                name: name_attr,
                master_page_name,
                layout_name,
            });
        }
        "draw:frame" | "draw:custom-shape" => {
            let frame_name = get_attr(e, b"draw:name").map(Cow::Owned);
            let presentation_class = get_attr(e, b"presentation:class").map(Cow::Owned);
            let x = get_attr(e, b"svg:x").map(Cow::Owned);
            let y = get_attr(e, b"svg:y").map(Cow::Owned);
            let width = get_attr(e, b"svg:width").map(Cow::Owned);
            let height = get_attr(e, b"svg:height").map(Cow::Owned);
            events.push_back(OdfEvent::StartShape {
                name: frame_name,
                presentation_class,
                x,
                y,
                width,
                height,
            });
        }
        "draw:text-box" => {
            events.push_back(OdfEvent::StartTextBox);
        }
        "presentation:notes" => {
            let style_name = get_attr(e, b"draw:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartNotes { style_name });
        }
        "draw:image" => {
            let href = get_attr(e, b"xlink:href")
                .map(Cow::Owned)
                .unwrap_or(Cow::Borrowed(""));
            let mime_type = get_attr(e, b"draw:mime-type").map(Cow::Owned);
            events.push_back(OdfEvent::Image { href, mime_type });
        }
        "text:p" => {
            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartParagraph { style_name });
        }
        "text:span" => {
            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartSpan { style_name });
        }
        _ => {}
    }
}

pub(crate) fn push_end_event(
    events: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    body_kind: BodyKind,
) {
    match body_kind {
        BodyKind::Text => push_text_end_event(events, name),
        BodyKind::Spreadsheet => push_spreadsheet_end_event(events, name),
        BodyKind::Presentation => push_presentation_end_event(events, name),
        BodyKind::None => {}
    }
}

fn push_text_end_event(events: &mut VecDeque<OdfEvent<'static>>, name: &str) {
    match name {
        "text:p" => events.push_back(OdfEvent::EndParagraph),
        "text:h" => events.push_back(OdfEvent::EndHeading),
        "text:span" => events.push_back(OdfEvent::EndSpan),
        "text:a" => events.push_back(OdfEvent::EndHyperlink),
        "text:list" => events.push_back(OdfEvent::EndList),
        "text:list-item" | "text:list-header" => events.push_back(OdfEvent::EndListItem),
        "table:table" => events.push_back(OdfEvent::EndTable),
        "table:table-row" => events.push_back(OdfEvent::EndRow),
        "table:table-cell" | "table:covered-table-cell" => events.push_back(OdfEvent::EndCell),
        "text:note" => events.push_back(OdfEvent::EndNote),
        "text:note-body" => events.push_back(OdfEvent::EndNoteBody),
        "draw:frame" => events.push_back(OdfEvent::EndFrame),
        "draw:text-box" => events.push_back(OdfEvent::EndTextBox),
        _ => {}
    }
}

fn push_spreadsheet_end_event(events: &mut VecDeque<OdfEvent<'static>>, name: &str) {
    match name {
        "table:table" => events.push_back(OdfEvent::EndSheet),
        "table:table-row" => events.push_back(OdfEvent::EndSheetRow),
        "table:table-cell" | "table:covered-table-cell" => events.push_back(OdfEvent::EndSheetCell),
        "text:p" => events.push_back(OdfEvent::EndParagraph),
        "text:span" => events.push_back(OdfEvent::EndSpan),
        _ => {}
    }
}

fn push_presentation_end_event(events: &mut VecDeque<OdfEvent<'static>>, name: &str) {
    match name {
        "draw:page" => events.push_back(OdfEvent::EndSlide),
        "draw:frame" | "draw:custom-shape" => events.push_back(OdfEvent::EndShape),
        "draw:text-box" => events.push_back(OdfEvent::EndTextBox),
        "presentation:notes" => events.push_back(OdfEvent::EndNotes),
        "text:p" => events.push_back(OdfEvent::EndParagraph),
        "text:span" => events.push_back(OdfEvent::EndSpan),
        _ => {}
    }
}

pub(crate) fn get_spreadsheet_value(
    e: &quick_xml::events::BytesStart<'_>,
    value_type: Option<&str>,
) -> Option<String> {
    let attr_name: &[u8] = match value_type {
        Some("date") => b"office:date-value",
        Some("time") => b"office:time-value",
        Some("boolean") => b"office:boolean-value",
        Some("currency") => b"office:value",
        _ => b"office:value",
    };
    get_attr(e, attr_name).or_else(|| get_attr(e, b"office:string-value"))
}

pub(crate) fn get_attr(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .map(|a| String::from_utf8_lossy(&a.value).to_string())
}
