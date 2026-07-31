//! Streaming ZimWiki writer — converts a stream of events directly to
//! ZimWiki text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`] or
//! [`crate::events::collect_doc_from_events`]. It is a second, independent
//! emission path from the tree-based `build()`, not a thin wrapper around it.
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document. Frames on the `Vec<Frame>` stack (`O(nesting depth)`) hold
//! only small metadata — a `usize` mark into `out`, a closing delimiter, a
//! counter — never a copy of accumulated content. Every ZimWiki construct is
//! **write-through**: unlike RST's heading underline (whose width depends on
//! the heading's plain-text length) or table (whose column widths depend on
//! every cell), ZimWiki's heading marker depends only on `level` (known at
//! `StartHeading`) and its table cells are written with no column-width
//! alignment at all — so no construct needs to buffer content to compute a
//! prefix. The one piece of look-ahead ZimWiki needs (whether a link has any
//! link-text children, to decide whether to emit the `|` separator) is
//! resolved with a single in-place `insert_str` once the children are known,
//! exactly like RST's figure caption lead-in.
//!
//! Constructs whose validity depends on their *enclosing* frame (e.g. a
//! `TableCell` only makes sense directly inside a `TableRow`) are written
//! optimistically and rolled back with `out.truncate(mark)` if, once the
//! construct closes, the frame that's left on top of the stack turns out not
//! to accept it — the same local, cascading-truncation check
//! `collect_doc_from_events`'s `push_block`/`push_inline` perform structurally,
//! but done in the output buffer instead of a tree.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (capacity
//! retained) as soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.

use crate::events::Event;
use std::io::Write;

/// How a `Paragraph`'s formatting depends on its immediate parent frame at
/// the point it opened — known at `StartParagraph`, so no deferral needed.
#[derive(Clone, Copy)]
enum ParaMode {
    /// Top-level (or any context other than list-item/blockquote): trailing
    /// blank line, no prefix.
    Normal,
    /// Directly inside a `ListItem`: no prefix, no trailing separator at all
    /// (consecutive paragraph children in one item are simply concatenated,
    /// mirroring `build_block`'s `List` arm).
    ListItemChild,
    /// Directly inside a `Blockquote`: `"> "` prefix, single trailing
    /// newline (mirroring `build_block`'s `Blockquote` arm).
    BlockquoteChild,
}

/// Frames carry only a mark into the shared output buffer plus tiny scalars
/// — never accumulated content.
enum Frame {
    Paragraph {
        mode: ParaMode,
        mark: usize,
    },
    Heading {
        /// Precomputed `"=".repeat(eq_count)`, reused for both the opening
        /// and closing marker — cheap enough (max 6 bytes) that storing it
        /// beats recomputing from `level` at close.
        marker: String,
        mark: usize,
    },
    Blockquote {
        mark: usize,
    },
    List {
        ordered: bool,
        /// Next ordinal to use for an unchecked ordered item. Mutated in
        /// place by `StartListItem` — mirrors `build_block`'s `let mut num`
        /// loop-local.
        num: usize,
        mark: usize,
    },
    /// Valid only directly inside `List` — checked, and rolled back if not,
    /// when the item closes (mirrors `collect_doc_from_events`'s `EndListItem`
    /// arm, which only pushes the item if its parent is `BlockFrame::List`).
    ListItem {
        mark: usize,
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
    },
    /// Any inline span whose closing delimiter is a fixed string known when
    /// the span opens (bold/italic/underline/strikethrough/sub/superscript).
    Inline {
        close: &'static str,
        mark: usize,
    },
    /// Links are the one inline span whose closing shape depends on
    /// look-ahead: the `|` separator is only emitted if link-text children
    /// actually arrived.
    Link {
        mark: usize,
        /// Where link-text content would begin, i.e. right after the URL —
        /// compared against `out.len()` at `EndLink` to tell "had text" from
        /// "had none".
        content_mark: usize,
    },
}

