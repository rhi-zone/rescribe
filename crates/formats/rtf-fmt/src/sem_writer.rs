//! Streaming RTF writer — converts a stream of semantic [`Event`]s directly
//! to RTF bytes.
//!
//! [`Writer`] never constructs an [`crate::ast::RtfDoc`] and never calls
//! [`crate::emit::emit`]. It is a second, independent emission path from the
//! tree-based builder, not a wrapper around it — mirroring rst-fmt's
//! `writer::Writer` (see that crate's module doc for the general technique
//! this is modeled on).
//!
//! # The font/color table problem
//!
//! RTF requires `\fonttbl`/`\colortbl` to be declared in the document header,
//! *before* any `\f<n>`/`\cf<n>` body reference — but which fonts/colors are
//! used, and in what order, can only be known by having walked the whole
//! document. A writer that discovered fonts/colors as it went would have to
//! buffer the entire body until the last event, to learn the final table,
//! before it could write the header — `O(document size)` memory, exactly the
//! defect this rewrite exists to remove.
//!
//! [`crate::sem_events::events`] resolves this for free: it already parses
//! the complete document into an `RtfDoc` before yielding its first event
//! (see that module's memory note), so computing the font/color tables
//! ([`crate::tables::build_font_map`]/[`crate::tables::build_color_map`])
//! at that same point costs nothing extra in *retained* memory — only the
//! small, bounded-size table itself is kept, not the document. That
//! computation is carried in `Event::StartDocument`, always the first event.
//! `Writer` reads it once, writes the header immediately, and never needs to
//! discover a font or color mid-stream — every `StartFont`/`StartColor`/
//! `StartBgColor` event's index is a lookup into a table it already has in
//! full.
//!
//! # Buffer model
//!
//! One growing output buffer (`Writer::out`) for the whole document. A small
//! `Vec<Ctx>` context stack (`O(nesting depth)`) tracks the handful of
//! places RTF's serialization depends on more than the current event alone:
//!
//! - **Blockquote / list-item paragraph elision.** `emit()` wraps a
//!   `Paragraph` in `\pard...\par` only when it is *not* a direct child of a
//!   `Blockquote`/`ListItem` — those containers write one shared
//!   `\pard...\par` around all of their children instead. Since the event
//!   stream reports a `Paragraph` the same way regardless of its parent,
//!   `Writer` must remember what the nearest enclosing block-level container
//!   was; that's what `Ctx::Blockquote`/`Ctx::ListItem` on the stack are for.
//!   `Ctx::List` similarly remembers `ordered` between `StartList` and each
//!   `StartListItem`. Entering `StartFootnote` resets the paragraph mode back
//!   to full-wrapper (`Ctx::Normal`), matching `emit_inline`'s `Footnote`
//!   arm, which calls `emit_block` unconditionally for its content
//!   regardless of the outer context.
//! - **Table row cell count.** `\trowd` must be followed by one `\cellx<N>`
//!   per cell, but the cell count for a row isn't known until `EndTableRow`.
//!   Cell content doesn't need to wait, though: `StartTableRow` writes
//!   `\trowd ` and records a mark right after it; each cell writes straight
//!   through; `EndTableRow` inserts the now-known `\cellx` list at that mark
//!   (a single `String::insert_str`, bounded by the row's own size, not the
//!   document's — the same "write-through + one in-place insert" technique
//!   rst-fmt uses for headings/figure captions).
//! - **Link empty-children fallback.** `emit_inline`'s `Link` arm falls back
//!   to the URL as display text when `children` is empty; the event stream
//!   doesn't say upfront whether children will turn out empty.
//!   `StartLink`/`EndLink` use the same in-place-insert technique: write the
//!   URL fallback into `out` only if nothing was written between them.
//!
//! Every other construct (headings, code blocks, bold/italic/…, inline
//! color/font/lang spans, footnotes) is write-through: the exact bytes to
//! emit are fully known at `Start*`/`End*` time from that event's own
//! payload plus the (already fully known) font/color tables, with no
//! lookahead and no buffering.
//!
//! `out` is flushed to the sink and cleared whenever the context stack
//! becomes empty (i.e. control has returned to the top level, between two
//! top-level blocks, or right after the header) — so memory is
//! `O(largest top-level block/table-row + nesting depth)`, never
//! `O(full document)`.

