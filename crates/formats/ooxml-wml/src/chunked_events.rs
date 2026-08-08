//! Chunk-resumable re-implementation of [`crate::events::WmlEventIter`]'s
//! state machine, for [`crate::batch::StreamingParser`].
//!
//! This is a genuinely independent implementation, not a wrapper around
//! [`crate::events::events`] — see `batch.rs`'s module docs for why (CLAUDE.md's
//! "three independent implementations" rule). It reuses only the *pure*,
//! single-token helpers from `events.rs` that have no internal read loop
//! (`dispatch_start`, `is_transparent_wrapper`, `local_name_owned`,
//! `build_leaf_event_owned`, `end_event_for`, `attr_string`, `push_entity`) —
//! anything with its own `read_event_into` loop (`read_text_content`,
//! `skip_element`, the `next()`/`read_props` orchestration) is reimplemented
//! here because it needs chunk-boundary-ambiguity awareness that
//! `WmlEventIter`'s live-infinite-reader version doesn't.
//!
//! # The chunk-resumable technique
//!
//! Same core technique as `docbook_fmt::batch::StreamingParser` (see that
//! module's docs for the full rationale): each attempt to make progress
//! builds a *fresh* `quick_xml::Reader` over the current unconsumed tail
//! (`ChunkedWmlReader::pending`) and reads from it. A truncated tag,
//! comment, CDATA, or declaration is unambiguous — quick-xml reports
//! `Err(Syntax(_))` rather than silently treating it as finished — so it is
//! always safe to treat any `Err` (when not the final call) as "need more
//! bytes, retry the whole attempt from scratch once more input arrives".
//! Plain text is the one ambiguous token type: quick-xml terminates a
//! `Text` token either at the next `<` or at the end of the buffered slice,
//! and those look identical from the return value alone, so a `Text` token
//! is only accepted when `reader.buffer_position()` did *not* land exactly
//! at the end of the currently-buffered bytes (proving it hit a real `<`),
//! or when this is the final call.
//!
//! ## Extending the technique to WML's props lookahead
//!
//! `WmlEventIter::read_props` peeks past a container's start tag
//! (`<w:p>`) for its props child (`<w:pPr>`); if the next thing is instead
//! a tracked child (e.g. a `<w:r>` with no `pPr`), it recurses to resolve
//! that child's own event (which may itself recurse for its own props/
//! child lookahead, and so on — bounded by nesting depth, not by document
//! size). This module mirrors that recursion directly through Rust's own
//! call stack (`open_container` calling `resolve_props` calling
//! `open_container` again for a found child), threading a `NeedMore`
//! sentinel through every `read_event_into` call via `?`. The whole
//! recursive chain for one top-level "logical step" (one `WmlEventIter`
//! `next()`-worth of progress, which may itself resolve several nested
//! containers' events at once — see `step_inner`) runs against *one*
//! `Reader` instance built fresh per attempt; if ambiguity is hit anywhere
//! in the chain, the entire attempt is abandoned — the stack mutations
//! `open_container`/`resolve_props` made along the way are undone by
//! restoring a pre-attempt clone of `stack` (cheap: `Vec<Frame>` is O(nesting
//! depth) and `Frame` is `Copy`) — and nothing is drained from `pending`, so
//! the next `feed()` (or `finish()`) retries the identical chain from byte
//! zero of the (now larger) unconsumed tail. Only once a whole logical step
//! is known-complete does `ChunkedWmlReader::try_step` commit: drain the
//! consumed prefix out of `pending` and dispatch the events accumulated in
//! `sink`.
//!
//! ## The one non-obvious extra ambiguity: props-element parsing
//!
//! Every generated [`ooxml_xml::FromXml`] impl for a props type (`pPr`,
//! `rPr`, …) reads child elements in a `loop { match
//! reader.read_event_into(...)? { ... Event::Eof => break, ... } }` — i.e.
//! it treats "ran out of buffered bytes" identically to "found the props
//! element's own matching end tag": both just `break` out of the loop and
//! return `Ok(props)`, with no signal distinguishing the two. So a props
//! element parsed with a `Reader` over a not-yet-complete buffer can return
//! `Ok` with a **silently incomplete** struct — the same failure mode
//! `docbook-fmt` calls out for plain text, but here `from_xml`'s `Ok` return
//! value carries no marker at all. This module resolves it the same way
//! `docbook-fmt` resolves the Text case: after a successful
//! `T::from_xml(...)` call, check whether `reader.buffer_position()` landed
//! exactly at the end of the currently-buffered bytes. If it did, the
//! result is ambiguous (could be a legitimate close exactly at the buffer's
//! last byte, or a silent early `break`) and is discarded unless this is
//! the final call — the *whole* logical step (not just the props element)
//! is abandoned and retried once more input arrives, per the module docs
//! above. This is conservative — it also holds back the case where the
//! props element genuinely closes exactly on the last currently-buffered
//! byte with no error at all — but that only costs an extra retry, never
//! correctness: `resolve_props` never dispatches a props struct that could
//! still be silently incomplete.
//!
//! A `from_xml` call that returns `Err` (a genuine XML syntax error inside
//! the props element, not merely a truncated buffer) is treated the same as
//! any other `Err` from this module's primitive token reads: retried as
//! "need more bytes" until `is_final`, at which point it falls back to
//! `T::default()` — matching `WmlEventIter`'s own `Err(_) => T::default()`
//! fallback exactly (see `read_props` in `events.rs`). This means a
//! genuinely (not just incompletely-buffered) malformed props element
//! forces buffering of the rest of that element until `finish()`, the same
//! tradeoff `docbook-fmt` accepts for any XML syntax error: this module
//! cannot distinguish "will never resolve" from "resolves with the next
//! chunk" from the return value alone, so it always waits it out rather
//! than risk discarding real content.

