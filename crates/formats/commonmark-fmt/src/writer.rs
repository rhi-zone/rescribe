//! Streaming CommonMark writer — converts a stream of events directly to
//! CommonMark bytes.
//!
//! # Memory model
//!
//! [`Writer`] never buffers the fed events and never reconstructs a
//! [`crate::ast::CmDoc`]/`Block`/`Inline` value. It is a second, independent
//! emission path from the tree-based [`crate::emit::emit`], not a thin
//! wrapper around it (though it reuses `emit`'s pure helper functions —
//! [`crate::emit::escape_text`], `escape_url`, `escape_title`,
//! `list_item_marker` — to guarantee identical formatting decisions rather
//! than a second, potentially-drifting copy of that logic).
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document. Frames on the `Vec<Frame>` stack (`O(nesting depth)`)
//! hold only a mark into `out` and small scalars — never a copy of
//! accumulated content — with two exceptions noted below. Children write
//! **straight through** into `out`; a frame that needs to decorate its own
//! content afterwards post-processes the `out[mark..]` range in place.
//!
//! Investigating [`crate::emit`] (the ground truth) shows CommonMark's own
//! block grammar needs far less lookahead than other formats in this
//! workspace:
//!
//! - **Write-through** (no per-frame buffering): `Paragraph`, `Heading`
//!   (ATX-style `#` prefixes need no content-length computation, unlike an
//!   RST-style underline), `CodeBlock`/`HtmlBlock`/`ThematicBreak` (atomic
//!   leaf events, already fully materialized by the reader), every inline
//!   span (`Emphasis`, `Strong`, `Strikethrough`, `Link`, `Code`,
//!   `HtmlInline`), and — a genuine surprise relative to `rst-fmt`'s
//!   width-computing table renderer — **`Table`**: `crate::emit::emit_table`
//!   does no column-width alignment pass at all (GFM's pipe-table grammar
//!   does not require padded columns), so cells and the alignment row (whose
//!   marker string is fully known at `StartTable`) write straight through.
//! - **Write-through + one-shot** (all needed data arrives together in a
//!   single event, so nothing is deferred): `Image` — `emit_inline` never
//!   iterates any children for images, only the `alt` string carried on
//!   `StartImage` itself, so the entire `![alt](url "title")` is written at
//!   `StartImage` and any interleaved `Text`/`EndImage` events are no-ops.
//! - **Deferred per-line transform** (every line of already-written content
//!   must be re-indented, because CommonMark's block quote and list-item
//!   continuation-line indentation apply to arbitrarily-nested content, not
//!   just the construct's own header): `Blockquote` (uniform `"> "` prefix)
//!   and `ListItem` (marker on the first line, space-width alignment on
//!   continuation lines, blank lines left bare per `crate::emit::emit_list`).
//!   These post-process `out[mark..]` through a *pooled, reused* scratch
//!   buffer (`Writer::scratch`), so the pool holds at most `O(nesting
//!   depth)` buffers for the whole document rather than one fresh
//!   allocation per construct. `ListItem` additionally carries a computed
//!   indent `String` on its frame (bounded by currently-open item nesting,
//!   not document size).
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties. Memory is `O(largest
//! top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Tight lists and the events() Text/StartImage ordering bug
//!
//! CommonMark's pulldown-cmark backend omits `Paragraph` events for items of
//! a *tight* list — bare inline events land directly under `StartItem`. This
//! `Writer` detects that by lazily opening an implicit `Frame::Paragraph`
//! the moment an inline event arrives with a `ListItem` on top of the stack,
//! and closing it (a) before any sibling block event, or (b) at `EndItem` if
//! still open — both by simply re-dispatching a synthetic `EndParagraph`.
//! Because this writes inline content **in true event-arrival order** rather
//! than accumulating it in a side buffer keyed by frame kind (the previous
//! buffer-then-emit implementation's approach), it does not exhibit the
//! latent reordering bug the old code had: text followed by a nested list in
//! the same tight item used to come out with the nested list emitted
//! *first* (see `test_writer_tight_item_text_before_nested_list_order`
//! below).
//!
//! Separately — and *not* fixed here, since it lives in `events.rs`, not
//! this module — `EventIter` has a real ordering bug where an image's alt
//! `Text` event is delivered *before* `StartImage` rather than between
//! `StartImage`/`EndImage` (see the `commonmark`/`events` `KnownFailure` in
//! `rescribe-fixtures`). This `Writer`, like the old one, has no way to
//! distinguish that leaked `Text` from genuine prose preceding an image, so
//! it faithfully writes it as paragraph text — reproducing
//! `"Alt text![Alt text](photo.jpg)"` for such input, exactly as documented.
//!
//! # Example
//! ```
//! use commonmark_fmt::writer::Writer;
//! use commonmark_fmt::events::OwnedEvent;
//! use std::borrow::Cow;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
//! w.write_event(OwnedEvent::EndHeading { level: 1 });
//! let bytes = w.finish().unwrap();
//! assert!(String::from_utf8(bytes).unwrap().starts_with("# Hello"));
//! ```

