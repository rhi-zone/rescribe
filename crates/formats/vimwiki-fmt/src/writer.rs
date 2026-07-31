#![allow(clippy::collapsible_if)]
//! Streaming VimWiki writer -- converts a stream of events directly to
//! VimWiki text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`] or
//! [`crate::events::collect_doc_from_events`]. It is a second, independent
//! emission path from the tree-based builder, not a thin wrapper around it.
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document, plus a small `plain` buffer used only inside `Link` (see
//! below). Frames on the `Vec<Frame>` stack (`O(nesting depth)`) hold only
//! small metadata. Almost every construct is **write-through**: the heading
//! marker depends only on `level`, table cells carry no column-width
//! alignment, and list-item markers depend only on the enclosing `List`'s
//! `ordered` flag, an incrementing ordinal, and the item's own `checked`
//! field (all known at `Start*`). `vimwiki-fmt`'s `ListItem` holds only flat
//! inline text in the AST (no nested block children at all — unlike
//! `twiki`/`zimwiki`), so there is no list-nesting depth to track.
//!
//! **`Blockquote` dissolves its children**: `collect_doc_from_events`'s
//! `EndBlockquote` arm flattens every accumulated child block into a single
//! `Block::Blockquote { inlines }` by extending with each `Paragraph`
//! child's inlines *and dropping every other block kind entirely* — the
//! emitter then writes `"> " + inlines + "\n\n"` as one unbroken run (no
//! separator between what were originally separate paragraphs). `Writer`
//! mirrors this directly: a `Paragraph` whose immediate parent is
//! `Blockquote` writes no wrapper at all (its content just flows straight
//! into the still-open `Blockquote`'s output), while any *other* block kind
//! nested under `Blockquote` is written speculatively and then discarded
//! wholesale via `out.truncate(mark)` once it closes and finds a non-empty,
//! non-top-level parent (blocks are otherwise valid only directly under the
//! document root — see [`Writer::block_end_top_only`]).
//!
//! **`Link` needs a side buffer**: `emit::build_inline` only appends the
//! `|label` separator when the label text differs from the URL
//! (`Inline::Link`'s `label` is a flat `String` in the AST, and `events()`
//! always re-emits it as a single `Text` event, but `Writer` handles the
//! fully general `Start`/`End` case). While a `Link` is open, `push_out` is
//! suppressed (`link_depth > 0`) and leaf events append their plain-text
//! contribution to `Writer::plain` instead (mirroring `collect_inline_text`
//! exactly — e.g. an `Image`'s contribution is its `alt` text, not its
//! `url`, matching `Inline::Image { alt, .. } => if let Some(a) = alt {
//! s.push_str(a) }`); at `EndLink` the accumulated label is compared against
//! the URL to decide whether to write the separator. `O(that link's own
//! content)`, not `O(document)`.
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
        /// Set when this paragraph's immediate parent is `Blockquote` — see
        /// the module doc comment. Suppresses the normal `"\n\n"` wrapper
        /// entirely (content flows straight into the blockquote's own run).
        in_blockquote: bool,
    },
    Heading {
        marker: String,
        mark: usize,
    },
    /// See the module doc comment: writes `"> "` immediately at `Start`
    /// and is only valid directly under the document root, exactly like
    /// every other block kind except `Paragraph`.
    Blockquote {
        mark: usize,
    },
    List {
        ordered: bool,
        /// Next ordinal for an ordered item, mutated in place by
        /// `StartListItem`.
        num: usize,
        mark: usize,
    },
    /// Valid only directly inside `List`.
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
    DefinitionList {
        mark: usize,
    },
    /// Valid only directly inside `DefinitionList`.
    DefinitionTerm {
        mark: usize,
    },
    /// Valid only directly inside `DefinitionList`.
    DefinitionDesc {
        mark: usize,
    },
    /// Any inline span whose closing delimiter is a fixed string known when
    /// the span opens (bold/italic/strikethrough/superscript/subscript).
    Inline {
        close: &'static str,
        mark: usize,
    },
    /// See the module doc comment: while this frame is on the stack,
    /// `link_depth > 0` suppresses `out` writes and routes leaf content into
    /// `Writer::plain` instead.
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