use std::borrow::Cow;

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::events::{
    attr_string, build_leaf_event_owned, end_event_for, is_transparent_wrapper, local_name_owned,
    push_entity,
};
use crate::generated::{
    ParagraphProperties, RunProperties, TableCellProperties, TableProperties, TableRowProperties,
};
use crate::generated_events::{WmlEvent, WmlStartKind, dispatch_start, is_text_element};
use ooxml_xml::FromXml;

type OwnedWmlEvent = WmlEvent<'static>;

/// Nesting-stack entry — mirrors `events.rs`'s private `ContextFrame`
/// (redefined here rather than shared, since it's a two-variant marker type,
/// not parsing/writing logic; see the module docs for what *is* reused).
#[derive(Debug, Clone, Copy)]
enum Frame {
    Tracked(WmlStartKind),
    Transparent,
}

/// Sentinel propagated via `?` when the currently-buffered bytes are not
/// enough to safely resolve the current step. Never escapes this module —
/// always caught by [`ChunkedWmlReader::try_step`] and turned into "wait for
/// more input".
struct NeedMore;

/// What one raw XML token, or a `<w:t>` text run, resolves to. Mirrors
/// `events.rs`'s private `XmlInfo`.
enum Info {
    ContainerStart(WmlStartKind),
    TransparentStart,
    Leaf(OwnedWmlEvent),
    HyperlinkStart {
        rel_id: Option<String>,
        anchor: Option<String>,
    },
    End,
    Text(String),
    /// Genuine end of `word/document.xml` — only ever produced when
    /// `is_final` is true (otherwise the ambiguous case returns
    /// `Err(NeedMore)` instead).
    Eof,
    Other,
}

/// Mirrors `events.rs`'s private `PropsOrInfo`. `tag_bytes` is an owned copy
/// of the props element's start-tag bytes, used to reconstruct a
/// `BytesStart` for `FromXml::from_xml` exactly as `events.rs` does.
enum PropsOrInfo {
    IsProps { is_empty: bool, tag_bytes: Vec<u8> },
    Info(Info),
}

