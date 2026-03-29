//! True SAX-based streaming iterator for SpreadsheetML (XLSX).
//!
//! `SmlEventIter` wraps a `quick_xml::Reader<&[u8]>` and emits [`SmlEvent`]
//! items without materialising the full workbook tree.
//!
//! # Memory model
//!
//! Memory usage is O(nesting depth).  Unlike DOCX, XLSX row and cell props come
//! from the element's own attributes rather than child elements, so no lookahead
//! buffering is required.  The iterator reads one XML event at a time.
//!
//! # Example
//!
//! ```ignore
//! use ooxml_sml::{SmlEvent, sml_events};
//!
//! for event in sml_events(worksheet_xml_bytes) {
//!     match event {
//!         SmlEvent::StartCell { props } => {
//!             let ref_str = props.reference.as_deref().map(|r| r.as_str()).unwrap_or("");
//!             println!("cell {}", ref_str);
//!         }
//!         SmlEvent::CellValue(v) => println!("  = {}", v),
//!         _ => {}
//!     }
//! }
//! ```

use std::borrow::Cow;

use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;

use super::generated::{Cell, Row};
use super::generated_events::{PropsStrategy, SmlEvent, SmlStartKind, dispatch_start,
    is_text_element, props_strategy};
use ooxml_xml::FromXml;

/// Return a streaming iterator over the SML events in the given worksheet XML bytes.
///
/// `bytes` should be the raw content of `xl/worksheets/sheet1.xml` (or similar)
/// extracted from the XLSX zip.
pub fn events(bytes: &[u8]) -> SmlEventIter<'_> {
    SmlEventIter::new(bytes)
}

// ---------------------------------------------------------------------------
// Internal extracted-event type
// ---------------------------------------------------------------------------

enum XmlInfo {
    ContainerStart {
        kind: SmlStartKind,
        /// Cloned raw bytes of the opening tag (for FromAttrs props parsing).
        tag_bytes: Vec<u8>,
    },
    Leaf(SmlEvent<'static>),
    End,
    Eof,
    Other,
}

// ---------------------------------------------------------------------------
// SmlEventIter
// ---------------------------------------------------------------------------

type SmlEventOwned = SmlEvent<'static>;

#[derive(Debug)]
struct ContextFrame {
    kind: SmlStartKind,
}

/// True streaming XLSX event iterator.
pub struct SmlEventIter<'input> {
    reader: Reader<&'input [u8]>,
    buf: Vec<u8>,
    stack: Vec<ContextFrame>,
    pending: [Option<SmlEventOwned>; 2],
    started: bool,
    done: bool,
}

impl<'input> SmlEventIter<'input> {
    pub fn new(bytes: &'input [u8]) -> Self {
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(false);
        Self {
            reader,
            buf: Vec::with_capacity(256),
            stack: Vec::new(),
            pending: [None, None],
            started: false,
            done: false,
        }
    }

    fn drain_pending(&mut self) -> Option<SmlEventOwned> {
        if let Some(e) = self.pending[0].take() {
            self.pending[0] = self.pending[1].take();
            Some(e)
        } else {
            None
        }
    }
}

impl<'input> Iterator for SmlEventIter<'input> {
    type Item = SmlEvent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(e) = self.drain_pending() {
            return Some(e);
        }
        if !self.started {
            self.started = true;
            return Some(SmlEvent::StartWorkbook);
        }
        if self.done {
            return None;
        }