use crate::emit::{escape_text, escape_title, escape_url, list_item_marker};
use crate::events::Event;
#[cfg(feature = "tables")]
use crate::options::ColumnAlignment;
#[cfg(feature = "frontmatter")]
use crate::options::FrontMatterKind;
use std::io::Write;

/// Default capacity reserved for `Writer::out`. Mirrors `rst_fmt::Writer`'s
/// rationale: skip the first several doubling reallocations for realistic
/// block sizes without committing to a document-specific guess.
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Streaming CommonMark writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to flush any remainder and recover the sink.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Every construct writes here
    /// directly; frames record marks into it. Cleared (capacity retained)
    /// after each top-level block is flushed.
    out: String,
    /// Pool of scratch buffers for the deferred per-line re-indent
    /// constructs (`Blockquote`, `ListItem`). Buffers are returned after
    /// use, so at most `O(nesting depth)` are ever allocated for a whole
    /// document instead of one per construct.
    scratch: Vec<String>,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level — a block closing with an empty stack
    /// is flushed to the sink immediately.
    stack: Vec<Frame>,
    /// Number of top-level document blocks seen so far, for the
    /// leading-blank-line-before-the-Nth-block rule (`crate::emit`'s
    /// `emit_blocks`: `i > 0 && !tight` — top level is never tight).
    doc_child_count: usize,
    /// Whether a front-matter block has already been written. Mirrors the
    /// old `DocBuilder`'s `get_or_insert`: only the first `FrontMatter`
    /// event is honored, and only if it arrives before any document block.
    #[cfg(feature = "frontmatter")]
    frontmatter_written: bool,
}

impl<W: Write> Writer<W> {
    /// Create a new writer that emits to `sink`.
    pub fn new(sink: W) -> Self {
        Self::with_capacity(sink, DEFAULT_OUT_CAPACITY)
    }

