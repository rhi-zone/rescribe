//! Incremental, chunk-fed `content.xml` → [`OdfEvent`] driver.
//!
//! This is the resumable counterpart of `events.rs`'s `parse_content_events`:
//! the same flat SAX dispatch (paragraph/heading/span/list/table/etc. opens
//! and closes are emitted the moment their opening or closing tag is seen),
//! restructured so it can be fed `content.xml` bytes incrementally — as they
//! arrive from a ZIP entry's `Data` events — instead of requiring the whole
//! entry buffered up front the way `events::extract_events` does.
//!
//! # Why one scanner covers every subtree-consuming construct
//!
//! `parse_content_events` delegates a handful of leaf constructs to
//! `parser.rs` helpers that consume their own subtree in one call:
//! `office:automatic-styles` (→ [`crate::parser::parse_auto_styles_block`]),
//! `office:annotation`, `text:note-citation`, field elements, an explicit
//! `<text:soft-page-break>…</text:soft-page-break>`, and any unrecognized
//! element (→ raw capture). Each of those helpers, starting right after an
//! element's opening tag, consumes exactly that element's children through
//! to its own matching closing tag — generic XML well-nestedness guarantees
//! the byte range consumed is identical no matter which helper does the
//! consuming. So a single generic depth-tracking pre-scan ([`subtree_end`])
//! determines whether the *complete* subtree for any of them is present in
//! the currently buffered bytes, without needing a bespoke "is this complete
//! yet" check per construct. Once the scan proves completeness, the
//! existing, unmodified `parser.rs`/`events.rs` helper is invoked for real —
//! this module never reimplements their logic, only gates when it is safe
//! to call them against a buffer that might otherwise be a truncated
//! mid-chunk snapshot. [`text_start_consumes_subtree`] mirrors
//! `events::push_text_start_event`'s own dispatch condition for exactly
//! this reason — see its doc comment for the cross-check.
//!
//! # Memory profile
//!
//! O(largest single token + nesting depth) for the flat majority of content
//! (paragraph/span/table/list opens and closes — the same shape as
//! `docbook-fmt`'s `batch.rs`, which this module's drain loop follows
//! closely), **plus** one documented, bounded exception: an
//! `<office:automatic-styles>` block, a single `<office:annotation>`, one
//! unrecognized element's raw capture, one field element's value, or one
//! `<text:note-citation>`/`<text:soft-page-break>` is buffered in full
//! before being processed — bounded by that *one construct's* size, not the
//! whole `content.xml`. This is the same class of documented, scoped
//! exception as `zip-fmt`'s own `Store`+data-descriptor case (see
//! `zip-fmt/src/batch.rs`'s module docs): the common case (body text) is
//! genuinely streamed token-by-token; a specific, named, typically-small
//! minority is not.

use std::collections::VecDeque;

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::events::{
    BodyKind, FIELD_ELEMENTS, INLINE_RECOGNIZED, OdfEvent, get_attr, push_end_event,
    push_presentation_start_event, push_spreadsheet_start_event, push_text_start_event,
};

