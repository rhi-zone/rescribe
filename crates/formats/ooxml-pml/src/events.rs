//! True SAX-based streaming iterator for PresentationML (PPTX).
//!
//! `PmlEventIter` wraps a `quick_xml::Reader<&[u8]>` and emits [`PmlEvent`]
//! items without materialising the full slide tree.
//!
//! # Memory model
//!
//! Memory usage is O(nesting depth + largest props element) — bounded by the
//! largest `pPr`/`rPr`/`tcPr`/`tblPr` child element buffered for props parsing.
//!
//! # Example
//!
//! ```ignore
//! use ooxml_pml::{PmlEvent, pml_events};
//!
//! for event in pml_events(slide_xml_bytes) {
//!     match event {
//!         PmlEvent::Text(t) => print!("{}", t),
//!         PmlEvent::LineBreak => println!(),
//!         _ => {}
//!     }
//! }
//! ```

use std::borrow::Cow;

use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;

use ooxml_dml::types::{CTTableCellProperties, CTTableProperties, TextCharacterProperties,
    TextParagraphProperties};
use super::generated_events::{PmlEvent, PmlStartKind, dispatch_start, is_text_element};
use ooxml_xml::FromXml;

/// Return a streaming iterator over the PML events in the given slide XML bytes.
///
/// `bytes` should be the raw content of `ppt/slides/slide1.xml` (or similar)
/// extracted from the PPTX zip.
pub fn events(bytes: &[u8]) -> PmlEventIter<'_> {
    PmlEventIter::new(bytes)
}

// ---------------------------------------------------------------------------
// Internal extracted-event type
// ---------------------------------------------------------------------------

enum XmlInfo {
    ContainerStart(PmlStartKind),
    HyperlinkStart {
        rel_id: Option<String>,
    },
    Leaf(PmlEvent<'static>),
    End,
    Text(String),
    Eof,
    Other,
}

// ---------------------------------------------------------------------------
// PmlEventIter
// ---------------------------------------------------------------------------

type PmlEventOwned = PmlEvent<'static>;

#[derive(Debug)]
struct ContextFrame {
    kind: PmlStartKind,
}

/// True streaming PPTX event iterator.
pub struct PmlEventIter<'input> {
    reader: Reader<&'input [u8]>,
    buf: Vec<u8>,
    stack: Vec<ContextFrame>,
    pending: [Option<PmlEventOwned>; 2],
    started: bool,
    done: bool,
}

impl<'input> PmlEventIter<'input> {
    pub fn new(bytes: &'input [u8]) -> Self {
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(false);
        Self {
            reader,
            buf: Vec::with_capacity(512),
            stack: Vec::new(),
            pending: [None, None],
            started: false,
            done: false,
        }
    }

    fn drain_pending(&mut self) -> Option<PmlEventOwned> {
        if let Some(e) = self.pending[0].take() {
            self.pending[0] = self.pending[1].take();
            Some(e)
        } else {
            None
        }
    }

    fn queue(&mut self, first: PmlEventOwned, second: Option<PmlEventOwned>) {
        self.pending[0] = Some(first);
        self.pending[1] = second;
    }
}

impl<'input> Iterator for PmlEventIter<'input> {
    type Item = PmlEvent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(e) = self.drain_pending() {
            return Some(e);
        }
        if !self.started {
            self.started = true;
            return Some(PmlEvent::StartPresentation);
        }
        if self.done {
            return None;
        }