    /// Like [`Writer::new`], but reserves `out_capacity` bytes for the
    /// shared output buffer up front.
    pub fn with_capacity(sink: W, out_capacity: usize) -> Self {
        Writer {
            sink,
            out: String::with_capacity(out_capacity),
            scratch: Vec::new(),
            stack: Vec::new(),
            doc_child_count: 0,
            #[cfg(feature = "frontmatter")]
            frontmatter_written: false,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: Event<'_>) {
        self.process(event);
    }

    /// Flush any remaining buffered bytes to the sink and recover it. Every
    /// completed top-level block was already flushed by `write_event`
    /// (using a fallible-swallowing flush, since `write_event` has no
    /// `Result` to propagate through); this final flush is the one place an
    /// I/O error can still surface.
    pub fn finish(mut self) -> std::io::Result<W> {
        if !self.out.is_empty() {
            self.sink.write_all(self.out.as_bytes())?;
            self.out.clear();
        }
        Ok(self.sink)
    }

    // ── Buffer primitives ────────────────────────────────────────────────

    fn push_out(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// Flush the completed top-level block to the sink and reset the
    /// buffer, keeping its capacity. Errors are swallowed here (no `Result`
    /// on `write_event` to propagate them through) — mirrors
    /// `rst_fmt::Writer::flush`; any error is still observable via
    /// `finish`'s own final, propagating flush if it persists.
    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    // ── Context predicates ───────────────────────────────────────────────

    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote { .. } | Frame::ListItem { .. })
        )
    }

    fn accepts_inline(&self) -> bool {
        match self.stack.last() {
            Some(
                Frame::Paragraph { .. }
                | Frame::Heading { .. }
                | Frame::Emphasis { .. }
                | Frame::Strong { .. }
                | Frame::Link { .. },
            ) => true,
            #[cfg(feature = "strikethrough")]
            Some(Frame::Strikethrough { .. }) => true,
            #[cfg(feature = "tables")]
            Some(Frame::TableCell) => true,
            _ => false,
        }
    }

    // ── Container child-open bookkeeping ─────────────────────────────────

    /// Mirrors `crate::emit::emit_blocks`'s `i > 0 && !tight` rule and
    /// `emit_list`'s `!tight && idx > 0` rule (the same shape: a blank-line
    /// separator before every child but the first, unless the container is
    /// tight). `List` uses this for its item separator (its children are
    /// items, not blocks, but the rule and bookkeeping are identical).
    /// Returns whether a blank-line separator should be written now.
    fn container_child_open(&mut self) -> bool {
        match self.stack.last_mut() {
            Some(Frame::Blockquote { child_count, .. }) => {
                let first = *child_count == 0;
                *child_count += 1;
                !first
            }
            Some(Frame::ListItem {
                child_count, tight, ..
            }) => {
                let first = *child_count == 0;
                *child_count += 1;
                !first && !*tight
            }
            _ => {
                let first = self.doc_child_count == 0;
                self.doc_child_count += 1;
                !first
            }
        }
    }

    /// A real `Paragraph` is always closed by its own `EndParagraph` before
    /// any sibling event can arrive (events are well-nested), so a
    /// `Paragraph` frame still open when a new block is about to start can
    /// only be the implicit one synthesized for a tight list item's bare
    /// inline run (see the module doc). Close it the same way an explicit
    /// `EndParagraph` would.
    fn close_dangling_implicit_paragraph(&mut self) {
        if self.stack.len() < 2 {
            return;
        }
        let top_is_paragraph = matches!(self.stack[self.stack.len() - 1], Frame::Paragraph { .. });
        let parent_is_item = matches!(self.stack[self.stack.len() - 2], Frame::ListItem { .. });
        if top_is_paragraph && parent_is_item {
            self.process(Event::EndParagraph);
        }
    }

    /// If an inline event arrives with a bare `ListItem` on top of the stack
    /// (a tight list item's un-wrapped inline run), lazily open an implicit
    /// paragraph so the normal `accepts_inline`-gated write-through path
    /// handles it uniformly.
    fn ensure_inline_context(&mut self) {
        if matches!(self.stack.last(), Some(Frame::ListItem { .. })) {
            self.process(Event::StartParagraph);
        }
    }

    /// Open a block: close any dangling implicit paragraph, write the
    /// container's blank-line separator if this isn't the container's first
    /// child, and return the mark where this block's own content begins.
    fn block_open(&mut self) -> usize {
        self.close_dangling_implicit_paragraph();
        if self.container_child_open() {
            self.push_out("\n");
        }
        self.out.len()
    }

    /// Close a block: discard it if the enclosing frame does not take block
    /// children, otherwise flush if it completed a top-level block.
    fn block_close(&mut self, mark: usize) {
        if !self.accepts_blocks() {
            self.out.truncate(mark);
            return;
        }
        if self.stack.is_empty() {
            self.flush();
        }
    }

    // ── Deferred per-line reindent ───────────────────────────────────────

    /// Re-indent `out[mark..]` with a uniform `"> "` prefix per line,
    /// matching `crate::emit`'s `Block::Blockquote` arm exactly (including
    /// its empty-content special case). Uses a pooled scratch buffer.
    fn reindent_blockquote(&mut self, mark: usize) {
        let mut buf = self.scratch.pop().unwrap_or_default();
        buf.clear();
        if self.out[mark..].is_empty() {
            buf.push_str(">\n");
        } else {
            for line in self.out[mark..].lines() {
                buf.push_str("> ");
                buf.push_str(line);
                buf.push('\n');
            }
        }
        self.out.truncate(mark);
        self.out.push_str(&buf);
        self.scratch.push(buf);
    }

    /// Re-indent `out[mark..]` for a list item: the first physical line
    /// (which already contains the marker, written at `StartItem`) is left
    /// untouched; every later non-blank line gets `indent` prepended; blank
    /// lines are left bare (no trailing whitespace), matching
    /// `crate::emit::emit_list`'s loop exactly.
    fn reindent_list_item(&mut self, mark: usize, indent: &str) {
        let mut buf = self.scratch.pop().unwrap_or_default();
        buf.clear();
        let mut first = true;
        for line in self.out[mark..].lines() {
            if first {
                buf.push_str(line);
                buf.push('\n');
                first = false;
            } else if line.is_empty() {
                buf.push('\n');
            } else {
                buf.push_str(indent);
                buf.push_str(line);
                buf.push('\n');
            }
        }
        self.out.truncate(mark);
        self.out.push_str(&buf);
        self.scratch.push(buf);
    }

    // ── Inline span helpers ──────────────────────────────────────────────

    /// Open an inline span whose opening delimiter is already known: write
    /// it straight through (only if the current context accepts inline
    /// content) and return the mark needed to discard the whole span if it
    /// turns out to have no valid enclosing context.
    fn open_span(&mut self, open: &str) -> usize {
        self.ensure_inline_context();
        let mark = self.out.len();
        if self.accepts_inline() {
            self.push_out(open);
        }
        mark
    }

    /// Close an inline span: write `close`, then discard the whole
    /// `mark..` range (open delimiter, children, close delimiter) if the
    /// (now-current, post-pop) context doesn't actually accept inline
    /// content — the same cascading discard `rst_fmt::Writer` uses.
    fn close_span(&mut self, mark: usize, close: &str) {
        self.push_out(close);
        if !self.accepts_inline() {
            self.out.truncate(mark);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event<'_>) {
        match event {
            Event::StartDocument | Event::EndDocument => {}

            // ── Block open/close ────────────────────────────────────────
            Event::StartParagraph => {
                let mark = self.block_open();
                self.stack.push(Frame::Paragraph { mark });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_close(mark);
                }
            }
            Event::StartHeading { level } => {
                let mark = self.block_open();
                for _ in 0..level {
                    self.push_out("#");
                }
                self.push_out(" ");
                self.stack.push(Frame::Heading { mark });
            }
            Event::EndHeading { .. } => {
                if let Some(Frame::Heading { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_close(mark);
                }
            }
            Event::StartBlockquote => {
                let mark = self.block_open();
                self.stack.push(Frame::Blockquote {
                    mark,
                    child_count: 0,
                });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark, .. }) = self.stack.pop() {
                    self.reindent_blockquote(mark);
                    self.block_close(mark);
                }
            }
            Event::StartList {
                ordered,
                start,
                tight,
            } => {
                let mark = self.block_open();
                self.stack.push(Frame::List {
                    ordered,
                    start,
                    tight,
                    item_idx: 0,
                    mark,
                });
            }
            Event::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.block_close(mark);
                }
            }
            Event::StartItem {
                #[cfg(feature = "task-lists")]
                checked,
            } => {
                #[allow(unused_mut)]
                let (marker, mut indent_width, tight, sep) = match self.stack.last_mut() {
                    Some(Frame::List {
                        ordered,
                        start,
                        tight,
                        item_idx,
                        ..
                    }) => {
                        let idx = *item_idx;
                        let sep = idx > 0 && !*tight;
                        *item_idx += 1;
                        let kind = if *ordered {
                            crate::ast::ListKind::Ordered {
                                start: *start,
                                marker: crate::ast::OrderedMarker::Period,
                            }
                        } else {
                            crate::ast::ListKind::Unordered { marker: '-' }
                        };
                        let (marker, indent) = list_item_marker(&kind, idx);
                        (marker, indent, *tight, sep)
                    }
                    _ => (String::new(), 0, true, false),
                };
                if sep {
                    self.push_out("\n");
                }
                #[cfg(feature = "task-lists")]
                let mut marker = marker;
                #[cfg(feature = "task-lists")]
                if let Some(checked) = checked {
                    let checkbox = if checked { "[x] " } else { "[ ] " };
                    marker.push_str(checkbox);
                    indent_width += checkbox.len();
                }
                let mark = self.out.len();
                self.push_out(&marker);
                self.stack.push(Frame::ListItem {
                    mark,
                    indent: " ".repeat(indent_width),
                    tight,
                    child_count: 0,
                });
            }
            Event::EndItem => {
                self.close_dangling_implicit_paragraph();
                if let Some(Frame::ListItem { mark, indent, .. }) = self.stack.pop() {
                    self.reindent_list_item(mark, &indent);
                    if !matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }

            // ── Leaf block events ───────────────────────────────────────
            Event::CodeBlock { language, content } => {
                let mark = self.block_open();
                let fence = if content.contains("```") {
                    "~~~"
                } else {
                    "```"
                };
                self.push_out(fence);
                if let Some(lang) = &language {
                    self.push_out(lang);
                }
                self.push_out("\n");
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out(fence);
                self.push_out("\n");
                self.block_close(mark);
            }
            Event::HtmlBlock(content) => {
                let mark = self.block_open();
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.block_close(mark);
            }
            Event::ThematicBreak => {
                let mark = self.block_open();
                self.push_out("---\n");
                self.block_close(mark);
            }
            #[cfg(feature = "frontmatter")]
            Event::FrontMatter { kind, content } => {
                if !self.frontmatter_written && self.stack.is_empty() && self.doc_child_count == 0 {
                    self.frontmatter_written = true;
                    let delim = match kind {
                        FrontMatterKind::Yaml => "---",
                        FrontMatterKind::Toml => "+++",
                    };
                    self.push_out(delim);
                    self.push_out("\n");
                    self.push_out(&content);
                    if !content.ends_with('\n') {
                        self.push_out("\n");
                    }
                    self.push_out(delim);
                    self.push_out("\n\n");
                    self.flush();
                }
            }
            // Reference-style link definitions are dropped, matching
            // `crate::emit::emit` (which never emits `CmDoc::link_defs` — a
            // pre-existing, separate lossiness gap in the AST builder path,
            // not something introduced or fixed here; see TODO.md).
            Event::LinkDef { .. } => {}

            #[cfg(feature = "tables")]
            Event::StartTable { alignments } => {
                let mark = self.block_open();
                self.stack.push(Frame::Table { mark, alignments });
            }
            #[cfg(feature = "tables")]
            Event::EndTable => {
                if let Some(Frame::Table { mark, .. }) = self.stack.pop() {
                    self.block_close(mark);
                }
            }
            #[cfg(feature = "tables")]
            Event::StartTableHead | Event::StartTableRow => {
                self.push_out("|");
                self.stack.push(Frame::TableRow);
            }
            #[cfg(feature = "tables")]
            Event::EndTableHead => {
                if matches!(self.stack.pop(), Some(Frame::TableRow)) {
                    self.push_out("\n");
                    let cells: Option<Vec<&'static str>> =
                        if let Some(Frame::Table { alignments, .. }) = self.stack.last() {
                            Some(
                                alignments
                                    .iter()
                                    .map(|a| match a {
                                        ColumnAlignment::None => "---",
                                        ColumnAlignment::Left => ":--",
                                        ColumnAlignment::Center => ":-:",
                                        ColumnAlignment::Right => "--:",
                                    })
                                    .collect(),
                            )
                        } else {
                            None
                        };
                    if let Some(cells) = cells {
                        self.push_out("|");
                        for cell in cells {
                            self.push_out(" ");
                            self.push_out(cell);
                            self.push_out(" |");
                        }
                        self.push_out("\n");
                    }
                }
            }
            #[cfg(feature = "tables")]
            Event::EndTableRow => {
                if matches!(self.stack.pop(), Some(Frame::TableRow)) {
                    self.push_out("\n");
                }
            }
            #[cfg(feature = "tables")]
            Event::StartTableCell => {
                self.push_out(" ");
                self.stack.push(Frame::TableCell);
            }
            #[cfg(feature = "tables")]
            Event::EndTableCell => {
                if matches!(self.stack.pop(), Some(Frame::TableCell)) {
                    self.push_out(" |");
                }
            }

            // ── Inline opens/closes ─────────────────────────────────────
            Event::StartEmphasis => {
                let mark = self.open_span("*");
                self.stack.push(Frame::Emphasis { mark });
            }
            Event::EndEmphasis => {
                if let Some(Frame::Emphasis { mark }) = self.stack.pop() {
                    self.close_span(mark, "*");
                }
            }
            Event::StartStrong => {
                let mark = self.open_span("**");
                self.stack.push(Frame::Strong { mark });
            }
            Event::EndStrong => {
                if let Some(Frame::Strong { mark }) = self.stack.pop() {
                    self.close_span(mark, "**");
                }
            }
            #[cfg(feature = "strikethrough")]
            Event::StartStrikethrough => {
                let mark = self.open_span("~~");
                self.stack.push(Frame::Strikethrough { mark });
            }
            #[cfg(feature = "strikethrough")]
            Event::EndStrikethrough => {
                if let Some(Frame::Strikethrough { mark }) = self.stack.pop() {
                    self.close_span(mark, "~~");
                }
            }
            Event::StartLink { url, title } => {
                let mark = self.open_span("[");
                self.stack.push(Frame::Link {
                    mark,
                    url: url.into_owned(),
                    title: title.map(|t| t.into_owned()),
                });
            }
            Event::EndLink => {
                if let Some(Frame::Link { mark, url, title }) = self.stack.pop() {
                    self.push_out("](");
                    self.push_out(&escape_url(&url));
                    if let Some(t) = &title {
                        self.push_out(" \"");
                        self.push_out(&escape_title(t));
                        self.push_out("\"");
                    }
                    self.close_span(mark, ")");
                }
            }
            Event::StartImage { url, title, alt } => {
                self.ensure_inline_context();
                if self.accepts_inline() {
                    self.push_out("![");
                    self.push_out(&escape_text(&alt));
                    self.push_out("](");
                    self.push_out(&escape_url(&url));
                    if let Some(t) = &title {
                        self.push_out(" \"");
                        self.push_out(&escape_title(t));
                        self.push_out("\"");
                    }
                    self.push_out(")");
                }
                self.stack.push(Frame::Image);
            }
            Event::EndImage => {
                self.stack.pop();
            }

            // ── Inline leaf events ──────────────────────────────────────
            Event::Text(text) => {
                self.ensure_inline_context();
                if matches!(self.stack.last(), Some(Frame::Image)) {
                    // Alt text was already written from StartImage's own
                    // field; interior Text events are discarded to avoid
                    // double-counting (see module doc re: events() bug).
                } else if self.accepts_inline() {
                    self.push_out(&escape_text(&text));
                }
            }
            Event::Code(text) => {
                self.ensure_inline_context();
                if self.accepts_inline() {
                    if text.contains('`') {
                        self.push_out("`` ");
                        self.push_out(&text);
                        self.push_out(" ``");
                    } else {
                        self.push_out("`");
                        self.push_out(&text);
                        self.push_out("`");
                    }
                }
            }
            Event::HtmlInline(text) => {
                self.ensure_inline_context();
                if self.accepts_inline() {
                    self.push_out(&text);
                }
            }
            Event::SoftBreak => {
                self.ensure_inline_context();
                if self.accepts_inline() {
                    self.push_out("\n");
                }
            }
            Event::HardBreak => {
                self.ensure_inline_context();
                if self.accepts_inline() {
                    self.push_out("  \n");
                }
            }
        }
    }
}

