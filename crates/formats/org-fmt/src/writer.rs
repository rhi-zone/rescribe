#![allow(clippy::collapsible_if)]
//! Streaming Org-mode writer — converts a stream of events directly to Org
//! text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`]. It is a second, independent
//! emission path from the tree-based `build()` function, not a thin wrapper
//! around it.
//!
//! # Construct classification
//!
//! Most constructs are write-straight-through into a single shared
//! `out: String` buffer. `emit.rs` itself already operates on one flat
//! `BuildContext::output` for the whole document (no per-construct
//! sub-buffers except `Table`'s cell-width measurement — see below), using
//! two idempotent tail operations everywhere it needs whitespace control:
//! [`Writer::ensure_newline`] (`if !out.is_empty() && !out.ends_with('\n')
//! { out.push('\n') }`) and [`Writer::ensure_blank_line`] (trim all trailing
//! whitespace, then push exactly `"\n\n"`). Both translate directly to
//! operating on `Writer::out`'s own tail — calling them at the *same
//! structural points* `build_block`/`build_list_item`/`build_table` do
//! reproduces `emit.rs`'s output exactly, since both implementations are
//! single-buffer sequential-mutation models over the same shape of tree.
//!
//! **Invalid-context handling differs from `rst-fmt`/`djot-fmt`/`t2t`'s
//! write-then-truncate-on-close pattern.** Those write speculatively and
//! truncate a `mark..` span if the parent turns out invalid. Here,
//! `ensure_blank_line`'s `trim_end()` operates on the *whole* buffer tail,
//! not a scoped span — truncating back to a mark after it ran could
//! incorrectly eat trailing whitespace that belongs to unrelated, already-
//! valid content before the mark. So invalid contexts are handled by
//! checking `accepts_blocks()`/`accepts_inline()` **before** writing
//! anything, pushing a `Frame::Discard` instead of the real frame when
//! invalid — nothing is ever written for a discarded construct or its
//! descendants (`accepts_blocks`/`accepts_inline` treat `Discard` as
//! non-accepting too, so invalidity cascades to children automatically).
//!
//! `List`/`ListItem` need `Writer::list_depth` (mirroring
//! `BuildContext::list_depth`, incremented/decremented around `List`) for
//! the `"  ".repeat(depth)` indent prefix, and each `List` frame tracks a
//! running item counter for ordered-list numbering — both write-through,
//! no buffering. `ListItem`'s children need position/type-dependent
//! dispatch (mirroring `build_list_item`'s `first`/`Block::Paragraph`/
//! `Block::List`/other match): decided at each child's own `Start`/`End`
//! by inspecting the `ListItem` frame, the same "known at open, applied at
//! close" shape used elsewhere. A bare inline run directly under a
//! `ListItem` (no `StartParagraph` wrapper — `events()` emits this for
//! `ListItemContent::Inline`, e.g. simple one-line items) is tracked via an
//! `in_inline_run` flag on the frame, opened by the first inline-producing
//! event and closed by the next block-level event or `EndListItem`.
//!
//! `Table` is the one genuinely content-dependent-prefix construct: column
//! widths depend on every cell's *trimmed, fully-formatted* markup length,
//! unknowable until the whole table is seen. Cells are captured under a
//! mark exactly like `rst-fmt`'s heading-plain-text technique (written
//! straight into `out`, sliced to an owned, trimmed `String`, truncated
//! back), then [`render_table`] reproduces `build_table`'s column-width and
//! header-separator logic at `EndTable`.
//!
//! **Document metadata (`Event::Metadata`) is a documented, deliberate
//! divergence from `build()`'s exact semantics, not an oversight.**
//! `build()` always collects *all* `OrgDoc.metadata` (wherever in the
//! source it appeared — `parse_next_block` can pick up a `#+KEY: value`
//! line at any point, per its own doc comment) and re-emits every entry at
//! the very top of the document, ahead of all blocks. Reproducing that
//! exactly in a genuinely incremental writer would require buffering
//! arbitrarily far ahead (metadata could appear immediately before the
//! *last* block), which defeats the entire point of streaming. This
//! `Writer` instead emits each `Metadata` event's line immediately,
//! write-through, wherever it arrives in the event stream, with the
//! single-blank-line-before-the-next-block rule applied once metadata
//! stops. Checked against every org fixture (`fixtures/org/*/input.org`) —
//! none currently has a generic `#+KEY:` metadata line after body content
//! has started (the two false positives found while auditing this,
//! `dynamic-block`'s `#+BEGIN:` and `figure`'s `#+CAPTION:`/`#+NAME:`, both
//! go through dedicated non-metadata code paths in `parse.rs`) — so this
//! divergence is currently unobservable in the byte-identical-to-builder
//! check, but a hand-fed event stream that put `Event::Metadata` after a
//! block would produce output ordered differently than `build()`'s
//! move-everything-to-the-top reconstruction. Documented here rather than
//! silently assumed away.
//!
//! Each top-level document child is flushed to the sink and `out` is
//! cleared (keeping its capacity) as soon as the frame stack empties.
//! `build()`'s final "ensure the whole document ends with exactly one
//! newline" step is reproduced at [`finish`](Writer::finish) via a
//! `ends_with_newline` flag updated on every flush (since by then the
//! relevant bytes may already have left `out`).
//!
//! # Example
//! ```no_run
//! use org_fmt::writer::Writer;
//! use org_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, todo: None, priority: None, tags: vec![], properties: vec![], scheduled: None, deadline: None });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::CheckboxState;
use crate::events::Event;
use std::io::Write;