/// Read one raw XML token and classify it, mirroring `events.rs`'s
/// `read_xml_info` — except any truncation (an `Err(Syntax(_))`, or a clean
/// `Ok(Eof)` reached before `is_final`) is reported as `Err(NeedMore)`
/// instead of silently treated as "done".
fn read_info(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    total_len: usize,
    is_final: bool,
) -> Result<Info, NeedMore> {
    buf.clear();
    match reader.read_event_into(buf) {
        Ok(XmlEvent::Start(ref e)) => {
            let local = local_name_owned(e.local_name().as_ref());
            if local == b"hyperlink" {
                let rel_id = attr_string(e, b"r:id");
                let anchor = attr_string(e, b"w:anchor");
                return Ok(Info::HyperlinkStart { rel_id, anchor });
            }
            if let Some(kind) = dispatch_start(&local) {
                return Ok(Info::ContainerStart(kind));
            }
            if is_text_element(&local) {
                let text = read_text_content(reader, total_len, is_final)?;
                return Ok(Info::Text(text));
            }
            if is_transparent_wrapper(&local) {
                return Ok(Info::TransparentStart);
            }
            skip_element(reader, is_final)?;
            Ok(Info::Other)
        }
        Ok(XmlEvent::Empty(ref e)) => {
            let local = local_name_owned(e.local_name().as_ref());
            if let Some(event) = build_leaf_event_owned(&local, e) {
                return Ok(Info::Leaf(event));
            }
            Ok(Info::Other)
        }
        Ok(XmlEvent::End(_)) => Ok(Info::End),
        // Character data outside a `<w:t>` is inter-element formatting
        // whitespace and is always discarded, so it needs no ambiguity
        // check: whichever way a chunk boundary splits it, both halves are
        // still discarded — see the module docs on the props-Text case for
        // contrast with content that *is* observable.
        Ok(XmlEvent::Text(_)) => Ok(Info::Other),
        Ok(XmlEvent::CData(ref e)) => {
            let text = e.decode().unwrap_or_default().into_owned();
            Ok(Info::Text(text))
        }
        Ok(XmlEvent::Eof) => {
            if is_final {
                Ok(Info::Eof)
            } else {
                Err(NeedMore)
            }
        }
        Err(_) => {
            if is_final {
                Ok(Info::Eof)
            } else {
                Err(NeedMore)
            }
        }
        Ok(_) => Ok(Info::Other),
    }
}

/// Same as [`read_info`] but also recognizes a props element with the given
/// local name, mirroring `events.rs`'s `read_xml_info_or_props`.
fn read_info_or_props(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    total_len: usize,
    is_final: bool,
    props_local: &[u8],
) -> Result<PropsOrInfo, NeedMore> {
    buf.clear();
    match reader.read_event_into(buf) {
        Ok(XmlEvent::Start(ref e)) => {
            let local = local_name_owned(e.local_name().as_ref());
            if local == props_local {
                return Ok(PropsOrInfo::IsProps {
                    is_empty: false,
                    tag_bytes: buf.clone(),
                });
            }
            if local == b"hyperlink" {
                let rel_id = attr_string(e, b"r:id");
                let anchor = attr_string(e, b"w:anchor");
                return Ok(PropsOrInfo::Info(Info::HyperlinkStart { rel_id, anchor }));
            }
            if let Some(kind) = dispatch_start(&local) {
                return Ok(PropsOrInfo::Info(Info::ContainerStart(kind)));
            }
            if is_text_element(&local) {
                let text = read_text_content(reader, total_len, is_final)?;
                return Ok(PropsOrInfo::Info(Info::Text(text)));
            }
            if is_transparent_wrapper(&local) {
                return Ok(PropsOrInfo::Info(Info::TransparentStart));
            }
            skip_element(reader, is_final)?;
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
            if let Some(event) = build_leaf_event_owned(&local, e) {
                return Ok(PropsOrInfo::Info(Info::Leaf(event)));
            }
            Ok(PropsOrInfo::Info(Info::Other))
        }
        Ok(XmlEvent::End(_)) => Ok(PropsOrInfo::Info(Info::End)),
        Ok(XmlEvent::Text(_)) => Ok(PropsOrInfo::Info(Info::Other)),
        Ok(XmlEvent::Eof) => {
            if is_final {
                Ok(PropsOrInfo::Info(Info::Eof))
            } else {
                Err(NeedMore)
            }
        }
        Err(_) => {
            if is_final {
                Ok(PropsOrInfo::Info(Info::Eof))
            } else {
                Err(NeedMore)
            }
        }
        Ok(_) => Ok(PropsOrInfo::Info(Info::Other)),
    }
}

