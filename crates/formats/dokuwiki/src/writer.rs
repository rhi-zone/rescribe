//! Streaming DokuWiki writer -- converts a stream of events directly to
//! DokuWiki text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`] or any of its helpers. It is
//! a second, independent emission path from the tree-based `build()`, not a
//! thin wrapper around it.
//!
//! # Buffer model
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document, mirroring `BuildContext::output`'s single amortized
//! geometric growth. Frames on the `Vec<Frame>` stack (`O(nesting depth)`)
//! hold only small metadata -- a `usize` mark into `out` and a handful of
//! scalars (`bool`, `u8`, a small `CloseDelim` enum) -- never a copy of
//! accumulated content. Children write **straight through** into `out`.
//!
//! Every DokuWiki construct's textual prefix is knowable at the moment its
//! `Start*` event arrives (heading marker count comes from `level`; list
//! item marker/indent comes from the already-open `List` frame; table cell
//! delimiter comes from the already-open `TableRow` frame) -- `build.rs`'s
//! own logic never pads or counts anything derived from content that hasn't
//! arrived yet (no column-width padding, no computed underline). So every
//! construct here is **write-through**, with one exception:
//!
//! - `DefinitionDesc` is the one construct whose emitted text depends on
//!   content not yet seen at open time: `build_block` only emits the `": "`
//!   lead-in (and the trailing `"\n"`) when the description turns out
//!   non-empty (see `build.rs`'s `if !item.desc.is_empty()`). Rather than
//!   buffer the description separately, `EndDefinitionDesc` checks whether
//!   anything was written to `out` since `StartDefinitionDesc`'s mark and,
//!   if so, `insert_str`s the lead-in at that mark now that the outcome is
//!   known -- an O(1) in-place insert into the tail of the shared buffer,
//!   not a second buffer.
//!
//! A `ListItem`'s own inline content (its `item.inlines`) is written as it
//! arrives, but the single `"\n"` that separates it from any nested child
//! blocks (`item.children`) can only be emitted once we know whether a child
//! block follows -- so it is written lazily, the moment the first child
//! block's `Start*`/leaf event arrives (or at `EndListItem`, if there were
//! no children). This is tracked by a single `bool` on the `ListItem` frame,
//! not a second buffer.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity, so the buffer is allocated once for the whole document) as
//! soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use dokuwiki::writer::Writer;
//! use dokuwiki::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::Event;
use std::io::Write;

/// The closing text for a `Frame::Inline` span. Every inline span DokuWiki
/// supports closes with a fixed string known the moment it opens (unlike,
/// say, an RST role, none of these depend on data collected while the span
/// was open) -- storing the discriminant instead of `&'static str` keeps
/// `Frame::Inline` a plain, cheaply-copied scalar payload.
#[derive(Clone, Copy)]
enum CloseDelim {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Superscript,
    Subscript,
    Link,
}

impl CloseDelim {
    fn as_str(self) -> &'static str {
        match self {
            CloseDelim::Bold => "**",
            CloseDelim::Italic => "//",
            CloseDelim::Underline => "__",
            CloseDelim::Strikethrough => "</del>",
            CloseDelim::Superscript => "</sup>",
            CloseDelim::Subscript => "</sub>",
            CloseDelim::Link => "]]",
        }
    }
}

