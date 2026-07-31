#![allow(clippy::collapsible_if)]
//! Streaming XWiki writer -- converts a stream of events directly to XWiki
//! text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`] or
//! [`crate::events::collect_doc_from_events`]. It is a second, independent
//! emission path from the tree-based builder, not a thin wrapper around it.
//! (This crate's `batch::StreamingParser` — the reader side — has its own,
//! separate `KnownFailure` for buffering all fed bytes before parsing; that
//! is out of scope here, this file only concerns the writer.)
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document, plus a small `plain` buffer used only inside `Link` (see
//! below). Frames on the `Vec<Frame>` stack (`O(nesting depth)`) hold only
//! small metadata. Almost every construct is **write-through**: the heading
//! marker depends only on `level`, table cells carry no column-width
//! alignment, and — unlike every other crate in this batch — XWiki's list
//! rendering has **no nesting-depth-dependent indentation at all**:
//! `emit::build_block`'s `List` arm never tracks depth, so a nested `List`
//! inside a `ListItem` renders with exactly the same bytes as a top-level
//! one. `ListItem` accepts *any* block kind (not just nested lists, unlike
//! `twiki`) — every kind except `Paragraph` renders with its own normal
//! formatting; `Paragraph` alone drops its `"\n\n"` wrapper so its text
//! flows straight into the item's line, mirroring `build_block`'s `List`
//! arm's `if let Block::Paragraph { inlines, .. } = block { build_inlines
//! (...) } else { build_block(other, ...) }` branch.
//!
//! **`Link` is the one construct needing real reordering, not just
//! suppression**: `build_inline` renders `Inline::Link` as
//! `[[label>>url]]` — the *label comes before the url*, but `label` (a flat
//! `String` in the AST, always re-emitted by `events()` as a single `Text`
//! event) is only known once the link's children have been seen, while
//! `url` is known immediately at `StartLink`. So `url` is held on the
//! `Link` frame rather than written immediately, `push_out` is suppressed
//! while the link is open (leaf content accumulates in `Writer::plain`
//! instead, mirroring `collect_inline_text`'s recursion), and at `EndLink`
//! the label is written first, then `">>"`, then the held `url`. Still
//! `O(that link's own content)`, not `O(document)`.
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
        /// Set when this paragraph's immediate parent is `ListItem` — see
        /// the module doc comment. Suppresses the normal `"\n\n"` wrapper
        /// entirely (content flows straight into the item's own line).
        in_list_item: bool,
    },
    Heading {
        marker: String,
        mark: usize,
    },
    Blockquote {
        mark: usize,
    },
    List {
        ordered: bool,
        mark: usize,
    },
    /// Accepts any block kind (see the module doc comment) — the only
    /// per-kind special case is `Paragraph`'s dropped wrapper.
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
    /// the span opens (bold/italic/underline/strikeout/superscript/
    /// subscript).
    Inline {
        close: &'static str,
        mark: usize,
    },
    /// See the module doc comment: while this frame is on the stack,
    /// `link_depth > 0` suppresses `out` writes and routes leaf content into
    /// `Writer::plain` instead. `url` is held here rather than written
    /// immediately, since the label (written first) isn't known until
    /// `EndLink`.
    Link {
        url: String,
        mark: usize,
        /// Where this link's own plain-text label begins in `Writer::plain`.
        plain_mark: usize,
    },
}