        loop {
            let info = self.read_xml_info();
            match info {
                XmlInfo::ContainerStart(kind) => {
                    let start_event = self.build_start_event(kind);
                    self.stack.push(ContextFrame { kind });
                    return Some(start_event);
                }
                XmlInfo::HyperlinkStart { rel_id } => {
                    self.stack.push(ContextFrame { kind: PmlStartKind::Hyperlink });
                    return Some(PmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                    });
                }
                XmlInfo::Leaf(e) => return Some(e),
                XmlInfo::End => {
                    if let Some(frame) = self.stack.pop() {
                        return Some(end_event_for(frame.kind));
                    }
                }
                XmlInfo::Text(t) => {
                    if !t.is_empty() {
                        return Some(PmlEvent::Text(Cow::Owned(t)));
                    }
                }
                XmlInfo::Eof => {
                    self.done = true;
                    return Some(PmlEvent::EndPresentation);
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

impl<'input> PmlEventIter<'input> {
    fn read_xml_info(&mut self) -> XmlInfo {
        self.buf.clear();
        match self.reader.read_event_into(&mut self.buf) {
            Ok(XmlEvent::Start(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == b"hlinkClick" || local == b"hlinkMouseOver" {
                    let rel_id = attr_string(e, b"r:id");
                    return XmlInfo::HyperlinkStart { rel_id };
                }
                if let Some(kind) = dispatch_start(&local) {
                    return XmlInfo::ContainerStart(kind);
                }
                if is_text_element(&local) {
                    let text = read_text_content(&mut self.reader);
                    return XmlInfo::Text(text);
                }
                skip_element(&mut self.reader);
                XmlInfo::Other
            }
            Ok(XmlEvent::Empty(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == b"br" {
                    return XmlInfo::Leaf(PmlEvent::LineBreak);
                }
                if local == b"fldId" {
                    let field_type = attr_string(e, b"type").map(Cow::Owned);
                    return XmlInfo::Leaf(PmlEvent::FieldId { field_type });
                }
                XmlInfo::Other
            }
            Ok(XmlEvent::End(_)) => XmlInfo::End,
            Ok(XmlEvent::Text(ref e)) => {
                let text = e.decode().unwrap_or_default().into_owned();
                XmlInfo::Text(text)
            }
            Ok(XmlEvent::CData(ref e)) => {
                let text = e.decode().unwrap_or_default().into_owned();
                XmlInfo::Text(text)
            }
            Ok(XmlEvent::Eof) | Err(_) => XmlInfo::Eof,
            Ok(_) => XmlInfo::Other,
        }
    }

    fn read_xml_info_or_props(&mut self, props_local: &[u8]) -> PropsOrInfo {
        self.buf.clear();
        match self.reader.read_event_into(&mut self.buf) {
            Ok(XmlEvent::Start(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == props_local {
                    return PropsOrInfo::IsProps { is_empty: false };
                }
                if local == b"hlinkClick" || local == b"hlinkMouseOver" {
                    let rel_id = attr_string(e, b"r:id");
                    return PropsOrInfo::Info(XmlInfo::HyperlinkStart { rel_id });
                }
                if let Some(kind) = dispatch_start(&local) {
                    return PropsOrInfo::Info(XmlInfo::ContainerStart(kind));
                }
                if is_text_element(&local) {
                    let text = read_text_content(&mut self.reader);
                    return PropsOrInfo::Info(XmlInfo::Text(text));
                }
                skip_element(&mut self.reader);
                PropsOrInfo::Info(XmlInfo::Other)
            }
            Ok(XmlEvent::Empty(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == props_local {
                    return PropsOrInfo::IsProps { is_empty: true };
                }
                PropsOrInfo::Info(XmlInfo::Other)
            }
            Ok(XmlEvent::End(_)) => PropsOrInfo::Info(XmlInfo::End),
            Ok(XmlEvent::Text(ref e)) => {
                let text = e.decode().unwrap_or_default().into_owned();
                PropsOrInfo::Info(XmlInfo::Text(text))
            }
            Ok(XmlEvent::Eof) | Err(_) => PropsOrInfo::Info(XmlInfo::Eof),
            Ok(_) => PropsOrInfo::Info(XmlInfo::Other),
        }
    }
}

enum PropsOrInfo {
    IsProps { is_empty: bool },
    Info(XmlInfo),
}

// ---------------------------------------------------------------------------
// Container start-event construction
// ---------------------------------------------------------------------------

impl<'input> PmlEventIter<'input> {
    fn build_start_event(&mut self, kind: PmlStartKind) -> PmlEvent<'static> {
        match kind {
            PmlStartKind::Paragraph => {
                let props = self.read_props::<TextParagraphProperties>(b"pPr");
                PmlEvent::StartParagraph { props: Box::new(props) }
            }
            PmlStartKind::Run => {
                let props = self.read_props::<TextCharacterProperties>(b"rPr");
                PmlEvent::StartRun { props: Box::new(props) }
            }
            PmlStartKind::Table => {
                let props = self.read_props::<CTTableProperties>(b"tblPr");
                PmlEvent::StartTable { props: Box::new(props) }
            }
            PmlStartKind::TableCell => {
                let props = self.read_props::<CTTableCellProperties>(b"tcPr");
                PmlEvent::StartTableCell { props: Box::new(props) }
            }
            PmlStartKind::Shape => PmlEvent::StartShape,
            PmlStartKind::GraphicFrame => PmlEvent::StartGraphicFrame,
            PmlStartKind::TableRow => PmlEvent::StartTableRow,
            PmlStartKind::Hyperlink => unreachable!(),
        }
    }

    fn read_props<T: FromXml + Default>(&mut self, expected_local: &[u8]) -> T {
        loop {
            match self.read_xml_info_or_props(expected_local) {
                PropsOrInfo::IsProps { is_empty } => {
                    let tag_bytes = self.buf.clone();
                    let tag_str = std::str::from_utf8(&tag_bytes).unwrap_or("");
                    let content = tag_str.trim_start_matches('<').trim_end_matches('>').trim_end_matches('/');
                    let name_len = content.bytes().position(|b| b == b' ').unwrap_or(content.len());
                    let start = quick_xml::events::BytesStart::from_content(content, name_len);
                    return T::from_xml(&mut self.reader, &start, is_empty).unwrap_or_default();
                }
                PropsOrInfo::Info(XmlInfo::ContainerStart(child_kind)) => {
                    let child_event = self.build_start_event(child_kind).into_owned();
                    self.stack.push(ContextFrame { kind: child_kind });
                    self.queue(child_event, None);
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::HyperlinkStart { rel_id }) => {
                    let ev = PmlEvent::StartHyperlink { rel_id: rel_id.map(Cow::Owned) };
                    self.stack.push(ContextFrame { kind: PmlStartKind::Hyperlink });
                    self.queue(ev, None);
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::Text(t)) => {
                    if !t.trim().is_empty() {
                        self.queue(PmlEvent::Text(Cow::Owned(t)), None);
                    }
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::End) => {
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::Eof) => {
                    self.done = true;
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::Leaf(_)) | PropsOrInfo::Info(XmlInfo::Other) => {
                    // continue scanning
                }
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

fn end_event_for(kind: PmlStartKind) -> PmlEvent<'static> {
    match kind {
        PmlStartKind::Shape => PmlEvent::EndShape,
        PmlStartKind::GraphicFrame => PmlEvent::EndGraphicFrame,
        PmlStartKind::Table => PmlEvent::EndTable,
        PmlStartKind::TableRow => PmlEvent::EndTableRow,
        PmlStartKind::TableCell => PmlEvent::EndTableCell,
        PmlStartKind::Paragraph => PmlEvent::EndParagraph,
        PmlStartKind::Run => PmlEvent::EndRun,
        PmlStartKind::Hyperlink => PmlEvent::EndHyperlink,
    }
}

fn attr_string(e: &quick_xml::events::BytesStart<'_>, qname: &[u8]) -> Option<String> {
    for attr in e.attributes().filter_map(|a| a.ok()) {
        if attr.key.as_ref() == qname {
            return Some(String::from_utf8_lossy(&attr.value).into_owned());
        }
    }
    None
}
