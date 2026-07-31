#![allow(clippy::collapsible_if)]
//! Streaming txt2tags writer — converts a stream of events directly to t2t
//! text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::emit`]. It is a second, independent
//! emission path from the tree-based `emit()` function, not a thin wrapper
//! around it.
//!
//! # Construct classification
//!
//! Every txt2tags construct is **write-straight-through** into a single
//! shared `out: String` buffer — reading `emit.rs` end to end before writing
//! this module found no construct anywhere whose prefix depends on content
//! not yet seen (no heading-underline-width computation like RST — a t2t
//! heading's `=`/`+` run width is fixed by `level.min(5)`, known at
//! `StartHeading` — and no table column-width pass like RST/Markdown — t2t
//! table cells are written `| cell |` with no padding at all). Unlike
//! `rst-fmt`/`djot-fmt`, there also isn't a generic "blank line between
//! siblings" rule to implement: `emit.rs`'s top-level loop
//! (`for block in &doc.blocks { build_block(block, ctx); }`) and every
//! block-container's child loop insert *no* separator of their own between
//! children — each block variant's own arm already writes its complete
//! trailing whitespace (`"\n\n"` for `Paragraph`/`Heading`/`CodeBlock`, a
//! single extra `"\n"` after `List`/`Table`/`Blockquote`/`DefinitionList`'s
//! own per-item terminators), so consecutive children simply concatenate.
//!
//! The one real subtlety is that `Blockquote`/`ListItem`/`DefinitionDesc`
//! give their `Paragraph` children *different* framing than a top-level
//! `Paragraph` gets, dispatched by child *type*, not by position:
//! - top-level (or any other context) `Paragraph`: inlines + `"\n\n"`.
//! - `Paragraph` directly inside `Blockquote`: a `"\t"` prefix, inlines,
//!   then a single `"\n"` (no blank line).
//! - `Paragraph` directly inside `ListItem`: no prefix, inlines, *no*
//!   trailing newline at all (`emit.rs`'s item loop concatenates all of an
//!   item's block children directly, only writing one `"\n"` after the
//!   whole item at `EndListItem`).
//! - `Paragraph` directly inside `DefinitionDesc`: no prefix, inlines, one
//!   `"\n"`.
//!
//! All four cases are decided at the `Paragraph` frame's `Start`/`End`
//! events purely by inspecting the parent frame already on the stack —
//! exactly the same "known at open, applied at close" shape `rst-fmt`'s
//! `BlockKind`/`ListItem.first` dispatch uses, just keyed on parent type
//! here instead of position. Non-`Paragraph` children of `Blockquote`/
//! `ListItem`/`DefinitionDesc` (a nested `List`, `Table`, etc.) get their
//! own unmodified top-level formatting — `emit.rs`'s `else { build_block
//! (child, ctx) }` branch, confirmed by reading it rather than assumed by
//! analogy with RST/djot's blockquotes (which *do* re-indent their full
//! subtree — t2t deliberately does not).
//!
//! The document header (title/author/date, always exactly 3 lines + a
//! blank line, even when all three are absent) needs its fields before any
//! block content is written. `events()`'s own contract emits `Event::Header`
//! (if any field is set) immediately after `Event::StartDocument` and before
//! any block event — so, mirroring `texinfo::writer::Writer`, the header is
//! written lazily: `StartDocument` alone produces no output yet; the event
//! immediately after decides it (either `Header`'s fields, or — if that next
//! event is anything else — three empty-string lines, matching `emit()`'s
//! unconditional header).
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties, mirroring `rst-fmt`.
//!
//! # Example
//! ```no_run
//! use t2t::writer::Writer;
//! use t2t::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, numbered: false });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::Event;
use std::io::Write;

