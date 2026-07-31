#![allow(clippy::collapsible_if)]
//! Streaming Djot writer — converts a stream of events directly to Djot
//! text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value for the document as a whole and never calls [`crate::emit::emit`].
//! It is a second, independent emission path from the tree-based `emit()`
//! function, not a thin wrapper around it (the one exception — rendering a
//! table caption's `Vec<Inline>` payload, see below — is a small,
//! self-contained AST-to-markup helper, not a call into `emit.rs`).
//!
//! # Construct classification
//!
//! Mirroring `rst-fmt`'s `Writer` (see its module doc): most constructs are
//! **write-straight-through** into a single shared `out: String` buffer.
//! Sibling blocks inside any block-container (the top-level document,
//! `Blockquote`, `Div`, `ListItem`, `DefinitionDesc`, `FootnoteDef`) are
//! separated by a blank line, driven by a `child_count` scalar on the
//! container's frame (or `Writer::top_child_count` at the top level) —
//! [`Writer::block_start`]/[`Writer::block_end`] implement this once,
//! generically, for every block kind.
//!
//! Three constructs are **deferred per-line re-indentation**, using a
//! pooled scratch buffer exactly like `rst-fmt`'s `Blockquote`/
//! `Admonition`/`CodeBlock`:
//! - `Blockquote`: every line of its already-written inner content gets a
//!   `"> "` prefix ([`Writer::reindent_all`]).
//! - `DefinitionDesc`: every line gets a `"  "` prefix (same helper).
//! - `ListItem`/`FootnoteDef`: the *first* line continues the marker/label
//!   line already written (`"- "`, `"1. "`, `"[^label]: "`), so only lines
//!   after the first get the `"  "` prefix ([`Writer::reindent_tail`]).
//!   `ListItem` additionally uses the parent `List`'s own single-newline
//!   (not blank-line) between-items rule, tracked as `item_count` on the
//!   `List` frame — a different cadence from the generic sibling-block rule,
//!   so it does not go through `block_start`/`block_end`.
//!
//! `Table` is the one genuinely content-dependent-prefix construct, exactly
//! as in `rst-fmt`: the header separator row's column count and alignments
//! aren't known until every row has been seen, so rows are collected as
//! `(formatted cell markup, alignment)` pairs — bounded by the table's own
//! size — and rendered by [`render_table`] at `EndTable`. Each cell's
//! *formatted* markup (not just plain text — Djot table cells can contain
//! inline spans) is captured the same way `rst-fmt` captures heading plain
//! text: written straight into `out` under a mark, then sliced out to an
//! owned `String` and truncated back at `EndTableCell`.
//!
//! `Div` (unlike `Blockquote`) is write-straight-through: `emit.rs`'s own
//! `Block::Div` arm writes its children directly via `emit_blocks`, no
//! sub-emitter or re-indentation — confirmed by reading `emit.rs` before
//! writing this module, not assumed by analogy with `Blockquote`.
//!
//! A table caption arrives as one atomic `Event::TableCaption(Vec<Inline>)`
//! payload immediately before `StartTable` (not as streamed Start/Text/End
//! sub-events — that's how `events()` itself defines the event, mirroring
//! `ast::Block::Table`'s `caption: Option<Vec<Inline>>` field). Formatting
//! that payload needs a small inline-AST-to-markup renderer
//! ([`render_inlines_ast`]) independent of `emit.rs`'s private `Emitter`
//! methods — this is the one place `Writer` renders from an AST fragment
//! rather than from streamed events, forced by the event vocabulary's own
//! shape, not a shortcut around the rest of the design.
//!
//! Unlike `rst-fmt`'s `Writer::side` (a side-stack that shrinks the hot
//! `Frame` enum by moving a few wide payload variants off it), this module
//! keeps `Table`/`TableRow` payloads inline on `Frame` directly. Djot's
//! frame-size distribution wasn't profiled the way `rst-fmt`'s was — this
//! is a documented simplification, not a claim that the layout is optimal.
//!
//! # Example
//! ```no_run
//! use djot_fmt::writer::Writer;
//! use djot_fmt::OwnedEvent;
//! use std::borrow::Cow;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, id: None, classes: vec![], kv: vec![] });
//! w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::{Alignment, Attr, BulletStyle, Inline, ListKind, OrderedDelimiter, OrderedStyle};
use crate::events::Event;
use std::io::Write;

/// Streaming Djot writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// document child (block, footnote def, or link def) is flushed to the sink
/// as soon as it closes. Call [`finish`](Writer::finish) to recover the
/// sink once all events have been fed.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Cleared (capacity retained) after
    /// each top-level document child is flushed.
    out: String,
    /// Pool of scratch buffers for the deferred per-line re-indent
    /// constructs (`Blockquote`, `DefinitionDesc`, `ListItem`,
    /// `FootnoteDef`). Buffers are returned after use, so at most
    /// `O(nesting depth)` are ever allocated for a whole document.
    scratch: Vec<String>,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level.
    stack: Vec<Frame>,
    /// Number of top-level document children (blocks, footnote defs, link
    /// defs) emitted so far — the top-level equivalent of a container
    /// frame's `child_count` field, since there's no `Frame::Document` on
    /// the stack itself (empty stack *is* top level).
    top_child_count: usize,
    /// Set by `Event::TableCaption`, consumed by the very next `StartTable`.
    /// Pre-rendered Djot markup for the caption (formatted eagerly at
    /// `TableCaption` time via [`render_inlines_ast`], since the payload is
    /// already a complete `Vec<Inline>` — no reason to defer it).
    pending_caption: Option<String>,
}

/// Default capacity reserved for `Writer::out`, mirroring `rst-fmt`'s
/// `DEFAULT_OUT_CAPACITY`.
const DEFAULT_OUT_CAPACITY: usize = 4096;

impl<W: Write> Writer<W> {
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
            top_child_count: 0,
            pending_caption: None,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level document child.
    pub fn write_event(&mut self, event: Event<'_>) {
        self.process(event);
    }

