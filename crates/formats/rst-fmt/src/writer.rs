#![allow(clippy::collapsible_if)]
//! Streaming RST writer — converts a stream of events directly to RST text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::Block`]/[`crate::Inline`] value and
//! never calls [`crate::build_block`]/`build_inlines`/`build_inline`. It is a
//! second, independent emission path from the tree-based `build()`/`emit()`
//! functions, not a thin wrapper around them.
//!
//! # Buffer model
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document, mirroring `BuildContext::output`'s single amortized
//! geometric growth. Frames on the `Vec<Frame>` stack (`O(nesting depth)`)
//! hold only small metadata — a `usize` mark into `out`, a `&'static str`
//! closing delimiter, a bool — never a copy of accumulated content. Children
//! write **straight through** into `out`; a frame that needs to decorate its
//! own content afterwards post-processes the `out[mark..]` range in place.
//! Constructs split into three classes:
//!
//! - **Write-through** (zero per-frame buffering, prefix known at `Start*`):
//!   `Paragraph`, `List`, `ListItem`, `Div`, `DefinitionList`/`Term`/`Desc`,
//!   `FootnoteDef`, `Image`/`HorizontalRule`/`MathDisplay`/`RawBlock`, and
//!   every inline span (`Emphasis`, `Strong`, `Link`, `RstSpan`, …). Even
//!   `build_list_item`'s per-child dispatch is write-through: the child's kind
//!   and the first/subsequent flag are both known when the *child* opens, so
//!   its continuation indent is emitted then rather than reconstructed later.
//! - **Write-through + one in-place insert** (prefix length depends on
//!   content, but content is contiguous at the end of `out` when it becomes
//!   known): `Heading` (underline width = plain-text byte length) and
//!   `Figure` (the `"\n   "` caption lead-in is emitted only if the caption
//!   turned out non-empty).
//! - **Deferred per-line transform** (every line of already-written content
//!   must be re-indented): `Blockquote`, `Admonition`, `CodeBlock`. These
//!   post-process `out[mark..]` through a *pooled, reused* scratch buffer
//!   (`Writer::scratch`), so the pool holds at most `O(nesting depth)`
//!   buffers for the whole document rather than one fresh allocation per
//!   construct.
//!
//! `Table` is the one genuinely content-dependent-prefix construct: column
//! widths are unknown until every cell of every row has been seen, so cells
//! are collected as owned strings and rendered by [`render_table`] at
//! `EndTable`. That collection is bounded by the table's own size, which is
//! already inside the documented `O(largest top-level block)` limit.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity, so the buffer is allocated once for the whole document) as
//! soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use rst_fmt::writer::Writer;
//! use rst_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedEvent;
use crate::{calculate_column_widths, emit_table_border};
use std::io::Write;

/// Which arm of `build_list_item`'s per-child dispatch a block takes when its
/// parent frame is a `ListItem`. Known at the child's `Start*` event, which is
/// what makes list items write-through.
#[derive(Clone, Copy)]
enum BlockKind {
    Paragraph,
    List,
    Other,
}