/// Streaming Org-mode writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// document child is flushed to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to ensure the trailing newline `build()`
/// always adds, flush the remainder, and recover the sink.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Cleared (capacity retained) after
    /// each top-level document child is flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level.
    stack: Vec<Frame>,
    /// Mirrors `BuildContext::list_depth`.
    list_depth: usize,
    /// Whether a `Metadata` line was just written and no blank-line
    /// separator has been inserted before the next (non-`Metadata`) event
    /// yet — see the module doc's metadata section.
    metadata_pending_blank: bool,
    /// Whether the last byte written to the sink (across all flushes) was
    /// `'\n'` — since `out` is cleared on each flush, `finish()` can't just
    /// inspect `out` to decide whether `build()`'s trailing-newline
    /// guarantee is already satisfied.
    ends_with_newline: bool,
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
            stack: Vec::new(),
            list_depth: 0,
            metadata_pending_blank: false,
            ends_with_newline: false,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level document child.
    pub fn write_event(&mut self, event: Event<'_>) {
        if self.metadata_pending_blank && !matches!(event, Event::Metadata { .. }) {
            self.out.push('\n');
            self.metadata_pending_blank = false;
        }
        self.process(event);
        self.maybe_flush();
    }

    /// Ensure the document ends with exactly one trailing newline (matching
    /// `build()`'s unconditional final check, which fires even for a fully
    /// empty document), flush any remainder, and recover the sink.
    pub fn finish(mut self) -> W {
        if !self.ends_with_newline {
            self.out.push('\n');
        }
        self.flush();
        self.sink
    }

    // ── Buffer primitives ─────────────────────────────────────────────────

    fn flush(&mut self) {
        if !self.out.is_empty() {
            self.ends_with_newline = self.out.ends_with('\n');
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    fn maybe_flush(&mut self) {
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Mirrors `BuildContext::ensure_newline`.
    fn ensure_newline(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
    }

    /// Mirrors `BuildContext::ensure_blank_line`.
    fn ensure_blank_line(&mut self) {
        let trimmed_len = self.out.trim_end().len();
        self.out.truncate(trimmed_len);
        self.out.push_str("\n\n");
    }

    /// Whether the top-of-stack frame accepts block children (mirrors the
    /// old `DocBuilder::push_block`'s match: `Document`/`Blockquote`/
    /// `Figure`/`ListItem` — `Discard` cascades non-acceptance to children).
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote | Frame::Figure { .. } | Frame::ListItem { .. })
        )
    }

    /// Whether the top-of-stack frame accepts inline children.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph
                    | Frame::Heading { .. }
                    | Frame::TableCell { .. }
                    | Frame::DefinitionTerm
                    | Frame::DefinitionDesc
                    | Frame::Div
                    | Frame::Caption
                    | Frame::BlockFootnoteDef
                    | Frame::Inline { .. }
                    | Frame::Link
                    | Frame::FootnoteDefinition
                    | Frame::ListItem { .. }
            )
        )
    }

    fn open_inline_span(&mut self, open: &str, close: &'static str) {
        if !self.accepts_inline() {
            self.stack.push(Frame::Discard);
            return;
        }
        self.note_list_item_inline_run();
        self.out.push_str(open);
        self.stack.push(Frame::Inline { close });
    }

    fn close_inline_span(&mut self) {
        match self.stack.pop() {
            Some(Frame::Inline { close }) => self.out.push_str(close),
            Some(Frame::Discard) | None => {}
            Some(other) => self.stack.push(other),
        }
    }

    /// If the top-of-stack frame is a `ListItem` not currently in a bare
    /// inline run, open one (writing no indent — `ListItemContent::Inline`
    /// never gets `build_list_item`'s content-indent treatment, only
    /// `Block::Paragraph` children do). No-op otherwise.
    fn note_list_item_inline_run(&mut self) {
        if let Some(Frame::ListItem {
            in_inline_run,
            first_child,
            ..
        }) = self.stack.last_mut()
        {
            if !*in_inline_run {
                *in_inline_run = true;
                *first_child = false;
            }
        }
    }

    /// Close a bare inline run under a `ListItem`, if one is open — mirrors
    /// flushing `current_inlines` in the old AST reconstruction. Called
    /// whenever a block-level event or `EndListItem` arrives.
    fn close_list_item_inline_run(&mut self) {
        if let Some(Frame::ListItem { in_inline_run, .. }) = self.stack.last_mut() {
            if *in_inline_run {
                *in_inline_run = false;
                self.ensure_newline();
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event<'_>) {
        match event {
            Event::Metadata { key, value } => {
                self.out.push_str("#+");
                self.out.push_str(&key.to_uppercase());
                self.out.push_str(": ");
                self.out.push_str(&value);
                self.out.push('\n');
                self.metadata_pending_blank = true;
                self.maybe_flush();
            }

            // ── Block open/close ────────────────────────────────────────
            Event::StartParagraph => {
                if !self.accepts_blocks() {
                    self.stack.push(Frame::Discard);
                    return;
                }
                self.close_list_item_inline_run();
                let list_item_pos = match self.stack.last() {
                    Some(Frame::ListItem { first_child, .. }) => Some(*first_child),
                    _ => None,
                };
                if let Some(false) = list_item_pos {
                    let indent = "  ".repeat(self.list_depth);
                    self.out.push_str(&indent);
                }
                self.stack.push(Frame::Paragraph);
            }
            Event::EndParagraph => {
                match self.stack.pop() {
                    Some(Frame::Paragraph) => {}
                    _ => return,
                }
                if let Some(Frame::ListItem { first_child, .. }) = self.stack.last_mut() {
                    // Matches build_list_item's Paragraph-child arms: a
                    // single ensure_newline, never a blank line.
                    *first_child = false;
                    self.ensure_newline();
                } else {
                    // Every other valid container (top-level, Blockquote,
                    // Figure): Paragraph's own build_block arm always ends
                    // with a full blank line.
                    self.ensure_blank_line();
                }
                self.maybe_flush();
            }
            Event::StartHeading {
                level,
                todo,
                priority,
                tags,
                properties,
                scheduled,
                deadline,
            } => {
                if !self.accepts_blocks() {
                    self.stack.push(Frame::Discard);
                    return;
                }
                self.ensure_newline();
                for _ in 0..level {
                    self.out.push('*');
                }
                self.out.push(' ');
                if let Some(kw) = &todo {
                    self.out.push_str(kw);
                    self.out.push(' ');
                }
                if let Some(p) = &priority {
                    self.out.push_str("[#");
                    self.out.push_str(p);
                    self.out.push_str("] ");
                }
                self.stack.push(Frame::Heading {
                    tags,
                    properties,
                    scheduled,
                    deadline,
                });
            }
            Event::EndHeading => {
                let Some(Frame::Heading {
                    tags,
                    properties,
                    scheduled,
                    deadline,
                }) = self.stack.pop()
                else {
                    return;
                };
                if !tags.is_empty() {
                    self.out.push_str("    :");
                    self.out.push_str(&tags.join(":"));
                    self.out.push(':');
                }
                self.ensure_newline();
                if !properties.is_empty() {
                    self.out.push_str(":PROPERTIES:\n");
                    for (k, v) in &properties {
                        self.out.push(':');
                        self.out.push_str(k);
                        self.out.push_str(": ");
                        self.out.push_str(v);
                        self.out.push('\n');
                    }
                    self.out.push_str(":END:\n");
                }
                if scheduled.is_some() || deadline.is_some() {
                    if let Some(s) = &scheduled {
                        self.out.push_str("SCHEDULED: ");
                        self.out.push_str(s);
                        if deadline.is_some() {
                            self.out.push(' ');
                        } else {
                            self.out.push('\n');
                        }
                    }
                    if let Some(d) = &deadline {
                        self.out.push_str("DEADLINE: ");
                        self.out.push_str(d);
                        self.out.push('\n');
                    }
                }
                self.ensure_blank_line();
                self.maybe_flush();
            }
            Event::StartBlockquote => {
                if !self.accepts_blocks() {
                    self.stack.push(Frame::Discard);
                    return;
                }
                self.close_list_item_inline_run();
                self.ensure_newline();
                self.out.push_str("#+BEGIN_QUOTE\n");
                self.stack.push(Frame::Blockquote);
            }
            Event::EndBlockquote => {
                if !matches!(self.stack.pop(), Some(Frame::Blockquote)) {
                    return;
                }
                self.out.push_str("#+END_QUOTE\n\n");
                self.maybe_flush();
            }
            Event::StartList { ordered, start } => {
                if !self.accepts_blocks() {
                    self.stack.push(Frame::Discard);
                    return;
                }
                self.close_list_item_inline_run();
                self.list_depth += 1;
                self.stack.push(Frame::List {
                    ordered,
                    counter: start.map(|s| s as i32).unwrap_or(1),
                    first_item: true,
                    emit_start_cookie: start,
                });
            }
            Event::EndList => {
                if !matches!(self.stack.pop(), Some(Frame::List { .. })) {
                    return;
                }
                self.list_depth -= 1;
                if self.list_depth == 0 {
                    self.ensure_newline();
                }
                self.maybe_flush();
            }
            Event::StartListItem { checkbox } => {
                let in_list = matches!(self.stack.last(), Some(Frame::List { .. }));
                if !in_list {
                    self.stack.push(Frame::Discard);
                    return;
                }
                if let Some(Frame::List {
                    ordered,
                    counter,
                    first_item,
                    emit_start_cookie,
                }) = self.stack.last_mut()
                {
                    let cookie = if *ordered && *first_item {
                        *emit_start_cookie
                    } else {
                        None
                    };
                    let indent = "  ".repeat(self.list_depth - 1);
                    self.out.push_str(&indent);
                    if *ordered {
                        if let Some(start_n) = cookie {
                            self.out.push_str(&format!("{}. [@{}] ", counter, start_n));
                        } else {
                            self.out.push_str(&format!("{}. ", counter));
                        }
                        *counter += 1;
                    } else {
                        self.out.push_str("- ");
                    }
                    *first_item = false;
                }
                if let Some(cb) = checkbox {
                    self.out.push_str(match cb {
                        CheckboxState::Unchecked => "[ ] ",
                        CheckboxState::Checked => "[X] ",
                        CheckboxState::Partial => "[-] ",
                    });
                }
                self.stack.push(Frame::ListItem {
                    first_child: true,
                    in_inline_run: false,
                });
            }
            Event::EndListItem => {
                self.close_list_item_inline_run();
                self.stack.pop();
            }
            Event::CodeBlock {
                language,
                header_args,
                name,
                content,
            } => {
                if self.accepts_blocks() {
                    self.close_list_item_inline_run();
                    self.ensure_newline();
                    if let Some(nm) = &name {
                        self.out.push_str("#+NAME: ");
                        self.out.push_str(nm);
                        self.out.push('\n');
                    }
                    if let Some(lang) = &language {
                        self.out.push_str("#+BEGIN_SRC ");
                        self.out.push_str(lang);
                        if let Some(args) = &header_args {
                            self.out.push(' ');
                            self.out.push_str(args);
                        }
                        self.out.push('\n');
                    } else {
                        self.out.push_str("#+BEGIN_SRC\n");
                    }
                    self.out.push_str(&content);
                    if !content.ends_with('\n') {
                        self.out.push('\n');
                    }
                    self.out.push_str("#+END_SRC\n\n");
                    self.maybe_flush();
                }
            }
            Event::RawBlock { format, content } => {
                if self.accepts_blocks() {
                    self.close_list_item_inline_run();
                    if format == "org" {
                        self.out.push_str(&content);
                    }
                    self.maybe_flush();
                }
            }
            Event::HorizontalRule => {
                if self.accepts_blocks() {
                    self.close_list_item_inline_run();
                    self.ensure_newline();
                    self.out.push_str("-----\n\n");
                    self.maybe_flush();
                }
            }
            Event::StartTable => {
                if !self.accepts_blocks() {
                    self.stack.push(Frame::Discard);
                    return;
                }
                self.close_list_item_inline_run();
                self.stack.push(Frame::Table { rows: Vec::new() });
            }
            Event::EndTable => {
                let Some(Frame::Table { rows }) = self.stack.pop() else {
                    return;
                };
                self.ensure_newline();
                render_table(&rows, &mut self.out);
                self.maybe_flush();
            }
            Event::StartTableRow { is_header } => {
                if matches!(self.stack.last(), Some(Frame::Table { .. })) {
                    self.stack.push(Frame::TableRow {
                        cells: Vec::new(),
                        is_header,
                    });
                } else {
                    self.stack.push(Frame::Discard);
                }
            }
            Event::EndTableRow => {
                if let Some(Frame::TableRow { cells, is_header }) = self.stack.pop() {
                    if let Some(Frame::Table { rows }) = self.stack.last_mut() {
                        rows.push((cells, is_header));
                    }
                }
            }
            Event::StartTableCell => {
                if matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                    let mark = self.out.len();
                    self.stack.push(Frame::TableCell { mark });
                } else {
                    self.stack.push(Frame::Discard);
                }
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell { mark }) = self.stack.pop() {
                    let cell = self.out[mark..].trim().to_string();
                    self.out.truncate(mark);
                    if let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut() {
                        cells.push(cell);
                    }
                }
            }
            Event::StartDefinitionList => {
                if !self.accepts_blocks() {
                    self.stack.push(Frame::Discard);
                    return;
                }
                self.close_list_item_inline_run();
                self.stack.push(Frame::DefinitionList);
            }
            Event::EndDefinitionList => {
                if !matches!(self.stack.pop(), Some(Frame::DefinitionList)) {
                    return;
                }
                self.ensure_newline();
                self.maybe_flush();
            }
            Event::StartDefinitionTerm => {
                if matches!(self.stack.last(), Some(Frame::DefinitionList)) {
                    self.out.push_str("- ");
                    self.stack.push(Frame::DefinitionTerm);
                } else {
                    self.stack.push(Frame::Discard);
                }
            }
            Event::EndDefinitionTerm => {
                if matches!(self.stack.pop(), Some(Frame::DefinitionTerm)) {
                    self.out.push_str(" :: ");
                }
            }
            Event::StartDefinitionDesc => {
                if matches!(self.stack.last(), Some(Frame::DefinitionList)) {
                    self.stack.push(Frame::DefinitionDesc);
                } else {
                    self.stack.push(Frame::Discard);
                }
            }
            Event::EndDefinitionDesc => {
                if matches!(self.stack.pop(), Some(Frame::DefinitionDesc)) {
                    self.ensure_newline();
                }
            }
            Event::StartDiv => {
                if self.accepts_blocks() {
                    self.close_list_item_inline_run();
                    self.stack.push(Frame::Div);
                } else {
                    self.stack.push(Frame::Discard);
                }
            }
            Event::EndDiv => {
                if matches!(self.stack.pop(), Some(Frame::Div)) {
                    self.maybe_flush();
                }
            }
            Event::StartFigure { name } => {
                if !self.accepts_blocks() {
                    self.stack.push(Frame::Discard);
                    return;
                }
                self.close_list_item_inline_run();
                self.stack.push(Frame::Figure { name });
            }
            Event::EndFigure => {
                if matches!(self.stack.pop(), Some(Frame::Figure { .. })) {
                    self.maybe_flush();
                }
            }
            Event::StartCaption => {
                if self.accepts_blocks() {
                    self.out.push_str("#+CAPTION: ");
                    self.stack.push(Frame::Caption);
                } else {
                    self.stack.push(Frame::Discard);
                }
            }
            Event::EndCaption => {
                if matches!(self.stack.pop(), Some(Frame::Caption)) {
                    self.ensure_newline();
                    // Figure's #+NAME: line is injected right after its
                    // Caption child specifically — mirrors build_block's
                    // Block::Figure arm detecting the Caption child.
                    if let Some(Frame::Figure { name }) = self.stack.last() {
                        if let Some(nm) = name.clone() {
                            self.out.push_str("#+NAME: ");
                            self.out.push_str(&nm);
                            self.ensure_newline();
                        }
                    }
                }
            }
            Event::StartBlockFootnoteDef { label } => {
                if !self.accepts_blocks() {
                    self.stack.push(Frame::Discard);
                    return;
                }
                self.close_list_item_inline_run();
                self.ensure_newline();
                self.out.push_str("[fn:");
                self.out.push_str(&label);
                self.out.push_str("] ");
                self.stack.push(Frame::BlockFootnoteDef);
            }
            Event::EndBlockFootnoteDef => {
                if matches!(self.stack.pop(), Some(Frame::BlockFootnoteDef)) {
                    self.ensure_blank_line();
                    self.maybe_flush();
                }
            }
            Event::UnknownBlock { .. } => {
                // Matches build_block's Block::Unknown arm: silently no-op.
                self.close_list_item_inline_run();
            }

            // ── Inline events ───────────────────────────────────────────
            Event::Text(cow) => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push_str(&cow);
                }
            }
            Event::SoftBreak => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push('\n');
                }
            }
            Event::LineBreak => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push_str("\\\\\n");
                }
            }
            Event::StartBold => self.open_inline_span("*", "*"),
            Event::EndBold => self.close_inline_span(),
            Event::StartItalic => self.open_inline_span("/", "/"),
            Event::EndItalic => self.close_inline_span(),
            Event::StartUnderline => self.open_inline_span("_", "_"),
            Event::EndUnderline => self.close_inline_span(),
            Event::StartStrikethrough => self.open_inline_span("+", "+"),
            Event::EndStrikethrough => self.close_inline_span(),
            Event::StartSuperscript => self.open_inline_span("^{", "}"),
            Event::EndSuperscript => self.close_inline_span(),
            Event::StartSubscript => self.open_inline_span("_{", "}"),
            Event::EndSubscript => self.close_inline_span(),
            Event::InlineCode(cow) => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push('=');
                    self.out.push_str(&cow);
                    self.out.push('=');
                }
            }
            Event::StartLink { url } => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push_str("[[");
                    self.out.push_str(&url);
                    self.out.push_str("][");
                    self.stack.push(Frame::Link);
                } else {
                    self.stack.push(Frame::Discard);
                }
            }
            Event::EndLink => {
                if matches!(self.stack.pop(), Some(Frame::Link)) {
                    self.out.push_str("]]");
                }
            }
            Event::InlineImage { url } => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push_str("[[");
                    if !url.starts_with("file:") && !url.starts_with("http") {
                        self.out.push_str("file:");
                    }
                    self.out.push_str(&url);
                    self.out.push_str("]]");
                }
            }
            Event::FootnoteRef { label } => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push_str("[fn:");
                    self.out.push_str(&label);
                    self.out.push(']');
                }
            }
            Event::StartFootnoteDefinition { label } => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push_str("[fn:");
                    self.out.push_str(&label);
                    self.out.push_str(": ");
                    self.stack.push(Frame::FootnoteDefinition);
                } else {
                    self.stack.push(Frame::Discard);
                }
            }
            Event::EndFootnoteDefinition => {
                if matches!(self.stack.pop(), Some(Frame::FootnoteDefinition)) {
                    self.out.push(']');
                }
            }
            Event::MathInline { source } => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push('$');
                    self.out.push_str(&source);
                    self.out.push('$');
                }
            }
            Event::Timestamp { active, value } => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    if active {
                        self.out.push('<');
                        self.out.push_str(&value);
                        self.out.push('>');
                    } else {
                        self.out.push('[');
                        self.out.push_str(&value);
                        self.out.push(']');
                    }
                }
            }
            Event::ExportSnippet { backend, value } => {
                if self.accepts_inline() {
                    self.note_list_item_inline_run();
                    self.out.push_str("@@");
                    self.out.push_str(&backend);
                    self.out.push(':');
                    self.out.push_str(&value);
                    self.out.push_str("@@");
                }
            }
        }
    }
}