    /// Recover the underlying sink. Does not write anything — every
    /// completed top-level document child was already flushed by
    /// `write_event`.
    pub fn finish(self) -> W {
        self.sink
    }

    // ── Buffer primitives ─────────────────────────────────────────────────

    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Whether the top-of-stack frame is a block-container (empty stack
    /// counts as the top-level document).
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(
                Frame::Blockquote { .. }
                    | Frame::Div { .. }
                    | Frame::ListItem { .. }
                    | Frame::DefinitionDesc { .. }
                    | Frame::FootnoteDef { .. }
            )
        )
    }

    /// Whether the top-of-stack frame accepts inline children.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::DefinitionTerm { .. }
                    | Frame::TableCell { .. }
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
                    | Frame::Image { .. }
            )
        )
    }

    /// Bump the current block-container's child counter and report whether
    /// a blank-line separator is needed before the child now being opened
    /// (i.e. this isn't the container's first child). Only meaningful when
    /// [`accepts_blocks`](Self::accepts_blocks) is true; callers check that
    /// separately (this only mutates counters on a real container frame).
    fn bump_child_count(&mut self) -> bool {
        match self.stack.last_mut() {
            None => {
                let needs_blank = self.top_child_count > 0;
                self.top_child_count += 1;
                needs_blank
            }
            Some(
                Frame::Blockquote { child_count, .. }
                | Frame::Div { child_count, .. }
                | Frame::ListItem { child_count, .. }
                | Frame::DefinitionDesc { child_count, .. }
                | Frame::FootnoteDef { child_count, .. },
            ) => {
                let needs_blank = *child_count > 0;
                *child_count += 1;
                needs_blank
            }
            _ => false,
        }
    }

    /// Open a block-level construct: write a blank-line separator first if
    /// this isn't the first child of its (possibly invalid) container, and
    /// return the mark this construct's content begins at.
    fn block_start(&mut self) -> usize {
        if self.accepts_blocks() && self.bump_child_count() {
            self.out.push('\n');
        }
        self.out.len()
    }

    /// Close a block-level construct: discard everything since `mark` if
    /// the container turned out invalid, otherwise ensure exactly one
    /// trailing newline and flush if this completed a top-level child.
    fn block_end(&mut self, mark: usize) {
        if !self.accepts_blocks() {
            self.out.truncate(mark);
            return;
        }
        if !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Close an inline span: discard it if the enclosing frame does not
    /// accept inline children.
    fn inline_end(&mut self, mark: usize) {
        if !self.accepts_inline() {
            self.out.truncate(mark);
        }
    }

    fn write_attr_line(&mut self, attr: &TmpAttr<'_>) {
        if attr.is_empty() {
            return;
        }
        write_attr(&mut self.out, attr);
        self.out.push('\n');
    }

    fn write_attr_inline(&mut self, attr: &TmpAttr<'_>) {
        if !attr.is_empty() {
            write_attr(&mut self.out, attr);
        }
    }

    /// Open an inline span whose opening delimiter is already known: write
    /// it straight through and record the marks needed to undo it if the
    /// span turns out to have no valid enclosing context.
    fn open_inline_span(&mut self, open: &str, close: &'static str, attr: TmpAttr<'_>) {
        let mark = self.out.len();
        self.out.push_str(open);
        self.stack.push(Frame::Inline {
            mark,
            close,
            attr_suffix: attr.format_suffix(),
        });
    }

    fn close_inline_span(&mut self) {
        if let Some(Frame::Inline {
            mark,
            close,
            attr_suffix,
        }) = self.stack.pop()
        {
            self.out.push_str(close);
            self.out.push_str(&attr_suffix);
            self.inline_end(mark);
        }
    }

    /// Re-indent every line of `out[mark..]` by `prefix`, in place, using a
    /// pooled scratch buffer — mirrors `rst-fmt::writer::Writer::reindent`.
    fn reindent_all(&mut self, mark: usize, prefix: &str) {
        let mut buf = self.scratch.pop().unwrap_or_default();
        buf.clear();
        for line in self.out[mark..].lines() {
            buf.push_str(prefix);
            buf.push_str(line);
            buf.push('\n');
        }
        self.out.truncate(mark);
        self.out.push_str(&buf);
        self.scratch.push(buf);
    }

    /// Like [`reindent_all`](Self::reindent_all), but the first line is left
    /// untouched (it continues a marker/label line already written before
    /// `mark`) — only lines after the first get `prefix`.
    fn reindent_tail(&mut self, mark: usize, prefix: &str) {
        let mut buf = self.scratch.pop().unwrap_or_default();
        buf.clear();
        for (i, line) in self.out[mark..].lines().enumerate() {
            if i > 0 {
                buf.push_str(prefix);
            }
            buf.push_str(line);
            buf.push('\n');
        }
        self.out.truncate(mark);
        self.out.push_str(&buf);
        self.scratch.push(buf);
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event<'_>) {
        match event {
            // ── Block open/close ────────────────────────────────────────
            Event::StartParagraph { id, classes, kv } => {
                let mark = self.block_start();
                self.write_attr_line(&TmpAttr::Borrowed(&id, &classes, &kv));
                self.stack.push(Frame::Paragraph { mark });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    self.block_end(mark);
                }
            }
            Event::StartHeading {
                level,
                id,
                classes,
                kv,
            } => {
                let mark = self.block_start();
                self.write_attr_line(&TmpAttr::Borrowed(&id, &classes, &kv));
                for _ in 0..level {
                    self.out.push('#');
                }
                self.out.push(' ');
                self.stack.push(Frame::Heading { mark });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { mark }) = self.stack.pop() {
                    self.block_end(mark);
                }
            }
            Event::StartBlockquote { id, classes, kv } => {
                let mark = self.block_start();
                self.write_attr_line(&TmpAttr::Borrowed(&id, &classes, &kv));
                let content_mark = self.out.len();
                self.stack.push(Frame::Blockquote {
                    mark,
                    content_mark,
                    child_count: 0,
                });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote {
                    mark, content_mark, ..
                }) = self.stack.pop()
                {
                    self.reindent_all(content_mark, "> ");
                    self.block_end(mark);
                }
            }
            Event::StartList {
                kind,
                tight: _,
                id,
                classes,
                kv,
            } => {
                let mark = self.block_start();
                self.write_attr_line(&TmpAttr::Borrowed(&id, &classes, &kv));
                self.stack.push(Frame::List {
                    mark,
                    kind,
                    item_count: 0,
                });
            }
            Event::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.block_end(mark);
                }
            }
            Event::StartListItem { checked } => {
                let mark = self.out.len();
                if let Some(Frame::List {
                    kind, item_count, ..
                }) = self.stack.last_mut()
                {
                    if *item_count > 0 {
                        self.out.push('\n');
                    }
                    let marker = list_item_marker(kind, *item_count);
                    *item_count += 1;
                    self.out.push_str(&marker);
                    if let Some(checked) = checked {
                        self.out.push_str(if checked { "[x] " } else { "[ ] " });
                    }
                }
                let content_mark = self.out.len();
                self.stack.push(Frame::ListItem {
                    mark,
                    content_mark,
                    child_count: 0,
                });
            }
            Event::EndListItem => {
                if let Some(Frame::ListItem {
                    mark, content_mark, ..
                }) = self.stack.pop()
                {
                    self.reindent_tail(content_mark, "  ");
                    if matches!(self.stack.last(), Some(Frame::List { .. })) {
                        // Mirrors emit.rs's emit_list: the item's own
                        // trailing newline is not kept — the *next* item's
                        // StartListItem inserts exactly one newline before
                        // its marker (or, for the last item, the enclosing
                        // List's own block_end ensures the single trailing
                        // newline instead). Keeping it here would double up
                        // into a blank line between items.
                        if self.out.ends_with('\n') {
                            self.out.pop();
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartCodeBlock {
                language,
                id,
                classes,
                kv,
            } => {
                let mark = self.block_start();
                self.write_attr_line(&TmpAttr::Borrowed(&id, &classes, &kv));
                self.out.push_str("```");
                if let Some(lang) = &language {
                    self.out.push_str(lang);
                }
                self.out.push('\n');
                let content_mark = self.out.len();
                self.stack.push(Frame::CodeBlock { mark, content_mark });
            }
            Event::CodeBlockContent(content) => {
                if matches!(self.stack.last(), Some(Frame::CodeBlock { .. })) {
                    self.out.push_str(&content);
                }
            }
            Event::EndCodeBlock => {
                if let Some(Frame::CodeBlock { mark, content_mark }) = self.stack.pop() {
                    if !self.out[content_mark..].ends_with('\n') {
                        self.out.push('\n');
                    }
                    self.out.push_str("```");
                    self.block_end(mark);
                }
            }
            Event::RawBlock { format, content } => {
                let mark = self.block_start();
                self.out.push_str("```=");
                self.out.push_str(&format);
                self.out.push('\n');
                self.out.push_str(&content);
                if !content.ends_with('\n') {
                    self.out.push('\n');
                }
                self.out.push_str("```");
                self.block_end(mark);
            }
            Event::StartDiv {
                class,
                id,
                classes,
                kv,
            } => {
                let mark = self.block_start();
                self.write_attr_line(&TmpAttr::Borrowed(&id, &classes, &kv));
                self.out.push_str(":::");
                if let Some(cls) = &class {
                    self.out.push(' ');
                    self.out.push_str(cls);
                }
                self.out.push('\n');
                self.stack.push(Frame::Div {
                    mark,
                    child_count: 0,
                });
            }
            Event::EndDiv => {
                if let Some(Frame::Div { mark, .. }) = self.stack.pop() {
                    // emit.rs writes an unconditional extra newline before
                    // the closing fence (`emit_blocks(blocks); newline();
                    // push(":::")`), i.e. a blank line even when the last
                    // child already ended with its own single trailing
                    // newline.
                    if !self.out.ends_with('\n') {
                        self.out.push('\n');
                    }
                    self.out.push('\n');
                    self.out.push_str(":::");
                    self.block_end(mark);
                }
            }
            Event::TableCaption(inlines) => {
                let mut s = String::new();
                render_inlines_ast(&inlines, &mut s);
                self.pending_caption = Some(s);
            }
            Event::StartTable => {
                let mark = self.block_start();
                let caption = self.pending_caption.take();
                if let Some(cap) = &caption {
                    self.out.push_str("^ ");
                    self.out.push_str(cap);
                    self.out.push('\n');
                }
                self.stack.push(Frame::Table {
                    mark,
                    rows: Vec::new(),
                });
            }
            Event::EndTable => {
                if let Some(Frame::Table { mark, rows }) = self.stack.pop() {
                    render_table(&rows, &mut self.out);
                    self.block_end(mark);
                }
            }
            Event::StartTableRow { is_header } => {
                self.stack.push(Frame::TableRow {
                    cells: Vec::new(),
                    is_header,
                });
            }
            Event::EndTableRow => {
                if let Some(Frame::TableRow { cells, is_header }) = self.stack.pop() {
                    if let Some(Frame::Table { rows, .. }) = self.stack.last_mut() {
                        rows.push((cells, is_header));
                    }
                }
            }
            Event::StartTableCell { alignment } => {
                let mark = self.out.len();
                self.stack.push(Frame::TableCell { mark, alignment });
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell { mark, alignment }) = self.stack.pop() {
                    let cell = self.out[mark..].to_string();
                    self.out.truncate(mark);
                    if let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut() {
                        cells.push((cell, alignment));
                    }
                }
            }
            Event::ThematicBreak { id, classes, kv } => {
                let mark = self.block_start();
                self.write_attr_line(&TmpAttr::Borrowed(&id, &classes, &kv));
                self.out.push_str("* * *");
                self.block_end(mark);
            }
            Event::StartDefinitionList { id, classes, kv } => {
                let mark = self.block_start();
                self.write_attr_line(&TmpAttr::Borrowed(&id, &classes, &kv));
                self.stack.push(Frame::DefinitionList {
                    mark,
                    item_count: 0,
                });
            }
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark, .. }) = self.stack.pop() {
                    self.block_end(mark);
                }
            }
            Event::StartDefinitionTerm => {
                let mark = self.out.len();
                if let Some(Frame::DefinitionList { item_count, .. }) = self.stack.last_mut() {
                    if *item_count > 0 {
                        self.out.push('\n');
                    }
                    *item_count += 1;
                }
                self.out.push_str(": ");
                self.stack.push(Frame::DefinitionTerm { mark });
            }
            Event::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.out.push('\n');
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartDefinitionDesc => {
                let content_mark = self.out.len();
                self.stack.push(Frame::DefinitionDesc {
                    content_mark,
                    child_count: 0,
                });
            }
            Event::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { content_mark, .. }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.reindent_all(content_mark, "  ");
                    } else {
                        self.out.truncate(content_mark);
                    }
                }
            }
            Event::StartFootnoteDef { label } => {
                let mark = self.block_start();
                self.out.push_str("[^");
                self.out.push_str(&label);
                self.out.push_str("]: ");
                let content_mark = self.out.len();
                self.stack.push(Frame::FootnoteDef {
                    mark,
                    content_mark,
                    child_count: 0,
                });
            }
            Event::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef {
                    mark, content_mark, ..
                }) = self.stack.pop()
                {
                    self.reindent_tail(content_mark, "  ");
                    self.block_end(mark);
                }
            }
            Event::LinkDef {
                label,
                url,
                title,
                id,
                classes,
                kv,
            } => {
                let mark = self.block_start();
                self.out.push('[');
                self.out.push_str(&label);
                self.out.push_str("]: ");
                self.out.push_str(&url);
                if let Some(t) = &title {
                    self.out.push_str(" \"");
                    self.out.push_str(t);
                    self.out.push('"');
                }
                self.write_attr_inline(&TmpAttr::Borrowed(&id, &classes, &kv));
                self.block_end(mark);
            }

            // ── Inline events ───────────────────────────────────────────
            Event::Text(cow) => {
                if self.accepts_inline() {
                    self.out.push_str(&cow);
                }
            }
            Event::SoftBreak => {
                if self.accepts_inline() {
                    self.out.push('\n');
                }
            }
            Event::HardBreak => {
                if self.accepts_inline() {
                    self.out.push_str("\\\n");
                }
            }
            Event::StartEmphasis { id, classes, kv } => {
                self.open_inline_span("_", "_", TmpAttr::Owned(id, classes, kv));
            }
            Event::EndEmphasis => self.close_inline_span(),
            Event::StartStrong { id, classes, kv } => {
                self.open_inline_span("*", "*", TmpAttr::Owned(id, classes, kv));
            }
            Event::EndStrong => self.close_inline_span(),
            Event::StartDelete { id, classes, kv } => {
                self.open_inline_span("{-", "-}", TmpAttr::Owned(id, classes, kv));
            }
            Event::EndDelete => self.close_inline_span(),
            Event::StartInsert { id, classes, kv } => {
                self.open_inline_span("{+", "+}", TmpAttr::Owned(id, classes, kv));
            }
            Event::EndInsert => self.close_inline_span(),
            Event::StartHighlight { id, classes, kv } => {
                self.open_inline_span("{=", "=}", TmpAttr::Owned(id, classes, kv));
            }
            Event::EndHighlight => self.close_inline_span(),
            Event::StartSubscript { id, classes, kv } => {
                self.open_inline_span("~", "~", TmpAttr::Owned(id, classes, kv));
            }
            Event::EndSubscript => self.close_inline_span(),
            Event::StartSuperscript { id, classes, kv } => {
                self.open_inline_span("^", "^", TmpAttr::Owned(id, classes, kv));
            }
            Event::EndSuperscript => self.close_inline_span(),
            Event::Verbatim {
                content,
                id,
                classes,
                kv,
            } => {
                if self.accepts_inline() {
                    let ticks = choose_backticks(&content);
                    self.out.push_str(&ticks);
                    let pad = content.starts_with('`') || content.ends_with('`');
                    if pad {
                        self.out.push(' ');
                    }
                    self.out.push_str(&content);
                    if pad {
                        self.out.push(' ');
                    }
                    self.out.push_str(&ticks);
                    self.write_attr_inline(&TmpAttr::Borrowed(&id, &classes, &kv));
                }
            }
            Event::MathInline(content) => {
                if self.accepts_inline() {
                    let ticks = choose_backticks(&content);
                    self.out.push('$');
                    self.out.push_str(&ticks);
                    self.out.push_str(&content);
                    self.out.push_str(&ticks);
                }
            }
            Event::MathDisplay(content) => {
                if self.accepts_inline() {
                    let ticks = choose_backticks(&content);
                    self.out.push_str("$$");
                    self.out.push_str(&ticks);
                    self.out.push_str(&content);
                    self.out.push_str(&ticks);
                }
            }
            Event::RawInline { format, content } => {
                if self.accepts_inline() {
                    let ticks = choose_backticks(&content);
                    self.out.push_str(&ticks);
                    self.out.push_str(&content);
                    self.out.push_str(&ticks);
                    self.out.push_str("{=");
                    self.out.push_str(&format);
                    self.out.push('}');
                }
            }
            Event::StartLink {
                url,
                title,
                id,
                classes,
                kv,
            } => {
                let mark = self.out.len();
                self.out.push('[');
                self.stack.push(Frame::Link {
                    mark,
                    url,
                    title,
                    attr_suffix: TmpAttr::Owned(id, classes, kv).format_suffix(),
                });
            }
            Event::EndLink => {
                if let Some(Frame::Link {
                    mark,
                    url,
                    title,
                    attr_suffix,
                }) = self.stack.pop()
                {
                    self.out.push_str("](");
                    self.out.push_str(&url);
                    if let Some(t) = &title {
                        self.out.push_str(" \"");
                        self.out.push_str(t);
                        self.out.push('"');
                    }
                    self.out.push(')');
                    self.out.push_str(&attr_suffix);
                    self.inline_end(mark);
                }
            }
            Event::StartImage {
                url,
                title,
                id,
                classes,
                kv,
            } => {
                let mark = self.out.len();
                self.out.push_str("![");
                self.stack.push(Frame::Image {
                    mark,
                    url,
                    title,
                    attr_suffix: TmpAttr::Owned(id, classes, kv).format_suffix(),
                });
            }
            Event::EndImage => {
                if let Some(Frame::Image {
                    mark,
                    url,
                    title,
                    attr_suffix,
                }) = self.stack.pop()
                {
                    self.out.push_str("](");
                    self.out.push_str(&url);
                    if let Some(t) = &title {
                        self.out.push_str(" \"");
                        self.out.push_str(t);
                        self.out.push('"');
                    }
                    self.out.push(')');
                    self.out.push_str(&attr_suffix);
                    self.inline_end(mark);
                }
            }
            Event::StartSpan { id, classes, kv } => {
                self.open_inline_span("[", "]", TmpAttr::Owned(id, classes, kv));
            }
            Event::EndSpan => self.close_inline_span(),
            Event::FootnoteRef(label) => {
                if self.accepts_inline() {
                    self.out.push_str("[^");
                    self.out.push_str(&label);
                    self.out.push(']');
                }
            }
            Event::Symbol(name) => {
                if self.accepts_inline() {
                    self.out.push(':');
                    self.out.push_str(&name);
                    self.out.push(':');
                }
            }
            Event::Autolink { url, is_email } => {
                if self.accepts_inline() {
                    self.out.push('<');
                    if is_email {
                        self.out
                            .push_str(url.strip_prefix("mailto:").unwrap_or(&url));
                    } else {
                        self.out.push_str(&url);
                    }
                    self.out.push('>');
                }
            }
        }
    }
}