/// Default capacity reserved for `Writer::out`. See `rst_fmt::writer`'s
/// identical constant for the rationale: skips the first several geometric
/// doublings without committing to a document-specific guess.
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Streaming ZimWiki writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to recover the sink once all events have been
/// fed — it only flushes whatever's left in the buffer (which is empty for
/// any well-formed event stream, since every complete top-level block is
/// already flushed by `write_event`).
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Cleared (capacity retained) after
    /// each top-level block is flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level — a block closing with an empty stack
    /// is flushed to the sink immediately.
    stack: Vec<Frame>,
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
            stack: Vec::new(),
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

    fn push_out(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// Flush the completed top-level block to the sink and reset the
    /// buffer, keeping its capacity.
    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Whether the top-of-stack frame accepts block children — mirrors
    /// `collect_doc_from_events`'s `push_block`, which only inserts into
    /// `Document`, `Blockquote`, and `ListItem`.
    fn accepts_block(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote { .. } | Frame::ListItem { .. })
        )
    }

    /// Whether the top-of-stack frame accepts inline children — mirrors
    /// `push_inline`, which inserts into an open inline frame if one exists,
    /// else `Paragraph`/`Heading`/`TableCell`.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::TableCell { .. }
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
            )
        )
    }

    /// Close a block: discard it if the enclosing frame does not take block
    /// children, otherwise flush if it completed a top-level block.
    fn block_end(&mut self, mark: usize) {
        if !self.accepts_block() {
            self.out.truncate(mark);
            return;
        }
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Close an inline span: discard it if the enclosing frame does not take
    /// inline children.
    fn inline_end(&mut self, mark: usize) {
        if !self.accepts_inline() {
            self.out.truncate(mark);
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
            self.inline_end(mark);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event<'_>) {
        match event {
            // ── Block open/close ────────────────────────────────────────
            Event::StartParagraph => {
                let mark = self.out.len();
                let mode = match self.stack.last() {
                    Some(Frame::ListItem { .. }) => ParaMode::ListItemChild,
                    Some(Frame::Blockquote { .. }) => ParaMode::BlockquoteChild,
                    _ => ParaMode::Normal,
                };
                if matches!(mode, ParaMode::BlockquoteChild) {
                    self.push_out("> ");
                }
                self.stack.push(Frame::Paragraph { mode, mark });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph { mode, mark }) = self.stack.pop() {
                    match mode {
                        ParaMode::Normal => self.push_out("\n\n"),
                        ParaMode::BlockquoteChild => self.push_out("\n"),
                        ParaMode::ListItemChild => {}
                    }
                    self.block_end(mark);
                }
            }
            Event::StartHeading { level } => {
                let mark = self.out.len();
                let level = (level as usize).clamp(1, 5);
                let marker = "=".repeat(7 - level);
                self.push_out(&marker);
                self.push_out(" ");
                self.stack.push(Frame::Heading { marker, mark });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { marker, mark }) = self.stack.pop() {
                    self.push_out(" ");
                    self.push_out(&marker);
                    self.push_out("\n\n");
                    self.block_end(mark);
                }
            }
            Event::StartBlockquote => {
                let mark = self.out.len();
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartList { ordered } => {
                let mark = self.out.len();
                self.stack.push(Frame::List {
                    ordered,
                    num: 1,
                    mark,
                });
            }
            Event::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartListItem { checked } => {
                let mark = self.out.len();
                let marker = if let Some(Frame::List { ordered, num, .. }) = self.stack.last_mut() {
                    Some(if let Some(c) = checked {
                        if c {
                            "[*] ".to_string()
                        } else {
                            "[ ] ".to_string()
                        }
                    } else if *ordered {
                        let s = format!("{num}. ");
                        *num += 1;
                        s
                    } else {
                        "* ".to_string()
                    })
                } else {
                    None
                };
                if let Some(marker) = marker {
                    self.push_out(&marker);
                }
                self.stack.push(Frame::ListItem { mark });
            }
            Event::EndListItem => {
                if let Some(Frame::ListItem { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.push_out("\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::CodeBlock { content } => {
                let mark = self.out.len();
                self.push_out("'''\n");
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("'''\n\n");
                self.block_end(mark);
            }
            Event::HorizontalRule => {
                let mark = self.out.len();
                self.push_out("----\n\n");
                self.block_end(mark);
            }
            Event::StartTable => {
                let mark = self.out.len();
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
            Event::StartTableCell => {
                let mark = self.out.len();
                self.push_out(" ");
                self.stack.push(Frame::TableCell { mark });
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell { mark }) = self.stack.pop() {
                    self.push_out(" |");
                    if !matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }

            // ── Inline events ────────────────────────────────────────────
            Event::Text(cow) => {
                if self.accepts_inline() {
                    self.push_out(&cow);
                }
            }
            Event::SoftBreak => {
                if self.accepts_inline() {
                    self.push_out(" ");
                }
            }
            Event::LineBreak => {
                if self.accepts_inline() {
                    self.push_out("\n");
                }
            }
            Event::InlineCode(cow) => {
                if self.accepts_inline() {
                    self.push_out("''");
                    self.push_out(&cow);
                    self.push_out("''");
                }
            }
            Event::InlineImage { url } => {
                if self.accepts_inline() {
                    self.push_out("{{");
                    self.push_out(&url);
                    self.push_out("}}");
                }
            }
            Event::StartBold => self.open_span("**", "**"),
            Event::EndBold => self.close_span(),
            Event::StartItalic => self.open_span("//", "//"),
            Event::EndItalic => self.close_span(),
            Event::StartUnderline => self.open_span("__", "__"),
            Event::EndUnderline => self.close_span(),
            Event::StartStrikethrough => self.open_span("~~", "~~"),
            Event::EndStrikethrough => self.close_span(),
            Event::StartSubscript => self.open_span("_{", "}"),
            Event::EndSubscript => self.close_span(),
            Event::StartSuperscript => self.open_span("^{", "}"),
            Event::EndSuperscript => self.close_span(),
            Event::StartLink { url } => {
                let mark = self.out.len();
                self.push_out("[[");
                self.push_out(&url);
                let content_mark = self.out.len();
                self.stack.push(Frame::Link { mark, content_mark });
            }
            Event::EndLink => {
                if let Some(Frame::Link { mark, content_mark }) = self.stack.pop() {
                    if self.out.len() > content_mark {
                        self.out.insert(content_mark, '|');
                    }
                    self.push_out("]]");
                    self.inline_end(mark);
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
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub(super) static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    // current/peak bytes are tracked per-thread (`thread_local!`, not a
    // shared `AtomicUsize`): the allocator is process-wide, and `cargo
    // test` runs other tests concurrently on other threads by default, so a
    // shared counter lets an unrelated test's allocations inflate this
    // measurement — confirmed as a real flake in this batch's `pod-fmt`
    // sibling (a spurious 407x ratio under full-workspace `cargo test -q`,
    // passing cleanly under `--test-threads=1`). Thread-local counters make
    // the measurement immune to what other threads in the same binary do,
    // so the `TEST_LOCK` mutex this file used to serialize just the two
    // memory-guard tests against each other is no longer needed.
    thread_local! {
        pub(super) static CURRENT: Cell<usize> = const { Cell::new(0) };
        pub(super) static PEAK: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) struct InstrumentedAlloc;

    unsafe impl GlobalAlloc for InstrumentedAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            let cur = CURRENT.with(|c| {
                let v = c.get() + layout.size();
                c.set(v);
                v
            });
            PEAK.with(|p| {
                if cur > p.get() {
                    p.set(cur);
                }
            });
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            CURRENT.with(|c| c.set(c.get().saturating_sub(layout.size())));
            unsafe { System.dealloc(ptr, layout) }
        }
    }
}

#[cfg(test)]
#[global_allocator]
static ALLOC_GUARD: alloc_guard::InstrumentedAlloc = alloc_guard::InstrumentedAlloc;

#[cfg(test)]
mod tests {
    use super::alloc_guard::{ALLOCS, CURRENT, PEAK};
    use super::*;
    use crate::events::OwnedEvent;
    use std::borrow::Cow;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 1 });
        w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("====== Hello ======"), "got: {s:?}");
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
        let input =
            "====== Hello ======\n\nA paragraph with **bold** text.\n\n* item one\n* item two\n";
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
    /// tree-based `emit::build()` for the same document — the guard that
    /// keeps the two independent emission paths honest.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "====== Title ======\n\nIntro paragraph with **bold** and //italic//.\n",
            "===== Sub =====\n\ntext with __underline__ and ~~strike~~.\n",
            "* bullet one\n* bullet two\n",
            "1. ordered one\n2. ordered two\n",
            "[ ] todo item\n[*] done item\n",
            "'''\ncode block\nline two\n'''\n",
            "> quoted paragraph text\n",
            "|A |B |\n|Cell 1 |Cell 2 |\n",
            "----\n\nAfter the transition.\n",
            "[[https://example.com]]\n",
            "[[https://example.com|click here]]\n",
            "{{image.png}}\n",
            "Sub_{script} and Super^{script}.\n",
            "Some ''inline code'' here.\n",
            "* outer one\n* outer two\n\n1. nested ordered\n",
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

    /// Regression guard against reintroducing per-block tree reconstruction:
    /// a large, deeply-nested event stream must complete with an allocation
    /// count that stays close to linear in event count.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use std::sync::atomic::Ordering;

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
                    evs.push(OwnedEvent::StartListItem { checked: None });
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
    /// multiple of a single paragraph's size, not `O(full document)` — the
    /// direct proof the writer never buffers the whole document (or a whole
    /// AST reconstruction of it) in memory at once.
    #[test]
    fn test_writer_peak_memory_bounded() {
        /// A sink that counts written bytes without retaining them — using
        /// `Vec<u8>` as the sink would conflate the *sink's own* inevitable
        /// growth to the final document size with the writer's internal
        /// buffering, making it impossible for any real streaming writer to
        /// pass a "peak << document size" assertion.
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

        // `CURRENT`/`PEAK` are thread-local, so resetting `PEAK` to this
        // thread's own current baseline is safe — no other thread's live
        // allocations are counted into this thread's cell.
        let baseline = CURRENT.with(|c| c.get());
        PEAK.with(|p| p.set(baseline));

        let mut w = Writer::new(CountingSink(0));
        for i in 0..N {
            w.write_event(OwnedEvent::StartParagraph);
            w.write_event(OwnedEvent::Text(Cow::Owned(format!(
                "paragraph number {i} with some filler text to give it realistic size"
            ))));
            w.write_event(OwnedEvent::EndParagraph);
        }
        let sink = w.finish();
        std::hint::black_box(&sink);

        let peak = PEAK.with(|p| p.get()).saturating_sub(baseline);
        let total_doc_bytes = sink.0;

        // The peak must stay well under the total document size — if the
        // writer buffered the whole document (or reconstructed an AST of
        // it) before writing anything, peak would scale with
        // `total_doc_bytes`. A generous ceiling (peak < 1/4 of the total
        // document) comfortably separates "bounded by a handful of
        // top-level blocks plus bookkeeping" from "buffers everything."
        assert!(
            (peak as f64) < (total_doc_bytes as f64) * 0.25,
            "peak allocated bytes ({peak}) is not small relative to total document size \
             ({total_doc_bytes}) — writer may be buffering the whole document instead of \
             flushing per top-level block"
        );
    }
}
