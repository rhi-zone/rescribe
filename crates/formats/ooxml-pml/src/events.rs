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
use std::collections::VecDeque;

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use super::generated_events::{
    PmlEvent, PmlStartKind, ShapeGeometry, ShapeTransform, dispatch_start, is_text_element,
};
use ooxml_dml::types::{
    CTTableCellProperties, CTTableProperties, TextCharacterProperties, TextParagraphProperties,
};
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
    /// A structural wrapper we descend into without emitting an event (e.g.
    /// `<p:txBody>`, whose actual content — `<a:p>` paragraphs — is what
    /// `dispatch_start` tracks).
    TransparentStart,
    HyperlinkStart {
        rel_id: Option<String>,
    },
    Leaf(PmlEvent<'static>),
    End,
    Text(String),
    Eof,
    Other,
}

/// Element local names that wrap tracked content and must be descended into
/// rather than skipped. `events()` takes the raw content of a
/// `ppt/slides/slideN.xml` part, so `<p:sld>`/`<p:cSld>`/`<p:spTree>` are the
/// document's own root structure (every `<p:sp>` on the slide lives inside
/// them) and `<p:txBody>` wraps a shape's/cell's paragraphs. None of these
/// have a dedicated `PmlEvent`; if they are not descended into (previously:
/// they fell through to `skip_element()`, since none is a tracked
/// container), the entire paragraph/run/text subtree beneath them — i.e. all
/// slide text — is silently dropped.
pub(crate) fn is_transparent_wrapper(local: &[u8]) -> bool {
    matches!(local, b"sld" | b"cSld" | b"spTree" | b"txBody")
}

// ---------------------------------------------------------------------------
// PmlEventIter
// ---------------------------------------------------------------------------

type PmlEventOwned = PmlEvent<'static>;

/// Context entry for the nesting stack.
#[derive(Debug, Clone, Copy)]
enum ContextFrame {
    /// A container that produced a `Start…` event and owes an `End…`.
    Tracked(PmlStartKind),
    /// A wrapper descended into silently; its end tag produces nothing.
    Transparent,
}

// A handful of pure, Reader-agnostic helpers below (`local_name_owned`,
// `attr_string`, `end_event_for`, `is_transparent_wrapper`) are `pub(crate)`
// so `crate::chunked_events` (the independent chunk-resumable tokenizer used
// by `StreamingParser<H>`) can call them as plain functions instead of
// duplicating them — see CLAUDE.md's "three independent implementations,
// not derived from one another" rule: implementations may share pure
// state-transition helpers, just not derive from each other's control flow.
// `skip_element`/`read_text_content` are deliberately NOT shared: their
// `Eof`/`Err` handling silently treats "no more bytes right now" as "done",
// which is correct for a fixed complete buffer but wrong for a genuinely
// truncated chunk — `chunked_events` needs its own variants that can instead
// report "need more input" and retry.

/// True streaming PPTX event iterator.
pub struct PmlEventIter<'input> {
    reader: Reader<&'input [u8]>,
    buf: Vec<u8>,
    stack: Vec<ContextFrame>,
    /// Events queued ahead of the main read loop. See `queue` for why this
    /// must be prepended to, not overwritten.
    pending: VecDeque<PmlEventOwned>,
    started: bool,
    done: bool,
    /// Set when a container's own end tag is consumed while scanning ahead
    /// for its content (`read_shape_transform` finding `</p:sp>` before
    /// `<p:spPr>`, or `read_props` finding the container's own `End` before
    /// its props element). Signals `open()` to pop the frame it
    /// speculatively pushed, since the matching `End…` event has already
    /// been queued and no end tag remains in the stream to pop it.
    close_immediately: bool,
}

impl<'input> PmlEventIter<'input> {
    pub fn new(bytes: &'input [u8]) -> Self {
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(false);
        Self {
            reader,
            buf: Vec::with_capacity(512),
            stack: Vec::new(),
            pending: VecDeque::new(),
            started: false,
            done: false,
            close_immediately: false,
        }
    }

    fn drain_pending(&mut self) -> Option<PmlEventOwned> {
        self.pending.pop_front()
    }