/// Borrowed-or-owned view over the three attr fields, avoiding a clone at
/// every `Start*` event just to build an `Attr` for formatting.
enum TmpAttr<'a> {
    Borrowed(&'a Option<String>, &'a [String], &'a [(String, String)]),
    Owned(Option<String>, Vec<String>, Vec<(String, String)>),
}

impl TmpAttr<'_> {
    fn is_empty(&self) -> bool {
        match self {
            TmpAttr::Borrowed(id, classes, kv) => {
                id.is_none() && classes.is_empty() && kv.is_empty()
            }
            TmpAttr::Owned(id, classes, kv) => id.is_none() && classes.is_empty() && kv.is_empty(),
        }
    }

    fn id(&self) -> Option<&str> {
        match self {
            TmpAttr::Borrowed(id, ..) => id.as_deref(),
            TmpAttr::Owned(id, ..) => id.as_deref(),
        }
    }

    fn classes(&self) -> &[String] {
        match self {
            TmpAttr::Borrowed(_, classes, _) => classes,
            TmpAttr::Owned(_, classes, _) => classes,
        }
    }

    fn kv(&self) -> &[(String, String)] {
        match self {
            TmpAttr::Borrowed(_, _, kv) => kv,
            TmpAttr::Owned(_, _, kv) => kv,
        }
    }

    /// Pre-render `format_attr_inline`'s output as an owned `String`, for
    /// storing on a `Frame` until the matching `End*` event.
    fn format_suffix(&self) -> String {
        if self.is_empty() {
            String::new()
        } else {
            let mut s = String::new();
            write_attr(&mut s, self);
            s
        }
    }
}

