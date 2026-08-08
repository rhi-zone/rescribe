//! Chunk-resumable XML tokenizer producing [`OwnedPmlEvent`]s, used by
//! [`crate::batch::StreamingParser`] to translate one slide part's bytes as
//! they arrive instead of requiring the whole slide buffered first.
//!
//! This is a genuinely independent implementation from [`crate::events`]'s
//! [`crate::events::PmlEventIter`] (a pull `Iterator` over a complete
//! `&[u8]`) — not derived from it, per CLAUDE.md's "three independent
//! implementations" rule. It reuses only the small set of pure,
//! `Reader`-agnostic helpers `events.rs` already exposes as `pub(crate)`
//! (`local_name_owned`, `attr_string`, `end_event_for`,
//! `is_transparent_wrapper`) as plain function calls; every part that reads
//! bytes off a `quick_xml::Reader` and must distinguish "genuinely done"
//! from "just ran out of currently-buffered bytes" is reimplemented here.
//!
//! # Technique
//!
//! Same core insight as `docbook-fmt::batch` (see that crate's module
//! docs): `quick_xml::Reader` reports a truncated tag/comment/CDATA/PI as
//! `Err(Syntax(Unclosed*))` rather than silently treating it as finished,
//! so those tokens are unambiguous. The one ambiguous low-level token is
//! plain text (terminated by `<` *or* by end-of-buffer, indistinguishably),
//! resolved the same way docbook-fmt does: a `Text` token is only accepted
//! when it did *not* consume all currently-buffered bytes (i.e. it hit a
//! real `<`), or when the caller has signalled `is_final` (no more input is
//! coming, so end-of-buffer means the text run really is over).
//!
//! `PmlEventIter` generalizes past single tokens, though: producing one
//! `PmlEvent` can require **multi-token lookahead** — `read_props` scans
//! forward through several sibling elements before it knows whether a
//! `<p:pPr>`/`<p:rPr>`/etc. child is present, and `read_shape_transform`
//! does the same for `<p:spPr>`. This module extends the docbook-fmt
//! technique to that shape: each attempt to produce the *next* event is
//! retried as a whole from a **fresh, disposable** `Reader` built over a
//! clone of the still-unconsumed bytes; if at any point during that attempt
//! a token is ambiguous or incomplete, the entire attempt is abandoned
//! without committing any of its provisional state (`stack` is restored
//! from a clone taken before the attempt started — a push-then-pop
//! sequence within one failed attempt means a length-only rollback isn't
//! enough — and `queue`/`close_immediately`, which are always
//! attempt-local, are simply cleared/reset), and `drain()` waits for more
//! input. Only a *fully successful* attempt commits: it advances the
//! consumed-bytes offset and appends to `stack`/`queue` for real.
//! Constructing a fresh `Reader` (and cloning the small unconsumed-bytes
//! window) is cheap; a failed attempt may redo work already done on a
//! previous `feed()` call, trading some redundant CPU for a simple,
//! obviously-correct rollback — same trade-off
//! `docbook-fmt::batch::StreamingParser::drain` makes.
//!
//! Every fresh `Reader` this module constructs also disables
//! `check_end_names`/`allow_unmatched_ends`: a reader built over just the
//! unconsumed tail never saw the `Start` tags a *previous* attempt's reader
//! already consumed, so quick_xml's own start/end name validation would
//! otherwise reject the very first `End` tag in a fresh window as
//! mismatched. Tag balance is instead enforced by hand via `stack`, which
//! *does* persist across attempts — the exact fix
//! `docbook-fmt::batch::StreamingParser` applies for the same reason (see
//! its module doc comment and `open_stack` field).
//!
//! One extra wrinkle beyond docbook-fmt: the generated `FromXml` parsers
//! for `pPr`/`rPr`/`tblPr`/`tcPr` (`ooxml-dml`'s codegen output) treat a
//! `quick_xml::events::Event::Eof` mid-element as "done" and return
//! whatever fields were parsed so far — *not* an error — because that is
//! correct when parsing a known-complete buffer (their only caller before
//! this module) but would silently produce a truncated/wrong properties
//! struct if called against a chunk-boundary-truncated buffer. This module
//! never calls into a generated `FromXml` impl (or `RawXmlElement::from_reader`
//! for `<a:custGeom>`, which *does* already return `Err` on a genuine `Eof`)
//! against possibly-incomplete bytes: [`element_fully_buffered`] does an
//! inexpensive balanced-tag pre-scan first, and the attempt is treated as
//! "need more input" unless the whole child element is already present.
//!
//! # Memory model
//!
//! `O(nesting depth + largest token + largest props/geometry element)` —
//! `pending` holds only bytes not yet proven to belong to a completed
//! event; `stack` is one frame per open tracked/transparent element.

