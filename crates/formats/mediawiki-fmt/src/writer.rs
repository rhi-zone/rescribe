//! Streaming MediaWiki writer -- converts a stream of events directly to
//! MediaWiki text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value from the event stream and never calls [`crate::emit::emit`]. It is
//! a second, independent emission path from the tree-based
//! `parse()`/`emit()` functions, not a thin wrapper around them.
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document. The frame stack (`O(nesting depth)`) holds only small
//! metadata -- a `usize` mark into `out`, a bool/enum, occasionally a short
//! owned `String` (a link's URL/accumulated text) -- never a copy of
//! accumulated child content. Children write **straight through** into
//! `out`. Constructs split into a few classes:
//!
//! - **Write-through** (prefix and suffix both known at `Start`/`End`,
//!   independent of content): `Blockquote`, `Heading` (marker run length
//!   comes from `level`, not content length -- unlike RST's underline),
//!   `List`/`ListItem`, `DefinitionTerm`, `Table`/`TableCell`, every inline
//!   span (`Bold`, `Italic`, ...), and the atomic leaf events (`CodeBlock`,
//!   `PreBlock`, `RawBlock`, `HorizontalRule`, `InlineImage`, ...).
//! - **Write-through + one piece of O(1) state**: a table row's `"|-\n"`
//!   separator is written only from the *second* row onward, tracked as a
//!   `bool` on the `Table` frame (mirrors `BuildContext`'s "is this the
//!   first row" check via `enumerate()`); a `DefinitionDesc`'s `": "` lead-in
//!   is only written if the description turns out non-empty, tracked as a
//!   `started: bool` flag set on its first inline contribution (mirrors
//!   RST's `Figure` caption lead-in -- the prefix's presence, not its
//!   *length*, depends on content).
//! - **Genuinely deferred**: `Link`. `build_inline`'s `Inline::Link{ url,
//!   text }` renders from a `text` field that is *not* the link's formatted
//!   inline content -- it is the concatenation of only the link's
//!   *top-level* `Inline::Text` children (`events_to_doc`'s
//!   `inlines.iter().map(|i| match i { Text(s) => s, _ => "" })`); any
//!   nested `Bold`/`Italic`/etc. inside a link contributes nothing, not even
//!   its own nested text. So a link's body is never written to `out` at
//!   all while open (`Writer::link_depth` gates every write, the same
//!   choke-point pattern as RST's `table_cell_depth`); only top-level
//!   `Text` events append to a small owned `String` on the `Link` frame,
//!   and the final `[url text]`/`[[url|text]]` is written once, at
//!   `EndLink`, from `url` + that accumulated text -- bounded by the link's
//!   own content, not the document.
//!
//! One more source of unavoidable non-streaming work: `Event::StartTable`
//! carries `caption: Option<Vec<Inline>>` as a pre-built AST fragment
//! directly in the event payload (not decomposed into a sub-stream of
//! inline events) -- a property of the event schema itself, not a choice
//! made here. Rendering it needs a small recursive `Inline -> String`
//! function ([`render_caption_inlines`]) mirroring `build_inline`; this is
//! narrowly scoped to that one embedded fragment (bounded by the caption's
//! own size) and is not "reconstructing the AST from streamed events" --
//! nothing is buffered or built from primitive `Start`/`End` events here.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use mediawiki_fmt::writer::Writer;
//! use mediawiki_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 2 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::Inline;
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming MediaWiki writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to recover the sink once all events have been
/// fed.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Every construct writes here
    /// directly (through [`Writer::push_out`]); frames record marks into
    /// it. Cleared (capacity retained) after each top-level block is
    /// flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level -- a block closing with an empty stack
    /// is flushed to the sink immediately.
    stack: Vec<Frame>,
    /// Mirrors `BuildContext::list_markers`: one marker char pushed per
    /// currently-open `List` (`'#'` ordered, `'*'` unordered), in nesting
    /// order. A list item's line prefix is the full accumulated string, not
    /// just the innermost marker repeated.
    list_markers: Vec<char>,
    /// Count of currently-open `Link` frames. `push_out` no-ops while this
    /// is nonzero -- see the module doc's "Genuinely deferred: Link"
    /// section. A `usize` (not a `bool`) so nested links (invalid per the
    /// grammar, but handled gracefully rather than panicking) still suppress
    /// correctly on the way back out.
    link_depth: usize,
    /// `emit()`'s final step is `ctx.output.trim_end().to_string() + "\n"`
    /// -- a transform over the *whole* document, applied once at the very
    /// end, not per top-level block. A per-block-flush streaming writer
    /// can't retroactively edit bytes already sent to the sink, so instead
    /// the trailing whitespace run of whatever was most recently flushed is
    /// held back here rather than sent immediately: if more non-whitespace
    /// content arrives, `flush` sends it first (it wasn't actually
    /// trailing after all); if nothing more arrives, `finish` discards it
    /// and writes exactly one `"\n"` in its place. Bounded by the gap
    /// between consecutive non-whitespace top-level blocks (normally a
    /// handful of bytes -- one block's `"\n\n"` separator); pathological
    /// documents consisting of very many consecutive whitespace-only
    /// top-level constructs are the one case this isn't strictly O(1) for.
    pending_ws: String,
}