/// Streaming txt2tags writer.
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
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level.
    stack: Vec<Frame>,
    /// Whether `StartDocument` has been seen but the header decision (see
    /// the module doc) hasn't been made yet.
    started: bool,
    /// Whether the header has been written yet.
    header_written: bool,
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
            started: false,
            header_written: false,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: Event<'_>) {
        if !self.header_written {
            match event {
                Event::StartDocument => {
                    self.started = true;
                    return;
                }
                Event::Header {
                    title,
                    author,
                    date,
                } => {
                    self.write_header(title.as_deref(), author.as_deref(), date.as_deref());
                    self.header_written = true;
                    return;
                }
                _ => {
                    self.write_header(None, None, None);
                    self.header_written = true;
                }
            }
        }
        self.process(event);
        self.maybe_flush();
    }

    /// Recover the underlying sink. Does not write anything — every
    /// completed top-level block was already flushed by `write_event`, and
    /// `t2t` has no document-level footer.
    pub fn finish(mut self) -> W {
        if !self.header_written {
            // Zero events fed (or only StartDocument): emit()'s own header
            // is unconditional, so an empty document still gets one.
            self.write_header(None, None, None);
        }
        self.flush();
        self.sink
    }

    fn write_header(&mut self, title: Option<&str>, author: Option<&str>, date: Option<&str>) {
        self.out.push_str(title.unwrap_or(""));
        self.out.push('\n');
        self.out.push_str(author.unwrap_or(""));
        self.out.push('\n');
        self.out.push_str(date.unwrap_or(""));
        self.out.push_str("\n\n");
        self.maybe_flush();
    }

    // ── Buffer primitives ─────────────────────────────────────────────────

    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    fn maybe_flush(&mut self) {
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Whether the top-of-stack frame accepts block children (mirrors the
    /// old `DocBuilder::push_block`'s match: only `Document`/`Blockquote`/
    /// `ListItem`/`DefinitionDesc` ever collected blocks).
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(
                Frame::Blockquote { .. } | Frame::ListItem { .. } | Frame::DefinitionDesc { .. }
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
                    | Frame::TableCell
                    | Frame::DefinitionTerm { .. }
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
            )
        )
    }

    /// Called right before the first inline byte a `Link` child would
    /// contribute — just bumps the flag deciding whether `EndLink` writes
    /// the `" "` before the URL (t2t writes `url` *after* any link text,
    /// unlike RST/texinfo/djot's `` `text <url>` `` shape).
    fn note_link_child(&mut self) {
        if let Some(Frame::Link { wrote_any, .. }) = self.stack.last_mut() {
            *wrote_any = true;
        }
    }

    fn open_inline_span(&mut self, open: &str, close: &'static str) {
        let mark = self.out.len();
        self.out.push_str(open);
        self.stack.push(Frame::Inline { mark, close });
    }

    fn close_inline_span(&mut self) {
        if let Some(Frame::Inline { mark, close }) = self.stack.pop() {
            self.out.push_str(close);
            if !self.accepts_inline() {
                self.out.truncate(mark);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event<'_>) {
        match event {
            Event::StartDocument | Event::EndDocument => {}
            Event::Header { .. } => {
                // Only meaningful as the event right after StartDocument,
                // handled in write_event() before reaching process(); a
                // Header arriving later (violating events()'s documented
                // ordering) has no well-defined header slot left and is
                // dropped.
            }

            // ── Block open/close ────────────────────────────────────────
            Event::StartParagraph => {
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::Blockquote { .. })) {
                    self.out.push('\t');
                }
                self.stack.push(Frame::Paragraph { mark });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    match self.stack.last() {
                        Some(Frame::Blockquote { .. } | Frame::DefinitionDesc { .. }) => {
                            self.out.push('\n');
                        }
                        Some(Frame::ListItem { .. }) => {}
                        _ => self.out.push_str("\n\n"),
                    }
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::StartHeading { level, numbered } => {
                let mark = self.out.len();
                let capped = (level as usize).min(5);
                let marker = if numbered { '+' } else { '=' };
                for _ in 0..capped {
                    self.out.push(marker);
                }
                self.out.push(' ');
                self.stack.push(Frame::Heading {
                    mark,
                    marker,
                    capped,
                });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading {
                    mark,
                    marker,
                    capped,
                }) = self.stack.pop()
                {
                    self.out.push(' ');
                    for _ in 0..capped {
                        self.out.push(marker);
                    }
                    self.out.push_str("\n\n");
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::CodeBlock { content } => {
                if self.accepts_blocks() {
                    self.out.push_str("```\n");
                    self.out.push_str(&content);
                    if !content.ends_with('\n') {
                        self.out.push('\n');
                    }
                    self.out.push_str("```\n\n");
                    self.maybe_flush();
                }
            }
            Event::RawBlock { content } => {
                if self.accepts_blocks() {
                    self.out.push_str("\"\"\"\n");
                    self.out.push_str(&content);
                    if !content.ends_with('\n') {
                        self.out.push('\n');
                    }
                    self.out.push_str("\"\"\"\n\n");
                    self.maybe_flush();
                }
            }
            Event::StartBlockquote => {
                let mark = self.out.len();
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::StartList { ordered } => {
                let mark = self.out.len();
                self.stack.push(Frame::List { mark, ordered });
            }
            Event::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.out.push('\n');
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::StartListItem => {
                let mark = self.out.len();
                if let Some(Frame::List { ordered, .. }) = self.stack.last() {
                    self.out.push_str(if *ordered { "+ " } else { "- " });
                }
                self.stack.push(Frame::ListItem { mark });
            }
            Event::EndListItem => {
                if let Some(Frame::ListItem { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.out.push('\n');
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartTable => {
                let mark = self.out.len();
                self.stack.push(Frame::Table { mark });
            }
            Event::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::StartTableRow { header } => {
                let mark = self.out.len();
                let in_table = matches!(self.stack.last(), Some(Frame::Table { .. }));
                if in_table {
                    self.out.push_str(if header { "||" } else { "|" });
                }
                self.stack.push(Frame::TableRow { mark });
            }
            Event::EndTableRow => {
                if let Some(Frame::TableRow { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::Table { .. })) {
                        self.out.push('\n');
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartTableCell => {
                if matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                    self.out.push(' ');
                }
                self.stack.push(Frame::TableCell);
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                        self.out.push_str(" |");
                    }
                }
            }
            Event::HorizontalRule => {
                if self.accepts_blocks() {
                    self.out.push_str("--------------------\n\n");
                    self.maybe_flush();
                }
            }
            Event::StartDefinitionList => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionList { mark });
            }
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::StartDefinitionTerm => {
                let mark = self.out.len();
                let in_list = matches!(self.stack.last(), Some(Frame::DefinitionList { .. }));
                if in_list {
                    self.out.push_str(": ");
                }
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
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionDesc { mark });
            }
            Event::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { mark }) = self.stack.pop() {
                    if !matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }

            // ── Inline events ───────────────────────────────────────────
            Event::Text(cow) => {
                if self.accepts_inline() {
                    self.note_link_child();
                    self.out.push_str(&cow);
                }
            }
            Event::LineBreak => {
                if self.accepts_inline() {
                    self.note_link_child();
                    self.out.push('\n');
                }
            }
            Event::SoftBreak => {
                if self.accepts_inline() {
                    self.note_link_child();
                    self.out.push(' ');
                }
            }
            Event::StartBold => self.open_inline_span("**", "**"),
            Event::EndBold => self.close_inline_span(),
            Event::StartItalic => self.open_inline_span("//", "//"),
            Event::EndItalic => self.close_inline_span(),
            Event::StartUnderline => self.open_inline_span("__", "__"),
            Event::EndUnderline => self.close_inline_span(),
            Event::StartStrikethrough => self.open_inline_span("--", "--"),
            Event::EndStrikethrough => self.close_inline_span(),
            Event::Code(cow) => {
                if self.accepts_inline() {
                    self.note_link_child();
                    self.out.push_str("``");
                    self.out.push_str(&cow);
                    self.out.push_str("``");
                }
            }
            Event::Verbatim(cow) => {
                if self.accepts_inline() {
                    self.note_link_child();
                    self.out.push_str("\"\"");
                    self.out.push_str(&cow);
                    self.out.push_str("\"\"");
                }
            }
            Event::Tagged(cow) => {
                if self.accepts_inline() {
                    self.note_link_child();
                    self.out.push_str("''");
                    self.out.push_str(&cow);
                    self.out.push_str("''");
                }
            }
            Event::StartLink { url } => {
                let mark = self.out.len();
                self.out.push('[');
                self.stack.push(Frame::Link {
                    mark,
                    url: url.into_owned(),
                    wrote_any: false,
                });
            }
            Event::EndLink => {
                if let Some(Frame::Link {
                    mark,
                    url,
                    wrote_any,
                }) = self.stack.pop()
                {
                    if wrote_any {
                        self.out.push(' ');
                    }
                    self.out.push_str(&url);
                    self.out.push(']');
                    if !self.accepts_inline() {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::Image { src } => {
                if self.accepts_inline() {
                    self.note_link_child();
                    self.out.push('[');
                    self.out.push_str(&src);
                    self.out.push(']');
                }
            }
        }
    }
}

/// Frames carry only a mark into the shared buffer and tiny scalars — never
/// accumulated content.
enum Frame {
    Paragraph {
        mark: usize,
    },
    Heading {
        mark: usize,
        marker: char,
        capped: usize,
    },
    Blockquote {
        mark: usize,
    },
    List {
        mark: usize,
        ordered: bool,
    },
    ListItem {
        mark: usize,
    },
    Table {
        mark: usize,
    },
    TableRow {
        mark: usize,
    },
    TableCell,
    DefinitionList {
        mark: usize,
    },
    DefinitionTerm {
        mark: usize,
    },
    DefinitionDesc {
        mark: usize,
    },
    /// Any inline span whose closing text is a fixed string
    /// (bold/italic/underline/strikethrough — all symmetric open==close).
    Inline {
        mark: usize,
        close: &'static str,
    },
    /// `[text url]` / `[url]` — t2t writes the URL *after* any link text
    /// (the reverse of RST/texinfo/djot's `text <url>` shape), so `url` is
    /// carried on the frame and written at `EndLink`, not `StartLink`.
    Link {
        mark: usize,
        url: String,
        wrote_any: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    // Single shared tracking allocator for this module (`#[global_allocator]`
    // may be declared at most once for the whole test binary) — see
    // texinfo::writer's identically-shaped test module for the rationale.
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TrackingAlloc;
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    // current/peak bytes are tracked per-thread (`thread_local!`, not a
    // shared `AtomicUsize`): `cargo test` runs this crate's other tests
    // concurrently with these two allocator-instrumented ones by default,
    // and a shared counter lets an unrelated concurrently-running test's
    // allocations inflate this test's measured peak — confirmed as a real
    // flake in the wiki-format streaming-writer sweep's `pod-fmt` sibling (a
    // spurious 407x ratio under full-workspace `cargo test -q`, passing
    // cleanly under `--test-threads=1`). Thread-local counters make the
    // measurement immune to what other threads in the same binary do, so
    // the `ALLOC_TRACKING_LOCK` mutex this file used to serialize just the
    // two allocator-instrumented tests against each other is no longer
    // needed.
    thread_local! {
        static CURRENT: Cell<usize> = const { Cell::new(0) };
        static PEAK: Cell<usize> = const { Cell::new(0) };
    }
    unsafe impl GlobalAlloc for TrackingAlloc {
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
    #[global_allocator]
    static GLOBAL: TrackingAlloc = TrackingAlloc;

    /// Regression guard: an incremental writer must not reintroduce
    /// per-block subtree reconstruction. Allocation count for feeding N
    /// events through `Writer` must stay near-linear in N.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        fn events_for(n: usize) -> Vec<Event<'static>> {
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(Event::StartHeading {
                    level: 2,
                    numbered: false,
                });
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
    /// document. `#[ignore]`d because it prints rather than asserts a
    /// threshold — run with `cargo test -p t2t --release \
    /// test_writer_peak_memory_and_throughput_report -- --ignored \
    /// --nocapture` to see the numbers.
    #[test]
    #[ignore]
    fn test_writer_peak_memory_and_throughput_report() {
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
            let mut s = String::from("Big Document\n\n\n");
            for i in 0..n {
                s.push_str(&format!(
                    "= Section {i} =\n\n\
                     Some plain text with **bold** and //italic// markup, and a \
                     [link {i} http://example.com/{i}].\n\n\
                     - first point {i}\n- second point {i}\n\n"
                ));
            }
            s
        }

        let input = synthetic_source(5000);

        let events: Vec<Event<'static>> = crate::events::events(&input)
            .map(Event::into_owned)
            .collect();
        let (doc, _diags) = crate::parse::parse(&input);

        let baseline = CURRENT.with(|c| c.get());
        PEAK.with(|p| p.set(baseline));
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
        let streaming_peak = PEAK.with(|p| p.get()).saturating_sub(baseline);
        std::hint::black_box(bytes_written);

        let baseline = CURRENT.with(|c| c.get());
        PEAK.with(|p| p.set(baseline));
        let start = std::time::Instant::now();
        let built = crate::emit::emit(std::hint::black_box(&doc));
        let builder_elapsed = start.elapsed();
        let builder_peak = PEAK.with(|p| p.get()).saturating_sub(baseline);
        std::hint::black_box(&built);

        eprintln!(
            "t2t streaming Writer vs parse()+emit() builder, {} bytes input, 5000 sections:\n\
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
            numbered: false,
        });
        w.write_event(Event::Text(Cow::Owned("Hello".to_string())));
        w.write_event(Event::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("= Hello ="), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text(Cow::Owned("World".to_string())));
        w.write_event(Event::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "= Hello =\n\nA paragraph with **bold** text.\n\n- item one\n- item two\n";
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

    /// The streaming `Writer` must produce byte-identical output to the
    /// tree-based `emit()` for the same document.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "= Hello =\n\nSome **bold** and //italic// and ``code``.\n",
            "- one\n- two\n\n  extra text\n",
            "+ first\n+ second\n",
            "```\nsome code\n```\n",
            "\t> a quote\n",
            ": term\ndefinition body\n\n: term2\nanother\n",
            "| A | B |\n",
            "[link text http://x/]\n",
            "[http://img.png]\n",
            "----------\n",
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
}
