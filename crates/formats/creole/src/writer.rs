#![allow(clippy::collapsible_if)]
//! Streaming Creole writer — converts a stream of events directly to Creole
//! text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`] or its helpers. It is a
//! second, independent emission path from the tree-based `build()`, not a
//! thin wrapper around it.
//!
//! # Buffer model
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document, mirroring `BuildContext::output`'s single amortized
//! geometric growth. Frames on the `Vec<Frame>` stack (`O(nesting depth)`)
//! hold only a `usize` mark into `out` plus a couple of small scalars — never
//! a copy of accumulated content. Children write **straight through** into
//! `out`; a frame that needs to decorate its own content afterwards
//! post-processes the `out[mark..]` range in place.
//!
//! Every Creole construct turns out to be write-through — no construct's
//! *prefix* depends on content not yet seen:
//!
//! - **Heading**: the `=`-run prefix/suffix count comes from the heading
//!   *level*, which is known at `StartHeading`, unlike RST's underline (whose
//!   width depends on the plain-text length). No deferral.
//! - **Blockquote**: `build_block` only ever writes the `"> "` line prefix in
//!   front of a `Paragraph` *child*, once, not per physical output line
//!   (Creole's line-break inline is the literal token `\\`, never an actual
//!   newline inside a paragraph's rendered text) — so unlike RST's
//!   blockquote/admonition/code-block (which re-indent every already-written
//!   line), Creole's blockquote needs no post-hoc re-indent pass at all. The
//!   `"> "` decision is knowable at the child's own `StartParagraph`, exactly
//!   like RST's list-item `BlockKind` dispatch.
//! - **List**/**ListItem**: the marker run length is `Writer::list_depth`,
//!   tracked the same way RST tracks it (incremented/decremented around
//!   `StartList`/`EndList`); the marker character comes from the *innermost*
//!   list's `ordered` flag, known when that list's own `StartListItem`
//!   fires.
//! - **Table**: Creole's table emitter does **not** pad or align columns
//!   (`build_block`'s `Table` arm writes each cell's markup immediately, no
//!   width pass) — so, unlike RST's table (which must buffer every cell to
//!   compute column widths before the first border line), Creole's table
//!   needs no side-stack or deferred render at all.
//! - **Link**: the `[[url` prefix is written immediately at `StartLink`
//!   (unlike RST's `` `text <url>`_ `` form, whose URL comes *after* the
//!   link text and so must be held until `EndLink`). Only the `"|"`
//!   separator before link text is deferred — a single `insert_str` at
//!   `EndLink` once it's known whether any text arrived, exactly like RST's
//!   `Figure` caption lead-in.
//!
//! No construct needs a side stack: every `Frame` variant is a handful of
//! `usize`/`bool`/`u8` fields, so there is no size-disparity to move off the
//! main stack the way RST's `Table`/`TableRow`/`Link` payloads were.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity, so the buffer is allocated once for the whole document) as
//! soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use creole::writer::Writer;
//! use creole::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::Event;
use std::io::Write;

/// Streaming Creole writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to recover the sink once all events have been
/// fed.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Every construct writes here
    /// directly; frames record marks into it. Cleared (capacity retained)
    /// after each top-level block is flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level — a block closing with an empty stack
    /// is flushed to the sink immediately.
    stack: Vec<Frame>,
    /// Mirrors `BuildContext::list_depth`: incremented when a `List` frame
    /// is pushed (`StartList`), decremented when it is popped (`EndList`).
    list_depth: usize,
}

