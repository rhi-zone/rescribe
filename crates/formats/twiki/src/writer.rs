//! Streaming TWiki writer — converts a stream of events directly to TWiki
//! text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`]. It is a second, independent
//! emission path from the tree-based builder, not a thin wrapper around it.
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document, plus a small `plain` buffer used only inside `Link`
//! (see below). Frames on the `Vec<Frame>` stack (`O(nesting depth)`) hold
//! only small metadata — a `usize` mark, a closing delimiter, a bool —
//! never a copy of accumulated content. Almost every TWiki construct is
//! **write-through**: the heading marker depends only on `level` (not text
//! length, unlike RST's underline), table cells carry no column-width
//! alignment, and list-item markers depend only on the enclosing `List`'s
//! `ordered` flag and the current nesting depth (both known at `Start*`).
//!
//! **`Link` is the one construct with a dedicated side buffer**:
//! `Inline::Link`'s `label` is a flat `String` in the AST (the parser never
//! nests markup inside a link label, and `events()` always re-emits it as a
//! single `Text` event), but the streaming `Event` vocabulary still models
//! `Link` as a `Start`/`End` pair with arbitrary content between them, like
//! every other inline span — so `Writer` handles the fully general case
//! defensively: while a `Link` is open, `push_out` is suppressed
//! (`link_depth > 0`) and every leaf event's plain-text contribution is
//! appended to `Writer::plain` instead (mirroring `collect_inline_text`'s
//! recursion — delimiters never contribute; a nested `Link`'s own
//! contribution to an *outer* link's label is just its own label, matching
//! `collect_inline_text`'s `Inline::Link { label, .. } => s.push_str(label)`
//! arm). At `EndLink` that accumulated slice becomes the label, written out
//! in one shot. `O(that link's own content)`, not `O(document)`.
//!
//! Constructs whose validity depends on their *enclosing* frame (e.g. a
//! `TableCell` only makes sense directly inside a `TableRow`; a `ListItem`
//! only renders a `List` child, per `emit::build_list_items`'s filter — see
//! its doc comment) are written optimistically and rolled back with
//! `out.truncate(mark)` once the construct closes, if the frame left on top
//! of the stack doesn't accept it.
//!
//! Each top-level block is flushed to the sink and `out` is cleared
//! (capacity retained) as soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.

use crate::events::Event;
use std::io::Write;

/// Frames carry only a mark into the shared output buffer plus tiny
/// scalars — never accumulated content.
enum Frame {
    Paragraph {
        mark: usize,
    },
    Heading {
        mark: usize,
    },
    /// `build_list_items` indents by `"   ".repeat(depth)` where the
    /// outermost list starts at `depth = 1` — mirrored by `Writer::list_depth`
    /// starting at 0 and being incremented *before* this frame is pushed.
    List {
        ordered: bool,
        mark: usize,
    },
    /// Valid only directly inside `List`, and only actually renders further
    /// nested `List` children — see `emit::build_list_items`'s doc comment
    /// on this module: any other block kind nested in a list item is
    /// structurally accepted by the AST but never visited by the emitter,
    /// so it is dropped here too (same final bytes, no tree built to hold
    /// it in the meantime).
    ListItem {
        mark: usize,
        /// `build_list_items` writes exactly one `'\n'` per item, right
        /// after its own `inlines` and *before* recursing into any nested
        /// `List` child — not after the nested content too. So this is set
        /// the moment a nested `List` child's `Start` event arrives (which
        /// is also when that `'\n'` is written), and `EndListItem` only
        /// writes it itself if no nested list showed up.
        wrote_own_line: bool,
    },
    Table {
        mark: usize,
    },
    /// Valid only directly inside `Table`.
    TableRow {
        mark: usize,
    },
    /// Valid only directly inside `TableRow`.
    TableCell {
        mark: usize,
        is_header: bool,
    },
    DefinitionList {
        mark: usize,
    },
    /// Valid only directly inside `DefinitionList`.
    DefinitionTerm {
        mark: usize,
    },
    /// Valid only directly inside `DefinitionList`, and only meaningful
    /// immediately after a `DefinitionTerm` — the parser always emits them
    /// as an adjacent pair, so no separate "awaiting desc" state is tracked.
    DefinitionDesc {
        mark: usize,
    },
    Blockquote {
        mark: usize,
    },
    /// Any inline span whose closing delimiter is a fixed string known when
    /// the span opens (bold/italic/bold-italic/strikethrough/superscript/
    /// subscript/underline/bold-code).
    Inline {
        close: &'static str,
        mark: usize,
    },
    /// See the module doc comment: while this frame is on the stack,
    /// `link_depth > 0` suppresses `out` writes and routes leaf content into
    /// `Writer::plain` instead.
    Link {
        mark: usize,
        /// Where this link's own plain-text label begins in `Writer::plain`.
        plain_mark: usize,
    },
}