use std::borrow::Cow;
use std::collections::VecDeque;

use quick_xml::Reader;
use quick_xml::events::BytesStart;
use quick_xml::events::Event as XmlEvent;

use crate::events::{attr_string, end_event_for, is_transparent_wrapper, local_name_owned};
use crate::generated_events::{
    OwnedPmlEvent, PmlEvent, PmlStartKind, ShapeGeometry, ShapeTransform, dispatch_start,
    is_text_element,
};
use ooxml_dml::types::{
    CTTableCellProperties, CTTableProperties, TextCharacterProperties, TextParagraphProperties,
};
use ooxml_xml::FromXml;

/// Sentinel meaning "the currently-buffered bytes do not yet prove the next
/// event complete" — the whole in-progress attempt is abandoned and retried
/// after more input arrives (or, at `finish()`, resolved as final).
struct Need;

/// Frame on the open-element context stack, mirroring
/// `crate::events::ContextFrame` (duplicated rather than shared: it is
/// trivial, `Copy`, and coupling the two independent implementations
/// through a shared type would violate the "not derived from one another"
/// rule for no real benefit).
#[derive(Debug, Clone, Copy)]
enum ContextFrame {
    Tracked(PmlStartKind),
    Transparent,
}

/// Low-level token classification, mirroring `crate::events::XmlInfo` in
/// spirit (same set of cases the outer state machine must react to) but
/// independently implemented against the chunk-aware reads below.
enum Info {
    ContainerStart(PmlStartKind),
    TransparentStart,
    HyperlinkStart { rel_id: Option<String> },
    Leaf(OwnedPmlEvent),
    End,
    Text(String),
    Eof,
    Other,
}

enum PropsOrInfo {
    IsProps { is_empty: bool, tag_bytes: Vec<u8> },
    Info(Info),
}

enum NextOutcome {
    Emit(OwnedPmlEvent),
    End,
}

/// Chunk-resumable tokenizer for one slide part's XML, producing the same
/// `OwnedPmlEvent` sequence `crate::events::events()` would for the
/// slide's complete bytes, without requiring them all up front.
pub(crate) struct ChunkedPmlTokenizer {
    /// Bytes fed so far but not yet proven to belong to a completed,
    /// dispatched event. Drained (prefix removed) as attempts succeed.
    pending: Vec<u8>,
    stack: Vec<ContextFrame>,
    /// Events already computed (via lookahead within a successful attempt)
    /// but not yet dispatched — drained before starting a new attempt, same
    /// "prepend, don't overwrite" ordering rationale as
    /// `PmlEventIter::queue`.
    queue: VecDeque<OwnedPmlEvent>,
    started: bool,
    done: bool,
    /// Attempt-local flag mirroring `PmlEventIter::close_immediately`: set
    /// when a props/spPr scan consumes the owning container's own end tag
    /// before finding what it was looking for. Always consumed (read and
    /// reset) synchronously within the same attempt that set it; reset to
    /// `false` on a rolled-back attempt for safety.
    close_immediately: bool,
}

