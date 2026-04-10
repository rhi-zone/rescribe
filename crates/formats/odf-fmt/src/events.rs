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
    StartParagraph { style_name: Option<Cow<'a, str>> },
    /// `</text:p>` closed.
    EndParagraph,

    /// `<text:h>` opened.
    StartHeading { style_name: Option<Cow<'a, str>>, outline_level: Option<u32> },
    /// `</text:h>` closed.
    EndHeading,

    /// `<text:span>` opened.
    StartSpan { style_name: Option<Cow<'a, str>> },
    /// `</text:span>` closed.
    EndSpan,

    /// `<text:a>` opened.
    StartHyperlink { href: Option<Cow<'a, str>>, title: Option<Cow<'a, str>> },
    /// `</text:a>` closed.
    EndHyperlink,

    /// `<text:list>` opened.
    StartList { style_name: Option<Cow<'a, str>> },
    /// `</text:list>` closed.
    EndList,

    /// `<text:list-item>` or `<text:list-header>` opened.
    StartListItem,
    /// `</text:list-item>` or `</text:list-header>` closed.
    EndListItem,

    /// `<table:table>` opened.
    StartTable { name: Option<Cow<'a, str>>, style_name: Option<Cow<'a, str>> },
    /// `</table:table>` closed.
    EndTable,

    /// `<table:table-row>` opened.
    StartRow { style_name: Option<Cow<'a, str>> },
    /// `</table:table-row>` closed.
    EndRow,

    /// `<table:table-cell>` or `<table:covered-table-cell>` opened.
    StartCell { style_name: Option<Cow<'a, str>>, value_type: Option<Cow<'a, str>>, covered: bool },
    /// `</table:table-cell>` closed.
    EndCell,

    /// `<text:note>` (footnote / endnote).
    StartNote { note_class: Cow<'a, str>, id: Option<Cow<'a, str>> },
    EndNote,

    /// `<draw:frame>` opened.
    StartFrame { name: Option<Cow<'a, str>>, anchor_type: Option<Cow<'a, str>> },
    EndFrame,

    /// `<draw:image>` inside a frame.
    Image { href: Cow<'a, str> },

    /// A run of text.
    Text(Cow<'a, str>),

    /// `<text:line-break/>`.
    LineBreak,
    /// `<text:tab/>`.
    Tab,
    /// `<text:s/>` — one or more spaces.
    Space { count: u32 },

    // ── ODS spreadsheet events ─────────────────────────────────────────────

    /// `<office:spreadsheet>` opened.
    StartSpreadsheet,
    /// `</office:spreadsheet>` closed.
    EndSpreadsheet,

    /// `<table:table>` opened (spreadsheet sheet).
    StartSheet { name: Option<Cow<'a, str>>, style_name: Option<Cow<'a, str>> },
    /// `</table:table>` closed (spreadsheet sheet).
    EndSheet,

    /// `<table:table-row>` opened (spreadsheet row).
    StartSheetRow { style_name: Option<Cow<'a, str>>, repeated: Option<u32> },
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
    StartNotes { style_name: Option<Cow<'a, str>> },
    /// `</presentation:notes>` closed.
    EndNotes,

    /// An element not otherwise handled.
    Unknown { name: Cow<'a, str> },
}

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

    let content_xml = {
        let mut f = match archive.by_name("content.xml") {
            Ok(f) => f,
            Err(_) => return VecDeque::new(),
        };
        let mut s = String::new();
        if f.read_to_string(&mut s).is_err() {
            return VecDeque::new();
        }
        s
    };

    let mut events = VecDeque::new();
    parse_content_events(&content_xml, &mut events);
    events
}

/// Which body section we are currently inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKind { None, Text, Spreadsheet, Presentation }

fn parse_content_events(xml: &str, events: &mut VecDeque<OdfEvent<'static>>) {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_body = false;
    let mut body_kind = BodyKind::None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "office:body" => { in_body = true; }
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
                    _ if body_kind != BodyKind::None => {
                        push_start_event(events, &name, e, body_kind);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                if body_kind != BodyKind::None {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match name.as_str() {
                        "text:line-break" => events.push_back(OdfEvent::LineBreak),
                        "text:tab" => events.push_back(OdfEvent::Tab),
                        "text:s" => {
                            let count = e.attributes().flatten()
                                .find(|a| a.key.as_ref() == b"text:c")
                                .and_then(|a| String::from_utf8_lossy(&a.value).parse::<u32>().ok())
                                .unwrap_or(1);
                            events.push_back(OdfEvent::Space { count });
                        }
                        "text:p" => {
                            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
                            events.push_back(OdfEvent::StartParagraph { style_name });
                            events.push_back(OdfEvent::EndParagraph);
                        }
                        "draw:image" => {
                            let href = get_attr(e, b"xlink:href")
                                .map(Cow::Owned)
                                .unwrap_or(Cow::Borrowed(""));
                            events.push_back(OdfEvent::Image { href });
                        }
                        // Self-closing spreadsheet cells (no content)
                        "table:table-cell" | "table:covered-table-cell"
                            if body_kind == BodyKind::Spreadsheet =>
                        {
                            push_spreadsheet_start_event(events, &name, e);
                            push_spreadsheet_end_event(events, &name);
                        }
                        "table:table-column" => {}
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
                    "office:body" => { in_body = false; }
                    _ if body_kind != BodyKind::None => {
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
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
}

fn push_start_event(
    events: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    e: &quick_xml::events::BytesStart<'_>,
    body_kind: BodyKind,
) {
    match body_kind {
        BodyKind::Text => push_text_start_event(events, name, e),
        BodyKind::Spreadsheet => push_spreadsheet_start_event(events, name, e),
        BodyKind::Presentation => push_presentation_start_event(events, name, e),
        BodyKind::None => {}
    }
}

fn push_text_start_event(
    events: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    e: &quick_xml::events::BytesStart<'_>,
) {
    match name {
        "text:p" => {
            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartParagraph { style_name });
        }
        "text:h" => {
            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
            let outline_level = get_attr(e, b"text:outline-level")
                .and_then(|s| s.parse::<u32>().ok());
            events.push_back(OdfEvent::StartHeading { style_name, outline_level });
        }
        "text:span" => {
            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartSpan { style_name });
        }
        "text:a" => {
            let href = get_attr(e, b"xlink:href").map(Cow::Owned);
            let title = get_attr(e, b"xlink:title").map(Cow::Owned);
            events.push_back(OdfEvent::StartHyperlink { href, title });
        }
        "text:list" => {
            let style_name = get_attr(e, b"text:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartList { style_name });
        }
        "text:list-item" | "text:list-header" => {
            events.push_back(OdfEvent::StartListItem);
        }
        "table:table" => {
            let name_attr = get_attr(e, b"table:name").map(Cow::Owned);
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartTable { name: name_attr, style_name });
        }
        "table:table-row" => {
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartRow { style_name });
        }
        "table:table-cell" | "table:covered-table-cell" => {
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            let value_type = get_attr(e, b"office:value-type").map(Cow::Owned);
            let covered = name == "table:covered-table-cell";
            events.push_back(OdfEvent::StartCell { style_name, value_type, covered });
        }
        "text:note" => {
            let note_class = get_attr(e, b"text:note-class")
                .map(Cow::Owned)
                .unwrap_or(Cow::Borrowed("footnote"));
            let id = get_attr(e, b"text:id").map(Cow::Owned);
            events.push_back(OdfEvent::StartNote { note_class, id });
        }
        "draw:frame" => {
            let frame_name = get_attr(e, b"draw:name").map(Cow::Owned);
            let anchor_type = get_attr(e, b"text:anchor-type").map(Cow::Owned);
            events.push_back(OdfEvent::StartFrame { name: frame_name, anchor_type });
        }
        "draw:image" => {
            let href = get_attr(e, b"xlink:href")
                .map(Cow::Owned)
                .unwrap_or(Cow::Borrowed(""));
            events.push_back(OdfEvent::Image { href });
        }
        _ => {
            events.push_back(OdfEvent::Unknown { name: Cow::Owned(name.to_owned()) });
        }
    }
}

fn push_spreadsheet_start_event(
    events: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    e: &quick_xml::events::BytesStart<'_>,
) {
    match name {
        "table:table" => {
            let name_attr = get_attr(e, b"table:name").map(Cow::Owned);
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartSheet { name: name_attr, style_name });
        }
        "table:table-row" => {
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            let repeated = get_attr(e, b"table:number-rows-repeated")
                .and_then(|s| s.parse::<u32>().ok());
            events.push_back(OdfEvent::StartSheetRow { style_name, repeated });
        }
        "table:table-cell" | "table:covered-table-cell" => {
            let style_name = get_attr(e, b"table:style-name").map(Cow::Owned);
            let value_type = get_attr(e, b"office:value-type").map(Cow::Owned);
            let value = get_spreadsheet_value(e, value_type.as_deref()).map(Cow::Owned);
            let formula = get_attr(e, b"table:formula").map(Cow::Owned);
            let covered = name == "table:covered-table-cell";
            events.push_back(OdfEvent::StartSheetCell { style_name, value_type, value, formula, covered });
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

fn push_presentation_start_event(
    events: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    e: &quick_xml::events::BytesStart<'_>,
) {
    match name {
        "draw:page" => {
            let name_attr = get_attr(e, b"draw:name").map(Cow::Owned);
            let master_page_name = get_attr(e, b"draw:master-page-name").map(Cow::Owned);
            let layout_name = get_attr(e, b"presentation:presentation-page-layout-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartSlide { name: name_attr, master_page_name, layout_name });
        }
        "draw:frame" | "draw:custom-shape" => {
            let frame_name = get_attr(e, b"draw:name").map(Cow::Owned);
            let presentation_class = get_attr(e, b"presentation:class").map(Cow::Owned);
            let x = get_attr(e, b"svg:x").map(Cow::Owned);
            let y = get_attr(e, b"svg:y").map(Cow::Owned);
            let width = get_attr(e, b"svg:width").map(Cow::Owned);
            let height = get_attr(e, b"svg:height").map(Cow::Owned);
            events.push_back(OdfEvent::StartShape { name: frame_name, presentation_class, x, y, width, height });
        }
        "draw:text-box" => {
            events.push_back(OdfEvent::StartTextBox);
        }
        "presentation:notes" => {
            let style_name = get_attr(e, b"draw:style-name").map(Cow::Owned);
            events.push_back(OdfEvent::StartNotes { style_name });
        }
        "draw:image" => {
            let href = get_attr(e, b"xlink:href").map(Cow::Owned).unwrap_or(Cow::Borrowed(""));
            events.push_back(OdfEvent::Image { href });
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

fn push_end_event(events: &mut VecDeque<OdfEvent<'static>>, name: &str, body_kind: BodyKind) {
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
        "draw:frame" => events.push_back(OdfEvent::EndFrame),
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

fn get_spreadsheet_value(
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
    get_attr(e, attr_name)
        .or_else(|| get_attr(e, b"office:string-value"))
}

fn get_attr(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    e.attributes().flatten()
        .find(|a| a.key.as_ref() == key)
        .map(|a| String::from_utf8_lossy(&a.value).to_string())
}