/// Read an already-opened text element's content (`<w:t>…</w:t>`), mirroring
/// `events.rs`'s `read_text_content` — except a `Text`/`CData`/`GeneralRef`
/// run that reaches exactly the end of the currently-buffered bytes is
/// ambiguous (see the module docs) and, unless `is_final`, aborts the whole
/// attempt via `Err(NeedMore)` rather than silently truncating the text.
fn read_text_content(
    reader: &mut Reader<&[u8]>,
    total_len: usize,
    is_final: bool,
) -> Result<String, NeedMore> {
    let mut text = String::new();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Text(ref e)) => {
                let consumed = reader.buffer_position() as usize;
                if consumed == total_len && !is_final {
                    return Err(NeedMore);
                }
                text.push_str(&e.decode().unwrap_or_default());
            }
            // CData and entity references are self-terminating (`]]>`, `;`)
            // — quick-xml reports a truncated one as `Err(Syntax(_))` rather
            // than a short `Ok`, so no extra ambiguity check is needed here;
            // the generic `Err`/`Eof` handling below already covers it.
            Ok(XmlEvent::CData(ref e)) => {
                text.push_str(&e.decode().unwrap_or_default());
            }
            Ok(XmlEvent::GeneralRef(ref e)) => {
                push_entity(&mut text, e);
            }
            Ok(XmlEvent::End(_)) => break,
            Ok(XmlEvent::Eof) => {
                if is_final {
                    break;
                }
                return Err(NeedMore);
            }
            Err(_) => {
                if is_final {
                    break;
                }
                return Err(NeedMore);
            }
            _ => {}
        }
    }
    Ok(text)
}

/// Skip an open element and all its children, mirroring `events.rs`'s
/// `skip_element` with the same `Err`/`Eof`-before-`is_final` → `NeedMore`
/// handling as every other primitive here.
fn skip_element(reader: &mut Reader<&[u8]>, is_final: bool) -> Result<(), NeedMore> {
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
                return Err(NeedMore);
            }
            Err(_) => {
                if is_final {
                    break;
                }
                return Err(NeedMore);
            }
            _ => {}
        }
    }
    Ok(())
}