fn write_attr(out: &mut String, attr: &TmpAttr<'_>) {
    out.push('{');
    let mut first = true;
    if let Some(id) = attr.id() {
        out.push('#');
        out.push_str(id);
        first = false;
    }
    for cls in attr.classes() {
        if !first {
            out.push(' ');
        }
        out.push('.');
        out.push_str(cls);
        first = false;
    }
    for (k, v) in attr.kv() {
        if !first {
            out.push(' ');
        }
        out.push_str(k);
        out.push('=');
        if v.contains('"') || v.contains(' ') {
            out.push('"');
            out.push_str(v);
            out.push('"');
        } else {
            out.push_str(v);
        }
        first = false;
    }
    out.push('}');
}

/// Small, self-contained inline-AST-to-Djot-markup renderer, independent of
/// `emit.rs`'s private `Emitter` methods — used only for `TableCaption`'s
/// atomic `Vec<Inline>` payload (see the module doc for why that one event
/// carries an AST fragment instead of streamed sub-events).
fn render_inlines_ast(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        render_inline_ast(inline, out);
    }
}

fn render_inline_ast(inline: &Inline, out: &mut String) {
    match inline {
        Inline::Text { content, .. } => out.push_str(content),
        Inline::SoftBreak { .. } => out.push('\n'),
        Inline::HardBreak { .. } => out.push_str("\\\n"),
        Inline::Emphasis { inlines, attr, .. } => {
            out.push('_');
            render_inlines_ast(inlines, out);
            out.push('_');
            write_attr_ast_inline(out, attr);
        }
        Inline::Strong { inlines, attr, .. } => {
            out.push('*');
            render_inlines_ast(inlines, out);
            out.push('*');
            write_attr_ast_inline(out, attr);
        }
        Inline::Delete { inlines, attr, .. } => {
            out.push_str("{-");
            render_inlines_ast(inlines, out);
            out.push_str("-}");
            write_attr_ast_inline(out, attr);
        }
        Inline::Insert { inlines, attr, .. } => {
            out.push_str("{+");
            render_inlines_ast(inlines, out);
            out.push_str("+}");
            write_attr_ast_inline(out, attr);
        }
        Inline::Highlight { inlines, attr, .. } => {
            out.push_str("{=");
            render_inlines_ast(inlines, out);
            out.push_str("=}");
            write_attr_ast_inline(out, attr);
        }
        Inline::Subscript { inlines, attr, .. } => {
            out.push('~');
            render_inlines_ast(inlines, out);
            out.push('~');
            write_attr_ast_inline(out, attr);
        }
        Inline::Superscript { inlines, attr, .. } => {
            out.push('^');
            render_inlines_ast(inlines, out);
            out.push('^');
            write_attr_ast_inline(out, attr);
        }
        Inline::Verbatim { content, attr, .. } => {
            let ticks = choose_backticks(content);
            out.push_str(&ticks);
            let pad = content.starts_with('`') || content.ends_with('`');
            if pad {
                out.push(' ');
            }
            out.push_str(content);
            if pad {
                out.push(' ');
            }
            out.push_str(&ticks);
            write_attr_ast_inline(out, attr);
        }
        Inline::MathInline { content, .. } => {
            let ticks = choose_backticks(content);
            out.push('$');
            out.push_str(&ticks);
            out.push_str(content);
            out.push_str(&ticks);
        }
        Inline::MathDisplay { content, .. } => {
            let ticks = choose_backticks(content);
            out.push_str("$$");
            out.push_str(&ticks);
            out.push_str(content);
            out.push_str(&ticks);
        }
        Inline::RawInline {
            format, content, ..
        } => {
            let ticks = choose_backticks(content);
            out.push_str(&ticks);
            out.push_str(content);
            out.push_str(&ticks);
            out.push_str("{=");
            out.push_str(format);
            out.push('}');
        }
        Inline::Link {
            inlines,
            url,
            title,
            attr,
            ..
        } => {
            out.push('[');
            render_inlines_ast(inlines, out);
            out.push_str("](");
            out.push_str(url);
            if let Some(t) = title {
                out.push_str(" \"");
                out.push_str(t);
                out.push('"');
            }
            out.push(')');
            write_attr_ast_inline(out, attr);
        }
        Inline::Image {
            inlines,
            url,
            title,
            attr,
            ..
        } => {
            out.push_str("![");
            render_inlines_ast(inlines, out);
            out.push_str("](");
            out.push_str(url);
            if let Some(t) = title {
                out.push_str(" \"");
                out.push_str(t);
                out.push('"');
            }
            out.push(')');
            write_attr_ast_inline(out, attr);
        }
        Inline::Span { inlines, attr, .. } => {
            out.push('[');
            render_inlines_ast(inlines, out);
            out.push(']');
            write_attr_ast_inline(out, attr);
        }
        Inline::FootnoteRef { label, .. } => {
            out.push_str("[^");
            out.push_str(label);
            out.push(']');
        }
        Inline::Symbol { name, .. } => {
            out.push(':');
            out.push_str(name);
            out.push(':');
        }
        Inline::Autolink { url, is_email, .. } => {
            out.push('<');
            if *is_email {
                out.push_str(url.strip_prefix("mailto:").unwrap_or(url));
            } else {
                out.push_str(url);
            }
            out.push('>');
        }
    }
}

