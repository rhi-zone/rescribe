//! True SAX-based streaming iterator for DOCX WordprocessingML.
//!
//! `WmlEventIter` wraps a `quick_xml::Reader<&[u8]>` and emits [`WmlEvent`]
//! items without materialising the full document tree.
//!
//! # Memory model
//!
//! Memory usage is O(nesting depth + largest props element).  The only
//! buffering that occurs is reading a single `pPr`/`rPr`/`tblPr`/… element
//! so its typed props struct can be returned on the matching
//! `StartParagraph`/`StartRun`/… event.  The rest of the document is never
//! held in memory simultaneously.

use std::borrow::Cow;

use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;

use super::generated::{
    ParagraphProperties, RunProperties, TableCellProperties, TableProperties, TableRowProperties,
};
use super::generated_events::{WmlEvent, WmlStartKind, dispatch_start, is_text_element};
use ooxml_xml::FromXml;

/// Return a streaming iterator over the WML events in the given XML bytes.
///
/// `bytes` should be the raw content of `word/document.xml` extracted from
/// the DOCX zip.
pub fn events(bytes: &[u8]) -> WmlEventIter<'_> {
    WmlEventIter::new(bytes)
}

// ---------------------------------------------------------------------------
// Internal extracted-event type (owned, no lifetime on buf)
// ---------------------------------------------------------------------------

/// What we extract from a raw quick-xml event before dropping the buf borrow.
enum XmlInfo {
    /// A tracked container start (e.g. `<w:p>`).
    ContainerStart(WmlStartKind),
    /// A tracked leaf empty element (e.g. `<w:br/>`).
    Leaf(WmlEvent<'static>),
    /// Hyperlink start — carries owned attribute values.
    HyperlinkStart {
        rel_id: Option<String>,
        anchor: Option<String>,
    },
    /// An end tag.
    End,
    /// Text content.
    Text(String),
    /// End of file.
    Eof,
    /// Anything else we don't need to track.
    Other,
}

// ---------------------------------------------------------------------------
// WmlEventIter
// ---------------------------------------------------------------------------

type WmlEventOwned = WmlEvent<'static>;

/// Context entry for the nesting stack.
#[derive(Debug)]
struct ContextFrame {
    kind: WmlStartKind,
}

/// True streaming DOCX event iterator.
pub struct WmlEventIter<'input> {
    reader: Reader<&'input [u8]>,
    buf: Vec<u8>,
    /// Nesting stack of container elements we have opened.
    stack: Vec<ContextFrame>,
    /// Up to two events queued ahead of the main read loop.
    pending: [Option<WmlEventOwned>; 2],
    /// True until the first event (StartDocument) has been yielded.
    started: bool,
    /// True once we have hit Eof or an unrecoverable error.
    done: bool,
}

impl<'input> WmlEventIter<'input> {
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

    /// Queue at most two events to be returned before resuming the main loop.
    fn queue(&mut self, first: WmlEventOwned, second: Option<WmlEventOwned>) {
        self.pending[0] = Some(first);
        self.pending[1] = second;
    }

    /// Drain the front of the pending queue.
    fn drain_pending(&mut self) -> Option<WmlEventOwned> {
        if let Some(e) = self.pending[0].take() {
            self.pending[0] = self.pending[1].take();
            Some(e)
        } else {
            None
        }
    }
}

impl<'input> Iterator for WmlEventIter<'input> {
    type Item = WmlEvent<'input>;