/// Resolve one raw token's worth (or more, via recursion — see module docs)
/// of progress, appending every event it decides on, in order, to `sink`.
/// Mirrors the body of `WmlEventIter::next()`'s loop.
///
/// Returns `Ok(true)` once at least one event became ready to dispatch (the
/// caller should commit), `Ok(false)` on genuine end of document (`is_final`
/// and nothing left), or `Err(NeedMore)` if the buffer was insufficient
/// anywhere along the way.
fn step_inner(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    stack: &mut Vec<Frame>,
    total_len: usize,
    is_final: bool,
    sink: &mut Vec<OwnedWmlEvent>,
) -> Result<bool, NeedMore> {
    loop {
        match read_info(reader, buf, total_len, is_final)? {
            Info::ContainerStart(kind) => {
                open_container(reader, buf, stack, total_len, is_final, kind, sink)?;
                return Ok(true);
            }
            Info::TransparentStart => {
                stack.push(Frame::Transparent);
            }
            Info::HyperlinkStart { rel_id, anchor } => {
                stack.push(Frame::Tracked(WmlStartKind::Hyperlink));
                sink.push(WmlEvent::StartHyperlink {
                    rel_id: rel_id.map(Cow::Owned),
                    anchor: anchor.map(Cow::Owned),
                });
                return Ok(true);
            }
            Info::Leaf(ev) => {
                sink.push(ev);
                return Ok(true);
            }
            Info::End => match stack.pop() {
                Some(Frame::Tracked(kind)) => {
                    sink.push(end_event_for(kind));
                    return Ok(true);
                }
                Some(Frame::Transparent) | None => {}
            },
            Info::Text(t) => {
                if !t.is_empty() {
                    sink.push(WmlEvent::Text(Cow::Owned(t)));
                    return Ok(true);
                }
            }
            Info::Eof => return Ok(false),
            Info::Other => {}
        }
    }
}

/// Open a tracked container: push its frame, resolve its props (recursing
/// into a found child container via `resolve_props` if one appears before
/// the props element — see module docs), and push the resulting `Start…`
/// event (and any events the recursion resolved) onto `sink` in document
/// order. Mirrors `WmlEventIter::open` + `build_start_event`.
fn open_container(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    stack: &mut Vec<Frame>,
    total_len: usize,
    is_final: bool,
    kind: WmlStartKind,
    sink: &mut Vec<OwnedWmlEvent>,
) -> Result<(), NeedMore> {
    stack.push(Frame::Tracked(kind));
    let close_immediately = match kind {
        WmlStartKind::Paragraph => resolve_props::<ParagraphProperties>(
            reader,
            buf,
            stack,
            total_len,
            is_final,
            kind,
            b"pPr",
            sink,
            |props| WmlEvent::StartParagraph {
                props: Box::new(props),
            },
        )?,
        WmlStartKind::Run => resolve_props::<RunProperties>(
            reader,
            buf,
            stack,
            total_len,
            is_final,
            kind,
            b"rPr",
            sink,
            |props| WmlEvent::StartRun {
                props: Box::new(props),
            },
        )?,
        WmlStartKind::Table => resolve_props::<TableProperties>(
            reader,
            buf,
            stack,
            total_len,
            is_final,
            kind,
            b"tblPr",
            sink,
            |props| WmlEvent::StartTable {
                props: Box::new(props),
            },
        )?,
        WmlStartKind::TableRow => resolve_props::<TableRowProperties>(
            reader,
            buf,
            stack,
            total_len,
            is_final,
            kind,
            b"trPr",
            sink,
            |props| WmlEvent::StartTableRow {
                props: Box::new(props),
            },
        )?,
        WmlStartKind::TableCell => resolve_props::<TableCellProperties>(
            reader,
            buf,
            stack,
            total_len,
            is_final,
            kind,
            b"tcPr",
            sink,
            |props| WmlEvent::StartTableCell {
                props: Box::new(props),
            },
        )?,
        // Hyperlink has no props child and carries its attributes on the
        // element itself; it is opened directly by `step_inner`/
        // `resolve_props`'s `HyperlinkStart` arms, never through here.
        WmlStartKind::Hyperlink => unreachable!("hyperlink does not go through open_container"),
    };
    if close_immediately {
        stack.pop();
    }
    Ok(())
}