/// Default capacity reserved for `Writer::out`. Skips the first several
/// geometric-growth doublings (pure overhead below any realistic block
/// size) without committing to a document-specific guess.
const DEFAULT_OUT_CAPACITY: usize = 4096;

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            out: String::with_capacity(DEFAULT_OUT_CAPACITY),
            stack: Vec::new(),
            list_markers: Vec::new(),
            link_depth: 0,
            pending_ws: String::new(),
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: OwnedEvent) {
        self.process(event);
    }

    /// Recover the underlying sink. `emit()`'s output always ends in
    /// exactly one `"\n"` (even for an empty document -- `"".trim_end() +
    /// "\n" == "\n"`), so this always writes one final `"\n"`, discarding
    /// whatever trailing whitespace `flush` had been holding back (it is,
    /// by construction, always pure whitespace or empty -- see
    /// `Writer::pending_ws`).
    pub fn finish(mut self) -> W {
        self.pending_ws.clear();
        let _ = self.sink.write_all(b"\n");
        self.sink
    }

    // -- Buffer primitives ---------------------------------------------

    /// Append to the shared output buffer. Suppressed while inside a
    /// `Link`'s body -- see the module doc.
    fn push_out(&mut self, s: &str) {
        if self.link_depth == 0 {
            self.out.push_str(s);
        }
    }

    /// Flush the completed top-level block to the sink and reset the
    /// buffer, keeping its capacity so the document only ever grows one
    /// buffer. Holds back the new trailing-whitespace run (prefixed with
    /// whatever was already held back, since that's now confirmed
    /// non-trailing) rather than sending it -- see `Writer::pending_ws`.
    fn flush(&mut self) {
        if self.out.is_empty() {
            return;
        }
        let mut combined = std::mem::take(&mut self.pending_ws);
        combined.push_str(&self.out);
        self.out.clear();
        let trim_len = combined.trim_end().len();
        let _ = self.sink.write_all(&combined.as_bytes()[..trim_len]);
        self.pending_ws.push_str(&combined[trim_len..]);
    }

    /// Whether the top-of-stack frame accepts block children
    /// (`Document`/`Blockquote`/`ListItem` in the original `push_block`).
    fn block_accepts(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote { .. }) | Some(Frame::ListItem)
        )
    }

    fn in_list_item(&self) -> bool {
        matches!(self.stack.last(), Some(Frame::ListItem))
    }

    /// Whether the top-of-stack frame is one of the original `push_inline`
    /// targets that actually renders formatted content into `out`. `Link`
    /// is deliberately excluded: its body never reaches `out` regardless
    /// (`push_out` is gated on `link_depth`), so nothing needs to check
    /// this while directly inside one.
    fn inline_accepts(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::Bold { .. }
                    | Frame::Italic { .. }
                    | Frame::Underline { .. }
                    | Frame::Strikethrough { .. }
                    | Frame::Superscript { .. }
                    | Frame::Subscript { .. }
                    | Frame::TableCell { .. }
                    | Frame::DefinitionTerm { .. }
                    | Frame::DefinitionDesc { .. }
            )
        )
    }

    /// Close a block-shaped construct written unconditionally at `mark..`:
    /// keep it (and flush if this just emptied the stack) if the enclosing
    /// frame accepts blocks, otherwise discard the whole thing.
    fn block_end(&mut self, mark: usize) {
        if self.block_accepts() {
            if self.stack.is_empty() {
                self.flush();
            }
        } else {
            self.out.truncate(mark);
        }
    }

    /// If a `DefinitionDesc` is open and hasn't written its `": "` lead-in
    /// yet, write it now (once) -- mirrors `build_block`'s
    /// `if !item.desc.is_empty() { ctx.write(": "); ... }`: the prefix's
    /// *presence* depends on whether any content ever arrives, so it's
    /// written lazily on first contribution rather than eagerly at
    /// `StartDefinitionDesc`.
    fn ensure_desc_prefix(&mut self) {
        let needs_prefix = matches!(
            self.stack.last(),
            Some(Frame::DefinitionDesc { started: false, .. })
        );
        if needs_prefix {
            if let Some(Frame::DefinitionDesc { started, .. }) = self.stack.last_mut() {
                *started = true;
            }
            self.push_out(": ");
        }
    }

    /// Write a leaf inline's contribution if the current context renders
    /// inline content (respecting the `DefinitionDesc` lazy lead-in and the
    /// `Link`-body suppression, both handled by `ensure_desc_prefix`/
    /// `push_out`).
    fn write_leaf(&mut self, s: &str) {
        if self.inline_accepts() {
            self.ensure_desc_prefix();
            self.push_out(s);
        }
    }

    /// Open an inline span: write the opening delimiter unconditionally
    /// (removed again at close if the context turns out invalid) and return
    /// the mark to truncate back to.
    fn open_span(&mut self, open: &str) -> usize {
        self.ensure_desc_prefix();
        let mark = self.out.len();
        self.push_out(open);
        mark
    }

    /// Close an inline span: write the closing delimiter, then discard the
    /// whole `mark..` region (open + content + close) if the enclosing
    /// frame does not accept inline children.
    fn close_span(&mut self, mark: usize, close: &str) {
        self.push_out(close);
        if !self.inline_accepts() {
            self.out.truncate(mark);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedEvent) {
        match event {
            OwnedEvent::StartDocument | OwnedEvent::EndDocument => {}

            // -- Block open/close --------------------------------------
            OwnedEvent::StartParagraph => {
                let mark = self.out.len();
                if self.in_list_item() {
                    let markers: String = self.list_markers.iter().collect();
                    self.push_out(&markers);
                    self.push_out(" ");
                }
                self.stack.push(Frame::Paragraph { mark });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    if self.in_list_item() {
                        self.push_out("\n");
                    } else if self.block_accepts() {
                        self.push_out("\n\n");
                        if self.stack.is_empty() {
                            self.flush();
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::StartHeading { level } => {
                let mark = self.out.len();
                self.push_out(&"=".repeat(level as usize));
                self.push_out(" ");
                self.stack.push(Frame::Heading { mark, level });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { mark, level }) = self.stack.pop() {
                    self.push_out(" ");
                    self.push_out(&"=".repeat(level as usize));
                    self.push_out("\n\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartBlockquote => {
                let mark = self.out.len();
                self.push_out("<blockquote>\n");
                self.stack.push(Frame::Blockquote { mark });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    // Mirrors build_block's "remove trailing blank line
                    // inside blockquote" -- a global (not scoped to this
                    // blockquote's own content) check against the tail of
                    // the whole buffer, exactly as the original does
                    // against ctx.output. Safe because the blockquote's
                    // content is always the most recently written text at
                    // this point (well-nested event streams).
                    if self.out.ends_with("\n\n") {
                        self.out.pop();
                    }
                    self.push_out("</blockquote>\n\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartList { ordered } => {
                let mark = self.out.len();
                self.list_markers.push(if ordered { '#' } else { '*' });
                self.stack.push(Frame::List { mark });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { mark }) = self.stack.pop() {
                    self.list_markers.pop();
                    if self.list_markers.is_empty() {
                        self.push_out("\n");
                    }
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartListItem => {
                self.stack.push(Frame::ListItem);
            }
            OwnedEvent::EndListItem => {
                // ListItem never writes anything of its own -- only its
                // children (each accepted/discarded on their own terms via
                // block_accepts()/in_list_item()) produce bytes.
                self.stack.pop();
            }
            OwnedEvent::StartDefinitionList => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionList { mark });
            }
            OwnedEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                let mark = self.out.len();
                self.push_out("; ");
                self.stack.push(Frame::DefinitionTerm { mark });
            }
            OwnedEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    if !matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionDesc {
                    mark,
                    started: false,
                });
            }
            OwnedEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { mark, started }) = self.stack.pop() {
                    if started {
                        self.push_out("\n");
                    }
                    if !matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::CodeBlock { language, content } => {
                let mark = self.out.len();
                if let Some(lang) = &language {
                    self.push_out("<syntaxhighlight lang=\"");
                    self.push_out(lang);
                    self.push_out("\">\n");
                    self.push_out(&content);
                    self.push_out("\n");
                    self.push_out("</syntaxhighlight>\n");
                    self.push_out("\n");
                } else {
                    for line in content.lines() {
                        self.push_out(" ");
                        self.push_out(line);
                        self.push_out("\n");
                    }
                    self.push_out("\n");
                }
                self.block_end(mark);
            }
            OwnedEvent::PreBlock { content } => {
                let mark = self.out.len();
                self.push_out("<pre>");
                self.push_out(&content);
                self.push_out("</pre>\n\n");
                self.block_end(mark);
            }
            OwnedEvent::RawBlock { content } => {
                let mark = self.out.len();
                self.push_out(&content);
                self.push_out("\n\n");
                self.block_end(mark);
            }
            OwnedEvent::HorizontalRule => {
                let mark = self.out.len();
                self.push_out("----\n\n");
                self.block_end(mark);
            }
            OwnedEvent::StartTable { caption } => {
                let mark = self.out.len();
                self.push_out("{|\n");
                if let Some(caption_inlines) = &caption {
                    self.push_out("|+ ");
                    let mut buf = String::new();
                    render_caption_inlines(caption_inlines, &mut buf);
                    self.push_out(&buf);
                    self.push_out("\n");
                }
                self.stack.push(Frame::Table {
                    mark,
                    seen_row: false,
                });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { mark, .. }) = self.stack.pop() {
                    self.push_out("|}\n\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartTableRow => {
                let mark = self.out.len();
                let seen = matches!(self.stack.last(), Some(Frame::Table { seen_row: true, .. }));
                if seen {
                    self.push_out("|-\n");
                }
                if let Some(Frame::Table { seen_row, .. }) = self.stack.last_mut() {
                    *seen_row = true;
                }
                self.stack.push(Frame::TableRow { mark });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { mark }) = self.stack.pop()
                    && !matches!(self.stack.last(), Some(Frame::Table { .. }))
                {
                    self.out.truncate(mark);
                }
            }
            OwnedEvent::StartTableCell { is_header } => {
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                    self.push_out(if is_header { "! " } else { "| " });
                }
                self.stack.push(Frame::TableCell { mark });
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                        self.push_out("\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }

            // -- Inline events ------------------------------------------
            OwnedEvent::Text(cow) => {
                if self.link_depth > 0 {
                    if let Some(Frame::Link { text, .. }) = self.stack.last_mut() {
                        text.push_str(&cow);
                    }
                    // Text nested deeper than a link's direct children
                    // (e.g. inside a Bold opened within the link)
                    // contributes nothing -- mirrors events_to_doc's
                    // `.map(|i| match i { Text(s) => s, _ => "" })`, which
                    // only ever looks at the link's *top-level* inlines.
                } else {
                    self.write_leaf(&cow);
                }
            }
            OwnedEvent::LineBreak => self.write_leaf("<br/>"),
            OwnedEvent::StartBold => {
                let mark = self.open_span("'''");
                self.stack.push(Frame::Bold { mark });
            }
            OwnedEvent::EndBold => {
                if let Some(Frame::Bold { mark }) = self.stack.pop() {
                    self.close_span(mark, "'''");
                }
            }
            OwnedEvent::StartItalic => {
                let mark = self.open_span("''");
                self.stack.push(Frame::Italic { mark });
            }
            OwnedEvent::EndItalic => {
                if let Some(Frame::Italic { mark }) = self.stack.pop() {
                    self.close_span(mark, "''");
                }
            }
            OwnedEvent::StartUnderline => {
                let mark = self.open_span("<u>");
                self.stack.push(Frame::Underline { mark });
            }
            OwnedEvent::EndUnderline => {
                if let Some(Frame::Underline { mark }) = self.stack.pop() {
                    self.close_span(mark, "</u>");
                }
            }
            OwnedEvent::StartStrikethrough => {
                let mark = self.open_span("<s>");
                self.stack.push(Frame::Strikethrough { mark });
            }
            OwnedEvent::EndStrikethrough => {
                if let Some(Frame::Strikethrough { mark }) = self.stack.pop() {
                    self.close_span(mark, "</s>");
                }
            }
            OwnedEvent::StartSuperscript => {
                let mark = self.open_span("<sup>");
                self.stack.push(Frame::Superscript { mark });
            }
            OwnedEvent::EndSuperscript => {
                if let Some(Frame::Superscript { mark }) = self.stack.pop() {
                    self.close_span(mark, "</sup>");
                }
            }
            OwnedEvent::StartSubscript => {
                let mark = self.open_span("<sub>");
                self.stack.push(Frame::Subscript { mark });
            }
            OwnedEvent::EndSubscript => {
                if let Some(Frame::Subscript { mark }) = self.stack.pop() {
                    self.close_span(mark, "</sub>");
                }
            }
            OwnedEvent::InlineCode(cow) => {
                let mut s = String::with_capacity(cow.len() + 13);
                s.push_str("<code>");
                s.push_str(&cow);
                s.push_str("</code>");
                self.write_leaf(&s);
            }
            OwnedEvent::StartLink { url } => {
                let mark = self.out.len();
                self.link_depth += 1;
                self.stack.push(Frame::Link {
                    mark,
                    url,
                    text: String::new(),
                });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { mark, url, text }) = self.stack.pop() {
                    self.link_depth -= 1;
                    let external = url.starts_with("http://") || url.starts_with("https://");
                    if external {
                        if text == url {
                            self.push_out(&format!("[{url}]"));
                        } else {
                            self.push_out(&format!("[{url} {text}]"));
                        }
                    } else if text == url {
                        self.push_out(&format!("[[{url}]]"));
                    } else {
                        self.push_out(&format!("[[{url}|{text}]]"));
                    }
                    if !self.inline_accepts() {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::InlineImage { url, alt } => {
                let s = if alt.is_empty() {
                    format!("[[File:{url}]]")
                } else {
                    format!("[[File:{url}|{alt}]]")
                };
                self.write_leaf(&s);
            }
            OwnedEvent::FootnoteRef { label, content } => {
                let s = render_footnote_ref(&label, content.as_deref());
                self.write_leaf(&s);
            }
            OwnedEvent::MathInline { source } => {
                let mut s = String::with_capacity(source.len() + 13);
                s.push_str("<math>");
                s.push_str(&source);
                s.push_str("</math>");
                self.write_leaf(&s);
            }
            OwnedEvent::Template { content } => {
                let mut s = String::with_capacity(content.len() + 4);
                s.push_str("{{");
                s.push_str(&content);
                s.push_str("}}");
                self.write_leaf(&s);
            }
            OwnedEvent::Nowiki { content } => {
                let mut s = String::with_capacity(content.len() + 17);
                s.push_str("<nowiki>");
                s.push_str(&content);
                s.push_str("</nowiki>");
                self.write_leaf(&s);
            }
        }
    }
}

/// Mirrors `build_inline`'s `FootnoteRef` arm exactly.
fn render_footnote_ref(label: &str, content: Option<&str>) -> String {
    if label.is_empty() {
        if let Some(c) = content {
            format!("<ref>{c}</ref>")
        } else {
            "<ref/>".to_string()
        }
    } else if let Some(c) = content {
        format!("<ref name=\"{label}\">{c}</ref>")
    } else {
        format!("<ref name=\"{label}\" />")
    }
}

/// Renders a pre-built `&[Inline]` fragment to text, mirroring
/// `build_inline`/`build_inlines` exactly. Used *only* for
/// `Event::StartTable`'s `caption` field, which the event schema carries as
/// an already-built AST fragment rather than a decomposed sub-stream of
/// inline events (see the module doc). Not a general AST-reconstruction
/// path: nothing here is built up from primitive `Start`/`End` events, and
/// its input is bounded by the caption's own size.
fn render_caption_inlines(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        render_caption_inline(inline, out);
    }
}

fn render_caption_inline(inline: &Inline, out: &mut String) {
    match inline {
        Inline::Text(s) => out.push_str(s),
        Inline::Bold(children) => {
            out.push_str("'''");
            render_caption_inlines(children, out);
            out.push_str("'''");
        }
        Inline::Italic(children) => {
            out.push_str("''");
            render_caption_inlines(children, out);
            out.push_str("''");
        }
        Inline::Code(s) => {
            out.push_str("<code>");
            out.push_str(s);
            out.push_str("</code>");
        }
        Inline::Link { url, text } => {
            let external = url.starts_with("http://") || url.starts_with("https://");
            if external {
                if text == url {
                    out.push_str(&format!("[{url}]"));
                } else {
                    out.push_str(&format!("[{url} {text}]"));
                }
            } else if text == url {
                out.push_str(&format!("[[{url}]]"));
            } else {
                out.push_str(&format!("[[{url}|{text}]]"));
            }
        }
        Inline::Image { url, alt } => {
            if alt.is_empty() {
                out.push_str(&format!("[[File:{url}]]"));
            } else {
                out.push_str(&format!("[[File:{url}|{alt}]]"));
            }
        }
        Inline::LineBreak => out.push_str("<br/>"),
        Inline::Strikeout(children) => {
            out.push_str("<s>");
            render_caption_inlines(children, out);
            out.push_str("</s>");
        }
        Inline::Underline(children) => {
            out.push_str("<u>");
            render_caption_inlines(children, out);
            out.push_str("</u>");
        }
        Inline::Subscript(children) => {
            out.push_str("<sub>");
            render_caption_inlines(children, out);
            out.push_str("</sub>");
        }
        Inline::Superscript(children) => {
            out.push_str("<sup>");
            render_caption_inlines(children, out);
            out.push_str("</sup>");
        }
        Inline::FootnoteRef { label, content } => {
            out.push_str(&render_footnote_ref(label, content.as_deref()));
        }
        Inline::MathInline { source } => {
            out.push_str("<math>");
            out.push_str(source);
            out.push_str("</math>");
        }
        Inline::Template { content } => {
            out.push_str("{{");
            out.push_str(content);
            out.push_str("}}");
        }
        Inline::Nowiki { content } => {
            out.push_str("<nowiki>");
            out.push_str(content);
            out.push_str("</nowiki>");
        }
    }
}

/// Frames carry only a mark into the shared buffer and tiny scalars -- never
/// accumulated content, except `Link`'s small bounded `text` accumulator
/// (see the module doc).
enum Frame {
    Paragraph {
        mark: usize,
    },
    Heading {
        mark: usize,
        level: u8,
    },
    Blockquote {
        mark: usize,
    },
    List {
        mark: usize,
    },
    /// No fields: `ListItem` never writes anything of its own (see
    /// `EndListItem`'s handler).
    ListItem,
    DefinitionList {
        mark: usize,
    },
    DefinitionTerm {
        mark: usize,
    },
    /// `started` mirrors `!item.desc.is_empty()`, decided lazily on first
    /// contribution -- see `Writer::ensure_desc_prefix`.
    DefinitionDesc {
        mark: usize,
        started: bool,
    },
    /// `seen_row` mirrors `enumerate()`'s `i > 0` check for the `"|-\n"`
    /// row separator -- `O(1)`, not a buffered copy of prior rows.
    Table {
        mark: usize,
        seen_row: bool,
    },
    TableRow {
        mark: usize,
    },
    TableCell {
        mark: usize,
    },
    Bold {
        mark: usize,
    },
    Italic {
        mark: usize,
    },
    Underline {
        mark: usize,
    },
    Strikethrough {
        mark: usize,
    },
    Superscript {
        mark: usize,
    },
    Subscript {
        mark: usize,
    },
    /// The one inline span with genuinely deferred rendering -- see the
    /// module doc's "Genuinely deferred: Link" section. `text` accumulates
    /// only the link's *top-level* `Text` children, bounded by the link's
    /// own content.
    Link {
        mark: usize,
        url: String,
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 2 });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned(
            "Hello".to_string(),
        )));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("== Hello =="), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned(
            "World".to_string(),
        )));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "== Hello ==\n\nA paragraph with '''bold''' text.\n\n* item one\n* item two\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted_text);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    /// The streaming `Writer` must produce *byte-identical* output to the
    /// tree-based `emit()` for the same document -- the guard that keeps
    /// the two independent emission paths honest, including the deferred
    /// `Link` construct and the lazy `DefinitionDesc`/table-row-separator
    /// state.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "== Title ==\n\nIntro paragraph with '''bold''' and ''italic'' and <code>code</code>.\n",
            "=== Sub ===\n\ntext with <u>underline</u>, <s>strike</s>, <sup>sup</sup>, <sub>sub</sub>.\n",
            "* bullet one\n* bullet two\n\n** nested a\n** nested b\n",
            "# ordered one\n# ordered two\n",
            "<syntaxhighlight lang=\"rust\">\nlet x = 1;\nlet y = 2;\n</syntaxhighlight>\n",
            " literal line one\n literal line two\n",
            "<blockquote>\nA quoted paragraph.\n\nSecond para of quote.\n</blockquote>\n",
            "; term\n: definition body\n\n; term2\n: another definition\n",
            "; term with no desc\n\n; term2\n: has desc\n",
            "{|\n! A\n! B\n|-\n| Cell 1\n| Cell 2\n|}\n",
            "{|\n|+ My caption\n! A\n! B\n|-\n| Cell 1\n| Cell 2\n|}\n",
            "----\n\nAfter the transition.\n",
            "A paragraph with an external [http://example.com/ link] and internal [[Page|text]].\n",
            "A paragraph with [http://example.com/] and [[Page]] (text == url).\n",
            "A paragraph mentioning [[File:img.png|alt text]] and a <ref name=\"x\">note</ref>.\n",
            "* item\n** nested item\n* item two\n\n# nested ordered\n# more\n",
            "A para\n\n* item\n\ncontinued paragraph\n",
            "A link with '''bold''' inside: [http://example.com/ some '''bold''' text].\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input);
            let built = crate::emit::emit(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events::events(input) {
                w.write_event(e);
            }
            let streamed = String::from_utf8(w.finish()).unwrap();

            assert_eq!(
                built, streamed,
                "streaming Writer diverged from emit() for input:\n{input}\n\
                 emit():\n{built:?}\nWriter:\n{streamed:?}"
            );
        }
    }

    /// Round-trip a broader construct mix entirely through
    /// `events() -> Writer`, proving the incremental per-top-level-block
    /// flush handles every construct `parse()` produces.
    #[test]
    fn test_writer_roundtrip_full_construct_mix() {
        let input = "\
== Title ==

Intro paragraph with '''bold''' and <code>code</code>.

<blockquote>
A block quote.
</blockquote>

* bullet one
* bullet two

# ordered one
# ordered two

<syntaxhighlight lang=\"rust\">
let x = 1;
</syntaxhighlight>

{|
! A
! B
|-
| Cell 1
| Cell 2
|}

----

After the transition.
";
        let (doc, _) = crate::parse::parse(input);
        assert!(
            doc.blocks.len() >= 7,
            "expected a rich construct mix, got {:?}",
            doc.blocks
        );

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events(input) {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();

        let (doc2, _) = crate::parse::parse(&emitted_text);
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "writer roundtrip block count mismatch\ninput blocks: {:#?}\nemitted text: \
             {emitted_text}\nreparsed blocks: {:#?}",
            doc.blocks,
            doc2.blocks,
        );
    }

    /// Nested lists (bullet-in-bullet, ordered-in-bullet) exercise
    /// `list_markers` bookkeeping, which -- unlike a plain depth counter --
    /// must accumulate the *actual* marker characters across levels.
    #[test]
    fn test_writer_roundtrip_nested_lists() {
        let input = "\
* outer one
* outer two
** inner a
** inner b
* outer three
";
        let (doc, _) = crate::parse::parse(input);
        let built = crate::emit::emit(&doc);

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events(input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).unwrap();
        assert_eq!(
            built, streamed,
            "streaming Writer diverged from emit() for nested-list input"
        );

        let (doc2, _) = crate::parse::parse(&streamed);
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "nested list roundtrip block count mismatch\nemitted:\n{streamed}"
        );
    }

    // A single process-wide `#[global_allocator]` tracks both allocation
    // count (for the no-subtree-reconstruction-blowup guard) and
    // current/peak bytes (for the peak-memory guard) -- Rust allows only
    // one `#[global_allocator]` per binary, so both tests below share this
    // one rather than each defining their own.
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TrackingAlloc;
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    static CURRENT: AtomicUsize = AtomicUsize::new(0);
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    unsafe impl GlobalAlloc for TrackingAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            let prev = CURRENT.fetch_add(layout.size(), Ordering::SeqCst);
            PEAK.fetch_max(prev + layout.size(), Ordering::SeqCst);
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
            unsafe { System.dealloc(ptr, layout) }
        }
    }
    #[global_allocator]
    static GLOBAL: TrackingAlloc = TrackingAlloc;
    /// `cargo test` runs this crate's other tests concurrently with these
    /// two allocator-instrumented ones by default, and they all share the
    /// one process-wide `TrackingAlloc`. Serializing just these two against
    /// *each other* removes the dominant source of cross-test interference
    /// without needing `--test-threads=1` for the whole binary.
    static ALLOC_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Regression guard against reintroducing per-block `Block`/`Inline`
    /// subtree reconstruction. A large, deeply-nested event stream must
    /// complete with an allocation count that stays close to linear in
    /// event count, not blow up the way tree materialization would.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        let _guard = ALLOC_TEST_GUARD.lock().unwrap();

        fn events_for(n: usize) -> Vec<OwnedEvent> {
            use std::borrow::Cow;
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(OwnedEvent::StartHeading { level: 2 });
                evs.push(OwnedEvent::Text(Cow::Owned(format!("Section {i}"))));
                evs.push(OwnedEvent::EndHeading);
                evs.push(OwnedEvent::StartParagraph);
                evs.push(OwnedEvent::Text(Cow::Owned("plain ".to_string())));
                evs.push(OwnedEvent::StartBold);
                evs.push(OwnedEvent::Text(Cow::Owned("bold".to_string())));
                evs.push(OwnedEvent::EndBold);
                evs.push(OwnedEvent::EndParagraph);
                evs.push(OwnedEvent::StartList { ordered: false });
                for j in 0..2 {
                    evs.push(OwnedEvent::StartListItem);
                    evs.push(OwnedEvent::StartParagraph);
                    evs.push(OwnedEvent::Text(Cow::Owned(format!("item {j}"))));
                    evs.push(OwnedEvent::EndParagraph);
                    evs.push(OwnedEvent::EndListItem);
                }
                evs.push(OwnedEvent::EndList);
            }
            evs
        }

        fn run(n: usize) -> usize {
            let before = ALLOCS.load(Ordering::Relaxed);
            let evs = events_for(n);
            let after_build = ALLOCS.load(Ordering::Relaxed);
            let mut out = Vec::new();
            {
                let mut w = Writer::new(&mut out);
                for e in evs {
                    w.write_event(e);
                }
                w.finish();
            }
            let after = ALLOCS.load(Ordering::Relaxed);
            std::hint::black_box(&out);
            after - after_build.max(before)
        }

        let small = run(200).max(1);
        let large = run(2000); // 10x the sections

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 20.0,
            "allocation count did not scale near-linearly: {small} allocs @200 sections -> \
             {large} allocs @2000 sections (ratio {ratio:.2}); this suggests reintroduced \
             per-block subtree reconstruction"
        );
    }

    /// Peak memory must stay near-flat as document size grows, not
    /// `O(full document)`. Uses `std::io::sink()` (discards bytes
    /// immediately) rather than a `Vec<u8>` sink -- with a growing `Vec<u8>`
    /// sink, the *sink itself* would retain the full output regardless of
    /// how incremental `Writer`'s own internal state is, defeating the
    /// point of the measurement. Compares peak growth across a 100x
    /// increase in document size (the same relative-comparison shape as the
    /// allocation-count guard above, and for the same reason: an absolute
    /// byte threshold is noisy under `cargo test`'s parallel execution).
    #[test]
    fn test_writer_peak_memory_bounded() {
        let _guard = ALLOC_TEST_GUARD.lock().unwrap();

        fn run(n: usize) -> usize {
            let baseline = CURRENT.load(Ordering::SeqCst);
            PEAK.store(baseline, Ordering::SeqCst);
            {
                let mut w = Writer::new(std::io::sink());
                for i in 0..n {
                    w.write_event(OwnedEvent::StartParagraph);
                    w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned(format!(
                        "Paragraph number {i} with representative body text to pad it out."
                    ))));
                    w.write_event(OwnedEvent::EndParagraph);
                }
                w.finish();
            }
            PEAK.load(Ordering::SeqCst).saturating_sub(baseline).max(1)
        }

        let small = run(2_000);
        let large = run(200_000); // 100x the paragraphs

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 10.0,
            "peak memory growth did not stay bounded: {small} bytes @2_000 paragraphs -> \
             {large} bytes @200_000 paragraphs (ratio {ratio:.2}); this suggests the writer is \
             buffering the whole document instead of flushing per top-level block"
        );
    }
}
