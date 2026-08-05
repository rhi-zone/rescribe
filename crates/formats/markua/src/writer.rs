#![allow(clippy::collapsible_if)]
//! Streaming Markua writer — converts a stream of events directly to Markua text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::Block`]/[`crate::Inline`] value and
//! never calls [`crate::emit::emit`]/`build`. It is a second, independent
//! emission path from the tree-based `emit()`/`build()` functions, not a thin
//! wrapper around them.
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document. Frames on the `Vec<Frame>` stack (`O(nesting depth)`) hold
//! only small metadata — a `usize` mark into `out`, a `&'static str` prefix,
//! a counter. Children write **straight through** into `out`; a frame that
//! needs to decorate its own content afterwards post-processes the
//! `out[mark..]` range in place.
//!
//! Every construct falls into one of three classes:
//!
//! - **Write-through** (no per-frame buffering, everything known at
//!   `Start*`): `Paragraph`, `Heading`, `CodeBlock`, `HorizontalRule`,
//!   `PageBreak`, `List`/`ListItem` (the bullet/ordinal is known the moment
//!   the item opens — Markua does not right-align or compute ordinal
//!   widths), `Table`/`TableRow`/`TableCell` (Markua's `| --- |` separator
//!   row only needs the *first* row's cell count, known at that row's own
//!   close — no column-width computation, unlike RST), `DefinitionList`/
//!   `Term`/`Desc`, and every inline span (`Strong`, `Emphasis`, `Link`, …).
//! - **Write-through + one in-place insert** (content is contiguous at the
//!   end of `out` when the deferred prefix becomes known): `Link` (the URL
//!   is only used as link text if no child inlines arrived — checked at
//!   `EndLink` by comparing `out.len()` to the mark taken right after `[`)
//!   and `Figure`/`Caption` (the `"Figure: "` lead-in is only emitted if
//!   caption content actually arrived, mirroring `Link`'s technique).
//! - **Deferred per-line transform** (every line of already-written content
//!   must be re-prefixed, and — for `SpecialBlock`/`DefinitionDesc` only —
//!   have trailing blank lines trimmed first): a *non-paragraph* block
//!   nested directly inside `Blockquote` (`"> "`, no trim), `SpecialBlock`
//!   (`"A> "`/`"W> "`/…, trimmed), or `DefinitionDesc` (`": "`, trimmed).
//!   Paragraphs get a cheaper direct path instead (prefix written at
//!   `StartParagraph`, single `"\n"` at `EndParagraph`) because re-running
//!   the line-splitting transform on already-correct single-line content
//!   would be pure overhead for the overwhelmingly common case. These
//!   post-process `out[mark..]` through a *pooled, reused* scratch buffer
//!   (`Writer::scratch`), so the pool holds at most `O(nesting depth)`
//!   buffers for the whole document rather than one fresh allocation per
//!   construct. Nesting (e.g. a `Blockquote` inside a `SpecialBlock`)
//!   composes automatically: each level's transform runs over whatever the
//!   inner level already wrote, which is exactly what the tree-based
//!   `emit_block`'s recursive-string-then-reprefix approach does too.
//!
//! Markua has **no genuinely content-dependent-prefix construct** the way
//! RST's table column widths are — every deferral above is a small, bounded
//! per-line or per-child transform, not a whole-subtree buffer.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use markua::writer::Writer;
//! use markua::OwnedMarkuaEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedMarkuaEvent::StartHeading { level: 1 });
//! w.write_event(OwnedMarkuaEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedMarkuaEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedMarkuaEvent;
use std::fmt::Write as _;
use std::io::Write;