impl ChunkedPmlTokenizer {
    pub(crate) fn new() -> Self {
        ChunkedPmlTokenizer {
            pending: Vec::new(),
            stack: Vec::new(),
            queue: VecDeque::new(),
            started: false,
            done: false,
            close_immediately: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Feed the next chunk of this slide's XML bytes, dispatching every
    /// event that can be proven complete from the bytes seen so far.
    pub(crate) fn feed(&mut self, chunk: &[u8], out: &mut dyn FnMut(OwnedPmlEvent)) {
        self.pending.extend_from_slice(chunk);
        self.drain(false, out);
    }

    /// Signal that no more bytes are coming for this slide: resolve any
    /// remaining ambiguous state as final and emit the closing
    /// `EndPresentation` event.
    pub(crate) fn finish(mut self, out: &mut dyn FnMut(OwnedPmlEvent)) {
        self.drain(true, out);
    }

    fn drain(&mut self, is_final: bool, out: &mut dyn FnMut(OwnedPmlEvent)) {
        loop {
            if self.done {
                return;
            }
            if let Some(ev) = self.queue.pop_front() {
                out(ev);
                continue;
            }
            if !self.started {
                self.started = true;
                out(PmlEvent::StartPresentation);
                continue;
            }
            match self.try_produce(is_final) {
                Ok(Some((event, consumed))) => {
                    self.pending.drain(0..consumed);
                    out(event);
                }
                Ok(None) => {
                    self.done = true;
                    out(PmlEvent::EndPresentation);
                    return;
                }
                Err(Need) => return,
            }
        }
    }

    /// Attempt to produce exactly one more event. On success, returns the
    /// event and how many bytes of `pending` it consumed (already reflected
    /// via a fresh `Reader`'s `buffer_position()`). On `Need`, `pending` is
    /// untouched and `stack`/`queue`/`close_immediately` are rolled back to
    /// their state before this attempt began.
    fn try_produce(&mut self, is_final: bool) -> Result<Option<(OwnedPmlEvent, usize)>, Need> {
        if self.pending.is_empty() {
            return if is_final { Ok(None) } else { Err(Need) };
        }
        // A failed attempt may have both pushed *and popped* frames before
        // hitting ambiguity (e.g. scanning through several closing tags
        // that only pop `Transparent` frames, none of them emittable, on
        // the way to something that turns out to be incomplete) — a
        // length-only rollback (`truncate`) would be a no-op once the
        // attempt has net-popped below the starting length, silently
        // losing already-committed frames. Clone-and-restore is correct
        // regardless of push/pop order within the attempt, and cheap:
        // `stack` is O(nesting depth).
        let saved_stack = self.stack.clone();
        // Fresh, disposable owned copy of the unconsumed bytes: lets this
        // attempt freely interleave `&mut self` (stack/queue mutation) with
        // `&mut Reader` calls without the reader's lifetime tying up
        // `self.pending` itself. Bounded by however much is currently
        // unconsumed, which — for the fast path this module targets — never
        // grows anywhere near the whole slide (see module docs / the
        // `pending_len` test).
        let window = self.pending.clone();
        let input = &window[..];
        let mut reader = Reader::from_reader(input);
        reader.config_mut().trim_text(false);
        // Each attempt builds a *fresh* `Reader` over just the unconsumed
        // tail, so it has no memory of `Start` tags a *previous* attempt's
        // reader already consumed — quick_xml's own start/end name
        // validation would otherwise reject the very first `End` tag in
        // this window as mismatched (it never saw the matching `Start`).
        // Tag balance is instead enforced by hand via `self.stack`, which
        // *does* persist across attempts — same fix docbook-fmt's
        // `StreamingParser::drain` applies for the identical reason (see
        // its module doc comment).
        reader.config_mut().check_end_names = false;
        reader.config_mut().allow_unmatched_ends = true;
        match self.run_next(&mut reader, input, is_final) {
            Ok(NextOutcome::Emit(event)) => {
                let consumed = reader.buffer_position() as usize;
                Ok(Some((event, consumed)))
            }
            Ok(NextOutcome::End) => Ok(None),
            Err(Need) => {
                self.stack = saved_stack;
                self.queue.clear();
                self.close_immediately = false;
                Err(Need)
            }
        }
    }

    /// One `PmlEventIter::next()`-equivalent cycle: loop over low-level
    /// tokens (via `read_xml_info`) until something is actually emittable,
    /// mirroring `PmlEventIter::next`'s own loop exactly.
    fn run_next(
        &mut self,
        reader: &mut Reader<&[u8]>,
        input: &[u8],
        is_final: bool,
    ) -> Result<NextOutcome, Need> {
        loop {
            match self.read_xml_info(reader, input, is_final)? {
                Info::ContainerStart(kind) => {
                    let ev = self.open(reader, input, is_final, kind)?;
                    return Ok(NextOutcome::Emit(ev));
                }
                Info::TransparentStart => {
                    self.stack.push(ContextFrame::Transparent);
                }
                Info::HyperlinkStart { rel_id } => {
                    self.stack
                        .push(ContextFrame::Tracked(PmlStartKind::Hyperlink));
                    return Ok(NextOutcome::Emit(PmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                    }));
                }
                Info::Leaf(e) => return Ok(NextOutcome::Emit(e)),
                Info::End => match self.stack.pop() {
                    Some(ContextFrame::Tracked(kind)) => {
                        return Ok(NextOutcome::Emit(end_event_for(kind)));
                    }
                    Some(ContextFrame::Transparent) | None => {}
                },
                Info::Text(t) => {
                    if !t.is_empty() {
                        return Ok(NextOutcome::Emit(PmlEvent::Text(Cow::Owned(t))));
                    }
                }
                Info::Eof => return Ok(NextOutcome::End),
                Info::Other => {}
            }
        }
    }

    fn open(
        &mut self,
        reader: &mut Reader<&[u8]>,
        input: &[u8],
        is_final: bool,
        kind: PmlStartKind,
    ) -> Result<OwnedPmlEvent, Need> {
        self.stack.push(ContextFrame::Tracked(kind));
        let ev = self.build_start_event(reader, input, is_final, kind)?;
        if self.close_immediately {
            self.close_immediately = false;
            self.stack.pop();
        }
        Ok(ev)
    }

    fn build_start_event(
        &mut self,
        reader: &mut Reader<&[u8]>,
        input: &[u8],
        is_final: bool,
        kind: PmlStartKind,
    ) -> Result<OwnedPmlEvent, Need> {
        match kind {
            PmlStartKind::Paragraph => {
                let props = self
                    .read_props::<TextParagraphProperties>(reader, input, is_final, b"pPr", kind)?;
                Ok(PmlEvent::StartParagraph {
                    props: Box::new(props),
                })
            }
            PmlStartKind::Run => {
                let props = self
                    .read_props::<TextCharacterProperties>(reader, input, is_final, b"rPr", kind)?;
                Ok(PmlEvent::StartRun {
                    props: Box::new(props),
                })
            }
            PmlStartKind::Table => {
                let props =
                    self.read_props::<CTTableProperties>(reader, input, is_final, b"tblPr", kind)?;
                Ok(PmlEvent::StartTable {
                    props: Box::new(props),
                })
            }
            PmlStartKind::TableCell => {
                let props = self
                    .read_props::<CTTableCellProperties>(reader, input, is_final, b"tcPr", kind)?;
                Ok(PmlEvent::StartTableCell {
                    props: Box::new(props),
                })
            }
            PmlStartKind::Shape => {
                let (transform, geometry) = self.read_shape_transform(reader, input, is_final)?;
                Ok(PmlEvent::StartShape {
                    transform,
                    geometry,
                })
            }
            PmlStartKind::GraphicFrame => Ok(PmlEvent::StartGraphicFrame),
            PmlStartKind::TableRow => Ok(PmlEvent::StartTableRow),
            PmlStartKind::Hyperlink => unreachable!(),
        }
    }

    // -----------------------------------------------------------------
    // Low-level, chunk-aware token reads
    // -----------------------------------------------------------------

    fn read_xml_info(
        &mut self,
        reader: &mut Reader<&[u8]>,
        input: &[u8],
        is_final: bool,
    ) -> Result<Info, Need> {
        let mut buf = Vec::new();
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == b"hlinkClick" || local == b"hlinkMouseOver" {
                    let rel_id = attr_string(e, b"r:id");
                    return Ok(Info::HyperlinkStart { rel_id });
                }
                if let Some(kind) = dispatch_start(&local) {
                    return Ok(Info::ContainerStart(kind));
                }
                if is_text_element(&local) {
                    let text = self.read_text_content(reader, input, is_final)?;
                    return Ok(Info::Text(text));
                }
                if is_transparent_wrapper(&local) {
                    return Ok(Info::TransparentStart);
                }
                self.skip_element(reader, is_final)?;
                Ok(Info::Other)
            }
            Ok(XmlEvent::Empty(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == b"br" {
                    return Ok(Info::Leaf(PmlEvent::LineBreak));
                }
                if local == b"fldId" {
                    let field_type = attr_string(e, b"type").map(Cow::Owned);
                    return Ok(Info::Leaf(PmlEvent::FieldId { field_type }));
                }
                Ok(Info::Other)
            }
            Ok(XmlEvent::End(_)) => Ok(Info::End),
            Ok(XmlEvent::Text(ref e)) => {
                let consumed = reader.buffer_position() as usize;
                if consumed == input.len() && !is_final {
                    return Err(Need);
                }
                let text = e.decode().unwrap_or_default().into_owned();
                Ok(Info::Text(text))
            }
            Ok(XmlEvent::CData(ref e)) => {
                // Unlike plain `Text`, CDATA has an explicit `]]>`
                // terminator, so a truncated CDATA section is reported as
                // `Err(Syntax(UnclosedCData))` (handled below), not a
                // silently-short `Ok` — no buffer-boundary ambiguity here.
                let text = e.decode().unwrap_or_default().into_owned();
                Ok(Info::Text(text))
            }
            Ok(XmlEvent::Eof) => {
                if is_final {
                    Ok(Info::Eof)
                } else {
                    Err(Need)
                }
            }
            Err(_) => {
                if is_final {
                    Ok(Info::Eof)
                } else {
                    Err(Need)
                }
            }
            Ok(_) => Ok(Info::Other),
        }
    }