        loop {
            let info = self.read_xml_info();
            match info {
                XmlInfo::ContainerStart { kind, tag_bytes } => {
                    let start_event = self.build_start_event(kind, &tag_bytes);
                    self.stack.push(ContextFrame { kind });
                    return Some(start_event);
                }
                XmlInfo::Leaf(e) => return Some(e),
                XmlInfo::End => {
                    if let Some(frame) = self.stack.pop() {
                        return Some(end_event_for(frame.kind));
                    }
                }
                XmlInfo::Eof => {
                    self.done = true;
                    return Some(SmlEvent::EndWorkbook);
                }
                XmlInfo::Other => {
                    if let Some(e) = self.drain_pending() {
                        return Some(e);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// XML reading
// ---------------------------------------------------------------------------

impl<'input> SmlEventIter<'input> {
    fn read_xml_info(&mut self) -> XmlInfo {
        self.buf.clear();
        match self.reader.read_event_into(&mut self.buf) {
            Ok(XmlEvent::Start(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if let Some(kind) = dispatch_start(&local) {
                    let tag_bytes = self.buf.clone();
                    return XmlInfo::ContainerStart { kind, tag_bytes };
                }
                // Text-content leaves: v, f, t — read text and emit with context.
                if is_text_element(&local) {
                    let text = read_text_content(&mut self.reader);
                    // Resolve to the right SmlEvent variant based on element name.
                    return XmlInfo::Leaf(text_leaf_event(&local, text));
                }
                skip_element(&mut self.reader);
                XmlInfo::Other
            }
            Ok(XmlEvent::Empty(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                // Empty containers (e.g. <row/>, <c/>) — still track them.
                if let Some(kind) = dispatch_start(&local) {
                    let tag_bytes = self.buf.clone();
                    // Empty element: start + immediate end.
                    let start_event = self.build_start_event(kind, &tag_bytes);
                    let end_event = end_event_for(kind);
                    // Return start; queue end.
                    self.pending[0] = Some(end_event);
                    return XmlInfo::Leaf(start_event);
                }
                XmlInfo::Other
            }
            Ok(XmlEvent::End(_)) => XmlInfo::End,
            Ok(XmlEvent::Eof) | Err(_) => XmlInfo::Eof,
            Ok(_) => XmlInfo::Other,
        }
    }
}

// ---------------------------------------------------------------------------
// Container start-event construction
// ---------------------------------------------------------------------------

impl<'input> SmlEventIter<'input> {
    fn build_start_event(&mut self, kind: SmlStartKind, tag_bytes: &[u8]) -> SmlEvent<'static> {
        match props_strategy(kind) {
            PropsStrategy::FromAttrs => {
                // Re-construct a BytesStart from the cloned tag bytes and parse
                // attributes into the typed props struct (no children consumed).
                let tag_str = std::str::from_utf8(tag_bytes).unwrap_or("");
                // Strip leading '<' if present (buf may include it).
                let content = tag_str.trim_start_matches('<').trim_end_matches('>').trim_end_matches('/');
                let name_len = content
                    .bytes()
                    .position(|b| b == b' ')
                    .unwrap_or(content.len());
                let start = quick_xml::events::BytesStart::from_content(content, name_len);
                match kind {
                    SmlStartKind::Row => {
                        let props = Row::from_xml(&mut self.reader, &start, true)
                            .unwrap_or_default();
                        SmlEvent::StartRow { props: Box::new(props) }
                    }
                    SmlStartKind::Cell => {
                        let props = Cell::from_xml(&mut self.reader, &start, true)
                            .unwrap_or_default();
                        SmlEvent::StartCell { props: Box::new(props) }
                    }
                    _ => start_event_no_props(kind),
                }
            }
            PropsStrategy::None => {
                start_event_no_props(kind)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

fn local_name_owned(raw: &[u8]) -> Vec<u8> {
    raw.iter()
        .position(|&b| b == b':')
        .map_or_else(|| raw.to_vec(), |i| raw[i + 1..].to_vec())
}

fn skip_element(reader: &mut Reader<&[u8]>) {
    let mut depth = 1u32;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(_)) => depth += 1,
            Ok(XmlEvent::End(_)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(XmlEvent::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

fn read_text_content(reader: &mut Reader<&[u8]>) -> String {
    let mut text = String::new();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Text(ref e)) => {
                text.push_str(&e.decode().unwrap_or_default());
            }
            Ok(XmlEvent::CData(ref e)) => {
                text.push_str(&e.decode().unwrap_or_default());
            }
            Ok(XmlEvent::End(_)) | Ok(XmlEvent::Eof) | Err(_) => break,
            _ => {}
        }
    }
    text
}

fn end_event_for(kind: SmlStartKind) -> SmlEvent<'static> {
    match kind {
        SmlStartKind::Worksheet => SmlEvent::EndWorksheet,
        SmlStartKind::SheetData => SmlEvent::EndSheetData,
        SmlStartKind::Row => SmlEvent::EndRow,
        SmlStartKind::Cell => SmlEvent::EndCell,
        SmlStartKind::InlineString => SmlEvent::EndInlineString,
    }
}

fn start_event_no_props(kind: SmlStartKind) -> SmlEvent<'static> {
    match kind {
        SmlStartKind::Worksheet => SmlEvent::StartWorksheet,
        SmlStartKind::SheetData => SmlEvent::StartSheetData,
        SmlStartKind::Row => SmlEvent::StartRow { props: Box::default() },
        SmlStartKind::Cell => SmlEvent::StartCell { props: Box::default() },
        SmlStartKind::InlineString => SmlEvent::StartInlineString,
    }
}

/// Map a text-content element local name + text to the right SmlEvent leaf variant.
fn text_leaf_event(local: &[u8], text: String) -> SmlEvent<'static> {
    match local {
        b"v" => SmlEvent::CellValue(Cow::Owned(text)),
        b"t" => SmlEvent::StringFragment(Cow::Owned(text)),
        b"f" => SmlEvent::Formula(Cow::Owned(text)),
        _ => SmlEvent::CellValue(Cow::Owned(text)),
    }
}
