//! SAX-style event iterator for ODF documents.
//!
//! `events(input)` returns an [`EventIter`] that yields one [`OdfEvent`] per
//! `next()` call. The parser holds state between calls — no full AST is built.
//!
//! Supported event types cover text document constructs (paragraphs, headings,
//! lists, tables, spans, hyperlinks). Unsupported constructs emit
//! [`OdfEvent::Unknown`] carrying the raw element name.

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

fn parse_content_events(xml: &str, events: &mut VecDeque<OdfEvent<'static>>) {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_body = false;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "office:body" => { in_body = true; }
                    "office:text" if in_body => {
                        in_text = true;
                        events.push_back(OdfEvent::StartText);
                    }
                    _ if in_text => {
                        push_start_event(events, &name, e);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                if in_text {
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
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "office:text" if in_text => {
                        in_text = false;
                        events.push_back(OdfEvent::EndText);
                    }
                    "office:body" => { in_body = false; }
                    _ if in_text => {
                        push_end_event(events, &name);
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if in_text => {
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

fn push_end_event(events: &mut VecDeque<OdfEvent<'static>>, name: &str) {
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

fn get_attr(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    e.attributes().flatten()
        .find(|a| a.key.as_ref() == key)
        .map(|a| String::from_utf8_lossy(&a.value).to_string())
}