/// Mirrors `build_table`: computes column widths from every cell's already-
/// trimmed formatted-markup length, then renders each row with padding and
/// (after the first row, if there's a header and more than one row total) a
/// separator row.
fn render_table(rows: &[(Vec<String>, bool)], out: &mut String) {
    if rows.is_empty() {
        let trimmed_len = out.trim_end().len();
        out.truncate(trimmed_len);
        out.push_str("\n\n");
        return;
    }

    let num_cols = rows.iter().map(|(cells, _)| cells.len()).max().unwrap_or(0);
    let mut col_widths = vec![0usize; num_cols];
    for (cells, _) in rows {
        for (i, cell) in cells.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.len());
        }
    }

    let has_header = rows.iter().any(|(_, is_header)| *is_header);
    let total_rows = rows.len();

    for (idx, (cells, _is_header)) in rows.iter().enumerate() {
        out.push('|');
        for (i, cell) in cells.iter().enumerate() {
            out.push(' ');
            out.push_str(cell);
            let padding = col_widths[i].saturating_sub(cell.len());
            for _ in 0..padding {
                out.push(' ');
            }
            out.push_str(" |");
        }
        out.push('\n');

        if has_header && idx == 0 && total_rows > 1 {
            out.push('|');
            for width in &col_widths {
                out.push('-');
                for _ in 0..*width {
                    out.push('-');
                }
                out.push_str("-+");
            }
            if !col_widths.is_empty() {
                out.pop();
                out.push('|');
            }
            out.push('\n');
        }
    }

    let trimmed_len = out.trim_end().len();
    out.truncate(trimmed_len);
    out.push_str("\n\n");
}