/// Default capacity reserved for `Writer::out`. See `rst_fmt::writer`'s
/// identical constant for the rationale.
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Streaming TWiki writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to recover the sink once all events have been
/// fed.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Cleared (capacity retained) after
    /// each top-level block is flushed.
    out: String,
    /// Accumulates leaf plain text while `link_depth > 0` — see the module
    /// doc comment. A frame records a mark and truncates back to it once
    /// its own label has been consumed.
    plain: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level — a block closing with an empty stack
    /// is flushed to the sink immediately.
    stack: Vec<Frame>,
    /// Current list nesting depth — 0 outside any list, 1 inside the
    /// outermost, mirroring `build_list_items`'s `depth` parameter (which
    /// starts at 1 for the top-level call).
    list_depth: usize,
    /// Count of currently-open `Link` frames. While `> 0`, `push_out`
    /// suppresses writes to `out` (only the eventual `label` is written, at
    /// `EndLink`) and leaf events append their plain-text contribution to
    /// `plain` instead of `out`.
    link_depth: usize,
}

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
            plain: String::new(),
            stack: Vec::new(),
            list_depth: 0,
            link_depth: 0,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: Event<'_>) {
        self.process(event);
    }

    /// Recover the underlying sink. Does not write anything beyond flushing
    /// any bytes left in the buffer — every completed top-level block was
    /// already flushed by `write_event`.
    pub fn finish(mut self) -> W {
        self.flush();
        self.sink
    }

    // ── Buffer primitives ─────────────────────────────────────────────────

    /// Append to the shared output buffer — suppressed while inside a
    /// `Link` (see the module doc comment).
    fn push_out(&mut self, s: &str) {
        if self.link_depth == 0 {
            self.out.push_str(s);
        }
    }

    /// Flush the completed top-level block to the sink and reset the
    /// buffer, keeping its capacity.
    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Whether the top-of-stack frame accepts an ordinary block child —
    /// mirrors `DocBuilder::push_block`'s match arms (`Document`,
    /// `Blockquote`; `ListItem` is handled separately by
    /// [`Writer::accepts_list`] since it only renders nested `List`s).
    fn accepts_generic_block(&self) -> bool {
        matches!(self.stack.last(), None | Some(Frame::Blockquote { .. }))
    }

    /// Whether the top-of-stack frame accepts a `List` child specifically —
    /// `ListItem` accepts nested lists (rendered by
    /// `build_list_items`'s recursion) even though it drops every other
    /// block kind.
    fn accepts_list(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote { .. } | Frame::ListItem { .. })
        )
    }

    /// Whether the top-of-stack frame accepts inline content — mirrors
    /// `DocBuilder::push_inline`'s match arms.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::TableCell { .. }
                    | Frame::DefinitionTerm { .. }
                    | Frame::DefinitionDesc { .. }
                    | Frame::ListItem { .. }
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
            )
        )
    }

    fn block_end_generic(&mut self, mark: usize) {
        if !self.accepts_generic_block() {
            self.out.truncate(mark);
            return;
        }
        if self.stack.is_empty() {
            self.flush();
        }
    }

    fn open_span(&mut self, open: &str, close: &'static str) {
        let mark = self.out.len();
        self.push_out(open);
        self.stack.push(Frame::Inline { close, mark });
    }

    fn close_span(&mut self) {
        if let Some(Frame::Inline { close, mark }) = self.stack.pop() {
            self.push_out(close);
            if !self.accepts_inline() {
                self.out.truncate(mark);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event<'_>) {
        match event {
            // ── Block open/close ────────────────────────────────────────
            Event::StartParagraph => {
                let mark = self.out.len();
                self.stack.push(Frame::Paragraph { mark });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    self.push_out("\n\n");
                    self.block_end_generic(mark);
                }
            }
            Event::StartHeading { level } => {
                let mark = self.out.len();
                self.push_out("---");
                for _ in 0..(level as usize).min(6) {
                    self.push_out("+");
                }
                self.push_out(" ");
                self.stack.push(Frame::Heading { mark });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end_generic(mark);
                }
            }
            Event::StartList { ordered } => {
                let mark = self.out.len();
                // If this list is a `ListItem`'s nested child, its parent
                // item's own trailing newline hasn't been written yet
                // (`EndListItem` only writes it when no nested list
                // arrives) — write it now, right before this list's own
                // content, matching `build_list_items`'s per-item ordering.
                if let Some(Frame::ListItem { wrote_own_line, .. }) = self.stack.last_mut()
                    && !*wrote_own_line
                {
                    *wrote_own_line = true;
                    self.push_out("\n");
                }
                self.list_depth += 1;
                self.stack.push(Frame::List { ordered, mark });
            }
            Event::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.list_depth -= 1;
                    // Nested lists (a `List` inside a `ListItem`) don't get
                    // their own trailing blank line — only the outermost
                    // `Block::List`'s `build_block` arm does, since nested
                    // lists are rendered entirely inside
                    // `build_list_items`'s recursion.
                    let nested = matches!(self.stack.last(), Some(Frame::ListItem { .. }));
                    if !nested {
                        self.push_out("\n");
                    }
                    if self.accepts_list() {
                        if self.stack.is_empty() {
                            self.flush();
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartListItem => {
                let mark = self.out.len();
                if let Some(Frame::List { ordered, .. }) = self.stack.last() {
                    let ordered = *ordered;
                    for _ in 0..self.list_depth {
                        self.push_out("   ");
                    }
                    self.push_out(if ordered { "1. " } else { "* " });
                }
                self.stack.push(Frame::ListItem {
                    mark,
                    wrote_own_line: false,
                });
            }
            Event::EndListItem => {
                if let Some(Frame::ListItem {
                    mark,
                    wrote_own_line,
                }) = self.stack.pop()
                {
                    if matches!(self.stack.last(), Some(Frame::List { .. })) {
                        if !wrote_own_line {
                            self.push_out("\n");
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::CodeBlock { content } => {
                let mark = self.out.len();
                self.push_out("<verbatim>\n");
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("</verbatim>\n\n");
                self.block_end_generic(mark);
            }
            Event::HorizontalRule => {
                let mark = self.out.len();
                self.push_out("---\n\n");
                self.block_end_generic(mark);
            }
            Event::RawBlock { content } => {
                let mark = self.out.len();
                self.push_out(&content);
                self.push_out("\n\n");
                self.block_end_generic(mark);
            }
            Event::StartTable => {
                let mark = self.out.len();
                self.stack.push(Frame::Table { mark });
            }
            Event::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end_generic(mark);
                }
            }
            Event::StartTableRow => {
                let mark = self.out.len();
                self.push_out("|");
                self.stack.push(Frame::TableRow { mark });
            }
            Event::EndTableRow => {
                if let Some(Frame::TableRow { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    if !matches!(self.stack.last(), Some(Frame::Table { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartTableCell { is_header } => {
                let mark = self.out.len();
                self.push_out(" ");
                if is_header {
                    self.push_out("*");
                }
                self.stack.push(Frame::TableCell { mark, is_header });
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell { mark, is_header }) = self.stack.pop() {
                    if is_header {
                        self.push_out("*");
                    }
                    self.push_out(" |");
                    if !matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartDefinitionList => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionList { mark });
            }
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end_generic(mark);
                }
            }
            Event::StartDefinitionTerm => {
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                    self.push_out("   $ ");
                }
                self.stack.push(Frame::DefinitionTerm { mark });
            }
            Event::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.push_out(": ");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartDefinitionDesc => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionDesc { mark });
            }
            Event::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.push_out("\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartBlockquote => {
                let mark = self.out.len();
                self.push_out("<blockquote>\n");
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.push_out("</blockquote>\n\n");
                    self.block_end_generic(mark);
                }
            }

            // ── Inline leaf events ───────────────────────────────────────
            Event::Text(cow) => {
                if self.link_depth > 0 {
                    self.plain.push_str(&cow);
                } else if self.accepts_inline() {
                    self.out.push_str(&cow);
                }
            }
            Event::LineBreak => {
                if self.link_depth > 0 {
                    self.plain.push('\n');
                } else if self.accepts_inline() {
                    self.out.push_str("%BR%");
                }
            }
            Event::InlineCode(cow) => {
                if self.link_depth > 0 {
                    self.plain.push_str(&cow);
                } else if self.accepts_inline() {
                    self.out.push('=');
                    self.out.push_str(&cow);
                    self.out.push('=');
                }
            }
            Event::Image { url, alt } => {
                if self.link_depth > 0 {
                    self.plain.push_str(&url);
                } else if self.accepts_inline() {
                    self.out.push_str("<img src=\"");
                    self.out.push_str(&url);
                    self.out.push('"');
                    if !alt.is_empty() {
                        self.out.push_str(" alt=\"");
                        self.out.push_str(&alt);
                        self.out.push('"');
                    }
                    self.out.push_str(" />");
                }
            }
            Event::RawInline { content } => {
                if self.link_depth > 0 {
                    self.plain.push_str(&content);
                } else if self.accepts_inline() {
                    self.out.push_str(&content);
                }
            }
            Event::WikiWord { word } => {
                if self.link_depth > 0 {
                    self.plain.push_str(&word);
                } else if self.accepts_inline() {
                    self.out.push_str(&word);
                }
            }

            // ── Inline spans ─────────────────────────────────────────────
            Event::StartBold => self.open_span("*", "*"),
            Event::EndBold => self.close_span(),
            Event::StartItalic => self.open_span("_", "_"),
            Event::EndItalic => self.close_span(),
            Event::StartBoldItalic => self.open_span("__", "__"),
            Event::EndBoldItalic => self.close_span(),
            Event::StartStrikethrough => self.open_span("<del>", "</del>"),
            Event::EndStrikethrough => self.close_span(),
            Event::StartSuperscript => self.open_span("<sup>", "</sup>"),
            Event::EndSuperscript => self.close_span(),
            Event::StartSubscript => self.open_span("<sub>", "</sub>"),
            Event::EndSubscript => self.close_span(),
            Event::StartUnderline => self.open_span("<u>", "</u>"),
            Event::EndUnderline => self.close_span(),
            Event::StartBoldCode => self.open_span("==", "=="),
            Event::EndBoldCode => self.close_span(),

            Event::StartLink { url } => {
                let mark = self.out.len();
                self.push_out("[[");
                self.push_out(&url);
                self.push_out("][");
                let plain_mark = self.plain.len();
                self.link_depth += 1;
                self.stack.push(Frame::Link { mark, plain_mark });
            }
            Event::EndLink => {
                if let Some(Frame::Link { mark, plain_mark }) = self.stack.pop() {
                    self.link_depth -= 1;
                    let label = self.plain[plain_mark..].to_string();
                    self.plain.truncate(plain_mark);
                    self.push_out(&label);
                    self.push_out("]]");
                    if self.link_depth > 0 {
                        // Nested inside an outer Link: contribute just this
                        // link's own label text to the outer label, mirroring
                        // `collect_inline_text`'s `Inline::Link { label, .. }
                        // => s.push_str(label)` arm.
                        self.plain.push_str(&label);
                    } else if !self.accepts_inline() {
                        self.out.truncate(mark);
                    }
                }
            }
        }
    }
}

/// Shared allocator instrumentation for the memory-guard tests below. Only
/// one `#[global_allocator]` may exist per test binary (all `#[cfg(test)]`
/// items compile into one binary), so both the allocation-count test and the
/// peak-memory test share this single allocator and its counters.
#[cfg(test)]
mod alloc_guard {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub(super) static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static CURRENT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static PEAK: AtomicUsize = AtomicUsize::new(0);

    /// The two memory-guard tests below both read process-wide allocator
    /// counters, so they must not run concurrently with each other (cargo
    /// test runs test functions in parallel threads by default) — each
    /// would pollute the other's counts. Ordinary tests elsewhere in this
    /// binary are cheap enough not to meaningfully disturb the large-N
    /// measurements here.
    pub(super) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    pub(super) struct InstrumentedAlloc;

    unsafe impl GlobalAlloc for InstrumentedAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            let cur = CURRENT.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK.fetch_max(cur, Ordering::SeqCst);
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
            unsafe { System.dealloc(ptr, layout) }
        }
    }
}