    /// Prepend an event to the pending queue — see `WmlEventIter::queue` in
    /// ooxml-wml's `events.rs` for the full rationale: a nested `open()`/
    /// `read_props` call may already have queued an event (e.g. `Text`)
    /// before this call's own event is queued, and that already-queued
    /// event is textually *after* this one, so overwriting instead of
    /// prepending would silently drop it.
    fn queue(&mut self, event: PmlEventOwned) {
        self.pending.push_front(event);
    }

    /// Build the `Start…` event for a container and push its frame, unless
    /// scanning ahead for its content already consumed its own end tag.
    ///
    /// The frame is pushed *before* `build_start_event` runs so that a
    /// nested container found before this one's expected content (e.g. a
    /// `<p:r>` found while scanning `<p:p>` for `<a:pPr>`) pushes its own
    /// frame *after* this one, keeping `End` tags popping in the right
    /// order.
    fn open(&mut self, kind: PmlStartKind) -> PmlEventOwned {
        self.stack.push(ContextFrame::Tracked(kind));
        let ev = self.build_start_event(kind);
        if self.close_immediately {
            self.close_immediately = false;
            self.stack.pop();
        }
        ev
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
                    return Some(self.open(kind));
                }
                XmlInfo::TransparentStart => {
                    self.stack.push(ContextFrame::Transparent);
                }
                XmlInfo::HyperlinkStart { rel_id } => {
                    self.stack
                        .push(ContextFrame::Tracked(PmlStartKind::Hyperlink));
                    return Some(PmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                    });
                }
                XmlInfo::Leaf(e) => return Some(e),
                XmlInfo::End => match self.stack.pop() {
                    Some(ContextFrame::Tracked(kind)) => return Some(end_event_for(kind)),
                    Some(ContextFrame::Transparent) | None => {}
                },
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
                if is_transparent_wrapper(&local) {
                    return XmlInfo::TransparentStart;
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
                if is_transparent_wrapper(&local) {
                    return PropsOrInfo::Info(XmlInfo::TransparentStart);
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
                let props = self.read_props::<TextParagraphProperties>(b"pPr", kind);
                PmlEvent::StartParagraph {
                    props: Box::new(props),
                }
            }
            PmlStartKind::Run => {
                let props = self.read_props::<TextCharacterProperties>(b"rPr", kind);
                PmlEvent::StartRun {
                    props: Box::new(props),
                }
            }
            PmlStartKind::Table => {
                let props = self.read_props::<CTTableProperties>(b"tblPr", kind);
                PmlEvent::StartTable {
                    props: Box::new(props),
                }
            }
            PmlStartKind::TableCell => {
                let props = self.read_props::<CTTableCellProperties>(b"tcPr", kind);
                PmlEvent::StartTableCell {
                    props: Box::new(props),
                }
            }
            PmlStartKind::Shape => {
                let (transform, geometry) = self.read_shape_transform();
                PmlEvent::StartShape {
                    transform,
                    geometry,
                }
            }
            PmlStartKind::GraphicFrame => PmlEvent::StartGraphicFrame,
            PmlStartKind::TableRow => PmlEvent::StartTableRow,
            PmlStartKind::Hyperlink => unreachable!(),
        }
    }

    /// Scan children of `<p:sp>` for `<p:spPr>` and extract `<a:xfrm>` geometry
    /// plus the shape's outline (`<a:prstGeom>` or `<a:custGeom>`).
    ///
    /// Consumes children up to and including `<p:spPr>`. Remaining children
    /// (`<p:style>`, `<p:txBody>`, etc.) are left for the normal SAX loop.
    /// On malformed input where `</p:sp>` is consumed before `<p:spPr>` is found,
    /// queues the matching `EndShape` and sets `self.close_immediately` so
    /// `open()` pops the frame it speculatively pushed.
    ///
    /// Uses the same `read_xml_info_or_props` scan as `read_props` (rather
    /// than an unconditional `skip_element` on every non-`spPr` child) so
    /// that a shape without `<p:spPr>` — schema-valid content can omit it —
    /// does not have its `<p:txBody>` (or any other tracked/transparent
    /// child) silently discarded before the normal SAX loop ever sees it.
    fn read_shape_transform(&mut self) -> (Option<ShapeTransform>, Option<ShapeGeometry>) {
        loop {
            match self.read_xml_info_or_props(b"spPr") {
                PropsOrInfo::IsProps { is_empty } => {
                    if is_empty {
                        return (None, None);
                    }
                    return self.extract_xfrm_from_sppr();
                }
                PropsOrInfo::Info(XmlInfo::TransparentStart) => {
                    self.stack.push(ContextFrame::Transparent);
                    return (None, None);
                }
                PropsOrInfo::Info(XmlInfo::ContainerStart(child_kind)) => {
                    let child_event = self.open(child_kind);
                    self.queue(child_event);
                    return (None, None);
                }
                PropsOrInfo::Info(XmlInfo::HyperlinkStart { rel_id }) => {
                    let ev = PmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                    };
                    self.stack
                        .push(ContextFrame::Tracked(PmlStartKind::Hyperlink));
                    self.queue(ev);
                    return (None, None);
                }
                PropsOrInfo::Info(XmlInfo::Text(t)) => {
                    if t.trim().is_empty() {
                        // Inter-element whitespace from pretty-printed XML —
                        // not real content, keep scanning for <p:spPr> or a
                        // tracked child.
                    } else {
                        self.queue(PmlEvent::Text(Cow::Owned(t)));
                        return (None, None);
                    }
                }
                PropsOrInfo::Info(XmlInfo::End) => {
                    // Consumed </p:sp> before any <p:spPr> — queue the
                    // matching EndShape now, since no end tag remains in the
                    // stream to trigger it later.
                    self.queue(end_event_for(PmlStartKind::Shape));
                    self.close_immediately = true;
                    return (None, None);
                }
                PropsOrInfo::Info(XmlInfo::Eof) => {
                    self.done = true;
                    return (None, None);
                }
                PropsOrInfo::Info(XmlInfo::Leaf(_)) | PropsOrInfo::Info(XmlInfo::Other) => {
                    // continue scanning
                }
            }
        }
    }

    /// Read the contents of an already-opened `<p:spPr>` element and extract
    /// the `<a:off>`/`<a:ext>` attributes from `<a:xfrm>`, plus the
    /// `<a:prstGeom>`/`<a:custGeom>` outline.
    fn extract_xfrm_from_sppr(&mut self) -> (Option<ShapeTransform>, Option<ShapeGeometry>) {
        let mut x: Option<i64> = None;
        let mut y: Option<i64> = None;
        let mut cx: Option<i64> = None;
        let mut cy: Option<i64> = None;
        let mut geometry: Option<ShapeGeometry> = None;
        let mut depth = 1u32;
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match self.reader.read_event_into(&mut buf) {
                Ok(XmlEvent::Start(ref e)) => {
                    let local = local_name_owned(e.local_name().as_ref());
                    if local == b"prstGeom" {
                        // `attr_string` only needs `e`; its borrow ends here,
                        // freeing `self.reader`/`self.buf` for the recursive scan.
                        let preset = attr_string(e, b"prst").unwrap_or_default();
                        let adjustments = self.read_av_lst_to_end();
                        geometry = Some(ShapeGeometry::Preset {
                            preset,
                            adjustments,
                        });
                    } else if local == b"custGeom" {
                        match ooxml_xml::RawXmlElement::from_reader(&mut self.reader, e) {
                            Ok(elem) => geometry = Some(ShapeGeometry::Custom(elem)),
                            Err(_) => {
                                self.done = true;
                                break;
                            }
                        }
                    } else {
                        depth += 1;
                    }
                }
                Ok(XmlEvent::Empty(ref e)) => {
                    let local = local_name_owned(e.local_name().as_ref());
                    if local == b"off" {
                        x = attr_string(e, b"x").and_then(|s| s.parse().ok());
                        y = attr_string(e, b"y").and_then(|s| s.parse().ok());
                    } else if local == b"ext" {
                        cx = attr_string(e, b"cx").and_then(|s| s.parse().ok());
                        cy = attr_string(e, b"cy").and_then(|s| s.parse().ok());
                    } else if local == b"prstGeom" {
                        // Self-closing prstGeom — no <a:avLst> children.
                        let preset = attr_string(e, b"prst").unwrap_or_default();
                        geometry = Some(ShapeGeometry::Preset {
                            preset,
                            adjustments: Vec::new(),
                        });
                    }
                }
                Ok(XmlEvent::End(_)) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Ok(XmlEvent::Eof) | Err(_) => {
                    self.done = true;
                    break;
                }
                _ => {}
            }
        }
        let transform = match (x, y, cx, cy) {
            (Some(x), Some(y), Some(cx), Some(cy)) => Some(ShapeTransform { x, y, cx, cy }),
            _ => None,
        };
        (transform, geometry)
    }

    /// Read `<a:avLst>/<a:gd>` adjustment pairs through the matching
    /// `</a:prstGeom>` end tag. Called with the reader positioned just after
    /// `<a:prstGeom ...>` was consumed as a `Start` event (so depth 1 = still
    /// inside `prstGeom`).
    fn read_av_lst_to_end(&mut self) -> Vec<(String, String)> {
        let mut adjustments = Vec::new();
        let mut depth = 1u32;
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match self.reader.read_event_into(&mut buf) {
                Ok(XmlEvent::Start(_)) => depth += 1,
                Ok(XmlEvent::Empty(ref e)) => {
                    let local = local_name_owned(e.local_name().as_ref());
                    if local == b"gd" {
                        let name = attr_string(e, b"name").unwrap_or_default();
                        let fmla = attr_string(e, b"fmla").unwrap_or_default();
                        adjustments.push((name, fmla));
                    }
                }
                Ok(XmlEvent::End(_)) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Ok(XmlEvent::Eof) | Err(_) => {
                    self.done = true;
                    break;
                }
                _ => {}
            }
        }
        adjustments
    }

    /// Scan ahead for the props child element.
    ///
    /// If the very next event is the expected props element, parse and
    /// return it. Otherwise queue the event for normal processing and
    /// return `T::default()`. `owner` is the container this props scan is
    /// for, needed to build the correct `End…` event if `owner`'s own end
    /// tag is hit before any props element appears.
    fn read_props<T: FromXml + Default>(
        &mut self,
        expected_local: &[u8],
        owner: PmlStartKind,
    ) -> T {
        loop {
            match self.read_xml_info_or_props(expected_local) {
                PropsOrInfo::IsProps { is_empty } => {
                    let tag_bytes = self.buf.clone();
                    let tag_str = std::str::from_utf8(&tag_bytes).unwrap_or("");
                    let content = tag_str
                        .trim_start_matches('<')
                        .trim_end_matches('>')
                        .trim_end_matches('/');
                    let name_len = content
                        .bytes()
                        .position(|b| b == b' ')
                        .unwrap_or(content.len());
                    let start = quick_xml::events::BytesStart::from_content(content, name_len);
                    return T::from_xml(&mut self.reader, &start, is_empty).unwrap_or_default();
                }
                PropsOrInfo::Info(XmlInfo::TransparentStart) => {
                    self.stack.push(ContextFrame::Transparent);
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::ContainerStart(child_kind)) => {
                    // A tracked child started before we saw the props
                    // element; `open` handles pushing its frame (and any of
                    // its own further-nested children) in the right order.
                    let child_event = self.open(child_kind);
                    self.queue(child_event);
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::HyperlinkStart { rel_id }) => {
                    let ev = PmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                    };
                    self.stack
                        .push(ContextFrame::Tracked(PmlStartKind::Hyperlink));
                    self.queue(ev);
                    return T::default();
                }
                PropsOrInfo::Info(XmlInfo::Text(t)) => {
                    if t.trim().is_empty() {
                        // Inter-element whitespace from pretty-printed XML —
                        // not real content, keep scanning for the props
                        // element or a tracked child.
                    } else {
                        self.queue(PmlEvent::Text(Cow::Owned(t)));
                        return T::default();
                    }
                }
                PropsOrInfo::Info(XmlInfo::End) => {
                    // The container closed before any props element
                    // appeared. Its own end tag has already been consumed,
                    // so queue the matching `End…` and tell `open` not to
                    // leave a frame that nothing would ever pop.
                    self.queue(end_event_for(owner));
                    self.close_immediately = true;
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

pub(crate) fn local_name_owned(raw: &[u8]) -> Vec<u8> {
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

pub(crate) fn end_event_for(kind: PmlStartKind) -> PmlEvent<'static> {
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

pub(crate) fn attr_string(e: &quick_xml::events::BytesStart<'_>, qname: &[u8]) -> Option<String> {
    for attr in e.attributes().filter_map(|a| a.ok()) {
        if attr.key.as_ref() == qname {
            return Some(String::from_utf8_lossy(&attr.value).into_owned());
        }
    }
    None
}