fn write_attr_ast_inline(out: &mut String, attr: &Attr) {
    if attr.is_empty() {
        return;
    }
    write_attr(out, &TmpAttr::Borrowed(&attr.id, &attr.classes, &attr.kv));
}

/// Mirrors `emit.rs`'s `render`-at-`EndTable` shape: cells carry their
/// already-*formatted* Djot markup (captured under a mark and sliced out at
/// `EndTableCell`, since cells can contain inline spans, not just plain
/// text), so no re-formatting is needed here — only the column-count and
/// header-alignment computation that genuinely needs every row to be known.
fn render_table(rows: &[(Vec<(String, Alignment)>, bool)], out: &mut String) {
    let max_cells = rows.iter().map(|(cells, _)| cells.len()).max().unwrap_or(0);
    let alignments: Vec<Alignment> = rows
        .iter()
        .find(|(_, is_header)| *is_header)
        .map(|(cells, _)| cells.iter().map(|(_, a)| a.clone()).collect())
        .unwrap_or_default();

    for (cells, is_header) in rows {
        out.push('|');
        for (cell, _) in cells {
            out.push(' ');
            out.push_str(cell);
            out.push_str(" |");
        }
        out.push('\n');
        if *is_header {
            out.push('|');
            for i in 0..max_cells {
                let align = alignments.get(i).unwrap_or(&Alignment::Default);
                out.push_str(match align {
                    Alignment::Left => ":---|",
                    Alignment::Right => "---:|",
                    Alignment::Center => ":---:|",
                    Alignment::Default => "----|",
                });
            }
            out.push('\n');
        }
    }
    if out.ends_with('\n') {
        out.pop();
    }
}

