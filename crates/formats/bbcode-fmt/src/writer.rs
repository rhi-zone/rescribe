//! Streaming BBCode writer — converts a stream of events directly to BBCode
//! text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::BbcodeDoc`]/[`crate::Block`]/
//! [`crate::Inline`] value and never calls [`crate::emit::emit`]. It is a
//! second, independent emission path from the tree-based `parse()`/`emit()`
//! functions, not a thin wrapper around them.
//!
//! # Buffer model
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document. Frames on the `Vec<Frame>` stack (`O(nesting depth)`) hold
//! only small metadata — a `usize` mark into `out` and a handful of scalars
//! (a heading level, a table cell's `is_header`, an alignment kind, a span's
//! `attr` string) — never a copy of accumulated child content. Children write
//! **straight through** into `out`; a frame that turns out to have no valid
//! enclosing context truncates `out` back to its own mark when it closes,
//! discarding itself and everything nested inside it in one operation.
//!
//! Unlike `rst-fmt`, no BBCode construct's prefix depends on content not yet
//! seen: there are no underline widths, no column widths (`emit_table` pads
//! nothing), no per-line re-indentation. Every open tag's text
//! (`[b]`, `[quote=author]`, `[h2]`, `[url=..]`, `[color=..]`, …) is fully
//! known at the `Start*`/leaf event itself. So every construct is
//! **write-through**: open text at `Start*`, children write straight into
//! the same buffer, close text at `End*`. No deferred/side stack (the
//! `Frame::Wide` pattern `rst-fmt` uses for its table) is needed here.
//!
//! Two pairs of constructs use a *targeted* membership check instead of the
//! generic "does the enclosing frame accept blocks/inlines" rule, mirroring
//! the original AST builder's `push_block`/`push_inline` exactly:
//! `List`/`ListItem` (an item only survives if its immediate parent is a
//! `List`) and `Table`/`TableRow`/`TableRow`/`TableCell` (same, one level
//! each). Every other pairing uses the generic `accepts_blocks`/
//! `accepts_inline` check against the *known* set of container kinds the
//! original `DocBuilder::push_block`/`push_inline` match arms accepted.
//!
//! A `Blockquote`'s direct `Paragraph` children get a single `"\n"` suffix
//! instead of the usual `"\n\n"` (mirroring `emit_block`'s `Block::Blockquote`
//! arm, which special-cases only that one nesting) — decided by checking the
//! parent frame at `EndParagraph`, the same way `rst-fmt` special-cases a
//! paragraph directly inside a list item.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use bbcode_fmt::writer::Writer;
//! use bbcode_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartParagraph);
//! w.write_event(OwnedEvent::StartBold);
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndBold);
//! w.write_event(OwnedEvent::EndParagraph);
//! let bytes = w.finish();
//! ```

use crate::ast::AlignKind;
use crate::events::Event;
use std::fmt::Write as _;
use std::io::Write;

/// The closing delimiter for a `Frame::Inline` span. Only a handful of
/// distinct closing strings ever occur for the constructs whose suffix is a
/// fixed string known at `Start*` (everything except `Span`, whose suffix
/// embeds the dynamic `attr`), so this stores the discriminant instead of a
/// `&'static str`.
#[derive(Clone, Copy)]
enum CloseDelim {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Subscript,
    Superscript,
    Link,
    Color,
    Size,
    Font,
    Email,
}

impl CloseDelim {
    fn as_str(self) -> &'static str {
        match self {
            CloseDelim::Bold => "[/b]",
            CloseDelim::Italic => "[/i]",
            CloseDelim::Underline => "[/u]",
            CloseDelim::Strikethrough => "[/s]",
            CloseDelim::Subscript => "[/sub]",
            CloseDelim::Superscript => "[/sup]",
            CloseDelim::Link => "[/url]",
            CloseDelim::Color => "[/color]",
            CloseDelim::Size => "[/size]",
            CloseDelim::Font => "[/font]",
            CloseDelim::Email => "[/email]",
        }
    }
}

fn align_tag(kind: AlignKind) -> &'static str {
    match kind {
        AlignKind::Center => "center",
        AlignKind::Left => "left",
        AlignKind::Right => "right",
    }
}