/// Scan ahead for `owner`'s props child (`props_local`), mirroring
/// `WmlEventIter::read_props`. Pushes `owner`'s own `Start…` event (built by
/// `build_event`) onto `sink` — with the real parsed props if the props
/// element was found, or `T::default()` otherwise — followed by whatever
/// else the scan resolved (a found child container's whole event chain, a
/// hyperlink start, a leaf, or text), in that order.
///
/// Returns `Ok(true)` if `owner`'s own end tag was consumed as part of this
/// scan (e.g. `<w:p></w:p>`, `owner`'s `End…` event has already been pushed
/// to `sink`) — the caller must not leave a frame on `stack` for something
/// that will never see a matching `End` token.
#[allow(clippy::too_many_arguments)]
fn resolve_props<T: FromXml + Default>(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    stack: &mut Vec<Frame>,
    total_len: usize,
    is_final: bool,
    owner: WmlStartKind,
    props_local: &[u8],
    sink: &mut Vec<OwnedWmlEvent>,
    build_event: impl FnOnce(T) -> OwnedWmlEvent,
) -> Result<bool, NeedMore> {
    loop {
        match read_info_or_props(reader, buf, total_len, is_final, props_local)? {
            PropsOrInfo::IsProps {
                is_empty,
                tag_bytes,
            } => {
                let start = quick_xml::events::BytesStart::from_content(
                    std::str::from_utf8(&tag_bytes).unwrap_or(""),
                    0,
                );
                match T::from_xml(reader, &start, is_empty) {
                    Ok(props) => {
                        // See module docs: every generated `FromXml` impl
                        // treats "ran out of buffered bytes mid-element" and
                        // "found the real closing tag" identically (both
                        // just `break`), so a result that consumed exactly
                        // to the end of the currently-buffered bytes is
                        // ambiguous and must wait for more input, unless
                        // this is the final call.
                        let consumed = reader.buffer_position() as usize;
                        if consumed == total_len && !is_final {
                            return Err(NeedMore);
                        }
                        sink.push(build_event(props));
                        return Ok(false);
                    }
                    Err(_) => {
                        if !is_final {
                            return Err(NeedMore);
                        }
                        // Genuinely malformed (not just truncated) props
                        // element: fall back to defaults, matching
                        // `WmlEventIter::read_props`'s own `Err(_) =>
                        // T::default()` exactly.
                        sink.push(build_event(T::default()));
                        return Ok(false);
                    }
                }
            }
            PropsOrInfo::Info(Info::TransparentStart) => {
                stack.push(Frame::Transparent);
                sink.push(build_event(T::default()));
                return Ok(false);
            }
            PropsOrInfo::Info(Info::ContainerStart(child_kind)) => {
                sink.push(build_event(T::default()));
                open_container(reader, buf, stack, total_len, is_final, child_kind, sink)?;
                return Ok(false);
            }
            PropsOrInfo::Info(Info::HyperlinkStart { rel_id, anchor }) => {
                sink.push(build_event(T::default()));
                stack.push(Frame::Tracked(WmlStartKind::Hyperlink));
                sink.push(WmlEvent::StartHyperlink {
                    rel_id: rel_id.map(Cow::Owned),
                    anchor: anchor.map(Cow::Owned),
                });
                return Ok(false);
            }
            PropsOrInfo::Info(Info::Leaf(ev)) => {
                sink.push(build_event(T::default()));
                sink.push(ev);
                return Ok(false);
            }
            PropsOrInfo::Info(Info::End) => {
                sink.push(build_event(T::default()));
                sink.push(end_event_for(owner));
                return Ok(true);
            }
            PropsOrInfo::Info(Info::Text(t)) => {
                if t.trim().is_empty() {
                    // Inter-element whitespace from pretty-printed XML: keep
                    // scanning for the props element or a tracked child,
                    // matching `WmlEventIter::read_props` exactly.
                } else {
                    sink.push(build_event(T::default()));
                    sink.push(WmlEvent::Text(Cow::Owned(t)));
                    return Ok(false);
                }
            }
            PropsOrInfo::Info(Info::Eof) => {
                sink.push(build_event(T::default()));
                return Ok(false);
            }
            PropsOrInfo::Info(Info::Other) => {}
        }
    }
}

/// What one `try_step` attempt accomplished.
enum StepOutcome {
    /// Progress was made (bytes consumed, events dispatched); try again.
    Progress,
    /// The buffer is insufficient to make further progress; wait for the
    /// next `feed()` (or `finish()`).
    NeedMoreData,
    /// Genuine end of document reached.
    Eof,
}