fn list_item_marker(kind: &ListKind, idx: u32) -> String {
    match kind {
        ListKind::Bullet(BulletStyle::Dash) => "- ".to_string(),
        ListKind::Bullet(BulletStyle::Star) => "* ".to_string(),
        ListKind::Bullet(BulletStyle::Plus) => "+ ".to_string(),
        ListKind::Task => "- ".to_string(),
        ListKind::Ordered {
            style,
            delimiter,
            start,
        } => {
            let n = start + idx;
            let num_str = format_ordered_number(n, style);
            match delimiter {
                OrderedDelimiter::Period => format!("{num_str}. "),
                OrderedDelimiter::Paren => format!("{num_str}) "),
                OrderedDelimiter::Enclosed => format!("({num_str}) "),
            }
        }
    }
}

fn format_ordered_number(n: u32, style: &OrderedStyle) -> String {
    match style {
        OrderedStyle::Decimal => n.to_string(),
        OrderedStyle::LowerAlpha => {
            let idx = (n.saturating_sub(1)) % 26;
            ((b'a' + idx as u8) as char).to_string()
        }
        OrderedStyle::UpperAlpha => {
            let idx = (n.saturating_sub(1)) % 26;
            ((b'A' + idx as u8) as char).to_string()
        }
        OrderedStyle::LowerRoman => to_roman(n).to_lowercase(),
        OrderedStyle::UpperRoman => to_roman(n),
    }
}