/// Frames carry only a mark into the shared buffer and tiny scalars --
/// never accumulated content. `mark` is where this construct's output
/// begins in `Writer::out`, so an invalidly-nested construct (its `End*`
/// arrives while the enclosing frame turns out not to accept it) can be
/// discarded wholesale by truncating back to it.
enum Frame {
    Paragraph {
        mark: usize,
        /// Whether the immediate parent (at `StartParagraph` time) was a
        /// `Blockquote` -- `build_block`'s `Blockquote` arm special-cases
        /// its direct `Paragraph` children (`"> " + inlines + "\n"`)
        /// instead of the generic `inlines + "\n\n"`.
        blockquote_mode: bool,
    },
    Heading {
        mark: usize,
        equals_count: u8,
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
        /// Whether the "\n" that separates this item's own inline content
        /// from any nested child blocks has been written yet -- see the
        /// module doc comment.
        wrote_newline: bool,
    },
    Table {
        mark: usize,
    },
    TableRow {
        mark: usize,
        is_header: bool,
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
    /// See the module doc comment -- the `": "` lead-in is inserted at
    /// `mark` at `EndDefinitionDesc` only if the description turned out
    /// non-empty.
    DefinitionDesc {
        mark: usize,
    },
    /// Any inline span whose closing text is a fixed string known when the
    /// span opens (bold/italic/underline/strikethrough/superscript/
    /// subscript/link).
    Inline {
        mark: usize,
        close: CloseDelim,
    },
}

/// Default capacity reserved for `Writer::out` by [`Writer::new`]. Skips the
/// first several doubling reallocations (pure overhead below any realistic
/// block size) without committing to a document-specific guess. Callers
/// with a better estimate should use [`Writer::with_capacity`].
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Streaming DokuWiki writer.
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
    /// assembled. Empty at top level -- a block closing with an empty stack
    /// is flushed to the sink immediately.
    stack: Vec<Frame>,
    /// Mirrors `BuildContext::list_depth`: incremented when a `List` frame
    /// is pushed (`StartList`), decremented when it is popped (`EndList`).
    list_depth: usize,
}

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
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Flush any remaining buffered bytes and recover the underlying sink.
    /// Every completed top-level block was already flushed by
    /// `write_event`; this only guards against a stream that ended without
    /// closing every open construct.
    pub fn finish(mut self) -> W {
        self.flush();
        self.sink
    }

    // -- Buffer primitives ---------------------------------------------

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

    /// Whether the top-of-stack frame accepts block children -- mirrors
    /// `DocBuilder::push_block`'s match arms (`Document`, `Blockquote`,
    /// `ListItem`; nothing else).
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote { .. } | Frame::ListItem { .. })
        )
    }

    /// Whether the top-of-stack frame accepts inline children -- mirrors
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
            )
        )
    }

    /// Open a block-level construct: flush the pending list-item
    /// continuation "\n" if this is the first child block of a `ListItem`
    /// (see the module doc comment), and return the mark to record on the
    /// new frame (or to pass to `block_end` for a single-event leaf block).
    fn block_start(&mut self) -> usize {
        if let Some(Frame::ListItem { wrote_newline, .. }) = self.stack.last_mut()
            && !*wrote_newline
        {
            *wrote_newline = true;
            self.out.push('\n');
        }
        self.out.len()
    }

    /// Close a block-level construct: discard everything written since
    /// `mark` if the (now current, i.e. parent) top-of-stack frame does not
    /// accept block children.
    fn block_end(&mut self, mark: usize) {
        if !self.accepts_blocks() {
            self.out.truncate(mark);
        }
    }

    /// Open an inline span with a statically-known opening/closing pair.
    fn open_inline(&mut self, open: &str, close: CloseDelim) {
        let mark = self.out.len();
        self.push_out(open);
        self.stack.push(Frame::Inline { mark, close });
    }

    /// Close an inline span: discard everything written since its mark
    /// (including the closing text just written) if the parent frame does
    /// not accept inline children.
    fn close_inline(&mut self) {
        if let Some(Frame::Inline { mark, close }) = self.stack.pop() {
            self.push_out(close.as_str());
            if !self.accepts_inline() {
                self.out.truncate(mark);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event<'_>) {
        match event {
            // -- Block open/close --------------------------------------
            Event::StartParagraph => {
                let mark = self.block_start();
                let blockquote_mode = matches!(self.stack.last(), Some(Frame::Blockquote { .. }));
                if blockquote_mode {
                    self.push_out("> ");
                }
                self.stack.push(Frame::Paragraph {
                    mark,
                    blockquote_mode,
                });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph {
                    mark,
                    blockquote_mode,
                }) = self.stack.pop()
                {
                    self.push_out(if blockquote_mode { "\n" } else { "\n\n" });
                    self.block_end(mark);
                }
            }
            Event::StartHeading { level } => {
                let mark = self.block_start();
                let equals_count = (7 - (level as usize).min(6)) as u8;
                for _ in 0..equals_count {
                    self.push_out("=");
                }
                self.push_out(" ");
                self.stack.push(Frame::Heading { mark, equals_count });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { mark, equals_count }) = self.stack.pop() {
                    self.push_out(" ");
                    for _ in 0..equals_count {
                        self.push_out("=");
                    }
                    self.push_out("\n\n");
                    self.block_end(mark);
                }
            }
            Event::StartBlockquote => {
                let mark = self.block_start();
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartList { ordered } => {
                let mark = self.block_start();
                self.list_depth += 1;
                self.stack.push(Frame::List { mark, ordered });
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
                let (in_list, ordered) = match self.stack.last() {
                    Some(Frame::List { ordered, .. }) => (true, *ordered),
                    _ => (false, false),
                };
                let mark = self.out.len();
                if in_list {
                    for _ in 0..self.list_depth {
                        self.push_out("  ");
                    }
                    self.push_out(if ordered { "- " } else { "* " });
                }
                self.stack.push(Frame::ListItem {
                    mark,
                    wrote_newline: false,
                });
            }
            Event::EndListItem => {
                if let Some(Frame::ListItem {
                    mark,
                    wrote_newline,
                }) = self.stack.pop()
                {
                    if !wrote_newline {
                        self.push_out("\n");
                    }
                    if !matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::CodeBlock { language, content } => {
                let mark = self.block_start();
                self.push_out("<code");
                if let Some(lang) = &language {
                    self.push_out(" ");
                    self.push_out(lang);
                }
                self.push_out(">\n");
                self.push_out(content.as_ref());
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("</code>\n\n");
                self.block_end(mark);
            }
            Event::FileBlock {
                language,
                filename,
                content,
            } => {
                let mark = self.block_start();
                self.push_out("<file");
                if let Some(lang) = &language {
                    self.push_out(" ");
                    self.push_out(lang);
                }
                if let Some(fname) = &filename {
                    if language.is_none() {
                        self.push_out(" ");
                    }
                    self.push_out(" ");
                    self.push_out(fname);
                }
                self.push_out(">\n");
                self.push_out(content.as_ref());
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("</file>\n\n");
                self.block_end(mark);
            }
            Event::HorizontalRule => {
                let mark = self.block_start();
                self.push_out("----\n\n");
                self.block_end(mark);
            }
            Event::StartTable => {
                let mark = self.block_start();
                self.stack.push(Frame::Table { mark });
            }
            Event::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartTableRow { is_header } => {
                let mark = self.out.len();
                self.stack.push(Frame::TableRow { mark, is_header });
            }
            Event::EndTableRow => {
                if let Some(Frame::TableRow { mark, is_header }) = self.stack.pop() {
                    self.push_out(if is_header { "^" } else { "|" });
                    self.push_out("\n");
                    if !matches!(self.stack.last(), Some(Frame::Table { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartTableCell => {
                let (in_row, is_header) = match self.stack.last() {
                    Some(Frame::TableRow { is_header, .. }) => (true, *is_header),
                    _ => (false, false),
                };
                let mark = self.out.len();
                if in_row {
                    self.push_out(if is_header { "^" } else { "|" });
                    self.push_out(" ");
                }
                self.stack.push(Frame::TableCell { mark });
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                        self.push_out(" ");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartDefinitionList => {
                let mark = self.block_start();
                self.stack.push(Frame::DefinitionList { mark });
            }
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            Event::StartDefinitionTerm => {
                let in_deflist = matches!(self.stack.last(), Some(Frame::DefinitionList { .. }));
                let mark = self.out.len();
                if in_deflist {
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
                self.stack.push(Frame::DefinitionDesc { mark });
            }
            Event::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::DefinitionList { .. })) {
                        // Only emit the "; "-mirroring ": " lead-in (and
                        // trailing "\n") if the description turned out
                        // non-empty -- see the module doc comment.
                        if self.out.len() > mark {
                            self.out.insert_str(mark, ": ");
                            self.out.push('\n');
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::RawBlock { format, content } => {
                let mark = self.block_start();
                self.push_out("<");
                self.push_out(&format);
                self.push_out(">\n");
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("</");
                self.push_out(&format);
                self.push_out(">\n\n");
                self.block_end(mark);
            }
            Event::Macro { name } => {
                let mark = self.block_start();
                self.push_out("~~");
                self.push_out(&name);
                self.push_out("~~\n\n");
                self.block_end(mark);
            }

            // -- Inline events ------------------------------------------
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
                    self.push_out("\\\\ ");
                }
            }
            Event::StartBold => self.open_inline("**", CloseDelim::Bold),
            Event::EndBold => self.close_inline(),
            Event::StartItalic => self.open_inline("//", CloseDelim::Italic),
            Event::EndItalic => self.close_inline(),
            Event::StartUnderline => self.open_inline("__", CloseDelim::Underline),
            Event::EndUnderline => self.close_inline(),
            Event::StartStrikethrough => self.open_inline("<del>", CloseDelim::Strikethrough),
            Event::EndStrikethrough => self.close_inline(),
            Event::StartSuperscript => self.open_inline("<sup>", CloseDelim::Superscript),
            Event::EndSuperscript => self.close_inline(),
            Event::StartSubscript => self.open_inline("<sub>", CloseDelim::Subscript),
            Event::EndSubscript => self.close_inline(),
            Event::InlineCode(cow) => {
                if self.accepts_inline() {
                    self.push_out("''");
                    self.push_out(&cow);
                    self.push_out("''");
                }
            }
            Event::Nowiki(cow) => {
                if self.accepts_inline() {
                    self.push_out("%%");
                    self.push_out(&cow);
                    self.push_out("%%");
                }
            }
            Event::StartLink { url } => {
                let mark = self.out.len();
                self.push_out("[[");
                self.push_out(&url);
                self.push_out("|");
                self.stack.push(Frame::Inline {
                    mark,
                    close: CloseDelim::Link,
                });
            }
            Event::EndLink => self.close_inline(),
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
            Event::FootnoteRef { content } => {
                if self.accepts_inline() {
                    self.push_out("((");
                    self.push_out(&content);
                    self.push_out("))");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;

    // A single process-wide instrumented allocator, shared by every
    // allocation-shape test below. Rust permits at most one
    // `#[global_allocator]` item per binary, so both the allocation-count
    // and peak-live-bytes tests read from these same statics rather than
    // each declaring their own allocator.
    //
    // `ALLOCS` is a monotonic count, safe as a process-wide atomic (other
    // threads' concurrent allocations only ever push it up, never distort a
    // before/after delta's *shape*, and the allocation-count test below
    // tolerates a generous 20x ceiling). `CURRENT`/`PEAK`, by contrast, are
    // tracked **thread-local**, not as shared atomics: `cargo test` runs
    // this crate's other tests concurrently on multiple threads by default,
    // and a shared peak counter lets an unrelated concurrently-running
    // test's allocations inflate the measured peak -- confirmed as a real
    // flake elsewhere in the wiki-format streaming-writer sweep (a spurious
    // 407x ratio for pod-fmt under full-workspace `cargo test -q`, passing
    // cleanly under `--test-threads=1`). Thread-local counters make the
    // peak-memory measurement immune to what other threads in the same
    // binary do, so `TEST_LOCK` only needs to serialize this file's two
    // allocator-instrumented tests against each other, not against every
    // other test in the crate.
    #[allow(unsafe_code)]
    mod alloc_probe {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;
        use std::sync::Mutex;
        use std::sync::atomic::{AtomicUsize, Ordering};

        pub static ALLOCS: AtomicUsize = AtomicUsize::new(0);
        pub static TEST_LOCK: Mutex<()> = Mutex::new(());

        thread_local! {
            pub static CURRENT: Cell<i64> = const { Cell::new(0) };
            pub static PEAK: Cell<i64> = const { Cell::new(0) };
        }

        struct InstrumentedAlloc;
        unsafe impl GlobalAlloc for InstrumentedAlloc {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                let ptr = unsafe { System.alloc(layout) };
                if !ptr.is_null() {
                    ALLOCS.fetch_add(1, Ordering::Relaxed);
                    let cur = CURRENT.with(|c| {
                        let v = c.get() + layout.size() as i64;
                        c.set(v);
                        v
                    });
                    PEAK.with(|p| {
                        if cur > p.get() {
                            p.set(cur);
                        }
                    });
                }
                ptr
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                CURRENT.with(|c| c.set(c.get() - layout.size() as i64));
                unsafe { System.dealloc(ptr, layout) }
            }
        }
        #[global_allocator]
        static GLOBAL: InstrumentedAlloc = InstrumentedAlloc;
    }

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
        assert!(s.contains("====== Hello ======"), "got: {s:?}");
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
        let input = "====== Hello ======\n\nA paragraph with **bold** text.\n\n  * item one\n  * item two\n";
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
    /// tree-based `build()` for the same document. This is the guard that
    /// keeps the two independent emission paths honest across every
    /// construct DokuWiki has, including the one deferred one
    /// (`DefinitionDesc`'s content-dependent `": "` lead-in) and the one
    /// lazily-written one (`ListItem`'s inline/children separator).
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "====== Title ======\n\nA paragraph with **bold**, //italic//, __underline__, \
             <del>strike</del>, <sup>sup</sup>, <sub>sub</sub>, ''code'', %%no**wiki**%%.\n",
            "  * item one\n  * item two\n    * nested one\n    * nested two\n  * item three\n",
            "  - one\n  - two\n  - three\n",
            "^ Name ^ Age ^\n| Alice | 30 |\n| Bob | 25 |\n",
            "; Term\n: Description\n",
            "; TermOnly\n",
            "> quoted text spanning the blockquote\n",
            "<code rust>\nfn main() {}\n</code>\n",
            "<file txt hello.txt>\nsome content\n</file>\n",
            "<html>\n<b>raw</b>\n</html>\n",
            "~~NOTOC~~\n",
            "----\n",
            "Click [[https://example.com|here]] and see {{img.png|alt text}} and a \
             footnote((note text)).\n",
            "line one\\\\ line two\n",
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

    /// Nested lists are the trickiest write-through path: `list_depth`
    /// bookkeeping happens on `Writer` itself (bracketing `StartList`/
    /// `EndList`), and each nested list's marker/indent is written straight
    /// into the shared buffer as soon as its own `StartListItem` fires.
    #[test]
    fn test_writer_roundtrip_nested_lists() {
        let input = "  * outer one\n  * outer two\n    * inner a\n    * inner b\n  * outer three\n";
        let (doc, _) = crate::parse::parse(input);

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
            "nested list roundtrip block count mismatch\nemitted:\n{emitted_text}"
        );

        fn count_lists(blocks: &[crate::ast::Block]) -> usize {
            let mut n = 0;
            for b in blocks {
                if let crate::ast::Block::List { items, .. } = b {
                    n += 1;
                    for item in items {
                        n += count_lists(&item.children);
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
    /// subtree reconstruction (the original defect: `write_event` buffered
    /// every event and `finish()` rebuilt the whole AST, then ran
    /// `emit::build` on it). A large, deeply-nested event stream must
    /// complete with an allocation count that stays close to linear in
    /// event count, not blow up the way tree materialization (an
    /// `Inline`/`Block` enum + `Vec` per node, on top of the formatting
    /// pass) would. This does not merely check output correctness -- the
    /// existing roundtrip tests already do that -- it checks the *cost
    /// shape* of getting there.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use alloc_probe::{ALLOCS, TEST_LOCK};
        use std::sync::atomic::Ordering;

        let _guard = TEST_LOCK.lock().unwrap();

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
                evs.push(OwnedEvent::StartBold);
                evs.push(OwnedEvent::Text(Cow::Owned("bold".to_string())));
                evs.push(OwnedEvent::EndBold);
                evs.push(OwnedEvent::EndParagraph);
                evs.push(OwnedEvent::StartList { ordered: false });
                for j in 0..2 {
                    evs.push(OwnedEvent::StartListItem);
                    evs.push(OwnedEvent::Text(Cow::Owned(format!("item {j}"))));
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

    /// Peak memory must stay within a small constant multiple of a single
    /// top-level block/event, not scale with the full document -- the
    /// direct proof that the sink actually receives bytes incrementally
    /// rather than everything being buffered until `finish()`. Tracks
    /// current and maximum live bytes via a global allocator wrapper (there
    /// is no portable in-process RSS query, so this counts allocator
    /// traffic instead, which is the more precise signal anyway: it can't
    /// be fooled by the OS not having reclaimed freed pages yet).
    #[test]
    fn test_writer_peak_memory_bounded() {
        use alloc_probe::{CURRENT, PEAK, TEST_LOCK};
        use std::cell::Cell;
        use std::io;

        let _guard = TEST_LOCK.lock().unwrap();

        // A sink that discards bytes rather than accumulating them, so the
        // measured peak reflects the *writer's* own memory use rather than
        // a growing `Vec<u8>` destination (which any real streaming
        // consumer -- a file, a socket -- would not hold in memory either).
        struct CountingSink {
            total: usize,
        }
        impl Write for CountingSink {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.total += buf.len();
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        // A single representative paragraph, used as the "one block" size
        // unit the peak is compared against.
        const PARA_TEXT: &str = "A modestly sized paragraph of representative body text used \
             as the per-block size unit for this test, repeated many thousands of times.";
        const N: usize = 20_000;
        let one_block_size = PARA_TEXT.len() + "\n\n".len();
        let full_doc_size = one_block_size * N;

        CURRENT.with(|c| c.set(0));
        PEAK.with(|p| p.set(0));

        let mut sink = CountingSink { total: 0 };
        {
            let mut w = Writer::new(&mut sink);
            // Events are generated and fed one at a time, in the same loop
            // that drives the writer, rather than pre-built into one giant
            // `Vec<OwnedEvent>` -- that would let the whole event stream's
            // owned strings sit alive simultaneously, which is exactly the
            // O(full document) shape this test exists to rule out for the
            // *writer*, and would also misattribute the vec's own memory
            // to the writer.
            for _ in 0..N {
                w.write_event(OwnedEvent::StartParagraph);
                w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned(
                    PARA_TEXT.to_string(),
                )));
                w.write_event(OwnedEvent::EndParagraph);
            }
            w.finish();
        }
        assert_eq!(sink.total, full_doc_size, "sink did not receive all bytes");

        // `PEAK` is thread-local and signed; a peak reading at or below
        // zero just means no net-positive live allocation was observed on
        // this thread during the measurement window, which trivially
        // satisfies both bounds below.
        let peak = PEAK.with(Cell::get).max(0) as usize;

        // The peak must stay well under the full document's size -- proof
        // the writer is not holding the whole thing in `out` at once. A
        // generous ceiling (64x a single block, plus a fixed floor for
        // allocator bookkeeping/fragmentation overhead) comfortably
        // separates "streaming" from "buffered".
        let ceiling = one_block_size * 64 + 1_000_000;
        assert!(
            peak < full_doc_size,
            "peak allocator bytes ({peak}) was not smaller than the full document size \
             ({full_doc_size}) -- writer is not streaming"
        );
        assert!(
            peak < ceiling,
            "peak allocator bytes ({peak}) exceeded the O(single block) ceiling ({ceiling})"
        );
    }
}