/// Streaming VimWiki writer.
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

    /// Every block kind except `Paragraph` (which gets `Blockquote`'s
    /// content-flattening treatment — see the module doc comment) is valid
    /// only directly under the document root: `collect_doc_from_events`'s
    /// `push_block` structurally accepts a `Blockquote` parent too, but
    /// `EndBlockquote`'s flatten step discards every non-`Paragraph` child
    /// it collected, so the net effect for final bytes is "top level only."
    fn block_end_top_only(&mut self, mark: usize) {
        if self.stack.is_empty() {
            self.flush();
        } else {
            self.out.truncate(mark);
        }
    }

    /// Whether the top-of-stack frame accepts inline content — mirrors
    /// `collect_doc_from_events`'s `push_inline` match arms.
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
                let in_blockquote = matches!(self.stack.last(), Some(Frame::Blockquote { .. }));
                self.stack.push(Frame::Paragraph {
                    mark,
                    in_blockquote,
                });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph {
                    mark,
                    in_blockquote,
                }) = self.stack.pop()
                {
                    if !in_blockquote {
                        self.push_out("\n\n");
                        self.block_end_top_only(mark);
                    }
                    // Inside a blockquote: content already flowed straight
                    // into the still-open Blockquote frame's run, no
                    // wrapper and nothing to accept/reject independently.
                }
            }
            Event::StartHeading { level } => {
                let mark = self.out.len();
                let marker = "=".repeat(level);
                self.push_out(&marker);
                self.push_out(" ");
                self.stack.push(Frame::Heading { marker, mark });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { marker, mark }) = self.stack.pop() {
                    self.push_out(" ");
                    self.push_out(&marker);
                    self.push_out("\n\n");
                    self.block_end_top_only(mark);
                }
            }
            Event::StartBlockquote => {
                let mark = self.out.len();
                self.push_out("> ");
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.push_out("\n\n");
                    self.block_end_top_only(mark);
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
                    self.block_end_top_only(mark);
                }
            }
            Event::StartListItem { checked } => {
                let mark = self.out.len();
                if let Some(Frame::List { ordered, .. }) = self.stack.last() {
                    let ordered = *ordered;
                    if ordered {
                        let num = if let Some(Frame::List { num, .. }) = self.stack.last_mut() {
                            let n = *num;
                            *num += 1;
                            n
                        } else {
                            unreachable!()
                        };
                        self.push_out(&format!("{num}. "));
                    } else {
                        self.push_out("* ");
                    }
                    if let Some(c) = checked {
                        self.push_out(if c { "[X] " } else { "[ ] " });
                    }
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
                self.push_out("{{{");
                if let Some(lang) = &language {
                    self.push_out(lang);
                }
                self.push_out("\n");
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("}}}\n\n");
                self.block_end_top_only(mark);
            }
            Event::HorizontalRule => {
                let mark = self.out.len();
                self.push_out("----\n\n");
                self.block_end_top_only(mark);
            }
            Event::StartTable => {
                let mark = self.out.len();
                self.stack.push(Frame::Table { mark });
            }
            Event::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end_top_only(mark);
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
            Event::StartDefinitionList => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionList { mark });
            }
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end_top_only(mark);
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

            // ── Inline leaf events ───────────────────────────────────────
            Event::Text(cow) => {
                if self.link_depth > 0 {
                    self.plain.push_str(&cow);
                } else if self.accepts_inline() {
                    self.out.push_str(&cow);
                }
            }
            Event::InlineCode(cow) => {
                if self.link_depth > 0 {
                    self.plain.push_str(&cow);
                } else if self.accepts_inline() {
                    self.out.push('`');
                    self.out.push_str(&cow);
                    self.out.push('`');
                }
            }
            Event::InlineImage { url, alt, style } => {
                if self.link_depth > 0 {
                    if let Some(a) = &alt {
                        self.plain.push_str(a);
                    }
                } else if self.accepts_inline() {
                    self.out.push_str("{{");
                    self.out.push_str(&url);
                    if let Some(a) = &alt {
                        self.out.push('|');
                        self.out.push_str(a);
                    }
                    if let Some(s) = &style {
                        if alt.is_none() {
                            self.out.push('|');
                        }
                        self.out.push('|');
                        self.out.push_str(s);
                    }
                    self.out.push_str("}}");
                }
            }

            // ── Inline spans ─────────────────────────────────────────────
            Event::StartBold => self.open_span("*", "*"),
            Event::EndBold => self.close_span(),
            Event::StartItalic => self.open_span("_", "_"),
            Event::EndItalic => self.close_span(),
            Event::StartStrikethrough => self.open_span("~~", "~~"),
            Event::EndStrikethrough => self.close_span(),
            Event::StartSuperscript => self.open_span("^", "^"),
            Event::EndSuperscript => self.close_span(),
            Event::StartSubscript => self.open_span(",,", ",,"),
            Event::EndSubscript => self.close_span(),

            Event::StartLink { url } => {
                let mark = self.out.len();
                self.push_out("[[");
                self.push_out(&url);
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
                    if label != url {
                        self.push_out("|");
                        self.push_out(&label);
                    }
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

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartHeading { level: 1 });
        w.write_event(Event::Text(std::borrow::Cow::Owned("Hello".to_string())));
        w.write_event(Event::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("= Hello ="), "got: {s:?}");
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
        let input = "= Hello =\n\nA paragraph with *bold* text.\n\n* item one\n* item two\n";
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
    /// tree-based `emit::build()` for the same document.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "= Title =\n\nIntro paragraph with *bold* and _italic_.\n",
            "== Sub ==\n\ntext with ~~strike~~ and ^super^ and ,,sub,,.\n",
            "* bullet one\n* bullet two\n",
            "1. ordered one\n2. ordered two\n",
            "- [ ] todo item\n- [X] done item\n",
            "{{{\ncode block\nline two\n}}}\n",
            "{{{python\nprint(1)\n}}}\n",
            "> quoted paragraph text\n",
            "| A | B |\n| Cell 1 | Cell 2 |\n",
            "----\n\nAfter the transition.\n",
            "[[https://example.com]]\n",
            "[[https://example.com|click here]]\n",
            "{{img.png}}\n",
            "{{img.png|alt text}}\n",
            "A paragraph with an `inline code` span.\n",
            "; term one\n: definition one\n; term two\n: definition two\n",
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

    /// A multi-paragraph blockquote must have its paragraphs' content
    /// merged into one unbroken run (no blank line between what were
    /// originally separate paragraphs) — proving `Writer` replicates
    /// `collect_doc_from_events`'s flattening rather than treating a nested
    /// `Paragraph` like a normal one.
    #[test]
    fn test_writer_blockquote_merges_paragraphs() {
        let input = "> first line\n> second line\n";
        let (doc, _) = crate::parse::parse(input);
        let built = crate::emit::build(&doc);

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events(input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).unwrap();

        assert_eq!(built, streamed);
    }

    /// Regression guard against reintroducing per-block tree reconstruction:
    /// a large, deeply-nested event stream must complete with an allocation
    /// count that stays close to linear in event count.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use std::borrow::Cow;
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