fn to_roman(n: u32) -> String {
    let vals = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut result = String::new();
    let mut n = n;
    for (val, sym) in vals {
        while n >= val {
            result.push_str(sym);
            n -= val;
        }
    }
    if result.is_empty() {
        "0".to_string()
    } else {
        result
    }
}

fn choose_backticks(content: &str) -> String {
    let mut max = 0;
    let mut current = 0;
    for c in content.chars() {
        if c == '`' {
            current += 1;
            max = max.max(current);
        } else {
            current = 0;
        }
    }
    "`".repeat(max + 1)
}

/// Frames carry only marks into the shared buffer and tiny scalars — never
/// accumulated content, except where a construct is genuinely
/// content-dependent (`Table`/`TableRow` collect formatted cell strings,
/// bounded by the table's own size — see the module doc).
enum Frame {
    Paragraph {
        mark: usize,
    },
    Heading {
        mark: usize,
    },
    Blockquote {
        mark: usize,
        content_mark: usize,
        child_count: usize,
    },
    List {
        mark: usize,
        kind: ListKind,
        item_count: u32,
    },
    ListItem {
        mark: usize,
        content_mark: usize,
        child_count: usize,
    },
    CodeBlock {
        mark: usize,
        content_mark: usize,
    },
    Div {
        mark: usize,
        child_count: usize,
    },
    Table {
        mark: usize,
        rows: Vec<(Vec<(String, Alignment)>, bool)>,
    },
    TableRow {
        cells: Vec<(String, Alignment)>,
        is_header: bool,
    },
    TableCell {
        mark: usize,
        alignment: Alignment,
    },
    DefinitionList {
        mark: usize,
        item_count: usize,
    },
    DefinitionTerm {
        mark: usize,
    },
    DefinitionDesc {
        content_mark: usize,
        child_count: usize,
    },
    FootnoteDef {
        mark: usize,
        content_mark: usize,
        child_count: usize,
    },
    /// Any inline span whose closing text is a fixed string plus an
    /// optional attr suffix (emphasis/strong/delete/insert/highlight/
    /// subscript/superscript/span) — the attr suffix is pre-rendered at
    /// `Start*` time (all fields are known then) and stashed here.
    Inline {
        mark: usize,
        close: &'static str,
        attr_suffix: String,
    },
    /// `[text](url["title"]){attr}` — closing text depends on data carried
    /// by the frame (url/title), not just a fixed string.
    Link {
        mark: usize,
        url: String,
        title: Option<String>,
        attr_suffix: String,
    },
    /// `![alt](url["title"]){attr}` — same shape as `Link`.
    Image {
        mark: usize,
        url: String,
        title: Option<String>,
        attr_suffix: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // Single shared tracking allocator for this module (`#[global_allocator]`
    // may be declared at most once for the whole test binary) — see
    // texinfo::writer's identically-shaped test module for the rationale.
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TrackingAlloc;
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    static CURRENT_BYTES: AtomicUsize = AtomicUsize::new(0);
    static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
    unsafe impl GlobalAlloc for TrackingAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            let cur = CURRENT_BYTES.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK_BYTES.fetch_max(cur, Ordering::Relaxed);
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            CURRENT_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
            unsafe { System.dealloc(ptr, layout) }
        }
    }
    #[global_allocator]
    static GLOBAL: TrackingAlloc = TrackingAlloc;

    /// Serializes the two tests below that share the module-level
    /// `ALLOCS`/`CURRENT_BYTES`/`PEAK_BYTES` atomics, since `cargo test`
    /// runs tests concurrently by default.
    static ALLOC_TRACKING_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Regression guard: an incremental writer must not reintroduce
    /// per-block subtree reconstruction. Allocation count for feeding N
    /// events through `Writer` must stay near-linear in N.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        let _guard = ALLOC_TRACKING_LOCK.lock().unwrap();

        fn events_for(n: usize) -> Vec<Event<'static>> {
            use std::borrow::Cow;
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(Event::StartHeading {
                    level: 2,
                    id: None,
                    classes: vec![],
                    kv: vec![],
                });
                evs.push(Event::Text(Cow::Owned(format!("Section {i}"))));
                evs.push(Event::EndHeading);
                evs.push(Event::StartParagraph {
                    id: None,
                    classes: vec![],
                    kv: vec![],
                });
                evs.push(Event::Text(Cow::Owned("plain ".to_string())));
                evs.push(Event::StartStrong {
                    id: None,
                    classes: vec![],
                    kv: vec![],
                });
                evs.push(Event::Text(Cow::Owned("bold".to_string())));
                evs.push(Event::EndStrong);
                evs.push(Event::EndParagraph);
                evs.push(Event::StartList {
                    kind: ListKind::Bullet(BulletStyle::Dash),
                    tight: true,
                    id: None,
                    classes: vec![],
                    kv: vec![],
                });
                for j in 0..2 {
                    evs.push(Event::StartListItem { checked: None });
                    evs.push(Event::Text(Cow::Owned(format!("item {j}"))));
                    evs.push(Event::EndListItem);
                }
                evs.push(Event::EndList);
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
        let large = run(2000);

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 20.0,
            "allocation count did not scale near-linearly: {small} allocs @200 sections -> \
             {large} allocs @2000 sections (ratio {ratio:.2}); this suggests reintroduced \
             per-block subtree reconstruction"
        );
    }

    /// Reports peak memory and throughput for the streaming `Writer` versus
    /// the tree-based `parse()` + `emit()` baseline, on a large synthetic
    /// document (many top-level sections). `#[ignore]`d because it prints
    /// rather than asserts a threshold — run with `cargo test -p djot-fmt \
    /// --release test_writer_peak_memory_and_throughput_report -- --ignored \
    /// --nocapture` to see the numbers.
    #[test]
    #[ignore]
    fn test_writer_peak_memory_and_throughput_report() {
        let _guard = ALLOC_TRACKING_LOCK.lock().unwrap();

        /// Discards written bytes instead of retaining them, so peak memory
        /// reflects the Writer's own internal state, not a `Vec<u8>` sink
        /// re-accumulating the whole document (the harness artifact this
        /// session's rst-fmt profiling and texinfo's first pass both hit).
        struct DiscardSink(usize);
        impl Write for DiscardSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0 += buf.len();
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        fn synthetic_source(n: usize) -> String {
            let mut s = String::new();
            for i in 0..n {
                s.push_str(&format!(
                    "## Section {i}\n\n\
                     Some plain text with **bold** and _italic_ markup, and a \
                     [link {i}](http://example.com/{i}).\n\n\
                     - first point {i}\n- second point {i}\n\n"
                ));
            }
            s
        }

        let input = synthetic_source(5000);

        // events()/parse() run outside the tracked window — see texinfo's
        // identically-shaped benchmark for why (attributing a non-Writer
        // API's own materialization cost to the Writer would misrepresent
        // it, same as feeding events() lazily-but-not-really would).
        let events: Vec<Event<'static>> = crate::events(&input).map(Event::into_owned).collect();
        let (doc, _diags) = crate::parse::parse(&input);

        let baseline = CURRENT_BYTES.load(Ordering::Relaxed);
        PEAK_BYTES.store(baseline, Ordering::Relaxed);
        let start = std::time::Instant::now();
        let sink = DiscardSink(0);
        let bytes_written = {
            let mut w = Writer::new(sink);
            for e in events {
                w.write_event(e);
            }
            w.finish().0
        };
        let streaming_elapsed = start.elapsed();
        let streaming_peak = PEAK_BYTES.load(Ordering::Relaxed).saturating_sub(baseline);
        std::hint::black_box(bytes_written);

        let baseline = CURRENT_BYTES.load(Ordering::Relaxed);
        PEAK_BYTES.store(baseline, Ordering::Relaxed);
        let start = std::time::Instant::now();
        let built = crate::emit::emit(std::hint::black_box(&doc));
        let builder_elapsed = start.elapsed();
        let builder_peak = PEAK_BYTES.load(Ordering::Relaxed).saturating_sub(baseline);
        std::hint::black_box(&built);

        eprintln!(
            "djot streaming Writer vs parse()+emit() builder, {} bytes input, 5000 sections:\n\
             \x20 streaming: {:>10} peak bytes, {:>10?}\n\
             \x20 builder:   {:>10} peak bytes, {:>10?}\n\
             \x20 peak ratio (builder/streaming): {:.2}x\n\
             \x20 throughput ratio (streaming/builder): {:.2}x",
            input.len(),
            streaming_peak,
            streaming_elapsed,
            builder_peak,
            builder_elapsed,
            builder_peak as f64 / streaming_peak.max(1) as f64,
            builder_elapsed.as_secs_f64() / streaming_elapsed.as_secs_f64().max(1e-12),
        );
    }

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartHeading {
            level: 1,
            id: None,
            classes: vec![],
            kv: vec![],
        });
        w.write_event(Event::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(Event::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("# Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartParagraph {
            id: None,
            classes: vec![],
            kv: vec![],
        });
        w.write_event(Event::Text(std::borrow::Cow::Owned("World".to_string())));
        w.write_event(Event::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "# Hello\n\nA paragraph with *strong* text.\n\n- item one\n- item two\n";
        let (doc, _) = crate::parse::parse(input);
        let evts: Vec<_> = crate::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc2, _) = crate::parse::parse(&emitted_text);
        assert_eq!(
            doc.strip_spans(),
            doc2.strip_spans(),
            "writer roundtrip mismatch"
        );
    }

    #[test]
    fn test_writer_table_caption_roundtrip() {
        let input = "^ Caption text\n| A | B |\n|---|---|\n| x | y |\n";
        let (doc, _) = crate::parse::parse(input);

        match &doc.blocks[0] {
            crate::ast::Block::Table {
                caption: Some(_), ..
            } => {}
            other => panic!(
                "Expected Table with caption after direct parse, got {:?}",
                other
            ),
        }

        let evts: Vec<_> = crate::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc2, _) = crate::parse::parse(&emitted_text);

        assert_eq!(
            doc.strip_spans(),
            doc2.strip_spans(),
            "table caption lost in event path; emitted: {emitted_text:?}"
        );
    }

    /// The streaming `Writer` must produce byte-identical output to the
    /// tree-based `emit()` for the same document.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "# Hello\n\nSome *bold* and _em_ and `code`.\n",
            "## Sub\n\ntext with a [link](http://x/) here.\n",
            "- one\n- two\n- three\n",
            "1. first\n2. second\n",
            "```rust\nfn main() {}\n```\n",
            "> A quote.\n>\n> Second para.\n",
            ": term\n  definition body\n\n: term2\n  another\n",
            "^ Caption\n| A | B |\n|---|---|\n| x | y |\n",
            "See[^1] for details.\n\n[^1]: Footnote body text.\n",
            "---\n\nAfter the break.\n",
            "![alt](img.png)\n",
            ":::note\nDiv body.\n:::\n",
            "[label]: http://example.com \"a title\"\n\nSee [label][].\n",
            "text with {-del-} and {+ins+} and {=hl=} and ~sub~ and ^sup^.\n",
            "A [span]{.cls} here.\n",
            "$x^2$ and $$y = mx + b$$\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input);
            let built = crate::emit::emit(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events(input) {
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
}