/// Whether `name`, encountered as a text-body `Start` tag with the given
/// `in_inline` context, causes `events::push_text_start_event` to consume
/// its own subtree via the reader (and therefore needs [`subtree_end`]
/// gating before it is safe to call against a possibly-truncated buffer).
///
/// Mirrors `push_text_start_event`'s dispatch condition exactly:
/// - the raw-capture guard (`in_inline && !INLINE_RECOGNIZED && !FIELD_ELEMENTS`)
/// - the explicit recursing arms (`text:note-citation`, `office:annotation`,
///   `text:soft-page-break`, any `FIELD_ELEMENTS` member)
/// - the catch-all `_ =>` arm (raw capture), which fires for every name not
///   in the explicit non-recursing arm list below
///
/// Kept as a separate pure predicate (rather than threading a "would
/// consume" flag out of `push_text_start_event` itself) so this module
/// never touches the fixture-verified `parser()`/`events()` code paths —
/// the fixture-driven cross-check in `content_stream` tests (and the
/// crate's existing fixture suite, exercised via the `StreamingParse`
/// vs. `events()`/`parse()` equivalence check) is the regression net for
/// keeping the two in sync.
fn text_start_consumes_subtree(name: &str, in_inline: bool) -> bool {
    if in_inline && !INLINE_RECOGNIZED.contains(&name) && !FIELD_ELEMENTS.contains(&name) {
        return true;
    }
    matches!(
        name,
        "text:note-citation" | "office:annotation" | "text:soft-page-break"
    ) || FIELD_ELEMENTS.contains(&name)
        || !matches!(
            name,
            "text:p"
                | "text:h"
                | "text:span"
                | "text:a"
                | "text:list"
                | "text:list-item"
                | "text:list-header"
                | "table:table-header-rows"
                | "table:table"
                | "table:table-row"
                | "table:table-cell"
                | "table:covered-table-cell"
                | "text:note"
                | "text:note-body"
                | "draw:frame"
                | "draw:text-box"
                | "draw:image"
        )
}

/// Build a `Reader` over a byte fragment that starts mid-document (either
/// right after some other, already-consumed opening tag, or — for the
/// top-level drain loop — after a prefix already consumed by a *previous*
/// `Reader` instance; see `drain()`'s doc comment). `quick_xml`'s default
/// `check_end_names`/`allow_unmatched_ends` validation tracks open tags
/// across calls on the *same* `Reader` instance, so a fresh instance that
/// starts mid-subtree has no record of the enclosing element(s) and would
/// raise a spurious `IllFormed(UnmatchedEndTag)` the moment it reaches
/// their closing tag. Both checks are disabled here for exactly the reason
/// `docbook-fmt`'s `batch.rs` disables them in its own per-call fresh
/// `Reader`: tag balance is instead enforced by hand (this module's
/// generic depth counters, or `parser.rs`/`events.rs`'s own explicit
/// `name == end_tag` checks), not by `quick_xml`'s cross-call state.
fn fragment_reader(bytes: &[u8]) -> Reader<&[u8]> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = false;
    reader.config_mut().allow_unmatched_ends = true;
    reader
}