/// Chunk-resumable WML event source. See the module docs for the
/// chunk-resumability technique.
///
/// Memory model: `pending` holds only the unconsumed tail — bounded by the
/// largest still-in-progress token/props-element/lookahead-chain, not the
/// part size (see `batch.rs`'s module docs for the full crate-level bound,
/// including the one documented exception: a genuine XML syntax error, as
/// opposed to a merely-incomplete buffer, cannot be distinguished from
/// "will resolve once more input arrives" and so forces buffering of the
/// rest of the malformed run until `finish()`).
pub(crate) struct ChunkedWmlReader {
    pending: Vec<u8>,
    stack: Vec<Frame>,
    started: bool,
    done: bool,
}

impl ChunkedWmlReader {
    pub(crate) fn new() -> Self {
        ChunkedWmlReader {
            pending: Vec::new(),
            stack: Vec::new(),
            started: false,
            done: false,
        }
    }

    /// Feed the next chunk of `word/document.xml` bytes, dispatching every
    /// event that can be proven complete so far to `dispatch`.
    pub(crate) fn feed(&mut self, chunk: &[u8], dispatch: &mut dyn FnMut(OwnedWmlEvent)) {
        self.pending.extend_from_slice(chunk);
        self.drain(false, dispatch);
    }

    /// Signal that this part has ended: drain any remaining buffered bytes,
    /// resolving ambiguous trailing state as final. Returns a diagnostic
    /// message if non-whitespace bytes remain unparsed (truncated or
    /// malformed trailing content) — `None` for a clean end (including
    /// harmless trailing whitespace).
    pub(crate) fn finish(&mut self, dispatch: &mut dyn FnMut(OwnedWmlEvent)) -> Option<String> {
        self.drain(true, dispatch);
        if self.pending.iter().any(|b| !b.is_ascii_whitespace()) {
            let preview_len = self.pending.len().min(80);
            Some(format!(
                "word/document.xml: {} unparsed trailing byte(s) after the last complete WML \
                 construct (truncated or malformed document); starts with {:?}",
                self.pending.len(),
                String::from_utf8_lossy(&self.pending[..preview_len]),
            ))
        } else {
            None
        }
    }

    /// Current unconsumed-byte count — used only by the memory-bound test
    /// below to confirm `pending` never grows anywhere near full document
    /// size, the structural evidence the streaming fix landed.
    #[cfg(test)]
    pub(crate) fn pending_len(&self) -> usize {
        self.pending.len()
    }

    fn drain(&mut self, is_final: bool, dispatch: &mut dyn FnMut(OwnedWmlEvent)) {
        if self.done {
            return;
        }
        if !self.started {
            self.started = true;
            dispatch(WmlEvent::StartDocument);
        }
        loop {
            let mut sink = Vec::new();
            let outcome = self.try_step(is_final, &mut sink);
            for ev in sink {
                dispatch(ev);
            }
            match outcome {
                StepOutcome::Progress => continue,
                StepOutcome::NeedMoreData => return,
                StepOutcome::Eof => {
                    dispatch(WmlEvent::EndDocument);
                    self.done = true;
                    return;
                }
            }
        }
    }