use crate::ast::Align;
use crate::sem_events::OwnedEvent as Event;
use std::io::Write;

/// A block-level (or link) context the writer is currently nested inside,
/// tracked so events later in the stream can be serialized correctly
/// without re-deriving information the event itself doesn't carry. See the
/// module doc's "Buffer model" section for what each variant is for.
enum Ctx {
    /// Full-wrapper zone: `Paragraph` here always gets its own
    /// `\pard...\par`. Pushed for `Footnote` content specifically, so that
    /// footnote content nested inside a `Blockquote`/`ListItem` doesn't
    /// inherit their bare-paragraph mode — matching `emit_inline`'s
    /// `Footnote` arm, which calls `emit_block` unconditionally.
    Normal,
    /// Direct child of `Blockquote`: a `Paragraph` here elides its own
    /// `\pard...\par` wrapper (the enclosing `Blockquote` supplies one
    /// shared wrapper for all of its children).
    Blockquote,
    /// Between `StartList` and its matching `EndList`; remembers `ordered`
    /// for the next `StartListItem`.
    List { ordered: bool },
    /// Direct child of a list item: a `Paragraph` here elides its own
    /// wrapper the same way `Blockquote`'s direct children do.
    ListItem,
    /// A `Paragraph` frame; `bare` records which wrapper mode `StartParagraph`
    /// chose, so `EndParagraph` knows whether to close it.
    Paragraph { bare: bool },
    /// Buffering a table row: `mark` is the offset in `out` right after the
    /// `"\trowd "` literal, where the `\cellx` list gets inserted once the
    /// cell count is known; `cell_count` counts cells seen so far.
    TableRow { mark: usize, cell_count: usize },
    /// A `Link`: `mark` is the offset in `out` right after the opening
    /// `\fldrslt ` literal. If nothing was written by `EndLink`, the URL is
    /// inserted there as the fallback display text.
    Link { mark: usize, url: String },
}

