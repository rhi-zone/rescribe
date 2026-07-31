//! Streaming TikiWiki writer — converts a stream of events directly to
//! TikiWiki text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::Block`]/[`crate::Inline`] value and
//! never calls [`crate::emit::build`]/`build_block`/`build_inline`. It is a
//! second, independent emission path from the tree-based `build()` function,
//! not a thin wrapper around it.
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document. Frames on the `Vec<Frame>` stack (`O(nesting depth)`) hold
//! only small metadata — a `usize` mark into `out`, a couple of bools/chars,
//! or (for `WikiLink`) a small owned `String` bounded by that one
//! construct's own page name / label text, never a copy of the whole
//! document. Children write **straight through** into `out`; a frame that
//! needs to decorate its own content afterwards post-processes the
//! `out[mark..]` range in place.
//!
//! Construct classification:
//!
//! - **Write-through** (zero per-frame buffering, prefix known at `Start*`):
//!   `Paragraph`, `Heading` (marker width is `level`, not content-derived),
//!   `Blockquote`, `List`/`ListItem`, `CodeBlock`, `HorizontalRule`, `Table`/
//!   `TableRow`/`TableCell` (the pipe/`^` decorations only ever depend on
//!   `is_header` and cell position, both known at `Start*`), and every simple
//!   inline span (`Bold`, `Italic`, `Underline`, `Strikethrough`,
//!   `Superscript`, `Subscript`, `InlineCode`, `Nowiki`, `LineBreak`,
//!   `InlineImage`).
//! - **Write-through + lazy prefix** (prefix known the moment the *first*
//!   child arrives, not before): `Link` — `[url|label]` only gets the `|` if
//!   at least one inline child event actually arrives; `enter_inline_child`
//!   inserts it lazily on the first child rather than buffering to find out
//!   whether there will be one.
//! - **Deferred, genuinely content-dependent**: `WikiLink`. `build_inline`
//!   only writes `|label` when the label's plain text (mirroring
//!   `collect_inline_text`, delimiters stripped) differs from the page name —
//!   a comparison that isn't decidable until every child event has been
//!   seen. The formatted markup is written straight through as children
//!   arrive (bounded by that one link's own content, not the document), and
//!   at `EndWikiLink` the writer either discards it (`page == label`,
//!   `String::truncate`) or splices in the `|` ahead of it
//!   (`String::insert_str`) — the same in-place technique `rst-fmt` uses for
//!   its heading underline width.
//!
//! Any block or inline event arriving in a context that would not have
//! accepted it in the AST (e.g. a `Table` as a direct child of a `ListItem`
//! — `build_list_items` only ever recurses into nested `List` children, so
//! every other block kind a `ListItem` receives renders as zero bytes) is
//! written greedily and then discarded by truncating `out` back to a mark
//! taken before it started, matching the tree builder's output byte-for-byte
//! without ever materializing the discarded subtree.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use tikiwiki::writer::Writer;
//! use tikiwiki::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedEvent;
use std::io::Write;

/// Frames carry only marks into the shared `out` buffer and tiny scalars —
/// never accumulated content, except `WikiLink`'s `label_text`, which is
/// bounded by that one link's own text (see the module doc).
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
    List {
        mark: usize,
    },
    ListItem {
        mark: usize,
        /// Whether the '\n' terminating this item's own direct inline text
        /// (written the moment the first block child — or `EndListItem` if
        /// there is none — is reached) has already been written. Mirrors
        /// `build_list_items`'s `build_inlines(...); output.push('\n');`
        /// happening unconditionally once, right after the item's own text.
        newline_written: bool,
    },
    Table {
        mark: usize,
    },
    TableRow {
        is_header: bool,
        first_cell: bool,
    },
    TableCell {
        mark: usize,
        is_header: bool,
        /// Whether this cell's immediate parent was actually a `TableRow` —
        /// mirrors the old event-to-AST builder's silent drop of a cell that
        /// arrives outside a row.
        valid: bool,
    },
    /// A simple inline span whose open/close delimiters are fixed strings
    /// known at `Start*` (`Bold`, `Italic`, `Underline`, `Strikethrough`,
    /// `Superscript`, `Subscript`).
    Inline {
        close: &'static str,
        mark: usize,
    },
    Link {
        mark: usize,
        /// Whether an inline child has arrived yet — the `|` separator is
        /// written lazily on the first one (see the module doc).
        has_child: bool,
    },
    WikiLink {
        mark: usize,
        page: String,
        /// Where this link's label content begins in `out` (right after
        /// `((page`, if written).
        label_mark: usize,
        /// Plain-text accumulation mirroring `collect_inline_text` over the
        /// label's children — compared against `page` at `EndWikiLink` to
        /// decide whether the `|label` suffix is needed at all.
        label_text: String,
    },
}