/// Streaming RST writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to recover the sink once all events have been
/// fed.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Every construct writes here directly;
    /// frames record marks into it. Cleared (capacity retained) after each
    /// top-level block is flushed.
    out: String,
    /// The single shared plain-text buffer, mirroring
    /// `collect_text_from_inlines` (delimiters never contribute). Appended to
    /// only inside a `Heading` or `TableCell` (the only two constructs that
    /// read plain text back out); a frame records a mark and truncates back to
    /// it when it closes. Nesting needs no per-frame copy because nested
    /// spans contribute nothing but their children's text.
    plain: String,
    /// Reusable scratch for runs of repeated underline/overline characters.
    rule: String,
    /// Pool of scratch buffers for the deferred per-line re-indent constructs
    /// (`Blockquote`, `Admonition`, `CodeBlock`). Buffers are returned after
    /// use, so at most `O(nesting depth)` are ever allocated for a whole
    /// document instead of one per construct.
    scratch: Vec<String>,
    /// Frame stack for the block/inline construct currently being assembled.
    /// Empty at top level — a block closing with an empty stack is flushed to
    /// the sink immediately.
    stack: Vec<Frame>,
    /// Mirrors `BuildContext::list_depth`: incremented when a `List` frame is
    /// pushed (`StartList`), decremented when it is popped (`EndList`) — the
    /// same bracketing `build_list` uses around its item loop.
    list_depth: usize,
    /// Count of currently-open `Heading` frames — one of the two contexts in
    /// which `plain` is tracked at all (a heading needs its own plain-text
    /// byte length to size the underline). Ordinary markup nested only inside
    /// paragraphs/list items never pays for a plain-text copy nothing reads.
    heading_depth: usize,
    /// Count of currently-open `TableCell` frames — the other `plain` context.
    /// `build_table` never writes formatted markup into a cell, so while this
    /// is non-zero *all* writes to `out` are suppressed and only `plain`
    /// accumulates.
    table_cell_depth: usize,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            out: String::new(),
            plain: String::new(),
            rule: String::new(),
            scratch: Vec::new(),
            stack: Vec::new(),
            list_depth: 0,
            heading_depth: 0,
            table_cell_depth: 0,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: OwnedEvent) {
        self.process(event);
    }

    /// Recover the underlying sink. Does not write anything — every
    /// completed top-level block was already flushed by `write_event`.
    pub fn finish(self) -> W {
        self.sink
    }

    // ── Buffer primitives ─────────────────────────────────────────────────

    /// Append to the shared output buffer. Suppressed inside a table cell,
    /// where only plain text is meaningful (`build_table` writes no markup
    /// into cells).
    fn push_out(&mut self, s: &str) {
        if self.table_cell_depth == 0 {
            self.out.push_str(s);
        }
    }

    /// Flush the completed top-level block to the sink and reset the buffer,
    /// keeping its capacity so the document only ever grows one buffer.
    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Whether the top-of-stack frame accepts block children. Anything else
    /// (a heading, a table row, an inline span, …) discards them — the same
    /// "unexpected context" behaviour the buffer-per-frame design got by
    /// dropping the child's buffer, achieved here by truncating back to the
    /// child's mark.
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(
                Frame::Blockquote { .. }
                    | Frame::Div { .. }
                    | Frame::Admonition { .. }
                    | Frame::ListItem { .. }
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
                    | Frame::DefinitionDesc { .. }
                    | Frame::FootnoteDef { .. }
                    | Frame::Figure { .. }
                    | Frame::TableCell { .. }
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
            )
        )
    }

    /// Only `Heading` and `TableCell` ever read `plain` back out.
    fn need_plain(&self) -> bool {
        self.heading_depth > 0 || self.table_cell_depth > 0
    }

    fn write_indent(&mut self) {
        for _ in 0..self.list_depth {
            self.push_out("   ");
        }
    }

    /// Open a block: emit the list-item continuation prefix if this block is a
    /// child of a `ListItem` (replicating `build_list_item`'s dispatch at the
    /// child's *start*, where its kind and position are already known), and
    /// return the mark to truncate back to if it turns out this block had no
    /// valid enclosing context.
    fn block_start(&mut self, kind: BlockKind) -> usize {
        let mark = self.out.len();
        let was_first = match self.stack.last_mut() {
            Some(Frame::ListItem { first, .. }) => Some(std::mem::replace(first, false)),
            _ => None,
        };
        if let Some(was_first) = was_first {
            match kind {
                BlockKind::Paragraph => {
                    if !was_first {
                        self.write_indent();
                        self.push_out("   ");
                    }
                }
                BlockKind::List => {
                    self.push_out("\n");
                    self.write_indent();
                }
                BlockKind::Other => {}
            }
        }
        mark
    }

    /// Close a block: discard it if the enclosing frame does not take block
    /// children, otherwise flush if it completed a top-level block.
    fn block_end(&mut self, mark: usize) {
        if !self.accepts_blocks() {
            self.out.truncate(mark);
            return;
        }
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Close an inline span: discard it if the enclosing frame does not take
    /// inline children.
    fn inline_end(&mut self, mark: usize, plain_mark: usize) {
        if !self.accepts_inline() {
            self.out.truncate(mark);
            self.plain.truncate(plain_mark);
        }
    }

    /// Re-indent every line of `out[mark..]` by three spaces, in place,
    /// replicating `build_blockquote`/`build_admonition`/`build_code_block`'s
    /// `for line in inner.lines() { "   " + line + "\n" }` exactly (including
    /// its normalisation of the final newline). Uses a pooled scratch buffer
    /// rather than a fresh allocation per construct.
    fn reindent(&mut self, mark: usize) {
        let mut buf = self.scratch.pop().unwrap_or_default();
        buf.clear();
        for line in self.out[mark..].lines() {
            buf.push_str("   ");
            buf.push_str(line);
            buf.push('\n');
        }
        self.out.truncate(mark);
        self.out.push_str(&buf);
        self.scratch.push(buf);
    }

    // ── Inline contribution ───────────────────────────────────────────────

    /// Dispatch a leaf inline's formatted + plain contribution.
    fn push_inline(&mut self, formatted: &str, plain: &str) {
        self.push_multi(&[formatted], plain);
    }

    /// Append each of `bufs` in order to the shared output buffer, plus
    /// `plain` to the shared plain-text buffer when we are in a context that
    /// reads plain text back out.
    fn push_multi(&mut self, bufs: &[&str], plain: &str) {
        if !self.accepts_inline() {
            return;
        }
        if self.table_cell_depth == 0 {
            for b in bufs {
                self.out.push_str(b);
            }
        }
        if self.need_plain() {
            self.plain.push_str(plain);
        }
    }

    /// Append `prefix + body + suffix` — shorthand for the common
    /// three-piece wrap (e.g. `"*"` + emphasis body + `"*"`).
    fn push_wrapped(&mut self, prefix: &str, body: &str, suffix: &str, plain: &str) {
        self.push_multi(&[prefix, body, suffix], plain);
    }

    /// Open an inline span whose opening delimiter is already known: write it
    /// straight through and record the marks needed to undo it if the span
    /// turns out to have no valid enclosing context.
    fn open_span(&mut self, open: &str, close: &'static str) {
        let mark = self.out.len();
        let plain_mark = self.plain.len();
        if self.accepts_inline() {
            self.push_out(open);
        }
        self.stack.push(Frame::Inline {
            close,
            mark,
            plain_mark,
        });
    }

    fn close_span(&mut self) {
        if let Some(Frame::Inline {
            close,
            mark,
            plain_mark,
        }) = self.stack.pop()
        {
            self.push_out(close);
            self.inline_end(mark, plain_mark);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedEvent) {
        match event {
            // ── Block open/close ───────────────────────────────────────────────
            OwnedEvent::StartParagraph => {
                let mark = self.block_start(BlockKind::Paragraph);
                self.stack.push(Frame::Paragraph { mark });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    // `build_list_item` overrides paragraph formatting for
                    // list-item children (single "\n", no blank line);
                    // everywhere else `build_block` writes "\n\n".
                    if matches!(self.stack.last(), Some(Frame::ListItem { .. })) {
                        self.push_out("\n");
                    } else {
                        self.push_out("\n\n");
                    }
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartHeading { level } => {
                let mark = self.block_start(BlockKind::Other);
                let content = self.out.len();
                let plain_mark = self.plain.len();
                self.heading_depth += 1;
                self.stack.push(Frame::Heading {
                    level,
                    mark,
                    content,
                    plain_mark,
                });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading {
                    level,
                    mark,
                    content,
                    plain_mark,
                }) = self.stack.pop()
                {
                    self.heading_depth -= 1;
                    // Underline width comes from the *plain* text's byte
                    // length (matching `collect_text_from_inlines`), never the
                    // formatted text's — which would wrongly count delimiters.
                    let width = self.plain.len() - plain_mark;
                    self.plain.truncate(plain_mark);
                    if self.table_cell_depth == 0 {
                        let underline_char = match level {
                            1 => '=',
                            2 => '-',
                            3 => '~',
                            4 => '^',
                            5 => '"',
                            _ => '\'',
                        };
                        self.rule.clear();
                        for _ in 0..width {
                            self.rule.push(underline_char);
                        }
                        self.out.push('\n');
                        self.out.push_str(&self.rule);
                        self.out.push_str("\n\n");
                        if level == 1 {
                            // Overline: same run plus its newline, inserted
                            // ahead of the heading text now that its width is
                            // known. The heading's text is the only thing
                            // after `content`, so this is a short memmove
                            // inside the shared buffer — not a second buffer.
                            self.rule.push('\n');
                            self.out.insert_str(content, &self.rule);
                        }
                    }
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartBlockquote => {
                let mark = self.block_start(BlockKind::Other);
                let content = self.out.len();
                self.stack.push(Frame::Blockquote { mark, content });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { mark, content }) = self.stack.pop() {
                    self.reindent(content);
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartList { ordered } => {
                let mark = self.block_start(BlockKind::List);
                self.list_depth += 1;
                self.stack.push(Frame::List { ordered, mark });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.list_depth -= 1;
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartListItem => {
                let ordered = matches!(self.stack.last(), Some(Frame::List { ordered: true, .. }));
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::List { .. })) {
                    self.push_out(if ordered { "#. " } else { "- " });
                }
                self.stack.push(Frame::ListItem { first: true, mark });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { mark, .. }) = self.stack.pop() {
                    if !matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::StartCodeBlock { language } => {
                let mark = self.block_start(BlockKind::Other);
                if let Some(lang) = &language {
                    self.push_out(".. code-block:: ");
                    self.push_out(lang);
                    self.push_out("\n\n");
                } else {
                    self.push_out("::\n\n");
                }
                let content = self.out.len();
                self.stack.push(Frame::CodeBlock { mark, content });
            }
            OwnedEvent::CodeBlockContent(content) => {
                if matches!(self.stack.last(), Some(Frame::CodeBlock { .. })) {
                    self.push_out(&content);
                }
            }
            OwnedEvent::EndCodeBlock => {
                if let Some(Frame::CodeBlock { mark, content }) = self.stack.pop() {
                    self.reindent(content);
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::RawBlock { format, content } => {
                let mark = self.block_start(BlockKind::Other);
                if format == "rst" {
                    self.push_out(&content);
                }
                self.block_end(mark);
            }
            OwnedEvent::StartDiv { .. } => {
                let mark = self.block_start(BlockKind::Other);
                self.stack.push(Frame::Div { mark });
            }
            OwnedEvent::EndDiv => {
                if let Some(Frame::Div { mark }) = self.stack.pop() {
                    self.block_end(mark);
                }
            }
            OwnedEvent::HorizontalRule => {
                let mark = self.block_start(BlockKind::Other);
                self.push_out("----\n\n");
                self.block_end(mark);
            }
            OwnedEvent::StartTable => {
                let mark = self.block_start(BlockKind::Other);
                self.stack.push(Frame::Table { mark, rows: vec![] });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { mark, rows }) = self.stack.pop() {
                    if self.table_cell_depth == 0 {
                        render_table(&rows, &mut self.out);
                    }
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartTableRow { is_header } => {
                self.stack.push(Frame::TableRow {
                    cells: vec![],
                    is_header,
                });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { cells, is_header }) = self.stack.pop() {
                    if let Some(Frame::Table { rows, .. }) = self.stack.last_mut() {
                        rows.push((cells, is_header));
                    }
                }
            }
            OwnedEvent::StartTableCell => {
                self.table_cell_depth += 1;
                let plain_mark = self.plain.len();
                self.stack.push(Frame::TableCell { plain_mark });
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell { plain_mark }) = self.stack.pop() {
                    self.table_cell_depth -= 1;
                    let cell = self.plain[plain_mark..].to_string();
                    self.plain.truncate(plain_mark);
                    if let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut() {
                        cells.push(cell);
                    }
                }
            }
            OwnedEvent::StartDefinitionList => {
                let mark = self.block_start(BlockKind::Other);
                self.stack.push(Frame::DefinitionList { mark });
            }
            OwnedEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionTerm { mark });
            }
            OwnedEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.push_out("\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                    self.push_out("   ");
                }
                self.stack.push(Frame::DefinitionDesc { mark });
            }
            OwnedEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.push_out("\n\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::StartFootnoteDef { label } => {
                let mark = self.block_start(BlockKind::Other);
                self.push_out(".. [");
                self.push_out(&label);
                self.push_out("] ");
                self.stack.push(Frame::FootnoteDef { mark });
            }
            OwnedEvent::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::MathDisplay { source } => {
                let mark = self.block_start(BlockKind::Other);
                self.push_out(".. math::\n\n   ");
                self.push_out(&source.replace('\n', "\n   "));
                self.push_out("\n\n");
                self.block_end(mark);
            }
            OwnedEvent::StartAdmonition { admonition_type } => {
                let mark = self.block_start(BlockKind::Other);
                self.push_out(".. ");
                self.push_out(&admonition_type);
                self.push_out("::\n\n");
                let content = self.out.len();
                self.stack.push(Frame::Admonition { mark, content });
            }
            OwnedEvent::EndAdmonition => {
                if let Some(Frame::Admonition { mark, content }) = self.stack.pop() {
                    self.reindent(content);
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartFigure { url, alt } => {
                let mark = self.block_start(BlockKind::Other);
                self.push_out(".. figure:: ");
                self.push_out(&url);
                self.push_out("\n");
                if let Some(alt_text) = &alt {
                    self.push_out("   :alt: ");
                    self.push_out(alt_text);
                    self.push_out("\n");
                }
                let caption = self.out.len();
                self.stack.push(Frame::Figure { mark, caption });
            }
            OwnedEvent::EndFigure => {
                if let Some(Frame::Figure { mark, caption }) = self.stack.pop() {
                    // The caption lead-in is only emitted if caption content
                    // actually arrived — `build_figure` distinguishes "no
                    // caption" from "a caption". Inserting it now (the caption
                    // text is the only thing after `caption`) avoids buffering
                    // the caption separately just to learn whether it existed.
                    if self.out.len() > caption {
                        self.out.insert_str(caption, "\n   ");
                        self.out.push('\n');
                    }
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::ImageBlock { url, alt, .. } => {
                let mark = self.block_start(BlockKind::Other);
                self.push_out(".. image:: ");
                self.push_out(&url);
                self.push_out("\n");
                if let Some(alt_text) = &alt {
                    self.push_out("   :alt: ");
                    self.push_out(alt_text);
                    self.push_out("\n");
                }
                self.push_out("\n");
                self.block_end(mark);
            }

            // ── Inline events ──────────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(&cow, &cow);
            }
            OwnedEvent::SoftBreak | OwnedEvent::LineBreak => {
                self.push_inline("\n", " ");
            }
            OwnedEvent::StartEmphasis => self.open_span("*", "*"),
            OwnedEvent::EndEmphasis => self.close_span(),
            OwnedEvent::StartStrong => self.open_span("**", "**"),
            OwnedEvent::EndStrong => self.close_span(),
            OwnedEvent::StartStrikeout => self.open_span(":strike:`", "`"),
            OwnedEvent::EndStrikeout => self.close_span(),
            OwnedEvent::StartUnderline => self.open_span(":underline:`", "`"),
            OwnedEvent::EndUnderline => self.close_span(),
            OwnedEvent::StartSubscript => self.open_span(":sub:`", "`"),
            OwnedEvent::EndSubscript => self.close_span(),
            OwnedEvent::StartSuperscript => self.open_span(":sup:`", "`"),
            OwnedEvent::EndSuperscript => self.close_span(),
            OwnedEvent::StartSmallCaps => self.open_span(":sc:`", "`"),
            OwnedEvent::EndSmallCaps => self.close_span(),
            OwnedEvent::Code(cow) => {
                self.push_wrapped("``", &cow, "``", &cow);
            }
            OwnedEvent::StartLink { url } => {
                let mark = self.out.len();
                let plain_mark = self.plain.len();
                if self.accepts_inline() {
                    self.push_out("`");
                }
                self.stack.push(Frame::Link {
                    url,
                    mark,
                    plain_mark,
                });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link {
                    url,
                    mark,
                    plain_mark,
                }) = self.stack.pop()
                {
                    self.push_out(" <");
                    self.push_out(&url);
                    self.push_out(">`_");
                    self.inline_end(mark, plain_mark);
                }
            }
            OwnedEvent::InlineImage { url, alt } => {
                if alt.is_empty() {
                    self.push_multi(&[".. image:: ", &url, "\n"], &alt);
                } else {
                    self.push_multi(&[".. image:: ", &url, "\n   :alt: ", &alt, "\n"], &alt);
                }
            }
            OwnedEvent::FootnoteRef { label } => {
                self.push_wrapped("[", &label, "]_", &label);
            }
            OwnedEvent::StartFootnoteDefInline { label } => {
                let mark = self.out.len();
                let plain_mark = self.plain.len();
                if self.accepts_inline() {
                    self.push_out(".. [");
                    self.push_out(&label);
                    self.push_out("] ");
                }
                self.stack.push(Frame::Inline {
                    close: "",
                    mark,
                    plain_mark,
                });
            }
            OwnedEvent::EndFootnoteDefInline => self.close_span(),
            OwnedEvent::StartQuoted { quote_type } => {
                let quote = if quote_type == "single" { "'" } else { "\"" };
                self.open_span(quote, quote);
            }
            OwnedEvent::EndQuoted => self.close_span(),
            OwnedEvent::MathInline { source } => {
                self.push_wrapped(":math:`", &source, "`", &source);
            }
            OwnedEvent::StartRstSpan { role } => {
                let mark = self.out.len();
                let plain_mark = self.plain.len();
                if self.accepts_inline() {
                    self.push_out(":");
                    self.push_out(&role);
                    self.push_out(":`");
                }
                self.stack.push(Frame::Inline {
                    close: "`",
                    mark,
                    plain_mark,
                });
            }
            OwnedEvent::EndRstSpan => self.close_span(),
        }
    }
}

/// Mirrors `build_table`: cells only ever carry plain text (never formatted
/// markup), so `rows` is `(cell plain text, is_header)` pairs directly —
/// no `text_rows` recomputation needed since the cells were never anything
/// but plain text to begin with.
///
/// This is the one construct whose *prefix* genuinely depends on content not
/// yet seen: the first border line cannot be emitted until every cell of every
/// row is known. So cells are collected (bounded by the table's own size, well
/// inside the documented `O(largest top-level block)` limit) — but the render
/// itself still writes straight into the shared output buffer, and borrows the
/// collected cells for the width pass rather than cloning them.
fn render_table(rows: &[(Vec<String>, bool)], out: &mut String) {
    if rows.is_empty() {
        return;
    }

    let col_widths = calculate_column_widths(rows.iter().map(|(cells, _)| cells.as_slice()));

    emit_table_border(&col_widths, out);

    let mut is_first = true;
    for (cells, is_header) in rows {
        out.push('|');
        for (i, cell) in cells.iter().enumerate() {
            let width = col_widths.get(i).copied().unwrap_or(1);
            out.push(' ');
            out.push_str(cell);
            for _ in cell.len()..width {
                out.push(' ');
            }
            out.push_str(" |");
        }
        out.push('\n');

        if is_first && *is_header && rows.len() > 1 {
            emit_table_border(&col_widths, out);
        }
        is_first = false;
    }

    emit_table_border(&col_widths, out);
    out.push('\n');
}

/// Frames carry only marks into the shared buffers and tiny scalars — never
/// accumulated content. `mark` is where this construct's output begins in
/// `Writer::out` (so it can be discarded wholesale if it turns out to have no
/// valid enclosing context); `content` is where its *inner* content begins,
/// after any header the construct emits up front.
enum Frame {
    Paragraph {
        mark: usize,
    },
    Heading {
        level: i64,
        mark: usize,
        content: usize,
        plain_mark: usize,
    },
    Blockquote {
        mark: usize,
        content: usize,
    },
    List {
        ordered: bool,
        mark: usize,
    },
    ListItem {
        /// Whether the next child is this item's first — the flag
        /// `build_list_item` uses to decide the continuation indent.
        first: bool,
        mark: usize,
    },
    CodeBlock {
        mark: usize,
        content: usize,
    },
    Div {
        mark: usize,
    },
    Table {
        mark: usize,
        rows: Vec<(Vec<String>, bool)>,
    },
    TableRow {
        cells: Vec<String>,
        is_header: bool,
    },
    TableCell {
        plain_mark: usize,
    },
    DefinitionList {
        mark: usize,
    },
    DefinitionTerm {
        mark: usize,
    },
    DefinitionDesc {
        mark: usize,
    },
    FootnoteDef {
        mark: usize,
    },
    Figure {
        mark: usize,
        /// Where the caption's text would begin — compared against
        /// `out.len()` at `EndFigure` to tell "had a caption" from "had none".
        caption: usize,
    },
    Admonition {
        mark: usize,
        content: usize,
    },
    /// Any inline span whose closing delimiter is a fixed string known when
    /// the span opens (emphasis, strong, roles, quotes, inline footnote defs).
    Inline {
        close: &'static str,
        mark: usize,
        plain_mark: usize,
    },
    /// Links are the one inline span whose closing text depends on data
    /// carried by the frame (the URL, moved out of the opening event — not a
    /// content buffer).
    Link {
        url: String,
        mark: usize,
        plain_mark: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 1 });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned(
            "Hello".to_string(),
        )));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("Hello"), "got: {s:?}");
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
        let input =
            "Section\n=======\n\nA paragraph with *emphasis* text.\n\n- item one\n- item two\n";
        let doc = crate::parse(input).unwrap();
        let evts: Vec<_> = crate::EventIter::new(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let doc2 = crate::parse(&emitted_text).unwrap();
        // Compare block counts as a basic sanity check
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    /// The streaming `Writer` must produce *byte-identical* output to the
    /// tree-based `build()` for the same document. This is the guard that
    /// keeps the two independent emission paths honest: the write-through
    /// buffer model may not drift from `build_*`'s formatting in any
    /// construct, including the deferred ones (heading underline widths,
    /// blockquote/admonition/code re-indent, list-item continuation indents,
    /// figure caption lead-in, table column widths).
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "Title\n=====\n\nIntro paragraph with **strong** and ``code``.\n",
            "Sub\n---\n\ntext with *em* and a `link <http://x/>`_ here.\n",
            "- bullet one\n- bullet two\n\n  - nested a\n  - nested b\n",
            "#. ordered one\n#. ordered two\n",
            ".. code-block:: rust\n\n   let x = 1;\n   let y = 2;\n",
            "::\n\n   literal block\n",
            "    An indented block quote.\n\n    Second para of quote.\n",
            ".. note::\n\n   Some note body.\n\n   Second para.\n",
            "term\n    definition body\n\nterm2\n    another definition\n",
            "+--------+--------+\n| A      | B      |\n+========+========+\n| Cell 1 | Cell 2 |\n+--------+--------+\n",
            "See [1]_ for details.\n\n.. [1] Footnote body text.\n",
            "----\n\nAfter the transition.\n",
            ".. figure:: img.png\n   :alt: alt text\n\n   The caption.\n",
            ".. image:: img.png\n   :alt: alt text\n",
            "| line one\n| line two\n",
            "A para\n\n- item\n\n  continued paragraph in item\n",
        ];
        for input in inputs {
            let doc = crate::parse(input).unwrap();
            let built = crate::build(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::EventIter::new(input) {
                w.write_event(e);
            }
            let streamed = String::from_utf8(w.finish()).unwrap();

            assert_eq!(
                built, streamed,
                "streaming Writer diverged from build() for input:\n{input}\n\
                 build():\n{built:?}\nWriter:\n{streamed:?}"
            );
        }
    }

    /// Round-trip a broader construct mix (headings, lists, code blocks,
    /// tables, definition lists, footnotes, admonitions, block quotes)
    /// entirely through `events() -> Writer`, proving the incremental
    /// per-top-level-block flush handles every construct `parse()`
    /// produces, not just paragraphs/headings/lists.
    #[test]
    fn test_writer_roundtrip_full_construct_mix() {
        let input = "\
Title
=====

Intro paragraph with **strong** and ``code``.

    An indented block quote.

- bullet one
- bullet two

1. ordered one
2. ordered two

.. code-block:: rust

   let x = 1;

term
    definition body

+--------+--------+
| A      | B      |
+========+========+
| Cell 1 | Cell 2 |
+--------+--------+

See [1]_ for details.

.. [1] Footnote body text.

----

After the transition.
";
        let doc = crate::parse(input).unwrap();
        assert!(
            doc.blocks.len() >= 10,
            "expected a rich construct mix, got {:?}",
            doc.blocks
        );

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::EventIter::new(input) {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();

        // Re-parsing the writer's output must recover the same block shapes,
        // in the same order — the same discriminant-only comparison
        // `test_events_matches_parse_shape` uses in lib.rs, applied here to
        // prove round-trip parity through the streaming writer specifically.
        let doc2 = crate::parse(&emitted_text).unwrap();
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "writer roundtrip block count mismatch\ninput blocks: {:#?}\nemitted text: {emitted_text}\nreparsed blocks: {:#?}",
            doc.blocks,
            doc2.blocks,
        );
        for (a, b) in doc.blocks.iter().zip(doc2.blocks.iter()) {
            assert_eq!(
                std::mem::discriminant(a),
                std::mem::discriminant(b),
                "block kind mismatch: {a:?} vs {b:?}"
            );
        }
    }

    /// Nested lists (bullet-in-bullet, ordered-in-bullet) are the trickiest
    /// path in the streaming rewrite: `list_depth` bookkeeping happens on
    /// `Writer` itself (bracketing `StartList`/`EndList`), and each nested
    /// list's text is written straight into the shared buffer behind the
    /// continuation indent emitted when the nested list *opened*.
    #[test]
    fn test_writer_roundtrip_nested_lists() {
        let input = "\
- outer one
- outer two

  - inner a
  - inner b

- outer three

  #. inner ordered one
  #. inner ordered two
";
        let doc = crate::parse(input).unwrap();

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::EventIter::new(input) {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();

        let doc2 = crate::parse(&emitted_text).unwrap();
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "nested list roundtrip block count mismatch\nemitted:\n{emitted_text}"
        );
        for (a, b) in doc.blocks.iter().zip(doc2.blocks.iter()) {
            assert_eq!(
                std::mem::discriminant(a),
                std::mem::discriminant(b),
                "block kind mismatch: {a:?} vs {b:?}\nemitted:\n{emitted_text}"
            );
        }
        // The nested lists must actually still be nested after roundtrip,
        // not flattened to sibling top-level lists.
        fn count_lists(blocks: &[crate::Block]) -> usize {
            let mut n = 0;
            for b in blocks {
                if let crate::Block::List { items, .. } = b {
                    n += 1;
                    for item in items {
                        n += count_lists(item);
                    }
                }
            }
            n
        }
        assert_eq!(
            count_lists(&doc.blocks),
            count_lists(&doc2.blocks),
            "nested list count changed across roundtrip\nemitted:\n{emitted_text}"
        );
        assert!(
            count_lists(&doc2.blocks) >= 3,
            "expected outer list + 2 nested lists, emitted:\n{emitted_text}"
        );
    }

    /// Regression guard against reintroducing per-block `Block`/`Inline`
    /// subtree reconstruction (the original perf bug: `Writer::process`
    /// rebuilt an owned tree per top-level block, then ran the same
    /// `build_block` pass `build()` already runs directly — ~7-8x slower).
    /// A large, deeply-nested event stream must complete with an allocation
    /// count that stays close to linear in event count, not blow up the way
    /// tree materialization (an `Inline`/`Block` enum + `Vec` per node, on
    /// top of the formatting pass) would. This does not merely check output
    /// correctness — the existing roundtrip tests already do that — it
    /// checks the *cost shape* of getting there.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingAlloc;
        static ALLOCS: AtomicUsize = AtomicUsize::new(0);
        unsafe impl GlobalAlloc for CountingAlloc {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                ALLOCS.fetch_add(1, Ordering::Relaxed);
                unsafe { System.alloc(layout) }
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                unsafe { System.dealloc(ptr, layout) }
            }
        }
        #[global_allocator]
        static GLOBAL: CountingAlloc = CountingAlloc;

        // Build an event stream for `n` top-level sections (heading +
        // paragraph with inline markup + a 2-item list), doubling `n` to
        // check allocation count scales roughly linearly, not
        // superlinearly (which tree reconstruction with repeated
        // re-formatting would exhibit).
        fn events_for(n: usize) -> Vec<OwnedEvent> {
            use std::borrow::Cow;
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(OwnedEvent::StartHeading { level: 2 });
                evs.push(OwnedEvent::Text(Cow::Owned(format!("Section {i}"))));
                evs.push(OwnedEvent::EndHeading);
                evs.push(OwnedEvent::StartParagraph);
                evs.push(OwnedEvent::Text(Cow::Owned("plain ".to_string())));
                evs.push(OwnedEvent::StartStrong);
                evs.push(OwnedEvent::Text(Cow::Owned("bold".to_string())));
                evs.push(OwnedEvent::EndStrong);
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

        // Allocation count for feeding events through Writer must scale
        // sub-quadratically: 10x the sections should cost well under 10x^2
        // (=100x) the allocations. A generous 20x ceiling comfortably
        // separates "linear-ish" from "reconstructing and reformatting a
        // full subtree per block" while tolerating fixed overhead noise at
        // the small end.
        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 20.0,
            "allocation count did not scale near-linearly: {small} allocs @200 sections -> \
             {large} allocs @2000 sections (ratio {ratio:.2}); this suggests reintroduced \
             per-block subtree reconstruction"
        );
    }
}