/// Frames carry only a mark into the shared buffer and tiny scalars — never
/// accumulated content — with two exceptions: `ListItem`'s `indent` (a
/// computed space-repeat string, bounded by currently-open item nesting) and
/// `Link`'s `url`/`title` (small strings carried from `StartLink` to
/// `EndLink`, unavoidable since the closing `](url)` needs them and they are
/// not yet in `out` when the span opens). `Table`'s `alignments` similarly
/// carries the small `Vec<ColumnAlignment>` from `StartTable` through to the
/// alignment row written at `EndTableHead`.
enum Frame {
    Paragraph {
        mark: usize,
    },
    Heading {
        mark: usize,
    },
    Blockquote {
        mark: usize,
        child_count: usize,
    },
    List {
        ordered: bool,
        start: u64,
        tight: bool,
        item_idx: usize,
        mark: usize,
    },
    ListItem {
        mark: usize,
        indent: String,
        tight: bool,
        child_count: usize,
    },
    Emphasis {
        mark: usize,
    },
    Strong {
        mark: usize,
    },
    #[cfg(feature = "strikethrough")]
    Strikethrough {
        mark: usize,
    },
    Link {
        mark: usize,
        url: String,
        title: Option<String>,
    },
    /// Marker frame while inside `StartImage`/`EndImage`: the entire
    /// `![alt](url)` was already written at `StartImage`, so this exists
    /// only to make interior `Text` events discardable.
    Image,
    #[cfg(feature = "tables")]
    Table {
        mark: usize,
        alignments: Vec<ColumnAlignment>,
    },
    #[cfg(feature = "tables")]
    TableRow,
    #[cfg(feature = "tables")]
    TableCell,
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;
    use std::borrow::Cow;