/// Frames carry only a mark into the shared buffer and tiny scalars — never
/// accumulated content. `mark` is where this construct's output begins in
/// `Writer::out`, so it can be discarded wholesale (truncated) if it turns
/// out to have no valid enclosing context.
enum Frame {
    Paragraph {
        mark: usize,
    },
    Blockquote {
        mark: usize,
    },
    List {
        mark: usize,
    },
    /// Only kept if its immediate parent (checked at `EndListItem`) is a
    /// `List` — a targeted membership check, not the generic
    /// `accepts_blocks`/`accepts_inline` rule (list items hold inline
    /// content, but a `List` itself does not generically accept inlines the
    /// way a `Paragraph` does).
    ListItem {
        mark: usize,
    },
    Table {
        mark: usize,
    },
    /// Kept only if its immediate parent is a `Table`.
    TableRow {
        mark: usize,
    },
    /// Kept only if its immediate parent is a `TableRow`. `is_header` is
    /// needed again at `EndTableCell` to close with the matching `[/th]` or
    /// `[/td]`.
    TableCell {
        is_header: bool,
        mark: usize,
    },
    /// `level` is needed again at `EndHeading` to close with `[/h{level}]`.
    Heading {
        level: u8,
        mark: usize,
    },
    /// `kind` is needed again at `EndAlignment` to close with the matching
    /// tag name.
    Alignment {
        kind: AlignKind,
        mark: usize,
    },
    Spoiler {
        mark: usize,
    },
    Indent {
        mark: usize,
    },
    /// Any inline span whose closing text is a fixed string known when the
    /// span opens (bold/italic/underline/strikethrough/sub/sup/link/color/
    /// size/font/email) — see [`CloseDelim`].
    Inline {
        close: CloseDelim,
        mark: usize,
    },
    /// `[attr=value]...[/attr]` — `attr` is needed again at `EndSpan` since
    /// the closing tag embeds it, unlike every other inline span.
    Span {
        attr: String,
        mark: usize,
    },
}