    fn next(&mut self) -> Option<Self::Item> {
        // Drain pending queue first.
        if let Some(e) = self.drain_pending() {
            return Some(e);
        }

        // Emit StartDocument on first call.
        if !self.started {
            self.started = true;
            return Some(WmlEvent::StartDocument);
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
                    // build_start_event may have enqueued a pending event;
                    // drain it after returning start_event.
                    return Some(start_event);
                }
                XmlInfo::HyperlinkStart { rel_id, anchor } => {
                    self.stack.push(ContextFrame { kind: WmlStartKind::Hyperlink });
                    return Some(WmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                        anchor: anchor.map(Cow::Owned),
                    });
                }
                XmlInfo::Leaf(e) => return Some(e),
                XmlInfo::End => {
                    if let Some(frame) = self.stack.pop() {
                        return Some(end_event_for(frame.kind));
                    }
                    // End tag without matching open frame — ignore.
                }
                XmlInfo::Text(t) => {
                    if !t.is_empty() {
                        return Some(WmlEvent::Text(Cow::Owned(t)));
                    }
                }
                XmlInfo::Eof => {
                    self.done = true;
                    return Some(WmlEvent::EndDocument);
                }
                XmlInfo::Other => {
                    // Drain any pending queued by a previous call (e.g. from
                    // read_props_child hitting an end-tag early).
                    if let Some(e) = self.drain_pending() {
                        return Some(e);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// XML reading (phase 1: extract info, drop borrow)
// ---------------------------------------------------------------------------

impl<'input> WmlEventIter<'input> {
    /// Read the next raw XML event and extract all needed info as owned data.
    /// After this returns, `self.buf` is no longer borrowed.
    fn read_xml_info(&mut self) -> XmlInfo {
        self.buf.clear();
        match self.reader.read_event_into(&mut self.buf) {
            Ok(XmlEvent::Start(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == b"hyperlink" {
                    let rel_id = attr_string(e, b"r:id");
                    let anchor = attr_string(e, b"w:anchor");
                    return XmlInfo::HyperlinkStart { rel_id, anchor };
                }
                if let Some(kind) = dispatch_start(&local) {
                    return XmlInfo::ContainerStart(kind);
                }
                // Text-content element (e.g. <w:t>): read its text and return it.
                if is_text_element(&local) {
                    let text = read_text_content(&mut self.reader);
                    return XmlInfo::Text(text);
                }
                // Untracked start — skip entire element.
                skip_element(&mut self.reader);
                XmlInfo::Other
            }
            Ok(XmlEvent::Empty(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if let Some(event) = build_leaf_event_owned(&local, e) {
                    return XmlInfo::Leaf(event);
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

    /// Same as `read_xml_info` but also accepts a props element with the given
    /// local name.  Returns the raw bytes of the props element start tag (or
    /// None if we hit something else first).
    fn read_xml_info_or_props(&mut self, props_local: &[u8]) -> PropsOrInfo {
        self.buf.clear();
        match self.reader.read_event_into(&mut self.buf) {
            Ok(XmlEvent::Start(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == props_local {
                    // This IS the props element — signal caller to parse it.
                    return PropsOrInfo::IsProps { is_empty: false };
                }
                if local == b"hyperlink" {
                    let rel_id = attr_string(e, b"r:id");
                    let anchor = attr_string(e, b"w:anchor");
                    return PropsOrInfo::Info(XmlInfo::HyperlinkStart { rel_id, anchor });
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
                if let Some(event) = build_leaf_event_owned(&local, e) {
                    return PropsOrInfo::Info(XmlInfo::Leaf(event));
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
    /// The next element is the props element; the caller should call FromXml.
    IsProps { is_empty: bool },
    /// Something else came first.
    Info(XmlInfo),
}

// ---------------------------------------------------------------------------
// Container start-event construction (phase 2: no buf borrow)
// ---------------------------------------------------------------------------

impl<'input> WmlEventIter<'input> {
    /// Build the typed `StartXxx` event.  For containers with a props child
    /// (`pPr`, `rPr`, …), this reads and parses that child first.
    fn build_start_event(&mut self, kind: WmlStartKind) -> WmlEvent<'static> {
        match kind {
            WmlStartKind::Paragraph => {
                let props = self.read_props::<ParagraphProperties>(b"pPr");
                WmlEvent::StartParagraph { props: Box::new(props) }
            }
            WmlStartKind::Run => {
                let props = self.read_props::<RunProperties>(b"rPr");
                WmlEvent::StartRun { props: Box::new(props) }
            }
            WmlStartKind::Table => {
                let props = self.read_props::<TableProperties>(b"tblPr");
                WmlEvent::StartTable { props: Box::new(props) }
            }
            WmlStartKind::TableRow => {
                let props = self.read_props::<TableRowProperties>(b"trPr");
                WmlEvent::StartTableRow { props: Box::new(props) }
            }
            WmlStartKind::TableCell => {
                let props = self.read_props::<TableCellProperties>(b"tcPr");
                WmlEvent::StartTableCell { props: Box::new(props) }
            }
            // Hyperlink is handled separately (attrs on the element itself).
            WmlStartKind::Hyperlink => unreachable!(),
        }
    }

    /// Scan ahead for the props child element.
    ///
    /// If the very next event is the expected props element, parse and return it.
    /// Otherwise queue the event for normal processing and return `T::default()`.
    fn read_props<T: FromXml + Default>(&mut self, expected_local: &[u8]) -> T {
        loop {
            match self.read_xml_info_or_props(expected_local) {
                PropsOrInfo::IsProps { is_empty } => {
                    // self.buf still contains the props start tag bytes.
                    // Build a temporary reader over those bytes to give FromXml
                    // a BytesStart it can inspect.
                    //
                    // In practice: we re-read from self.reader for child elements.
                    // quick-xml's Reader<&[u8]> doesn't let us "unread", but we
                    // can reconstruct a BytesStart from self.buf if needed.
                    // The generated FromXml impls call from_xml(reader, start, is_empty)
                    // where they use `start` only for attribute parsing.
                    //
                    // We build a minimal BytesStart from the buf slice.
                    let tag_bytes = self.buf.clone();
                    let start = quick_xml::events::BytesStart::from_content(
                        std::str::from_utf8(&tag_bytes).unwrap_or(""),
                        0,
                    );
                    match T::from_xml(&mut self.reader, &start, is_empty) {
                        Ok(props) => return props,
                        Err(_) => return T::default(),
                    }
                }
                PropsOrInfo::Info(XmlInfo::ContainerStart(child_kind)) => {
                    // A tracked child started before we saw the props element.
                    let child_event = self.build_start_event(child_kind).into_owned();
                    self.stack.push(ContextFrame { kind: child_kind });
                    self.queue(child_event, None);
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::HyperlinkStart { rel_id, anchor }) => {
                    let ev = WmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                        anchor: anchor.map(Cow::Owned),
                    };
                    self.stack.push(ContextFrame { kind: WmlStartKind::Hyperlink });
                    self.queue(ev, None);
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::Leaf(ev)) => {
                    self.queue(ev, None);
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::End) => {
                    // Container closed before props were seen.
                    // Queue the end event (the caller will push the frame first).
                    // We signal this with a sentinel: push EndXxx after the Start.
                    // Since we haven't pushed the frame yet, we use a placeholder.
                    // Simplest: return default and let the next End tag pop the frame.
                    // But we consumed the End already — queue it.
                    let end = end_event_for_opt(self.stack.last().map(|f| f.kind));
                    if let Some(e) = end {
                        self.queue(e, None);
                    }
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::Text(t)) => {
                    if !t.trim().is_empty() {
                        self.queue(WmlEvent::Text(Cow::Owned(t)), None);
                    }
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::Eof) => {
                    self.done = true;
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::Other) => {
                    // Continue scanning.
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (no self borrow)
// ---------------------------------------------------------------------------

/// Strip the namespace prefix: `b"w:p"` → `b"p"` (returns owned Vec).
fn local_name_owned(raw: &[u8]) -> Vec<u8> {
    raw.iter()
        .position(|&b| b == b':')
        .map_or_else(|| raw.to_vec(), |i| raw[i + 1..].to_vec())
}

/// Read all text content of an already-opened element until its end tag.
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

/// Skip an open element and all its children.
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

/// Build the `EndXxx` event for a container kind.
fn end_event_for(kind: WmlStartKind) -> WmlEvent<'static> {
    match kind {
        WmlStartKind::Paragraph => WmlEvent::EndParagraph,
        WmlStartKind::Run => WmlEvent::EndRun,
        WmlStartKind::Table => WmlEvent::EndTable,
        WmlStartKind::TableRow => WmlEvent::EndTableRow,
        WmlStartKind::TableCell => WmlEvent::EndTableCell,
        WmlStartKind::Hyperlink => WmlEvent::EndHyperlink,
    }
}

fn end_event_for_opt(kind: Option<WmlStartKind>) -> Option<WmlEvent<'static>> {
    kind.map(end_event_for)
}

/// Build an owned leaf event from an empty element, if tracked.
fn build_leaf_event_owned(
    local: &[u8],
    e: &quick_xml::events::BytesStart<'_>,
) -> Option<WmlEvent<'static>> {
    match local {
        b"br" => Some(WmlEvent::LineBreak),
        b"footnoteReference" => {
            let id = attr_i32(e, b"w:id").unwrap_or(0);
            Some(WmlEvent::FootnoteRef { id })
        }
        b"endnoteReference" => {
            let id = attr_i32(e, b"w:id").unwrap_or(0);
            Some(WmlEvent::EndnoteRef { id })
        }
        b"blip" => {
            let rel_id = attr_string(e, b"r:embed")
                .map(Cow::Owned)
                .unwrap_or_default();
            Some(WmlEvent::Image { rel_id })
        }
        _ => None,
    }
}

/// Get an attribute value as an owned String.
fn attr_string(e: &quick_xml::events::BytesStart<'_>, qname: &[u8]) -> Option<String> {
    for attr in e.attributes().filter_map(|a| a.ok()) {
        if attr.key.as_ref() == qname {
            return Some(String::from_utf8_lossy(&attr.value).into_owned());
        }
    }
    None
}

/// Get an attribute value parsed as `i32`.
fn attr_i32(e: &quick_xml::events::BytesStart<'_>, qname: &[u8]) -> Option<i32> {
    for attr in e.attributes().filter_map(|a| a.ok()) {
        if attr.key.as_ref() == qname {
            let s = std::str::from_utf8(&attr.value).ok()?;
            return s.parse().ok();
        }
    }
    None
}