/// Frames carry only tiny scalars/owned strings needed at `End*` time — no
/// accumulated tree content, except `Table`/`TableRow` (bounded by the
/// table's own size, collecting already-formatted, trimmed cell strings —
/// see the module doc).
enum Frame {
    Paragraph,
    Heading {
        tags: Vec<String>,
        properties: Vec<(String, String)>,
        scheduled: Option<String>,
        deadline: Option<String>,
    },
    Blockquote,
    List {
        ordered: bool,
        counter: i32,
        first_item: bool,
        emit_start_cookie: Option<u64>,
    },
    ListItem {
        /// Whether the next child is this item's first — decides
        /// content-indent for `Paragraph` children (mirrors
        /// `build_list_item`'s `first` flag) and whether a `Paragraph`
        /// gets it at all.
        first_child: bool,
        /// Whether a bare inline run (no `StartParagraph` wrapper) is
        /// currently open — mirrors `ListItemContent::Inline`.
        in_inline_run: bool,
    },
    Table {
        rows: Vec<(Vec<String>, bool)>,
    },
    TableRow {
        cells: Vec<String>,
        is_header: bool,
    },
    TableCell {
        mark: usize,
    },
    DefinitionList,
    DefinitionTerm,
    DefinitionDesc,
    Div,
    Figure {
        name: Option<String>,
    },
    Caption,
    BlockFootnoteDef,
    /// Any inline span whose closing text is a fixed string
    /// (bold/italic/underline/strikethrough/superscript/subscript).
    Inline {
        close: &'static str,
    },
    Link,
    FootnoteDefinition,
    /// Marker for a construct with no valid enclosing context — nothing is
    /// ever written for it or its descendants (see the module doc's
    /// "Invalid-context handling" section for why this differs from the
    /// write-then-truncate pattern used elsewhere).
    Discard,
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
                    todo: None,
                    priority: None,
                    tags: vec![],
                    properties: vec![],
                    scheduled: None,
                    deadline: None,
                });
                evs.push(Event::Text(Cow::Owned(format!("Section {i}"))));
                evs.push(Event::EndHeading);
                evs.push(Event::StartParagraph);
                evs.push(Event::Text(Cow::Owned("plain ".to_string())));
                evs.push(Event::StartBold);
                evs.push(Event::Text(Cow::Owned("bold".to_string())));
                evs.push(Event::EndBold);
                evs.push(Event::EndParagraph);
                evs.push(Event::StartList {
                    ordered: false,
                    start: None,
                });
                for j in 0..2 {
                    evs.push(Event::StartListItem { checkbox: None });
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
    /// the tree-based `parse()` + `build()` baseline, on a large synthetic
    /// document. `#[ignore]`d because it prints rather than asserts a
    /// threshold — run with `cargo test -p org-fmt --release \
    /// test_writer_peak_memory_and_throughput_report -- --ignored \
    /// --nocapture` to see the numbers.
    #[test]
    #[ignore]
    fn test_writer_peak_memory_and_throughput_report() {
        let _guard = ALLOC_TRACKING_LOCK.lock().unwrap();

        /// Discards written bytes instead of retaining them, so peak memory
        /// reflects the Writer's own internal state, not a `Vec<u8>` sink
        /// re-accumulating the whole document.
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
                    "** Section {i}\n\n\
                     Some plain text with *bold* and /italic/ markup, and a \
                     [[http://example.com/{i}][link {i}]].\n\n\
                     - first point {i}\n- second point {i}\n\n"
                ));
            }
            s
        }

        let input = synthetic_source(5000);

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
        let built = crate::emit::build(std::hint::black_box(&doc));
        let builder_elapsed = start.elapsed();
        let builder_peak = PEAK_BYTES.load(Ordering::Relaxed).saturating_sub(baseline);
        std::hint::black_box(&built);

        eprintln!(
            "org streaming Writer vs parse()+build() builder, {} bytes input, 5000 sections:\n\
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
            todo: None,
            priority: None,
            tags: vec![],
            properties: vec![],
            scheduled: None,
            deadline: None,
        });
        w.write_event(Event::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(Event::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("* Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text(std::borrow::Cow::Owned("World".to_string())));
        w.write_event(Event::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "* Hello\n\nA paragraph with *bold* text.\n\n- item one\n- item two\n";
        let evts: Vec<_> = crate::events(input).collect();
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

    /// The streaming `Writer` must produce byte-identical output to the
    /// tree-based `build()` for the same document.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "* Hello\n\nSome *bold* and /italic/ and =code=.\n",
            "** TODO [#A] Sub :tag1:tag2:\nSCHEDULED: <2024-01-01>\n\nBody text.\n",
            "- one\n- two\n- three\n",
            "1. first\n2. second\n",
            "#+BEGIN_SRC rust\nfn main() {}\n#+END_SRC\n",
            "#+BEGIN_QUOTE\nA quote.\n\nSecond para.\n#+END_QUOTE\n",
            "- term :: definition\n- term2 :: another\n",
            "| A | B |\n|---|---|\n| x | y |\n",
            "See [fn:1] for details.\n\n[fn:1] Footnote body text.\n",
            "-----\n\nAfter the rule.\n",
            "#+TITLE: My Doc\n#+AUTHOR: Someone\n\n* Heading\n",
            "[[http://x/][a link]]\n",
            "- outer\n\n  - nested a\n  - nested b\n",
            "- [ ] todo item\n- [X] done item\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input);
            let built = crate::emit::build(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events(input) {
                w.write_event(e.into_owned());
            }
            let streamed = String::from_utf8(w.finish()).unwrap();

            assert_eq!(
                built, streamed,
                "streaming Writer diverged from build() for input:\n{input}\n\
                 build():\n{built:?}\nWriter:\n{streamed:?}"
            );
        }
    }
}