#[cfg(test)]
#[global_allocator]
static ALLOC_GUARD: alloc_guard::InstrumentedAlloc = alloc_guard::InstrumentedAlloc;

#[cfg(test)]
mod tests {
    use super::alloc_guard::{ALLOCS, CURRENT, PEAK, TEST_LOCK};
    use super::*;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartHeading { level: 1 });
        w.write_event(Event::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(Event::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("---+ Hello"), "got: {s:?}");
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
        let input =
            "---+ Hello\n\nThis is a paragraph with *bold* text.\n\n   * item one\n   * item two\n";
        let (doc, _) = crate::parse::parse(input);
        let evts: Vec<_> = crate::events::events(&doc).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    /// The streaming `Writer` must produce *byte-identical* output to the
    /// tree-based `emit::build()` for the same document.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "---+ Title\n\nIntro paragraph with *bold* and _italic_.\n",
            "---++ Sub\n\ntext with __bold italic__ and ==bold code==.\n",
            "   * bullet one\n   * bullet two\n",
            "   1. ordered one\n   1. ordered two\n",
            "<verbatim>\ncode block\nline two\n</verbatim>\n",
            "<blockquote>\nquoted paragraph text\n</blockquote>\n",
            "| *A* | *B* |\n| Cell 1 | Cell 2 |\n",
            "---\n\nAfter the transition.\n",
            "[[https://example.com][click here]]\n",
            "A paragraph with an =inline code= span.\n",
            "Text with <sup>super</sup> and <sub>sub</sub>.\n",
            "Text with <u>underline</u> and <del>strike</del>.\n",
            "   * outer one\n   * outer two\n\n      * inner a\n      * inner b\n",
            "   $ term one: definition one\n   $ term two: definition two\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input);
            let built = crate::emit::build(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events::events(&doc) {
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

    /// `Inline::Link`'s `label` is already a flat `String` in the AST (the
    /// parser doesn't nest markup inside a link label), and `events()`
    /// re-emits it as a single `Text` event between `StartLink`/`EndLink` —
    /// so this exercises `Writer`'s `plain`-buffer machinery for the
    /// (only reachable, but still worth covering directly) case of a
    /// single-`Text`-child link, proving it reconstructs the label via
    /// `Writer::plain` rather than writing straight through like other
    /// inline spans.
    #[test]
    fn test_writer_link_label_roundtrips() {
        let (doc, _) = crate::parse::parse("[[https://example.com][click here]]\n");
        let built = crate::emit::build(&doc);

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events(&doc) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).unwrap();

        assert_eq!(built, streamed);
        assert!(
            streamed.contains("[[https://example.com][click here]]"),
            "got: {streamed:?}"
        );
    }

    /// Regression guard against reintroducing per-block tree reconstruction:
    /// a large, deeply-nested event stream must complete with an allocation
    /// count that stays close to linear in event count, not blow up the way
    /// tree materialization would.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use std::borrow::Cow;
        use std::sync::atomic::Ordering;

        let _guard = TEST_LOCK.lock().unwrap();

        fn build_events(n: usize) -> Vec<Event<'static>> {
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(Event::StartHeading { level: 2 });
                evs.push(Event::Text(Cow::Owned(format!("Section {i}"))));
                evs.push(Event::EndHeading);
                evs.push(Event::StartParagraph);
                evs.push(Event::Text(Cow::Owned("plain ".to_string())));
                evs.push(Event::StartBold);
                evs.push(Event::Text(Cow::Owned("bold".to_string())));
                evs.push(Event::EndBold);
                evs.push(Event::EndParagraph);
                evs.push(Event::StartList { ordered: false });
                for j in 0..2 {
                    evs.push(Event::StartListItem);
                    evs.push(Event::Text(Cow::Owned(format!("item {j}"))));
                    evs.push(Event::EndListItem);
                }
                evs.push(Event::EndList);
            }
            evs
        }

        fn run(n: usize) -> usize {
            let before = ALLOCS.load(Ordering::Relaxed);
            let evs = build_events(n);
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

    /// Peak-memory guard: feeding a large synthetic document through
    /// `Writer` must keep peak allocated bytes within a small constant
    /// multiple of a single paragraph's size, not `O(full document)`.
    #[test]
    fn test_writer_peak_memory_bounded() {
        use std::borrow::Cow;
        use std::sync::atomic::Ordering;

        let _guard = TEST_LOCK.lock().unwrap();

        /// A sink that counts written bytes without retaining them — using
        /// `Vec<u8>` as the sink would conflate the sink's own inevitable
        /// growth to the final document size with the writer's internal
        /// buffering.
        struct CountingSink(usize);
        impl Write for CountingSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0 += buf.len();
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        const N: usize = 20_000;

        // Never reset `CURRENT` — it tracks every live allocation in the
        // whole test binary (other tests' data may still be outstanding).
        // Resetting it to 0 would make an unrelated later `dealloc` of
        // pre-existing memory underflow the counter. Instead take a
        // baseline and measure the *rise* above it.
        let baseline = CURRENT.load(Ordering::SeqCst);
        PEAK.store(baseline, Ordering::SeqCst);

        let mut w = Writer::new(CountingSink(0));
        for i in 0..N {
            w.write_event(Event::StartParagraph);
            w.write_event(Event::Text(Cow::Owned(format!(
                "paragraph number {i} with some filler text to give it realistic size"
            ))));
            w.write_event(Event::EndParagraph);
        }
        let sink = w.finish();
        std::hint::black_box(&sink);

        let peak = PEAK.load(Ordering::SeqCst).saturating_sub(baseline);
        let total_doc_bytes = sink.0;

        assert!(
            (peak as f64) < (total_doc_bytes as f64) * 0.25,
            "peak allocated bytes ({peak}) is not small relative to total document size \
             ({total_doc_bytes}) — writer may be buffering the whole document instead of \
             flushing per top-level block"
        );
    }
}