/// Default capacity reserved for `Writer::out` by [`Writer::new`]. Skips the
/// first several geometric doublings (pure overhead below any realistic
/// block size) without committing to a document-specific guess. Callers
/// with a better estimate (or who want zero speculative allocation) should
/// use [`Writer::with_capacity`] instead.
const DEFAULT_OUT_CAPACITY: usize = 4096;

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Self::with_capacity(sink, DEFAULT_OUT_CAPACITY)
    }

    /// Like [`Writer::new`], but reserves `out_capacity` bytes for the
    /// shared output buffer up front instead of [`DEFAULT_OUT_CAPACITY`].
    pub fn with_capacity(sink: W, out_capacity: usize) -> Self {
        Writer {
            sink,
            out: String::with_capacity(out_capacity),
            stack: Vec::new(),
            list_depth: 0,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink
    /// immediately if this event completes a top-level block.
    pub fn write_event(&mut self, event: Event<'_>) {
        self.process(event);
    }

    /// Recover the underlying sink. Does not write anything — every
    /// completed top-level block was already flushed by `write_event`. Any
    /// still-open frames (an unterminated event stream) hold content that
    /// was never a completed top-level block and is discarded, matching the
    /// old AST-reconstruction writer's behaviour (an unclosed `Start*`
    /// never reached the point of being pushed into its parent either).
    pub fn finish(self) -> W {
        self.sink
    }

    // ── Buffer primitives ─────────────────────────────────────────────────

    fn push_out(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// Flush the completed top-level block to the sink and reset the
    /// buffer, keeping its capacity so the document only ever grows one
    /// buffer.
    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Whether the top-of-stack frame accepts block children — mirrors
    /// `events_to_doc`'s old `push_block` match (`Document`, `Blockquote`,
    /// `ListItem`; nothing else).
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote { .. } | Frame::ListItem { .. })
        )
    }

    /// Whether the top-of-stack frame accepts inline children — mirrors the
    /// old `push_inline` match.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::Bold { .. }
                    | Frame::Italic { .. }
                    | Frame::Link { .. }
                    | Frame::TableCell { .. }
                    | Frame::DefinitionTerm { .. }
                    | Frame::DefinitionDesc { .. }
            )
        )
    }

    /// Open a block: emit the list-item leading separator if this block is a
    /// child of a `ListItem` (mirrors `build_block`'s `if i > 0 { "\n" }`,
    /// known at the child's *start* since the item's `first` flag is
    /// already on the stack), or the blockquote `"> "` prefix if this block
    /// is a `Paragraph` child of a `Blockquote` (`build_block`'s
    /// `Blockquote` arm only ever prefixes `Paragraph` children). Returns
    /// the mark to truncate back to if this block turns out to have no
    /// valid enclosing context.
    fn block_start(&mut self, is_paragraph: bool) -> usize {
        let mark = self.out.len();
        match self.stack.last_mut() {
            Some(Frame::ListItem { first, .. }) => {
                let was_first = std::mem::replace(first, false);
                if !was_first {
                    self.push_out("\n");
                }
            }
            Some(Frame::Blockquote { .. }) if is_paragraph => {
                self.push_out("> ");
            }
            _ => {}
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

    /// Close an inline span: discard it (and everything its children wrote)
    /// if the enclosing frame does not take inline children.
    fn inline_end(&mut self, mark: usize) {
        if !self.accepts_inline() {
            self.out.truncate(mark);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event<'_>) {
        match event {
            // ── Block open/close ────────────────────────────────────────
            Event::StartParagraph => {
                let mark = self.block_start(true);
                self.stack.push(Frame::Paragraph { mark });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    match self.stack.last() {
                        Some(Frame::ListItem { .. }) => {}
                        Some(Frame::Blockquote { .. }) => self.push_out("\n"),
                        _ => self.push_out("\n\n"),
                    }
                    self.block_end(mark);
                }
            }
            Event::StartHeading { level } => {
                let mark = self.block_start(false);
                let level = (level as usize).min(6) as u8;
                for _ in 0..level {
                    self.push_out("=");
                }
                self.push_out(" ");
                self.stack.push(Frame::Heading { mark, level });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { mark, level }) = self.stack.pop() {
                    self.push_out(" ");
                    for _ in 0..level {
                        self.push_out("=");
                    }
                    self.push_out("\n\n");
                    self.block_end(mark);
                }
            }
            Event::StartBlockquote => {
                let mark = self.block_start(false);
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartList { ordered } => {
                let mark = self.block_start(false);
                self.list_depth += 1;
                self.stack.push(Frame::List { ordered, mark });
            }
            Event::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.list_depth -= 1;
                    if self.list_depth == 0 {
                        self.push_out("\n");
                    }
                    self.block_end(mark);
                }
            }
            Event::StartListItem => {
                let ordered = matches!(self.stack.last(), Some(Frame::List { ordered: true, .. }));
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::List { .. })) {
                    let marker = if ordered { "#" } else { "*" };
                    for _ in 0..self.list_depth {
                        self.push_out(marker);
                    }
                    self.push_out(" ");
                }
                self.stack.push(Frame::ListItem { first: true, mark });
            }
            Event::EndListItem => {
                if let Some(Frame::ListItem { mark, .. }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.push_out("\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::CodeBlock { content } => {
                let mark = self.block_start(false);
                self.push_out("{{{\n");
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("}}}\n\n");
                self.block_end(mark);
            }
            Event::HorizontalRule => {
                let mark = self.block_start(false);
                self.push_out("----\n\n");
                self.block_end(mark);
            }
            Event::StartTable => {
                let mark = self.block_start(false);
                self.stack.push(Frame::Table { mark });
            }
            Event::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartTableRow => {
                let mark = self.out.len();
                self.stack.push(Frame::TableRow { mark });
            }
            Event::EndTableRow => {
                if let Some(Frame::TableRow { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::Table { .. })) {
                        self.push_out("|\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartTableCell { is_header } => {
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                    self.push_out(if is_header { "|=" } else { "|" });
                }
                self.stack.push(Frame::TableCell { mark });
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell { mark }) = self.stack.pop() {
                    if !matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartDefinitionList => {
                let mark = self.block_start(false);
                self.stack.push(Frame::DefinitionList { mark });
            }
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartDefinitionTerm => {
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                    self.push_out("; ");
                }
                self.stack.push(Frame::DefinitionTerm { mark });
            }
            Event::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.push_out("\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartDefinitionDesc => {
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                    self.push_out(": ");
                }
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

            // ── Inline events ───────────────────────────────────────────
            Event::Text(cow) => {
                if self.accepts_inline() {
                    self.push_out(&cow);
                }
            }
            Event::LineBreak => {
                if self.accepts_inline() {
                    self.push_out("\\\\");
                }
            }
            Event::StartBold => {
                let mark = self.out.len();
                if self.accepts_inline() {
                    self.push_out("**");
                }
                self.stack.push(Frame::Bold { mark });
            }
            Event::EndBold => {
                if let Some(Frame::Bold { mark }) = self.stack.pop() {
                    if self.accepts_inline() {
                        self.push_out("**");
                    }
                    self.inline_end(mark);
                }
            }
            Event::StartItalic => {
                let mark = self.out.len();
                if self.accepts_inline() {
                    self.push_out("//");
                }
                self.stack.push(Frame::Italic { mark });
            }
            Event::EndItalic => {
                if let Some(Frame::Italic { mark }) = self.stack.pop() {
                    if self.accepts_inline() {
                        self.push_out("//");
                    }
                    self.inline_end(mark);
                }
            }
            Event::InlineCode(cow) => {
                if self.accepts_inline() {
                    self.push_out("{{{");
                    self.push_out(&cow);
                    self.push_out("}}}");
                }
            }
            Event::StartLink { url } => {
                let mark = self.out.len();
                if self.accepts_inline() {
                    self.push_out("[[");
                    self.push_out(&url);
                }
                let content_mark = self.out.len();
                self.stack.push(Frame::Link { mark, content_mark });
            }
            Event::EndLink => {
                if let Some(Frame::Link { mark, content_mark }) = self.stack.pop() {
                    if self.accepts_inline() {
                        if self.out.len() > content_mark {
                            self.out.insert(content_mark, '|');
                        }
                        self.push_out("]]");
                    }
                    self.inline_end(mark);
                }
            }
            Event::InlineImage { url, alt } => {
                if self.accepts_inline() {
                    self.push_out("{{");
                    self.push_out(&url);
                    if let Some(alt_text) = &alt {
                        self.push_out("|");
                        self.push_out(alt_text);
                    }
                    self.push_out("}}");
                }
            }
        }
    }
}