/// Default capacity reserved for `Writer::out` by [`Writer::new`]. See
/// `rst_fmt::writer::DEFAULT_OUT_CAPACITY` for the rationale (skip the first
/// several doublings of geometric growth without committing to a
/// document-specific guess).
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Streaming BBCode writer.
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
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: Event<'_>) {
        self.process(event);
    }

    /// Recover the underlying sink. A well-formed (balanced) event stream
    /// has already flushed everything by the time its last `End*` event is
    /// processed, so this is normally a no-op; it also flushes any content
    /// left over from an unbalanced stream rather than silently dropping it.
    pub fn finish(mut self) -> W {
        self.flush();
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

    /// Whether the top-of-stack frame accepts block children, mirroring
    /// `DocBuilder::push_block`'s match arms (`Document`, `Blockquote`,
    /// `Alignment`, `Spoiler`, `Indent`) exactly. `None` (empty stack, i.e.
    /// the document root) always accepts.
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(
                Frame::Blockquote { .. }
                    | Frame::Alignment { .. }
                    | Frame::Spoiler { .. }
                    | Frame::Indent { .. }
            )
        )
    }

    /// Whether the top-of-stack frame accepts inline children, mirroring
    /// `DocBuilder::push_inline`'s match arms (`Paragraph`, `ListItem`,
    /// `Heading`, every inline span, `TableCell`) exactly.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::ListItem { .. }
                    | Frame::Heading { .. }
                    | Frame::Inline { .. }
                    | Frame::Span { .. }
                    | Frame::TableCell { .. }
            )
        )
    }

    /// Close a block: discard it (truncate back to `mark`) if the enclosing
    /// frame does not take block children, otherwise flush if it completed a
    /// top-level block.
    fn end_block(&mut self, mark: usize) {
        if !self.accepts_blocks() {
            self.out.truncate(mark);
            return;
        }
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Close an inline span: discard it if the enclosing frame does not take
    /// inline children. Inline spans are never top-level blocks, so there is
    /// no flush case here.
    fn end_inline(&mut self, mark: usize) {
        if !self.accepts_inline() {
            self.out.truncate(mark);
        }
    }

    fn open_span(&mut self, open: &str, close: CloseDelim) {
        let mark = self.out.len();
        self.push_out(open);
        self.stack.push(Frame::Inline { close, mark });
    }

    fn close_span(&mut self) {
        if let Some(Frame::Inline { close, mark }) = self.stack.pop() {
            self.push_out(close.as_str());
            self.end_inline(mark);
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
                    // `emit_block`'s `Block::Blockquote` arm special-cases
                    // only its direct `Paragraph` children with a single
                    // "\n" instead of the usual blank-line separator.
                    if matches!(self.stack.last(), Some(Frame::Blockquote { .. })) {
                        self.push_out("\n");
                    } else {
                        self.push_out("\n\n");
                    }
                    self.end_block(mark);
                }
            }
            Event::StartBlockquote { author } => {
                let mark = self.out.len();
                if let Some(author) = &author {
                    self.push_out("[quote=");
                    self.push_out(author);
                    self.push_out("]\n");
                } else {
                    self.push_out("[quote]\n");
                }
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    self.push_out("[/quote]\n\n");
                    self.end_block(mark);
                }
            }
            Event::StartList { ordered } => {
                let mark = self.out.len();
                self.push_out(if ordered { "[list=1]\n" } else { "[list]\n" });
                self.stack.push(Frame::List { mark });
            }
            Event::EndList => {
                if let Some(Frame::List { mark }) = self.stack.pop() {
                    self.push_out("[/list]\n\n");
                    self.end_block(mark);
                }
            }
            Event::StartListItem => {
                let mark = self.out.len();
                self.push_out("[*]");
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
                    self.push_out("[code=");
                    self.push_out(lang);
                    self.push_out("]\n");
                } else {
                    self.push_out("[code]\n");
                }
                self.push_out(&content);
                if !content.ends_with('\n') {
                    self.push_out("\n");
                }
                self.push_out("[/code]\n\n");
                self.end_block(mark);
            }
            Event::StartTable => {
                let mark = self.out.len();
                self.push_out("[table]\n");
                self.stack.push(Frame::Table { mark });
            }
            Event::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    self.push_out("[/table]\n\n");
                    self.end_block(mark);
                }
            }
            Event::StartTableRow => {
                let mark = self.out.len();
                self.push_out("[tr]");
                self.stack.push(Frame::TableRow { mark });
            }
            Event::EndTableRow => {
                if let Some(Frame::TableRow { mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::Table { .. })) {
                        self.push_out("[/tr]\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartTableCell { is_header } => {
                let mark = self.out.len();
                self.push_out(if is_header { "[th]" } else { "[td]" });
                self.stack.push(Frame::TableCell { is_header, mark });
            }
            Event::EndTableCell => {
                if let Some(Frame::TableCell { is_header, mark }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::TableRow { .. })) {
                        self.push_out(if is_header { "[/th]" } else { "[/td]" });
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::HorizontalRule => {
                let mark = self.out.len();
                self.push_out("[hr]\n\n");
                self.end_block(mark);
            }
            Event::StartHeading { level } => {
                let mark = self.out.len();
                write!(self.out, "[h{level}]").unwrap();
                self.stack.push(Frame::Heading { level, mark });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { level, mark }) = self.stack.pop() {
                    write!(self.out, "[/h{level}]\n\n").unwrap();
                    self.end_block(mark);
                }
            }
            Event::StartAlignment { kind } => {
                let mark = self.out.len();
                self.push_out("[");
                self.push_out(align_tag(kind));
                self.push_out("]\n");
                self.stack.push(Frame::Alignment { kind, mark });
            }
            Event::EndAlignment => {
                if let Some(Frame::Alignment { kind, mark }) = self.stack.pop() {
                    self.push_out("[/");
                    self.push_out(align_tag(kind));
                    self.push_out("]\n\n");
                    self.end_block(mark);
                }
            }
            Event::StartSpoiler => {
                let mark = self.out.len();
                self.push_out("[spoiler]\n");
                self.stack.push(Frame::Spoiler { mark });
            }
            Event::EndSpoiler => {
                if let Some(Frame::Spoiler { mark }) = self.stack.pop() {
                    self.push_out("[/spoiler]\n\n");
                    self.end_block(mark);
                }
            }
            Event::Preformatted { content } => {
                let mark = self.out.len();
                self.push_out("[pre]");
                self.push_out(&content);
                self.push_out("[/pre]\n\n");
                self.end_block(mark);
            }
            Event::StartIndent => {
                let mark = self.out.len();
                self.push_out("[indent]\n");
                self.stack.push(Frame::Indent { mark });
            }
            Event::EndIndent => {
                if let Some(Frame::Indent { mark }) = self.stack.pop() {
                    self.push_out("[/indent]\n\n");
                    self.end_block(mark);
                }
            }

            // ── Inline events ───────────────────────────────────────────
            Event::Text(cow) => {
                if self.accepts_inline() {
                    self.push_out(&cow);
                }
            }
            Event::StartBold => self.open_span("[b]", CloseDelim::Bold),
            Event::EndBold => self.close_span(),
            Event::StartItalic => self.open_span("[i]", CloseDelim::Italic),
            Event::EndItalic => self.close_span(),
            Event::StartUnderline => self.open_span("[u]", CloseDelim::Underline),
            Event::EndUnderline => self.close_span(),
            Event::StartStrikethrough => self.open_span("[s]", CloseDelim::Strikethrough),
            Event::EndStrikethrough => self.close_span(),
            Event::StartSubscript => self.open_span("[sub]", CloseDelim::Subscript),
            Event::EndSubscript => self.close_span(),
            Event::StartSuperscript => self.open_span("[sup]", CloseDelim::Superscript),
            Event::EndSuperscript => self.close_span(),
            Event::InlineCode(cow) => {
                if self.accepts_inline() {
                    self.push_out("[code]");
                    self.push_out(&cow);
                    self.push_out("[/code]");
                }
            }
            Event::StartLink { url } => {
                let mark = self.out.len();
                self.push_out("[url=");
                self.push_out(&url);
                self.push_out("]");
                self.stack.push(Frame::Inline {
                    close: CloseDelim::Link,
                    mark,
                });
            }
            Event::EndLink => self.close_span(),
            Event::InlineImage { url, width, height } => {
                if self.accepts_inline() {
                    if let (Some(w), Some(h)) = (width, height) {
                        write!(self.out, "[img={w}x{h}]").unwrap();
                    } else {
                        self.push_out("[img]");
                    }
                    self.push_out(&url);
                    self.push_out("[/img]");
                }
            }
            Event::StartColor { value } => {
                let mark = self.out.len();
                self.push_out("[color=");
                self.push_out(&value);
                self.push_out("]");
                self.stack.push(Frame::Inline {
                    close: CloseDelim::Color,
                    mark,
                });
            }
            Event::EndColor => self.close_span(),
            Event::StartSize { value } => {
                let mark = self.out.len();
                self.push_out("[size=");
                self.push_out(&value);
                self.push_out("]");
                self.stack.push(Frame::Inline {
                    close: CloseDelim::Size,
                    mark,
                });
            }
            Event::EndSize => self.close_span(),
            Event::StartFont { name } => {
                let mark = self.out.len();
                self.push_out("[font=");
                self.push_out(&name);
                self.push_out("]");
                self.stack.push(Frame::Inline {
                    close: CloseDelim::Font,
                    mark,
                });
            }
            Event::EndFont => self.close_span(),
            Event::StartEmail { addr } => {
                let mark = self.out.len();
                self.push_out("[email=");
                self.push_out(&addr);
                self.push_out("]");
                self.stack.push(Frame::Inline {
                    close: CloseDelim::Email,
                    mark,
                });
            }
            Event::EndEmail => self.close_span(),
            Event::Noparse(cow) => {
                if self.accepts_inline() {
                    self.push_out("[noparse]");
                    self.push_out(&cow);
                    self.push_out("[/noparse]");
                }
            }
            Event::StartSpan { attr, value } => {
                let mark = self.out.len();
                self.push_out("[");
                self.push_out(&attr);
                self.push_out("=");
                self.push_out(&value);
                self.push_out("]");
                self.stack.push(Frame::Span { attr, mark });
            }
            Event::EndSpan => {
                if let Some(Frame::Span { attr, mark }) = self.stack.pop() {
                    self.push_out("[/");
                    self.push_out(&attr);
                    self.push_out("]");
                    self.end_inline(mark);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;
    use std::borrow::Cow;

    #[test]
    fn test_writer_roundtrip() {
        let input = "[b]bold[/b]";
        let events: Vec<OwnedEvent> = crate::events::events_str(input)
            .map(|e| e.into_owned())
            .collect();

        let mut w = Writer::new(Vec::<u8>::new());
        for ev in events {
            w.write_event(ev);
        }
        let bytes = w.finish();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains("[b]bold[/b]"));
    }

    #[test]
    fn test_writer_complex() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::StartBold);
        w.write_event(OwnedEvent::Text(Cow::Owned("hello".to_string())));
        w.write_event(OwnedEvent::EndBold);
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains("[b]hello[/b]"));
    }

    /// The streaming `Writer` must produce *byte-identical* output to the
    /// tree-based `parse()` + `emit()` path for the same document. This is
    /// the guard that keeps the two independent emission paths honest.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "[b]bold[/b] and [i]italic[/i] and [u]under[/u] and [s]strike[/s]",
            "[sub]sub[/sub] and [sup]sup[/sup]",
            "[quote]a plain quote paragraph[/quote]",
            "[quote=Alice]quoted text[/quote]\n\nsecond paragraph",
            "[list]\n[*]one\n[*]two\n[/list]",
            "[list=1]\n[*]first\n[*]second\n[/list]",
            "[code=rust]\nlet x = 1;\nlet y = 2;\n[/code]",
            "[code]no language here[/code]",
            "[table]\n[tr][th]A[/th][th]B[/th][/tr]\n[tr][td]1[/td][td]2[/td][/tr]\n[/table]",
            "[hr]",
            "[h1]Heading One[/h1]\n\n[h3]Heading Three[/h3]",
            "[center]\ncentered text\n[/center]\n\n[left]\nleft text\n[/left]\n\n\
             [right]\nright text\n[/right]",
            "[spoiler]\nhidden content\n[/spoiler]",
            "[pre]  preformatted   text  [/pre]",
            "[indent]\nindented content\n[/indent]",
            "[url=https://example.com]a link[/url]",
            "[img]https://example.com/x.png[/img]",
            "[img=100x50]https://example.com/y.png[/img]",
            "[color=red]red text[/color] [size=12]sized[/size] [font=Arial]fonted[/font]",
            "[email=a@b.com]mail me[/email]",
            "[noparse][b]raw bbcode[/b][/noparse]",
            "[custom=value]spanned content[/custom]",
            "[b]nested [i]bold-italic[/i] text[/b]",
            "[quote=Bob]\n[list]\n[*]nested item\n[/list]\n[/quote]",
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
                "streaming Writer diverged from parse()+emit() for input:\n{input}\n\
                 emit():\n{built:?}\nWriter:\n{streamed:?}"
            );
        }
    }

    // A single combined allocation-tracking global allocator, shared by the
    // two memory-shape tests below (`test_writer_no_subtree_reconstruction_
    // blowup` needs allocation *count*, `test_writer_peak_memory_bounded`
    // needs *peak bytes resident* — only one `#[global_allocator]` may be
    // defined per test binary, so both metrics are tracked here together).
    // `#[allow(unsafe_code)]` overrides the crate-wide `#![deny(unsafe_code)]`
    // for this test-only `GlobalAlloc` impl, which is unavoidably unsafe.
    //
    // Counters are **thread-local**, not process-global atomics: `cargo
    // test` runs the test binary's tests concurrently on multiple OS
    // threads by default (one thread per test), and a shared global counter
    // is polluted by every other test's unrelated allocations happening
    // during the measurement window (confirmed empirically — a shared-atomic
    // version of this tracker saw peak-byte deltas of several MB from
    // concurrently-running fixture/round-trip tests, swamping the signal).
    // Tracking is further gated by a thread-local `ENABLED` flag so that
    // only the bytes/allocations this thread performs *inside* its own
    // measurement window count at all — allocations on this same thread
    // before/after the window (e.g. building the synthetic input, or test
    // harness bookkeeping) are excluded too.
    //
    // The `const` initializer form of `thread_local!` is required here: it
    // compiles to purely static (no lazy `Once`-guarded heap box) storage on
    // supported platforms, avoiding the classic reentrant-allocator hazard
    // of a `GlobalAlloc` impl that itself allocates on a `thread_local!`'s
    // first access.
    #[allow(unsafe_code)]
    mod alloc_tracking {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;

        pub struct TrackingAlloc;

        thread_local! {
            static ENABLED: Cell<bool> = const { Cell::new(false) };
            static ALLOCS: Cell<usize> = const { Cell::new(0) };
            static CURRENT: Cell<usize> = const { Cell::new(0) };
            static PEAK: Cell<usize> = const { Cell::new(0) };
        }

        /// RAII guard: zeroes this thread's counters and enables tracking
        /// on construction, disables tracking on drop (including on panic,
        /// via unwind, so a failed assertion never leaves tracking stuck on
        /// for whatever runs next on this thread).
        pub struct Window;

        impl Window {
            pub fn start() -> Self {
                ALLOCS.with(|c| c.set(0));
                CURRENT.with(|c| c.set(0));
                PEAK.with(|c| c.set(0));
                ENABLED.with(|e| e.set(true));
                Window
            }

            pub fn allocs(&self) -> usize {
                ALLOCS.with(Cell::get)
            }

            pub fn peak(&self) -> usize {
                PEAK.with(Cell::get)
            }
        }

        impl Drop for Window {
            fn drop(&mut self) {
                ENABLED.with(|e| e.set(false));
            }
        }

        unsafe impl GlobalAlloc for TrackingAlloc {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                let ptr = unsafe { System.alloc(layout) };
                if !ptr.is_null() {
                    ENABLED.with(|e| {
                        if e.get() {
                            ALLOCS.with(|c| c.set(c.get() + 1));
                            CURRENT.with(|c| {
                                let now = c.get() + layout.size();
                                c.set(now);
                                PEAK.with(|p| p.set(p.get().max(now)));
                            });
                        }
                    });
                }
                ptr
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                ENABLED.with(|e| {
                    if e.get() {
                        CURRENT.with(|c| c.set(c.get().saturating_sub(layout.size())));
                    }
                });
                unsafe { System.dealloc(ptr, layout) }
            }
        }
    }
    #[allow(unsafe_code)]
    #[global_allocator]
    static GLOBAL: alloc_tracking::TrackingAlloc = alloc_tracking::TrackingAlloc;

    /// Regression guard against reintroducing per-block `Block`/`Inline`
    /// subtree reconstruction. A large, deeply-nested event stream must
    /// complete with an allocation count that stays close to linear in
    /// event count, not blow up the way tree materialization (an
    /// `Inline`/`Block` enum + `Vec` per node, plus a full `emit()` pass)
    /// would.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use alloc_tracking::Window;

        // Build an event stream for `n` top-level sections (heading +
        // paragraph with inline markup + a 2-item list), doubling `n` to
        // check allocation count scales roughly linearly, not
        // superlinearly.
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
                    evs.push(OwnedEvent::Text(Cow::Owned(format!("item {j}"))));
                    evs.push(OwnedEvent::EndListItem);
                }
                evs.push(OwnedEvent::EndList);
            }
            evs
        }

        fn run(n: usize) -> usize {
            let evs = events_for(n);
            let window = Window::start();
            let mut out = Vec::new();
            {
                let mut w = Writer::new(&mut out);
                for e in evs {
                    w.write_event(e);
                }
                w.finish();
            }
            let allocs = window.allocs();
            std::hint::black_box(&out);
            allocs
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

    /// Peak memory (current bytes allocated, tracked via a peak-tracking
    /// global allocator) must stay within a small constant multiple of a
    /// single event's/block's size on a large synthetic document — not grow
    /// with the number of top-level blocks. This is the direct memory-shape
    /// check that the allocation-*count* guard above doesn't give: a buggy
    /// implementation could still allocate a linear number of small chunks
    /// that are never freed (e.g. an ever-growing `Vec<OwnedEvent>`), which
    /// looks "linear" by count but is `O(full document)` by bytes resident.
    #[test]
    fn test_writer_peak_memory_bounded() {
        use alloc_tracking::Window;

        // A few thousand paragraphs, each with some inline markup, fed
        // through the writer with a sink that discards bytes rather than
        // accumulating them (so the *sink* isn't what bounds peak memory —
        // the writer's own internal state is what's under test).
        struct DiscardSink;
        impl Write for DiscardSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        const N: usize = 5000;
        const PARA_TEXT: &str = "This is a moderately long paragraph of plain BBCode text, \
                                  long enough to be a representative single block payload.";

        let window = Window::start();

        let mut w = Writer::new(DiscardSink);
        for i in 0..N {
            w.write_event(Event::StartParagraph);
            w.write_event(Event::Text(std::borrow::Cow::Owned(format!(
                "{PARA_TEXT} #{i} "
            ))));
            w.write_event(Event::StartBold);
            w.write_event(Event::Text(std::borrow::Cow::Borrowed("bold aside")));
            w.write_event(Event::EndBold);
            w.write_event(Event::EndParagraph);
        }
        w.finish();

        let peak = window.peak();
        // A generous bound: 64 KiB comfortably covers the shared `out`
        // buffer's geometric growth up to one block's size plus the tiny
        // frame stack, while `N` (5000) single-paragraph blocks at ~150
        // bytes each would be ~750 KiB if genuinely accumulated across the
        // whole document instead of flushed per top-level block. Thread-local
        // tracking (see `alloc_tracking`) means this reflects only this
        // test's own thread, not noise from concurrently-running tests.
        const MAX_PEAK_BYTES: usize = 64 * 1024;
        assert!(
            peak < MAX_PEAK_BYTES,
            "peak allocated bytes {peak} exceeded {MAX_PEAK_BYTES} for {N} paragraphs; \
             expected O(largest top-level block), not O(full document)"
        );
    }
}