    fn read_xml_info_or_props(
        &mut self,
        reader: &mut Reader<&[u8]>,
        input: &[u8],
        is_final: bool,
        props_local: &[u8],
    ) -> Result<PropsOrInfo, Need> {
        let mut buf = Vec::new();
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == props_local {
                    return Ok(PropsOrInfo::IsProps {
                        is_empty: false,
                        tag_bytes: buf.clone(),
                    });
                }
                if local == b"hlinkClick" || local == b"hlinkMouseOver" {
                    let rel_id = attr_string(e, b"r:id");
                    return Ok(PropsOrInfo::Info(Info::HyperlinkStart { rel_id }));
                }
                if let Some(kind) = dispatch_start(&local) {
                    return Ok(PropsOrInfo::Info(Info::ContainerStart(kind)));
                }
                if is_text_element(&local) {
                    let text = self.read_text_content(reader, input, is_final)?;
                    return Ok(PropsOrInfo::Info(Info::Text(text)));
                }
                if is_transparent_wrapper(&local) {
                    return Ok(PropsOrInfo::Info(Info::TransparentStart));
                }
                self.skip_element(reader, is_final)?;
                Ok(PropsOrInfo::Info(Info::Other))
            }
            Ok(XmlEvent::Empty(ref e)) => {
                let local = local_name_owned(e.local_name().as_ref());
                if local == props_local {
                    return Ok(PropsOrInfo::IsProps {
                        is_empty: true,
                        tag_bytes: buf.clone(),
                    });
                }
                Ok(PropsOrInfo::Info(Info::Other))
            }
            Ok(XmlEvent::End(_)) => Ok(PropsOrInfo::Info(Info::End)),
            Ok(XmlEvent::Text(ref e)) => {
                let consumed = reader.buffer_position() as usize;
                if consumed == input.len() && !is_final {
                    return Err(Need);
                }
                let text = e.decode().unwrap_or_default().into_owned();
                Ok(PropsOrInfo::Info(Info::Text(text)))
            }
            // Matches `crate::events::PmlEventIter::read_xml_info_or_props`
            // exactly: no dedicated `CData` arm (falls through to `Other`,
            // i.e. ignored) — an existing asymmetry with `read_xml_info`
            // that this module preserves for byte-for-byte event parity.
            Ok(XmlEvent::Eof) => {
                if is_final {
                    Ok(PropsOrInfo::Info(Info::Eof))
                } else {
                    Err(Need)
                }
            }
            Err(_) => {
                if is_final {
                    Ok(PropsOrInfo::Info(Info::Eof))
                } else {
                    Err(Need)
                }
            }
            Ok(_) => Ok(PropsOrInfo::Info(Info::Other)),
        }
    }

    fn read_text_content(
        &mut self,
        reader: &mut Reader<&[u8]>,
        input: &[u8],
        is_final: bool,
    ) -> Result<String, Need> {
        let mut text = String::new();
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_event_into(&mut buf) {
                Ok(XmlEvent::Text(ref e)) => {
                    let consumed = reader.buffer_position() as usize;
                    if consumed == input.len() && !is_final {
                        return Err(Need);
                    }
                    text.push_str(&e.decode().unwrap_or_default());
                }
                Ok(XmlEvent::CData(ref e)) => {
                    text.push_str(&e.decode().unwrap_or_default());
                }
                Ok(XmlEvent::End(_)) => break,
                Ok(XmlEvent::Eof) => {
                    if is_final {
                        break;
                    }
                    return Err(Need);
                }
                Err(_) => {
                    if is_final {
                        break;
                    }
                    return Err(Need);
                }
                _ => {}
            }
        }
        Ok(text)
    }

    fn skip_element(&mut self, reader: &mut Reader<&[u8]>, is_final: bool) -> Result<(), Need> {
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
                Ok(XmlEvent::Eof) => {
                    if is_final {
                        break;
                    }
                    return Err(Need);
                }
                Err(_) => {
                    if is_final {
                        break;
                    }
                    return Err(Need);
                }
                _ => {}
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------
    // Shape geometry (`<p:spPr>` / `<a:xfrm>` / `<a:prstGeom>` / `<a:custGeom>`)
    // -----------------------------------------------------------------

    fn read_shape_transform(
        &mut self,
        reader: &mut Reader<&[u8]>,
        input: &[u8],
        is_final: bool,
    ) -> Result<(Option<ShapeTransform>, Option<ShapeGeometry>), Need> {
        loop {
            match self.read_xml_info_or_props(reader, input, is_final, b"spPr")? {
                PropsOrInfo::IsProps { is_empty, .. } => {
                    if is_empty {
                        return Ok((None, None));
                    }
                    if !is_final {
                        let pos = reader.buffer_position() as usize;
                        if pos > input.len() || !element_fully_buffered(&input[pos..]) {
                            return Err(Need);
                        }
                    }
                    return self.extract_xfrm_from_sppr(reader, is_final);
                }
                PropsOrInfo::Info(Info::TransparentStart) => {
                    self.stack.push(ContextFrame::Transparent);
                    return Ok((None, None));
                }
                PropsOrInfo::Info(Info::ContainerStart(child_kind)) => {
                    let child_event = self.open(reader, input, is_final, child_kind)?;
                    self.queue.push_front(child_event);
                    return Ok((None, None));
                }
                PropsOrInfo::Info(Info::HyperlinkStart { rel_id }) => {
                    self.stack
                        .push(ContextFrame::Tracked(PmlStartKind::Hyperlink));
                    self.queue.push_front(PmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                    });
                    return Ok((None, None));
                }
                PropsOrInfo::Info(Info::Text(t)) => {
                    if t.trim().is_empty() {
                        // Inter-element whitespace — keep scanning.
                    } else {
                        self.queue.push_front(PmlEvent::Text(Cow::Owned(t)));
                        return Ok((None, None));
                    }
                }
                PropsOrInfo::Info(Info::End) => {
                    self.queue.push_front(end_event_for(PmlStartKind::Shape));
                    self.close_immediately = true;
                    return Ok((None, None));
                }
                PropsOrInfo::Info(Info::Eof) => {
                    self.done = true;
                    return Ok((None, None));
                }
                PropsOrInfo::Info(Info::Leaf(_)) | PropsOrInfo::Info(Info::Other) => {}
            }
        }
    }

    fn extract_xfrm_from_sppr(
        &mut self,
        reader: &mut Reader<&[u8]>,
        is_final: bool,
    ) -> Result<(Option<ShapeTransform>, Option<ShapeGeometry>), Need> {
        let mut x: Option<i64> = None;
        let mut y: Option<i64> = None;
        let mut cx: Option<i64> = None;
        let mut cy: Option<i64> = None;
        let mut geometry: Option<ShapeGeometry> = None;
        let mut depth = 1u32;
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_event_into(&mut buf) {
                Ok(XmlEvent::Start(ref e)) => {
                    let local = local_name_owned(e.local_name().as_ref());
                    if local == b"prstGeom" {
                        let preset = attr_string(e, b"prst").unwrap_or_default();
                        let adjustments = self.read_av_lst_to_end(reader, is_final)?;
                        geometry = Some(ShapeGeometry::Preset {
                            preset,
                            adjustments,
                        });
                    } else if local == b"custGeom" {
                        // No `element_fully_buffered` pre-check needed here
                        // (unlike `read_props`'s `pPr`/`rPr`/etc.): this
                        // function only runs after `read_shape_transform`
                        // already confirmed the *entire* `<p:spPr>...</p:spPr>`
                        // subtree — which contains this `<a:custGeom>` — is
                        // fully buffered, so by construction these bytes are
                        // already known-complete. `RawXmlElement::from_reader`
                        // also already returns `Err` (not a silent partial
                        // result) on a genuine `Eof`, so even without that
                        // guarantee it couldn't silently corrupt data — it's
                        // just redundant to check again.
                        match ooxml_xml::RawXmlElement::from_reader(reader, e) {
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
                Ok(XmlEvent::Eof) => {
                    if is_final {
                        self.done = true;
                        break;
                    }
                    return Err(Need);
                }
                Err(_) => {
                    if is_final {
                        self.done = true;
                        break;
                    }
                    return Err(Need);
                }
                _ => {}
            }
        }
        let transform = match (x, y, cx, cy) {
            (Some(x), Some(y), Some(cx), Some(cy)) => Some(ShapeTransform { x, y, cx, cy }),
            _ => None,
        };
        Ok((transform, geometry))
    }

    fn read_av_lst_to_end(
        &mut self,
        reader: &mut Reader<&[u8]>,
        is_final: bool,
    ) -> Result<Vec<(String, String)>, Need> {
        let mut adjustments = Vec::new();
        let mut depth = 1u32;
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_event_into(&mut buf) {
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
                Ok(XmlEvent::Eof) => {
                    if is_final {
                        self.done = true;
                        break;
                    }
                    return Err(Need);
                }
                Err(_) => {
                    if is_final {
                        self.done = true;
                        break;
                    }
                    return Err(Need);
                }
                _ => {}
            }
        }
        Ok(adjustments)
    }

    // -----------------------------------------------------------------
    // Properties (`pPr` / `rPr` / `tblPr` / `tcPr`)
    // -----------------------------------------------------------------

    fn read_props<T: FromXml + Default>(
        &mut self,
        reader: &mut Reader<&[u8]>,
        input: &[u8],
        is_final: bool,
        expected_local: &[u8],
        owner: PmlStartKind,
    ) -> Result<T, Need> {
        loop {
            match self.read_xml_info_or_props(reader, input, is_final, expected_local)? {
                PropsOrInfo::IsProps {
                    is_empty,
                    tag_bytes,
                } => {
                    if !is_empty && !is_final {
                        let pos = reader.buffer_position() as usize;
                        if pos > input.len() || !element_fully_buffered(&input[pos..]) {
                            return Err(Need);
                        }
                    }
                    let start = bytes_start_from_raw(&tag_bytes);
                    return Ok(T::from_xml(reader, &start, is_empty).unwrap_or_default());
                }
                PropsOrInfo::Info(Info::TransparentStart) => {
                    self.stack.push(ContextFrame::Transparent);
                    return Ok(T::default());
                }
                PropsOrInfo::Info(Info::ContainerStart(child_kind)) => {
                    let child_event = self.open(reader, input, is_final, child_kind)?;
                    self.queue.push_front(child_event);
                    return Ok(T::default());
                }
                PropsOrInfo::Info(Info::HyperlinkStart { rel_id }) => {
                    self.stack
                        .push(ContextFrame::Tracked(PmlStartKind::Hyperlink));
                    self.queue.push_front(PmlEvent::StartHyperlink {
                        rel_id: rel_id.map(Cow::Owned),
                    });
                    return Ok(T::default());
                }
                PropsOrInfo::Info(Info::Text(t)) => {
                    if t.trim().is_empty() {
                        // Inter-element whitespace — keep scanning.
                    } else {
                        self.queue.push_front(PmlEvent::Text(Cow::Owned(t)));
                        return Ok(T::default());
                    }
                }
                PropsOrInfo::Info(Info::End) => {
                    self.queue.push_front(end_event_for(owner));
                    self.close_immediately = true;
                    return Ok(T::default());
                }
                PropsOrInfo::Info(Info::Eof) => {
                    self.done = true;
                    return Ok(T::default());
                }
                PropsOrInfo::Info(Info::Leaf(_)) | PropsOrInfo::Info(Info::Other) => {}
            }
        }
    }
}