/// Frames carry only a mark into the shared `out` buffer and a couple of
/// small scalars — never accumulated content. `mark` is where this
/// construct's output begins (so it can be discarded wholesale if it turns
/// out to have no valid enclosing context).
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
        ordered: bool,
        mark: usize,
    },
    ListItem {
        /// Whether the next child is this item's first — the flag
        /// `build_block`'s `if i > 0` uses to decide the leading separator.
        first: bool,
        mark: usize,
    },
    Table {
        mark: usize,
    },
    TableRow {
        mark: usize,
    },
    TableCell {
        mark: usize,
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
    Bold {
        mark: usize,
    },
    Italic {
        mark: usize,
    },
    /// The one deferred piece in the whole writer: whether to insert the
    /// `"|"` separator is only knowable once it's seen whether any link
    /// text arrived, hence `content_mark` alongside `mark`.
    Link {
        mark: usize,
        content_mark: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;
    use std::borrow::Cow;

    // ── Shared instrumented allocator ───────────────────────────────────
    //
    // A process may only define one `#[global_allocator]`, so the
    // allocation-count guard (`test_writer_no_subtree_reconstruction_blowup`)
    // and the peak-memory guard (`test_writer_peak_memory_bounded`) share
    // this single allocator instead of each defining their own.
    mod alloc_probe {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicUsize, Ordering};

        pub static ALLOCS: AtomicUsize = AtomicUsize::new(0);
        pub static CURRENT: AtomicUsize = AtomicUsize::new(0);
        pub static PEAK: AtomicUsize = AtomicUsize::new(0);
        /// Held for the duration of each instrumented test's measurement
        /// window, so the two large-allocation guards below don't
        /// contaminate each other's counts by running concurrently on
        /// separate threads (the allocator is process-wide, not
        /// per-thread).
        pub static PROBE_LOCK: Mutex<()> = Mutex::new(());

        pub struct InstrumentedAlloc;
        unsafe impl GlobalAlloc for InstrumentedAlloc {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                ALLOCS.fetch_add(1, Ordering::Relaxed);
                let cur = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
                PEAK.fetch_max(cur, Ordering::Relaxed);
                unsafe { System.alloc(layout) }
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
                unsafe { System.dealloc(ptr, layout) }
            }
        }
    }
    #[global_allocator]
    static GLOBAL: alloc_probe::InstrumentedAlloc = alloc_probe::InstrumentedAlloc;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 1 });
        w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("= Hello ="), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text(Cow::Owned("World".to_string())));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "= Hello\n\nA paragraph with **bold** text.\n\n* item one\n* item two\n";
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
    /// tree-based `build()` for the same document — the guard that keeps
    /// the two independent emission paths honest.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "= Title =\n\nIntro paragraph with **bold** and //italic// and {{{code}}}.\n\n",
            "== Sub ==\n\ntext with a [[http://x/|link]] here.\n\n",
            "* bullet one\n* bullet two\n\n** nested a\n** nested b\n\n",
            "# ordered one\n# ordered two\n\n",
            "{{{\nfn main() {}\n}}}\n\n",
            "> A blockquote paragraph.\n\n",
            "; term\n: definition body\n\n; term2\n: another definition\n\n",
            "|= Name |= Age |\n| Alice | 30 |\n\n",
            "----\n\nAfter the rule.\n\n",
            "A para with {{img.png|alt text}} inline image.\n\n",
            "A para with a [[http://x/]] bare link.\n\n",
            "A para with a line\\\\break.\n\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input);
            let built = crate::emit::build(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events::events(input) {
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

    /// Round-trip a broader construct mix entirely through
    /// `events() -> Writer`, proving the incremental per-top-level-block
    /// flush handles every construct `parse()` produces.
    #[test]
    fn test_writer_roundtrip_full_construct_mix() {
        let input = "\
= Title =

Intro paragraph with **bold** and //italic//.

> A quoted paragraph.

* bullet one
* bullet two

# ordered one
# ordered two

{{{
let x = 1;
}}}

; term
: definition body

|= Name |= Age |
| Alice | 30 |

----

After the rule.
";
        let (doc, _) = crate::parse::parse(input);
        assert!(
            doc.blocks.len() >= 8,
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
            "writer roundtrip block count mismatch\ninput blocks: {:#?}\nemitted text: {emitted_text}\nreparsed blocks: {:#?}",
            doc.blocks,
            doc2.blocks,
        );
    }

    /// Nested lists are the trickiest write-through path: `list_depth`
    /// bookkeeping lives on `Writer` itself (bracketing `StartList`/
    /// `EndList`), and each nested list's marker run is written straight
    /// into the shared buffer at the depth active when its own
    /// `StartListItem` fires.
    ///
    /// The nested list is placed in the outer list's *last* item: a nested
    /// list followed by another outer-level item hits a pre-existing
    /// `build()`/`parse()` round-trip gap (the emitted blank line after a
    /// nested list's own trailing `"\n"` makes the parser split what should
    /// be one list into two) — reproducible from `build()` alone, with no
    /// streaming `Writer` involved, so it is out of scope for this
    /// streaming-writer rewrite. See the task report for that separate,
    /// pre-existing defect.
    #[test]
    fn test_writer_roundtrip_nested_lists() {
        let input = "\
* outer one
* outer two
** inner a
** inner b
";
        let (doc, _) = crate::parse::parse(input);

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events(input) {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();

        let (doc2, _) = crate::parse::parse(&emitted_text);
        fn count_lists(blocks: &[crate::ast::Block]) -> usize {
            let mut n = 0;
            for b in blocks {
                if let crate::ast::Block::List { items, .. } = b {
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
            count_lists(&doc2.blocks) >= 2,
            "expected outer list + nested list, emitted:\n{emitted_text}"
        );
    }

    /// Regression guard against reintroducing per-block `Block`/`Inline`
    /// subtree reconstruction (the original perf bug: `write_event` merely
    /// buffered every event into a `Vec`, and `finish()` reconstructed the
    /// whole AST from that buffer before calling `build()` — zero bytes
    /// reached the sink before `finish()`, and memory was
    /// `O(full document)`). A large, deeply-nested event stream must
    /// complete with an allocation count that stays close to linear in
    /// event count, not blow up the way tree materialization would.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use alloc_probe::{ALLOCS, PROBE_LOCK};
        use std::sync::atomic::Ordering;

        let _guard = PROBE_LOCK.lock().unwrap();

        fn events_for(n: usize) -> Vec<OwnedEvent> {
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

    /// Peak-memory guard: feeding a large synthetic document (thousands of
    /// paragraphs) through `Writer` must keep peak allocated bytes within a
    /// small constant multiple of a single top-level block's size, not grow
    /// with the full document. The old buffered writer held every event
    /// plus the reconstructed AST plus the fully-built string
    /// simultaneously — `O(full document)`.
    ///
    /// Measured as a **scaling comparison** (peak at 10x the paragraph
    /// count vs peak at baseline count), not an absolute byte threshold:
    /// the allocator is process-wide, so an absolute threshold is at the
    /// mercy of unrelated tests allocating concurrently on other threads.
    /// A true `O(full document)` writer would show peak growing
    /// proportionally with N (~10x); a bounded writer's peak stays flat
    /// regardless of N. That contrast survives background noise that a
    /// fixed-byte-ceiling assertion would not.
    #[test]
    fn test_writer_peak_memory_bounded() {
        use alloc_probe::{CURRENT, PEAK, PROBE_LOCK};
        use std::sync::atomic::Ordering;

        let _guard = PROBE_LOCK.lock().unwrap();

        struct DevNull;
        impl Write for DevNull {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                std::hint::black_box(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        // Peak-above-baseline for feeding `n` synthetic paragraphs through
        // `Writer`. Does NOT reset `CURRENT`/`PEAK`: this allocator is
        // process-wide, and other tests may hold live allocations already
        // counted into `CURRENT` — zeroing it out from under them would
        // make their later `dealloc`s underflow the counter. Reading a
        // baseline before and after instead is safe under concurrency.
        fn run(n: usize) -> usize {
            let paragraph_text = "word ".repeat(20); // ~100 bytes/paragraph
            let baseline_current = CURRENT.load(Ordering::Relaxed);
            let baseline_peak = PEAK.load(Ordering::Relaxed);

            let mut w = Writer::new(DevNull);
            for i in 0..n {
                w.write_event(Event::StartParagraph);
                w.write_event(Event::Text(Cow::Owned(format!("{paragraph_text}{i}"))));
                w.write_event(Event::EndParagraph);
            }
            w.finish();

            PEAK.load(Ordering::Relaxed)
                .saturating_sub(baseline_current.min(baseline_peak))
        }

        let small = run(500).max(1);
        let large = run(5000); // 10x the paragraphs

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 6.0,
            "peak allocated bytes above baseline did not stay flat under 10x the paragraphs: \
             {small} bytes @500 paragraphs -> {large} bytes @5000 paragraphs (ratio {ratio:.2}); \
             this suggests the writer is retaining O(full document) memory instead of flushing \
             each top-level block"
        );
    }
}