/// Streaming RTF writer.
///
/// Feed [`Event`]s (the same type [`crate::sem_events::events`] and
/// [`crate::batch::StreamingParser`] produce) with
/// [`write_event`](Writer::write_event), then call [`finish`](Writer::finish)
/// to flush any remaining buffered bytes and recover the sink.
///
/// # Example
/// ```no_run
/// use rtf_fmt::sem_writer::Writer;
/// use rtf_fmt::{Align, OwnedEvent as Event};
/// use std::borrow::Cow;
///
/// let mut w = Writer::new(Vec::<u8>::new());
/// w.write_event(Event::StartDocument { fonts: vec!["Times New Roman".into()], colors: vec![] });
/// w.write_event(Event::StartParagraph { align: Align::Default, para_props: Cow::Borrowed("") });
/// w.write_event(Event::Text(Cow::Borrowed("Hello")));
/// w.write_event(Event::EndParagraph);
/// w.write_event(Event::EndDocument);
/// let bytes = w.finish();
/// ```
pub struct Writer<W: Write> {
    sink: W,
    out: String,
    color_map: Vec<(u8, u8, u8)>,
    font_map: Vec<String>,
    ctx: Vec<Ctx>,
    /// Stack of pending `\shprslt` fallback payloads, one per currently-open
    /// `StartShape`/`EndShape` pair (shapes are not expected to nest in
    /// practice, but a stack keeps this correct if they ever do). Deferred
    /// to `EndShape` because `\shprslt` is written *after* `\shptxt`'s
    /// content, which arrives as events between `StartShape` and `EndShape`.
    shape_fallback: Vec<String>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            out: String::new(),
            color_map: Vec::new(),
            font_map: Vec::new(),
            ctx: Vec::new(),
            shape_fallback: Vec::new(),
        }
    }

    /// Write `s` with RTF special-character escaping — identical policy to
    /// `emit.rs`'s `Ctx::push_escaped`, so body text matches byte-for-byte.
    fn push_escaped(&mut self, s: &str) {
        self.out.push_str(&escape_rtf(s));
    }

    /// Flush `out` to the sink if the context stack is empty (i.e. we're
    /// currently at the top level — between top-level blocks, or right
    /// after the header). Called after every event; see the module doc's
    /// memory-bound explanation.
    fn maybe_flush(&mut self) {
        if self.ctx.is_empty() && !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Write one semantic event.
    pub fn write_event(&mut self, event: Event) {
        self.handle(event);
        self.maybe_flush();
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::StartDocument { fonts, colors } => {
                self.font_map = fonts;
                self.color_map = colors;
                self.out.push_str("{\\rtf1\\ansi\\deff0");
                self.out.push_str("{\\fonttbl");
                for (i, name) in self.font_map.iter().enumerate() {
                    self.out.push_str(&format!("{{\\f{i} {name};}}"));
                }
                self.out.push('}');
                if !self.color_map.is_empty() {
                    self.out.push_str("{\\colortbl;");
                    for (r, g, b) in &self.color_map {
                        self.out.push_str(&format!("\\red{r}\\green{g}\\blue{b};"));
                    }
                    self.out.push('}');
                }
                self.out.push('\n');
            }
            // `Writer` has already written and (per `maybe_flush`) likely
            // already flushed the `\fonttbl`/`\colortbl` header by the time
            // this arrives (it's the second-to-last event) — correcting it
            // in place would require buffering the header, exactly what this
            // writer exists to avoid. No-op: every body `StartFont`/
            // `StartColor`/`StartBgColor` event already resolved its index
            // against `StartDocument`'s table, and that resolution is
            // self-consistent by construction (the header was written from
            // the same table), so the output RTF is valid regardless of
            // whether it used the true first-use order. See
            // `Event::TableOrderResolved`'s doc comment.
            Event::TableOrderResolved { .. } => {}

            Event::EndDocument => {
                self.out.push('}');
            }

            // ── Blocks ───────────────────────────────────────────────────
            Event::StartParagraph { align, para_props } => {
                let bare = matches!(self.ctx.last(), Some(Ctx::Blockquote) | Some(Ctx::ListItem));
                if !bare {
                    self.out.push_str("\\pard");
                    match align {
                        Align::Left => self.out.push_str("\\ql"),
                        Align::Right => self.out.push_str("\\qr"),
                        Align::Center => self.out.push_str("\\qc"),
                        Align::Justify => self.out.push_str("\\qj"),
                        Align::Default => {}
                    }
                    if !para_props.is_empty() {
                        self.out.push_str(&para_props);
                    }
                    self.out.push(' ');
                }
                self.ctx.push(Ctx::Paragraph { bare });
            }
            Event::EndParagraph => {
                if let Some(Ctx::Paragraph { bare }) = self.ctx.pop()
                    && !bare
                {
                    self.out.push_str("\\par\n");
                }
            }

            Event::StartHeading { level } => {
                let size = match level {
                    1 => 48,
                    2 => 40,
                    3 => 32,
                    4 => 28,
                    _ => 24,
                };
                self.out.push_str(&format!("\\pard\\fs{size} \\b "));
            }
            Event::EndHeading => {
                self.out.push_str("\\b0\\par\n");
            }

            Event::StartCodeBlock => {
                self.out.push_str("\\pard\\f1\\fs20 ");
            }
            Event::CodeBlockContent(content) => {
                for line in content.lines() {
                    self.push_escaped(line);
                    self.out.push_str("\\line ");
                }
            }
            Event::EndCodeBlock => {
                self.out.push_str("\\f0\\par\n");
            }

            Event::StartBlockquote => {
                self.out.push_str("\\pard\\li720 ");
                self.ctx.push(Ctx::Blockquote);
            }
            Event::EndBlockquote => {
                self.ctx.pop();
                self.out.push_str("\\par\n");
            }

            Event::StartList { ordered } => {
                self.ctx.push(Ctx::List { ordered });
            }
            Event::EndList => {
                self.ctx.pop();
            }
            Event::StartListItem => {
                let ordered = matches!(self.ctx.last(), Some(Ctx::List { ordered: true }));
                self.out.push_str("\\pard ");
                if ordered {
                    self.out.push_str(
                        "{\\*\\pn\\pnlvlbody\\pnf0\\pnindent0\\pnstart1\\pndec{\\pntxta.}}\\fi-360\\li720 ",
                    );
                } else {
                    self.out.push_str(
                        "{\\*\\pn\\pnlvlblt\\pnf2\\pnindent0{\\pntxtb\\'B7}}\\fi-360\\li720 ",
                    );
                }
                self.ctx.push(Ctx::ListItem);
            }
            Event::EndListItem => {
                self.ctx.pop();
                self.out.push_str("\\par\n");
            }

            Event::StartTable | Event::EndTable => {}
            Event::StartTableRow => {
                self.out.push_str("\\trowd ");
                let mark = self.out.len();
                self.ctx.push(Ctx::TableRow {
                    mark,
                    cell_count: 0,
                });
            }
            Event::EndTableRow => {
                if let Some(Ctx::TableRow { mark, cell_count }) = self.ctx.pop() {
                    let mut cellx = String::new();
                    for i in 0..cell_count {
                        let right = (i + 1) * 2000;
                        cellx.push_str(&format!("\\cellx{right}"));
                    }
                    self.out.insert_str(mark, &cellx);
                }
                self.out.push_str("\\row\n");
            }
            Event::StartTableCell => {
                self.out.push_str("\\pard\\intbl ");
            }
            Event::EndTableCell => {
                self.out.push_str("\\cell ");
                if let Some(Ctx::TableRow { cell_count, .. }) = self.ctx.last_mut() {
                    *cell_count += 1;
                }
            }

            Event::HorizontalRule => {
                self.out
                    .push_str("\\pard\\brdrb\\brdrs\\brdrw10\\brsp20 \\par\n");
            }

            // ── Inlines ──────────────────────────────────────────────────
            Event::Text(t) => self.push_escaped(&t),
            Event::LineBreak => self.out.push_str("\\line "),
            Event::SoftBreak => self.out.push(' '),

            Event::StartBold => self.out.push_str("{\\b "),
            Event::EndBold => self.out.push('}'),
            Event::StartItalic => self.out.push_str("{\\i "),
            Event::EndItalic => self.out.push('}'),
            Event::StartUnderline => self.out.push_str("{\\ul "),
            Event::EndUnderline => self.out.push('}'),
            Event::StartStrikethrough => self.out.push_str("{\\strike "),
            Event::EndStrikethrough => self.out.push('}'),
            Event::StartSuperscript => self.out.push_str("{\\super "),
            Event::EndSuperscript => self.out.push('}'),
            Event::StartSubscript => self.out.push_str("{\\sub "),
            Event::EndSubscript => self.out.push('}'),
            Event::StartAllCaps => self.out.push_str("{\\caps "),
            Event::EndAllCaps => self.out.push('}'),
            Event::StartSmallCaps => self.out.push_str("{\\scaps "),
            Event::EndSmallCaps => self.out.push('}'),
            Event::StartHidden => self.out.push_str("{\\v "),
            Event::EndHidden => self.out.push('}'),

            Event::Code(text) => {
                self.out.push_str("{\\f1 ");
                self.push_escaped(&text);
                self.out.push('}');
            }

            Event::StartLink { url } => {
                self.out.push_str("{\\field{\\*\\fldinst HYPERLINK \"");
                self.out.push_str(&url);
                self.out.push_str("\"}{\\fldrslt ");
                let mark = self.out.len();
                self.ctx.push(Ctx::Link { mark, url });
            }
            Event::EndLink => {
                if let Some(Ctx::Link { mark, url }) = self.ctx.pop()
                    && self.out.len() == mark
                {
                    self.push_escaped_at(mark, &url);
                }
                self.out.push_str("}}");
            }

            Event::Image { url, alt } => {
                let label = if !alt.is_empty() { &alt } else { &url };
                self.out.push_str("[Image: ");
                self.push_escaped(label);
                self.out.push(']');
            }

            Event::StartFontSize { size } => {
                self.out.push_str(&format!("{{\\fs{size} "));
            }
            Event::EndFontSize => self.out.push('}'),

            Event::StartColor { r, g, b } => {
                let idx = self
                    .color_map
                    .iter()
                    .position(|c| *c == (r, g, b))
                    .map(|i| i + 1)
                    .unwrap_or(0);
                self.out.push_str(&format!("{{\\cf{idx} "));
            }
            Event::EndColor => self.out.push('}'),

            Event::StartBgColor { r, g, b } => {
                let idx = self
                    .color_map
                    .iter()
                    .position(|c| *c == (r, g, b))
                    .map(|i| i + 1)
                    .unwrap_or(0);
                self.out.push_str(&format!("{{\\cb{idx} "));
            }
            Event::EndBgColor => self.out.push('}'),

            Event::StartCharSpan { char_props } => {
                self.out.push('{');
                self.out.push_str(&char_props);
                self.out.push(' ');
            }
            Event::EndCharSpan => self.out.push('}'),

            Event::StartFont { name } => {
                let idx = self.font_map.iter().position(|f| *f == name).unwrap_or(0);
                self.out.push_str(&format!("{{\\f{idx} "));
            }
            Event::EndFont => self.out.push('}'),

            Event::StartLang { lcid } => {
                self.out.push_str(&format!("{{\\lang{lcid} "));
            }
            Event::EndLang => self.out.push('}'),

            Event::StartFootnote => {
                self.out.push_str("{\\footnote ");
                self.ctx.push(Ctx::Normal);
            }
            Event::EndFootnote => {
                self.ctx.pop();
                self.out.push('}');
            }

            Event::StartShape {
                x,
                y,
                width,
                height,
                z_order,
                shape_props,
                named_props,
                fallback_raw,
            } => {
                // Same reconstruction as emit.rs's emit_shape: shpright/
                // shpbottom are the exact inverse of how parse.rs derived
                // x/y/width/height, so the numeric fields round-trip
                // byte-for-byte through parse → events → this writer.
                self.out.push_str("{\\shp{\\*\\shpinst");
                self.out.push_str(&format!(
                    "\\shpleft{x}\\shptop{y}\\shpright{}\\shpbottom{}\\shpz{z_order}",
                    x + width,
                    y + height,
                ));
                self.out.push_str(&shape_props);
                for (name, value) in &named_props {
                    self.out.push_str("{\\sp{\\sn ");
                    self.out.push_str(name);
                    self.out.push_str("}{\\sv ");
                    self.out.push_str(value);
                    self.out.push_str("}}");
                }
                self.out.push('}');
                self.shape_fallback.push(fallback_raw);
                // `\shptxt`'s content, if any, is written by the following
                // block events (same nested-Paragraph mechanism as
                // `StartFootnote`); open its wrapper group now and let
                // `EndShape` close it before appending `\shprslt`/closing `}`.
                self.out.push_str("{\\shptxt ");
                self.ctx.push(Ctx::Normal);
            }
            Event::EndShape => {
                self.ctx.pop();
                self.out.push('}'); // close {\shptxt ...
                if let Some(fallback_raw) = self.shape_fallback.pop()
                    && !fallback_raw.is_empty()
                {
                    self.out.push_str("{\\shprslt");
                    self.out.push_str(&fallback_raw);
                    self.out.push('}');
                }
                self.out.push('}'); // close {\shp ...
            }
        }
    }

    /// Insert `s`, RTF-escaped (same policy as [`Writer::push_escaped`]), at
    /// byte offset `at` in `out`.
    fn push_escaped_at(&mut self, at: usize, s: &str) {
        self.out.insert_str(at, &escape_rtf(s));
    }

    /// Flush any remaining buffered bytes and return the sink.
    pub fn finish(mut self) -> W {
        let _ = self.sink.write_all(self.out.as_bytes());
        self.sink
    }
}

