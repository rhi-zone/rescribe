//! Streaming Jira wiki markup writer — converts a stream of events directly
//! to Jira text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`]. It is a second, independent
//! emission path from the tree-based `parse()`/`build()` functions, not a
//! thin wrapper around them.
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document. The frame stack (`O(nesting depth)`) holds only small
//! metadata — a `usize` mark into `out`, a bool/enum, occasionally a short
//! owned `String` for a link URL — never a copy of accumulated child
//! content. Children write **straight through** into `out`.
//!
//! Every Jira construct's *prefix* is knowable at its `Start*`/leaf event:
//! heading level, list marker/ordering, panel title, code fence language,
//! link URL, table cell header-ness. None of Jira wiki markup's constructs
//! need a byte count, width, or other content-derived prefix the way RST's
//! heading underlines or column-padded tables do — the **one** exception is
//! a table row's closing `||`/`|` delimiter, which `build_table` derives
//! from the row's *first* cell's header-ness. That is tracked as a single
//! `Option<bool>` set once per row (the first `StartTableCell`), not
//! buffered content — still `O(1)` per row, not `O(row size)`.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use jira_fmt::writer::Writer;
//! use jira_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming Jira wiki markup writer.
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

/// Default capacity reserved for `Writer::out`. Skips the first several
/// geometric-growth doublings (pure overhead below any realistic block
/// size) without committing to a document-specific guess.
const DEFAULT_OUT_CAPACITY: usize = 4096;

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            out: String::with_capacity(DEFAULT_OUT_CAPACITY),
            stack: Vec::new(),
            list_depth: 0,
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

    // ── Buffer primitives ───────────────────────────────────────────────

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

    /// Whether the top-of-stack frame accepts block children directly
    /// (`Block::Blockquote`/`Block::Panel`, or the document root). `List`,
    /// `Table`, `TableRow`, `TableCell`, and every inline span are not valid
    /// block parents. `ListItem` is handled separately at each call site
    /// (it "accepts" every block kind, but with the special
    /// paragraph-becomes-inline demotion `build_block`'s
    /// `push_block`/`ListItemContent` does).
    fn block_accepts(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::Blockquote { .. }) | Some(Frame::Panel { .. })
        )
    }

    fn in_list_item(&self) -> bool {
        matches!(self.stack.last(), Some(Frame::ListItem { .. }))
    }

    fn inline_accepts(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::Bold { .. }
                    | Frame::Italic { .. }
                    | Frame::Underline { .. }
                    | Frame::Strikethrough { .. }
                    | Frame::Superscript { .. }
                    | Frame::Subscript { .. }
                    | Frame::Link { .. }
                    | Frame::ColorSpan { .. }
                    | Frame::TableCell { .. }
            )
        )
    }

    /// Close a block-shaped construct written unconditionally at `mark..`:
    /// keep it (and flush if this just emptied the stack) if the enclosing
    /// frame accepts blocks (directly, or via `ListItem`'s "any block is a
    /// `NestedList`" rule), otherwise discard the whole thing.
    fn block_end(&mut self, mark: usize) {
        if self.block_accepts() || self.in_list_item() {
            if self.stack.is_empty() {
                self.flush();
            }
        } else {
            self.out.truncate(mark);
        }
    }

    /// Open an inline span: write the opening delimiter unconditionally
    /// (removed again at close if the context turns out invalid) and return
    /// the mark to truncate back to.
    fn open_span(&mut self, open: &str) -> usize {
        let mark = self.out.len();
        self.out.push_str(open);
        mark
    }

    /// Close an inline span: write the closing delimiter, then discard the
    /// whole `mark..` region (open + content + close) if the enclosing
    /// frame does not accept inline children.
    fn close_span(&mut self, mark: usize, close: &str) {
        self.out.push_str(close);
        if !self.inline_accepts() {
            self.out.truncate(mark);
        }
    }

    /// Every non-`Paragraph` block construct opening as a child of a
    /// `ListItem` gets a leading `"\n"` — mirrors `build_block`'s
    /// `ListItemContent::NestedList` arm (`ctx.write("\n"); build_block(...)`).
    /// `Paragraph` is exempt: `ListItemContent::Inline` has no such prefix,
    /// since it is not treated as a nested block at all.
    fn nested_list_item_prefix(&mut self) {
        if self.in_list_item() {
            self.push_out("\n");
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedEvent) {
        match event {
            // ── Block open/close ─────────────────────────────────────────
            OwnedEvent::StartParagraph => {
                let mark = self.out.len();
                self.stack.push(Frame::Paragraph { mark });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    if self.in_list_item() {
                        // `ListItemContent::Inline`: no "\n\n" separator,
                        // content stays exactly as written.
                    } else if self.block_accepts() {
                        self.push_out("\n\n");
                        if self.stack.is_empty() {
                            self.flush();
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::StartHeading { level } => {
                let mark = self.out.len();
                self.nested_list_item_prefix();
                self.push_out(&format!("h{level}. "));
                self.stack.push(Frame::Heading { mark });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { mark }) = self.stack.pop() {
                    self.push_out("\n\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartBlockquote => {
                let mark = self.out.len();
                self.nested_list_item_prefix();
                self.push_out("{quote}\n");
                self.stack.push(Frame::Blockquote { mark });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.push_out("{quote}\n\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartPanel { title } => {
                let mark = self.out.len();
                self.nested_list_item_prefix();
                if let Some(t) = &title {
                    self.push_out("{panel:title=");
                    self.push_out(t);
                    self.push_out("}\n");
                } else {
                    self.push_out("{panel}\n");
                }
                self.stack.push(Frame::Panel { mark });
            }
            OwnedEvent::EndPanel => {
                if let Some(Frame::Panel { mark }) = self.stack.pop() {
                    self.push_out("{panel}\n\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartList { ordered } => {
                let mark = self.out.len();
                self.nested_list_item_prefix();
                self.list_depth += 1;
                self.stack.push(Frame::List { mark, ordered });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    self.list_depth -= 1;
                    if self.list_depth == 0 {
                        self.push_out("\n");
                    }
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartListItem => {
                let mark = self.out.len();
                if let Some(Frame::List { ordered, .. }) = self.stack.last() {
                    let marker = if *ordered { '#' } else { '*' };
                    for _ in 0..self.list_depth {
                        self.out.push(marker);
                    }
                    self.push_out(" ");
                }
                self.stack.push(Frame::ListItem { mark });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    if !matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::CodeBlock { language, content } => {
                let mark = self.out.len();
                self.nested_list_item_prefix();
                if let Some(lang) = &language {
                    self.push_out("{code:");
                    self.push_out(lang);
                    self.push_out("}\n");
                } else {
                    self.push_out("{code}\n");
                }
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("{code}\n\n");
                self.block_end(mark);
            }
            OwnedEvent::Noformat { content } => {
                let mark = self.out.len();
                self.nested_list_item_prefix();
                self.push_out("{noformat}\n");
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("{noformat}\n\n");
                self.block_end(mark);
            }
            OwnedEvent::HorizontalRule => {
                let mark = self.out.len();
                self.nested_list_item_prefix();
                self.push_out("----\n\n");
                self.block_end(mark);
            }
            OwnedEvent::StartTable => {
                let mark = self.out.len();
                self.nested_list_item_prefix();
                self.stack.push(Frame::Table { mark });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    self.push_out("\n");
                    self.block_end(mark);
                }
            }
            OwnedEvent::StartTableRow => {
                let mark = self.out.len();
                self.stack.push(Frame::TableRow {
                    mark,
                    row_is_header: None,
                });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow {
                    mark,
                    row_is_header,
                }) = self.stack.pop()
                {
                    self.push_out(if row_is_header.unwrap_or(false) {
                        "||\n"
                    } else {
                        "|\n"
                    });
                    if !matches!(self.stack.last(), Some(Frame::Table { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::StartTableCell { is_header } => {
                let mark = self.out.len();
                let in_row = matches!(self.stack.last(), Some(Frame::TableRow { .. }));
                if let Some(Frame::TableRow { row_is_header, .. }) = self.stack.last_mut()
                    && row_is_header.is_none()
                {
                    *row_is_header = Some(is_header);
                }
                if in_row {
                    self.push_out(if is_header { "||" } else { "|" });
                }
                self.stack.push(Frame::TableCell { mark });
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell { mark }) = self.stack.pop()
                    && !matches!(self.stack.last(), Some(Frame::TableRow { .. }))
                {
                    self.out.truncate(mark);
                }
            }

            // ── Inline events ────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                if self.inline_accepts() {
                    self.push_out(&cow);
                }
            }
            OwnedEvent::StartBold => {
                let mark = self.open_span("*");
                self.stack.push(Frame::Bold { mark });
            }
            OwnedEvent::EndBold => {
                if let Some(Frame::Bold { mark }) = self.stack.pop() {
                    self.close_span(mark, "*");
                }
            }
            OwnedEvent::StartItalic => {
                let mark = self.open_span("_");
                self.stack.push(Frame::Italic { mark });
            }
            OwnedEvent::EndItalic => {
                if let Some(Frame::Italic { mark }) = self.stack.pop() {
                    self.close_span(mark, "_");
                }
            }
            OwnedEvent::StartUnderline => {
                let mark = self.open_span("+");
                self.stack.push(Frame::Underline { mark });
            }
            OwnedEvent::EndUnderline => {
                if let Some(Frame::Underline { mark }) = self.stack.pop() {
                    self.close_span(mark, "+");
                }
            }
            OwnedEvent::StartStrikethrough => {
                let mark = self.open_span("-");
                self.stack.push(Frame::Strikethrough { mark });
            }
            OwnedEvent::EndStrikethrough => {
                if let Some(Frame::Strikethrough { mark }) = self.stack.pop() {
                    self.close_span(mark, "-");
                }
            }
            OwnedEvent::StartSuperscript => {
                let mark = self.open_span("^");
                self.stack.push(Frame::Superscript { mark });
            }
            OwnedEvent::EndSuperscript => {
                if let Some(Frame::Superscript { mark }) = self.stack.pop() {
                    self.close_span(mark, "^");
                }
            }
            OwnedEvent::StartSubscript => {
                let mark = self.open_span("~");
                self.stack.push(Frame::Subscript { mark });
            }
            OwnedEvent::EndSubscript => {
                if let Some(Frame::Subscript { mark }) = self.stack.pop() {
                    self.close_span(mark, "~");
                }
            }
            OwnedEvent::InlineCode(cow) => {
                if self.inline_accepts() {
                    self.push_out("{{");
                    self.push_out(&cow);
                    self.push_out("}}");
                }
            }
            OwnedEvent::StartLink { url } => {
                let mark = self.out.len();
                self.push_out("[");
                self.stack.push(Frame::Link { mark, url });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { mark, url }) = self.stack.pop() {
                    self.push_out("|");
                    self.push_out(&url);
                    self.push_out("]");
                    if !self.inline_accepts() {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::InlineImage { url, alt } => {
                if self.inline_accepts() {
                    self.push_out("!");
                    self.push_out(&url);
                    if let Some(a) = &alt {
                        self.push_out("|");
                        self.push_out(a);
                    }
                    self.push_out("!");
                }
            }
            OwnedEvent::StartColorSpan { color } => {
                let mark = self.out.len();
                self.push_out("{color:");
                self.push_out(&color);
                self.push_out("}");
                self.stack.push(Frame::ColorSpan { mark });
            }
            OwnedEvent::EndColorSpan => {
                if let Some(Frame::ColorSpan { mark }) = self.stack.pop() {
                    self.close_span(mark, "{color}");
                }
            }
            OwnedEvent::Mention(cow) => {
                if self.inline_accepts() {
                    self.push_out("@");
                    self.push_out(&cow);
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
    },
    Blockquote {
        mark: usize,
    },
    Panel {
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
    /// `row_is_header` mirrors `build_table`'s `row_is_header` local: set
    /// once, from the row's *first* cell, and read back at `EndTableRow` to
    /// choose the closing `"||"`/`"|"`. `O(1)` per row, not a buffered copy
    /// of the row's cells.
    TableRow {
        mark: usize,
        row_is_header: Option<bool>,
    },
    TableCell {
        mark: usize,
    },
    Bold {
        mark: usize,
    },
    Italic {
        mark: usize,
    },
    Underline {
        mark: usize,
    },
    Strikethrough {
        mark: usize,
    },
    Superscript {
        mark: usize,
    },
    Subscript {
        mark: usize,
    },
    /// The one inline span whose closing text depends on data carried by
    /// the frame (the URL, moved out of the opening event — not a content
    /// buffer).
    Link {
        mark: usize,
        url: String,
    },
    ColorSpan {
        mark: usize,
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
        assert!(s.contains("h1. Hello"), "got: {s:?}");
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
        let input = "h1. Hello\n\nA paragraph with *bold* text.\n\n* item one\n* item two\n";
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
    /// the two independent emission paths honest, including the one
    /// content-dependent construct (a table row's closing `||`/`|`).
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "h1. Title\n\nIntro paragraph with *bold* and _italic_ and {{code}}.\n",
            "h2. Sub\n\ntext with +underline+, -strike-, ^sup^, ~sub~.\n",
            "* bullet one\n* bullet two\n\n** nested a\n** nested b\n",
            "# ordered one\n# ordered two\n",
            "{code:java}\nint x = 1;\nint y = 2;\n{code}\n",
            "{noformat}\nliteral block\n{noformat}\n",
            "{quote}\nA quoted paragraph.\n\nSecond para of quote.\n{quote}\n",
            "{panel:title=Note}\nSome panel body.\n{panel}\n",
            "{panel}\nAnonymous panel body.\n{panel}\n",
            "||A||B||\n|Cell 1|Cell 2|\n",
            "|A|B|\n|Cell 1|Cell 2|\n",
            "----\n\nAfter the transition.\n",
            "A paragraph with a [link|http://example.com/] and an image !img.png|alt text!.\n",
            "A paragraph mentioning @someone and {color:red}red text{color}.\n",
            "* item\n** nested item\n* item two\n\n# nested ordered\n# more\n",
            "A para\n\n* item\n\ncontinued paragraph\n",
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
h1. Title

Intro paragraph with *bold* and {{code}}.

{quote}
A block quote.
{quote}

* bullet one
* bullet two

# ordered one
# ordered two

{code:rust}
let x = 1;
{code}

||A||B||
|Cell 1|Cell 2|

----

After the transition.
";
        let (doc, _) = crate::parse::parse(input);
        assert!(
            doc.blocks.len() >= 7,
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
            "writer roundtrip block count mismatch\ninput blocks: {:#?}\nemitted text: \
             {emitted_text}\nreparsed blocks: {:#?}",
            doc.blocks,
            doc2.blocks,
        );
    }

    /// Nested lists (bullet-in-bullet) are the trickiest path in the
    /// streaming rewrite: `list_depth` bookkeeping happens on `Writer`
    /// itself (bracketing `StartList`/`EndList`), and each nested list's
    /// marker is written straight into the shared buffer at
    /// `StartListItem`. This asserts the streaming `Writer` matches
    /// `crate::emit::build()` byte-for-byte for a nested-list document —
    /// *not* that the emitted text re-parses back to the same block count.
    /// It doesn't: `build_block`'s `List` arm unconditionally writes a
    /// trailing `"\n"` after each item's `item.children` loop, including
    /// when the item's last child was itself a nested list (whose own last
    /// item already wrote its own trailing `"\n"`) — that stacks into a
    /// blank line, which `parse()` reads as the list ending. This is a
    /// pre-existing `build()`/`parse()` round-trip defect (reproduced here:
    /// `crate::emit::build(&crate::parse::parse(input).0)` loses the nested
    /// list on re-parse too), independent of streaming vs. non-streaming —
    /// out of scope for this pass, which is about `Writer` incrementality,
    /// not `build()`'s own construct coverage. Tracked in TODO.md by the
    /// process that owns cross-cutting fidelity gaps.
    #[test]
    fn test_writer_roundtrip_nested_lists_matches_builder() {
        let input = "\
* outer one
* outer two
** inner a
** inner b
* outer three
";
        let (doc, _) = crate::parse::parse(input);
        let built = crate::emit::build(&doc);

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events(input) {
            w.write_event(e);
        }
        let emitted_text = String::from_utf8(w.finish()).unwrap();

        assert_eq!(
            built, emitted_text,
            "streaming Writer diverged from build() for nested-list input"
        );
    }

    // A single process-wide `#[global_allocator]` tracks both allocation
    // count (for the no-subtree-reconstruction-blowup guard) and
    // current/peak bytes (for the peak-memory guard) — Rust allows only one
    // `#[global_allocator]` per binary, so both tests below share this one
    // rather than each defining their own.
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TrackingAlloc;
    static ALLOCS: AtomicUsize = AtomicUsize::new(0);
    static CURRENT: AtomicUsize = AtomicUsize::new(0);
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    /// `cargo test` runs this crate's ~40 other tests concurrently with
    /// these two allocator-instrumented ones by default, and they all share
    /// the one process-wide `TrackingAlloc` (only one `#[global_allocator]`
    /// per binary). Serializing just these two against *each other* (the
    /// two heavy-allocation outliers in this file) removes the dominant
    /// source of cross-test interference without needing `--test-threads=1`
    /// for the whole binary.
    static ALLOC_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
    unsafe impl GlobalAlloc for TrackingAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
            let prev = CURRENT.fetch_add(layout.size(), Ordering::SeqCst);
            PEAK.fetch_max(prev + layout.size(), Ordering::SeqCst);
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
            unsafe { System.dealloc(ptr, layout) }
        }
    }
    #[global_allocator]
    static GLOBAL: TrackingAlloc = TrackingAlloc;

    /// Regression guard against reintroducing per-block `Block`/`Inline`
    /// subtree reconstruction. A large, deeply-nested event stream must
    /// complete with an allocation count that stays close to linear in
    /// event count, not blow up the way tree materialization would.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        let _guard = ALLOC_TEST_GUARD.lock().unwrap();
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

    /// Peak memory must stay near-flat as document size grows, not
    /// `O(full document)` — the direct proof the writer doesn't buffer the
    /// whole document before emitting. Uses `std::io::sink()` (discards
    /// bytes immediately, retains nothing) rather than a `Vec<u8>` sink —
    /// with a growing `Vec<u8>` sink, the *sink itself* would retain the
    /// full output regardless of how incremental `Writer`'s own internal
    /// state is, defeating the point of the measurement.
    ///
    /// This compares peak growth (relative to a baseline snapshot taken
    /// immediately before each run — `PEAK`/`CURRENT` are process-wide
    /// statics shared with the allocation-count guard above, since only one
    /// `#[global_allocator]` is allowed per binary) across a 100x increase
    /// in document size, the same relative-comparison shape as
    /// `test_writer_no_subtree_reconstruction_blowup` above and for the same
    /// reason: `cargo test` runs other tests concurrently in the same
    /// process, so an absolute byte threshold is noisy, but a writer whose
    /// peak memory is `O(largest top-level block)` should show peak growth
    /// essentially *flat* against a 100x input increase (both runs are a
    /// stream of same-sized single-paragraph top-level blocks), while a
    /// buffer-then-build writer's peak would scale with document size.
    #[test]
    fn test_writer_peak_memory_bounded() {
        let _guard = ALLOC_TEST_GUARD.lock().unwrap();
        fn run(n: usize) -> usize {
            let baseline = CURRENT.load(Ordering::SeqCst);
            PEAK.store(baseline, Ordering::SeqCst);
            {
                let mut w = Writer::new(std::io::sink());
                for i in 0..n {
                    w.write_event(OwnedEvent::StartParagraph);
                    w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned(format!(
                        "Paragraph number {i} with representative body text to pad it out."
                    ))));
                    w.write_event(OwnedEvent::EndParagraph);
                }
                w.finish();
            }
            PEAK.load(Ordering::SeqCst).saturating_sub(baseline).max(1)
        }

        let small = run(2_000);
        let large = run(200_000); // 100x the paragraphs

        // A buffer-then-build writer would show peak growth scaling
        // ~linearly with paragraph count (100x in, ~100x peak out). A
        // bounded writer's peak is dominated by the shared `out` buffer's
        // single-paragraph high-water mark plus fixed overhead, so growth
        // stays well under a 10x ratio even under concurrent-test noise.
        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 10.0,
            "peak memory growth did not stay bounded: {small} bytes @2_000 paragraphs -> \
             {large} bytes @200_000 paragraphs (ratio {ratio:.2}); this suggests the writer is \
             buffering the whole document instead of flushing per top-level block"
        );
    }
}