/// Default capacity reserved for `Writer::out`. See `rst_fmt::writer`'s
/// identical constant for the rationale.
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Streaming XWiki writer.
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
    /// doc comment.
    plain: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level — a block closing with an empty stack
    /// is flushed to the sink immediately.
    stack: Vec<Frame>,
    /// Count of currently-open `Link` frames — see the module doc comment.
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
    /// mirrors `collect_doc_from_events`'s `push_block`, which accepts
    /// `Document`, `Blockquote`, and `ListItem` uniformly (no per-block-kind
    /// filtering, unlike `twiki`).
    fn accepts_block(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote { .. } | Frame::ListItem { .. })
        )
    }

    /// Whether the top-of-stack frame accepts inline content — mirrors
    /// `push_inline`'s match arms. Note `ListItem` is *not* included: its
    /// own text must arrive wrapped in a `Paragraph`, unlike `twiki`/
    /// `vimwiki-fmt` where list items accept raw inline events directly.
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

    fn block_end(&mut self, mark: usize) {
        if !self.accepts_block() {
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
                let in_list_item = matches!(self.stack.last(), Some(Frame::ListItem { .. }));
                self.stack.push(Frame::Paragraph { mark, in_list_item });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph { mark, in_list_item }) = self.stack.pop() {
                    if !in_list_item {
                        self.push_out("\n\n");
                        self.block_end(mark);
                    }
                    // Inside a list item: content already flowed straight
                    // into the item's own line; the item's closing "\n" is
                    // written unconditionally by EndListItem regardless of
                    // this paragraph's presence.
                }
            }
            Event::StartHeading { level } => {
                let mark = self.out.len();
                let marker = "=".repeat(level as usize);
                self.push_out(&marker);
                self.push_out(" ");
                self.stack.push(Frame::Heading { marker, mark });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { marker, mark }) = self.stack.pop() {
                    self.push_out(" ");
                    self.push_out(&marker);
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartBlockquote => {
                let mark = self.out.len();
                self.push_out("{{quote}}\n");
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.push_out("{{/quote}}\n\n");
                    self.block_end(mark);
                }
            }
            Event::StartList { ordered } => {
                let mark = self.out.len();
                self.stack.push(Frame::List { ordered, mark });
            }
            Event::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartListItem => {
                let mark = self.out.len();
                if let Some(Frame::List { ordered, .. }) = self.stack.last() {
                    self.push_out(if *ordered { "1. " } else { "* " });
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
            Event::CodeBlock { language, content } => {
                let mark = self.out.len();
                if let Some(lang) = &language {
                    self.push_out("{{code language=\"");
                    self.push_out(lang);
                    self.push_out("\"}}\n");
                } else {
                    self.push_out("{{code}}\n");
                }
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("{{/code}}\n\n");
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
            Event::StartTableCell { is_header } => {
                let mark = self.out.len();
                if is_header {
                    self.push_out("=");
                }
                self.stack.push(Frame::TableCell { mark });
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell { mark }) = self.stack.pop() {
                    self.push_out("|");
                    if !matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::MacroBlock {
                name,
                params,
                content,
            } => {
                let mark = self.out.len();
                self.push_out("{{");
                self.push_out(&name);
                if !params.is_empty() {
                    self.push_out(" ");
                    self.push_out(&params);
                }
                self.push_out("}}\n");
                self.push_out(&content);
                if !content.is_empty() && !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("{{/");
                self.push_out(&name);
                self.push_out("}}\n\n");
                self.block_end(mark);
            }
            Event::MacroInline { name, params } => {
                let mark = self.out.len();
                self.push_out("{{");
                self.push_out(&name);
                if !params.is_empty() {
                    self.push_out(" ");
                    self.push_out(&params);
                }
                self.push_out("/}}\n\n");
                self.block_end(mark);
            }

            // ── Inline leaf events ───────────────────────────────────────
            Event::Text(cow) => {
                if self.link_depth > 0 {
                    self.plain.push_str(&cow);
                } else if self.accepts_inline() {
                    self.out.push_str(&cow);
                }
            }
            Event::SoftBreak => {
                if self.link_depth > 0 {
                    self.plain.push(' ');
                } else if self.accepts_inline() {
                    self.out.push(' ');
                }
            }
            Event::LineBreak => {
                if self.link_depth > 0 {
                    self.plain.push('\n');
                } else if self.accepts_inline() {
                    self.out.push_str("\\\\ ");
                }
            }
            Event::InlineCode(cow) => {
                if self.link_depth > 0 {
                    self.plain.push_str(&cow);
                } else if self.accepts_inline() {
                    self.out.push_str("##");
                    self.out.push_str(&cow);
                    self.out.push_str("##");
                }
            }
            Event::InlineImage { url, alt, params } => {
                if self.link_depth > 0 {
                    self.plain.push_str(&url);
                } else if self.accepts_inline() {
                    self.out.push_str("[[image:");
                    self.out.push_str(&url);
                    let has_alt = alt.is_some();
                    let has_params = !params.is_empty();
                    if has_alt || has_params {
                        self.out.push_str("||");
                        if let Some(alt_text) = &alt {
                            self.out.push_str("alt=\"");
                            self.out.push_str(alt_text);
                            self.out.push('"');
                            if has_params {
                                self.out.push(' ');
                            }
                        }
                        let param_strs: Vec<String> =
                            params.iter().map(|(k, v)| format!("{k}=\"{v}\"")).collect();
                        self.out.push_str(&param_strs.join(" "));
                    }
                    self.out.push_str("]]");
                }
            }

            // ── Inline spans ─────────────────────────────────────────────
            Event::StartBold => self.open_span("**", "**"),
            Event::EndBold => self.close_span(),
            Event::StartItalic => self.open_span("//", "//"),
            Event::EndItalic => self.close_span(),
            Event::StartUnderline => self.open_span("__", "__"),
            Event::EndUnderline => self.close_span(),
            Event::StartStrikeout => self.open_span("--", "--"),
            Event::EndStrikeout => self.close_span(),
            Event::StartSuperscript => self.open_span("^^", "^^"),
            Event::EndSuperscript => self.close_span(),
            Event::StartSubscript => self.open_span("~~", "~~"),
            Event::EndSubscript => self.close_span(),

            Event::StartLink { url } => {
                // "[[" is the only prefix known immediately — the label
                // (written before the url) isn't known until EndLink, so
                // url is held on the frame instead of written now.
                let mark = self.out.len();
                self.push_out("[[");
                let plain_mark = self.plain.len();
                self.link_depth += 1;
                self.stack.push(Frame::Link {
                    url,
                    mark,
                    plain_mark,
                });
            }
            Event::EndLink => {
                if let Some(Frame::Link {
                    url,
                    mark,
                    plain_mark,
                }) = self.stack.pop()
                {
                    self.link_depth -= 1;
                    let label = self.plain[plain_mark..].to_string();
                    self.plain.truncate(plain_mark);
                    self.push_out(&label);
                    self.push_out(">>");
                    self.push_out(&url);
                    self.push_out("]]");
                    if self.link_depth > 0 {
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
    use std::borrow::Cow;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartHeading { level: 1 });
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
    fn test_writer_roundtrip() {
        let input = "= Hello =\n\nA paragraph with **bold** text.\n";
        let (doc, _) = crate::parse::parse(input);
        let evts: Vec<_> = crate::events::events(&doc)
            .map(|e| e.into_owned())
            .collect();
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
    /// tree-based `emit::build()` for the same document.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "= Title =\n\nIntro paragraph with **bold** and //italic//.\n",
            "== Sub ==\n\ntext with __underline__ and --strikeout--.\n",
            "text with ^^super^^ and ~~sub~~.\n",
            "* bullet one\n* bullet two\n",
            "1. ordered one\n1. ordered two\n",
            "* outer\n** nested\n",
            "{{code}}\nplain code\n{{/code}}\n",
            "{{code language=\"rust\"}}\nlet x = 1;\n{{/code}}\n",
            "{{quote}}\nA quoted paragraph.\n{{/quote}}\n",
            "|=Header 1|=Header 2|\n|Cell 1|Cell 2|\n",
            "----\n\nAfter the transition.\n",
            "[[label>>https://example.com]]\n",
            "[[image:img.png]]\n",
            "[[image:img.png||alt=\"a description\"]]\n",
            "A paragraph with an ##inline code## span.\n",
            "{{info}}\nAn info macro body.\n{{/info}}\n",
            "{{warning param=\"x\"/}}\n",
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

    /// Regression guard against reintroducing per-block tree reconstruction:
    /// a large, deeply-nested event stream must complete with an allocation
    /// count that stays close to linear in event count.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use std::sync::atomic::Ordering;

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
                    evs.push(Event::StartParagraph);
                    evs.push(Event::Text(Cow::Owned(format!("item {j}"))));
                    evs.push(Event::EndParagraph);
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

        // `CURRENT`/`PEAK` are thread-local, so resetting `PEAK` to this
        // thread's own current baseline is safe — no other thread's live
        // allocations are counted into this thread's cell.
        let baseline = CURRENT.with(|c| c.get());
        PEAK.with(|p| p.set(baseline));

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

        let peak = PEAK.with(|p| p.get()).saturating_sub(baseline);
        let total_doc_bytes = sink.0;

        assert!(
            (peak as f64) < (total_doc_bytes as f64) * 0.25,
            "peak allocated bytes ({peak}) is not small relative to total document size \
             ({total_doc_bytes}) — writer may be buffering the whole document instead of \
             flushing per top-level block"
        );
    }
}