/// RTF special-character escaping — identical policy to `emit.rs`'s
/// `Ctx::push_escaped`, factored out as a free function so both direct
/// write-through (`Writer::push_escaped`) and in-place insertion
/// (`Writer::push_escaped_at`, for the `Link` empty-children fallback) share
/// one implementation.
fn escape_rtf(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\t' => out.push_str("\\tab "),
            '\n' => out.push_str("\\'0a"),
            '\r' => out.push_str("\\'0d"),
            '\u{00A0}' => out.push_str("\\~"),
            '\u{2002}' => out.push_str("\\enspace "),
            '\u{2003}' => out.push_str("\\emspace "),
            '\u{2014}' => out.push_str("\\emdash "),
            '\u{2013}' => out.push_str("\\endash "),
            '\u{2018}' => out.push_str("\\lquote "),
            '\u{2019}' => out.push_str("\\rquote "),
            '\u{201C}' => out.push_str("\\ldblquote "),
            '\u{201D}' => out.push_str("\\rdblquote "),
            '\u{2022}' => out.push_str("\\bullet "),
            c if c.is_ascii() => out.push(c),
            c => {
                out.push_str(&format!("\\u{}?", c as u32));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem_events::events;

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartDocument {
            fonts: vec!["Times New Roman".to_string()],
            colors: vec![],
        });
        w.write_event(Event::StartParagraph {
            align: Align::Default,
            para_props: "".into(),
        });
        w.write_event(Event::Text("Hello".into()));
        w.write_event(Event::EndParagraph);
        w.write_event(Event::EndDocument);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("{\\rtf1"));
        assert!(s.contains("Hello"));
        assert!(s.contains("\\par"));
    }

    /// The whole point of this rewrite: `Writer` fed by `events()` must
    /// reproduce `emit()`'s exact canonical bytes, for every construct
    /// `emit()` knows how to write — not just plain paragraphs.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            r"{\rtf1 Hello world\par}",
            r"{\rtf1 {\b bold} and {\i italic} and {\ul underline} and {\strike struck}\par}",
            r"{\rtf1\pard\qc centered\par}",
            r"{\rtf1\pard\qr\li720\sa200 indented right\par}",
            r"{\rtf1\outlinelevel0 Heading Text\par}",
            r"{\rtf1{\dn3 lowered}\par}",
            r"{\rtf1{\shad shadowed}\par}",
            r"{\rtf1{\colortbl;\red0\green128\blue0;}\cf1 green text\par}",
            r"{\rtf1{\colortbl;\red255\green0\blue0;}\cb1 bg red\par}",
            r"{\rtf1\fs36 medium text\par}",
            "{\\rtf1{\\field{\\*\\fldinst HYPERLINK \"http://example.com\"}{\\fldrslt click}}\\par}",
            "{\\rtf1{\\field{\\*\\fldinst HYPERLINK \"http://example.com\"}{\\fldrslt}}\\par}",
            r"{\rtf1{\super super}{\sub sub}\par}",
            r"{\rtf1{\caps caps}{\scaps scaps}{\v hidden}\par}",
            r"{\rtf1{\lang1031 german text}\par}",
            "{\\rtf1{\\footnote\\pard footnote body\\par}main text\\par}",
        ];
        for input in inputs {
            let (doc, _diags) = crate::parse::parse(input.as_bytes());
            let built = crate::emit::emit(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in events(input.as_bytes()) {
                w.write_event(e);
            }
            let streamed = String::from_utf8(w.finish()).expect("utf8");
            assert_eq!(
                built, streamed,
                "diverged for input {input:?}\n  build():  {built:?}\n  streamed: {streamed:?}"
            );
        }
    }

    /// Table row cell-count deferral (`\trowd`'s `\cellx` list) is the one
    /// genuinely content-dependent-prefix construct — this exercises it with
    /// varying cell counts per row.
    #[test]
    fn test_writer_byte_identical_table() {
        let input = r"{\rtf1\trowd\cellx2000\cellx4000\pard\intbl a1\cell\pard\intbl a2\cell\row\trowd\cellx2000\cellx4000\cellx6000\pard\intbl b1\cell\pard\intbl b2\cell\pard\intbl b3\cell\row}";
        let (doc, _diags) = crate::parse::parse(input.as_bytes());
        let built = crate::emit::emit(&doc);
        let mut w = Writer::new(Vec::<u8>::new());
        for e in events(input.as_bytes()) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("utf8");
        assert_eq!(built, streamed);
    }

    /// Blockquote/list-item paragraph elision — the one context-dependent
    /// serialization decision the event stream alone doesn't carry.
    #[test]
    fn test_writer_byte_identical_blockquote_and_list() {
        let inputs = [
            r"{\rtf1\pard\li720 quoted text\par}",
            "{\\rtf1{\\*\\pn\\pnlvlblt\\pnf2\\pnindent0{\\pntxtb\\'B7}}\\fi-360\\li720 bullet item\\par}",
        ];
        for input in inputs {
            let (doc, _diags) = crate::parse::parse(input.as_bytes());
            let built = crate::emit::emit(&doc);
            let mut w = Writer::new(Vec::<u8>::new());
            for e in events(input.as_bytes()) {
                w.write_event(e);
            }
            let streamed = String::from_utf8(w.finish()).expect("utf8");
            assert_eq!(built, streamed, "diverged for input {input:?}");
        }
    }

    /// A genuine incremental writer must emit some bytes before `finish()`
    /// — a buffer-then-emit wrapper (the pattern CLAUDE.md explicitly
    /// forbids) emits zero. The header alone (from `StartDocument`) already
    /// proves this, since it's written and flushed before any body event.
    #[test]
    fn test_writer_is_incremental() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));

        struct ObservableSink(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);
        impl Write for ObservableSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.borrow_mut().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut w = Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartDocument {
            fonts: vec!["Times New Roman".to_string()],
            colors: vec![],
        });
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        assert!(
            pre_finish > 0,
            "Writer wrote zero bytes to the sink after StartDocument and before finish()"
        );

        // Also true mid-document: a completed top-level paragraph flushes
        // immediately, well before EndDocument/finish().
        let observed2 = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w2 = Writer::new(ObservableSink(observed2.clone()));
        w2.write_event(Event::StartDocument {
            fonts: vec!["Times New Roman".to_string()],
            colors: vec![],
        });
        w2.write_event(Event::StartParagraph {
            align: Align::Default,
            para_props: "".into(),
        });
        w2.write_event(Event::Text("Hello world".into()));
        w2.write_event(Event::EndParagraph);
        let pre_finish2 = observed2.borrow().len();
        let _ = w2.finish();
        assert!(
            pre_finish2 > pre_finish,
            "completing a top-level paragraph should flush more bytes before finish()"
        );
    }

    /// Peak memory must stay roughly *constant* as document size grows, not
    /// scale with it. Uses a peak-to-peak ratio between a small and a large
    /// run (10x the paragraphs) rather than an absolute byte ceiling:
    /// `cargo test` runs this binary's tests on multiple threads sharing one
    /// process-wide `#[global_allocator]`, so unrelated concurrent
    /// allocations add noise an absolute threshold can't distinguish from a
    /// real regression at these sizes, but genuine unbounded growth still
    /// shows up as a large ratio.
    #[test]
    fn test_writer_peak_memory_bounded() {
        use crate::alloc_probe::{CURRENT_BYTES, PEAK_BYTES};

        struct NullSink;
        impl Write for NullSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        fn run_peak(n: usize) -> usize {
            PEAK_BYTES.with(|p| p.set(0));
            let before = CURRENT_BYTES.with(|c| c.get());
            let mut w = Writer::new(NullSink);
            w.write_event(Event::StartDocument {
                fonts: vec!["Times New Roman".to_string()],
                colors: vec![],
            });
            for i in 0..n {
                w.write_event(Event::StartParagraph {
                    align: Align::Default,
                    para_props: "".into(),
                });
                w.write_event(Event::Text(
                    format!("paragraph number {i} with some filler text to give it realistic size")
                        .into(),
                ));
                w.write_event(Event::EndParagraph);
            }
            w.write_event(Event::EndDocument);
            let _ = w.finish();
            PEAK_BYTES.with(|p| p.get()).saturating_sub(before)
        }

        let small = run_peak(500).max(1);
        let large = run_peak(5000); // 10x the paragraphs

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 20.0,
            "peak memory did not stay roughly constant across document sizes: \
             {small} bytes peak @500 paragraphs -> {large} bytes peak @5000 paragraphs \
             (ratio {ratio:.2}); this suggests the writer is buffering O(document) instead \
             of O(nesting depth)"
        );
    }
}