    /// Attempt one logical step (see `step_inner`'s docs). On success,
    /// drains the consumed prefix from `pending` and reports `Progress`
    /// (with any resolved events left in `sink` for the caller to
    /// dispatch); on ambiguity, rolls `stack` back to its pre-attempt state,
    /// discards `sink`, and reports `NeedMoreData` without touching
    /// `pending` at all — the entire attempt is retried from scratch on the
    /// next call.
    fn try_step(&mut self, is_final: bool, sink: &mut Vec<OwnedWmlEvent>) -> StepOutcome {
        if self.pending.is_empty() {
            return if is_final {
                StepOutcome::Eof
            } else {
                StepOutcome::NeedMoreData
            };
        }
        let stack_snapshot = self.stack.clone();
        let total_len = self.pending.len();
        let mut reader = Reader::from_reader(&self.pending[..]);
        reader.config_mut().trim_text(false);
        // Each attempt builds a *fresh* `Reader` over just the unconsumed
        // tail (already-consumed prefixes were dropped by a previous
        // attempt's commit — see the module docs). That means this reader
        // never sees the `Start` tag matching an `End` tag consumed by an
        // *earlier* attempt, so quick-xml's own start/end name validation
        // would otherwise misreport every such `End` tag as unmatched. Tag
        // balance is instead enforced by `stack` below, which *does*
        // persist across attempts — same reasoning as
        // `docbook_fmt::batch::StreamingParser::drain`'s identical config.
        reader.config_mut().check_end_names = false;
        reader.config_mut().allow_unmatched_ends = true;
        let mut buf = Vec::new();
        match step_inner(
            &mut reader,
            &mut buf,
            &mut self.stack,
            total_len,
            is_final,
            sink,
        ) {
            Ok(true) => {
                let consumed = reader.buffer_position() as usize;
                self.pending.drain(0..consumed);
                StepOutcome::Progress
            }
            Ok(false) => {
                // Genuine document end: still drop whatever trailing
                // whitespace-only tokens `step_inner` legitimately consumed
                // while scanning past them before hitting Eof, so `finish`'s
                // malformed-trailing-content check doesn't flag harmless
                // trailing whitespace.
                let consumed = reader.buffer_position() as usize;
                self.pending.drain(0..consumed);
                StepOutcome::Eof
            }
            Err(NeedMore) => {
                self.stack = stack_snapshot;
                sink.clear();
                StepOutcome::NeedMoreData
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeding a large synthetic `word/document.xml` in small fixed-size
    /// chunks must never let `pending` grow anywhere near the document's
    /// total size — the structural evidence that chunk-resumable parsing
    /// actually landed, not just that output is still correct (which a
    /// buffer-then-parse implementation would also pass).
    #[test]
    fn pending_buffer_stays_bounded_for_a_large_document() {
        let mut body = String::new();
        body.push_str(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
"#,
        );
        for i in 0..20_000 {
            body.push_str(&format!(
                "<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr><w:r><w:rPr><w:b/></w:rPr>\
                 <w:t>paragraph number {i} with some filler text to pad the element out</w:t>\
                 </w:r></w:p>\n"
            ));
        }
        body.push_str("</w:body></w:document>");
        let bytes = body.into_bytes();
        assert!(
            bytes.len() > 2_000_000,
            "test input should be multiple MB, got {} bytes",
            bytes.len()
        );

        let mut reader = ChunkedWmlReader::new();
        let mut event_count = 0usize;
        let mut max_pending = 0usize;
        for chunk in bytes.chunks(256) {
            reader.feed(chunk, &mut |_ev| event_count += 1);
            max_pending = max_pending.max(reader.pending_len());
        }
        let diag = reader.finish(&mut |_ev| event_count += 1);
        assert!(diag.is_none(), "unexpected diagnostic: {diag:?}");

        // 20,000 paragraphs each emit >= 6 events (StartParagraph, StartRun,
        // Text, EndRun, EndParagraph, plus StartDocument/EndDocument once) —
        // confirm parsing actually happened, not just "nothing crashed".
        assert!(
            event_count > 100_000,
            "expected many events, got {event_count}"
        );

        // The real assertion: pending never grew anywhere near the ~2MB+
        // document size. A generous bound (a few KB) comfortably covers the
        // largest single props element / lookahead chain in this fixture
        // while still being orders of magnitude below the document size.
        assert!(
            max_pending < 8192,
            "pending buffer grew to {max_pending} bytes — memory bound regressed \
             (document is {} bytes)",
            bytes.len()
        );
    }
}