    fn run(events: Vec<OwnedEvent>) -> String {
        let mut w = Writer::new(Vec::<u8>::new());
        for e in events {
            w.write_event(e);
        }
        let bytes = w.finish().unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn test_writer_paragraph() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartParagraph,
            OwnedEvent::Text(Cow::Borrowed("Hello")),
            OwnedEvent::EndParagraph,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("Hello"), "got: {out:?}");
        assert!(out.ends_with('\n'), "should end with newline, got: {out:?}");
    }

    #[test]
    fn test_writer_heading() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartHeading { level: 2 },
            OwnedEvent::Text(Cow::Borrowed("My heading")),
            OwnedEvent::EndHeading { level: 2 },
            OwnedEvent::EndDocument,
        ]);
        assert!(out.starts_with("## My heading"), "got: {out:?}");
    }

    #[test]
    fn test_writer_matches_emit() {
        let input = "# Title\n\nA paragraph with *em* and **strong**.\n\n- item one\n- item two\n";
        let (doc, _) = crate::parse::parse(input.as_bytes());
        let expected = String::from_utf8(crate::emit::emit(&doc)).unwrap();

        let evts: Vec<_> = crate::events::events_str(input)
            .map(|e| e.into_owned())
            .collect();
        let out = run(evts);

        let (ast_expected, _) = crate::parse::parse(expected.as_bytes());
        let (ast_out, _) = crate::parse::parse(out.as_bytes());
        assert_eq!(
            ast_expected.strip_spans(),
            ast_out.strip_spans(),
            "writer output doesn't match emit output\nexpected:\n{expected}\ngot:\n{out}"
        );
    }

    #[test]
    fn test_writer_code_block() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::CodeBlock {
                language: Some(Cow::Borrowed("rust")),
                content: Cow::Borrowed("fn main() {}\n"),
            },
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("```rust"), "got: {out:?}");
        assert!(out.contains("fn main() {}"), "got: {out:?}");
    }

    #[test]
    fn test_writer_blockquote() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartBlockquote,
            OwnedEvent::StartParagraph,
            OwnedEvent::Text(Cow::Borrowed("quoted")),
            OwnedEvent::EndParagraph,
            OwnedEvent::EndBlockquote,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("> quoted"), "got: {out:?}");
    }

    #[test]
    fn test_writer_thematic_break() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::ThematicBreak,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("---"), "got: {out:?}");
    }

    #[test]
    fn test_writer_link() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartParagraph,
            OwnedEvent::StartLink {
                url: Cow::Borrowed("https://example.com"),
                title: None,
            },
            OwnedEvent::Text(Cow::Borrowed("click")),
            OwnedEvent::EndLink,
            OwnedEvent::EndParagraph,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("[click](https://example.com)"), "got: {out:?}");
    }

    #[test]
    fn test_writer_image() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartParagraph,
            OwnedEvent::StartImage {
                url: Cow::Borrowed("img.png"),
                title: None,
                alt: Cow::Borrowed("alt text"),
            },
            // Text between StartImage/EndImage — should not be double-counted.
            OwnedEvent::Text(Cow::Borrowed("alt text")),
            OwnedEvent::EndImage,
            OwnedEvent::EndParagraph,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("![alt text](img.png)"), "got: {out:?}");
    }

    #[test]
    fn test_writer_emphasis_strong() {
        let out = run(vec![
            OwnedEvent::StartDocument,
            OwnedEvent::StartParagraph,
            OwnedEvent::StartEmphasis,
            OwnedEvent::Text(Cow::Borrowed("em")),
            OwnedEvent::EndEmphasis,
            OwnedEvent::Text(Cow::Borrowed(" and ")),
            OwnedEvent::StartStrong,
            OwnedEvent::Text(Cow::Borrowed("strong")),
            OwnedEvent::EndStrong,
            OwnedEvent::EndParagraph,
            OwnedEvent::EndDocument,
        ]);
        assert!(out.contains("*em*"), "got: {out:?}");
        assert!(out.contains("**strong**"), "got: {out:?}");
    }

    /// The streaming `Writer` must produce byte-identical output to the
    /// tree-based `emit()` for the same document, fed via `events()`. This
    /// is the guard keeping the two independent emission paths honest.
    ///
    /// All inputs here are genuinely *tight* lists (no blank line anywhere
    /// inside them). `events()`/`EventIter` has a real, pre-existing,
    /// separate bug (see `crates/formats/commonmark-fmt/src/events.rs`'s
    /// `PdEvent::Start(Tag::List(..))` arm: "Tightness is unknown until we
    /// see paragraphs; emit with tight=true tentatively") where it *always*
    /// reports `tight: true`, even for lists `parse()` correctly determines
    /// are loose. A loose-list input would therefore diverge from `emit()`
    /// no matter how faithfully this `Writer` reproduces whatever `events()`
    /// actually sends it — the same "garbage in" situation as the
    /// documented `fixtures/commonmark/image` divergence from the
    /// `commonmark`/`events` Text/StartImage ordering bug. Not this
    /// module's bug to fix; TODO.md tracks it separately.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "# Title\n\nA paragraph with *em* and **strong**.\n\n- item one\n- item two\n",
            "## Sub\n\ntext with `code` and a [link](http://x/) here.\n",
            "- bullet one\n- bullet two\n  - nested a\n  - nested b\n",
            "1. ordered one\n2. ordered two\n",
            "```rust\nlet x = 1;\nlet y = 2;\n```\n",
            "> A quoted paragraph.\n>\n> Second para of quote.\n",
            "> > Nested quote.\n",
            "Text with \\*escaped\\* markup and a \\\\ backslash.\n",
            "- - x\n- x\n",
            // Deliberately no bare top-level image input here: pulldown-cmark
            // wraps it in a real Paragraph, which is exactly the shape that
            // triggers the separate `events()` Text/StartImage ordering bug
            // documented above (reproduced verbatim by
            // `fixtures/commonmark/image` in the `rescribe-fixtures` harness)
            // — `test_writer_image` above already covers correct `Image`
            // handling via a hand-built, bug-free event sequence.
            "line one  \nline two\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input.as_bytes());
            let built = crate::emit::emit(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events::events_str(input) {
                w.write_event(e);
            }
            let streamed = w.finish().unwrap();

            assert_eq!(
                String::from_utf8_lossy(&built),
                String::from_utf8_lossy(&streamed),
                "streaming Writer diverged from emit() for input:\n{input}"
            );
        }
    }

    /// Regression guard for the buffer-then-emit implementation's latent
    /// reordering bug: text at the top of a tight list item followed by a
    /// nested block-level construct used to come out with the nested
    /// construct emitted *before* the text, because the old code
    /// accumulated `tight_inlines` separately from `blocks` and only
    /// appended the synthesized paragraph at `EndItem`, after any nested
    /// blocks already collected. This write-through implementation writes
    /// events in true arrival order, so it cannot exhibit that bug. Uses a
    /// genuinely tight list (no blank line anywhere) so `events()`'s
    /// separate always-`tight: true` limitation (see
    /// `test_writer_byte_identical_to_builder`'s doc comment) doesn't
    /// interfere.
    #[test]
    fn test_writer_tight_item_text_before_nested_list_order() {
        let input = "- text\n  - nested\n";
        let (doc, _) = crate::parse::parse(input.as_bytes());
        let built = String::from_utf8(crate::emit::emit(&doc)).unwrap();

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events_str(input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish().unwrap()).unwrap();

        assert_eq!(built, streamed, "input:\n{input}");
        let text_pos = streamed.find("text").expect("text present");
        let nested_pos = streamed.find("nested").expect("nested present");
        assert!(
            text_pos < nested_pos,
            "expected \"text\" before \"nested\", got:\n{streamed:?}"
        );
    }

    /// The streaming `Writer` must write bytes to the sink incrementally —
    /// before `finish()` — not only once the whole event stream has been
    /// consumed. This is the direct regression guard for the original bug
    /// (buffer-then-reconstruct-then-emit entirely inside `finish()`).
    #[test]
    fn test_writer_incremental_flush_before_finish() {
        use std::cell::RefCell;
        use std::rc::Rc;

        struct ObservableSink(Rc<RefCell<Vec<u8>>>);
        impl Write for ObservableSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.borrow_mut().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let observed = Rc::new(RefCell::new(Vec::<u8>::new()));
        let mut w = Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartDocument);
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text(Cow::Borrowed("Hello world")));
        w.write_event(Event::EndParagraph);
        w.write_event(Event::EndDocument);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        assert!(
            pre_finish > 0,
            "Writer wrote zero bytes to the sink before finish() — not a genuine incremental \
             streaming writer"
        );
    }

    /// Peak-memory guard: a large synthetic document (many independent
    /// top-level sections) must not cause `Writer::out` to grow anywhere
    /// close to the size of the full document, since each top-level block
    /// is flushed and the buffer cleared (capacity retained) before the
    /// next one starts.
    #[test]
    fn test_writer_out_buffer_stays_bounded() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartDocument);
        let mut max_out_len = 0usize;
        for i in 0..5000 {
            w.write_event(Event::StartHeading { level: 2 });
            w.write_event(Event::Text(Cow::Owned(format!("Section {i}"))));
            w.write_event(Event::EndHeading { level: 2 });
            w.write_event(Event::StartParagraph);
            w.write_event(Event::Text(Cow::Borrowed(
                "Some reasonably long paragraph text repeated to pad out the section body a bit.",
            )));
            w.write_event(Event::EndParagraph);
            max_out_len = max_out_len.max(w.out.len());
        }
        w.write_event(Event::EndDocument);
        let _ = w.finish();
        assert!(
            max_out_len < 8192,
            "Writer::out grew to {max_out_len} bytes across 5000 sections — buffer is not being \
             flushed and cleared per top-level block"
        );
    }
}