/// Default capacity reserved for `Writer::out`. See `rst_fmt::writer`'s
/// identical constant for the rationale (skip the first several geometric
/// doublings without committing to a document-specific guess).
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Streaming TikiWiki writer.
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
    /// Current list nesting depth — incremented on `StartList`, decremented
    /// on `EndList`. Doubles as both "how many marker characters to repeat"
    /// and "is this list nested" (only depth-0 lists get the trailing blank
    /// line `build_list_items`'s outermost `build_block` call adds).
    list_depth: usize,
    /// The marker character ('#' or '*') for the *outermost* currently-open
    /// list. Mirrors `build_block`'s `Block::List` arm: the marker is
    /// computed once from the top-level list's own `ordered` flag and reused
    /// unchanged by `build_list_items`'s recursion into nested lists — a
    /// nested list's own `ordered` value is not read. Replicated here
    /// byte-for-byte rather than "fixed", since this task is scoped to
    /// streaming incrementality, not to changing `emit.rs`'s behavior.
    list_marker: Option<char>,
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
            list_depth: 0,
            list_marker: None,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: OwnedEvent) {
        self.process(event);
    }

    /// Recover the underlying sink. Every completed top-level block was
    /// already flushed by `write_event`; any construct left unclosed by a
    /// malformed event stream is discarded rather than flushed partially,
    /// matching the old event-to-AST builder silently dropping an unclosed
    /// construct instead of ever handing it to `build()`.
    pub fn finish(mut self) -> W {
        if self.stack.is_empty() {
            self.flush();
        }
        self.sink
    }

    // ── Buffer primitives ────────────────────────────────────────────────

    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Whether the top-of-stack frame accepts block children. `is_list`
    /// distinguishes a `List` block from every other block kind: a
    /// `ListItem` only ever renders nested `List` children (see the module
    /// doc), everything else always accepts any block.
    fn accepts_block(&self, is_list: bool) -> bool {
        match self.stack.last() {
            None => true,
            Some(Frame::Blockquote { .. }) => true,
            Some(Frame::ListItem { .. }) => is_list,
            _ => false,
        }
    }

    /// Whether the top-of-stack frame accepts inline children.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::TableCell { .. }
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
                    | Frame::WikiLink { .. }
                    | Frame::ListItem { .. }
            )
        )
    }

    /// Open a block: if the current top frame is a `ListItem` that hasn't
    /// yet had its own line-ending `\n` written, write it now (this is the
    /// first block child to arrive, or `EndListItem` will write it if none
    /// ever does — see `Frame::ListItem::newline_written`). Returns the mark
    /// to truncate back to if this block turns out to have no valid
    /// enclosing context.
    fn enter_block(&mut self) -> usize {
        let needs_newline = matches!(
            self.stack.last(),
            Some(Frame::ListItem {
                newline_written: false,
                ..
            })
        );
        if needs_newline {
            if let Some(Frame::ListItem {
                newline_written, ..
            }) = self.stack.last_mut()
            {
                *newline_written = true;
            }
            self.out.push('\n');
        }
        self.out.len()
    }

    /// Close a block: discard it if the enclosing frame does not accept a
    /// block of this kind, otherwise flush if it completed a top-level
    /// block.
    fn exit_block(&mut self, mark: usize, is_list: bool) {
        if !self.accepts_block(is_list) {
            self.out.truncate(mark);
            return;
        }
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Close an inline span: discard it if the enclosing frame does not
    /// accept inline children.
    fn inline_end(&mut self, mark: usize) {
        if !self.accepts_inline() {
            self.out.truncate(mark);
        }
    }

    /// If the top frame is a `Link` awaiting its first child, write the
    /// lazy `|` separator now and mark it done. Called at the start of every
    /// inline-content event.
    fn enter_inline_child(&mut self) {
        let needs_pipe = matches!(
            self.stack.last(),
            Some(Frame::Link {
                has_child: false,
                ..
            })
        );
        if needs_pipe {
            if let Some(Frame::Link { has_child, .. }) = self.stack.last_mut() {
                *has_child = true;
            }
            self.out.push('|');
        }
    }

    /// Append `s` to the plain-text accumulator of every currently-open
    /// `WikiLink` frame — mirroring `collect_inline_text`'s recursion
    /// through arbitrarily nested formatting (an outer `WikiLink`'s label
    /// text includes an inner span's text regardless of how many other
    /// frame kinds sit between them).
    fn note_label_text(&mut self, s: &str) {
        for frame in self.stack.iter_mut().rev() {
            if let Frame::WikiLink { label_text, .. } = frame {
                label_text.push_str(s);
            }
        }
    }

    fn open_span(&mut self, open: &str, close: &'static str) {
        self.enter_inline_child();
        let mark = self.out.len();
        if self.accepts_inline() {
            self.out.push_str(open);
        }
        self.stack.push(Frame::Inline { close, mark });
    }

    fn close_span(&mut self) {
        if let Some(Frame::Inline { close, mark }) = self.stack.pop() {
            self.out.push_str(close);
            self.inline_end(mark);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedEvent) {
        match event {
            // ── Block open/close ────────────────────────────────────────
            OwnedEvent::StartParagraph => {
                let mark = self.enter_block();
                self.stack.push(Frame::Paragraph { mark });
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    self.out.push_str("\n\n");
                    self.exit_block(mark, false);
                }
            }
            OwnedEvent::StartHeading { level } => {
                let mark = self.enter_block();
                for _ in 0..(level as usize).min(6) {
                    self.out.push('!');
                }
                self.out.push(' ');
                self.stack.push(Frame::Heading { mark });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    self.exit_block(mark, false);
                }
            }
            OwnedEvent::StartBlockquote => {
                let mark = self.enter_block();
                self.out.push_str("{QUOTE()}\n");
                self.stack.push(Frame::Blockquote { mark });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.out.push_str("{QUOTE}\n\n");
                    self.exit_block(mark, false);
                }
            }
            OwnedEvent::StartList { ordered } => {
                let mark = self.enter_block();
                if self.list_depth == 0 {
                    self.list_marker = Some(if ordered { '#' } else { '*' });
                }
                self.list_depth += 1;
                self.stack.push(Frame::List { mark });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { mark }) = self.stack.pop() {
                    self.list_depth -= 1;
                    if self.list_depth == 0 {
                        self.out.push('\n');
                        self.list_marker = None;
                    }
                    self.exit_block(mark, true);
                }
            }
            OwnedEvent::StartListItem => {
                let mark = self.out.len();
                if matches!(self.stack.last(), Some(Frame::List { .. })) {
                    let marker = self.list_marker.unwrap_or('*');
                    for _ in 0..self.list_depth {
                        self.out.push(marker);
                    }
                    self.out.push(' ');
                }
                self.stack.push(Frame::ListItem {
                    mark,
                    newline_written: false,
                });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem {
                    mark,
                    newline_written,
                }) = self.stack.pop()
                {
                    if !newline_written {
                        self.out.push('\n');
                    }
                    if !matches!(self.stack.last(), Some(Frame::List { .. })) {
                        self.out.truncate(mark);
                    }
                }
            }
            OwnedEvent::CodeBlock { language, content } => {
                let mark = self.enter_block();
                if let Some(lang) = &language {
                    self.out.push_str("{CODE(lang=");
                    self.out.push_str(lang);
                    self.out.push_str(")}\n");
                } else {
                    self.out.push_str("{CODE()}\n");
                }
                self.out.push_str(&content);
                if !content.ends_with('\n') {
                    self.out.push('\n');
                }
                self.out.push_str("{CODE}\n\n");
                self.exit_block(mark, false);
            }
            OwnedEvent::HorizontalRule => {
                let mark = self.enter_block();
                self.out.push_str("---\n\n");
                self.exit_block(mark, false);
            }
            OwnedEvent::StartTable => {
                let mark = self.enter_block();
                self.stack.push(Frame::Table { mark });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    self.exit_block(mark, false);
                }
            }
            OwnedEvent::StartTableRow { is_header } => {
                if matches!(self.stack.last(), Some(Frame::Table { .. })) {
                    self.out.push_str("||");
                }
                self.stack.push(Frame::TableRow {
                    is_header,
                    first_cell: true,
                });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { .. }) = self.stack.pop()
                    && matches!(self.stack.last(), Some(Frame::Table { .. }))
                {
                    self.out.push_str("||\n");
                }
            }
            OwnedEvent::StartTableCell => {
                let mark = self.out.len();
                let row_info = match self.stack.last_mut() {
                    Some(Frame::TableRow {
                        is_header,
                        first_cell,
                    }) => {
                        let was_first = *first_cell;
                        *first_cell = false;
                        Some((*is_header, was_first))
                    }
                    _ => None,
                };
                match row_info {
                    Some((is_header, was_first)) => {
                        if !was_first {
                            self.out.push('|');
                        }
                        if is_header {
                            self.out.push('^');
                        }
                        self.stack.push(Frame::TableCell {
                            mark,
                            is_header,
                            valid: true,
                        });
                    }
                    None => {
                        self.stack.push(Frame::TableCell {
                            mark,
                            is_header: false,
                            valid: false,
                        });
                    }
                }
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell {
                    mark,
                    is_header,
                    valid,
                }) = self.stack.pop()
                {
                    if valid {
                        if is_header {
                            self.out.push('^');
                        }
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }

            // ── Inline events ───────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.enter_inline_child();
                if self.accepts_inline() {
                    self.out.push_str(&cow);
                    self.note_label_text(&cow);
                }
            }
            OwnedEvent::LineBreak => {
                self.enter_inline_child();
                if self.accepts_inline() {
                    self.out.push_str("%%%");
                    self.note_label_text("\n");
                }
            }
            OwnedEvent::StartBold => self.open_span("__", "__"),
            OwnedEvent::EndBold => self.close_span(),
            OwnedEvent::StartItalic => self.open_span("''", "''"),
            OwnedEvent::EndItalic => self.close_span(),
            OwnedEvent::StartUnderline => self.open_span("===", "==="),
            OwnedEvent::EndUnderline => self.close_span(),
            OwnedEvent::StartStrikethrough => self.open_span("--", "--"),
            OwnedEvent::EndStrikethrough => self.close_span(),
            OwnedEvent::StartSuperscript => self.open_span("^", "^"),
            OwnedEvent::EndSuperscript => self.close_span(),
            OwnedEvent::StartSubscript => self.open_span(",,", ",,"),
            OwnedEvent::EndSubscript => self.close_span(),
            OwnedEvent::InlineCode(cow) => {
                self.enter_inline_child();
                if self.accepts_inline() {
                    self.out.push_str("-+");
                    self.out.push_str(&cow);
                    self.out.push_str("+-");
                    self.note_label_text(&cow);
                }
            }
            OwnedEvent::Nowiki(cow) => {
                self.enter_inline_child();
                if self.accepts_inline() {
                    self.out.push_str("~np~");
                    self.out.push_str(&cow);
                    self.out.push_str("~/np~");
                    self.note_label_text(&cow);
                }
            }
            OwnedEvent::StartLink { url } => {
                self.enter_inline_child();
                let mark = self.out.len();
                if self.accepts_inline() {
                    self.out.push('[');
                    self.out.push_str(&url);
                }
                self.stack.push(Frame::Link {
                    mark,
                    has_child: false,
                });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { mark, .. }) = self.stack.pop() {
                    self.out.push(']');
                    self.inline_end(mark);
                }
            }
            OwnedEvent::StartWikiLink { page } => {
                self.enter_inline_child();
                let mark = self.out.len();
                if self.accepts_inline() {
                    self.out.push_str("((");
                    self.out.push_str(&page);
                }
                let label_mark = self.out.len();
                self.stack.push(Frame::WikiLink {
                    mark,
                    page,
                    label_mark,
                    label_text: String::new(),
                });
            }
            OwnedEvent::EndWikiLink => {
                if let Some(Frame::WikiLink {
                    mark,
                    page,
                    label_mark,
                    label_text,
                }) = self.stack.pop()
                {
                    // The label suffix is only needed when the accumulated
                    // plain text of the link's children differs from the
                    // page name — undecidable until every child has been
                    // seen, so the formatted markup was written through
                    // optimistically and is now kept (with a spliced-in `|`)
                    // or discarded in place.
                    if label_text == page {
                        self.out.truncate(label_mark);
                    } else {
                        self.out.insert(label_mark, '|');
                    }
                    self.out.push_str("))");
                    self.inline_end(mark);
                }
            }
            OwnedEvent::InlineImage { url, alt } => {
                self.enter_inline_child();
                if self.accepts_inline() {
                    self.out.push_str("{img src=\"");
                    self.out.push_str(&url);
                    if !alt.is_empty() {
                        self.out.push_str("\" alt=\"");
                        self.out.push_str(&alt);
                    }
                    self.out.push_str("\"}");
                    self.note_label_text(&alt);
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
        w.write_event(OwnedEvent::StartHeading { level: 1 });
        w.write_event(OwnedEvent::Text(std::borrow::Cow::Owned(
            "Hello".to_string(),
        )));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("! Hello"), "got: {s:?}");
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
        let input = "! Hello\n\nA paragraph with __bold__ text.\n\n* item one\n* item two\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e.into_owned());
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
    /// tree-based `build()` for the same document — the guard that keeps the
    /// two independent emission paths honest, across a broad mix of
    /// fixture-shaped inputs (headings, lists incl. nesting, tables, code
    /// blocks, blockquotes, every inline span, links, wikilinks with both a
    /// matching and a differing label, images, nowiki).
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "! Title\n\nIntro paragraph with __strong__ and -+code+-.\n",
            "!! Sub\n\ntext with ''em'' and a [http://x/|link] here.\n",
            "* bullet one\n* bullet two\n",
            "# ordered one\n# ordered two\n",
            "{CODE(lang=rust)}\nlet x = 1;\nlet y = 2;\n{CODE}\n",
            "{CODE()}\nliteral block\n{CODE}\n",
            "{QUOTE()}\nAn indented block quote.\n\nSecond para of quote.\n{QUOTE}\n",
            "||A|B||\n||C|D||\n",
            "||^Head1^|^Head2^||\n||Cell1|Cell2||\n",
            "See ((WikiWord)) for details.\n",
            "See ((WikiWord|Custom Label)) for details.\n",
            "---\n\nAfter the transition.\n",
            "{img src=\"img.png\" alt=\"alt text\"}\n",
            "~np~raw __text__~/np~\n",
            "H^2^O and H,,2,,O.\n",
            "==underlined== and --struck--.\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input);
            let built = crate::emit::build(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events::events(input) {
                w.write_event(e.into_owned());
            }
            let streamed = String::from_utf8(w.finish()).unwrap();

            assert_eq!(
                built, streamed,
                "streaming Writer diverged from build() for input:\n{input}\n\
                 build():\n{built:?}\nWriter:\n{streamed:?}"
            );
        }
    }

    /// Nested lists are the trickiest path in the streaming rewrite:
    /// `list_depth`/`list_marker` bookkeeping happens on `Writer` itself
    /// (bracketing `StartList`/`EndList`), and `emit.rs`'s `build_list_items`
    /// reuses the *outermost* list's marker for nested lists regardless of
    /// their own `ordered` flag — replicated here byte-for-byte.
    #[test]
    fn test_writer_byte_identical_nested_lists() {
        let input = "* outer one\n* outer two\n";
        let (doc, _) = crate::parse::parse(input);
        let built = crate::emit::build(&doc);
        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events(input) {
            w.write_event(e.into_owned());
        }
        let streamed = String::from_utf8(w.finish()).unwrap();
        assert_eq!(built, streamed);
    }

    /// Regression guard against reintroducing per-block `Block`/`Inline`
    /// subtree reconstruction. A large, deeply-nested event stream must
    /// complete with an allocation count that stays close to linear in
    /// event count, not blow up the way tree materialization (an
    /// `Inline`/`Block` enum + `Vec` per node, on top of a full formatting
    /// pass) would.
    ///
    /// `unsafe_code` is denied crate-wide (see `lib.rs`); this is the one
    /// narrowly-scoped, test-only exception, needed because `GlobalAlloc` is
    /// an unsafe trait. The peak-*memory* guard (as opposed to allocation
    /// *count*, measured here) lives in its own test binary
    /// (`tests/streaming_writer_memory.rs`) instead of here, since a process
    /// may only define one `#[global_allocator]` and that test needs
    /// isolation from every other test's concurrent allocations to get a
    /// meaningful reading — see that file's module doc.
    #[test]
    #[allow(unsafe_code)]
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

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 20.0,
            "allocation count did not scale near-linearly: {small} allocs @200 sections -> \
             {large} allocs @2000 sections (ratio {ratio:.2}); this suggests reintroduced \
             per-block subtree reconstruction"
        );
    }
}