/// Default capacity reserved for `Writer::out`. Skips the first several
/// geometric-growth doublings (pure overhead below any realistic top-level
/// block size) without committing to a document-specific guess.
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Frame stack entry for the construct currently being assembled. Payload is
/// kept to small scalars/marks only — no accumulated content is ever copied
/// into a `Frame`.
enum Frame {
    Paragraph {
        mark: usize,
        mode: ParaMode,
    },
    Heading {
        mark: usize,
    },
    Blockquote {
        mark: usize,
    },
    List {
        mark: usize,
        ordered: bool,
        num: u32,
    },
    ListItem {
        mark: usize,
    },
    Table {
        mark: usize,
        row_idx: u32,
    },
    TableRow {
        mark: usize,
        cell_count: u32,
    },
    TableCell {
        mark: usize,
    },
    SpecialBlock {
        mark: usize,
        prefix: &'static str,
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
    Figure {
        mark: usize,
    },
    Caption {
        mark: usize,
    },
    /// Inline span with a fixed close delimiter (`Strong`, `Emphasis`, …).
    Inline {
        mark: usize,
        close: &'static str,
    },
    Link {
        mark: usize,
        content_mark: usize,
        url: String,
    },
}

/// How a `Paragraph` frame closes, decided once at `StartParagraph` from the
/// enclosing frame — never revisited, never requiring a reindent pass.
enum ParaMode {
    /// Top-level-shaped context (`Document`, `Figure`, …): trailing `"\n\n"`.
    Normal,
    /// Inside `Blockquote`/`SpecialBlock`/`DefinitionDesc`: the prefix was
    /// already written at `StartParagraph`; trailing `"\n"` only, no blank
    /// line (matching `emit_block`'s dedicated single-line paragraph arm in
    /// each of those three cases, as opposed to the generic multi-line
    /// reindent arm used for every other block kind there).
    Quoted,
    /// Inside `ListItem`: no trailing newline at all — `EndListItem` supplies
    /// the item's single closing `"\n"`.
    ListItemBody,
}

/// Streaming Markua writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to recover the sink once all events have been
/// fed — `finish` does not itself build or emit anything, every completed
/// top-level block was already flushed by `write_event`.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Every construct writes here
    /// directly; frames record marks into it. Cleared (capacity retained)
    /// after each top-level block is flushed.
    out: String,
    /// Pool of scratch buffers for the deferred per-line reindent path
    /// (nested `Blockquote`/`SpecialBlock`/`DefinitionDesc` children).
    /// Buffers are returned after use, so at most `O(nesting depth)` are
    /// ever allocated for a whole document instead of one per construct.
    scratch: Vec<String>,
    /// Frame stack for the construct currently being assembled. Empty at top
    /// level — a block closing with an empty stack is flushed to the sink
    /// immediately.
    stack: Vec<Frame>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            out: String::with_capacity(DEFAULT_OUT_CAPACITY),
            scratch: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: OwnedMarkuaEvent) {
        self.process(event);
    }

    /// Recover the underlying sink. Does not write anything — every
    /// completed top-level block was already flushed by `write_event`.
    pub fn finish(self) -> W {
        self.sink
    }

    // ── Buffer primitives ────────────────────────────────────────────────

    /// Flush the completed top-level block to the sink and reset the
    /// buffer, keeping its capacity so the document only ever grows one
    /// buffer.
    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Whether the current top-of-stack frame accepts block children —
    /// mirrors `DocBuilder::push_block`'s dispatch targets exactly
    /// (`Document` is the empty stack). A `SpecialBlock` of unrecognized
    /// kind (empty `prefix`) does *not* accept blocks: `emit_block` drops
    /// its children entirely rather than emitting them unprefixed, so this
    /// writer must too.
    fn accepts_block(&self) -> bool {
        match self.stack.last() {
            None
            | Some(
                Frame::Blockquote { .. }
                | Frame::ListItem { .. }
                | Frame::Figure { .. }
                | Frame::DefinitionDesc { .. },
            ) => true,
            Some(Frame::SpecialBlock { prefix, .. }) => !prefix.is_empty(),
            _ => false,
        }
    }

    /// Whether the current top-of-stack frame accepts inline children —
    /// mirrors `DocBuilder::push_inline`'s dispatch targets exactly.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
                    | Frame::TableCell { .. }
                    | Frame::DefinitionTerm { .. }
                    | Frame::Caption { .. }
            )
        )
    }

    /// The per-line prefix + trim-trailing-blank-lines flag for the current
    /// top-of-stack frame, if it is one of the three "quoting" containers
    /// whose non-paragraph children get reindented. `None` for everything
    /// else, including an unrecognized-kind `SpecialBlock` (see
    /// `accepts_block`).
    fn quoting_ctx(&self) -> Option<(&'static str, bool)> {
        match self.stack.last() {
            Some(Frame::Blockquote { .. }) => Some(("> ", false)),
            Some(Frame::SpecialBlock { prefix, .. }) if !prefix.is_empty() => Some((prefix, true)),
            Some(Frame::DefinitionDesc { .. }) => Some((": ", true)),
            _ => None,
        }
    }

    /// Re-prefix every line of `out[mark..]` in place, optionally trimming
    /// trailing blank lines first — replicating `emit_block`'s
    /// `for line in inner[.trim_end()].lines() { prefix + line + "\n" }`
    /// exactly. Uses a pooled scratch buffer rather than a fresh allocation
    /// per construct.
    fn reindent_prefixed(&mut self, mark: usize, prefix: &str, trim: bool) {
        let mut buf = self.scratch.pop().unwrap_or_default();
        buf.clear();
        let region = if trim {
            self.out[mark..].trim_end()
        } else {
            &self.out[mark..]
        };
        for line in region.lines() {
            buf.push_str(prefix);
            buf.push_str(line);
            buf.push('\n');
        }
        self.out.truncate(mark);
        self.out.push_str(&buf);
        self.scratch.push(buf);
    }

    /// Close a block that opened at `out.len() == mark`. Non-paragraph
    /// blocks get reindented first if the enclosing frame is a quoting
    /// container; blocks in a context that does not accept them at all are
    /// discarded (truncated back to `mark`, mirroring `DocBuilder`'s silent
    /// `_ => {}` drop). Flushes to the sink if this closed the last open
    /// frame.
    fn close_block(&mut self, mark: usize, paragraph_like: bool) {
        if let Some((prefix, trim)) = self.quoting_ctx() {
            if !paragraph_like {
                self.reindent_prefixed(mark, prefix, trim);
            }
        } else if !self.accepts_block() {
            self.out.truncate(mark);
            return;
        }
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Open an inline span whose opening delimiter is already known.
    fn open_span(&mut self, open: &str, close: &'static str) {
        let mark = self.out.len();
        if self.accepts_inline() {
            self.out.push_str(open);
        }
        self.stack.push(Frame::Inline { mark, close });
    }

    fn close_span(&mut self) {
        if let Some(Frame::Inline { mark, close }) = self.stack.pop() {
            if self.accepts_inline() {
                self.out.push_str(close);
            } else {
                self.out.truncate(mark);
            }
        }
    }

    /// Write a leaf inline's text, gated on the current context accepting
    /// inline content.
    fn push_leaf_inline(&mut self, s: &str) {
        if self.accepts_inline() {
            self.out.push_str(s);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedMarkuaEvent) {
        match event {
            // ── Block open/close ────────────────────────────────────────
            OwnedMarkuaEvent::StartParagraph => {
                let mark = self.out.len();
                let mode = if matches!(self.stack.last(), Some(Frame::ListItem { .. })) {
                    ParaMode::ListItemBody
                } else if let Some((prefix, _)) = self.quoting_ctx() {
                    self.out.push_str(prefix);
                    ParaMode::Quoted
                } else {
                    ParaMode::Normal
                };
                self.stack.push(Frame::Paragraph { mark, mode });
            }
            OwnedMarkuaEvent::EndParagraph => {
                if let Some(Frame::Paragraph { mark, mode }) = self.stack.pop() {
                    match mode {
                        ParaMode::Normal => self.out.push_str("\n\n"),
                        ParaMode::Quoted => self.out.push('\n'),
                        ParaMode::ListItemBody => {}
                    }
                    self.close_block(mark, true);
                }
            }
            OwnedMarkuaEvent::StartHeading { level } => {
                let mark = self.out.len();
                for _ in 0..level {
                    self.out.push('#');
                }
                self.out.push(' ');
                self.stack.push(Frame::Heading { mark });
            }
            OwnedMarkuaEvent::EndHeading => {
                if let Some(Frame::Heading { mark }) = self.stack.pop() {
                    self.out.push_str("\n\n");
                    self.close_block(mark, false);
                }
            }
            OwnedMarkuaEvent::StartBlockquote => {
                let mark = self.out.len();
                self.stack.push(Frame::Blockquote { mark });
            }
            OwnedMarkuaEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    self.close_block(mark, false);
                }
            }
            OwnedMarkuaEvent::StartList { ordered } => {
                let mark = self.out.len();
                self.stack.push(Frame::List {
                    mark,
                    ordered,
                    num: 1,
                });
            }
            OwnedMarkuaEvent::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.out.push('\n');
                    self.close_block(mark, false);
                }
            }
            OwnedMarkuaEvent::StartListItem => {
                let mark = self.out.len();
                if let Some(Frame::List { ordered, num, .. }) = self.stack.last_mut() {
                    if *ordered {
                        let _ = write!(self.out, "{num}. ");
                        *num += 1;
                    } else {
                        self.out.push_str("- ");
                    }
                }
                self.stack.push(Frame::ListItem { mark });
            }
            OwnedMarkuaEvent::EndListItem => {
                if let Some(Frame::ListItem { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.out.push('\n');
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedMarkuaEvent::CodeBlock { language, content } => {
                let mark = self.out.len();
                self.out.push_str("```");
                if let Some(lang) = &language {
                    self.out.push_str(lang);
                }
                self.out.push('\n');
                self.out.push_str(&content);
                if !content.ends_with('\n') {
                    self.out.push('\n');
                }
                self.out.push_str("```\n\n");
                self.close_block(mark, false);
            }
            OwnedMarkuaEvent::HorizontalRule => {
                let mark = self.out.len();
                self.out.push_str("* * *\n\n");
                self.close_block(mark, false);
            }
            OwnedMarkuaEvent::PageBreak => {
                let mark = self.out.len();
                self.out.push_str("{pagebreak}\n\n");
                self.close_block(mark, false);
            }
            OwnedMarkuaEvent::StartTable => {
                let mark = self.out.len();
                self.stack.push(Frame::Table { mark, row_idx: 0 });
            }
            OwnedMarkuaEvent::EndTable => {
                if let Some(Frame::Table { mark, .. }) = self.stack.pop() {
                    self.out.push('\n');
                    self.close_block(mark, false);
                }
            }
            OwnedMarkuaEvent::StartTableRow => {
                let mark = self.out.len();
                self.out.push('|');
                self.stack.push(Frame::TableRow {
                    mark,
                    cell_count: 0,
                });
            }
            OwnedMarkuaEvent::EndTableRow => {
                if let Some(Frame::TableRow { mark, cell_count }) = self.stack.pop() {
                    self.out.push('\n');
                    if let Some(Frame::Table { row_idx, .. }) = self.stack.last_mut() {
                        let is_first = *row_idx == 0;
                        *row_idx += 1;
                        if is_first {
                            self.out.push('|');
                            for _ in 0..cell_count {
                                self.out.push_str(" --- |");
                            }
                            self.out.push('\n');
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedMarkuaEvent::StartTableCell => {
                let mark = self.out.len();
                self.out.push(' ');
                self.stack.push(Frame::TableCell { mark });
            }
            OwnedMarkuaEvent::EndTableCell => {
                if let Some(Frame::TableCell { mark }) = self.stack.pop() {
                    self.out.push_str(" |");
                    if let Some(Frame::TableRow { cell_count, .. }) = self.stack.last_mut() {
                        *cell_count += 1;
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedMarkuaEvent::StartSpecialBlock { kind } => {
                let mark = self.out.len();
                let prefix = match kind.as_str() {
                    "aside" => "A> ",
                    "blurb" => "B> ",
                    "warning" => "W> ",
                    "tip" => "T> ",
                    "error" => "E> ",
                    "discussion" => "D> ",
                    "question" => "Q> ",
                    "information" => "I> ",
                    "exercise" => "X> ",
                    _ => "",
                };
                self.stack.push(Frame::SpecialBlock { mark, prefix });
            }
            OwnedMarkuaEvent::EndSpecialBlock => {
                if let Some(Frame::SpecialBlock { mark, prefix }) = self.stack.pop() {
                    if prefix.is_empty() {
                        // Unrecognized block type: `emit_block` never enters
                        // its child-emitting loop at all for this case, so
                        // nothing — not even an unprefixed rendering of the
                        // children — is emitted.
                        self.out.truncate(mark);
                    } else {
                        self.out.push('\n');
                    }
                    self.close_block(mark, false);
                }
            }
            OwnedMarkuaEvent::StartDefinitionList => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionList { mark });
            }
            OwnedMarkuaEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    self.close_block(mark, false);
                }
            }
            OwnedMarkuaEvent::StartDefinitionTerm => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionTerm { mark });
            }
            OwnedMarkuaEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.out.push('\n');
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedMarkuaEvent::StartDefinitionDesc => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionDesc { mark });
            }
            OwnedMarkuaEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { mark }) = self.stack.pop() {
                    if !matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedMarkuaEvent::StartFigure => {
                let mark = self.out.len();
                self.stack.push(Frame::Figure { mark });
            }
            OwnedMarkuaEvent::EndFigure => {
                if let Some(Frame::Figure { mark }) = self.stack.pop() {
                    self.close_block(mark, false);
                }
            }
            OwnedMarkuaEvent::StartCaption => {
                let mark = self.out.len();
                self.stack.push(Frame::Caption { mark });
            }
            OwnedMarkuaEvent::EndCaption => {
                if let Some(Frame::Caption { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::Figure { .. })) {
                        // The "Figure: " lead-in is only emitted if caption
                        // content actually arrived — mirrors `Link`'s
                        // empty-children check. Inserting it now (the
                        // caption text is the only thing after `mark`)
                        // avoids buffering the caption separately just to
                        // learn whether it existed.
                        if self.out.len() > mark {
                            self.out.insert_str(mark, "Figure: ");
                            self.out.push_str("\n\n");
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }

            // ── Inline events ───────────────────────────────────────────
            OwnedMarkuaEvent::Text(cow) => self.push_leaf_inline(&cow),
            OwnedMarkuaEvent::SoftBreak => self.push_leaf_inline(" "),
            OwnedMarkuaEvent::LineBreak => self.push_leaf_inline("\\\n"),
            OwnedMarkuaEvent::StartStrong => self.open_span("**", "**"),
            OwnedMarkuaEvent::EndStrong => self.close_span(),
            OwnedMarkuaEvent::StartEmphasis => self.open_span("*", "*"),
            OwnedMarkuaEvent::EndEmphasis => self.close_span(),
            OwnedMarkuaEvent::StartStrikethrough => self.open_span("~~", "~~"),
            OwnedMarkuaEvent::EndStrikethrough => self.close_span(),
            OwnedMarkuaEvent::StartSubscript => self.open_span("~", "~"),
            OwnedMarkuaEvent::EndSubscript => self.close_span(),
            OwnedMarkuaEvent::StartSuperscript => self.open_span("^", "^"),
            OwnedMarkuaEvent::EndSuperscript => self.close_span(),
            OwnedMarkuaEvent::StartUnderline => self.open_span("[underline]#", "#"),
            OwnedMarkuaEvent::EndUnderline => self.close_span(),
            OwnedMarkuaEvent::StartSmallCaps => self.open_span("[smallcaps]#", "#"),
            OwnedMarkuaEvent::EndSmallCaps => self.close_span(),
            OwnedMarkuaEvent::StartFootnoteRef => self.open_span("^[", "]"),
            OwnedMarkuaEvent::EndFootnoteRef => self.close_span(),
            OwnedMarkuaEvent::InlineCode(cow) => {
                if self.accepts_inline() {
                    if cow.contains('`') {
                        self.out.push_str("`` ");
                        self.out.push_str(&cow);
                        self.out.push_str(" ``");
                    } else {
                        self.out.push('`');
                        self.out.push_str(&cow);
                        self.out.push('`');
                    }
                }
            }
            OwnedMarkuaEvent::StartLink { url } => {
                let mark = self.out.len();
                if self.accepts_inline() {
                    self.out.push('[');
                }
                let content_mark = self.out.len();
                self.stack.push(Frame::Link {
                    mark,
                    content_mark,
                    url,
                });
            }
            OwnedMarkuaEvent::EndLink => {
                if let Some(Frame::Link {
                    mark,
                    content_mark,
                    url,
                }) = self.stack.pop()
                {
                    if self.accepts_inline() {
                        // No child inlines arrived: fall back to the URL as
                        // the link text, matching `emit_inline`'s
                        // `if children.is_empty() { push_str(url) }`.
                        if self.out.len() == content_mark {
                            self.out.push_str(&url);
                        }
                        self.out.push_str("](");
                        self.out.push_str(&url);
                        self.out.push(')');
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedMarkuaEvent::Image { url, alt } => {
                if self.accepts_inline() {
                    self.out.push_str("![");
                    self.out.push_str(&alt);
                    self.out.push_str("](");
                    self.out.push_str(&url);
                    self.out.push(')');
                }
            }
            OwnedMarkuaEvent::IndexTerm { term } => {
                if self.accepts_inline() {
                    self.out.push_str("i[");
                    self.out.push_str(&term);
                    self.out.push(']');
                }
            }
            OwnedMarkuaEvent::MathInline { content } => {
                if self.accepts_inline() {
                    self.out.push('$');
                    self.out.push_str(&content);
                    self.out.push('$');
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMarkuaEvent::StartHeading { level: 1 });
        w.write_event(OwnedMarkuaEvent::Text(std::borrow::Cow::Owned(
            "Hello".to_string(),
        )));
        w.write_event(OwnedMarkuaEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("# Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMarkuaEvent::StartParagraph);
        w.write_event(OwnedMarkuaEvent::Text(std::borrow::Cow::Owned(
            "World".to_string(),
        )));
        w.write_event(OwnedMarkuaEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "# Hello\n\nA paragraph with **bold** text.\n\n- item one\n- item two\n";
        let evts: Vec<_> = crate::events::events_str(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse::parse_str(input);
        let (doc_emit, _) = crate::parse::parse_str(&emitted_text);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    /// Incrementality probe: at least one byte must reach the sink *before*
    /// `finish()` is called. This is the exact defect the old
    /// buffer-everything-then-reconstruct-the-AST implementation had (see
    /// `crates/rescribe-fixtures/src/streaming_harness.rs`'s prior `markua`
    /// `KnownFailure` entry for `streaming_writer`).
    #[test]
    fn test_writer_flushes_incrementally() {
        struct Probe {
            saw_bytes_before_finish: std::cell::Cell<bool>,
        }
        impl Write for &Probe {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if !buf.is_empty() {
                    self.saw_bytes_before_finish.set(true);
                }
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let probe = Probe {
            saw_bytes_before_finish: std::cell::Cell::new(false),
        };
        let mut w = Writer::new(&probe);
        w.write_event(OwnedMarkuaEvent::StartParagraph);
        w.write_event(OwnedMarkuaEvent::Text(std::borrow::Cow::Owned(
            "hi".to_string(),
        )));
        w.write_event(OwnedMarkuaEvent::EndParagraph);
        assert!(
            probe.saw_bytes_before_finish.get(),
            "no bytes reached the sink before finish() — writer is still buffer-everything"
        );
        w.finish();
    }

    /// The streaming `Writer` must produce *byte-identical* output to the
    /// tree-based `emit()`/`build()` for the same document. This is the
    /// guard that keeps the two independent emission paths honest.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "# Title\n\nIntro paragraph with **strong** and `code`.\n",
            "## Sub\n\ntext with *em* and a [link](http://x/) here.\n",
            "- bullet one\n- bullet two\n",
            "1. ordered one\n2. ordered two\n",
            "```rust\nlet x = 1;\nlet y = 2;\n```\n",
            "```\nliteral block\n```\n",
            "> A blockquote.\n> Second line.\n",
            "W> Some warning body.\n\nW> Second paragraph.\n",
            "A> - item 1\nA> - item 2\n",
            "term\n: definition body\n\nterm2\n: another definition\n",
            "| A | B |\n| --- | --- |\n| 1 | 2 |\n",
            "See ^[a note] for details.\n",
            "* * *\n\nAfter the transition.\n",
            "{pagebreak}\n",
            "H~2~O and x^2^\n",
            "See i[Markua] for details.\n",
            "Solve $x^2 + 1 = 0$.\n",
            "![Alt text](image.png)\n",
            "> W> Nested warning inside blockquote.\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse_str(input);
            let built = crate::emit::emit(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events::events_str(input) {
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

    /// `Block::Figure` is never constructed by `parse()` (confirmed: no
    /// Markua syntax builds one — see `MarkuaDoc`/`Block::Figure` doc notes
    /// and `crates/rescribe-fixtures/src/streaming_harness.rs`'s `markua`
    /// entry), so the `StartFigure`/`StartCaption`/`EndCaption`/`EndFigure`
    /// event path can only be exercised via a hand-built AST and a
    /// hand-built matching event stream (`EventIter::expand_block`'s own
    /// `Block::Figure` arm defines what a real event stream for a figure
    /// looks like). This is exactly that: it proves the rewritten writer's
    /// figure/caption handling matches `emit()`'s tree-based semantics byte
    /// for byte, including the caption itself — the *old*
    /// buffer-and-reconstruct writer discarded the caption unconditionally
    /// (`writer.rs:315-330` before this rewrite: `EndFigure` always built
    /// `Block::Figure { caption: vec![], .. }`). That was a real bug in the
    /// abandoned reconstruction path, not a limitation inherent to the
    /// events themselves; this rewrite fixes it as a side effect of no
    /// longer routing through that path, while the reader-side gap (`parse`
    /// never producing `Block::Figure`) remains open and out of scope here.
    #[test]
    fn test_writer_figure_caption_matches_builder() {
        use crate::ast::{Block, Inline, MarkuaDoc, Span};

        let doc = MarkuaDoc {
            blocks: vec![Block::Figure {
                caption: vec![Inline::Text("The caption.".to_string(), Span::NONE)],
                body: Box::new(Block::Paragraph {
                    inlines: vec![Inline::Text("Figure body.".to_string(), Span::NONE)],
                    span: Span::NONE,
                }),
                span: Span::NONE,
            }],
            span: Span::NONE,
            title: None,
            author: None,
            description: None,
        };
        let built = crate::emit::emit(&doc);
        assert!(
            built.contains("Figure: The caption."),
            "sanity: emit() should render the caption; got {built:?}"
        );

        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMarkuaEvent::StartFigure);
        w.write_event(OwnedMarkuaEvent::StartParagraph);
        w.write_event(OwnedMarkuaEvent::Text(std::borrow::Cow::Borrowed(
            "Figure body.",
        )));
        w.write_event(OwnedMarkuaEvent::EndParagraph);
        w.write_event(OwnedMarkuaEvent::StartCaption);
        w.write_event(OwnedMarkuaEvent::Text(std::borrow::Cow::Borrowed(
            "The caption.",
        )));
        w.write_event(OwnedMarkuaEvent::EndCaption);
        w.write_event(OwnedMarkuaEvent::EndFigure);
        let streamed = String::from_utf8(w.finish()).unwrap();

        assert_eq!(
            built, streamed,
            "streaming Writer diverged from emit() for a hand-built Figure/Caption \
             event stream\nemit():\n{built:?}\nWriter:\n{streamed:?}"
        );
    }

    /// An empty caption (`StartCaption` immediately followed by `EndCaption`,
    /// no inline content) must not emit the `"Figure: "` lead-in at all —
    /// matching `emit_block`'s `if !caption.is_empty()` guard.
    #[test]
    fn test_writer_figure_empty_caption_omits_label() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMarkuaEvent::StartFigure);
        w.write_event(OwnedMarkuaEvent::StartParagraph);
        w.write_event(OwnedMarkuaEvent::Text(std::borrow::Cow::Borrowed("Body.")));
        w.write_event(OwnedMarkuaEvent::EndParagraph);
        w.write_event(OwnedMarkuaEvent::StartCaption);
        w.write_event(OwnedMarkuaEvent::EndCaption);
        w.write_event(OwnedMarkuaEvent::EndFigure);
        let streamed = String::from_utf8(w.finish()).unwrap();
        assert!(
            !streamed.contains("Figure:"),
            "empty caption must not produce a \"Figure: \" label, got {streamed:?}"
        );
    }

    /// Round-trip a broader construct mix entirely through
    /// `events() -> Writer`, proving the incremental per-top-level-block
    /// flush handles every construct `parse()` produces.
    #[test]
    fn test_writer_roundtrip_full_construct_mix() {
        let input = "\
# Title

Intro paragraph with **strong** and `code`.

> A blockquote.

- bullet one
- bullet two

1. ordered one
2. ordered two

```rust
let x = 1;
```

term
: definition body

| A | B |
| --- | --- |
| 1 | 2 |

W> A warning.

See ^[a note] for details.

* * *

After the transition.
";
        let (doc, _) = crate::parse::parse_str(input);
        assert!(
            doc.blocks.len() >= 9,
            "expected a rich construct mix, got {:?}",
            doc.blocks
        );

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events_str(input) {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();

        let (doc2, _) = crate::parse::parse_str(&emitted_text);
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "writer roundtrip block count mismatch\ninput blocks: {:#?}\n\
             emitted text: {emitted_text}\nreparsed blocks: {:#?}",
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

    /// Nested lists round-trip through the streaming writer.
    #[test]
    fn test_writer_roundtrip_nested_lists() {
        let input = "\
- outer one
- outer two

  - inner a
  - inner b

- outer three
";
        let (doc, _) = crate::parse::parse_str(input);

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events_str(input) {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();

        let (doc2, _) = crate::parse::parse_str(&emitted_text);
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "nested list roundtrip block count mismatch\nemitted:\n{emitted_text}"
        );
    }

    // A test binary may only declare one `#[global_allocator]`. The
    // peak-live-bytes guard (`test_writer_peak_memory_bounded`, following the
    // same peak-tracking-allocator pattern) lives in its own dedicated
    // integration test binary instead of here — see
    // `tests/streaming_writer_memory.rs` — precisely so it is not sharing
    // this allocator with the other ~65 unrelated tests in this crate's unit
    // test binary. Those run concurrently by default, and every allocation
    // any of them makes on any thread bumps the *same* global atomics a
    // peak-tracking allocator would use, so a peak-bytes assertion measured
    // here would be flaky by construction (confirmed while developing this
    // test: it failed intermittently under the full `cargo test -p markua`
    // suite and passed reliably under `--test-threads=1`). A plain
    // allocation *count* delta (as used below, mirroring
    // `rst-fmt`'s `test_writer_no_subtree_reconstruction_blowup`) is far less
    // sensitive to that noise in practice, so it stays here.
    struct CountingAlloc;
    static ALLOCS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    unsafe impl std::alloc::GlobalAlloc for CountingAlloc {
        unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            unsafe { std::alloc::System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
            unsafe { std::alloc::System.dealloc(ptr, layout) }
        }
    }
    #[global_allocator]
    static GLOBAL: CountingAlloc = CountingAlloc;

    /// Regression guard against reintroducing per-block `Block`/`Inline`
    /// subtree reconstruction (the original defect: `Writer::write_event`
    /// buffered every event into a `Vec<OwnedMarkuaEvent>` and only ran
    /// `events_to_doc` + `emit::emit` inside `finish()`). A large,
    /// deeply-nested event stream must complete with an allocation count
    /// that stays close to linear in event count, not blow up the way full
    /// tree materialization would.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use std::borrow::Cow;
        use std::sync::atomic::Ordering;

        fn events_for(n: usize) -> Vec<OwnedMarkuaEvent> {
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(OwnedMarkuaEvent::StartHeading { level: 2 });
                evs.push(OwnedMarkuaEvent::Text(Cow::Owned(format!("Section {i}"))));
                evs.push(OwnedMarkuaEvent::EndHeading);
                evs.push(OwnedMarkuaEvent::StartParagraph);
                evs.push(OwnedMarkuaEvent::Text(Cow::Owned("plain ".to_string())));
                evs.push(OwnedMarkuaEvent::StartStrong);
                evs.push(OwnedMarkuaEvent::Text(Cow::Owned("bold".to_string())));
                evs.push(OwnedMarkuaEvent::EndStrong);
                evs.push(OwnedMarkuaEvent::EndParagraph);
                evs.push(OwnedMarkuaEvent::StartList { ordered: false });
                for j in 0..2 {
                    evs.push(OwnedMarkuaEvent::StartListItem);
                    evs.push(OwnedMarkuaEvent::StartParagraph);
                    evs.push(OwnedMarkuaEvent::Text(Cow::Owned(format!("item {j}"))));
                    evs.push(OwnedMarkuaEvent::EndParagraph);
                    evs.push(OwnedMarkuaEvent::EndListItem);
                }
                evs.push(OwnedMarkuaEvent::EndList);
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
}