/// Scan `remaining`, which must start immediately *after* an element's
/// already-consumed opening tag, for the byte offset just past that
/// element's matching closing tag. Returns `None` if the buffered bytes run
/// out (cleanly or mid-token) before the closing tag is found — the caller
/// must wait for more input.
fn subtree_end(remaining: &[u8]) -> Option<usize> {
    let mut reader = fragment_reader(remaining);
    let mut buf = Vec::new();
    let mut depth: u32 = 0;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(_)) => depth += 1,
            Ok(XmlEvent::Empty(_)) => {}
            Ok(XmlEvent::End(_)) => {
                if depth == 0 {
                    return Some(reader.buffer_position() as usize);
                }
                depth -= 1;
            }
            Ok(XmlEvent::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

/// Resumable state for driving `content.xml` bytes, fed incrementally, into
/// [`OdfEvent`]s. See the module docs for the overall strategy.
pub(crate) struct ContentDriver {
    pending: Vec<u8>,
    in_body: bool,
    body_kind: BodyKind,
    /// Content model of each open text-body container: `true` = inline.
    text_ctx: Vec<bool>,
    /// Parallel element-name stack, matched against closing tags.
    open_names: Vec<String>,
}

impl ContentDriver {
    pub(crate) fn new() -> Self {
        ContentDriver {
            pending: Vec::new(),
            in_body: false,
            body_kind: BodyKind::None,
            text_ctx: Vec::new(),
            open_names: Vec::new(),
        }
    }

    /// Feed the next chunk of `content.xml` bytes, dispatching every event
    /// that can be proven complete from the bytes seen so far.
    pub(crate) fn feed(&mut self, chunk: &[u8], out: &mut VecDeque<OdfEvent<'static>>) {
        self.pending.extend_from_slice(chunk);
        self.drain(out, false);
    }

    /// Signal that no more `content.xml` bytes are coming (the ZIP entry
    /// ended): drain whatever remains, permissively — matching
    /// `parse_content_events`'s own `Eof | Err(_) => break` tolerance for a
    /// truncated/malformed document.
    pub(crate) fn finish(&mut self, out: &mut VecDeque<OdfEvent<'static>>) {
        self.drain(out, true);
    }

    fn drain(&mut self, out: &mut VecDeque<OdfEvent<'static>>, is_final: bool) {
        loop {
            if self.pending.is_empty() {
                return;
            }

            let mut reader = fragment_reader(&self.pending[..]);
            let mut buf = Vec::new();
            let total_len = self.pending.len();

            match reader.read_event_into(&mut buf) {
                Ok(XmlEvent::Eof) => {
                    if is_final {
                        self.pending.clear();
                    }
                    return;
                }
                Ok(XmlEvent::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let tag_end = reader.buffer_position() as usize;

                    if name == "office:automatic-styles" {
                        let Some(rel_end) = subtree_end(&self.pending[tag_end..]) else {
                            return;
                        };
                        let total = tag_end + rel_end;
                        let mut sub = fragment_reader(&self.pending[tag_end..total]);
                        let (styles, list_styles) =
                            crate::parser::parse_auto_styles_block(&mut sub);
                        for style in styles {
                            out.push_back(OdfEvent::AutomaticStyle(style));
                        }
                        for (style_name, is_ordered) in list_styles {
                            out.push_back(OdfEvent::ListStyle(style_name, is_ordered));
                        }
                        self.pending.drain(0..total);
                        continue;
                    }

                    if name == "office:body" {
                        self.in_body = true;
                        self.pending.drain(0..tag_end);
                        continue;
                    }

                    if self.in_body && self.body_kind == BodyKind::None {
                        match name.as_str() {
                            "office:text" => {
                                self.body_kind = BodyKind::Text;
                                out.push_back(OdfEvent::StartText);
                                self.pending.drain(0..tag_end);
                                continue;
                            }
                            "office:spreadsheet" => {
                                self.body_kind = BodyKind::Spreadsheet;
                                out.push_back(OdfEvent::StartSpreadsheet);
                                self.pending.drain(0..tag_end);
                                continue;
                            }
                            "office:presentation" => {
                                self.body_kind = BodyKind::Presentation;
                                out.push_back(OdfEvent::StartPresentation);
                                self.pending.drain(0..tag_end);
                                continue;
                            }
                            _ => {}
                        }
                    }

                    match self.body_kind {
                        BodyKind::Text => {
                            let attrs = crate::parser::collect_attrs(e);
                            let in_inline = self.text_ctx.last().copied().unwrap_or(false);
                            if text_start_consumes_subtree(&name, in_inline) {
                                let Some(rel_end) = subtree_end(&self.pending[tag_end..]) else {
                                    return;
                                };
                                let total = tag_end + rel_end;
                                let mut sub = fragment_reader(&self.pending[tag_end..total]);
                                push_text_start_event(out, &name, &attrs, &mut sub, in_inline);
                                self.pending.drain(0..total);
                                continue;
                            }
                            // Doesn't touch the reader beyond the opening
                            // tag already consumed above.
                            let mut dummy = fragment_reader(&b""[..]);
                            let consumed =
                                push_text_start_event(out, &name, &attrs, &mut dummy, in_inline);
                            debug_assert!(
                                !consumed,
                                "text_start_consumes_subtree predicate diverged from \
                                 push_text_start_event for element `{name}`"
                            );
                            if crate::events::INLINE_CONTAINERS.contains(&name.as_str()) {
                                self.text_ctx.push(true);
                                self.open_names.push(name);
                            } else if crate::events::BLOCK_CONTAINERS.contains(&name.as_str()) {
                                self.text_ctx.push(false);
                                self.open_names.push(name);
                            }
                            self.pending.drain(0..tag_end);
                            continue;
                        }
                        BodyKind::Spreadsheet => {
                            push_spreadsheet_start_event(out, &name, e);
                        }
                        BodyKind::Presentation => {
                            push_presentation_start_event(out, &name, e);
                        }
                        BodyKind::None => {}
                    }
                    self.pending.drain(0..tag_end);
                }
                Ok(XmlEvent::Empty(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let consumed = reader.buffer_position() as usize;
                    handle_empty(out, &name, e, self.in_body, self.body_kind);
                    self.pending.drain(0..consumed);
                }
                Ok(XmlEvent::End(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let consumed = reader.buffer_position() as usize;
                    match name.as_str() {
                        "office:text" if self.body_kind == BodyKind::Text => {
                            self.body_kind = BodyKind::None;
                            out.push_back(OdfEvent::EndText);
                        }
                        "office:spreadsheet" if self.body_kind == BodyKind::Spreadsheet => {
                            self.body_kind = BodyKind::None;
                            out.push_back(OdfEvent::EndSpreadsheet);
                        }
                        "office:presentation" if self.body_kind == BodyKind::Presentation => {
                            self.body_kind = BodyKind::None;
                            out.push_back(OdfEvent::EndPresentation);
                        }
                        "office:body" => {
                            self.in_body = false;
                        }
                        _ if self.body_kind != BodyKind::None => {
                            if self.open_names.last().is_some_and(|n| *n == name) {
                                self.open_names.pop();
                                self.text_ctx.pop();
                            }
                            push_end_event(out, &name, self.body_kind);
                        }
                        _ => {}
                    }
                    self.pending.drain(0..consumed);
                }
                Ok(XmlEvent::Text(ref t)) => {
                    let consumed = reader.buffer_position() as usize;
                    // A `Text` token that consumed all currently-buffered
                    // bytes is ambiguous: quick-xml can't tell whether the
                    // source's next byte is really `<` (the run is
                    // complete) or more text is still to come in the next
                    // chunk. Wait for more input rather than risk emitting
                    // a split text run that `events()`/`parse()` (which see
                    // the whole buffer at once) would have emitted as one.
                    if consumed == total_len && !is_final {
                        return;
                    }
                    if self.body_kind != BodyKind::None {
                        let text = t.decode().unwrap_or_default().into_owned();
                        if !text.is_empty() {
                            out.push_back(OdfEvent::Text(text.into()));
                        }
                    }
                    self.pending.drain(0..consumed);
                }
                Ok(XmlEvent::GeneralRef(ref e)) => {
                    let consumed = reader.buffer_position() as usize;
                    if self.body_kind != BodyKind::None {
                        let text = crate::parser::decode_general_ref(e);
                        if !text.is_empty() {
                            out.push_back(OdfEvent::Text(text.into()));
                        }
                    }
                    self.pending.drain(0..consumed);
                }
                Ok(_) => {
                    let consumed = reader.buffer_position() as usize;
                    self.pending.drain(0..consumed);
                }
                Err(_) => {
                    // A token spanning the chunk boundary (unterminated
                    // tag/comment/CDATA/decl) — wait for more data, unless
                    // this is the final drain, in which case there is no
                    // more data coming: drop the unparseable remainder,
                    // matching `parse_content_events`'s own `Err(_) =>
                    // break` tolerance.
                    if is_final {
                        self.pending.clear();
                    }
                    return;
                }
            }
        }
    }
}

fn handle_empty(
    out: &mut VecDeque<OdfEvent<'static>>,
    name: &str,
    e: &quick_xml::events::BytesStart<'_>,
    in_body: bool,
    body_kind: BodyKind,
) {
    if name == "office:text" && in_body && body_kind == BodyKind::None {
        out.push_back(OdfEvent::StartText);
        out.push_back(OdfEvent::EndText);
        return;
    }
    if body_kind == BodyKind::None {
        return;
    }
    match name {
        "text:line-break" => out.push_back(OdfEvent::LineBreak),
        "text:tab" => out.push_back(OdfEvent::Tab),
        "text:s" => {
            let count = e
                .attributes()
                .flatten()
                .find(|a| a.key.as_ref() == b"text:c")
                .and_then(|a| String::from_utf8_lossy(&a.value).parse::<u32>().ok())
                .unwrap_or(1);
            out.push_back(OdfEvent::Space { count });
        }
        "text:soft-hyphen" => out.push_back(OdfEvent::SoftHyphen),
        "text:soft-page-break" => out.push_back(OdfEvent::SoftPageBreak),
        "text:bookmark" | "text:bookmark-start" => {
            let bm = get_attr(e, b"text:name").unwrap_or_default();
            out.push_back(OdfEvent::Bookmark { name: bm.into() });
        }
        "text:bookmark-end" => {}
        "text:p" => {
            let style_name = get_attr(e, b"text:style-name").map(Into::into);
            out.push_back(OdfEvent::StartParagraph { style_name });
            out.push_back(OdfEvent::EndParagraph);
        }
        "draw:image" => {
            let href = get_attr(e, b"xlink:href")
                .map(Into::into)
                .unwrap_or(std::borrow::Cow::Borrowed(""));
            let mime_type = get_attr(e, b"draw:mime-type").map(Into::into);
            out.push_back(OdfEvent::Image { href, mime_type });
        }
        "table:table-cell" | "table:covered-table-cell" if body_kind == BodyKind::Spreadsheet => {
            push_spreadsheet_start_event(out, name, e);
            push_end_event(out, name, BodyKind::Spreadsheet);
        }
        "table:table-cell" | "table:covered-table-cell" if body_kind == BodyKind::Text => {
            let attrs = crate::parser::collect_attrs(e);
            let mut dummy = fragment_reader(&b""[..]);
            push_text_start_event(out, name, &attrs, &mut dummy, false);
            out.push_back(OdfEvent::EndCell);
        }
        "table:table-column" => {}
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for a real bug caught while building this module:
    /// `fragment_reader`'s readers start mid-document (after some other,
    /// already-consumed opening tag), so `quick_xml`'s default
    /// `check_end_names` raised a spurious `IllFormed(UnmatchedEndTag)` the
    /// moment `subtree_end`/the gated helper calls reached the enclosing
    /// element's own closing tag — silently producing zero events for any
    /// document containing an `<office:automatic-styles>` block (i.e.
    /// nearly every real ODT). See `fragment_reader`'s doc comment for the
    /// fix.
    #[test]
    fn drains_automatic_styles_and_body_content() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-weight="bold"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">bold text</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#;
        let mut driver = ContentDriver::new();
        let mut out = VecDeque::new();
        driver.feed(xml, &mut out);
        driver.finish(&mut out);
        let events: Vec<String> = out.iter().map(|e| format!("{e:?}")).collect();
        assert!(
            events.iter().any(|e| e.starts_with("AutomaticStyle")),
            "expected an AutomaticStyle event, got: {events:#?}"
        );
        assert!(events.contains(&"StartText".to_string()));
        assert!(events.contains(&"EndText".to_string()));
        assert!(
            events
                .iter()
                .any(|e| e == r#"Text("Before ")"# || e.contains("Before")),
            "expected paragraph text, got: {events:#?}"
        );
    }

    /// Same document, fed one byte at a time — exercises the
    /// wait-for-more-data paths in `drain()`/`subtree_end` instead of
    /// always having the whole subtree available on the first call.
    #[test]
    fn drains_incrementally_byte_at_a_time() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:automatic-styles/>
  <office:body>
    <office:text>
      <text:p>Hello, world!</text:p>
    </office:text>
  </office:body>
</office:document-content>"#;
        let mut driver = ContentDriver::new();
        let mut out = VecDeque::new();
        for b in xml {
            driver.feed(std::slice::from_ref(b), &mut out);
        }
        driver.finish(&mut out);
        let events: Vec<String> = out.iter().map(|e| format!("{e:?}")).collect();
        assert!(events.contains(&"StartText".to_string()));
        assert!(events.contains(&"EndText".to_string()));
        assert!(events.iter().any(|e| e.contains("Hello, world!")));
    }
}