/// Returns `true` if the element whose opening tag was just consumed (depth
/// starts at 1, same convention as `skip_element`) is fully present in
/// `bytes` — i.e. scanning forward hits a balanced closing tag before
/// running out of bytes or hitting a genuinely malformed token. Used to
/// avoid handing a possibly chunk-truncated properties/geometry element to
/// a parser (`FromXml::from_xml`) whose own truncation handling is
/// "silently return partial data" rather than "error" — see this module's
/// doc comment.
fn element_fully_buffered(bytes: &[u8]) -> bool {
    let mut probe = Reader::from_reader(bytes);
    probe.config_mut().trim_text(false);
    // Same reasoning as the main attempt reader above: this probe starts
    // mid-document with no memory of already-open tags.
    probe.config_mut().check_end_names = false;
    probe.config_mut().allow_unmatched_ends = true;
    let mut depth = 1u32;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match probe.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(_)) => depth += 1,
            Ok(XmlEvent::End(_)) => {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            Ok(XmlEvent::Eof) => return false,
            Err(_) => return false,
            _ => {}
        }
    }
}

/// Reconstruct a `BytesStart` from the raw bytes `quick_xml` filled into its
/// scratch buffer for a just-consumed Start/Empty tag — mirrors the inline
/// snippet `crate::events::PmlEventIter::read_props` uses for the same
/// purpose (not extracted to a shared helper since it is a few lines acting
/// on a plain `&[u8]`, not `Reader` state).
fn bytes_start_from_raw(tag_bytes: &[u8]) -> BytesStart<'_> {
    let tag_str = std::str::from_utf8(tag_bytes).unwrap_or("");
    let content = tag_str
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim_end_matches('/');
    let name_len = content
        .bytes()
        .position(|b| b == b' ')
        .unwrap_or(content.len());
    BytesStart::from_content(content, name_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(bytes: &[u8], chunk_size: usize) -> Vec<OwnedPmlEvent> {
        let mut events = Vec::new();
        let mut tok = ChunkedPmlTokenizer::new();
        if chunk_size == 0 {
            tok.finish(&mut |e| events.push(e));
            return events;
        }
        for chunk in bytes.chunks(chunk_size) {
            tok.feed(chunk, &mut |e| events.push(e));
        }
        tok.finish(&mut |e| events.push(e));
        events
    }

    fn run_whole(bytes: &[u8]) -> Vec<OwnedPmlEvent> {
        crate::events::events(bytes)
            .map(|e| e.into_owned())
            .collect()
    }

    fn texts(events: &[OwnedPmlEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|e| match e {
                PmlEvent::Text(t) => Some(t.to_string()),
                _ => None,
            })
            .collect()
    }

    fn slide_xml(text: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="TextBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="100" y="200"/><a:ext cx="300" cy="400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst><a:gd name="adj1" fmla="val 1000"/></a:avLst></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr b="1"/><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#
        )
    }

    #[test]
    fn chunked_matches_whole_buffer_across_chunk_sizes() {
        let xml = slide_xml("Hello, chunked world!");
        let bytes = xml.as_bytes();
        let expected = run_whole(bytes);
        for chunk_size in [1usize, 2, 3, 7, 16, 64, bytes.len() + 1] {
            let got = run(bytes, chunk_size);
            assert_eq!(
                texts(&got),
                texts(&expected),
                "chunk_size={chunk_size} produced different text events"
            );
            assert_eq!(
                got.len(),
                expected.len(),
                "chunk_size={chunk_size} produced a different event count"
            );
        }
    }

    #[test]
    fn chunked_handles_empty_and_truncated_and_garbage_input_without_panicking() {
        // Empty input.
        {
            let mut events = Vec::new();
            let tok = ChunkedPmlTokenizer::new();
            tok.finish(&mut |e| events.push(e));
        }

        // Truncated real slide XML, fed byte-at-a-time.
        let xml = slide_xml("Truncated");
        let bytes = xml.as_bytes();
        let half = &bytes[..bytes.len() / 2];
        {
            let mut events = Vec::new();
            let mut tok = ChunkedPmlTokenizer::new();
            for b in half {
                tok.feed(std::slice::from_ref(b), &mut |e| events.push(e));
            }
            tok.finish(&mut |e| events.push(e));
        }

        // Arbitrary non-XML garbage bytes.
        for seed in 0..8u8 {
            let garbage: Vec<u8> = (0..200u8)
                .map(|i| i.wrapping_mul(seed).wrapping_add(i))
                .collect();
            let mut events = Vec::new();
            let mut tok = ChunkedPmlTokenizer::new();
            for chunk in garbage.chunks(3) {
                tok.feed(chunk, &mut |e| events.push(e));
            }
            tok.finish(&mut |e| events.push(e));
        }
    }

    /// Structural memory-bound evidence: feed a slide with a large amount of
    /// text/shapes in small fixed-size chunks and confirm the tokenizer's
    /// own unconsumed-byte buffer never grows anywhere near the full slide
    /// size — mirrors `ooxml-opc::batch`'s
    /// `large_part_streams_as_multiple_bounded_chunks_after_content_types`.
    #[test]
    fn pending_buffer_stays_bounded_for_a_large_slide() {
        let mut body = String::new();
        for i in 0..2000 {
            body.push_str(&format!(
                r#"<p:sp><p:nvSpPr><p:cNvPr id="{i}" name="s{i}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>paragraph number {i} with some filler text to make this element reasonably sized on its own</a:t></a:r></a:p></p:txBody></p:sp>"#
            ));
        }
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{body}</p:spTree></p:cSld></p:sld>"#
        );
        let bytes = xml.as_bytes();
        assert!(
            bytes.len() > 200_000,
            "test fixture should be large; got {} bytes",
            bytes.len()
        );

        let mut tok = ChunkedPmlTokenizer::new();
        let mut max_pending = 0usize;
        let mut events = Vec::new();
        for chunk in bytes.chunks(256) {
            tok.feed(chunk, &mut |e| events.push(e));
            max_pending = max_pending.max(tok.pending_len());
        }
        tok.finish(&mut |e| events.push(e));

        assert!(
            max_pending < 4096,
            "tokenizer's unconsumed buffer grew to {max_pending} bytes while streaming a \
             {}-byte slide — expected it to stay bounded by nesting depth / largest token, \
             not the whole slide",
            bytes.len()
        );

        let text_count = events
            .iter()
            .filter(|e| matches!(e, PmlEvent::Text(_)))
            .count();
        assert_eq!(text_count, 2000, "expected one Text event per paragraph");
    }
}
